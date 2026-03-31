use crate::config::{MinimapCaptureConfig, Settings};
use crate::observability::minimap_capture_backend::CaptureBackendResult;
use crate::observability::minimap_capture_state::{
    MinimapCaptureHealth, MinimapCaptureStatusSnapshot,
};
use crate::state::AppState;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedFrame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureAttemptResult {
    WindowNotFound,
    CaptureFailed(String),
    Success,
    SuccessWithFrame(CapturedFrame),
}

pub fn map_backend_result_to_attempt(result: CaptureBackendResult) -> CaptureAttemptResult {
    match result {
        CaptureBackendResult::WindowNotFound => CaptureAttemptResult::WindowNotFound,
        CaptureBackendResult::CaptureError(reason) => CaptureAttemptResult::CaptureFailed(reason),
        CaptureBackendResult::Success {
            pixels,
            width,
            height,
            ..
        } => CaptureAttemptResult::SuccessWithFrame(CapturedFrame {
            pixels,
            width,
            height,
        }),
    }
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
        CaptureAttemptResult::SuccessWithFrame(_) => MinimapCaptureStatusSnapshot {
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
    }
}

pub fn start_minimap_capture_worker(
    settings: Arc<Mutex<Settings>>,
    app_state: Arc<Mutex<AppState>>,
) {
    use crate::observability::minimap_capture_backend::capture_window_region;

    let mut success_count: u64 = 0;

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

        let start = std::time::Instant::now();
        let backend_result = capture_window_region(
            config.minimap_x,
            config.minimap_y,
            config.minimap_width,
            config.minimap_height,
        );
        let capture_duration_ms = start.elapsed().as_millis() as u64;

        let attempt = map_backend_result_to_attempt(backend_result);

        let mut artifact_path: Option<String> = None;

        if let CaptureAttemptResult::SuccessWithFrame(ref frame) = attempt {
            success_count += 1;
            if crate::observability::minimap_artifacts::should_persist_sample(
                success_count,
                config.sample_every_n,
            ) {
                let timestamp = format!(
                    "{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                );
                let file_stem = format!("capture-{}", timestamp);

                if let Ok(saved_path) =
                    crate::observability::minimap_artifacts::save_capture_artifact(
                        &config.artifact_output_dir,
                        &file_stem,
                        &frame.pixels,
                        frame.width,
                        frame.height,
                    )
                {
                    let metadata = crate::observability::minimap_artifacts::build_artifact_metadata(
                        timestamp,
                        "bound".to_string(),
                        config.minimap_x,
                        config.minimap_y,
                        config.minimap_width,
                        config.minimap_height,
                        frame.width,
                        frame.height,
                        capture_duration_ms,
                        "success".to_string(),
                        None,
                    );
                    let _ = crate::observability::minimap_artifacts::save_metadata_json(
                        &config.artifact_output_dir,
                        &file_stem,
                        &metadata,
                    );
                    artifact_path = Some(saved_path);
                }
            }
        }

        let status = process_capture_attempt(&config, attempt, artifact_path, previous_failures);

        let mut final_status = status;
        if final_status.health == MinimapCaptureHealth::Healthy {
            final_status.last_capture_duration_ms = Some(capture_duration_ms);
        }

        app_state.lock().unwrap().minimap_capture = Some(final_status);
        std::thread::sleep(std::time::Duration::from_millis(config.capture_interval_ms));
    }
}
