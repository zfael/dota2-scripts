use crate::ipc_types::UpdateStateDto;
use crate::TauriAppState;

/// Returns current update check state
#[tauri::command]
pub fn get_update_state(state: tauri::State<'_, TauriAppState>) -> Result<UpdateStateDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    let update_state = app
        .update_state
        .lock()
        .map_err(|e| format!("Failed to lock update state: {}", e))?;

    let dto = match &*update_state {
        dota2_scripts::state::UpdateCheckState::Idle => UpdateStateDto::Idle,
        dota2_scripts::state::UpdateCheckState::Checking => UpdateStateDto::Checking,
        dota2_scripts::state::UpdateCheckState::Available {
            version,
            release_notes,
        } => UpdateStateDto::Available {
            version: version.clone(),
            release_notes: release_notes.clone(),
        },
        dota2_scripts::state::UpdateCheckState::Downloading => UpdateStateDto::Downloading,
        dota2_scripts::state::UpdateCheckState::Error(msg) => UpdateStateDto::Error {
            message: msg.clone(),
        },
        dota2_scripts::state::UpdateCheckState::UpToDate => UpdateStateDto::UpToDate,
    };

    Ok(dto)
}
