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
    settings: Arc<Mutex<Settings>>,
    app_state: Arc<Mutex<AppState>>,
) {
    loop {
        let config = {
            let guard = settings.lock().unwrap();
            guard.minimap_capture.clone()
        };

        if !config.enabled {
            std::thread::sleep(std::time::Duration::from_millis(500));
            continue;
        }

        let previous_failures = app_state
            .lock()
            .unwrap()
            .minimap_capture
            .as_ref()
            .map(|snapshot| snapshot.consecutive_failures)
            .unwrap_or(0);

        let status = process_capture_attempt(
            &config,
            CaptureAttemptResult::WindowNotFound,
            None,
            previous_failures,
        );

        app_state.lock().unwrap().minimap_capture = Some(status);
        std::thread::sleep(std::time::Duration::from_millis(config.capture_interval_ms));
    }
}
