use crate::config::{MinimapCaptureConfig, Settings};
use crate::observability::minimap_capture_state::{
    MinimapCaptureHealth, MinimapCaptureStatusSnapshot,
};
use crate::state::AppState;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureAttemptResult {
    WindowNotFound,
    CaptureFailed(String),
    Success,
}

pub fn process_capture_attempt(
    config: &MinimapCaptureConfig,
    result: CaptureAttemptResult,
    last_artifact_path: Option<String>,
    previous_failures: u32,
) -> MinimapCaptureStatusSnapshot {
    match result {
        CaptureAttemptResult::Success => MinimapCaptureStatusSnapshot {
            enabled: config.enabled,
            capture_interval_ms: config.capture_interval_ms,
            last_success_at: Some("success".to_string()),
            last_failure_at: None,
            consecutive_failures: 0,
            last_capture_duration_ms: Some(0),
            last_artifact_path,
            sampling_mode: format!("every-{}", config.sample_every_n),
            window_binding_status: "bound".to_string(),
            health: MinimapCaptureHealth::Healthy,
        },
        CaptureAttemptResult::WindowNotFound => MinimapCaptureStatusSnapshot {
            enabled: config.enabled,
            capture_interval_ms: config.capture_interval_ms,
            last_success_at: None,
            last_failure_at: Some("window-not-found".to_string()),
            consecutive_failures: previous_failures + 1,
            last_capture_duration_ms: None,
            last_artifact_path,
            sampling_mode: format!("every-{}", config.sample_every_n),
            window_binding_status: "window-not-found".to_string(),
            health: MinimapCaptureHealth::Unhealthy,
        },
        CaptureAttemptResult::CaptureFailed(reason) => MinimapCaptureStatusSnapshot {
            enabled: config.enabled,
            capture_interval_ms: config.capture_interval_ms,
            last_success_at: None,
            last_failure_at: Some(reason.clone()),
            consecutive_failures: previous_failures + 1,
            last_capture_duration_ms: None,
            last_artifact_path,
            sampling_mode: format!("every-{}", config.sample_every_n),
            window_binding_status: reason,
            health: MinimapCaptureHealth::Unhealthy,
        },
    }
}

pub fn start_minimap_capture_worker(
    _settings: Arc<Mutex<Settings>>,
    _app_state: Arc<Mutex<AppState>>,
) {
}
