use dota2_scripts::observability::rune_alerts::{RuneAlertManager, RuneAlertSettings};

fn default_settings() -> RuneAlertSettings {
    RuneAlertSettings {
        enabled: true,
        alert_lead_seconds: 10,
        interval_seconds: 120,
        audio_enabled: true,
    }
}

#[test]
fn triggers_one_alert_when_entering_the_lead_window_for_a_rune_cycle() {
    let mut manager = RuneAlertManager::new(default_settings());

    assert!(manager.update(109).is_none());

    let alert = manager
        .update(110)
        .expect("expected alert at the start of the lead window");

    assert_eq!(alert.rune_time_seconds, 120);
    assert_eq!(alert.seconds_until_rune, 10);
}

#[test]
fn does_not_repeat_alerts_within_the_same_rune_cycle() {
    let mut manager = RuneAlertManager::new(default_settings());

    assert!(manager.update(110).is_some());
    assert!(manager.update(111).is_none());
    assert!(manager.update(119).is_none());
}

#[test]
fn alerts_again_for_the_next_rune_cycle() {
    let mut manager = RuneAlertManager::new(default_settings());

    assert!(manager.update(110).is_some());
    assert!(manager.update(230).is_some());
}

#[test]
fn snapshot_reports_next_rune_and_last_alert_details() {
    let mut manager = RuneAlertManager::new(default_settings());

    let _ = manager.update(110);
    let snapshot = manager.snapshot(112);

    assert_eq!(snapshot.next_rune_time_seconds, Some(120));
    assert_eq!(snapshot.seconds_until_next_rune, Some(8));
    assert_eq!(snapshot.last_alerted_rune_time_seconds, Some(120));
    assert_eq!(snapshot.last_alert_clock_time_seconds, Some(110));
}

#[test]
fn disabled_settings_suppress_alerts_but_keep_next_rune_visibility() {
    let mut manager = RuneAlertManager::new(RuneAlertSettings {
        enabled: false,
        ..default_settings()
    });

    assert!(manager.update(110).is_none());

    let snapshot = manager.snapshot(110);
    assert_eq!(snapshot.next_rune_time_seconds, Some(120));
    assert_eq!(snapshot.seconds_until_next_rune, Some(10));
    assert_eq!(snapshot.last_alerted_rune_time_seconds, None);
}
