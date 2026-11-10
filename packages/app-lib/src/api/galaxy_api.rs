//! Galaxy宝可梦: client for the server data API, i.e. the GalaxyLauncherApi
//! plugin on the main server (contract: GALAXY_LAUNCHER_API.md). The home page
//! reads server status, player data, events and announcements through it, and
//! the crash dialog uploads reports to it.
//!
//! The webview CSP does not allow the API host, so the frontend goes through
//! these commands. Only the fixed GET paths of the contract are reachable and
//! every response is size-capped.
//!
//! The base URL comes from `GALAXY_API_BASE` (development) or `apiBase` in
//! `galaxy-launcher.json` next to the exe (written by `galaxy-stage.ps1`).
//! Without either the features stay hidden.

use crate::api::galaxy::{find_game_root, is_valid_player_name};
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const API_BASE_ENV: &str = "GALAXY_API_BASE";
const GET_TIMEOUT: Duration = Duration::from_secs(8);
const POST_TIMEOUT: Duration = Duration::from_secs(45);
const GET_MAX_BYTES: usize = 1024 * 1024;
const POST_RESPONSE_MAX_BYTES: usize = 64 * 1024;
/// The server refuses request bodies over 2 MB; stay below that.
const CRASH_BODY_BUDGET: usize = 2 * 1024 * 1024 - 64 * 1024;
const CRASH_FILE_MAX_BYTES: usize = 1024 * 1024;
const CRASH_MAX_FILES: usize = 4;
const CRASH_SUMMARY_MAX_CHARS: usize = 500;

fn api_error(message: impl Into<String>) -> crate::Error {
    crate::ErrorKind::OtherError(message.into()).as_error()
}

/// http(s) with a host, no credentials, query or fragment; trailing slashes
/// removed. `None` when the value is unusable.
pub fn normalize_api_base(raw: &str) -> Option<String> {
    let parsed = url::Url::parse(raw.trim()).ok()?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return None;
    }
    Some(parsed.as_str().trim_end_matches('/').to_string())
}

/// The configured API base: `GALAXY_API_BASE`, then `apiBase` in
/// `galaxy-launcher.json`.
pub fn api_base() -> Option<String> {
    let raw = std::env::var(API_BASE_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(crate::api::galaxy_update::configured_api_base)?;
    let base = normalize_api_base(&raw);
    if base.is_none() {
        tracing::warn!(raw, "Ignoring an invalid Galaxy API base");
    }
    base
}

fn require_base() -> crate::Result<String> {
    api_base().ok_or_else(|| api_error("这个启动器没有配置服务器数据接口"))
}

/// The GET paths of the contract the frontend may request.
pub fn is_allowed_get_path(path: &str) -> bool {
    match path {
        "/v1/health" | "/v1/summary" | "/v1/announcements" => true,
        _ => path
            .strip_prefix("/v1/player/")
            .is_some_and(is_valid_player_name),
    }
}

async fn read_limited(
    mut response: reqwest::Response,
    max: usize,
) -> crate::Result<String> {
    if response
        .content_length()
        .is_some_and(|length| length > max as u64)
    {
        return Err(api_error("服务器返回的内容太大"));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| api_error(format!("读取服务器数据失败：{error}")))?
    {
        if body.len() + chunk.len() > max {
            return Err(api_error("服务器返回的内容太大"));
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body)
        .map_err(|_| api_error("服务器返回的不是 UTF-8 文本"))
}

/// The `error` text of an API error response.
fn error_text(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let text = value.get("error")?.as_str()?.trim();
    (!text.is_empty()).then(|| text.chars().take(200).collect())
}

/// GETs one of the contract's paths and returns the JSON text.
#[tracing::instrument]
pub async fn get(path: &str) -> crate::Result<String> {
    if !is_allowed_get_path(path) {
        return Err(api_error("不支持的服务器数据接口地址"));
    }
    let url = format!("{}{path}", require_base()?);
    let client = crate::util::fetch::configured_client().await?;
    let request = async {
        let response = client
            .get(&url)
            .header(ACCEPT, "application/json")
            .send()
            .await
            .map_err(|error| {
                api_error(format!("连不上服务器数据接口：{error}"))
            })?;
        let status = response.status();
        let body = read_limited(response, GET_MAX_BYTES).await?;
        if !status.is_success() {
            return Err(api_error(error_text(&body).unwrap_or_else(|| {
                format!("服务器数据接口返回 {status}")
            })));
        }
        Ok(body)
    };
    tokio::time::timeout(GET_TIMEOUT, request)
        .await
        .map_err(|_| api_error("服务器数据接口超时"))?
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalaxyCrashFile {
    pub name: String,
    pub content: String,
}

/// What the crash dialog sends; the backend adds the launcher, OS and Java
/// versions and trims the files to the server's limits.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalaxyCrashUpload {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub modpack_version: Option<String>,
    #[serde(default)]
    pub memory_mb: Option<u64>,
    #[serde(default)]
    pub files: Vec<GalaxyCrashFile>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalaxyCrashUploadResult {
    pub id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CrashBody {
    schema: u32,
    name: Option<String>,
    launcher_version: String,
    modpack_version: Option<String>,
    os: String,
    java_version: Option<String>,
    memory_mb: Option<u64>,
    summary: String,
    files: Vec<CrashBodyFile>,
}

#[derive(Debug, Serialize)]
struct CrashBodyFile {
    name: String,
    content: String,
}

/// `[A-Za-z0-9._-]`, at most 100 characters.
fn safe_file_name(name: &str, index: usize) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .take(100)
        .collect();
    if cleaned.trim_matches('.').is_empty() {
        format!("file{}.txt", index + 1)
    } else {
        cleaned
    }
}

/// Crash reports keep their start (the exception is at the top), logs keep
/// their end (the crash is at the bottom).
fn keeps_head(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with("crash-") || lower.starts_with("hs_err")
}

/// Cuts `text` to at most `max` bytes on a char boundary and notes the cut.
fn trim_content(text: &str, max: usize, head: bool) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let dropped_kb = (text.len() - max).div_ceil(1024);
    if head {
        let mut end = max;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}\n…（后面省略约 {dropped_kb} KB）", &text[..end])
    } else {
        let mut start = text.len() - max;
        while !text.is_char_boundary(start) {
            start += 1;
        }
        format!("…（前面省略约 {dropped_kb} KB）\n{}", &text[start..])
    }
}

fn one_line(text: &str, max_chars: usize) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

/// `JAVA_VERSION` from the bundled runtime's `release` file.
fn bundled_java_version() -> Option<String> {
    let release = find_game_root()?
        .join("runtime")
        .join("java-runtime-delta")
        .join("release");
    let text = std::fs::read_to_string(release).ok()?;
    text.lines().find_map(|line| {
        let value = line.strip_prefix("JAVA_VERSION=")?.trim().trim_matches('"');
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn os_text() -> String {
    let os = sysinfo::System::long_os_version()
        .or_else(sysinfo::System::name)
        .unwrap_or_else(|| std::env::consts::OS.to_string());
    format!("{os} {}", std::env::consts::ARCH)
}

/// Builds the request body and shrinks the largest file until the JSON fits
/// the server's 2 MB limit.
fn crash_body(upload: GalaxyCrashUpload) -> crate::Result<Vec<u8>> {
    let name = upload
        .name
        .map(|name| name.trim().to_string())
        .filter(|name| is_valid_player_name(name));
    let mut files: Vec<CrashBodyFile> = upload
        .files
        .into_iter()
        .filter(|file| !file.content.is_empty())
        .take(CRASH_MAX_FILES)
        .enumerate()
        .map(|(index, file)| {
            let name = safe_file_name(&file.name, index);
            let content = trim_content(
                &file.content,
                CRASH_FILE_MAX_BYTES,
                keeps_head(&name),
            );
            CrashBodyFile { name, content }
        })
        .collect();
    let mut body = CrashBody {
        schema: 1,
        name,
        launcher_version: crate::api::galaxy_update::current_version()
            .unwrap_or_else(|| format!("dev-{}", env!("CARGO_PKG_VERSION"))),
        modpack_version: upload
            .modpack_version
            .map(|version| one_line(&version, 100))
            .filter(|version| !version.is_empty()),
        os: os_text(),
        java_version: bundled_java_version(),
        memory_mb: upload.memory_mb,
        summary: one_line(&upload.summary, CRASH_SUMMARY_MAX_CHARS),
        files: Vec::new(),
    };
    loop {
        body.files = std::mem::take(&mut files);
        let json = serde_json::to_vec(&body)?;
        files = std::mem::take(&mut body.files);
        if json.len() <= CRASH_BODY_BUDGET {
            return Ok(json);
        }
        let Some(index) = files
            .iter()
            .enumerate()
            .max_by_key(|(_, file)| file.content.len())
            .map(|(index, _)| index)
        else {
            return Err(api_error("崩溃报告太大，无法上传"));
        };
        // Each round removes a file or cuts the largest one by at least a quarter.
        let excess = json.len() - CRASH_BODY_BUDGET;
        let current = files[index].content.len();
        let target = current
            .saturating_sub(excess + 16 * 1024)
            .min(current * 3 / 4);
        if target < 4 * 1024 {
            files.remove(index);
            continue;
        }
        let head = keeps_head(&files[index].name);
        files[index].content = trim_content(&files[index].content, target, head);
    }
}

/// Uploads a crash report to the server (POST /v1/crash) and returns its id.
#[tracing::instrument(skip(upload))]
pub async fn upload_crash(
    upload: GalaxyCrashUpload,
) -> crate::Result<GalaxyCrashUploadResult> {
    let url = format!("{}/v1/crash", require_base()?);
    let body = crash_body(upload)?;
    let client = crate::util::fetch::configured_client().await?;
    let request = async {
        let response = client
            .post(&url)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .body(body)
            .send()
            .await
            .map_err(|error| api_error(format!("连不上服务器：{error}")))?;
        let status = response.status();
        let text = read_limited(response, POST_RESPONSE_MAX_BYTES).await?;
        if !status.is_success() {
            return Err(api_error(error_text(&text).unwrap_or_else(|| {
                format!("服务器返回 {status}")
            })));
        }
        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|_| api_error("服务器的回复无法识别"))?;
        let id = value
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| api_error("服务器没有返回报告编号"))?;
        Ok(GalaxyCrashUploadResult { id: id.to_string() })
    };
    tokio::time::timeout(POST_TIMEOUT, request)
        .await
        .map_err(|_| api_error("上传超时，请检查网络后重试"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_base_is_normalized_and_checked() {
        assert_eq!(
            normalize_api_base(" http://mc9.yunmaoc.com:28095/ ").as_deref(),
            Some("http://mc9.yunmaoc.com:28095")
        );
        assert_eq!(
            normalize_api_base("https://example.com/api/").as_deref(),
            Some("https://example.com/api")
        );
        assert!(normalize_api_base("ftp://example.com").is_none());
        assert!(normalize_api_base("http://user:pw@example.com").is_none());
        assert!(normalize_api_base("http://example.com/?x=1").is_none());
        assert!(normalize_api_base("not a url").is_none());
    }

    #[test]
    fn only_contract_paths_are_allowed() {
        assert!(is_allowed_get_path("/v1/summary"));
        assert!(is_allowed_get_path("/v1/announcements"));
        assert!(is_allowed_get_path("/v1/health"));
        assert!(is_allowed_get_path("/v1/player/Steve_01"));
        assert!(!is_allowed_get_path("/v1/player/ab"));
        assert!(!is_allowed_get_path("/v1/player/a/b"));
        assert!(!is_allowed_get_path("/v1/player/../crash"));
        assert!(!is_allowed_get_path("/v1/crash"));
        assert!(!is_allowed_get_path("/v1/summary?x=1"));
        assert!(!is_allowed_get_path("http://evil/v1/summary"));
    }

    #[test]
    fn content_is_trimmed_on_char_boundaries() {
        let text = "崩溃".repeat(1000);
        let tail = trim_content(&text, 100, false);
        assert!(tail.starts_with("…（前面省略约"));
        assert!(tail.ends_with("崩溃"));
        let head = trim_content(&text, 100, true);
        assert!(head.starts_with("崩溃"));
        assert_eq!(trim_content("short", 100, true), "short");
        assert_eq!(safe_file_name("../latest.log", 0), "..latest.log");
        assert_eq!(safe_file_name("日志", 1), "file2.txt");
    }

    #[test]
    fn crash_body_fits_the_server_limit() {
        let big = "x".repeat(3 * 1024 * 1024);
        let upload = GalaxyCrashUpload {
            name: Some("bad name!".into()),
            summary: "  游戏\n崩溃  ".into(),
            modpack_version: Some("1.0.5".into()),
            memory_mb: Some(6144),
            files: vec![
                GalaxyCrashFile { name: "crash-1.txt".into(), content: big.clone() },
                GalaxyCrashFile { name: "latest.log".into(), content: big.clone() },
                GalaxyCrashFile { name: "debug.log".into(), content: big },
            ],
        };
        let json = crash_body(upload).unwrap();
        assert!(json.len() <= CRASH_BODY_BUDGET);
        let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert!(value["name"].is_null(), "invalid names are dropped");
        assert_eq!(value["summary"], "游戏 崩溃");
        assert_eq!(value["schema"], 1);
        assert!(!value["files"].as_array().unwrap().is_empty());
    }
}
