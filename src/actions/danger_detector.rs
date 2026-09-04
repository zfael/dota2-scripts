use crate::actions::activity::{push_activity, ActivityCategory};
use crate::config::DangerDetectionConfig;
use crate::models::GsiWebhookEvent;
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, info};

lazy_static! {
    static ref HP_TRACKER: Mutex<HpTracker> = Mutex::new(HpTracker::default());
    static ref SELF_DAMAGE: Mutex<Option<SelfDamage>> = Mutex::new(None);
}

/// How long a self-inflicted HP cost stays claimable before it is written off.
///
/// The sacrifice is instant in game, but it only reaches us on the next GSI event, and
/// those ticks are not evenly spaced. A second and a half comfortably covers a slow tick
/// while keeping the window in which real burst damage could be mistaken for our own
/// short.
const SELF_DAMAGE_WINDOW: Duration = Duration::from_millis(1500);

#[derive(Debug, Default)]
struct HpTracker {
    last_hp: Option<u32>,
    last_hp_percent: Option<u32>,
    last_update: Option<Instant>,
    danger_detected: bool,
    danger_start_time: Option<Instant>,
}

/// HP the hero spent on itself, which must not be read as incoming damage.
#[derive(Debug, Clone, Copy)]
struct SelfDamage {
    /// HP still unaccounted for in an observed drop.
    hp: u32,
    /// What spent it, for the log line only.
    source: &'static str,
    at: Instant,
}

/// Record HP the hero is about to spend on itself, so the next GSI event does not read it
/// as incoming damage.
///
/// Soul Ring is why this exists. Its 170 HP sacrifice trips both danger triggers on its
/// own - `rapid_loss_hp` defaults to 100, and any loss at all counts once HP sits under
/// `hp_threshold_percent` - so paying for mana used to fire Blade Mail, Ghost Scepter and
/// the rest of the defensive kit at nobody. Pressing Soul Ring is in fact mild evidence of
/// the opposite: the automation only fires it above its own health floor.
///
/// Only the sacrifice itself is discounted, never real damage on top of it. A gank that
/// lands in the same window still shows up as whatever it dealt beyond the 170.
pub fn note_self_damage(hp: u32, source: &'static str) {
    if hp == 0 {
        return;
    }

    if let Ok(mut pending) = SELF_DAMAGE.lock() {
        // Two sacrifices inside one window (Soul Ring plus an armlet toggle, say) owe the
        // tracker the sum of both.
        let carried = pending
            .filter(|entry| entry.at.elapsed() < SELF_DAMAGE_WINDOW)
            .map(|entry| entry.hp)
            .unwrap_or(0);

        *pending = Some(SelfDamage {
            hp: carried + hp,
            source,
            at: Instant::now(),
        });
        debug!("💢 Self-inflicted {}HP from {} - the next HP drop is discounted by that much", hp, source);
    }
}

/// Claim up to `observed` HP of the drop just reported against the pending self-inflicted
/// cost, returning how much was ours. Whatever is left stays claimable until the window
/// expires, so a tick that lands between the key press and the sacrifice does not eat the
/// whole allowance.
fn claim_self_damage(observed: u32) -> u32 {
    let Ok(mut pending) = SELF_DAMAGE.lock() else {
        return 0;
    };
    let Some(entry) = *pending else {
        return 0;
    };

    if entry.at.elapsed() >= SELF_DAMAGE_WINDOW {
        *pending = None;
        return 0;
    }

    let claimed = observed.min(entry.hp);
    if claimed == 0 {
        return 0;
    }

    let remaining = entry.hp - claimed;
    *pending = if remaining == 0 {
        None
    } else {
        Some(SelfDamage {
            hp: remaining,
            ..entry
        })
    };

    debug!(
        "💢 Discounted {}HP of self-inflicted cost ({}) from a {}HP drop, {}HP still claimable",
        claimed, entry.source, observed, remaining
    );
    claimed
}

/// Drop any pending self-inflicted cost. Called on death, where the tracker restarts and
/// an unclaimed sacrifice would otherwise leak into the next life.
pub fn reset_self_damage() {
    if let Ok(mut pending) = SELF_DAMAGE.lock() {
        *pending = None;
    }
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
        reset_self_damage();
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
        let observed_delta = last_hp as i32 - current_hp as i32;

        // Subtract anything we spent on ourselves since the last event - a Soul Ring
        // sacrifice is not an enemy. Only the surplus counts as incoming damage.
        let self_damage = if observed_delta > 0 {
            claim_self_damage(observed_delta as u32)
        } else {
            0
        };
        let hp_delta = observed_delta - self_damage as i32;

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
                "⚠️ DANGER DETECTED! HP: {}/{} ({}%), lost {}HP in {}ms{}",
                current_hp,
                max_hp,
                current_hp_percent,
                hp_delta,
                time_delta_ms,
                if self_damage > 0 {
                    format!(" (after discounting {}HP of self-inflicted cost)", self_damage)
                } else {
                    String::new()
                }
            );
            push_activity(
                ActivityCategory::Danger,
                format!("⚠ Danger detected — HP {}%", current_hp_percent),
            );
        } else if !in_danger && tracker.danger_detected {
            // Check if danger should be cleared
            if let Some(danger_start) = tracker.danger_start_time {
                if danger_start.elapsed().as_secs() >= config.clear_delay_seconds {
                    tracker.danger_detected = false;
                    tracker.danger_start_time = None;
                    info!("✓ Danger cleared - HP stabilized at {}HP ({}%)", current_hp, current_hp_percent);
                    push_activity(
                        ActivityCategory::Danger,
                        format!("✓ Danger cleared — HP {}%", current_hp_percent),
                    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::soul_ring::SOUL_RING_HEALTH_COST;

    lazy_static! {
        /// `HP_TRACKER` and `SELF_DAMAGE` are process-wide, so these tests cannot run
        /// alongside each other.
        static ref TEST_LOCK: Mutex<()> = Mutex::new(());
    }

    /// Take the serialising lock, ignoring poisoning from an unrelated failing test.
    fn guard() -> std::sync::MutexGuard<'static, ()> {
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        *HP_TRACKER.lock().unwrap() = HpTracker::default();
        reset_self_damage();
        guard
    }

    fn config() -> DangerDetectionConfig {
        DangerDetectionConfig::default()
    }

    /// A live event at `hp` out of 1000 max.
    fn at_hp(hp: u32) -> GsiWebhookEvent {
        let mut event = GsiWebhookEvent::default();
        event.hero.alive = true;
        event.hero.name = "npc_dota_hero_mirana".to_string();
        event.hero.max_health = 1000;
        event.hero.health = hp;
        event.hero.health_percent = hp / 10;
        event
    }

    /// The reported bug: buying mana with Soul Ring read as a 170HP burst, which tripped
    /// `rapid_loss_hp` (100) and fired Blade Mail, Ghost Scepter and friends at nobody.
    #[test]
    fn soul_ring_sacrifice_alone_is_not_danger() {
        let _lock = guard();
        let config = config();

        // Seed the tracker at full health.
        assert!(!update(&at_hp(1000), &config));

        note_self_damage(SOUL_RING_HEALTH_COST, "Soul Ring");
        assert!(!update(&at_hp(1000 - SOUL_RING_HEALTH_COST), &config));
        assert!(!is_in_danger());
    }

    /// The same sacrifice below `hp_threshold_percent`, where *any* HP loss counts.
    #[test]
    fn soul_ring_sacrifice_under_the_low_hp_threshold_is_not_danger() {
        let _lock = guard();
        let config = config();

        assert!(!update(&at_hp(600), &config)); // 60%, under the 70% threshold

        note_self_damage(SOUL_RING_HEALTH_COST, "Soul Ring");
        assert!(!update(&at_hp(600 - SOUL_RING_HEALTH_COST), &config));
        assert!(!is_in_danger());
    }

    /// Only the sacrifice is discounted. Getting jumped mid-combo must still register.
    #[test]
    fn real_damage_on_top_of_a_sacrifice_still_triggers_danger() {
        let _lock = guard();
        let config = config();

        assert!(!update(&at_hp(1000), &config));

        note_self_damage(SOUL_RING_HEALTH_COST, "Soul Ring");
        // 170 ours + 300 theirs; the 300 is well past rapid_loss_hp.
        assert!(update(&at_hp(1000 - SOUL_RING_HEALTH_COST - 300), &config));
        assert!(is_in_danger());
    }

    /// A tick landing between the key press and the sacrifice must not eat the whole
    /// allowance - the rest of it still has to cover the drop that follows.
    #[test]
    fn an_unrelated_tick_does_not_consume_the_whole_allowance() {
        let _lock = guard();
        let config = config();

        assert!(!update(&at_hp(1000), &config));

        note_self_damage(SOUL_RING_HEALTH_COST, "Soul Ring");
        // Chip damage arrives first, claiming 10 of the 170.
        assert!(!update(&at_hp(990), &config));
        // Then the sacrifice itself: the remaining 160 covers it.
        assert!(!update(&at_hp(990 - 160), &config));
        assert!(!is_in_danger());
    }

    /// The allowance is spent once. A second burst of the same size is real damage.
    #[test]
    fn the_allowance_is_not_reusable() {
        let _lock = guard();
        let config = config();

        assert!(!update(&at_hp(1000), &config));

        note_self_damage(SOUL_RING_HEALTH_COST, "Soul Ring");
        assert!(!update(&at_hp(830), &config));
        assert!(update(&at_hp(660), &config));
    }

    /// Overlapping sacrifices owe the tracker the sum of both.
    #[test]
    fn concurrent_sacrifices_accumulate() {
        let _lock = guard();
        let config = config();

        assert!(!update(&at_hp(1000), &config));

        note_self_damage(SOUL_RING_HEALTH_COST, "Soul Ring");
        note_self_damage(50, "test");
        assert!(!update(&at_hp(1000 - SOUL_RING_HEALTH_COST - 50), &config));
        assert!(!is_in_danger());
    }

    /// Nothing pending means the old behaviour, unchanged.
    #[test]
    fn plain_burst_damage_still_triggers_danger() {
        let _lock = guard();
        let config = config();

        assert!(!update(&at_hp(1000), &config));
        assert!(update(&at_hp(800), &config));
        assert!(is_in_danger());
    }

    /// Dying drops any unclaimed allowance rather than carrying it into the next life.
    #[test]
    fn death_clears_the_pending_allowance() {
        let _lock = guard();
        let config = config();

        assert!(!update(&at_hp(1000), &config));
        note_self_damage(SOUL_RING_HEALTH_COST, "Soul Ring");

        let mut dead = at_hp(0);
        dead.hero.alive = false;
        assert!(!update(&dead, &config));

        // Fresh life, fresh tracker: the next 200HP burst is real.
        assert!(!update(&at_hp(1000), &config));
        assert!(update(&at_hp(800), &config));
    }

    /// HP regen between events must not consume the allowance.
    #[test]
    fn healing_does_not_consume_the_allowance() {
        let _lock = guard();
        let config = config();

        assert!(!update(&at_hp(800), &config));
        note_self_damage(SOUL_RING_HEALTH_COST, "Soul Ring");
        assert!(!update(&at_hp(900), &config)); // regen, no claim
        assert!(!update(&at_hp(900 - SOUL_RING_HEALTH_COST), &config));
        assert!(!is_in_danger());
    }
}
