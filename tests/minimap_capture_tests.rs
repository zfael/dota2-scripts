use dota2_scripts::config::Settings;

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
