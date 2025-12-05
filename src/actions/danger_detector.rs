use crate::config::DangerDetectionConfig;
use crate::models::GsiWebhookEvent;
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::time::Instant;
use tracing::{debug, info};

lazy_static! {
    static ref HP_TRACKER: Mutex<HpTracker> = Mutex::new(HpTracker::default());
}

#[derive(Debug, Default)]
struct HpTracker {
    last_hp: Option<u32>,
    last_hp_percent: Option<u32>,
    last_update: Option<Instant>,
    danger_detected: bool,
    danger_start_time: Option<Instant>,
}

/// Update danger detection state based on current GSI event
/// Returns true if hero is currently in danger
pub fn update(event: &GsiWebhookEvent, config: &DangerDetectionConfig) -> bool {
    if !config.enabled {
        return false;
    }

    if !event.hero.is_alive() {
        // Reset tracker when dead
        if let Ok(mut tracker) = HP_TRACKER.try_lock() {
            *tracker = HpTracker::default();
        }
        return false;
    }

    if let Ok(mut tracker) = HP_TRACKER.try_lock() {
        let current_hp = event.hero.health;
        let current_hp_percent = event.hero.health_percent;
        let max_hp = event.hero.max_health;
        let now = Instant::now();

        // First event - initialize
        if tracker.last_hp.is_none() {
            tracker.last_hp = Some(current_hp);
            tracker.last_hp_percent = Some(current_hp_percent);
            tracker.last_update = Some(now);
            return false;
        }

        let last_hp = tracker.last_hp.unwrap();
        let time_delta_ms = tracker.last_update.unwrap().elapsed().as_millis();

        // Calculate HP change (positive = HP loss)
        let hp_delta = last_hp as i32 - current_hp as i32;

        // Detection logic
        let is_rapid_loss = hp_delta > config.rapid_loss_hp as i32
            && time_delta_ms < config.time_window_ms as u128;
        let is_low_hp = current_hp_percent < config.hp_threshold_percent && hp_delta > 0;

        let in_danger = is_rapid_loss || is_low_hp;

        // State transitions
        if in_danger && !tracker.danger_detected {
            // Danger detected
            tracker.danger_detected = true;
            tracker.danger_start_time = Some(now);
            info!(
                "⚠️ DANGER DETECTED! HP: {}/{} ({}%), lost {}HP in {}ms",
                current_hp, max_hp, current_hp_percent, hp_delta, time_delta_ms
            );
        } else if !in_danger && tracker.danger_detected {
            // Check if danger should be cleared
            if let Some(danger_start) = tracker.danger_start_time {
                if danger_start.elapsed().as_secs() >= config.clear_delay_seconds {
                    tracker.danger_detected = false;
                    tracker.danger_start_time = None;
                    info!("✓ Danger cleared - HP stabilized at {}HP ({}%)", current_hp, current_hp_percent);
                }
            }
        }

        // Update tracker
        tracker.last_hp = Some(current_hp);
        tracker.last_hp_percent = Some(current_hp_percent);
        tracker.last_update = Some(now);

        return tracker.danger_detected;
    }

    false
}

/// Check if hero is currently in danger state
pub fn is_in_danger() -> bool {
    if let Ok(tracker) = HP_TRACKER.try_lock() {
        return tracker.danger_detected;
    }
    false
}
