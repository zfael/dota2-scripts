use crate::ipc_types::MinimapStatusDto;
use crate::TauriAppState;

/// Returns current minimap capture status
#[tauri::command]
pub fn get_minimap_status(state: tauri::State<'_, TauriAppState>) -> Result<MinimapStatusDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    let dto = if let Some(ref snapshot) = app.minimap_capture {
        MinimapStatusDto {
            enabled: snapshot.enabled,
            health: snapshot.health.as_str().to_string(),
            capture_interval_ms: snapshot.capture_interval_ms,
            window_binding_status: snapshot.window_binding_status.clone(),
            consecutive_failures: snapshot.consecutive_failures,
            last_capture_duration_ms: snapshot.last_capture_duration_ms,
            sampling_mode: snapshot.sampling_mode.clone(),
        }
    } else {
        MinimapStatusDto {
            enabled: false,
            health: "idle".to_string(),
            capture_interval_ms: 0,
            window_binding_status: "unknown".to_string(),
            consecutive_failures: 0,
            last_capture_duration_ms: None,
            sampling_mode: "disabled".to_string(),
        }
    };

    Ok(dto)
}
