//! Galaxy宝可梦 built-in updater: release-manifest validation, the plan rules
//! and the local file operations (hash cache, install with rollback, backups).
//!
//! No network and no `crate::` items in here, so the rules can be unit-tested
//! without building the whole app-lib (see GALAXY_FORK.md, "Built-in updater
//! (galaxy_sync)"). Paths in manifests and state files use `/` and are
//! relative to the instance directory.

use base64::Engine as _;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering};
use std::time::UNIX_EPOCH;

/// DreamingFish player version whose behaviour this client implements.
pub const PLAYER_COMPAT_VERSION: &str = "0.3.2";
/// `requiredCapabilities` this client understands; anything else means the
/// release needs a newer launcher.
pub const SUPPORTED_CAPABILITIES: &[&str] = &[
    "forced-directory-sync-v1",
    "forced-file-sync-v1",
    "released-paths-v1",
    "maintenance-policy-v2",
    "simplified-maintenance-v1",
];
/// Launcher-owned state directory inside the instance.
pub const STATE_DIR: &str = ".galaxy-sync";
/// DreamingFish bootstrap directory (binding + bundled baseline).
pub const BOOTSTRAP_DIR: &str = ".dreamingfish-bootstrap";
pub const EMPTY_SHA256: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
/// X.509 SubjectPublicKeyInfo prefix of an Ed25519 key (12 bytes + 32 key bytes).
const SPKI_ED25519_PREFIX: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];
const MAX_FILE_SIZE: u64 = 16 * 1024 * 1024 * 1024;
const MAX_PATH_LEN: usize = 1024;
const HASH_WORKERS: usize = 4;
#[cfg(windows)]
const CROSS_DEVICE_ERROR: i32 = 17; // ERROR_NOT_SAME_DEVICE
#[cfg(not(windows))]
const CROSS_DEVICE_ERROR: i32 = 18; // EXDEV

// ---- crypto -------------------------------------------------------------------

/// Accepts the binding's base64 X.509 SubjectPublicKeyInfo DER (44 bytes) or a
/// raw 32-byte key.
pub fn parse_public_key(text: &str) -> Result<VerifyingKey, String> {
    let der = base64::engine::general_purpose::STANDARD
        .decode(text.trim())
        .map_err(|_| "公钥不是有效的 base64".to_string())?;
    let raw = if der.len() == 44 && der[..12] == SPKI_ED25519_PREFIX {
        &der[12..]
    } else if der.len() == 32 {
        &der[..]
    } else {
        return Err("公钥不是 Ed25519 公钥".to_string());
    };
    let raw = <[u8; 32]>::try_from(raw).map_err(|_| "公钥长度不对".to_string())?;
    VerifyingKey::from_bytes(&raw).map_err(|_| "公钥无效".to_string())
}

/// Strict Ed25519 verification over the exact bytes (never re-serialized).
pub fn verify_signature(key: &VerifyingKey, body: &[u8], signature_base64: &str) -> bool {
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(signature_base64.trim())
    else {
        return false;
    };
    let Ok(bytes) = <[u8; 64]>::try_from(bytes.as_slice()) else {
        return false;
    };
    key.verify_strict(body, &Signature::from_bytes(&bytes)).is_ok()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_sha256_hex(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Numeric comparison of dotted versions (`0.3.10` > `0.3.2`); missing and
/// non-numeric parts count as 0.
pub fn compare_versions(left: &str, right: &str) -> Ordering {
    let parse = |text: &str| -> Vec<u64> {
        text.trim()
            .trim_start_matches(['v', 'V'])
            .split(['.', '-', '+'])
            .map(|part| part.parse().unwrap_or(0))
            .collect()
    };
    let (left, right) = (parse(left), parse(right));
    for index in 0..left.len().max(right.len()) {
        let ordering = left
            .get(index)
            .unwrap_or(&0)
            .cmp(right.get(index).unwrap_or(&0));
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    Ordering::Equal
}

// ---- paths --------------------------------------------------------------------

fn is_reserved_device_name(segment: &str) -> bool {
    let stem = segment
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    match stem.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$" => true,
        _ => {
            let bytes = stem.as_bytes();
            bytes.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && (b'1'..=b'9').contains(&bytes[3])
        }
    }
}

/// Validates a manifest path and returns it unchanged. Rejects absolute
/// paths, drive letters / `:` (also NTFS streams), `\`, `.` / `..` and empty
/// segments, characters Windows cannot store, trailing dots or spaces (Windows
/// strips them, so two paths could alias), device names, and anything inside
/// the launcher's own `.galaxy-sync` / `.dreamingfish-bootstrap` directories.
pub fn safe_relative_path(raw: &str) -> Result<String, String> {
    let fail = |why: &str| Err(format!("{why}：{raw}"));
    if raw.is_empty() || raw.len() > MAX_PATH_LEN {
        return fail("路径为空或过长");
    }
    if raw.starts_with('/') || raw.contains('\\') || raw.contains(':') {
        return fail("不允许绝对路径或盘符");
    }
    if raw
        .chars()
        .any(|c| c.is_control() || matches!(c, '<' | '>' | '"' | '|' | '?' | '*'))
    {
        return fail("路径含有非法字符");
    }
    for segment in raw.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return fail("路径不能包含空段、. 或 ..");
        }
        if segment.ends_with('.') || segment.ends_with(' ') {
            return fail("路径段不能以点或空格结尾");
        }
        if is_reserved_device_name(segment) {
            return fail("路径使用了系统保留名");
        }
    }
    let first = raw.split('/').next().unwrap_or("").to_ascii_lowercase();
    if first == STATE_DIR || first == BOOTSTRAP_DIR {
        return fail("不能写入启动器自己的目录");
    }
    Ok(raw.to_string())
}

/// Comparison key: Windows file names are case-insensitive.
pub fn path_key(path: &str) -> String {
    if cfg!(windows) {
        path.to_lowercase()
    } else {
        path.to_string()
    }
}

/// `root` joined with a validated `/`-separated relative path.
pub fn absolute(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in relative.split('/') {
        path.push(segment);
    }
    path
}

fn under(key: &str, prefix: &str) -> bool {
    key == prefix
        || (key.len() > prefix.len()
            && key.starts_with(prefix)
            && key.as_bytes()[prefix.len()] == b'/')
}

/// Keeps `[A-Za-z0-9._-]` (for backup folder names built from a release id).
pub fn folder_safe(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .take(80)
        .collect();
    cleaned.trim_matches('.').to_string()
}

/// `parent/base`, or `parent/base-2`, `-3`, … when that already exists.
pub fn unique_dir(parent: &Path, base: &str) -> PathBuf {
    let first = parent.join(base);
    if !first.exists() {
        return first;
    }
    (2..1000)
        .map(|n| parent.join(format!("{base}-{n}")))
        .find(|candidate| !candidate.exists())
        .unwrap_or(first)
}

// ---- release manifest -----------------------------------------------------------

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub schema_version: u32,
    pub project_id: String,
    pub release_id: String,
    pub sequence: u64,
    #[serde(default)]
    pub display_version: Option<String>,
    #[serde(default)]
    pub changelog: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub minimum_player_version: Option<String>,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    #[serde(default)]
    pub files: Vec<ManifestFile>,
    #[serde(default)]
    pub withdrawals: Vec<Withdrawal>,
    #[serde(default)]
    pub corrections: Vec<Correction>,
    #[serde(default)]
    pub optional_groups: Vec<OptionalGroup>,
    #[serde(default)]
    pub forced_sync_directories: Vec<serde_json::Value>,
    #[serde(default)]
    pub forced_sync_files: Vec<serde_json::Value>,
    #[serde(default)]
    pub cleanup_directories: Vec<serde_json::Value>,
    #[serde(default)]
    pub released_paths: Vec<serde_json::Value>,
    #[serde(default)]
    pub retained_self_managed_paths: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    #[serde(default)]
    pub policy: Option<String>,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub optional_group: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Withdrawal {
    // `kind` (REMOVAL / VERSION) does not change what happens to a copy.
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub items: Vec<WithdrawalItem>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalItem {
    pub path: String,
    pub sha256: String,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Correction {
    #[serde(default)]
    pub id: Option<String>,
    pub path: String,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default, rename = "badSha256")]
    pub bad_sha256: Vec<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionalGroup {
    pub id: String,
    #[serde(default = "default_true")]
    pub default_install: bool,
}

fn default_true() -> bool {
    true
}

pub fn parse_manifest(raw: &[u8]) -> Result<Manifest, String> {
    let text = raw.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(raw);
    serde_json::from_slice(text).map_err(|error| format!("整合包清单格式错误：{error}"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rejection {
    PathUnsafe(String),
    Invalid(String),
    NeedsNewerLauncher(String),
    ProjectMismatch(String),
}

impl Rejection {
    pub fn code(&self) -> &'static str {
        match self {
            Self::PathUnsafe(_) => "PATH_UNSAFE",
            Self::Invalid(_) => "MANIFEST_INVALID",
            Self::NeedsNewerLauncher(_) => "NEEDS_NEWER_LAUNCHER",
            Self::ProjectMismatch(_) => "PROJECT_MISMATCH",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::PathUnsafe(detail) => {
                format!("整合包清单里有不安全的文件路径，已停止更新（{detail}）")
            }
            Self::Invalid(detail) => format!("整合包清单无效：{detail}"),
            Self::NeedsNewerLauncher(detail) => {
                format!("这个整合包版本需要更新的启动器（{detail}），请下载最新的启动器")
            }
            Self::ProjectMismatch(detail) => {
                format!("更新服务器返回的不是本服的整合包（{detail}）")
            }
        }
    }
}

/// Video / shader settings the game rewrites whenever the player changes
/// them in-game (and the home page's 画质档位 edits). The release ships them
/// as SYNC, which reset the player's settings before every launch; they are
/// handled like INITIAL instead. `forcedSyncFiles` and corrections still
/// apply, so the server owner can push a new copy on purpose.
const PLAYER_PREFERENCE_FILES: &[&str] = &["config/iris.properties", "config/sodium-options.json"];

fn is_player_preference(key: &str) -> bool {
    PLAYER_PREFERENCE_FILES.iter().any(|path| path_key(path) == key)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Kept identical to the release (SYNC, REQUIRED, forcedSyncFiles).
    Sync,
    /// Provided once, then left to the player (INITIAL, DEFAULT_CONFIG,
    /// policy LEGACY_MISSING_ONLY).
    Initial,
}

#[derive(Clone, Debug)]
pub struct FileSpec {
    pub path: String,
    pub key: String,
    pub sha256: String,
    pub size: u64,
    pub mode: Mode,
}

#[derive(Clone, Debug)]
pub struct WithdrawnCopy {
    pub path: String,
    pub key: String,
    pub sha256: String,
    pub size: Option<u64>,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorrectionMode {
    KnownBad,
    Once,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct CorrectionSpec {
    pub id: String,
    pub path: String,
    pub key: String,
    pub mode: CorrectionMode,
    pub bad: HashSet<String>,
    pub reason: String,
}

/// A verified manifest reduced to what the plan rules need, every path
/// validated.
#[derive(Clone, Debug, Default)]
pub struct Release {
    /// Files to install; optional groups that are off by default and
    /// released / self-managed paths are already left out.
    pub files: Vec<FileSpec>,
    pub withdrawals: Vec<WithdrawnCopy>,
    pub corrections: Vec<CorrectionSpec>,
    /// forcedSyncDirectories + cleanupDirectories.
    pub cleanup_dirs: Vec<String>,
    /// releasedPaths + retainedSelfManagedPaths (keys): never touched.
    pub unmanaged: Vec<String>,
}

impl Release {
    pub fn is_unmanaged(&self, key: &str) -> bool {
        self.unmanaged.iter().any(|prefix| under(key, prefix))
    }
}

fn value_paths(values: &[serde_json::Value], field: &str) -> Result<Vec<String>, Rejection> {
    let mut out = Vec::new();
    for value in values {
        let path = match value {
            serde_json::Value::String(text) => Some(text.as_str()),
            serde_json::Value::Object(map) => map
                .get("path")
                .or_else(|| map.get("directory"))
                .and_then(serde_json::Value::as_str),
            _ => None,
        }
        .ok_or_else(|| Rejection::Invalid(format!("{field} 的格式无法识别")))?;
        out.push(safe_relative_path(path.trim_end_matches('/')).map_err(Rejection::PathUnsafe)?);
    }
    Ok(out)
}

fn group_id(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) if !text.is_empty() => Some(text.clone()),
        serde_json::Value::Object(map) => map
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        _ => None,
    }
}

/// Checks a verified manifest against this client (schema, project,
/// capabilities, minimum player version, paths, hashes) and applies the
/// optional-group / released-path filters.
pub fn validate(manifest: &Manifest, project_id: &str) -> Result<Release, Rejection> {
    if manifest.schema_version != 1 {
        return Err(Rejection::NeedsNewerLauncher(format!(
            "清单格式 {}",
            manifest.schema_version
        )));
    }
    if manifest.project_id != project_id {
        return Err(Rejection::ProjectMismatch(format!(
            "{} ≠ {project_id}",
            manifest.project_id
        )));
    }
    if let Some(missing) = manifest
        .required_capabilities
        .iter()
        .find(|capability| !SUPPORTED_CAPABILITIES.contains(&capability.as_str()))
    {
        return Err(Rejection::NeedsNewerLauncher(format!("需要功能 {missing}")));
    }
    if let Some(minimum) = manifest
        .minimum_player_version
        .as_deref()
        .map(str::trim)
        .filter(|minimum| !minimum.is_empty())
        && compare_versions(minimum, PLAYER_COMPAT_VERSION) == Ordering::Greater
    {
        return Err(Rejection::NeedsNewerLauncher(format!(
            "需要更新器 {minimum}，当前兼容 {PLAYER_COMPAT_VERSION}"
        )));
    }
    if manifest.release_id.trim().is_empty() {
        return Err(Rejection::Invalid("缺少 releaseId".to_string()));
    }

    let mut release = Release::default();
    for path in value_paths(&manifest.released_paths, "releasedPaths")?
        .into_iter()
        .chain(value_paths(
            &manifest.retained_self_managed_paths,
            "retainedSelfManagedPaths",
        )?)
    {
        release.unmanaged.push(path_key(&path));
    }
    let forced_files: HashSet<String> = value_paths(&manifest.forced_sync_files, "forcedSyncFiles")?
        .iter()
        .map(|path| path_key(path))
        .collect();
    release.cleanup_dirs = value_paths(&manifest.forced_sync_directories, "forcedSyncDirectories")?;
    release
        .cleanup_dirs
        .extend(value_paths(&manifest.cleanup_directories, "cleanupDirectories")?);
    let disabled_groups: HashSet<&str> = manifest
        .optional_groups
        .iter()
        .filter(|group| !group.default_install)
        .map(|group| group.id.as_str())
        .collect();

    let mut seen = HashSet::new();
    for file in &manifest.files {
        let path = safe_relative_path(&file.path).map_err(Rejection::PathUnsafe)?;
        let key = path_key(&path);
        if !seen.insert(key.clone()) {
            return Err(Rejection::Invalid(format!("文件重复：{path}")));
        }
        if !is_sha256_hex(&file.sha256) {
            return Err(Rejection::Invalid(format!("SHA-256 无效：{path}")));
        }
        let sha256 = file.sha256.to_ascii_lowercase();
        if file.size > MAX_FILE_SIZE || (file.size == 0 && sha256 != EMPTY_SHA256) {
            return Err(Rejection::Invalid(format!("文件大小无效：{path}")));
        }
        let off = file
            .optional_group
            .as_ref()
            .and_then(group_id)
            .is_some_and(|group| disabled_groups.contains(group.as_str()));
        if off || release.is_unmanaged(&key) {
            continue;
        }
        let preset = file.preset.as_deref().unwrap_or("SYNC");
        let mode = if forced_files.contains(&key) {
            Mode::Sync
        } else if file.policy.as_deref() == Some("LEGACY_MISSING_ONLY")
            || matches!(preset, "INITIAL" | "DEFAULT_CONFIG")
            || is_player_preference(&key)
        {
            Mode::Initial
        } else {
            Mode::Sync
        };
        release.files.push(FileSpec {
            path,
            key,
            sha256,
            size: file.size,
            mode,
        });
    }

    for withdrawal in &manifest.withdrawals {
        let reason = withdrawal
            .reason
            .clone()
            .filter(|reason| !reason.trim().is_empty())
            .unwrap_or_else(|| "服主已撤回这个文件".to_string());
        for item in &withdrawal.items {
            let path = safe_relative_path(&item.path).map_err(Rejection::PathUnsafe)?;
            if !is_sha256_hex(&item.sha256) {
                return Err(Rejection::Invalid(format!("撤回项的 SHA-256 无效：{path}")));
            }
            release.withdrawals.push(WithdrawnCopy {
                key: path_key(&path),
                path,
                sha256: item.sha256.to_ascii_lowercase(),
                size: item.size,
                reason: reason.clone(),
            });
        }
    }

    for (index, correction) in manifest.corrections.iter().enumerate() {
        let path = safe_relative_path(&correction.path).map_err(Rejection::PathUnsafe)?;
        let mode = match correction.mode.as_deref() {
            Some("KNOWN_BAD") => CorrectionMode::KnownBad,
            Some("ONCE") => CorrectionMode::Once,
            _ => CorrectionMode::Unknown,
        };
        release.corrections.push(CorrectionSpec {
            id: correction
                .id
                .clone()
                .filter(|id| !id.is_empty())
                .unwrap_or_else(|| format!("correction-{index}-{path}")),
            key: path_key(&path),
            path,
            mode,
            bad: correction
                .bad_sha256
                .iter()
                .map(|sha| sha.to_ascii_lowercase())
                .collect(),
            reason: correction
                .reason
                .clone()
                .filter(|reason| !reason.trim().is_empty())
                .unwrap_or_else(|| "服主标记为需要修正".to_string()),
        });
    }
    Ok(release)
}

/// Anti-replay: a release must not be older than the newest one seen, and the
/// same sequence must be the same manifest.
pub fn check_replay(
    sequence: u64,
    sha256: &str,
    highest_sequence: u64,
    highest_sha256: Option<&str>,
) -> Result<(), String> {
    if sequence < highest_sequence {
        return Err(format!(
            "服务器返回的版本比本地旧（{sequence} < {highest_sequence}），可能被篡改，已停止更新"
        ));
    }
    if sequence == highest_sequence
        && highest_sha256.is_some_and(|known| !known.eq_ignore_ascii_case(sha256))
    {
        return Err(format!(
            "服务器返回的版本 {sequence} 和本地记录的内容不同，可能被篡改，已停止更新"
        ));
    }
    Ok(())
}

// ---- launcher-owned state -------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StateFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

/// `.galaxy-sync/state.json`: the last fully applied release.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub release_id: Option<String>,
    #[serde(default)]
    pub sequence: u64,
    #[serde(default)]
    pub manifest_sha256: Option<String>,
    #[serde(default)]
    pub highest_sequence: u64,
    #[serde(default)]
    pub highest_manifest_sha256: Option<String>,
    #[serde(default)]
    pub display_version: Option<String>,
    #[serde(default)]
    pub files: Vec<StateFile>,
    #[serde(default)]
    pub initialized_paths: Vec<String>,
    #[serde(default)]
    pub applied_corrections: Vec<String>,
    #[serde(default)]
    pub synced_at: Option<String>,
}

/// `.galaxy-sync/file-index.json` entry: the hash of a file as long as its
/// size and modification time stay the same.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    pub size: u64,
    pub mtime_ms: u64,
    pub sha256: String,
}

/// Keyed by [`path_key`].
pub type FileIndex = BTreeMap<String, IndexEntry>;

/// Writes `bytes` to `<path>.tmp`, flushes it and renames it over `path`.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    let temp = path.with_file_name(name);
    {
        let mut file = fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&temp, path).inspect_err(|_| {
        let _ = fs::remove_file(&temp);
    })
}

// ---- local files ----------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default)]
pub struct HashProgress {
    pub done_files: u64,
    pub total_files: u64,
    pub done_bytes: u64,
    pub total_bytes: u64,
}

fn modified_ms(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_millis() as u64)
}

/// `(size, mtime ms)` of a regular file.
pub fn stat_file(path: &Path) -> Option<(u64, u64)> {
    let metadata = fs::metadata(path).ok()?;
    metadata
        .is_file()
        .then(|| (metadata.len(), modified_ms(&metadata)))
}

/// SHA-256 of a file and the number of bytes read.
pub fn hash_file(path: &Path, counter: Option<&AtomicU64>) -> io::Result<(String, u64)> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut total = 0u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        total += read as u64;
        if let Some(counter) = counter {
            counter.fetch_add(read as u64, AtomicOrdering::Relaxed);
        }
    }
    Ok((format!("{:x}", hasher.finalize()), total))
}

/// Local file lookups for one plan, backed by the hash cache. A file is
/// re-hashed when its size or modification time changed, and every file is
/// re-hashed once when `force` is set ("修复游戏文件").
pub struct LocalFiles<'a> {
    root: PathBuf,
    index: &'a mut FileIndex,
    force: bool,
    stats: HashMap<String, Option<(u64, u64)>>,
    /// Hashed during this run (trusted even when `force` is set).
    fresh: HashSet<String>,
    touched: HashSet<String>,
    on_progress: &'a (dyn Fn(HashProgress) + Sync),
    pub hashed_files: u64,
    pub hashed_bytes: u64,
}

impl<'a> LocalFiles<'a> {
    pub fn new(
        root: &Path,
        index: &'a mut FileIndex,
        force: bool,
        on_progress: &'a (dyn Fn(HashProgress) + Sync),
    ) -> Self {
        Self {
            root: root.to_path_buf(),
            index,
            force,
            stats: HashMap::new(),
            fresh: HashSet::new(),
            touched: HashSet::new(),
            on_progress,
            hashed_files: 0,
            hashed_bytes: 0,
        }
    }

    pub fn stat(&mut self, path: &str) -> Option<(u64, u64)> {
        let key = path_key(path);
        if let Some(stat) = self.stats.get(&key) {
            return *stat;
        }
        let stat = stat_file(&absolute(&self.root, path));
        self.stats.insert(key, stat);
        stat
    }

    pub fn exists(&mut self, path: &str) -> bool {
        self.stat(path).is_some()
    }

    fn size(&mut self, path: &str) -> Option<u64> {
        self.stat(path).map(|(size, _)| size)
    }

    fn cached(&self, key: &str, size: u64, mtime: u64) -> Option<String> {
        if self.force && !self.fresh.contains(key) {
            return None;
        }
        self.index
            .get(key)
            .filter(|entry| entry.size == size && entry.mtime_ms == mtime)
            .map(|entry| entry.sha256.clone())
    }

    fn record(&mut self, key: String, size: u64, mtime: u64, sha256: String) {
        self.fresh.insert(key.clone());
        self.index.insert(
            key,
            IndexEntry {
                size,
                mtime_ms: mtime,
                sha256,
            },
        );
    }

    /// Hash of the local file; `None` when it is missing or unreadable.
    pub fn sha(&mut self, path: &str) -> Option<String> {
        let (size, mtime) = self.stat(path)?;
        let key = path_key(path);
        self.touched.insert(key.clone());
        if let Some(sha) = self.cached(&key, size, mtime) {
            return Some(sha);
        }
        let (sha, read) = hash_file(&absolute(&self.root, path), None).ok()?;
        self.hashed_files += 1;
        self.hashed_bytes += read;
        if read == size {
            self.record(key, size, mtime, sha.clone());
        }
        Some(sha)
    }

    /// Hashes the cache misses among `paths` on a few threads (the first run
    /// hashes ~1200 files / ~1.4 GB) and reports progress.
    pub fn prefetch(&mut self, paths: &[String]) {
        let mut todo = Vec::new();
        for path in paths {
            let Some((size, mtime)) = self.stat(path) else {
                continue;
            };
            let key = path_key(path);
            self.touched.insert(key.clone());
            if self.cached(&key, size, mtime).is_none() {
                todo.push((key, absolute(&self.root, path), size, mtime));
            }
        }
        if todo.is_empty() {
            return;
        }
        let total_files = todo.len() as u64;
        let total_bytes = todo.iter().map(|item| item.2).sum();
        let next = AtomicUsize::new(0);
        let done_files = AtomicU64::new(0);
        let done_bytes = AtomicU64::new(0);
        let results = Mutex::new(Vec::with_capacity(todo.len()));
        let report = self.on_progress;
        report(HashProgress {
            done_files: 0,
            total_files,
            done_bytes: 0,
            total_bytes,
        });
        std::thread::scope(|scope| {
            for _ in 0..HASH_WORKERS.min(todo.len()) {
                scope.spawn(|| {
                    loop {
                        let index = next.fetch_add(1, AtomicOrdering::Relaxed);
                        let Some((key, path, size, mtime)) = todo.get(index) else {
                            break;
                        };
                        let hashed = hash_file(path, Some(&done_bytes)).ok();
                        if let Ok(mut results) = results.lock() {
                            results.push((key.clone(), *size, *mtime, hashed));
                        }
                        let files = done_files.fetch_add(1, AtomicOrdering::Relaxed) + 1;
                        report(HashProgress {
                            done_files: files,
                            total_files,
                            done_bytes: done_bytes.load(AtomicOrdering::Relaxed),
                            total_bytes,
                        });
                    }
                });
            }
        });
        for (key, size, mtime, hashed) in results.into_inner().unwrap_or_default() {
            let Some((sha, read)) = hashed else { continue };
            self.hashed_files += 1;
            self.hashed_bytes += read;
            if read == size {
                self.record(key, size, mtime, sha);
            }
        }
    }

    /// Drops cache entries for files this run did not look at.
    pub fn finish(self) {
        let touched = self.touched;
        self.index.retain(|key, _| touched.contains(key));
    }
}

/// Regular files below `root/dir` as `/`-separated paths relative to `root`.
/// Symlinks and junctions are not followed.
fn list_files(root: &Path, dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_string()];
    while let Some(relative) = stack.pop() {
        let Ok(entries) = fs::read_dir(absolute(root, &relative)) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else { continue };
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let child = format!("{relative}/{name}");
            if kind.is_symlink() {
                continue;
            } else if kind.is_dir() {
                stack.push(child);
            } else if kind.is_file() {
                out.push(child);
            }
        }
    }
    out.sort();
    out
}

// ---- plan -------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Install {
    pub sha256: String,
    pub size: u64,
}

/// What happens to one path: the current local file is moved into the backup
/// folder first (`backup` = reason), then `install` is put in place.
#[derive(Clone, Debug)]
pub struct Action {
    pub path: String,
    pub backup: Option<String>,
    pub install: Option<Install>,
}

#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub actions: Vec<Action>,
    /// `state.files` after this release is applied.
    pub files: Vec<StateFile>,
    pub initialized_paths: Vec<String>,
    pub applied_corrections: Vec<String>,
    pub warnings: Vec<String>,
}

impl Plan {
    pub fn files_to_download(&self) -> u64 {
        self.actions.iter().filter(|action| action.install.is_some()).count() as u64
    }

    pub fn bytes_to_download(&self) -> u64 {
        self.actions
            .iter()
            .filter_map(|action| action.install.as_ref())
            .map(|install| install.size)
            .sum()
    }

    pub fn files_to_remove(&self) -> u64 {
        self.actions
            .iter()
            .filter(|action| action.backup.is_some() && action.install.is_none())
            .count() as u64
    }
}

#[derive(Default)]
struct Actions(BTreeMap<String, Action>);

impl Actions {
    fn entry(&mut self, path: &str, key: &str) -> &mut Action {
        self.0.entry(key.to_string()).or_insert_with(|| Action {
            path: path.to_string(),
            backup: None,
            install: None,
        })
    }

    fn backup(&mut self, path: &str, key: &str, reason: String) {
        let action = self.entry(path, key);
        if action.backup.is_none() {
            action.backup = Some(reason);
        }
    }

    fn install(&mut self, file: &FileSpec, backup: Option<String>) {
        let action = self.entry(&file.path, &file.key);
        action.install = Some(Install {
            sha256: file.sha256.clone(),
            size: file.size,
        });
        if action.backup.is_none() {
            action.backup = backup;
        }
    }

    fn contains(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }
}

fn push_unique(list: &mut Vec<String>, value: &str) {
    if !list.iter().any(|existing| existing == value) {
        list.push(value.to_string());
    }
}

/// The plan rules of GALAXY_SYNC.md: compares the release with the local files
/// and the last applied state. Never looks at anything outside the release's
/// files, the previously applied files, withdrawn / corrected paths and the
/// cleanup directories.
pub fn build_plan(release: &Release, state: &SyncState, local: &mut LocalFiles<'_>) -> Plan {
    let mut plan = Plan::default();
    let mut actions = Actions::default();
    let manifest_keys: HashSet<&str> = release.files.iter().map(|file| file.key.as_str()).collect();
    let initialized_before: HashSet<String> =
        state.initialized_paths.iter().map(|path| path_key(path)).collect();
    let applied_before: HashSet<&str> =
        state.applied_corrections.iter().map(String::as_str).collect();
    let mut applied = state.applied_corrections.clone();
    let mut corrections: HashMap<&str, Vec<&CorrectionSpec>> = HashMap::new();
    for correction in &release.corrections {
        corrections.entry(correction.key.as_str()).or_default().push(correction);
        if correction.mode == CorrectionMode::Unknown {
            plan.warnings.push(format!("不认识的修正方式，已跳过：{}", correction.path));
        }
    }

    // The bulk of the work: present SYNC files whose size matches.
    let candidates: Vec<String> = release
        .files
        .iter()
        .filter(|file| file.mode == Mode::Sync)
        .filter(|file| local.size(&file.path) == Some(file.size))
        .map(|file| file.path.clone())
        .collect();
    local.prefetch(&candidates);

    for file in &release.files {
        let present = local.exists(&file.path);
        let mut replace: Option<String> = None;
        let mut once_install = false;
        for correction in corrections.get(file.key.as_str()).into_iter().flatten() {
            match correction.mode {
                CorrectionMode::KnownBad => {
                    if present
                        && let Some(sha) = local.sha(&file.path)
                        && sha != file.sha256
                        && correction.bad.contains(&sha)
                    {
                        replace = Some(format!("问题文件：{}", correction.reason));
                    }
                }
                CorrectionMode::Once => {
                    if applied_before.contains(correction.id.as_str()) {
                        continue;
                    }
                    push_unique(&mut applied, &correction.id);
                    if !present {
                        once_install = true;
                    } else if local.sha(&file.path).as_deref() != Some(file.sha256.as_str()) {
                        replace = Some(format!("重置为默认：{}", correction.reason));
                    }
                }
                CorrectionMode::Unknown => {}
            }
        }
        match file.mode {
            Mode::Sync => {
                if !present {
                    actions.install(file, None);
                } else if local.size(&file.path) != Some(file.size)
                    || local.sha(&file.path).as_deref() != Some(file.sha256.as_str())
                {
                    actions.install(file, Some(replace.unwrap_or_else(|| "被新版本替换".to_string())));
                }
            }
            Mode::Initial => {
                push_unique(&mut plan.initialized_paths, &file.path);
                if let Some(reason) = replace {
                    actions.install(file, Some(reason));
                } else if !present && (once_install || !initialized_before.contains(&file.key)) {
                    actions.install(file, None);
                }
            }
        }
    }

    // Removed since the previously applied release: only unchanged copies.
    for old in &state.files {
        let Ok(path) = safe_relative_path(&old.path) else {
            plan.warnings.push(format!("本地记录里的路径不安全，已忽略：{}", old.path));
            continue;
        };
        let key = path_key(&path);
        if manifest_keys.contains(key.as_str()) || release.is_unmanaged(&key) || actions.contains(&key) {
            continue;
        }
        if local.size(&path) == Some(old.size)
            && local
                .sha(&path)
                .is_some_and(|sha| sha.eq_ignore_ascii_case(&old.sha256))
        {
            actions.backup(&path, &key, "新版本已移除".to_string());
        }
    }

    // Withdrawn copies (REMOVAL / VERSION), unless the path is a current file.
    for copy in &release.withdrawals {
        if manifest_keys.contains(copy.key.as_str())
            || release.is_unmanaged(&copy.key)
            || actions.contains(&copy.key)
        {
            continue;
        }
        let Some(size) = local.size(&copy.path) else { continue };
        if copy.size.is_some_and(|expected| expected != size) {
            continue;
        }
        if local.sha(&copy.path).as_deref() == Some(copy.sha256.as_str()) {
            actions.backup(&copy.path, &copy.key, copy.reason.clone());
        }
    }

    // Corrections on paths that are not release files.
    for correction in &release.corrections {
        if manifest_keys.contains(correction.key.as_str()) || release.is_unmanaged(&correction.key) {
            continue;
        }
        match correction.mode {
            CorrectionMode::KnownBad => {
                if local.exists(&correction.path)
                    && local
                        .sha(&correction.path)
                        .is_some_and(|sha| correction.bad.contains(&sha))
                {
                    actions.backup(
                        &correction.path,
                        &correction.key,
                        format!("问题文件：{}", correction.reason),
                    );
                }
            }
            CorrectionMode::Once => {
                if applied_before.contains(correction.id.as_str()) {
                    continue;
                }
                push_unique(&mut applied, &correction.id);
                if local.exists(&correction.path) {
                    actions.backup(
                        &correction.path,
                        &correction.key,
                        format!("服主要求移除一次：{}", correction.reason),
                    );
                }
            }
            CorrectionMode::Unknown => {}
        }
    }

    // forcedSyncDirectories / cleanupDirectories: extra files go to the backup.
    for dir in &release.cleanup_dirs {
        if release.is_unmanaged(&path_key(dir)) {
            continue;
        }
        for path in list_files(&local.root, dir) {
            let Ok(path) = safe_relative_path(&path) else { continue };
            let key = path_key(&path);
            if manifest_keys.contains(key.as_str()) || release.is_unmanaged(&key) || actions.contains(&key) {
                continue;
            }
            actions.backup(&path, &key, "强制同步目录里的额外文件".to_string());
        }
    }

    plan.files = release
        .files
        .iter()
        .map(|file| StateFile {
            path: file.path.clone(),
            sha256: file.sha256.clone(),
            size: file.size,
        })
        .collect();
    plan.applied_corrections = applied;
    plan.actions = actions.0.into_values().collect();
    plan
}

// ---- install ----------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupItem {
    pub path: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default)]
pub struct InstallReport {
    pub installed: Vec<StateFile>,
    pub backed_up: Vec<BackupItem>,
    /// Set when at least one file was moved into it.
    pub backup_dir: Option<PathBuf>,
}

/// Where a verified download waits until it is installed.
pub fn part_path(downloads: &Path, sha256: &str) -> PathBuf {
    downloads.join(format!("{sha256}.part"))
}

/// Same-volume rename; copy + delete when the target is on another volume
/// (e.g. a junctioned `mods` folder).
pub fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(error) if error.raw_os_error() == Some(CROSS_DEVICE_ERROR) => {
            fs::copy(from, to)?;
            fs::remove_file(from)
        }
        Err(error) => Err(error),
    }
}

pub fn describe_io_error(error: &io::Error) -> String {
    // 32 = ERROR_SHARING_VIOLATION, 33 = ERROR_LOCK_VIOLATION
    if matches!(error.raw_os_error(), Some(32 | 33)) {
        "文件正被其他程序使用（请先关闭游戏）".to_string()
    } else if error.kind() == io::ErrorKind::PermissionDenied {
        "没有权限（文件可能被占用或是只读的）".to_string()
    } else {
        error.to_string()
    }
}

#[derive(Default)]
struct Journal {
    /// (original location, backup location)
    moved: Vec<(PathBuf, PathBuf)>,
    /// (installed file, the download it was renamed from)
    installed: Vec<(PathBuf, Option<PathBuf>)>,
    report: InstallReport,
}

impl Journal {
    /// Undoes everything in reverse order; false when something could not be
    /// restored.
    fn rollback(&mut self) -> bool {
        let mut complete = true;
        for (target, part) in self.installed.drain(..).rev() {
            let undone = match part {
                Some(part) => move_file(&target, &part),
                None => fs::remove_file(&target),
            };
            if undone.is_err() && target.exists() {
                complete = false;
            }
        }
        for (original, backup) in self.moved.drain(..).rev() {
            if move_file(&backup, &original).is_err() {
                complete = false;
            }
        }
        complete
    }
}

fn install_steps(
    root: &Path,
    downloads: &Path,
    backup_dir: &Path,
    actions: &[Action],
    journal: &mut Journal,
) -> Result<(), String> {
    let back_up = |journal: &mut Journal, path: &str, reason: &str| -> Result<(), String> {
        let target = absolute(root, path);
        if !target.is_file() {
            return Ok(());
        }
        let backup = absolute(backup_dir, path);
        move_file(&target, &backup)
            .map_err(|error| format!("无法移走 {path}：{}", describe_io_error(&error)))?;
        journal.moved.push((target, backup));
        journal.report.backed_up.push(BackupItem {
            path: path.to_string(),
            reason: reason.to_string(),
        });
        Ok(())
    };

    // 1. Everything that will be replaced or removed goes to the backup first.
    for action in actions {
        if let Some(reason) = &action.backup {
            back_up(journal, &action.path, reason)?;
        }
    }

    // 2. Verified downloads go into place; one object can serve several paths.
    let mut uses: HashMap<&str, usize> = HashMap::new();
    for install in actions.iter().filter_map(|action| action.install.as_ref()) {
        *uses.entry(install.sha256.as_str()).or_default() += 1;
    }
    for action in actions {
        let Some(install) = &action.install else { continue };
        let target = absolute(root, &action.path);
        if target.is_file() {
            // Appeared after the plan was made: keep it as well.
            back_up(journal, &action.path, "安装前已存在")?;
        } else if target.exists() {
            return Err(format!("{} 被同名文件夹占用", action.path));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("无法创建文件夹 {}：{}", parent.display(), describe_io_error(&error))
            })?;
        }
        let left = uses.get_mut(install.sha256.as_str()).map_or(0, |left| {
            *left -= 1;
            *left
        });
        let failed = |error: io::Error| format!("无法安装 {}：{}", action.path, describe_io_error(&error));
        if install.size == 0 {
            fs::File::create(&target).map_err(failed)?;
            journal.installed.push((target, None));
        } else {
            let part = part_path(downloads, &install.sha256);
            if left == 0 {
                move_file(&part, &target).map_err(failed)?;
                journal.installed.push((target, Some(part)));
            } else {
                let mut name = target.file_name().unwrap_or_default().to_os_string();
                name.push(".galaxy-tmp");
                let temp = target.with_file_name(name);
                fs::copy(&part, &temp).map_err(failed)?;
                fs::rename(&temp, &target).map_err(|error| {
                    let _ = fs::remove_file(&temp);
                    failed(error)
                })?;
                journal.installed.push((target, None));
            }
        }
        journal.report.installed.push(StateFile {
            path: action.path.clone(),
            sha256: install.sha256.clone(),
            size: install.size,
        });
    }
    Ok(())
}

/// Applies `actions` once every download is verified in `downloads`. Old and
/// removed files are moved to `backup_dir` (with an `index.json`). On any
/// failure everything done so far is undone (best effort) so the instance is
/// never left with half a mods folder.
pub fn install(
    root: &Path,
    downloads: &Path,
    backup_dir: &Path,
    release_id: &str,
    created_at: &str,
    actions: &[Action],
) -> Result<InstallReport, String> {
    let mut journal = Journal::default();
    if let Err(message) = install_steps(root, downloads, backup_dir, actions, &mut journal) {
        let restored = journal.rollback();
        remove_empty_dirs(backup_dir);
        return Err(if restored {
            format!("{message}。已恢复原来的文件")
        } else {
            format!(
                "{message}。有些文件没能自动恢复，原文件在 {}",
                backup_dir.display()
            )
        });
    }
    let mut report = journal.report;
    if report.backed_up.is_empty() {
        remove_empty_dirs(backup_dir);
    } else {
        let index = serde_json::json!({
            "releaseId": release_id,
            "createdAt": created_at,
            "items": report.backed_up,
        });
        let bytes = serde_json::to_vec_pretty(&index).unwrap_or_default();
        let _ = write_atomic(&backup_dir.join("index.json"), &bytes);
        report.backup_dir = Some(backup_dir.to_path_buf());
    }
    Ok(report)
}

/// Removes `dir` and its sub-directories when they contain no files.
fn remove_empty_dirs(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                remove_empty_dirs(&entry.path());
            }
        }
    }
    let _ = fs::remove_dir(dir);
}

/// Keeps the newest `keep` backup folders (names start with the timestamp).
pub fn prune_backups(backups: &Path, keep: usize) -> Vec<String> {
    let Ok(entries) = fs::read_dir(backups) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort_unstable_by(|left, right| right.cmp(left));
    names
        .into_iter()
        .skip(keep)
        .filter(|name| fs::remove_dir_all(backups.join(name)).is_ok())
        .collect()
}

/// Deletes leftover files in the downloads folder.
pub fn clear_dir_files(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|kind| kind.is_file()) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::{Value, json};

    const BINDING_KEY: &str = "MCowBQYDK2VwAyEAPqMOIqOlK/TQgA+Bihv3Pt0bms91Kf77iMDOfKv+dTA=";

    fn sha(text: &str) -> String {
        sha256_hex(text.as_bytes())
    }

    fn write(root: &Path, path: &str, text: &str) {
        let target = absolute(root, path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, text).unwrap();
    }

    fn read(root: &Path, path: &str) -> Option<String> {
        fs::read_to_string(absolute(root, path)).ok()
    }

    fn file(path: &str, content: &str, preset: &str) -> Value {
        json!({ "path": path, "sha256": sha(content), "size": content.len(),
                "policy": "ENFORCED", "preset": preset, "executable": false })
    }

    fn manifest(files: Vec<Value>, extra: Value) -> Manifest {
        let mut value = json!({
            "schemaVersion": 1, "projectId": "galaxy-pokemon", "releaseId": "r000002-test",
            "sequence": 2, "displayVersion": "1.0.1", "minimumPlayerVersion": "0.3.2",
            "requiredCapabilities": ["simplified-maintenance-v1", "maintenance-policy-v2"],
            "files": files,
        });
        if let (Value::Object(target), Value::Object(extra)) = (&mut value, extra) {
            target.extend(extra);
        }
        serde_json::from_value(value).unwrap()
    }

    fn plan_with(root: &Path, manifest: &Manifest, state: &SyncState, index: &mut FileIndex, force: bool) -> Plan {
        let release = validate(manifest, "galaxy-pokemon").unwrap();
        let noop = |_: HashProgress| {};
        let mut local = LocalFiles::new(root, index, force, &noop);
        let plan = build_plan(&release, state, &mut local);
        local.finish();
        plan
    }

    fn plan_for(root: &Path, manifest: &Manifest, state: &SyncState) -> Plan {
        plan_with(root, manifest, state, &mut FileIndex::new(), false)
    }

    fn action<'a>(plan: &'a Plan, path: &str) -> Option<&'a Action> {
        plan.actions.iter().find(|action| action.path == path)
    }

    fn state_from(plan: &Plan) -> SyncState {
        SyncState {
            schema_version: 1,
            files: plan.files.clone(),
            initialized_paths: plan.initialized_paths.clone(),
            applied_corrections: plan.applied_corrections.clone(),
            ..SyncState::default()
        }
    }

    #[test]
    fn paths_are_validated() {
        for ok in ["mods/a.jar", "options.txt", "config/x/y.json", "launcher-update/latest.json"] {
            assert_eq!(safe_relative_path(ok).as_deref(), Ok(ok));
        }
        for bad in [
            "", "/abs", "C:/x", "c:x", "a\\b", "../x", "a/../b", "a//b", "a/./b", "mods/",
            "mods/x.jar:stream", "con", "mods/NUL.txt", "mods/com1.jar", "mods/a.", "mods/a ",
            ".galaxy-sync/state.json", ".DreamingFish-Bootstrap/x", "a/b?c", "a\u{0}b",
        ] {
            assert!(safe_relative_path(bad).is_err(), "{bad:?}");
        }
        assert!(safe_relative_path("mods/com10.jar").is_ok());
    }

    #[test]
    fn binding_key_parses_and_signatures_cover_the_exact_bytes() {
        assert!(parse_public_key(BINDING_KEY).is_ok());
        assert!(parse_public_key("AAAA").is_err());

        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let mut der = SPKI_ED25519_PREFIX.to_vec();
        der.extend_from_slice(signing.verifying_key().as_bytes());
        let key = parse_public_key(&base64::engine::general_purpose::STANDARD.encode(der)).unwrap();
        let body = b"{\r\n  \"sequence\" : 7\r\n}";
        let signature =
            base64::engine::general_purpose::STANDARD.encode(signing.sign(body).to_bytes());
        assert!(verify_signature(&key, body, &format!("{signature}\n")));
        assert!(!verify_signature(&key, b"{\"sequence\":7}", &signature));
        let mut tampered = body.to_vec();
        tampered[5] ^= 1;
        assert!(!verify_signature(&key, &tampered, &signature));
        assert!(!verify_signature(&key, body, "not base64!"));
        assert!(!verify_signature(&parse_public_key(BINDING_KEY).unwrap(), body, &signature));
    }

    #[test]
    fn validation_rejects_what_this_client_cannot_apply() {
        let base = manifest(vec![file("mods/a.jar", "a", "SYNC")], json!({}));
        assert!(validate(&base, "galaxy-pokemon").is_ok());
        assert_eq!(validate(&base, "other").unwrap_err().code(), "PROJECT_MISMATCH");
        let caps = manifest(vec![], json!({ "requiredCapabilities": ["future-thing-v9"] }));
        assert_eq!(validate(&caps, "galaxy-pokemon").unwrap_err().code(), "NEEDS_NEWER_LAUNCHER");
        let newer = manifest(vec![], json!({ "minimumPlayerVersion": "0.3.10" }));
        assert_eq!(validate(&newer, "galaxy-pokemon").unwrap_err().code(), "NEEDS_NEWER_LAUNCHER");
        let schema = manifest(vec![], json!({ "schemaVersion": 2 }));
        assert_eq!(validate(&schema, "galaxy-pokemon").unwrap_err().code(), "NEEDS_NEWER_LAUNCHER");
        let escape = manifest(vec![file("../evil.jar", "x", "SYNC")], json!({}));
        assert_eq!(validate(&escape, "galaxy-pokemon").unwrap_err().code(), "PATH_UNSAFE");
        let drive = manifest(vec![file("C:/Windows/evil.dll", "x", "SYNC")], json!({}));
        assert_eq!(validate(&drive, "galaxy-pokemon").unwrap_err().code(), "PATH_UNSAFE");
        let withdrawn = manifest(vec![], json!({ "withdrawals": [{ "id": "w", "kind": "REMOVAL",
            "items": [{ "path": "/etc/passwd", "sha256": sha("x"), "size": 1 }] }] }));
        assert_eq!(validate(&withdrawn, "galaxy-pokemon").unwrap_err().code(), "PATH_UNSAFE");
        let dup = manifest(vec![file("mods/a.jar", "a", "SYNC"), file("mods/a.jar", "b", "SYNC")], json!({}));
        assert_eq!(validate(&dup, "galaxy-pokemon").unwrap_err().code(), "MANIFEST_INVALID");
        if cfg!(windows) {
            let case = manifest(vec![file("mods/A.jar", "a", "SYNC"), file("mods/a.jar", "b", "SYNC")], json!({}));
            assert_eq!(validate(&case, "galaxy-pokemon").unwrap_err().code(), "MANIFEST_INVALID");
        }
        let mut bad_hash = file("mods/a.jar", "a", "SYNC");
        bad_hash["sha256"] = json!("xyz");
        assert_eq!(validate(&manifest(vec![bad_hash], json!({})), "galaxy-pokemon").unwrap_err().code(), "MANIFEST_INVALID");
    }

    #[test]
    fn replay_is_refused() {
        assert!(check_replay(7, "aa", 7, Some("AA")).is_ok());
        assert!(check_replay(8, "bb", 7, Some("aa")).is_ok());
        assert!(check_replay(6, "bb", 7, Some("aa")).is_err());
        assert!(check_replay(7, "bb", 7, Some("aa")).is_err());
        assert!(check_replay(1, "bb", 0, None).is_ok());
    }

    #[test]
    fn sync_files_follow_the_release_and_extra_files_are_left_alone() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        write(root, "mods/a.jar", "a1");
        write(root, "mods/b.jar", "old");
        write(root, "mods/player-added.jar", "mine");
        write(root, "saves/world/level.dat", "world");
        let m = manifest(
            vec![file("mods/a.jar", "a1", "SYNC"), file("mods/b.jar", "b2", "REQUIRED"), file("mods/c.jar", "c", "SYNC")],
            json!({}),
        );
        let plan = plan_for(root, &m, &SyncState::default());
        assert_eq!(plan.actions.len(), 2);
        assert!(action(&plan, "mods/a.jar").is_none());
        let b = action(&plan, "mods/b.jar").unwrap();
        assert!(b.backup.is_some() && b.install.as_ref().unwrap().sha256 == sha("b2"));
        let c = action(&plan, "mods/c.jar").unwrap();
        assert!(c.backup.is_none() && c.install.is_some());
        assert_eq!((plan.files_to_download(), plan.bytes_to_download(), plan.files_to_remove()), (2, 3, 0));
    }

    #[test]
    fn initial_files_are_provided_once_and_then_left_to_the_player() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        write(root, "options.txt", "player settings");
        let m = manifest(
            vec![
                file("options.txt", "defaults", "INITIAL"),
                file("servers.dat", "servers", "INITIAL"),
                file("config/legacy.json", "legacy", "SYNC"),
                file("config/default.toml", "default", "DEFAULT_CONFIG"),
            ],
            json!({}),
        );
        let mut legacy = m.clone();
        legacy.files[2].policy = Some("LEGACY_MISSING_ONLY".to_string());
        write(root, "config/legacy.json", "edited by player");
        write(root, "config/default.toml", "edited");
        let plan = plan_for(root, &legacy, &SyncState::default());
        assert!(action(&plan, "options.txt").is_none(), "existing options.txt is kept");
        assert!(action(&plan, "servers.dat").unwrap().install.is_some());
        assert!(action(&plan, "config/legacy.json").is_none());
        assert!(action(&plan, "config/default.toml").is_none());
        assert_eq!(plan.initialized_paths.len(), 4);

        // Recorded once: deleting or editing them later is the player's business.
        let state = state_from(&plan);
        fs::remove_file(absolute(root, "options.txt")).unwrap();
        write(root, "servers.dat", "my servers");
        let again = plan_for(root, &legacy, &state);
        assert!(again.actions.is_empty(), "{:?}", again.actions);
    }

    #[test]
    fn forced_sync_files_behave_like_sync() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        write(root, "options.txt", "player settings");
        let m = manifest(vec![file("options.txt", "defaults", "INITIAL")], json!({ "forcedSyncFiles": ["options.txt"] }));
        let plan = plan_for(root, &m, &SyncState::default());
        let options = action(&plan, "options.txt").unwrap();
        assert!(options.backup.is_some() && options.install.is_some());
    }

    #[test]
    fn video_settings_are_left_to_the_player_even_when_released_as_sync() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        write(root, "config/iris.properties", "enableShaders=true");
        write(root, "config/sodium-options.json", "{\"player\":1}");
        let files = vec![
            file("config/iris.properties", "enableShaders=false", "SYNC"),
            file("config/sodium-options.json", "{}", "SYNC"),
        ];
        let plan = plan_for(root, &manifest(files.clone(), json!({})), &SyncState::default());
        assert!(action(&plan, "config/iris.properties").is_none());
        assert!(action(&plan, "config/sodium-options.json").is_none());
        assert_eq!(plan.initialized_paths.len(), 2);

        let forced = manifest(files, json!({ "forcedSyncFiles": ["config/iris.properties"] }));
        let plan = plan_for(root, &forced, &SyncState::default());
        assert!(action(&plan, "config/iris.properties").unwrap().install.is_some());
        assert!(action(&plan, "config/sodium-options.json").is_none());
    }

    #[test]
    fn optional_groups_that_are_off_are_not_installed() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        let mut shader = file("shaderpacks/fancy.zip", "fancy", "SYNC");
        shader["optionalGroup"] = json!("shaders");
        let mut extra = file("mods/extra.jar", "extra", "SYNC");
        extra["optionalGroup"] = json!("extras");
        let m = manifest(
            vec![shader, extra],
            json!({ "optionalGroups": [
                { "id": "shaders", "title": "光影", "description": "", "defaultInstall": false },
                { "id": "extras", "title": "额外", "description": "", "defaultInstall": true } ] }),
        );
        write(root, "shaderpacks/fancy.zip", "fancy");
        let previous = SyncState {
            files: vec![StateFile { path: "shaderpacks/fancy.zip".into(), sha256: sha("fancy"), size: 5 }],
            ..SyncState::default()
        };
        let plan = plan_for(root, &m, &previous);
        assert!(action(&plan, "mods/extra.jar").unwrap().install.is_some());
        let fancy = action(&plan, "shaderpacks/fancy.zip").unwrap();
        assert!(fancy.install.is_none() && fancy.backup.is_some(), "treated as absent from the release");
        assert!(!plan.files.iter().any(|f| f.path == "shaderpacks/fancy.zip"));
    }

    #[test]
    fn removed_files_go_to_the_backup_only_when_unchanged() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        write(root, "mods/gone.jar", "gone");
        write(root, "config/gone.json", "player edited");
        let previous = SyncState {
            files: vec![
                StateFile { path: "mods/gone.jar".into(), sha256: sha("gone"), size: 4 },
                StateFile { path: "config/gone.json".into(), sha256: sha("orig"), size: 4 },
            ],
            ..SyncState::default()
        };
        let plan = plan_for(root, &manifest(vec![], json!({})), &previous);
        assert!(action(&plan, "mods/gone.jar").unwrap().backup.is_some());
        assert!(action(&plan, "config/gone.json").is_none());
        assert_eq!(plan.files_to_remove(), 1);
    }

    #[test]
    fn withdrawals_and_corrections_are_applied() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        write(root, "mods/bad-1.0.jar", "bad");
        write(root, "mods/other-1.0.jar", "other build");
        write(root, "mods/kept.jar", "bad");
        write(root, "config/broken.json", "broken");
        write(root, "config/stray.json", "broken");
        let m = manifest(
            vec![file("mods/kept.jar", "good", "SYNC"), file("config/broken.json", "fixed", "INITIAL")],
            json!({
                "withdrawals": [{ "id": "w1", "kind": "VERSION", "reason": "会崩溃", "items": [
                    { "path": "mods/bad-1.0.jar", "sha256": sha("bad"), "size": 3 },
                    { "path": "mods/other-1.0.jar", "sha256": sha("other"), "size": 5 },
                    { "path": "mods/kept.jar", "sha256": sha("bad"), "size": 3 } ] }],
                "corrections": [
                    { "id": "c1", "path": "config/broken.json", "mode": "KNOWN_BAD", "badSha256": [sha("broken")], "reason": "坏配置" },
                    { "id": "c2", "path": "config/stray.json", "mode": "KNOWN_BAD", "badSha256": [sha("broken")], "reason": "坏配置" } ]
            }),
        );
        let state = SyncState { initialized_paths: vec!["config/broken.json".into()], ..SyncState::default() };
        let plan = plan_for(root, &m, &state);
        assert!(action(&plan, "mods/bad-1.0.jar").unwrap().backup.as_deref() == Some("会崩溃"));
        assert!(action(&plan, "mods/other-1.0.jar").is_none(), "different content is not the withdrawn copy");
        let kept = action(&plan, "mods/kept.jar").unwrap();
        assert!(kept.install.is_some(), "a release file is replaced, not withdrawn");
        let broken = action(&plan, "config/broken.json").unwrap();
        assert!(broken.backup.is_some() && broken.install.is_some(), "known-bad initial file is reinstalled");
        let stray = action(&plan, "config/stray.json").unwrap();
        assert!(stray.backup.is_some() && stray.install.is_none());
    }

    #[test]
    fn once_corrections_apply_once() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        write(root, "options.txt", "player settings");
        let m = manifest(
            vec![file("options.txt", "new defaults", "INITIAL")],
            json!({ "corrections": [{ "id": "reset-options", "path": "options.txt", "mode": "ONCE", "badSha256": [], "reason": "新默认设置" }] }),
        );
        let state = SyncState { initialized_paths: vec!["options.txt".into()], ..SyncState::default() };
        let plan = plan_for(root, &m, &state);
        let options = action(&plan, "options.txt").unwrap();
        assert!(options.backup.is_some() && options.install.is_some());
        assert_eq!(plan.applied_corrections, vec!["reset-options".to_string()]);
        let next = SyncState { applied_corrections: plan.applied_corrections.clone(), ..state };
        assert!(plan_for(root, &m, &next).actions.is_empty());
    }

    #[test]
    fn cleanup_directories_back_up_extra_files_except_released_paths() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        write(root, "mods/core.jar", "core");
        write(root, "mods/extra.jar", "extra");
        write(root, "mods/sub/nested.jar", "nested");
        write(root, "mods/mine/keep.jar", "keep");
        write(root, "config/player.json", "player");
        let m = manifest(
            vec![file("mods/core.jar", "core", "SYNC")],
            json!({ "forcedSyncDirectories": ["mods"], "releasedPaths": ["mods/mine"] }),
        );
        let plan = plan_for(root, &m, &SyncState::default());
        let paths: Vec<&str> = plan.actions.iter().map(|action| action.path.as_str()).collect();
        assert_eq!(paths, vec!["mods/extra.jar", "mods/sub/nested.jar"]);
        assert!(plan.actions.iter().all(|action| action.install.is_none()));
    }

    #[test]
    fn hash_cache_is_trusted_until_size_or_mtime_changes() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        write(root, "mods/a.jar", "a1");
        let m = manifest(vec![file("mods/a.jar", "a1", "SYNC")], json!({}));
        let (size, mtime) = stat_file(&absolute(root, "mods/a.jar")).unwrap();
        let mut index = FileIndex::new();
        index.insert(path_key("mods/a.jar"), IndexEntry { size, mtime_ms: mtime, sha256: sha("stale") });
        let cached = plan_with(root, &m, &SyncState::default(), &mut index, false);
        assert_eq!(cached.files_to_download(), 1, "the cache entry is believed");
        let forced = plan_with(root, &m, &SyncState::default(), &mut index, true);
        assert!(forced.actions.is_empty(), "修复游戏文件 re-hashes");
        assert_eq!(index[&path_key("mods/a.jar")].sha256, sha("a1"));
        index.get_mut(&path_key("mods/a.jar")).unwrap().mtime_ms += 1;
        index.get_mut(&path_key("mods/a.jar")).unwrap().sha256 = sha("stale");
        assert!(plan_with(root, &m, &SyncState::default(), &mut index, false).actions.is_empty());
    }

    #[test]
    fn install_backs_up_and_rolls_back_on_failure() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().join("instance");
        let downloads = temp.path().join("downloads");
        write(&root, "mods/b.jar", "old");
        write(&root, "mods/gone.jar", "gone");
        write(&downloads, &format!("{}.part", sha("b2")), "b2");
        write(&downloads, &format!("{}.part", sha("c")), "c");
        let actions = vec![
            Action { path: "mods/b.jar".into(), backup: Some("被新版本替换".into()), install: Some(Install { sha256: sha("b2"), size: 2 }) },
            Action { path: "mods/c.jar".into(), backup: None, install: Some(Install { sha256: sha("c"), size: 1 }) },
            Action { path: "config/c-copy.txt".into(), backup: None, install: Some(Install { sha256: sha("c"), size: 1 }) },
            Action { path: "config/empty.txt".into(), backup: None, install: Some(Install { sha256: EMPTY_SHA256.into(), size: 0 }) },
            Action { path: "mods/gone.jar".into(), backup: Some("新版本已移除".into()), install: None },
        ];
        let backups = temp.path().join("backups").join("20261102-120000_r2");
        let report = install(&root, &downloads, &backups, "r2", "now", &actions).unwrap();
        assert_eq!(read(&root, "mods/b.jar").as_deref(), Some("b2"));
        assert_eq!(read(&root, "mods/c.jar").as_deref(), Some("c"));
        assert_eq!(read(&root, "config/c-copy.txt").as_deref(), Some("c"));
        assert_eq!(read(&root, "config/empty.txt").as_deref(), Some(""));
        assert!(!absolute(&root, "mods/gone.jar").exists());
        assert_eq!(read(&backups, "mods/b.jar").as_deref(), Some("old"));
        assert_eq!(read(&backups, "mods/gone.jar").as_deref(), Some("gone"));
        assert!(backups.join("index.json").is_file());
        assert_eq!((report.installed.len(), report.backed_up.len()), (4, 2));

        // A file where a folder must go makes the install fail half-way.
        write(&root, "mods/b.jar", "old");
        write(&root, "blocker", "i am a file");
        write(&downloads, &format!("{}.part", sha("b2")), "b2");
        write(&downloads, &format!("{}.part", sha("x")), "x");
        let failing = vec![
            Action { path: "mods/b.jar".into(), backup: Some("被新版本替换".into()), install: Some(Install { sha256: sha("b2"), size: 2 }) },
            Action { path: "blocker/x.txt".into(), backup: None, install: Some(Install { sha256: sha("x"), size: 1 }) },
        ];
        let second = temp.path().join("backups").join("20261102-120001_r3");
        let error = install(&root, &downloads, &second, "r3", "now", &failing).unwrap_err();
        assert!(error.contains("已恢复原来的文件"), "{error}");
        assert_eq!(read(&root, "mods/b.jar").as_deref(), Some("old"));
        assert_eq!(read(&downloads, &format!("{}.part", sha("b2"))).as_deref(), Some("b2"), "download kept for the retry");
        assert!(!second.exists());
    }

    #[test]
    fn prune_keeps_the_newest_backups() {
        let temp = tempfile::TempDir::new().unwrap();
        for name in ["20261101-100000_r1", "20261101-110000_r2", "20261102-090000_r3", "20261102-100000_r4", "20261102-100000_r4-2"] {
            fs::create_dir_all(temp.path().join(name)).unwrap();
        }
        let removed = prune_backups(temp.path(), 3);
        assert_eq!(removed.len(), 2);
        assert!(temp.path().join("20261102-100000_r4-2").exists());
        assert!(!temp.path().join("20261101-100000_r1").exists());
        assert_eq!(folder_safe("r000007-20261031-174732-fef3f576"), "r000007-20261031-174732-fef3f576");
        assert_eq!(folder_safe("../a b"), "_a_b");
    }
}
