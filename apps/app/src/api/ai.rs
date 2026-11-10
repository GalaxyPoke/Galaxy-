use crate::api::Result;
use theseus::ai::{self, AiProviderDefinition, AiState};

// Galaxy宝可梦 (slimming phase 2): the AI settings page is hidden, so only the
// read-only state used by the crash modal and the translation settings is
// registered. The removed commands (provider/model/credential/OAuth editing)
// are listed in GALAXY_FORK.md, "Slimming phase 2 (Rust)".
pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("ai")
        .invoke_handler(tauri::generate_handler![ai_get_catalog, ai_get_state])
        .build()
}

#[tauri::command]
pub fn ai_get_catalog() -> Vec<AiProviderDefinition> {
    ai::catalog()
}

#[tauri::command]
pub async fn ai_get_state() -> Result<AiState> {
    Ok(ai::get_state().await?)
}
