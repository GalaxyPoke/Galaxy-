//! Galaxy宝可梦: launcher self-update through the DreamingFish channel.
//!
//! The release script (`dfs-release.ps1`) puts `launcher-update/latest.json`
//! and a zip of the whole `launcher\` folder into the modpack. The launcher's
//! built-in modpack updater (`galaxy_sync`) verifies and syncs it into the
//! instance directory like any other pack file, so no extra port or download
//! server is needed. After a sync that changed `launcher-update/*` (and on
//! every start) the frontend calls [`check`]; if the user accepts, [`apply`] verifies the
//! SHA-256, extracts the zip to `<launcher>\.galaxy-update`, and starts a
//! hidden PowerShell that waits for the launcher to exit, copies the files
//! over `launcher\` with robocopy and starts the new `GalaxyPokemon.exe`.
//!
//! The installed version is read from `galaxy-launcher.json` next to the exe
//! (on macOS from the bundle's `Contents/Resources`, see `launcher_dir`).
//! Builds without that file (dev builds in `target\`) never update, and only
//! Windows builds offer the synced update, which is the Windows launcher.
//!
//! `latest.json`:
//! `{ "version": "1.0.1", "notes": "...", "file": "launcher.zip", "sha256": "<hex>", "size": 123 }`

use crate::api::galaxy::{display_path, find_game_root, instance_dir_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

pub const VERSION_FILE: &str = "galaxy-launcher.json";
const UPDATE_DIR: &str = "launcher-update";
const MANIFEST_FILE: &str = "latest.json";
const STAGING_DIR: &str = ".galaxy-update";
const LAUNCHER_EXE: &str = "GalaxyPokemon.exe";
const UPDATE_LOG: &str = "galaxy-update.log";

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalaxyLauncherUpdate {
    /// `None` when this build has no `galaxy-launcher.json` (self-update off).
    pub current_version: Option<String>,
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub size: Option<u64>,
}

#[derive(Deserialize)]
struct VersionFile {
    version: String,
}

/// Optional fields of `galaxy-launcher.json` besides the version.
#[derive(Deserialize)]
struct LauncherInfo {
    /// Base URL of the server data API (GALAXY_LAUNCHER_API.md), written by
    /// `galaxy-stage.ps1 -ApiBase`.
    #[serde(default, rename = "apiBase")]
    api_base: Option<String>,
}

/// `apiBase` from `galaxy-launcher.json` next to the exe, unvalidated.
pub fn configured_api_base() -> Option<String> {
    let info: LauncherInfo = read_json(&launcher_dir()?.join(VERSION_FILE))?;
    info.api_base.filter(|base| !base.trim().is_empty())
}

#[derive(Clone, Debug, Deserialize)]
struct Manifest {
    version: String,
    #[serde(default)]
    notes: String,
    file: String,
    sha256: String,
    #[serde(default)]
    size: Option<u64>,
}

fn other(message: impl Into<String>) -> crate::Error {
    crate::ErrorKind::OtherError(message.into()).as_error()
}

/// Folder holding `galaxy-launcher.json`: next to the exe, except on macOS
/// where it is the bundle's `Contents/Resources` (a data file beside the
/// binary in `Contents/MacOS` would be treated as code when signing).
fn launcher_dir() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    if cfg!(target_os = "macos") {
        return Some(exe_dir.parent()?.join("Resources"));
    }
    Some(exe_dir)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()
}

fn read_version(dir: &Path) -> Option<String> {
    let parsed: VersionFile = read_json(&dir.join(VERSION_FILE))?;
    let version = parsed.version.trim().to_string();
    (!version.is_empty()).then_some(version)
}

/// Version of the running launcher, from `galaxy-launcher.json`.
pub fn current_version() -> Option<String> {
    read_version(&launcher_dir()?)
}

/// Numeric comparison of dotted versions (`1.0.10` > `1.0.9`); missing parts
/// count as 0 and non-numeric parts as 0.
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

fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains(['/', '\\', ':'])
        && name != "."
        && name != ".."
}

fn read_manifest(dir: &Path) -> Option<(Manifest, PathBuf)> {
    let manifest: Manifest = read_json(&dir.join(MANIFEST_FILE))?;
    let valid_hash = manifest.sha256.len() == 64
        && manifest.sha256.bytes().all(|byte| byte.is_ascii_hexdigit());
    if manifest.version.trim().is_empty()
        || !valid_hash
        || !is_plain_file_name(&manifest.file)
    {
        tracing::warn!("Ignoring malformed launcher update manifest");
        return None;
    }
    let payload = dir.join(&manifest.file);
    payload.is_file().then_some((manifest, payload))
}

/// The update to offer, if the synced manifest is newer than this build.
/// The synced payload is the Windows `launcher\` folder, so other platforms
/// never offer it.
fn pending() -> Option<(String, Manifest, PathBuf)> {
    if !cfg!(windows) {
        return None;
    }
    let current = current_version()?;
    let dir = instance_dir_for(&find_game_root()?).join(UPDATE_DIR);
    let (manifest, payload) = read_manifest(&dir)?;
    (compare_versions(&manifest.version, &current) == Ordering::Greater)
        .then_some((current, manifest, payload))
}

#[tracing::instrument]
pub async fn check() -> crate::Result<GalaxyLauncherUpdate> {
    tokio::task::spawn_blocking(|| {
        let mut result = GalaxyLauncherUpdate {
            current_version: current_version(),
            ..GalaxyLauncherUpdate::default()
        };
        if let Some((_, manifest, payload)) = pending() {
            result.available = true;
            result.notes = Some(manifest.notes.trim().to_string())
                .filter(|notes| !notes.is_empty());
            result.size = manifest.size.or_else(|| {
                std::fs::metadata(&payload).ok().map(|meta| meta.len())
            });
            result.version = Some(manifest.version);
        }
        result
    })
    .await
    .map_err(|error| other(format!("检查启动器更新失败：{error}")))
}

/// Verifies and stages the pending update, then starts the helper that swaps
/// the files once this process exits. The caller must exit the app afterwards.
#[tracing::instrument]
pub async fn apply() -> crate::Result<String> {
    tokio::task::spawn_blocking(apply_blocking)
        .await
        .map_err(|error| other(format!("启动器更新失败：{error}")))?
}

fn apply_blocking() -> crate::Result<String> {
    let launcher =
        launcher_dir().ok_or_else(|| other("找不到启动器所在的文件夹"))?;
    let (_, manifest, payload) =
        pending().ok_or_else(|| other("没有可用的启动器更新"))?;

    let mut file = std::fs::File::open(&payload)
        .map_err(|error| other(format!("无法读取更新文件：{error}")))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)
        .map_err(|error| other(format!("无法读取更新文件：{error}")))?;
    let digest = format!("{:x}", hasher.finalize());
    if !digest.eq_ignore_ascii_case(&manifest.sha256) {
        return Err(other(
            "更新文件校验失败（SHA-256 不一致）。请在首页点「修复游戏文件」重新同步后再试。",
        ));
    }

    let staging = launcher.join(STAGING_DIR);
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|error| {
            other(format!("无法清理旧的更新临时文件：{error}"))
        })?;
    }
    std::fs::create_dir_all(&staging).map_err(|error| {
        other(format!(
            "启动器目录没有写入权限，请把游戏装到非系统目录（例如 D:\\Games）：{error}"
        ))
    })?;

    let staged = extract(&payload, &staging).and_then(|()| {
        if !staging.join(LAUNCHER_EXE).is_file() {
            return Err(format!("更新包里缺少 {LAUNCHER_EXE}"));
        }
        match read_version(&staging) {
            Some(version)
                if compare_versions(&version, &manifest.version)
                    == Ordering::Equal =>
            {
                Ok(())
            }
            _ => Err("更新包的版本号和更新清单不一致".to_string()),
        }
    });
    if let Err(message) = staged {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(other(format!("启动器更新失败：{message}")));
    }

    spawn_swap(&launcher, &staging)?;
    tracing::info!(version = %manifest.version, "Launcher update staged; exiting to apply");
    Ok(manifest.version)
}

fn extract(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path)
        .map_err(|error| format!("无法打开更新包：{error}"))?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(file))
        .map_err(|error| format!("更新包已损坏：{error}"))?;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("更新包已损坏：{error}"))?;
        let Some(relative) = entry.enclosed_name() else {
            return Err(format!("更新包里第 {} 个文件的路径不安全", index + 1));
        };
        let out = dest.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)
                .map_err(|error| format!("解压失败：{error}"))?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("解压失败：{error}"))?;
        }
        let mut writer = std::fs::File::create(&out)
            .map_err(|error| format!("解压失败：{error}"))?;
        std::io::copy(&mut entry, &mut writer)
            .map_err(|error| format!("解压失败：{error}"))?;
    }
    Ok(())
}

#[cfg(windows)]
fn spawn_swap(launcher: &Path, staging: &Path) -> crate::Result<()> {
    use base64::Engine;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;

    let quote = |path: &Path| format!("'{}'", display_path(path).replace('\'', "''"));
    let script = [
        "$ErrorActionPreference = 'Continue'".to_string(),
        format!("$launcher = {}", quote(launcher)),
        format!("$staging = {}", quote(staging)),
        format!("$exe = {}", quote(&launcher.join(LAUNCHER_EXE))),
        format!("$log = {}", quote(&launcher.join(UPDATE_LOG))),
        format!(
            "try {{ Wait-Process -Id {} -Timeout 90 -ErrorAction SilentlyContinue }} catch {{}}",
            std::process::id()
        ),
        "Start-Sleep -Milliseconds 800".to_string(),
        "$ok = $false; $code = -1".to_string(),
        "for ($i = 0; $i -lt 20; $i++) { & robocopy.exe $staging $launcher /E /R:2 /W:1 /NFL /NDL /NJH /NJS /NP | Out-Null; $code = $LASTEXITCODE; if ($code -lt 8) { $ok = $true; break }; Start-Sleep -Seconds 1 }".to_string(),
        "try { \"$(Get-Date -Format s) update ok=$ok robocopy=$code\" | Out-File -LiteralPath $log -Append -Encoding utf8 } catch {}".to_string(),
        "if ($ok) { Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue }".to_string(),
        "Start-Process -FilePath $exe -WorkingDirectory $launcher".to_string(),
    ]
    .join("\n");
    let utf16: Vec<u8> = script
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(utf16);

    let spawn = |flags: u32| {
        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-WindowStyle",
                "Hidden",
                "-EncodedCommand",
                &encoded,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(flags)
            .spawn()
    };
    let base = CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW;
    spawn(base | CREATE_BREAKAWAY_FROM_JOB)
        .or_else(|_| spawn(base))
        .map(|_| ())
        .map_err(|error| other(format!("无法启动更新程序：{error}")))
}

#[cfg(not(windows))]
fn spawn_swap(_launcher: &Path, _staging: &Path) -> crate::Result<()> {
    Err(other("启动器自动更新只支持 Windows"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_numerically() {
        assert_eq!(compare_versions("1.0.10", "1.0.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("v1.2.0", "1.10.0"), Ordering::Less);
    }

    #[test]
    fn rejects_paths_in_manifest_file() {
        assert!(is_plain_file_name("launcher.zip"));
        assert!(!is_plain_file_name("../launcher.zip"));
        assert!(!is_plain_file_name("a\\b.zip"));
        assert!(!is_plain_file_name(".."));
    }
}
