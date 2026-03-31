use crate::ipc_types::{DiagnosticsDto, QueueMetricsDto, SyntheticInputDto};
use crate::TauriAppState;

/// Returns diagnostics: GSI metrics, synthetic input, keyboard state
#[tauri::command]
pub fn get_diagnostics(state: tauri::State<'_, TauriAppState>) -> Result<DiagnosticsDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    Ok(DiagnosticsDto {
        gsi_connected: app.last_event.is_some(),
        keyboard_hook_active: true,
        queue_metrics: QueueMetricsDto {
            events_processed: app.metrics.events_processed,
            events_dropped: app.metrics.events_dropped,
            current_queue_depth: app.metrics.current_queue_depth,
            max_queue_depth: 10,
        },
        synthetic_input: SyntheticInputDto {
            queue_depth: 0,
            total_queued: 0,
            peak_depth: 0,
            completions: 0,
            drops: 0,
        },
        soul_ring_state: "ready".to_string(),
        blocked_keys: vec![],
    })
}
