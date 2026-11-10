//! Galaxy宝可梦: built-in modpack updater (GALAXY_SYNC.md).
//!
//! The launcher is the DreamingFish client now. When it opens (and before
//! every launch) the frontend calls [`check`]: it fetches the signed release
//! manifest from the DreamingFish update server, verifies it with the key from
//! `.dreamingfish-bootstrap/project-binding.json` (or the built-in constants)
//! and compares it with the instance through a hash cache. [`apply`] downloads
//! what is missing (resumable, size + SHA-256 verified, at most 4 at a time),
//! moves replaced / removed files into `.galaxy-sync/backups/` and puts the
//! new files in place, undoing everything if a step fails. The server side
//! (DreamingFishAdmin `serve`, `dfs-release.ps1`) is unchanged.
//!
//! Launcher-owned state lives in `<instance>/.galaxy-sync/` (`state.json`,
//! `release-manifest.json` + `release-manifest.sig`, `file-index.json`,
//! `downloads/`, `backups/`); DreamingFish's own player state is never used.
//! The rules themselves are in [`engine`].

mod engine;

use crate::api::galaxy::{find_game_root, instance_dir_for};
use engine::{
    FileIndex, HashProgress, IndexEntry, InstallReport, LocalFiles, Manifest,
    Plan, SyncState, absolute, path_key, sha256_hex, stat_file, write_atomic,
};
use futures::TryStreamExt;
use reqwest::header;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

/// Event with [`GalaxySyncProgress`] payloads, at most ~5 per second.
pub const PROGRESS_EVENT: &str = "galaxy_sync_progress";
/// Used when the instance has no `project-binding.json`; the file wins when it
/// exists (lets the owner move the server).
pub const DEFAULT_BASE_URL: &str = "http://mc9.yunmaoc.com:28080";
pub const DEFAULT_PROJECT_ID: &str = "galaxy-pokemon";
pub const DEFAULT_PUBLIC_KEY: &str =
    "MCowBQYDK2VwAyEAPqMOIqOlK/TQgA+Bihv3Pt0bms91Kf77iMDOfKv+dTA=";

const BINDING_FILE: &str = "project-binding.json";
const BUNDLED_RELEASE_DIR: &str = "bundled-release";
const STATE_FILE: &str = "state.json";
const MANIFEST_COPY: &str = "release-manifest.json";
const SIGNATURE_COPY: &str = "release-manifest.sig";
const INDEX_FILE: &str = "file-index.json";
const DOWNLOADS_DIR: &str = "downloads";
const BACKUPS_DIR: &str = "backups";
const SIGNATURE_HEADER: &str = "x-dfs-signature";

const MANIFEST_MAX_BYTES: usize = 32 * 1024 * 1024;
const SIGNATURE_MAX_BYTES: usize = 4 * 1024;
const HISTORY_MAX_BYTES: usize = 1024 * 1024;
/// Connect + response headers.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MANIFEST_TIMEOUT: Duration = Duration::from_secs(60);
const HISTORY_TIMEOUT: Duration = Duration::from_secs(5);
const OBJECT_HEADERS_TIMEOUT: Duration = Duration::from_secs(20);
/// Longest pause between two body chunks of an object download.
const OBJECT_CHUNK_TIMEOUT: Duration = Duration::from_secs(60);
const PARALLEL_DOWNLOADS: usize = 4;
/// 429 / 503 retries (the server answers `Retry-After: 2` when it is full).
const BUSY_RETRIES: usize = 2;
const KEEP_BACKUPS: usize = 3;
/// `apply` reuses the manifest `check` just verified when it is this fresh.
const MANIFEST_REUSE: Duration = Duration::from_secs(5 * 60);
const PROGRESS_INTERVAL: Duration = Duration::from_millis(200);
const DISK_MARGIN: u64 = 64 * 1024 * 1024;
const HISTORY_LIMIT: usize = 20;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SyncStatus {
    UpToDate,
    UpdateAvailable,
    /// The update server could not be reached; the local pack is used.
    Offline,
    /// Refused for safety (signature, replay, unsafe path, newer launcher
    /// needed, local record tampered).
    Blocked,
    /// `apply` failed; nothing was left half-installed.
    Error,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalaxySyncHistoryEntry {
    pub release_id: String,
    pub sequence: u64,
    pub display_version: Option<String>,
    pub changelog: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalaxySyncResult {
    pub status: SyncStatus,
    /// Machine-readable reason for offline / blocked / error.
    pub code: Option<String>,
    /// Chinese text for the UI.
    pub message: Option<String>,
    /// displayVersion of the last applied release (or the bundled baseline).
    pub local_version: Option<String>,
    /// displayVersion of the server's release.
    pub remote_version: Option<String>,
    pub release_id: Option<String>,
    pub sequence: Option<u64>,
    pub changelog: Option<String>,
    pub released_at: Option<String>,
    pub files_to_download: u64,
    pub bytes_to_download: u64,
    pub files_to_remove: u64,
    /// Files had to be hashed for this check (first run or 修复游戏文件).
    pub hashed_files: u64,
    /// `apply` changed files.
    pub applied: bool,
    pub files_installed: u64,
    pub files_backed_up: u64,
    pub backup_dir: Option<String>,
    /// `launcher-update/*` changed: re-run the launcher update check.
    pub launcher_update_changed: bool,
    /// `/history`, newest first (online checks only, best effort).
    pub history: Vec<GalaxySyncHistoryEntry>,
    pub checked_at: String,
}

impl GalaxySyncResult {
    fn new(status: SyncStatus) -> Self {
        Self {
            status,
            code: None,
            message: None,
            local_version: None,
            remote_version: None,
            release_id: None,
            sequence: None,
            changelog: None,
            released_at: None,
            files_to_download: 0,
            bytes_to_download: 0,
            files_to_remove: 0,
            hashed_files: 0,
            applied: false,
            files_installed: 0,
            files_backed_up: 0,
            backup_dir: None,
            launcher_update_changed: false,
            history: Vec::new(),
            checked_at: now_rfc3339(),
        }
    }

    fn failure(status: SyncStatus, code: &str, message: impl Into<String>) -> Self {
        let mut result = Self::new(status);
        result.code = Some(code.to_string());
        result.message = Some(message.into());
        result
    }
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalaxySyncProgress {
    /// "checking" | "hashing" | "downloading" | "applying" | "done" | "error"
    pub phase: &'static str,
    pub done_files: u64,
    pub total_files: u64,
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub current_file: Option<String>,
    pub message: Option<String>,
}

/// `galaxy_sync_status`: lets a reloaded page pick up a running sync.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalaxySyncSnapshot {
    pub busy: bool,
    pub last_result: Option<GalaxySyncResult>,
    pub progress: Option<GalaxySyncProgress>,
}

/// Serializes check / apply / launch.
static SYNC_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static LAST_RESULT: Mutex<Option<GalaxySyncResult>> = Mutex::new(None);
static LAST_PROGRESS: Mutex<Option<GalaxySyncProgress>> = Mutex::new(None);
static VERIFIED: Mutex<Option<VerifiedManifest>> = Mutex::new(None);

#[derive(Clone)]
struct VerifiedManifest {
    fetched: Instant,
    base_url: String,
    project_id: String,
    raw: Arc<Vec<u8>>,
    signature: String,
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn other_error(message: impl Into<String>) -> crate::Error {
    crate::ErrorKind::OtherError(message.into()).as_error()
}

fn busy_error() -> crate::Error {
    crate::ErrorKind::InputError("正在检查或更新整合包，请稍候".to_string()).as_error()
}

/// Held by a launch so it cannot overlap a check or an apply.
pub(crate) fn try_launch_guard()
-> crate::Result<tokio::sync::MutexGuard<'static, ()>> {
    SYNC_LOCK.try_lock().map_err(|_| {
        crate::ErrorKind::InputError(
            "整合包正在检查或更新，请等它完成后再开始游戏".to_string(),
        )
        .as_error()
    })
}

pub fn status() -> GalaxySyncSnapshot {
    GalaxySyncSnapshot {
        busy: SYNC_LOCK.try_lock().is_err(),
        last_result: LAST_RESULT.lock().ok().and_then(|slot| slot.clone()),
        progress: LAST_PROGRESS.lock().ok().and_then(|slot| slot.clone()),
    }
}

// ---- progress ---------------------------------------------------------------------

#[derive(Default)]
struct Reporter {
    last: Mutex<Option<Instant>>,
}

impl Reporter {
    fn send(&self, progress: GalaxySyncProgress, force: bool) {
        if let Ok(mut last) = self.last.lock() {
            if !force && last.is_some_and(|at| at.elapsed() < PROGRESS_INTERVAL) {
                return;
            }
            *last = Some(Instant::now());
        }
        if let Ok(mut slot) = LAST_PROGRESS.lock() {
            *slot = Some(progress.clone());
        }
        crate::event::emit::emit_galaxy_sync_progress(progress);
    }

    fn phase(&self, phase: &'static str, message: Option<String>) {
        self.send(
            GalaxySyncProgress {
                phase,
                message,
                ..GalaxySyncProgress::default()
            },
            true,
        );
    }
}

// ---- binding, state files -------------------------------------------------------

struct Context {
    instance_dir: PathBuf,
    sync_dir: PathBuf,
    base_url: String,
    project_id: String,
    key: ed25519_dalek::VerifyingKey,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BindingFile {
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    public_key: Option<String>,
}

fn strip_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes)
}

fn valid_base_url(text: &str) -> Option<String> {
    let parsed = url::Url::parse(text.trim()).ok()?;
    (matches!(parsed.scheme(), "http" | "https")
        && parsed.host_str().is_some()
        && parsed.username().is_empty()
        && parsed.password().is_none())
    .then(|| text.trim().trim_end_matches('/').to_string())
}

fn valid_project_id(text: &str) -> bool {
    (1..=128).contains(&text.len())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

/// The binding file wins; the built-in constants fill anything missing or
/// unusable.
fn load_context() -> Result<Context, GalaxySyncResult> {
    let not_found = || {
        GalaxySyncResult::failure(
            SyncStatus::Blocked,
            "GAME_NOT_FOUND",
            "没有找到游戏目录（.minecraft），无法检查整合包更新",
        )
    };
    let root = find_game_root().ok_or_else(not_found)?;
    let root = dunce::canonicalize(&root).unwrap_or(root);
    let instance_dir = instance_dir_for(&root);
    if !instance_dir.is_dir() {
        return Err(not_found());
    }

    let binding_path = instance_dir.join(engine::BOOTSTRAP_DIR).join(BINDING_FILE);
    let binding = match std::fs::read(&binding_path) {
        Ok(bytes) => serde_json::from_slice::<BindingFile>(strip_bom(&bytes))
            .inspect_err(|error| {
                tracing::warn!(%error, "Galaxy sync: unreadable project-binding.json, using the built-in values");
            })
            .unwrap_or_default(),
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(%error, "Galaxy sync: cannot read project-binding.json, using the built-in values");
            }
            BindingFile::default()
        }
    };
    let base_url = binding
        .base_url
        .as_deref()
        .and_then(valid_base_url)
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    let project_id = binding
        .project_id
        .filter(|id| valid_project_id(id))
        .unwrap_or_else(|| DEFAULT_PROJECT_ID.to_string());
    let key = binding
        .public_key
        .as_deref()
        .and_then(|key| engine::parse_public_key(key).ok())
        .map_or_else(|| engine::parse_public_key(DEFAULT_PUBLIC_KEY), Ok)
        .map_err(|message| {
            GalaxySyncResult::failure(SyncStatus::Blocked, "BINDING_INVALID", message)
        })?;
    Ok(Context {
        sync_dir: instance_dir.join(engine::STATE_DIR),
        instance_dir,
        base_url,
        project_id,
        key,
    })
}

fn load_state(sync_dir: &Path) -> Option<SyncState> {
    let path = sync_dir.join(STATE_FILE);
    let bytes = std::fs::read(&path).ok()?;
    match serde_json::from_slice::<SyncState>(strip_bom(&bytes)) {
        Ok(state) => Some(state),
        Err(error) => {
            tracing::warn!(%error, "Galaxy sync: state.json is corrupt; starting over");
            let aside = sync_dir.join(format!(
                "state.json.corrupt-{}",
                chrono::Local::now().format("%Y%m%d-%H%M%S")
            ));
            let _ = std::fs::rename(&path, aside);
            None
        }
    }
}

fn load_index(sync_dir: &Path) -> FileIndex {
    std::fs::read(sync_dir.join(INDEX_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(strip_bom(&bytes)).ok())
        .unwrap_or_default()
}

fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    write_atomic(path, &bytes).map_err(|error| format!("无法写入 {}：{error}", path.display()))
}

/// A signed manifest on disk (bundled baseline or the applied copy).
struct SignedCopy {
    sha256: String,
    manifest: Manifest,
}

enum CopyCheck {
    Missing,
    Valid(SignedCopy),
    /// The signature does not match the bytes.
    Invalid { sha256: String },
}

fn read_signed(ctx: &Context, manifest: &Path, signature: &Path) -> CopyCheck {
    let (Ok(raw), Ok(sig)) = (std::fs::read(manifest), std::fs::read_to_string(signature)) else {
        return CopyCheck::Missing;
    };
    let sha256 = sha256_hex(&raw);
    if !engine::verify_signature(&ctx.key, &raw, &sig) {
        return CopyCheck::Invalid { sha256 };
    }
    match engine::parse_manifest(&raw) {
        Ok(manifest) if manifest.project_id == ctx.project_id => {
            CopyCheck::Valid(SignedCopy { sha256, manifest })
        }
        _ => CopyCheck::Invalid { sha256 },
    }
}

fn bundled_baseline(ctx: &Context) -> Option<SignedCopy> {
    let dir = ctx.instance_dir.join(engine::BOOTSTRAP_DIR).join(BUNDLED_RELEASE_DIR);
    match read_signed(ctx, &dir.join("manifest.json"), &dir.join("manifest.sig")) {
        CopyCheck::Valid(copy) => Some(copy),
        CopyCheck::Invalid { .. } => {
            tracing::warn!("Galaxy sync: the bundled baseline manifest has a bad signature; ignoring it");
            None
        }
        CopyCheck::Missing => None,
    }
}

// ---- network ----------------------------------------------------------------------

enum FetchError {
    Offline(String),
    Blocked(&'static str, String),
}

fn describe_request_error(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "连接更新服务器超时".to_string()
    } else if error.is_connect() {
        "无法连接更新服务器".to_string()
    } else {
        format!("网络错误：{error}")
    }
}

fn retry_after(response: &reqwest::Response) -> u64 {
    response
        .headers()
        .get(header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(2)
        .clamp(1, 10)
}

/// Sends a request (headers within `headers_timeout`) and retries 429 / 503
/// twice, honouring `Retry-After`.
async fn send_with_retry(
    make: impl Fn() -> reqwest::RequestBuilder,
    headers_timeout: Duration,
) -> Result<reqwest::Response, String> {
    let mut busy = 0;
    loop {
        let response = tokio::time::timeout(headers_timeout, make().send())
            .await
            .map_err(|_| "连接更新服务器超时".to_string())?
            .map_err(|error| {
                tracing::debug!(%error, "Galaxy sync: request failed");
                describe_request_error(&error)
            })?;
        let status = response.status().as_u16();
        if matches!(status, 429 | 503) && busy < BUSY_RETRIES {
            busy += 1;
            tokio::time::sleep(Duration::from_secs(retry_after(&response))).await;
            continue;
        }
        return Ok(response);
    }
}

enum ReadError {
    TooLarge,
    Network(String),
}

async fn read_capped(mut response: reqwest::Response, cap: usize) -> Result<Vec<u8>, ReadError> {
    if response.content_length().is_some_and(|length| length > cap as u64) {
        return Err(ReadError::TooLarge);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| ReadError::Network(describe_request_error(&error)))?
    {
        if body.len() + chunk.len() > cap {
            return Err(ReadError::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// `GET /v1/projects/{id}/latest`: the raw manifest bytes and the base64
/// signature (`X-Dfs-Signature`, or `{uri}.sig`).
async fn fetch_latest(ctx: &Context) -> Result<(Vec<u8>, String), FetchError> {
    let client = crate::util::fetch::configured_client()
        .await
        .map_err(|error| FetchError::Offline(error.to_string()))?;
    let url = format!("{}/v1/projects/{}/latest", ctx.base_url, ctx.project_id);
    let work = async {
        let get = |url: &str, accept: &'static str| {
            client
                .get(url)
                .header(header::ACCEPT, accept)
                .header(header::ACCEPT_ENCODING, "identity")
                .timeout(MANIFEST_TIMEOUT)
        };
        let response = send_with_retry(|| get(&url, "application/json"), CONNECT_TIMEOUT)
            .await
            .map_err(FetchError::Offline)?;
        let status = response.status();
        if !status.is_success() {
            return Err(FetchError::Offline(format!("更新服务器返回 {status}")));
        }
        let header_signature = response
            .headers()
            .get(SIGNATURE_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let body = read_capped(response, MANIFEST_MAX_BYTES).await.map_err(|error| match error {
            ReadError::TooLarge => FetchError::Blocked(
                "MANIFEST_TOO_LARGE",
                "整合包清单超过 32 MB，已停止更新".to_string(),
            ),
            ReadError::Network(message) => FetchError::Offline(message),
        })?;
        let signature = match header_signature {
            Some(signature) => signature,
            None => {
                let sig_url = format!("{url}.sig");
                let response = send_with_retry(|| get(&sig_url, "text/plain"), CONNECT_TIMEOUT)
                    .await
                    .map_err(FetchError::Offline)?;
                if !response.status().is_success() {
                    return Err(FetchError::Blocked(
                        "SIGNATURE_MISSING",
                        format!("整合包清单没有签名（{}），已停止更新", response.status()),
                    ));
                }
                let bytes = read_capped(response, SIGNATURE_MAX_BYTES).await.map_err(|error| match error {
                    ReadError::TooLarge => FetchError::Blocked(
                        "SIGNATURE_INVALID",
                        "整合包清单的签名格式不对，已停止更新".to_string(),
                    ),
                    ReadError::Network(message) => FetchError::Offline(message),
                })?;
                String::from_utf8_lossy(&bytes).trim().to_string()
            }
        };
        Ok((body, signature))
    };
    tokio::time::timeout(MANIFEST_TIMEOUT, work)
        .await
        .map_err(|_| FetchError::Offline("获取整合包清单超时".to_string()))?
}

#[derive(Deserialize)]
struct HistoryBody {
    #[serde(default)]
    releases: Vec<HistoryRelease>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryRelease {
    #[serde(default)]
    release_id: String,
    #[serde(default)]
    sequence: u64,
    #[serde(default)]
    display_version: Option<String>,
    #[serde(default)]
    changelog: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
}

/// `GET /v1/projects/{id}/history` (unsigned, only shown as text).
async fn fetch_history(base_url: &str, project_id: &str) -> Vec<GalaxySyncHistoryEntry> {
    let work = async {
        let client = crate::util::fetch::configured_client().await.ok()?;
        let url = format!("{base_url}/v1/projects/{project_id}/history");
        let response = client
            .get(&url)
            .header(header::ACCEPT, "application/json")
            .header(header::ACCEPT_ENCODING, "identity")
            .send()
            .await
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        let bytes = read_capped(response, HISTORY_MAX_BYTES).await.ok()?;
        let body: HistoryBody = serde_json::from_slice(strip_bom(&bytes)).ok()?;
        Some(
            body.releases
                .into_iter()
                .take(HISTORY_LIMIT)
                .map(|release| GalaxySyncHistoryEntry {
                    release_id: release.release_id,
                    sequence: release.sequence,
                    display_version: release.display_version,
                    changelog: release.changelog,
                    created_at: release.created_at,
                })
                .collect(),
        )
    };
    tokio::time::timeout(HISTORY_TIMEOUT, work)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Download progress shared by the parallel downloads.
struct DownloadProgress {
    reporter: Arc<Reporter>,
    done_files: AtomicU64,
    total_files: u64,
    done_bytes: AtomicU64,
    total_bytes: u64,
    current: Mutex<Option<String>>,
}

impl DownloadProgress {
    fn tick(&self, force: bool) {
        self.reporter.send(
            GalaxySyncProgress {
                phase: "downloading",
                done_files: self.done_files.load(Ordering::Relaxed),
                total_files: self.total_files,
                done_bytes: self.done_bytes.load(Ordering::Relaxed).min(self.total_bytes),
                total_bytes: self.total_bytes,
                current_file: self.current.lock().ok().and_then(|current| current.clone()),
                message: None,
            },
            force,
        );
    }

    /// Moves the byte counter from `*counted` to `now` for one object.
    fn set_object_bytes(&self, counted: &mut u64, now: u64) {
        if now >= *counted {
            self.done_bytes.fetch_add(now - *counted, Ordering::Relaxed);
        } else {
            let less = *counted - now;
            let _ = self.done_bytes.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |bytes| {
                Some(bytes.saturating_sub(less))
            });
        }
        *counted = now;
        self.tick(false);
    }
}

enum PartError {
    /// Worth another attempt (network, server busy, interrupted).
    Transient(String),
    Fatal(String),
}

/// Downloads (or resumes) one object into its `.part` file.
async fn fetch_part(
    client: &reqwest::Client,
    url: &str,
    part: &Path,
    size: u64,
    counted: &mut u64,
    progress: &DownloadProgress,
) -> Result<(), PartError> {
    let mut existing = tokio::fs::metadata(part).await.map_or(0, |meta| meta.len());
    if existing > size {
        let _ = tokio::fs::remove_file(part).await;
        existing = 0;
    }
    progress.set_object_bytes(counted, existing);
    if existing == size {
        return Ok(());
    }
    let response = send_with_retry(
        || {
            let request = client.get(url).header(header::ACCEPT_ENCODING, "identity");
            if existing > 0 {
                request.header(header::RANGE, format!("bytes={existing}-"))
            } else {
                request
            }
        },
        OBJECT_HEADERS_TIMEOUT,
    )
    .await
    .map_err(PartError::Transient)?;
    let status = response.status().as_u16();
    let append = match status {
        206 if existing > 0 => {
            let expected = format!("bytes {existing}-");
            let range_ok = response
                .headers()
                .get(header::CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.starts_with(&expected));
            if !range_ok {
                let _ = tokio::fs::remove_file(part).await;
                progress.set_object_bytes(counted, 0);
                return Err(PartError::Transient("续传位置不对，重新下载".to_string()));
            }
            true
        }
        200 => false,
        416 => {
            let _ = tokio::fs::remove_file(part).await;
            progress.set_object_bytes(counted, 0);
            return Err(PartError::Transient("续传失败，重新下载".to_string()));
        }
        404 => {
            return Err(PartError::Fatal(
                "更新服务器上没有这个文件（object_not_found）".to_string(),
            ));
        }
        429 | 500..=599 => {
            return Err(PartError::Transient(format!("更新服务器繁忙（{status}）")));
        }
        _ => return Err(PartError::Fatal(format!("更新服务器返回 {status}"))),
    };

    let opened = if append {
        tokio::fs::OpenOptions::new().append(true).open(part).await
    } else {
        tokio::fs::File::create(part).await
    };
    let file = opened.map_err(|error| PartError::Fatal(format!("无法写入下载文件：{}", engine::describe_io_error(&error))))?;
    let mut written = if append { existing } else { 0 };
    progress.set_object_bytes(counted, written);
    let mut writer = tokio::io::BufWriter::with_capacity(256 * 1024, file);
    let mut response = response;
    loop {
        let chunk = tokio::time::timeout(OBJECT_CHUNK_TIMEOUT, response.chunk())
            .await
            .map_err(|_| PartError::Transient("下载超时".to_string()))?
            .map_err(|error| PartError::Transient(describe_request_error(&error)))?;
        let Some(chunk) = chunk else { break };
        written += chunk.len() as u64;
        if written > size {
            drop(writer);
            let _ = tokio::fs::remove_file(part).await;
            progress.set_object_bytes(counted, 0);
            return Err(PartError::Fatal("服务器返回的文件比清单里的大".to_string()));
        }
        writer.write_all(&chunk).await.map_err(|error| {
            PartError::Fatal(format!("写入下载文件失败：{}", engine::describe_io_error(&error)))
        })?;
        progress.set_object_bytes(counted, written);
    }
    writer.flush().await.map_err(|error| {
        PartError::Fatal(format!("写入下载文件失败：{}", engine::describe_io_error(&error)))
    })?;
    drop(writer);
    if written < size {
        return Err(PartError::Transient("下载中断".to_string()));
    }
    Ok(())
}

/// Downloads one object to `downloads/<sha>.part` and verifies size + SHA-256
/// (a mismatch deletes the part and retries once).
async fn download_object(
    client: &reqwest::Client,
    base_url: &str,
    object: &PendingObject,
    downloads: &Path,
    progress: &DownloadProgress,
) -> Result<(), String> {
    let part = engine::part_path(downloads, &object.sha256);
    let url = format!("{base_url}/v1/objects/sha256/{}", object.sha256);
    let label = &object.label;
    let mut counted = 0u64;
    let mut mismatches = 0;
    let mut transient = 0;
    if let Ok(mut current) = progress.current.lock() {
        *current = Some(label.clone());
    }
    loop {
        match fetch_part(client, &url, &part, object.size, &mut counted, progress).await {
            Ok(()) => {}
            Err(PartError::Fatal(message)) => return Err(format!("下载 {label} 失败：{message}")),
            Err(PartError::Transient(message)) => {
                transient += 1;
                if transient > 3 {
                    return Err(format!("下载 {label} 失败：{message}"));
                }
                tracing::warn!(file = %label, %message, attempt = transient, "Galaxy sync: retrying download");
                tokio::time::sleep(Duration::from_secs(transient)).await;
                continue;
            }
        }
        let check = part.clone();
        let (actual, length) = tokio::task::spawn_blocking(move || engine::hash_file(&check, None))
            .await
            .map_err(|error| error.to_string())?
            .map_err(|error| format!("无法读取下载的 {label}：{error}"))?;
        if length == object.size && actual == object.sha256 {
            return Ok(());
        }
        let _ = tokio::fs::remove_file(&part).await;
        progress.set_object_bytes(&mut counted, 0);
        mismatches += 1;
        tracing::warn!(file = %label, expected = %object.sha256, %actual, "Galaxy sync: downloaded file failed verification");
        if mismatches > 1 {
            return Err(format!("{label} 下载后校验失败（SHA-256 不一致）"));
        }
    }
}

struct PendingObject {
    sha256: String,
    size: u64,
    label: String,
    /// How many paths use this object.
    uses: u64,
}

fn pending_objects(plan: &Plan) -> Vec<PendingObject> {
    let mut objects: Vec<PendingObject> = Vec::new();
    for action in &plan.actions {
        let Some(install) = &action.install else { continue };
        if let Some(existing) = objects.iter_mut().find(|object| object.sha256 == install.sha256) {
            existing.uses += 1;
            continue;
        }
        objects.push(PendingObject {
            sha256: install.sha256.clone(),
            size: install.size,
            label: action.path.clone(),
            uses: 1,
        });
    }
    objects
}

async fn download_all(ctx: &Context, plan: &Plan, reporter: &Arc<Reporter>) -> Result<(), String> {
    let downloads = ctx.sync_dir.join(DOWNLOADS_DIR);
    tokio::fs::create_dir_all(&downloads)
        .await
        .map_err(|error| format!("无法创建下载文件夹：{}", engine::describe_io_error(&error)))?;
    let objects = pending_objects(plan);
    let progress = Arc::new(DownloadProgress {
        reporter: Arc::clone(reporter),
        done_files: AtomicU64::new(0),
        total_files: plan.files_to_download(),
        done_bytes: AtomicU64::new(0),
        total_bytes: objects.iter().map(|object| object.size).sum(),
        current: Mutex::new(None),
    });
    // Empty files need no download.
    let (empty, objects): (Vec<_>, Vec<_>) = objects.into_iter().partition(|object| object.size == 0);
    progress
        .done_files
        .fetch_add(empty.iter().map(|object| object.uses).sum(), Ordering::Relaxed);
    progress.tick(true);

    let client = crate::util::fetch::configured_client()
        .await
        .map_err(|error| error.to_string())?;
    futures::stream::iter(objects.into_iter().map(Ok::<_, String>))
        .try_for_each_concurrent(PARALLEL_DOWNLOADS, |object| {
            let client = client.clone();
            let progress = Arc::clone(&progress);
            let downloads = downloads.clone();
            let base_url = ctx.base_url.clone();
            async move {
                download_object(&client, &base_url, &object, &downloads, &progress).await?;
                progress.done_files.fetch_add(object.uses, Ordering::Relaxed);
                progress.tick(false);
                Ok(())
            }
        })
        .await?;
    progress.tick(true);
    Ok(())
}

// ---- check / apply -------------------------------------------------------------

/// A verified release compared with the local files.
struct Prepared {
    ctx: Context,
    raw: Arc<Vec<u8>>,
    signature: String,
    sha256: String,
    manifest: Manifest,
    plan: Plan,
    index: FileIndex,
    local_version: Option<String>,
    highest: (u64, Option<String>),
    hashed_files: u64,
    hashed_bytes: u64,
}

fn verified_from_cache(ctx: &Context) -> Option<(Arc<Vec<u8>>, String)> {
    let slot = VERIFIED.lock().ok()?;
    let cached = slot.as_ref()?;
    (cached.fetched.elapsed() < MANIFEST_REUSE
        && cached.base_url == ctx.base_url
        && cached.project_id == ctx.project_id)
        .then(|| (Arc::clone(&cached.raw), cached.signature.clone()))
}

/// The newest release this install has proof of: the state's record or the
/// bundled baseline, whichever has the higher sequence.
fn highest_seen(state: Option<&SyncState>, baseline: Option<&SignedCopy>) -> (u64, Option<String>) {
    let from_state = state.map(|state| (state.highest_sequence, state.highest_manifest_sha256.clone()));
    let from_baseline = baseline.map(|copy| (copy.manifest.sequence, Some(copy.sha256.clone())));
    match (from_state, from_baseline) {
        (Some(state), Some(baseline)) if baseline.0 > state.0 => baseline,
        (Some(state), _) => state,
        (None, Some(baseline)) => baseline,
        (None, None) => (0, None),
    }
}

async fn prepare(
    force_rehash: bool,
    reporter: &Arc<Reporter>,
    reuse_manifest: bool,
) -> Result<Prepared, GalaxySyncResult> {
    reporter.phase("checking", None);
    let ctx = load_context()?;

    let mut state = load_state(&ctx.sync_dir);
    let copy = read_signed(
        &ctx,
        &ctx.sync_dir.join(MANIFEST_COPY),
        &ctx.sync_dir.join(SIGNATURE_COPY),
    );
    let copy = match copy {
        CopyCheck::Invalid { sha256 }
            if state.as_ref().and_then(|state| state.manifest_sha256.as_deref())
                == Some(sha256.as_str()) =>
        {
            // The bytes are the ones that were applied, but the key no longer
            // matches (the binding was changed): forget the old record.
            tracing::warn!("Galaxy sync: the applied manifest copy does not verify with the current key; re-baselining");
            state = state.map(|old| SyncState {
                initialized_paths: old.initialized_paths,
                applied_corrections: old.applied_corrections,
                ..SyncState::default()
            });
            None
        }
        CopyCheck::Invalid { .. } if force_rehash => {
            tracing::warn!("Galaxy sync: the applied manifest copy was modified; 修复游戏文件 starts over");
            let _ = std::fs::remove_file(ctx.sync_dir.join(MANIFEST_COPY));
            let _ = std::fs::remove_file(ctx.sync_dir.join(SIGNATURE_COPY));
            state = state.map(|old| SyncState {
                initialized_paths: old.initialized_paths,
                applied_corrections: old.applied_corrections,
                ..SyncState::default()
            });
            None
        }
        CopyCheck::Invalid { .. } => {
            tracing::warn!("Galaxy sync: the applied manifest copy failed signature verification");
            return Err(GalaxySyncResult::failure(
                SyncStatus::Blocked,
                "LOCAL_STATE_INVALID",
                "本地的整合包记录校验失败（可能被修改或损坏），已停止更新。请点「修复游戏文件」重新检查。",
            ));
        }
        CopyCheck::Valid(copy) => Some(copy),
        CopyCheck::Missing => None,
    };
    let baseline = bundled_baseline(&ctx);
    let local_version = state
        .as_ref()
        .and_then(|state| state.display_version.clone())
        .or_else(|| copy.as_ref().and_then(|copy| copy.manifest.display_version.clone()))
        .or_else(|| baseline.as_ref().and_then(|copy| copy.manifest.display_version.clone()));
    let highest = highest_seen(state.as_ref(), baseline.as_ref());

    let cached = if reuse_manifest { verified_from_cache(&ctx) } else { None };
    let (raw, signature) = match cached {
        Some(cached) => cached,
        None => match fetch_latest(&ctx).await {
            Ok((raw, signature)) => (Arc::new(raw), signature),
            Err(FetchError::Offline(message)) => {
                tracing::info!(base_url = %ctx.base_url, %message, "Galaxy sync: update server unreachable");
                let mut result = GalaxySyncResult::failure(
                    SyncStatus::Offline,
                    "OFFLINE",
                    format!("暂时连不上更新服务器，将使用本地整合包（{message}）"),
                );
                result.local_version = local_version;
                return Err(result);
            }
            Err(FetchError::Blocked(code, message)) => {
                return Err(GalaxySyncResult::failure(SyncStatus::Blocked, code, message));
            }
        },
    };
    if !engine::verify_signature(&ctx.key, &raw, &signature) {
        tracing::warn!(base_url = %ctx.base_url, bytes = raw.len(), "Galaxy sync: manifest signature verification failed");
        let mut result = GalaxySyncResult::failure(
            SyncStatus::Blocked,
            "SIGNATURE_INVALID",
            "整合包清单的签名校验失败（可能被篡改），已停止更新",
        );
        result.local_version = local_version;
        return Err(result);
    }
    let sha256 = sha256_hex(&raw);
    let blocked = |code: &str, message: String| {
        let mut result = GalaxySyncResult::failure(SyncStatus::Blocked, code, message);
        result.local_version.clone_from(&local_version);
        result
    };
    let manifest =
        engine::parse_manifest(&raw).map_err(|message| blocked("MANIFEST_INVALID", message))?;
    let release = engine::validate(&manifest, &ctx.project_id)
        .map_err(|rejection| blocked(rejection.code(), rejection.message()))?;
    engine::check_replay(manifest.sequence, &sha256, highest.0, highest.1.as_deref())
        .map_err(|message| blocked("REPLAY", message))?;
    tracing::info!(
        release = %manifest.release_id,
        sequence = manifest.sequence,
        version = manifest.display_version.as_deref().unwrap_or(""),
        bytes = raw.len(),
        %sha256,
        "Galaxy sync: release manifest verified"
    );
    if let Ok(mut slot) = VERIFIED.lock() {
        *slot = Some(VerifiedManifest {
            fetched: Instant::now(),
            base_url: ctx.base_url.clone(),
            project_id: ctx.project_id.clone(),
            raw: Arc::clone(&raw),
            signature: signature.clone(),
        });
    }

    let root = ctx.instance_dir.clone();
    let index = load_index(&ctx.sync_dir);
    let state_for_plan = state.unwrap_or_default();
    let hashing = Arc::clone(reporter);
    let planned = tokio::task::spawn_blocking(move || {
        let mut index = index;
        let report = |progress: HashProgress| {
            hashing.send(
                GalaxySyncProgress {
                    phase: "hashing",
                    done_files: progress.done_files,
                    total_files: progress.total_files,
                    done_bytes: progress.done_bytes,
                    total_bytes: progress.total_bytes,
                    current_file: None,
                    message: None,
                },
                progress.done_files == 0 || progress.done_files == progress.total_files,
            );
        };
        let mut local = LocalFiles::new(&root, &mut index, force_rehash, &report);
        let plan = engine::build_plan(&release, &state_for_plan, &mut local);
        let hashed = (local.hashed_files, local.hashed_bytes);
        local.finish();
        (plan, index, hashed)
    })
    .await;
    let (plan, index, (hashed_files, hashed_bytes)) = planned.map_err(|error| {
        GalaxySyncResult::failure(SyncStatus::Error, "CHECK_FAILED", format!("检查整合包文件失败：{error}"))
    })?;
    for warning in &plan.warnings {
        tracing::warn!(%warning, "Galaxy sync: plan warning");
    }
    if let Err(error) = save_json(&ctx.sync_dir.join(INDEX_FILE), &index) {
        tracing::warn!(%error, "Galaxy sync: could not save the hash cache");
    }
    Ok(Prepared {
        ctx,
        raw,
        signature,
        sha256,
        manifest,
        plan,
        index,
        local_version,
        highest,
        hashed_files,
        hashed_bytes,
    })
}

fn summarize(prepared: &Prepared, status: SyncStatus) -> GalaxySyncResult {
    let mut result = GalaxySyncResult::new(status);
    let manifest = &prepared.manifest;
    result.local_version.clone_from(&prepared.local_version);
    result.remote_version.clone_from(&manifest.display_version);
    result.release_id = Some(manifest.release_id.clone());
    result.sequence = Some(manifest.sequence);
    result.changelog.clone_from(&manifest.changelog);
    result.released_at.clone_from(&manifest.created_at);
    result.files_to_download = prepared.plan.files_to_download();
    result.bytes_to_download = prepared.plan.bytes_to_download();
    result.files_to_remove = prepared.plan.files_to_remove();
    result.hashed_files = prepared.hashed_files;
    result
}

/// Records the applied release: signed manifest copy, state.json and the
/// hash cache (installed files are known, removed ones are gone).
fn commit(prepared: &Prepared, report: &InstallReport) -> Result<(), String> {
    let ctx = &prepared.ctx;
    let manifest = &prepared.manifest;
    let copy_path = ctx.sync_dir.join(MANIFEST_COPY);
    let copy_current = std::fs::read(&copy_path)
        .is_ok_and(|bytes| sha256_hex(&bytes) == prepared.sha256)
        && ctx.sync_dir.join(SIGNATURE_COPY).is_file();
    if !copy_current {
        write_atomic(&copy_path, &prepared.raw)
            .map_err(|error| format!("无法保存整合包清单：{error}"))?;
        write_atomic(
            &ctx.sync_dir.join(SIGNATURE_COPY),
            format!("{}\n", prepared.signature.trim()).as_bytes(),
        )
        .map_err(|error| format!("无法保存整合包清单签名：{error}"))?;
    }
    let (highest_sequence, highest_sha) = if manifest.sequence >= prepared.highest.0 {
        (manifest.sequence, Some(prepared.sha256.clone()))
    } else {
        prepared.highest.clone()
    };
    let state = SyncState {
        schema_version: 1,
        project_id: manifest.project_id.clone(),
        release_id: Some(manifest.release_id.clone()),
        sequence: manifest.sequence,
        manifest_sha256: Some(prepared.sha256.clone()),
        highest_sequence,
        highest_manifest_sha256: highest_sha,
        display_version: manifest.display_version.clone(),
        files: prepared.plan.files.clone(),
        initialized_paths: prepared.plan.initialized_paths.clone(),
        applied_corrections: prepared.plan.applied_corrections.clone(),
        synced_at: Some(now_rfc3339()),
    };
    save_json(&ctx.sync_dir.join(STATE_FILE), &state)?;

    let mut index = prepared.index.clone();
    for item in &report.backed_up {
        index.remove(&path_key(&item.path));
    }
    for file in &report.installed {
        if let Some((size, mtime)) = stat_file(&absolute(&ctx.instance_dir, &file.path)) {
            index.insert(
                path_key(&file.path),
                IndexEntry {
                    size,
                    mtime_ms: mtime,
                    sha256: file.sha256.clone(),
                },
            );
        }
    }
    if let Err(error) = save_json(&ctx.sync_dir.join(INDEX_FILE), &index) {
        tracing::warn!(%error, "Galaxy sync: could not save the hash cache");
    }
    Ok(())
}

fn log_result(action: &str, result: &GalaxySyncResult, started: Instant) {
    tracing::info!(
        action,
        status = ?result.status,
        code = result.code.as_deref().unwrap_or(""),
        local = result.local_version.as_deref().unwrap_or(""),
        remote = result.remote_version.as_deref().unwrap_or(""),
        release = result.release_id.as_deref().unwrap_or(""),
        download_files = result.files_to_download,
        download_bytes = result.bytes_to_download,
        remove_files = result.files_to_remove,
        hashed_files = result.hashed_files,
        installed = result.files_installed,
        backed_up = result.files_backed_up,
        backup_dir = result.backup_dir.as_deref().unwrap_or(""),
        launcher_update_changed = result.launcher_update_changed,
        millis = started.elapsed().as_millis() as u64,
        message = result.message.as_deref().unwrap_or(""),
        "Galaxy sync: {action} finished"
    );
}

fn finish(reporter: &Reporter, result: &GalaxySyncResult) {
    if let Ok(mut slot) = LAST_RESULT.lock() {
        *slot = Some(result.clone());
    }
    let phase = if matches!(result.status, SyncStatus::Blocked | SyncStatus::Error) {
        "error"
    } else {
        "done"
    };
    reporter.phase(phase, result.message.clone());
}

/// Fetches and verifies the server's release and compares it with the
/// instance (`force_rehash`: ignore the hash cache, for 修复游戏文件). When
/// nothing has to change the release is recorded right away.
#[tracing::instrument]
pub async fn check(force_rehash: bool) -> crate::Result<GalaxySyncResult> {
    let _guard = SYNC_LOCK.try_lock().map_err(|_| busy_error())?;
    let started = Instant::now();
    tracing::info!(force_rehash, "Galaxy sync: check started");
    let reporter = Arc::new(Reporter::default());
    let result = match prepare(force_rehash, &reporter, false).await {
        Err(result) => result,
        Ok(prepared) => {
            let mut result = if prepared.plan.actions.is_empty() {
                // A few small files; fine on the async thread.
                match commit(&prepared, &InstallReport::default()) {
                    Ok(()) => {
                        let mut result = summarize(&prepared, SyncStatus::UpToDate);
                        result.local_version.clone_from(&prepared.manifest.display_version);
                        result
                    }
                    Err(message) => GalaxySyncResult::failure(SyncStatus::Error, "STATE_WRITE_FAILED", message),
                }
            } else {
                for action in prepared.plan.actions.iter().take(20) {
                    tracing::info!(
                        path = %action.path,
                        backup = action.backup.as_deref().unwrap_or(""),
                        install = action.install.is_some(),
                        "Galaxy sync: planned"
                    );
                }
                summarize(&prepared, SyncStatus::UpdateAvailable)
            };
            if prepared.hashed_files > 0 {
                tracing::info!(files = prepared.hashed_files, bytes = prepared.hashed_bytes, "Galaxy sync: hashed local files");
            }
            result.history = fetch_history(&prepared.ctx.base_url, &prepared.ctx.project_id).await;
            result
        }
    };
    log_result("check", &result, started);
    finish(&reporter, &result);
    Ok(result)
}

async fn game_running() -> bool {
    crate::api::process::get_all()
        .await
        .is_ok_and(|processes| !processes.is_empty())
}

fn available_space(path: &Path) -> Option<u64> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut best: Option<(usize, u64)> = None;
    for disk in &disks {
        let mount = disk.mount_point();
        if path.starts_with(mount) {
            let length = mount.as_os_str().len();
            if best.is_none_or(|(best_length, _)| length > best_length) {
                best = Some((length, disk.available_space()));
            }
        }
    }
    best.map(|(_, space)| space)
}

/// Checks again (reusing a manifest verified moments ago), downloads every
/// object, then swaps the files in with backups and rollback.
#[tracing::instrument]
pub async fn apply() -> crate::Result<GalaxySyncResult> {
    let _guard = SYNC_LOCK.try_lock().map_err(|_| busy_error())?;
    if game_running().await {
        return Err(crate::ErrorKind::InputError("请先关闭游戏，再更新整合包".to_string()).as_error());
    }
    let started = Instant::now();
    tracing::info!("Galaxy sync: apply started");
    let reporter = Arc::new(Reporter::default());
    match apply_locked(&reporter).await {
        Ok(result) => {
            log_result("apply", &result, started);
            finish(&reporter, &result);
            Ok(result)
        }
        Err(message) => {
            let result = GalaxySyncResult::failure(SyncStatus::Error, "APPLY_FAILED", message.clone());
            log_result("apply", &result, started);
            finish(&reporter, &result);
            Err(other_error(message))
        }
    }
}

async fn apply_locked(reporter: &Arc<Reporter>) -> Result<GalaxySyncResult, String> {
    let prepared = match prepare(false, reporter, true).await {
        Ok(prepared) => prepared,
        Err(result) if result.status == SyncStatus::Error => {
            return Err(result.message.unwrap_or_default());
        }
        Err(result) => return Ok(result),
    };
    if prepared.plan.actions.is_empty() {
        commit(&prepared, &InstallReport::default())?;
        let mut result = summarize(&prepared, SyncStatus::UpToDate);
        result.local_version.clone_from(&prepared.manifest.display_version);
        return Ok(result);
    }

    let needed = prepared.plan.bytes_to_download();
    if let Some(free) = available_space(&prepared.ctx.instance_dir)
        && free < needed + DISK_MARGIN
    {
        return Err(format!(
            "磁盘空间不足：更新需要约 {} MB，游戏所在磁盘只剩 {} MB",
            (needed + DISK_MARGIN) / 1024 / 1024,
            free / 1024 / 1024
        ));
    }
    tracing::info!(
        files = prepared.plan.files_to_download(),
        bytes = needed,
        remove = prepared.plan.files_to_remove(),
        "Galaxy sync: downloading"
    );
    download_all(&prepared.ctx, &prepared.plan, reporter).await?;

    reporter.phase("applying", None);
    let ctx = &prepared.ctx;
    let backup_dir = engine::unique_dir(
        &ctx.sync_dir.join(BACKUPS_DIR),
        &format!(
            "{}_{}",
            chrono::Local::now().format("%Y%m%d-%H%M%S"),
            engine::folder_safe(&prepared.manifest.release_id)
        ),
    );
    let downloads = ctx.sync_dir.join(DOWNLOADS_DIR);
    let report = {
        let instance_dir = ctx.instance_dir.clone();
        let downloads = downloads.clone();
        let backup_dir = backup_dir.clone();
        let release_id = prepared.manifest.release_id.clone();
        let actions = prepared.plan.actions.clone();
        tokio::task::spawn_blocking(move || {
            engine::install(
                &instance_dir,
                &downloads,
                &backup_dir,
                &release_id,
                &now_rfc3339(),
                &actions,
            )
        })
        .await
        .map_err(|error| format!("安装更新失败：{error}"))??
    };
    tracing::info!(
        installed = report.installed.len(),
        backed_up = report.backed_up.len(),
        backup_dir = %report.backup_dir.as_ref().map(|path| path.display().to_string()).unwrap_or_default(),
        "Galaxy sync: files installed"
    );
    commit(&prepared, &report)
        .map_err(|message| format!("更新已经装好，但保存更新记录失败：{message}"))?;
    let pruned = engine::prune_backups(&ctx.sync_dir.join(BACKUPS_DIR), KEEP_BACKUPS);
    if !pruned.is_empty() {
        tracing::info!(?pruned, "Galaxy sync: removed old backups");
    }
    engine::clear_dir_files(&downloads);

    let mut result = summarize(&prepared, SyncStatus::UpToDate);
    result.local_version.clone_from(&prepared.manifest.display_version);
    result.applied = true;
    result.files_installed = report.installed.len() as u64;
    result.files_backed_up = report.backed_up.len() as u64;
    result.backup_dir = report
        .backup_dir
        .as_deref()
        .map(crate::api::galaxy::display_path);
    let is_launcher_update = |path: &str| path_key(path).starts_with("launcher-update/");
    result.launcher_update_changed = report.installed.iter().any(|file| is_launcher_update(&file.path))
        || report.backed_up.iter().any(|item| is_launcher_update(&item.path));
    Ok(result)
}
