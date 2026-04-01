use crate::ipc_types::{DiagnosticsDto, QueueMetricsDto, SyntheticInputDto};
use crate::TauriAppState;
use dota2_scripts::actions::SOUL_RING_STATE;

/// Returns diagnostics: GSI metrics, synthetic input, keyboard state
#[tauri::command]
pub fn get_diagnostics(state: tauri::State<'_, TauriAppState>) -> Result<DiagnosticsDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    Ok(DiagnosticsDto {
        gsi_connected: app.has_recent_gsi_activity(),
        keyboard_hook_active: true,
        queue_metrics: QueueMetricsDto {
            events_processed: app.metrics.events_processed,
            events_dropped: app.metrics.events_dropped,
            current_queue_depth: app.metrics.current_queue_depth,
            max_queue_depth: 10,
        },
        synthetic_input: {
            let snap = state.executor_metrics.snapshot();
            SyntheticInputDto {
                queue_depth: snap.queue_depth as usize,
                total_queued: snap.total_queued,
                peak_depth: 0,
                completions: snap.completions,
                drops: snap.drops,
            }
        },
        soul_ring_state: {
            match SOUL_RING_STATE.lock() {
                Ok(sr) => {
                    if !sr.available {
                        "unavailable".to_string()
                    } else if !sr.can_cast {
                        "cooldown".to_string()
                    } else {
                        "ready".to_string()
                    }
                }
                Err(_) => "unknown".to_string(),
            }
        },
        blocked_keys: {
            let mut keys = Vec::new();
            if app.sf_enabled.lock().map(|v| *v).unwrap_or(false) {
                keys.extend(["Q", "W", "E"].iter().map(|s| s.to_string()));
            }
            if app.od_enabled.lock().map(|v| *v).unwrap_or(false) {
                keys.push("R".to_string());
            }
            if let Ok(sr) = SOUL_RING_STATE.lock() {
                if sr.available && sr.can_cast {
                    if let Some(key) = sr.slot_key {
                        keys.push(format!("SoulRing({})", key));
                    }
                }
            }
            keys
        },
    })
}
