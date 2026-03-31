use dota2_scripts::config::Settings;
use dota2_scripts::observability::minimap_artifacts::{
    artifact_metadata_path, build_artifact_metadata, should_persist_sample, MinimapArtifactMetadata,
};
use dota2_scripts::config::MinimapCaptureConfig;
use dota2_scripts::observability::minimap_capture_backend::{
    CaptureBackendResult, capture_window_region, find_dota2_window_rect,
};
use dota2_scripts::observability::minimap_capture::{
    process_capture_attempt, map_backend_result_to_attempt, CaptureAttemptResult,
};
use dota2_scripts::observability::minimap_capture_backend::WindowRect;
use dota2_scripts::observability::minimap_capture_state::{
    MinimapCaptureHealth, MinimapCaptureStatusSnapshot,
};
use dota2_scripts::state::AppState;

#[test]
fn minimap_capture_defaults_are_exposed_through_settings() {
    let settings = Settings::default();

    assert!(!settings.minimap_capture.enabled);
    assert_eq!(settings.minimap_capture.minimap_x, 10);
    assert_eq!(settings.minimap_capture.minimap_y, 815);
    assert_eq!(settings.minimap_capture.minimap_width, 260);
    assert_eq!(settings.minimap_capture.minimap_height, 260);
    assert_eq!(settings.minimap_capture.capture_interval_ms, 1000);
    assert_eq!(settings.minimap_capture.sample_every_n, 30);
    assert_eq!(
        settings.minimap_capture.artifact_output_dir,
        "logs/minimap_capture"
    );
}

#[test]
fn sample_policy_keeps_every_nth_success() {
    assert!(!should_persist_sample(1, 30));
    assert!(!should_persist_sample(29, 30));
    assert!(should_persist_sample(30, 30));
}

#[test]
fn status_snapshot_reports_unhealthy_after_consecutive_failures() {
    let snapshot = MinimapCaptureStatusSnapshot {
        enabled: true,
        capture_interval_ms: 1000,
        last_success_at: None,
        last_failure_at: Some("2026-03-31T01:00:00Z".to_string()),
        consecutive_failures: 3,
        last_capture_duration_ms: None,
        last_artifact_path: None,
        sampling_mode: "every-30".to_string(),
        window_binding_status: "window-not-found".to_string(),
        health: MinimapCaptureHealth::Unhealthy,
    };

    assert_eq!(snapshot.health, MinimapCaptureHealth::Unhealthy);
    assert_eq!(snapshot.window_binding_status, "window-not-found");
}

#[test]
fn artifact_metadata_carries_capture_context() {
    let metadata = MinimapArtifactMetadata {
        capture_timestamp: "2026-03-31T01:00:00Z".to_string(),
        window_binding_status: "bound".to_string(),
        minimap_x: 10,
        minimap_y: 20,
        minimap_width: 300,
        minimap_height: 200,
        image_width: 300,
        image_height: 200,
        capture_duration_ms: 17,
        capture_result: "success".to_string(),
        failure_reason: None,
    };

    assert_eq!(metadata.window_binding_status, "bound");
    assert_eq!(metadata.minimap_width, 300);
    assert_eq!(metadata.capture_result, "success");
}

#[test]
fn failure_artifacts_are_always_persisted() {
    let metadata = build_artifact_metadata(
        "2026-03-31T01:00:00Z".to_string(),
        "window-not-found".to_string(),
        10,
        20,
        300,
        200,
        0,
        0,
        5,
        "failure".to_string(),
        Some("window-not-found".to_string()),
    );

    assert_eq!(metadata.capture_result, "failure");
    assert_eq!(metadata.failure_reason.as_deref(), Some("window-not-found"));
    assert!(!should_persist_sample(1, 30));
}

#[test]
fn artifact_metadata_path_uses_json_sidecar_name() {
    let path = artifact_metadata_path("logs/minimap_capture", "capture-001");

    assert_eq!(path, "logs/minimap_capture/capture-001.json");
}

#[test]
fn app_state_defaults_without_minimap_capture_status() {
    let state = AppState::default();
    assert!(state.minimap_capture.is_none());
}

#[test]
fn failed_window_binding_marks_status_unhealthy() {
    let config = MinimapCaptureConfig {
        enabled: true,
        minimap_x: 10,
        minimap_y: 20,
        minimap_width: 300,
        minimap_height: 200,
        capture_interval_ms: 1000,
        sample_every_n: 30,
        artifact_output_dir: "logs/minimap_capture".to_string(),
    };

    let status = process_capture_attempt(&config, CaptureAttemptResult::WindowNotFound, None, 0);

    assert_eq!(status.window_binding_status, "window-not-found");
    assert_eq!(status.consecutive_failures, 1);
    assert_eq!(status.health.as_str(), "unhealthy");
}

#[test]
fn disabled_worker_leaves_minimap_capture_status_idle() {
    let state = AppState::default();
    assert!(state.minimap_capture.is_none());

    let settings = Settings::default();
    assert!(!settings.minimap_capture.enabled);
}

#[test]
fn capture_attempt_success_builds_healthy_status() {
    let config = Settings::default().minimap_capture;
    let status = process_capture_attempt(
        &config,
        CaptureAttemptResult::Success,
        Some("logs/minimap_capture/sample.png".to_string()),
        2,
    );

    assert_eq!(status.window_binding_status, "bound");
    assert_eq!(status.consecutive_failures, 0);
    assert_eq!(status.health, MinimapCaptureHealth::Healthy);
}

#[test]
fn minimap_capture_status_formats_window_binding_label() {
    let snapshot = MinimapCaptureStatusSnapshot {
        enabled: true,
        capture_interval_ms: 1000,
        last_success_at: None,
        last_failure_at: Some("window-not-found".to_string()),
        consecutive_failures: 2,
        last_capture_duration_ms: None,
        last_artifact_path: None,
        sampling_mode: "every-30".to_string(),
        window_binding_status: "window-not-found".to_string(),
        health: MinimapCaptureHealth::Unhealthy,
    };

    assert_eq!(snapshot.window_binding_status, "window-not-found");
}

#[test]
fn find_dota2_window_returns_not_found_when_dota_not_running() {
    let result = find_dota2_window_rect();
    assert!(matches!(result, CaptureBackendResult::WindowNotFound));
}

#[test]
fn capture_rejects_zero_dimension_region() {
    let result = capture_window_region(0, 0, 0, 0);
    assert!(matches!(result, CaptureBackendResult::CaptureError(_)));
}

#[test]
fn save_capture_artifact_creates_png_file() {
    use dota2_scripts::observability::minimap_artifacts::save_capture_artifact;

    let dir = std::env::temp_dir().join("minimap_test_png");
    let _ = std::fs::remove_dir_all(&dir);

    // Create a 2x2 RGBA test image (red, green, blue, yellow pixels)
    let pixels: Vec<u8> = vec![
        255, 0, 0, 255,    // pixel (0,0) red
        0, 255, 0, 255,    // pixel (1,0) green
        0, 0, 255, 255,    // pixel (0,1) blue
        255, 255, 0, 255,  // pixel (1,1) yellow
    ];

    let result = save_capture_artifact(dir.to_str().unwrap(), "test-capture", &pixels, 2, 2);

    assert!(result.is_ok());
    let saved_path = result.unwrap();
    assert!(saved_path.ends_with("test-capture.png"));
    assert!(std::path::Path::new(&saved_path).exists());

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_metadata_json_creates_sidecar_file() {
    use dota2_scripts::observability::minimap_artifacts::{
        build_artifact_metadata, save_metadata_json,
    };

    let dir = std::env::temp_dir().join("minimap_test_json");
    let _ = std::fs::remove_dir_all(&dir);

    let metadata = build_artifact_metadata(
        "1711843200".to_string(),
        "bound".to_string(),
        10, 700, 260, 260,
        260, 260,
        12,
        "success".to_string(),
        None,
    );

    let result = save_metadata_json(dir.to_str().unwrap(), "test-capture", &metadata);
    assert!(result.is_ok());

    let json_path = dir.join("test-capture.json");
    assert!(json_path.exists());

    let content = std::fs::read_to_string(&json_path).unwrap();
    assert!(content.contains("\"capture_result\": \"success\""));
    assert!(content.contains("\"minimap_width\": 260"));

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn worker_maps_backend_window_not_found_to_unhealthy() {
    use dota2_scripts::observability::minimap_capture_backend::CaptureBackendResult;

    let backend_result = CaptureBackendResult::WindowNotFound;
    let attempt = map_backend_result_to_attempt(backend_result);

    assert_eq!(attempt, CaptureAttemptResult::WindowNotFound);
}

#[test]
fn worker_maps_backend_capture_error_to_failed() {
    use dota2_scripts::observability::minimap_capture_backend::CaptureBackendResult;

    let backend_result = CaptureBackendResult::CaptureError("test error".to_string());
    let attempt = map_backend_result_to_attempt(backend_result);

    assert!(matches!(attempt, CaptureAttemptResult::CaptureFailed(_)));
}

#[test]
fn worker_maps_backend_success_to_success_with_frame() {
    use dota2_scripts::observability::minimap_capture_backend::CaptureBackendResult;

    let backend_result = CaptureBackendResult::Success {
        window_rect: WindowRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        },
        pixels: vec![0u8; 300 * 200 * 4],
        width: 300,
        height: 200,
    };

    let attempt = map_backend_result_to_attempt(backend_result);

    match attempt {
        CaptureAttemptResult::SuccessWithFrame(frame) => {
            assert_eq!(frame.width, 300);
            assert_eq!(frame.height, 200);
            assert_eq!(frame.pixels.len(), 300 * 200 * 4);
        }
        other => panic!("expected SuccessWithFrame, got {:?}", other),
    }
}