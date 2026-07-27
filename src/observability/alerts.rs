//! Scheduled map-objective alerts.
//!
//! Every tracked event happens on a clock-determined schedule, so "what fires
//! next" is pure arithmetic over `map.clock_time`. This module owns the
//! schedules, the cue assigned to each event, and the fire-once bookkeeping.
//!
//! Audio playback lives in [`crate::audio`]; this module decides *what* should
//! sound and *when*, and stays testable without an audio device.

use crate::audio::motif::{Motif, Timbre, Tone};
use crate::config::{AlertEventConfig, AlertsConfig};
use lazy_static::lazy_static;
use std::sync::Mutex;
use tracing::{info, warn};

/// A map objective the app can announce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertEvent {
    PowerRune,
    WisdomRune,
    WaterRune,
    BountyRune,
    Tormentor,
    NeutralItem,
    Stack,
}

impl AlertEvent {
    pub const ALL: [AlertEvent; 7] = [
        AlertEvent::PowerRune,
        AlertEvent::WisdomRune,
        AlertEvent::WaterRune,
        AlertEvent::BountyRune,
        AlertEvent::Tormentor,
        AlertEvent::NeutralItem,
        AlertEvent::Stack,
    ];

    /// Stable identifier used in config and over IPC.
    pub fn key(self) -> &'static str {
        match self {
            AlertEvent::PowerRune => "power_rune",
            AlertEvent::WisdomRune => "wisdom_rune",
            AlertEvent::WaterRune => "water_rune",
            AlertEvent::BountyRune => "bounty_rune",
            AlertEvent::Tormentor => "tormentor",
            AlertEvent::NeutralItem => "neutral_item",
            AlertEvent::Stack => "stack",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            AlertEvent::PowerRune => "Power Rune",
            AlertEvent::WisdomRune => "Wisdom Rune",
            AlertEvent::WaterRune => "Water Rune",
            AlertEvent::BountyRune => "Bounty Rune",
            AlertEvent::Tormentor => "Tormentor",
            AlertEvent::NeutralItem => "Neutral Item",
            AlertEvent::Stack => "Stack Timing",
        }
    }

    pub fn schedule(self) -> Schedule {
        match self {
            // Every 2 minutes from 6:00.
            AlertEvent::PowerRune => Schedule::Periodic {
                start: 360,
                interval: 120,
            },
            // Every 7 minutes from 7:00.
            AlertEvent::WisdomRune => Schedule::Periodic {
                start: 420,
                interval: 420,
            },
            // Only twice per game.
            AlertEvent::WaterRune => Schedule::Fixed(&[120, 240]),
            // Every 3 minutes from the horn.
            AlertEvent::BountyRune => Schedule::Periodic {
                start: 0,
                interval: 180,
            },
            // First at 20:00, then on a 10 minute respawn.
            AlertEvent::Tormentor => Schedule::Periodic {
                start: 1200,
                interval: 600,
            },
            AlertEvent::NeutralItem => Schedule::Fixed(&[420, 1020, 1620, 2220, 3600]),
            // Pull/stack timing, each minute.
            AlertEvent::Stack => Schedule::Periodic {
                start: 53,
                interval: 60,
            },
        }
    }

    /// The cue for this event.
    ///
    /// Rhythm encodes cadence deliberately: the two-minute power rune gets two
    /// pulses, the seven-minute wisdom rune three, and bounty two fast ticks.
    /// Timbre encodes category — runes bright, economy wooden, objectives brass.
    /// See [`crate::audio::motif`] for why these channels were chosen.
    pub fn cue(self) -> Motif {
        match self {
            // Two ascending bell blips.
            AlertEvent::PowerRune => Motif::new(
                vec![Tone::new(784.0, 90), Tone::new(1046.0, 110)],
                60,
                Timbre::Bell,
            ),
            // Three ascending wooden notes.
            AlertEvent::WisdomRune => Motif::new(
                vec![
                    Tone::new(523.0, 100),
                    Tone::new(659.0, 100),
                    Tone::new(784.0, 120),
                ],
                30,
                Timbre::Wood,
            ),
            // One soft drop.
            AlertEvent::WaterRune => Motif::new(vec![Tone::new(587.0, 200)], 0, Timbre::Sine),
            // Two quick high ticks.
            AlertEvent::BountyRune => Motif::new(
                vec![Tone::new(1318.0, 60), Tone::new(1318.0, 60)],
                60,
                Timbre::Bell,
            ),
            // Descending brass — heavy, and falling because it is a threat.
            AlertEvent::Tormentor => Motif::new(
                vec![Tone::new(330.0, 200), Tone::new(220.0, 260)],
                40,
                Timbre::Brass,
            ),
            // Four-note ascending arpeggio.
            AlertEvent::NeutralItem => Motif::new(
                vec![
                    Tone::new(523.0, 80),
                    Tone::new(659.0, 80),
                    Tone::new(784.0, 80),
                    Tone::new(1046.0, 100),
                ],
                20,
                Timbre::Wood,
            ),
            // A single dry tick; this one fires every minute, so it stays tiny.
            AlertEvent::Stack => Motif::new(vec![Tone::new(880.0, 120)], 0, Timbre::Wood),
        }
    }
}

/// When an event occurs on the game clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schedule {
    /// First at `start`, then every `interval` seconds forever.
    Periodic { start: i32, interval: i32 },
    /// A fixed set of times, ascending.
    Fixed(&'static [i32]),
}

impl Schedule {
    /// The first occurrence strictly after `clock_time_seconds`.
    ///
    /// Strictly after, so standing exactly on an occurrence reports the next one
    /// rather than the current one — matching how a countdown should read.
    pub fn next_occurrence(&self, clock_time_seconds: i32) -> Option<i32> {
        match *self {
            Schedule::Periodic { start, interval } => {
                if interval <= 0 {
                    return None;
                }
                if clock_time_seconds < start {
                    return Some(start);
                }
                let elapsed = clock_time_seconds - start;
                Some(start + (elapsed / interval + 1) * interval)
            }
            Schedule::Fixed(times) => times.iter().copied().find(|&t| t > clock_time_seconds),
        }
    }
}

/// An alert that just became due.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiredAlert {
    pub event: AlertEvent,
    /// Game-clock time of the occurrence being announced.
    pub occurrence_time_seconds: i32,
    pub seconds_until: i32,
}

/// Countdown state for one event, for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertCountdown {
    pub event: AlertEvent,
    pub enabled: bool,
    pub next_occurrence_seconds: Option<i32>,
    pub seconds_until: Option<i32>,
}

/// Tracks which occurrences have already been announced.
#[derive(Debug, Clone, Default)]
pub struct AlertScheduler {
    last_fired: Vec<(AlertEvent, i32)>,
    last_clock: Option<i32>,
}

impl AlertScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    fn last_fired_for(&self, event: AlertEvent) -> Option<i32> {
        self.last_fired
            .iter()
            .find(|(e, _)| *e == event)
            .map(|(_, t)| *t)
    }

    fn record_fired(&mut self, event: AlertEvent, occurrence: i32) {
        match self.last_fired.iter_mut().find(|(e, _)| *e == event) {
            Some(entry) => entry.1 = occurrence,
            None => self.last_fired.push((event, occurrence)),
        }
    }

    /// Forget every fired occurrence. Called when a new game starts.
    pub fn reset(&mut self) {
        self.last_fired.clear();
        self.last_clock = None;
    }

    /// Advance to `clock_time_seconds` and return any alerts now due.
    ///
    /// A clock that jumps backwards means a new game, so state is cleared —
    /// otherwise the previous game's fired occurrences would suppress this
    /// game's alerts.
    pub fn update(
        &mut self,
        clock_time_seconds: i32,
        config: &AlertsConfig,
    ) -> Vec<FiredAlert> {
        if self
            .last_clock
            .is_some_and(|previous| clock_time_seconds < previous)
        {
            self.reset();
        }
        self.last_clock = Some(clock_time_seconds);

        if !config.enabled {
            return Vec::new();
        }

        let mut fired = Vec::new();

        for event in AlertEvent::ALL {
            let event_config = config.for_event(event);
            if !event_config.enabled {
                continue;
            }

            let Some(next) = event.schedule().next_occurrence(clock_time_seconds) else {
                continue;
            };

            let seconds_until = next - clock_time_seconds;
            if seconds_until > event_config.lead_seconds.max(0) {
                continue;
            }
            if self.last_fired_for(event) == Some(next) {
                continue;
            }

            self.record_fired(event, next);
            fired.push(FiredAlert {
                event,
                occurrence_time_seconds: next,
                seconds_until,
            });
        }

        fired
    }

    /// Countdown for every event, for display.
    pub fn countdowns(
        &self,
        clock_time_seconds: i32,
        config: &AlertsConfig,
    ) -> Vec<AlertCountdown> {
        AlertEvent::ALL
            .iter()
            .map(|&event| {
                let next = event.schedule().next_occurrence(clock_time_seconds);
                AlertCountdown {
                    event,
                    enabled: config.enabled && config.for_event(event).enabled,
                    next_occurrence_seconds: next,
                    seconds_until: next.map(|t| t - clock_time_seconds),
                }
            })
            .collect()
    }
}

/// Effective playback volume for an event: per-event volume scaled by master.
pub fn effective_volume(config: &AlertsConfig, event_config: &AlertEventConfig) -> f32 {
    (config.master_volume.clamp(0.0, 1.0) * event_config.volume.clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

/// Sound one event's cue.
///
/// Resolution order is per-event `sound_file`, then the selected voice pack,
/// then the built-in synthesised cue. A file that cannot be opened or decoded
/// falls back to the cue rather than going silent — a mistyped path or an
/// incomplete pack should not quietly disable an alert.
pub fn play_alert(event: AlertEvent, config: &AlertsConfig) -> bool {
    let event_config = config.for_event(event);
    let volume = effective_volume(config, event_config);

    let resolved = crate::audio::voice_pack::resolve_sound_path(
        event.key(),
        &event_config.sound_file,
        &config.voice_pack,
        std::path::Path::new(crate::audio::voice_pack::VOICE_PACK_DIR),
    );

    if let Some(path) = resolved {
        if crate::audio::player::play_file(&path.to_string_lossy(), volume) {
            return true;
        }
    }

    crate::audio::player::play_motif(&event.cue(), volume)
}

lazy_static! {
    static ref ALERT_SCHEDULER: Mutex<AlertScheduler> = Mutex::new(AlertScheduler::new());
    static ref LATEST_COUNTDOWNS: Mutex<Vec<AlertCountdown>> = Mutex::new(Vec::new());
}

/// Advance the global scheduler and sound anything now due.
///
/// Called from the GSI handler on every packet. Returns the countdowns so the UI
/// can render them without recomputing the schedules.
pub fn process_clock_time(
    clock_time_seconds: i32,
    config: &AlertsConfig,
) -> Vec<AlertCountdown> {
    let (fired, countdowns) = {
        let mut scheduler = match ALERT_SCHEDULER.lock() {
            Ok(scheduler) => scheduler,
            Err(e) => {
                warn!("alerts: scheduler lock poisoned: {e}");
                return Vec::new();
            }
        };
        let fired = scheduler.update(clock_time_seconds, config);
        let countdowns = scheduler.countdowns(clock_time_seconds, config);
        (fired, countdowns)
    };

    for alert in &fired {
        info!(
            "alert: {} in {}s",
            alert.event.display_name(),
            alert.seconds_until
        );
        play_alert(alert.event, config);
    }

    if let Ok(mut latest) = LATEST_COUNTDOWNS.lock() {
        *latest = countdowns.clone();
    }

    countdowns
}

/// Most recent countdowns, for readers that are not on the GSI path.
pub fn latest_countdowns() -> Vec<AlertCountdown> {
    LATEST_COUNTDOWNS
        .lock()
        .map(|latest| latest.clone())
        .unwrap_or_default()
}

#[cfg(test)]
pub fn reset_alert_state_for_tests() {
    if let Ok(mut scheduler) = ALERT_SCHEDULER.lock() {
        scheduler.reset();
    }
    if let Ok(mut latest) = LATEST_COUNTDOWNS.lock() {
        latest.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> AlertsConfig {
        AlertsConfig::default()
    }

    #[test]
    fn power_runes_run_every_two_minutes_from_six_minutes() {
        let schedule = AlertEvent::PowerRune.schedule();

        assert_eq!(schedule.next_occurrence(0), Some(360));
        assert_eq!(schedule.next_occurrence(359), Some(360));
        assert_eq!(schedule.next_occurrence(360), Some(480));
        assert_eq!(schedule.next_occurrence(361), Some(480));
        assert_eq!(schedule.next_occurrence(479), Some(480));
        assert_eq!(schedule.next_occurrence(600), Some(720));
    }

    #[test]
    fn wisdom_runes_run_every_seven_minutes_from_seven_minutes() {
        let schedule = AlertEvent::WisdomRune.schedule();

        assert_eq!(schedule.next_occurrence(0), Some(420));
        assert_eq!(schedule.next_occurrence(420), Some(840));
        assert_eq!(schedule.next_occurrence(840), Some(1260));
    }

    #[test]
    fn bounty_runes_start_at_the_horn() {
        let schedule = AlertEvent::BountyRune.schedule();

        assert_eq!(schedule.next_occurrence(-30), Some(0));
        assert_eq!(schedule.next_occurrence(0), Some(180));
        assert_eq!(schedule.next_occurrence(180), Some(360));
    }

    #[test]
    fn tormentor_first_spawns_at_twenty_minutes_then_respawns_every_ten() {
        let schedule = AlertEvent::Tormentor.schedule();

        assert_eq!(schedule.next_occurrence(0), Some(1200));
        assert_eq!(schedule.next_occurrence(1200), Some(1800));
        assert_eq!(schedule.next_occurrence(1800), Some(2400));
    }

    #[test]
    fn stack_timing_fires_just_before_each_minute() {
        let schedule = AlertEvent::Stack.schedule();

        assert_eq!(schedule.next_occurrence(0), Some(53));
        assert_eq!(schedule.next_occurrence(53), Some(113));
        assert_eq!(schedule.next_occurrence(113), Some(173));
    }

    #[test]
    fn fixed_schedules_run_out_rather_than_repeating() {
        let water = AlertEvent::WaterRune.schedule();

        assert_eq!(water.next_occurrence(0), Some(120));
        assert_eq!(water.next_occurrence(120), Some(240));
        // Water runes only spawn twice; after that there is nothing to announce.
        assert_eq!(water.next_occurrence(240), None);
        assert_eq!(water.next_occurrence(9999), None);
    }

    #[test]
    fn neutral_item_tiers_follow_the_fixed_drop_times() {
        let neutrals = AlertEvent::NeutralItem.schedule();

        assert_eq!(neutrals.next_occurrence(0), Some(420));
        assert_eq!(neutrals.next_occurrence(420), Some(1020));
        assert_eq!(neutrals.next_occurrence(2220), Some(3600));
        assert_eq!(neutrals.next_occurrence(3600), None);
    }

    #[test]
    fn a_zero_interval_schedule_never_fires_rather_than_dividing_by_zero() {
        let broken = Schedule::Periodic {
            start: 0,
            interval: 0,
        };
        assert_eq!(broken.next_occurrence(10), None);
    }

    fn fired_events(fired: &[FiredAlert]) -> Vec<AlertEvent> {
        fired.iter().map(|a| a.event).collect()
    }

    #[test]
    fn an_alert_fires_once_inside_its_lead_window() {
        let mut scheduler = AlertScheduler::new();
        let config = config();
        // Power rune at 360 with a 15s default lead.
        assert!(!fired_events(&scheduler.update(344, &config)).contains(&AlertEvent::PowerRune));

        let fired = scheduler.update(345, &config);
        let power = fired
            .iter()
            .find(|a| a.event == AlertEvent::PowerRune)
            .expect("power rune should fire at 345");
        assert_eq!(power.occurrence_time_seconds, 360);
        assert_eq!(power.seconds_until, 15);

        // Still inside the window, but already announced.
        assert!(scheduler.update(350, &config).is_empty());
        assert!(scheduler.update(359, &config).is_empty());
    }

    #[test]
    fn power_and_bounty_runes_coincide_every_six_minutes() {
        // 6:00, 12:00, 18:00 … carry both a power rune (every 2 min from 6:00)
        // and a bounty rune (every 3 min from 0:00), so both cues fire together.
        let mut scheduler = AlertScheduler::new();
        let fired = fired_events(&scheduler.update(345, &config()));

        assert!(fired.contains(&AlertEvent::PowerRune));
        assert!(fired.contains(&AlertEvent::BountyRune));
    }

    #[test]
    fn the_next_occurrence_fires_again() {
        let mut scheduler = AlertScheduler::new();
        let config = config();

        assert!(fired_events(&scheduler.update(345, &config)).contains(&AlertEvent::PowerRune));
        // Past 360, the next power rune is 480; its window opens at 465.
        assert!(!fired_events(&scheduler.update(400, &config)).contains(&AlertEvent::PowerRune));
        assert!(fired_events(&scheduler.update(465, &config)).contains(&AlertEvent::PowerRune));
    }

    #[test]
    fn disabling_an_event_silences_only_that_event() {
        let mut config = config();
        config.power_rune.enabled = false;

        let mut scheduler = AlertScheduler::new();
        let fired = fired_events(&scheduler.update(345, &config));

        assert!(!fired.contains(&AlertEvent::PowerRune));
        // The bounty rune sharing this moment is unaffected.
        assert!(fired.contains(&AlertEvent::BountyRune));
    }

    #[test]
    fn the_master_switch_silences_everything() {
        let mut config = config();
        config.enabled = false;

        let mut scheduler = AlertScheduler::new();
        assert!(scheduler.update(345, &config).is_empty());
    }

    #[test]
    fn a_new_game_clears_already_fired_occurrences() {
        let mut scheduler = AlertScheduler::new();
        let config = config();

        assert!(fired_events(&scheduler.update(345, &config)).contains(&AlertEvent::PowerRune));
        assert!(scheduler.update(350, &config).is_empty());

        // Clock jumps backwards: a new game started.
        scheduler.update(-30, &config);
        assert!(fired_events(&scheduler.update(345, &config)).contains(&AlertEvent::PowerRune));
    }

    #[test]
    fn a_longer_lead_opens_the_window_earlier() {
        let mut config = config();
        config.power_rune.lead_seconds = 30;

        let mut scheduler = AlertScheduler::new();
        assert!(fired_events(&scheduler.update(330, &config)).contains(&AlertEvent::PowerRune));
    }

    #[test]
    fn countdowns_cover_every_event() {
        let scheduler = AlertScheduler::new();
        let countdowns = scheduler.countdowns(0, &config());

        assert_eq!(countdowns.len(), AlertEvent::ALL.len());
        let power = countdowns
            .iter()
            .find(|c| c.event == AlertEvent::PowerRune)
            .unwrap();
        assert_eq!(power.next_occurrence_seconds, Some(360));
        assert_eq!(power.seconds_until, Some(360));
    }

    #[test]
    fn countdowns_report_exhausted_fixed_schedules_as_having_no_next() {
        let scheduler = AlertScheduler::new();
        let countdowns = scheduler.countdowns(3000, &config());

        let water = countdowns
            .iter()
            .find(|c| c.event == AlertEvent::WaterRune)
            .unwrap();
        assert_eq!(water.next_occurrence_seconds, None);
        assert_eq!(water.seconds_until, None);
    }

    #[test]
    fn every_event_has_a_distinct_key_and_a_non_empty_cue() {
        let mut keys: Vec<&str> = AlertEvent::ALL.iter().map(|e| e.key()).collect();
        keys.sort_unstable();
        let unique = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), unique, "event keys must be unique");

        for event in AlertEvent::ALL {
            let cue = event.cue();
            assert!(!cue.tones.is_empty(), "{:?} has no cue", event);
            assert!(
                cue.duration_ms() <= 500,
                "{:?} cue is {}ms; cues stay under 500ms so they do not mask spell audio",
                event,
                cue.duration_ms()
            );
        }
    }

    #[test]
    fn cue_pulse_counts_encode_the_event_cadence() {
        // Rhythm is the primary discriminator, so these counts are load-bearing.
        assert_eq!(AlertEvent::PowerRune.cue().tones.len(), 2);
        assert_eq!(AlertEvent::WisdomRune.cue().tones.len(), 3);
        assert_eq!(AlertEvent::NeutralItem.cue().tones.len(), 4);
        assert_eq!(AlertEvent::Stack.cue().tones.len(), 1);
    }

    #[test]
    fn master_volume_scales_per_event_volume() {
        let mut config = config();
        config.master_volume = 0.5;
        config.power_rune.volume = 0.5;

        assert!((effective_volume(&config, &config.power_rune) - 0.25).abs() < 0.001);
    }

    #[test]
    fn volumes_are_clamped_into_range() {
        let mut config = config();
        config.master_volume = 5.0;
        config.power_rune.volume = 5.0;

        assert!(effective_volume(&config, &config.power_rune) <= 1.0);
    }
}
