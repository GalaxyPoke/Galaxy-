//! Galaxy宝可梦: glue for the launcher's single fixed server and instance.
//!
//! The game ships as a PCL/HMCL-style `.minecraft` next to the launcher. On
//! start the frontend calls [`bootstrap`], which links that directory through
//! the existing "directly associated instance" machinery (version isolated,
//! nothing copied) and reuses the link on every later start.
//!
//! macOS is the exception: the launcher is a signed app bundle that may run
//! from a read-only location, so the game lives in
//! `~/Library/Application Support/GalaxyPokemon/.minecraft` and the copy
//! shipped in `Contents/Resources/.minecraft` is installed there on the first
//! start (`install_bundled_game`).

use crate::State;
use crate::api::pack::import::ImportLauncherType;
use crate::launcher::ExternalGameDirMode;
use crate::state::{
    CreateDirectLinkInstance, Credentials, EditInstance,
    InstanceLaunchOverridesPatch, InstanceMetadata,
};
use serde::Serialize;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const SERVER_ADDRESS: &str = "mc9.yunmaoc.com:27852";
pub const INSTANCE_NAME: &str = "Galaxy宝可梦";
/// Version JSON id; the isolated instance directory is `versions/<id>`.
pub const VERSION_ID: &str = "1.21.1-Fabric 0.19.3";
/// Overrides the `.minecraft` lookup (used during development).
pub const GAME_ROOT_ENV: &str = "GALAXY_GAME_ROOT";
/// Event emitted when the DreamingFish updater refuses to let the game start.
/// The launcher no longer loads the in-game agent (the built-in updater in
/// `galaxy_sync` replaced it), so this only fires if a version JSON or custom
/// arguments still smuggle it in; kept inert on purpose.
pub const LAUNCH_BLOCKED_EVENT: &str = "galaxy_launch_blocked";
pub const DREAMINGFISH_BLOCKED_MARKER: &str =
    "DreamingFish updater blocked Minecraft startup";

const DREAMINGFISH_DIR: &str = ".dreamingfish-bootstrap";
const DREAMINGFISH_AGENT_JAR: &str = "bootstrap-agent.jar";
/// macOS data folder under `~/Library/Application Support`; ASCII because
/// some mods break on non-ASCII game paths.
const MAC_DATA_DIR: &str = "GalaxyPokemon";

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalaxyBootstrap {
    pub instance_id: Option<String>,
    pub server_address: String,
    pub game_root: Option<String>,
    pub instance_dir: Option<String>,
    pub game_found: bool,
    /// The built-in modpack updater (`galaxy_sync`) is configured: the
    /// instance has `.dreamingfish-bootstrap/project-binding.json`, or the
    /// built-in server constants are used. True whenever the game is found.
    pub dreamingfish: bool,
    pub java_path: Option<String>,
    /// Base URL of the server data API (`galaxy_api`), `None` when this build
    /// has none configured: the home page then hides the features that need it.
    pub api_base: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalaxyPlayer {
    pub uuid: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalaxyLaunchBlockedPayload {
    pub instance_id: String,
    pub reason: String,
}

/// Serializes bootstraps so two concurrent calls cannot both create the link.
static BOOTSTRAP_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Finds the game root in the documented order: `GALAXY_GAME_ROOT`, then
/// `<exe dir>\..\.minecraft`, then `<exe dir>\.minecraft`. On macOS the exe
/// is inside the app bundle, so the last two are replaced by `mac_game_root`.
pub fn find_game_root() -> Option<PathBuf> {
    let env_root = std::env::var_os(GAME_ROOT_ENV);
    let candidates = if cfg!(target_os = "macos") {
        let mut candidates = candidate_game_roots(env_root, None);
        candidates.extend(mac_game_root());
        candidates
    } else {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf));
        candidate_game_roots(env_root, exe_dir.as_deref())
    };
    candidates.into_iter().find(|candidate| candidate.is_dir())
}

fn candidate_game_roots(
    env_root: Option<OsString>,
    exe_dir: Option<&Path>,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(root) = env_root.filter(|root| !root.is_empty()) {
        candidates.push(PathBuf::from(root));
    }
    if let Some(exe_dir) = exe_dir {
        if let Some(parent) = exe_dir.parent() {
            candidates.push(parent.join(".minecraft"));
        }
        candidates.push(exe_dir.join(".minecraft"));
    }
    candidates
}

/// macOS: where the game lives, `~/Library/Application Support/GalaxyPokemon/.minecraft`.
fn mac_game_root() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join(MAC_DATA_DIR).join(".minecraft"))
}

/// macOS: the modpack shipped in the app bundle, `Contents/Resources/.minecraft`.
fn bundled_game_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.parent()?.join("Resources").join(".minecraft"))
}

/// macOS: on the first start, installs the bundled modpack to
/// `mac_game_root`. It is copied to a `.partial` sibling and renamed once
/// complete, so an interrupted copy starts over on the next start. Later
/// modpack changes arrive through `galaxy_sync`, never from the bundle.
async fn install_bundled_game() -> Result<(), String> {
    let (Some(target), Some(source)) = (mac_game_root(), bundled_game_root())
    else {
        return Ok(());
    };
    if target.is_dir() || !source.is_dir() {
        return Ok(());
    }
    let _guard = BOOTSTRAP_LOCK.lock().await;
    tokio::task::spawn_blocking(move || {
        if target.is_dir() {
            return Ok(());
        }
        let partial = target.with_extension("partial");
        if partial.exists() {
            std::fs::remove_dir_all(&partial)?;
        }
        tracing::info!(
            from = %source.display(),
            to = %target.display(),
            "Installing the bundled Galaxy game"
        );
        copy_tree(&source, &partial)?;
        std::fs::rename(&partial, &target)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

/// Recursive copy of regular files and directories; symlinks are skipped.
/// `std::fs::copy` clones files on APFS, so this is quick on the same volume.
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = to.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

pub fn instance_dir_for(root: &Path) -> PathBuf {
    root.join("versions").join(VERSION_ID)
}

fn version_json_for(root: &Path) -> PathBuf {
    instance_dir_for(root).join(format!("{VERSION_ID}.json"))
}

/// The Java 21 runtime bundled with the modpack, if present.
pub fn bundled_java_for(root: &Path) -> Option<PathBuf> {
    let bin = root.join("runtime").join("java-runtime-delta").join("bin");
    let java = if cfg!(windows) {
        bin.join("javaw.exe")
    } else {
        bin.join("java")
    };
    java.is_file().then_some(java)
}

/// Removes the `\\?\` prefix `std::fs::canonicalize` adds on Windows; Java
/// does not accept it in `-javaagent:`.
pub(crate) fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else if let Some(stripped) = text.strip_prefix(r"\\?\") {
        stripped.to_string()
    } else {
        text.into_owned()
    }
}

fn canonical_or_self(path: &Path) -> PathBuf {
    crate::util::io::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn same_path(left: &Path, right: &Path) -> bool {
    let normalize = |path: &Path| {
        let text = display_path(&canonical_or_self(path)).replace('/', "\\");
        let text = text.trim_end_matches('\\').to_string();
        if cfg!(windows) {
            text.to_lowercase()
        } else {
            text
        }
    };
    normalize(left) == normalize(right)
}

fn is_galaxy_link(
    metadata: &InstanceMetadata,
    root: &Path,
    version_json: &Path,
) -> bool {
    let instance = &metadata.instance;
    if !instance.is_direct_linked() {
        return false;
    }
    if let Some(json) = instance.linked_version_json_path.as_deref()
        && same_path(Path::new(json), version_json)
    {
        return true;
    }
    instance.linked_version_id.as_deref() == Some(VERSION_ID)
        && instance
            .linked_dot_minecraft
            .as_deref()
            .is_some_and(|linked| same_path(Path::new(linked), root))
}

/// True for the record [`bootstrap`] owns. Direct-link reconciliation
/// (`sync_direct_link_instances`, run by the frontend on start and on every
/// window focus) only knows the roots configured in Settings and would
/// otherwise drop this link as "root no longer configured", which is what made
/// the id returned by `bootstrap` unknown to `run`.
pub(crate) fn is_bootstrap_link(metadata: &InstanceMetadata) -> bool {
    if !metadata.instance.is_direct_linked() {
        return false;
    }
    let Some(root) = find_game_root() else {
        return false;
    };
    let root = canonical_or_self(&root);
    is_galaxy_link(metadata, &root, &version_json_for(&root))
}

/// Idempotently links the bundled game and returns what the frontend needs.
/// Expected problems (missing game directory, missing version) are reported
/// in `error` rather than as a failed command.
#[tracing::instrument]
pub async fn bootstrap() -> crate::Result<GalaxyBootstrap> {
    let mut result = GalaxyBootstrap {
        server_address: SERVER_ADDRESS.to_string(),
        api_base: crate::api::galaxy_api::api_base(),
        ..GalaxyBootstrap::default()
    };

    let installed = if cfg!(target_os = "macos") {
        install_bundled_game().await
    } else {
        Ok(())
    };
    if let Err(error) = installed {
        tracing::error!(%error, "Installing the bundled Galaxy game failed");
        result.error = Some(format!(
            "准备游戏文件失败：{error}。请确认磁盘空间足够，然后重新打开启动器。"
        ));
        return Ok(result);
    }
    let Some(root) = find_game_root() else {
        result.error = Some(if cfg!(target_os = "macos") {
            format!(
                "未找到游戏目录（{}）。请重新下载 Mac 安装包，把 {INSTANCE_NAME} 拖进「应用程序」后再打开。",
                mac_game_root().as_deref().map(display_path).unwrap_or_default()
            )
        } else {
            format!(
                "未找到游戏目录（.minecraft）。请确认启动器位于 {INSTANCE_NAME} 安装目录中，或设置环境变量 {GAME_ROOT_ENV}。"
            )
        });
        return Ok(result);
    };
    let root = canonical_or_self(&root);
    result.game_root = Some(display_path(&root));

    let instance_dir = instance_dir_for(&root);
    let version_json = version_json_for(&root);
    if !version_json.is_file() {
        result.error = Some(format!(
            "游戏目录中缺少版本 {VERSION_ID}（{}）。请重新安装或修复游戏文件。",
            display_path(&version_json)
        ));
        return Ok(result);
    }
    result.game_found = true;
    result.instance_dir = Some(display_path(&instance_dir));
    // The built-in updater falls back to its constants when the instance has
    // no project-binding.json, so it is always available once the game is.
    result.dreamingfish = true;
    if !instance_dir
        .join(DREAMINGFISH_DIR)
        .join("project-binding.json")
        .is_file()
    {
        tracing::info!("No project-binding.json; the built-in updater uses its built-in server settings");
    }
    let java = bundled_java_for(&root).map(|java| display_path(&java));
    result.java_path.clone_from(&java);

    match link_instance(&root, &version_json, java.as_deref()).await {
        Ok(instance_id) => {
            crate::anti_piracy::exempt_instance(&instance_id);
            result.instance_id = Some(instance_id);
        }
        Err(error) => {
            tracing::error!(%error, "Galaxy instance bootstrap failed");
            result.error = Some(format!(
                "关联游戏实例失败：{}",
                error.user_facing_message()
            ));
        }
    }
    Ok(result)
}

async fn link_instance(
    root: &Path,
    version_json: &Path,
    java: Option<&str>,
) -> crate::Result<String> {
    let _guard = BOOTSTRAP_LOCK.lock().await;

    let existing = crate::api::instance::list()
        .await?
        .into_iter()
        .filter(|metadata| is_galaxy_link(metadata, root, version_json))
        .min_by_key(|metadata| metadata.instance.created);

    let metadata = match existing {
        Some(metadata) => metadata,
        None => {
            tracing::info!(root = %root.display(), "Linking the Galaxy宝可梦 game directory");
            crate::api::instance::create_with_direct_link(
                CreateDirectLinkInstance {
                    name: Some(INSTANCE_NAME.to_string()),
                    launcher_type: ImportLauncherType::Generic,
                    base_path: root.to_path_buf(),
                    instance_folder: format!("versions/{VERSION_ID}"),
                    instance_path: None,
                    game_dir_mode: Some(ExternalGameDirMode::Isolated),
                },
            )
            .await?
        }
    };
    let instance_id = metadata.instance.id.clone();

    // Keep the bundled Java selected; also drop an override that points at a
    // runtime that no longer exists (e.g. the install folder was moved).
    let current = metadata.launch_overrides.java_path.as_deref();
    let desired = match java {
        Some(java) if current != Some(java) => Some(Some(java.to_string())),
        None if current.is_some_and(|path| !Path::new(path).is_file()) => {
            Some(None)
        }
        _ => None,
    };
    if let Some(java_path) = desired {
        crate::api::instance::edit(
            &instance_id,
            EditInstance {
                launch_overrides: Some(InstanceLaunchOverridesPatch {
                    java_path: Some(java_path),
                    ..InstanceLaunchOverridesPatch::default()
                }),
                ..EditInstance::default()
            },
        )
        .await?;
    }

    Ok(instance_id)
}

pub fn is_valid_player_name(name: &str) -> bool {
    (3..=16).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// Creates or reuses the offline account for `name` and makes it the active
/// account. Galaxy宝可梦 is offline-only, so this does not go through the
/// anti-piracy "official login first" gate that `add_offline_user` applies.
#[tracing::instrument]
pub async fn set_player(name: &str) -> crate::Result<GalaxyPlayer> {
    let name = name.trim();
    if !is_valid_player_name(name) {
        return Err(crate::ErrorKind::InputError(
            "游戏名需为 3–16 位，只能包含英文字母、数字和下划线".to_string(),
        )
        .as_error());
    }

    let state = State::get().await?;
    let mut credentials = Credentials::offline(name)?;
    let account_id = credentials.account_id();

    for mut user in Credentials::get_all_without_refresh(&state.data).await? {
        if user.active && user.account_id() != account_id {
            user.active = false;
            user.upsert(&state.data).await?;
        }
    }
    credentials.active = true;
    credentials.upsert(&state.data).await?;

    Ok(GalaxyPlayer {
        uuid: credentials.offline_profile.id.as_hyphenated().to_string(),
    })
}

/// Largest announcements response accepted by [`fetch_announcements`].
pub const ANNOUNCEMENTS_MAX_BYTES: usize = 1024 * 1024;
const ANNOUNCEMENTS_TIMEOUT: Duration = Duration::from_secs(10);

fn announcements_error(message: impl Into<String>) -> crate::Error {
    crate::ErrorKind::OtherError(message.into()).as_error()
}

/// GETs the announcements JSON for the frontend (the webview CSP only allows a
/// fixed list of hosts). The URL is a constant in the frontend
/// (`GALAXY_ANNOUNCEMENTS_URL`); only http/https without credentials is
/// accepted and the body is capped at [`ANNOUNCEMENTS_MAX_BYTES`].
#[tracing::instrument]
pub async fn fetch_announcements(url: &str) -> crate::Result<String> {
    let parsed = url::Url::parse(url.trim())
        .map_err(|_| announcements_error("公告地址无效"))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(announcements_error("公告地址只支持 http / https"));
    }

    let client = crate::util::fetch::configured_client().await?;
    let request = async {
        let mut response = client
            .get(parsed)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|error| announcements_error(format!("无法连接公告服务器：{error}")))?;
        if !response.status().is_success() {
            return Err(announcements_error(format!(
                "公告服务器返回 {}",
                response.status()
            )));
        }
        if response
            .content_length()
            .is_some_and(|length| length > ANNOUNCEMENTS_MAX_BYTES as u64)
        {
            return Err(announcements_error("公告内容过大"));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| announcements_error(format!("读取公告失败：{error}")))?
        {
            if body.len() + chunk.len() > ANNOUNCEMENTS_MAX_BYTES {
                return Err(announcements_error("公告内容过大"));
            }
            body.extend_from_slice(&chunk);
        }
        String::from_utf8(body)
            .map_err(|_| announcements_error("公告内容不是 UTF-8 文本"))
    };
    tokio::time::timeout(ANNOUNCEMENTS_TIMEOUT, request)
        .await
        .map_err(|_| announcements_error("获取公告超时"))?
}

/// True for a `-javaagent:` that loads the DreamingFish in-game updater.
pub(crate) fn is_dreamingfish_agent_arg(arg: &str) -> bool {
    let arg = arg.trim().to_ascii_lowercase().replace('\\', "/");
    arg.starts_with("-javaagent:")
        && arg.contains(&format!(
            "{}/{}",
            DREAMINGFISH_DIR, DREAMINGFISH_AGENT_JAR
        ))
}

/// Drops DreamingFish `-javaagent:` arguments a version JSON or custom JVM
/// arguments may still carry: the launcher's built-in updater has already
/// synced the pack, and the agent would open its own window and block the
/// game. Returns how many were removed.
pub(crate) fn strip_dreamingfish_agent(args: &mut Vec<String>) -> usize {
    let before = args.len();
    args.retain(|arg| !is_dreamingfish_agent_arg(arg));
    before - args.len()
}

/// Text after the colon that follows the DreamingFish block marker.
pub(crate) fn launch_blocked_reason(line: &str) -> Option<String> {
    let start = line.find(DREAMINGFISH_BLOCKED_MARKER)?;
    let rest = &line[start + DREAMINGFISH_BLOCKED_MARKER.len()..];
    let reason = rest
        .find([':', '：'])
        .map(|index| {
            let colon_len = rest[index..].chars().next().map_or(1, char::len_utf8);
            rest[index + colon_len..].trim()
        })
        .unwrap_or("");
    Some(if reason.is_empty() {
        line.trim().to_string()
    } else {
        reason.to_string()
    })
}

/// Called for every line of game output. Emits [`LAUNCH_BLOCKED_EVENT`] once
/// per burst (the same line can arrive through stdout and the log4j stream).
pub(crate) fn observe_output_line(instance_id: &str, line: &str) {
    let Some(reason) = launch_blocked_reason(line) else {
        return;
    };

    static LAST: Mutex<Option<(String, String, Instant)>> = Mutex::new(None);
    if let Ok(mut last) = LAST.lock() {
        if last.as_ref().is_some_and(|(id, previous, at)| {
            id == instance_id
                && *previous == reason
                && at.elapsed() < Duration::from_secs(30)
        }) {
            return;
        }
        *last = Some((instance_id.to_string(), reason.clone(), Instant::now()));
    }

    tracing::warn!(instance_id, %reason, "DreamingFish updater blocked Minecraft startup");
    crate::event::emit::emit_galaxy_launch_blocked(GalaxyLaunchBlockedPayload {
        instance_id: instance_id.to_string(),
        reason,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_root_candidates_follow_the_documented_order() {
        let exe_dir = Path::new(r"C:\Galaxy\launcher");
        let candidates = candidate_game_roots(
            Some(OsString::from(r"D:\dev\.minecraft")),
            Some(exe_dir),
        );
        assert_eq!(
            candidates,
            vec![
                PathBuf::from(r"D:\dev\.minecraft"),
                PathBuf::from(r"C:\Galaxy").join(".minecraft"),
                exe_dir.join(".minecraft"),
            ]
        );
        assert_eq!(
            candidate_game_roots(Some(OsString::new()), Some(exe_dir)).len(),
            2
        );
    }

    #[test]
    fn player_names_match_the_server_rules() {
        for name in ["abc", "Steve", "player_01", "A234567890123456"] {
            assert!(is_valid_player_name(name), "{name}");
        }
        for name in ["", "ab", "A2345678901234567", "has space", "名字", "a-b"]
        {
            assert!(!is_valid_player_name(name), "{name}");
        }
    }

    #[test]
    fn offline_uuid_matches_java_name_uuid_from_bytes() {
        // UUID.nameUUIDFromBytes("OfflinePlayer:Notch".getBytes(UTF_8))
        let credentials = Credentials::offline("Notch").unwrap();
        assert_eq!(
            credentials.offline_profile.id.as_hyphenated().to_string(),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
    }

    #[test]
    fn blocked_reason_is_the_text_after_the_colon() {
        assert_eq!(
            launch_blocked_reason(
                "[12:00:01] [main/ERROR]: DreamingFish updater blocked Minecraft startup: 无法连接更新服务器"
            )
            .as_deref(),
            Some("无法连接更新服务器")
        );
        assert_eq!(
            launch_blocked_reason(
                "DreamingFish updater blocked Minecraft startup：需要更新"
            )
            .as_deref(),
            Some("需要更新")
        );
        assert_eq!(
            launch_blocked_reason("DreamingFish updater blocked Minecraft startup")
                .as_deref(),
            Some("DreamingFish updater blocked Minecraft startup")
        );
        assert_eq!(launch_blocked_reason("[main/INFO]: Loading"), None);
    }

    #[test]
    fn dreamingfish_agent_args_are_stripped() {
        let mut args = vec![
            "-Xmx4G".to_string(),
            r"-javaagent:D:\Galaxy\.minecraft\versions\1.21.1-Fabric 0.19.3\.dreamingfish-bootstrap\bootstrap-agent.jar".to_string(),
            "-javaagent:/opt/x/.DreamingFish-Bootstrap/Bootstrap-Agent.jar=debug".to_string(),
            "-javaagent:/opt/authlib-injector.jar=https://example".to_string(),
        ];
        assert_eq!(strip_dreamingfish_agent(&mut args), 2);
        assert_eq!(
            args,
            vec![
                "-Xmx4G".to_string(),
                "-javaagent:/opt/authlib-injector.jar=https://example".to_string(),
            ]
        );
        assert_eq!(strip_dreamingfish_agent(&mut args), 0);
    }

    #[test]
    fn verbatim_prefix_is_removed() {
        assert_eq!(
            display_path(Path::new(r"\\?\D:\Galaxy\.minecraft")),
            r"D:\Galaxy\.minecraft"
        );
        assert_eq!(
            display_path(Path::new(r"\\?\UNC\server\share\.minecraft")),
            r"\\server\share\.minecraft"
        );
    }
}
