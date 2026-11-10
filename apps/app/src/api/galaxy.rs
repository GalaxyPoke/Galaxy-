//! `plugin:galaxy|*` commands for the Galaxy宝可梦 frontend.
//! The contract lives in GALAXY_CONTRACT.md at the repository root.
use crate::api::Result;
use theseus::galaxy::{GalaxyBootstrap, GalaxyPlayer};
use theseus::galaxy_api::{GalaxyCrashUpload, GalaxyCrashUploadResult};
use theseus::galaxy_graphics::GalaxyGraphicsState;
use theseus::galaxy_sync::{GalaxySyncResult, GalaxySyncSnapshot};
use theseus::galaxy_update::GalaxyLauncherUpdate;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("galaxy")
        .invoke_handler(tauri::generate_handler![
            galaxy_bootstrap,
            galaxy_set_player,
            galaxy_launcher_update_check,
            galaxy_launcher_update_apply,
            galaxy_fetch_announcements,
            galaxy_sync_check,
            galaxy_sync_apply,
            galaxy_sync_status,
            galaxy_api_get,
            galaxy_crash_upload,
            galaxy_graphics_get,
            galaxy_graphics_apply,
        ])
        .build()
}

/// GETs one of the server data API's fixed paths (GALAXY_LAUNCHER_API.md):
/// `/v1/summary`, `/v1/announcements`, `/v1/health`, `/v1/player/{name}`.
#[tauri::command]
pub async fn galaxy_api_get(path: String) -> Result<String> {
    Ok(theseus::galaxy_api::get(&path).await?)
}

/// Uploads a crash report to the server; returns the report id.
#[tauri::command]
pub async fn galaxy_crash_upload(
    report: GalaxyCrashUpload,
) -> Result<GalaxyCrashUploadResult> {
    Ok(theseus::galaxy_api::upload_crash(report).await?)
}

/// Current graphics settings and the preset they match.
#[tauri::command]
pub async fn galaxy_graphics_get() -> Result<GalaxyGraphicsState> {
    Ok(theseus::galaxy_graphics::current()?)
}

/// Applies `low` / `balanced` / `high`, or `custom` (restore own settings).
#[tauri::command]
pub async fn galaxy_graphics_apply(
    preset: String,
) -> Result<GalaxyGraphicsState> {
    Ok(theseus::galaxy_graphics::apply(&preset).await?)
}

/// Finds the bundled `.minecraft`, links the fixed instance once and reuses
/// it afterwards.
#[tauri::command]
pub async fn galaxy_bootstrap() -> Result<GalaxyBootstrap> {
    Ok(theseus::galaxy::bootstrap().await?)
}

/// Creates or reuses the offline account `name` and makes it active.
#[tauri::command]
pub async fn galaxy_set_player(name: String) -> Result<GalaxyPlayer> {
    Ok(theseus::galaxy::set_player(&name).await?)
}

/// GETs the announcements JSON (http/https only, 10 s timeout, 1 MB cap).
#[tauri::command]
pub async fn galaxy_fetch_announcements(url: String) -> Result<String> {
    Ok(theseus::galaxy::fetch_announcements(&url).await?)
}

/// Reports whether the modpack sync has delivered a newer launcher build.
#[tauri::command]
pub async fn galaxy_launcher_update_check() -> Result<GalaxyLauncherUpdate> {
    Ok(theseus::galaxy_update::check().await?)
}

/// Verifies and stages the update, starts the swap helper, then exits so the
/// helper can replace the files and start the new launcher.
#[tauri::command]
pub async fn galaxy_launcher_update_apply<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<String> {
    let version = theseus::galaxy_update::apply().await?;
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        app.exit(0);
    });
    Ok(version)
}

/// Built-in modpack updater: fetches and verifies the signed release and
/// compares it with the instance (`force_rehash` ignores the hash cache).
#[tauri::command]
pub async fn galaxy_sync_check(
    force_rehash: Option<bool>,
) -> Result<GalaxySyncResult> {
    Ok(theseus::galaxy_sync::check(force_rehash.unwrap_or(false)).await?)
}

/// Downloads and installs the pending modpack update (refused while the game
/// runs). Progress arrives as `galaxy_sync_progress` events.
#[tauri::command]
pub async fn galaxy_sync_apply() -> Result<GalaxySyncResult> {
    Ok(theseus::galaxy_sync::apply().await?)
}

/// Last result and progress, for page reloads.
#[tauri::command]
pub async fn galaxy_sync_status() -> Result<GalaxySyncSnapshot> {
    Ok(theseus::galaxy_sync::status())
}
