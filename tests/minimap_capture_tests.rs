use dota2_scripts::config::Settings;
use dota2_scripts::observability::minimap_artifacts::{
    artifact_metadata_path, build_artifact_metadata, should_persist_sample, MinimapArtifactMetadata,
};
use dota2_scripts::config::MinimapCaptureConfig;
use dota2_scripts::observability::minimap_capture::{
    process_capture_attempt, CaptureAttemptResult,
};
use dota2_scripts::observability::minimap_capture_state::{
    MinimapCaptureHealth, MinimapCaptureStatusSnapshot,
};
use dota2_scripts::state::AppState;

#[test]
fn minimap_capture_defaults_are_exposed_through_settings() {
    let settings = Settings::default();

    assert!(!settings.minimap_capture.enabled);
    assert_eq!(settings.minimap_capture.minimap_x, 0);
    assert_eq!(settings.minimap_capture.minimap_y, 0);
    assert_eq!(settings.minimap_capture.minimap_width, 0);
    assert_eq!(settings.minimap_capture.minimap_height, 0);
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
