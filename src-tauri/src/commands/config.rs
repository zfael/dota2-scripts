use crate::TauriAppState;
use dota2_scripts::config::Settings;

/// Returns the full config as JSON (snake_case keys matching config.toml)
#[tauri::command]
pub fn get_config(state: tauri::State<'_, TauriAppState>) -> Result<Settings, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;
    Ok(settings.clone())
}
