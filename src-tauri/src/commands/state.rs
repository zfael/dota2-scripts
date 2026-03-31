use crate::ipc_types::AppStateDto;
use crate::TauriAppState;

/// Returns current app state (selected hero, enabled flags)
#[tauri::command]
pub fn get_app_state(state: tauri::State<'_, TauriAppState>) -> Result<AppStateDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    Ok(AppStateDto {
        selected_hero: app.selected_hero.map(|h| h.to_display_name().to_string()),
        gsi_enabled: app.gsi_enabled,
        standalone_enabled: app.standalone_enabled,
    })
}
