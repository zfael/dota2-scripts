//! Morphling — Attribute Shift on danger.
//!
//! Shifts toward strength when the danger detector fires, holds it while the
//! fight lasts, and hands it back once things are quiet. All of it is driven by
//! one number: `hero.max_health`.
//!
//! ## Why `max_health` and nothing else
//!
//! GSI has no field for "a shift is running". `abilities.abilityN.ability_active`
//! looks like the answer and is not: a live capture
//! (`logs/morphling_shift/probe_2026-09-05_19-34-54.jsonl`, replayed with
//! `examples/morphling_shift_probe.rs`) reports it `true` for every ability on
//! every tick, `plus_guild_banner` and `plus_high_five` included.
//!
//! What does work is the side effect. Strength gain raises `max_health`, agility
//! gain lowers it, and the probe measured that signal cleanly across 12 shift
//! episodes:
//!
//! | Measured | Value |
//! |---|---|
//! | GSI tick interval | 226ms median (211–313) |
//! | Shift rate | ~345 HP/s ≈ 15.6 attribute points/s |
//! | HP per attribute point | 22 (every step was a multiple of it) |
//! | Key press → first visible movement | 95–391ms |
//! | Same key again | stops the shift (~180ms) |
//! | Opposite key | reverses it, no stop press needed |
//! | Exhausted pool | `max_health` simply plateaus |
//!
//! **None of those numbers appear below.** They describe the game this logic was
//! tested against, not the rules it follows. Everything here is expressed in
//! terms of the last delta actually observed, so a patch that changes the rate,
//! the HP per point, or the tick cadence changes nothing in this file. The one
//! number that is configured — `target_hp_gain` — is in HP, because HP is what
//! GSI reports.
//!
//! ## The rules
//!
//! 1. **A new fight buys a slice of strength.** The target is fixed when the
//!    fight starts. A fight that keeps going does not keep buying — otherwise a
//!    long teamfight ratchets into a full shift nobody asked for. A *new* fight,
//!    even one that starts before the shift back was due, buys another slice.
//! 2. **Stop early, because the stop lands late.** A press takes about a tick to
//!    take effect, so the stop is issued a tick before the target. The lookahead
//!    is asymmetric — one tick going up, two coming back — so both errors land
//!    on the side of more strength.
//! 3. **A plateau is exhaustion.** Stop deciding; do not press at a ceiling.
//! 4. **Shifting back is opt-in and gated**, on a quiet period *and* a health
//!    floor, and is abandoned the moment danger returns.
//! 5. **The player wins.** `max_health` moving while nothing of ours is running
//!    means they are on the toggles, so the automation stands down.
//!
//! The decision loop was proved against a simulator wired with the measured
//! constants before any of it ran in a game: `examples/morphling_shift_control.rs`,
//! 16 checks over both models of how current HP responds to losing max HP.
//!
//! ## The other half: not reading our own shift as a gank
//!
//! Handing strength back costs current HP, and `danger_detector` reads HP
//! disappearing as incoming damage — the same bug Soul Ring had. So every
//! `max_health` drop is declared with [`crate::actions::danger_detector::note_self_damage`]
//! *before* the detector sees the event. This runs whether or not the automation
//! is enabled: a shift the player does by hand causes exactly the same false
//! reading.

use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::HeroScript;
use crate::config::{MorphlingConfig, Settings};
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info};

lazy_static! {
    static ref CONTROLLER: Mutex<ShiftController> = Mutex::new(ShiftController::new());
    static ref LAST_VITALS: Mutex<Option<Vitals>> = Mutex::new(None);
}

/// The health pair a shift moves, kept between events so a drop can be
/// attributed before the danger detector reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vitals {
    pub max_health: u32,
    pub health: u32,
}

impl Vitals {
    fn from_event(event: &GsiWebhookEvent) -> Self {
        Self {
            max_health: event.hero.max_health,
            health: event.hero.health,
        }
    }
}

/// HP lost purely because `max_health` fell — ours to own, not an enemy's.
///
/// Capped at the health actually lost: at partial HP the game clamps rather than
/// subtracting, and claiming more than was lost would discount real damage that
/// lands in the same window.
pub fn shift_self_damage(previous: Vitals, current: Vitals) -> u32 {
    let max_drop = previous.max_health.saturating_sub(current.max_health);
    if max_drop == 0 {
        return 0;
    }
    previous.health.saturating_sub(current.health).min(max_drop)
}

/// Declare what our own shift is about to cost. Must run before
/// `danger_detector::update` for the same event.
fn declare_shift_self_damage(event: &GsiWebhookEvent) {
    let current = Vitals::from_event(event);
    let mut last = match LAST_VITALS.lock() {
        Ok(last) => last,
        Err(_) => return,
    };

    if !event.hero.is_alive() {
        *last = None;
        return;
    }

    if let Some(previous) = *last {
        let owed = shift_self_damage(previous, current);
        if owed > 0 {
            crate::actions::danger_detector::note_self_damage(owed, "Attribute Shift");
        }
    }
    *last = Some(current);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Press {
    Strength,
    Agility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    Idle,
    Press(Press, &'static str),
    Note(&'static str),
}

#[derive(Debug, Clone, Copy)]
struct Observation {
    at: Instant,
    alive: bool,
    max_health: i64,
    health_percent: i64,
    in_danger: bool,
}

impl Observation {
    fn from_event(event: &GsiWebhookEvent, in_danger: bool) -> Self {
        Self {
            at: Instant::now(),
            alive: event.hero.is_alive(),
            max_health: event.hero.max_health as i64,
            health_percent: event.hero.health_percent as i64,
            in_danger,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Idle,
    Gaining {
        target_max_health: i64,
        started: Instant,
    },
    Holding {
        danger_clear_since: Option<Instant>,
    },
    Returning {
        target_max_health: i64,
        started: Instant,
    },
}

struct ShiftController {
    phase: Phase,
    /// `max_health` before the first shift of this sequence — what "back to
    /// normal" means, however many fights it takes to get there.
    baseline_max_health: Option<i64>,
    /// What this fight is aiming at, fixed when it starts so a second push aims
    /// at the same number instead of stacking a new target on the gains.
    episode_target: Option<i64>,
    last: Option<Observation>,
    /// The last non-zero `max_health` step. The only thing standing in for a
    /// shift rate, and it is measured, not assumed.
    last_step: i64,
    last_move: Option<Instant>,
    last_press: Option<Instant>,
    /// Set when a shift ran out of pool, cleared when `max_health` moves the
    /// other way.
    exhausted: bool,
    /// Consecutive ticks where `max_health` moved with nothing of ours running.
    unexplained_moves: u32,
    danger_was: bool,
    /// A fight that started since we last got to decide. Latched, because the
    /// edge can land inside the settle window.
    danger_edge_pending: bool,
    manual_until: Option<Instant>,
}

impl ShiftController {
    fn new() -> Self {
        Self {
            phase: Phase::Idle,
            baseline_max_health: None,
            episode_target: None,
            last: None,
            last_step: 0,
            last_move: None,
            last_press: None,
            exhausted: false,
            unexplained_moves: 0,
            danger_was: false,
            danger_edge_pending: false,
            manual_until: None,
        }
    }

    fn reset(&mut self) {
        self.phase = Phase::Idle;
        self.baseline_max_health = None;
        self.episode_target = None;
        self.last_press = None;
        self.exhausted = false;
        self.danger_edge_pending = false;
        self.unexplained_moves = 0;
    }

    fn stand_down(&mut self, at: Instant, config: &MorphlingConfig) {
        self.manual_until = Some(at + Duration::from_secs(config.manual_override_seconds));
        self.phase = Phase::Idle;
        self.baseline_max_health = None;
        self.episode_target = None;
        self.last_press = None;
    }

    fn press(&mut self, at: Instant, press: Press, why: &'static str) -> Decision {
        self.last_press = Some(at);
        Decision::Press(press, why)
    }

    /// How far `max_health` travels between deciding to stop and the shift
    /// actually stopping: one tick to see the state being decided on, one for
    /// the press to land. Both terms are the step the game last actually moved.
    fn stopping_distance(&self) -> i64 {
        self.last_step.abs() * 2
    }

    /// Measured from the later of "last moved" and "this phase started".
    ///
    /// `max_health` sits still for most of a game, so timing the stillness from
    /// the last movement alone would declare the pool empty on the first tick
    /// after any press — the last move having been minutes ago — and latch
    /// `exhausted` before the shift had a chance to start. From the press, the
    /// window is instead "did anything happen after we asked", which the
    /// measured 95–391ms press latency clears comfortably.
    fn plateaued(&self, obs: &Observation, started: Instant, config: &MorphlingConfig) -> bool {
        let since = match self.last_move {
            Some(at) if at > started => at,
            _ => started,
        };
        obs.at.saturating_duration_since(since) >= Duration::from_millis(config.plateau_ms)
            && obs.at.saturating_duration_since(started) >= Duration::from_millis(config.press_settle_ms)
    }

    /// Open a fresh strength push, one target's worth above wherever we are now.
    /// The baseline is left alone.
    fn start_gaining(
        &mut self,
        obs: Observation,
        config: &MorphlingConfig,
        why: &'static str,
    ) -> Decision {
        self.baseline_max_health.get_or_insert(obs.max_health);
        let target = obs.max_health + config.target_hp_gain as i64;
        self.episode_target = Some(target);
        self.phase = Phase::Gaining {
            target_max_health: target,
            started: obs.at,
        };
        self.press(obs.at, Press::Strength, why)
    }

    fn observe(&mut self, obs: Observation, config: &MorphlingConfig) -> Decision {
        if !config.enabled {
            self.last = Some(obs);
            return Decision::Idle;
        }

        if !obs.alive {
            let was_busy = self.phase != Phase::Idle;
            self.reset();
            self.last = Some(obs);
            return if was_busy {
                Decision::Note("died — dropping the shift plan")
            } else {
                Decision::Idle
            };
        }

        // The first observation of a session has nothing to compare against, so
        // it is not movement — counting it as such would let one real move
        // afterwards look like the two-in-a-row that means the player is
        // shifting by hand.
        let mut moved = false;
        match self.last {
            Some(previous) => {
                let step = obs.max_health - previous.max_health;
                if step != 0 {
                    self.last_step = step;
                    self.last_move = Some(obs.at);
                    moved = true;
                    if step < 0 {
                        self.exhausted = false;
                    }
                }
            }
            None => self.last_move = Some(obs.at),
        }
        self.last = Some(obs);

        // Rule 1: a fight that starts is not a fight that is still going. Latch
        // it before any guard below can return early and lose it.
        if obs.in_danger && !self.danger_was {
            self.danger_edge_pending = true;
        }
        self.danger_was = obs.in_danger;

        // Rule 5: while the player is driving, stay out of it — and keep staying
        // out for as long as the shift keeps moving, rather than cutting back in
        // mid-shift when the window happens to expire.
        if self.manual_until.is_some_and(|until| obs.at < until) {
            self.danger_edge_pending = false;
            if moved {
                self.manual_until =
                    Some(obs.at + Duration::from_secs(config.manual_override_seconds));
            }
            return Decision::Idle;
        }

        let settle = Duration::from_millis(config.press_settle_ms);

        // Rule 5 again, inferred rather than hooked: movement with nothing of
        // ours running is the player on the toggles. Counted only once our own
        // press has had time to land, and only from the second tick — an item or
        // a level moves `max_health` once and stops.
        let believes_shifting = matches!(self.phase, Phase::Gaining { .. } | Phase::Returning { .. });
        let settled = self
            .last_press
            .is_none_or(|at| obs.at.saturating_duration_since(at) >= settle);
        if moved && !believes_shifting && settled {
            self.unexplained_moves += 1;
            if self.unexplained_moves >= 2 {
                self.stand_down(obs.at, config);
                self.unexplained_moves = 0;
                return Decision::Note("shift moving on its own — the player has the toggles");
            }
        } else if !moved {
            self.unexplained_moves = 0;
        }

        // Rule 2: never press through a press that has not landed yet. A second
        // press of the same key stops the shift, so an impatient controller
        // toggles itself off in the middle of a fight.
        if self
            .last_press
            .is_some_and(|at| obs.at.saturating_duration_since(at) < settle)
        {
            return Decision::Idle;
        }

        if self.danger_edge_pending {
            self.danger_edge_pending = false;
            if self.exhausted {
                return Decision::Note("new danger, but already at full strength");
            }
            let why = if matches!(self.phase, Phase::Returning { .. }) {
                "danger returned — reversing to strength"
            } else {
                "danger — shifting to strength"
            };
            return self.start_gaining(obs, config, why);
        }

        match self.phase {
            Phase::Idle => self.decide_idle(obs, config),
            Phase::Gaining {
                target_max_health,
                started,
            } => self.decide_gaining(obs, config, target_max_health, started),
            Phase::Holding { danger_clear_since } => {
                self.decide_holding(obs, config, danger_clear_since)
            }
            Phase::Returning {
                target_max_health,
                started,
            } => self.decide_returning(obs, config, target_max_health, started),
        }
    }

    fn decide_idle(&mut self, obs: Observation, config: &MorphlingConfig) -> Decision {
        if !obs.in_danger {
            return Decision::Idle;
        }
        if self.exhausted {
            return Decision::Note("in danger, but already at full strength");
        }
        self.start_gaining(obs, config, "danger — shifting to strength")
    }

    fn decide_gaining(
        &mut self,
        obs: Observation,
        config: &MorphlingConfig,
        target_max_health: i64,
        started: Instant,
    ) -> Decision {
        // Rule 2: one tick of lookahead here against two on the way back, so
        // both errors land on the side of more strength. Overshooting into
        // strength costs a little damage; undershooting costs the HP the shift
        // was for.
        let projected = obs.max_health + self.last_step.max(0);
        if obs.max_health >= target_max_health || projected >= target_max_health {
            self.phase = Phase::Holding {
                danger_clear_since: (!obs.in_danger).then_some(obs.at),
            };
            return self.press(obs.at, Press::Strength, "target reached — stopping");
        }

        // Rule 3.
        if self.plateaued(&obs, started, config) {
            self.exhausted = true;
            self.phase = Phase::Holding {
                danger_clear_since: (!obs.in_danger).then_some(obs.at),
            };
            return Decision::Note("max_health stopped moving — strength pool is empty");
        }

        if obs.at.saturating_duration_since(started) >= Duration::from_secs(config.max_shift_seconds)
        {
            self.phase = Phase::Holding {
                danger_clear_since: (!obs.in_danger).then_some(obs.at),
            };
            return self.press(obs.at, Press::Strength, "shift cap reached — stopping");
        }

        Decision::Idle
    }

    fn decide_holding(
        &mut self,
        obs: Observation,
        config: &MorphlingConfig,
        danger_clear_since: Option<Instant>,
    ) -> Decision {
        if obs.in_danger {
            self.phase = Phase::Holding {
                danger_clear_since: None,
            };

            // Rule 1: still the same fight. Top up only if genuinely short of
            // the target it set — a stop that landed early, say. "Short" has to
            // mean more than a tick's worth, or a long fight becomes a press
            // every settle window.
            let target = self.episode_target.unwrap_or(obs.max_health);
            let slack = self
                .stopping_distance()
                .max(config.target_hp_gain as i64 / 4);
            if !self.exhausted && obs.max_health + slack < target {
                self.phase = Phase::Gaining {
                    target_max_health: target,
                    started: obs.at,
                };
                return self.press(
                    obs.at,
                    Press::Strength,
                    "danger, and short of the target — topping up",
                );
            }
            return Decision::Idle;
        }

        let clear_since = match danger_clear_since {
            Some(since) => since,
            None => {
                self.phase = Phase::Holding {
                    danger_clear_since: Some(obs.at),
                };
                return Decision::Idle;
            }
        };

        if !config.return_to_agility {
            return Decision::Idle;
        }

        // Rule 4: the fight has to have been over for a while — not taking
        // damage for a moment is not the same as it being over — and we have to
        // be healthy enough to give the HP back.
        if obs.at.saturating_duration_since(clear_since)
            < Duration::from_secs(config.return_delay_seconds)
        {
            return Decision::Idle;
        }
        if obs.health_percent < config.return_min_health_percent as i64 {
            return Decision::Idle;
        }

        let Some(baseline) = self.baseline_max_health else {
            self.phase = Phase::Idle;
            return Decision::Idle;
        };
        if obs.max_health <= baseline + self.stopping_distance() {
            self.phase = Phase::Idle;
            self.baseline_max_health = None;
            self.episode_target = None;
            return Decision::Idle;
        }

        self.phase = Phase::Returning {
            target_max_health: baseline,
            started: obs.at,
        };
        self.press(obs.at, Press::Agility, "quiet — shifting back to agility")
    }

    fn decide_returning(
        &mut self,
        obs: Observation,
        config: &MorphlingConfig,
        target_max_health: i64,
        started: Instant,
    ) -> Decision {
        // Rule 4: the opposite key reverses a running shift on its own, so
        // danger costs one press, not two.
        if obs.in_danger {
            return self.start_gaining(obs, config, "danger returned — reversing to strength");
        }

        // Giving strength back costs current HP. Below the floor, keep it.
        if obs.health_percent < config.return_min_health_percent as i64 {
            self.phase = Phase::Holding {
                danger_clear_since: Some(obs.at),
            };
            return self.press(obs.at, Press::Agility, "health dropped — keeping the strength");
        }

        // Stopping short leaves a sliver of strength unspent. Stopping late
        // spends agility the player never had, putting the build past where they
        // set it — so the error belongs on the near side of the baseline.
        let projected = obs.max_health - self.stopping_distance();
        if obs.max_health <= target_max_health || projected <= target_max_health {
            self.phase = Phase::Idle;
            self.baseline_max_health = None;
            self.episode_target = None;
            return self.press(obs.at, Press::Agility, "back to baseline — stopping");
        }

        if self.plateaued(&obs, started, config) {
            self.phase = Phase::Idle;
            self.baseline_max_health = None;
            self.episode_target = None;
            return Decision::Note("max_health stopped moving — agility pool is empty");
        }

        if obs.at.saturating_duration_since(started) >= Duration::from_secs(config.max_shift_seconds)
        {
            self.phase = Phase::Idle;
            self.baseline_max_health = None;
            self.episode_target = None;
            return self.press(obs.at, Press::Agility, "shift cap reached — stopping");
        }

        Decision::Idle
    }
}

/// Morphling script.
///
/// GSI-driven only: no key interception, no standalone combo. The shift keys are
/// sent as ordinary synthetic presses, so they obey whatever Attribute Shift is
/// bound to in `[heroes.morphling]`.
pub struct MorphlingScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl MorphlingScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }

    fn run_shift_controller(&self, event: &GsiWebhookEvent, in_danger: bool, config: &MorphlingConfig) {
        let decision = match CONTROLLER.lock() {
            Ok(mut controller) => {
                controller.observe(Observation::from_event(event, in_danger), config)
            }
            Err(_) => return,
        };

        match decision {
            Decision::Idle => {}
            Decision::Note(note) => debug!("🌊 Morphling: {}", note),
            Decision::Press(press, why) => {
                let key = match press {
                    Press::Strength => config.strength_key,
                    Press::Agility => config.agility_key,
                };
                info!(
                    "🌊 Morphling {} shift ({}): {} — max_hp {}",
                    match press {
                        Press::Strength => "strength",
                        Press::Agility => "agility",
                    },
                    key,
                    why,
                    event.hero.max_health
                );
                self.executor.enqueue("morphling-shift", move || {
                    crate::input::simulation::press_key(key);
                });
            }
        }
    }
}

impl HeroScript for MorphlingScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // Before anything reads this event: whatever our own shift just cost is
        // not incoming damage. Runs regardless of `enabled`, since a manual
        // shift produces the same false reading.
        declare_shift_self_damage(event);

        let settings = self.settings.lock().unwrap();
        let config = settings.heroes.morphling.clone();
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);

        self.run_shift_controller(event, in_danger, &config);

        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    fn handle_standalone_trigger(&self) {
        // Nothing to trigger by hand: the shift is entirely GSI-driven.
    }

    fn hero_name(&self) -> &'static str {
        Hero::Morphling.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> MorphlingConfig {
        MorphlingConfig::default()
    }

    /// A timeline of observations, spaced like the live feed the probe measured.
    struct Timeline {
        base: Instant,
        now_ms: u64,
        max_health: i64,
        health_percent: i64,
    }

    impl Timeline {
        fn new(max_health: i64) -> Self {
            Self {
                base: Instant::now(),
                now_ms: 0,
                max_health,
                health_percent: 100,
            }
        }

        /// Advance one measured tick, moving `max_health` by `step`.
        fn tick(&mut self, step: i64, in_danger: bool) -> Observation {
            self.now_ms += 226;
            self.max_health += step;
            Observation {
                at: self.base + Duration::from_millis(self.now_ms),
                alive: true,
                max_health: self.max_health,
                health_percent: self.health_percent,
                in_danger,
            }
        }

        /// Advance without a tick's worth of movement, for waiting out delays.
        fn wait(&mut self, ms: u64, in_danger: bool) -> Observation {
            self.now_ms += ms;
            Observation {
                at: self.base + Duration::from_millis(self.now_ms),
                alive: true,
                max_health: self.max_health,
                health_percent: self.health_percent,
                in_danger,
            }
        }
    }

    fn is_press(decision: Decision, press: Press) -> bool {
        matches!(decision, Decision::Press(actual, _) if actual == press)
    }

    #[test]
    fn a_shift_down_is_declared_as_our_own_damage() {
        let previous = Vitals {
            max_health: 2000,
            health: 2000,
        };
        let current = Vitals {
            max_health: 1912,
            health: 1912,
        };
        assert_eq!(shift_self_damage(previous, current), 88);
    }

    /// At partial health the game clamps instead of subtracting, so claiming the
    /// whole max_health drop would discount real damage landing alongside it.
    #[test]
    fn only_the_health_actually_lost_is_claimed() {
        let previous = Vitals {
            max_health: 2000,
            health: 900,
        };
        let current = Vitals {
            max_health: 1912,
            health: 900,
        };
        assert_eq!(shift_self_damage(previous, current), 0);
    }

    #[test]
    fn gaining_max_health_is_never_claimed() {
        let previous = Vitals {
            max_health: 2000,
            health: 1500,
        };
        let current = Vitals {
            max_health: 2088,
            health: 1588,
        };
        assert_eq!(shift_self_damage(previous, current), 0);
    }

    #[test]
    fn danger_starts_a_strength_shift() {
        let mut controller = ShiftController::new();
        let config = config();
        let mut timeline = Timeline::new(2000);

        assert_eq!(
            controller.observe(timeline.tick(0, false), &config),
            Decision::Idle
        );
        assert!(is_press(
            controller.observe(timeline.tick(0, true), &config),
            Press::Strength
        ));
    }

    /// The target is fixed when the fight starts, so one long fight does not
    /// ratchet into a full shift.
    #[test]
    fn a_sustained_fight_does_not_keep_buying_strength() {
        let mut controller = ShiftController::new();
        let config = config();
        let mut timeline = Timeline::new(2000);

        controller.observe(timeline.tick(0, false), &config);
        assert!(is_press(
            controller.observe(timeline.tick(0, true), &config),
            Press::Strength
        ));

        // Past the settle window, climbing toward the 330 target.
        controller.observe(timeline.wait(500, true), &config);
        let mut presses = 0;
        for _ in 0..20 {
            if let Decision::Press(..) = controller.observe(timeline.tick(88, true), &config) {
                presses += 1;
            }
        }
        // Exactly one: the stop once the target is reached.
        assert_eq!(presses, 1, "a sustained fight should stop once, not ratchet");
    }

    /// A fresh burst before the shift back was ever due has to buy another
    /// slice, even though the first one is still held.
    #[test]
    fn a_new_fight_buys_another_slice() {
        let mut controller = ShiftController::new();
        let config = config();
        let mut timeline = Timeline::new(2000);

        controller.observe(timeline.tick(0, false), &config);
        controller.observe(timeline.tick(0, true), &config);
        controller.observe(timeline.wait(500, true), &config);
        for _ in 0..6 {
            controller.observe(timeline.tick(88, true), &config);
        }

        // Danger clears, but nowhere near long enough to start shifting back.
        controller.observe(timeline.wait(600, false), &config);
        controller.observe(timeline.wait(600, false), &config);

        assert!(
            is_press(
                controller.observe(timeline.wait(600, true), &config),
                Press::Strength
            ),
            "a second fight should shift again without waiting for the return"
        );
    }

    #[test]
    fn the_shift_back_waits_for_the_quiet_period() {
        let mut controller = ShiftController::new();
        let config = config();
        let mut timeline = Timeline::new(2000);

        controller.observe(timeline.tick(0, false), &config);
        controller.observe(timeline.tick(0, true), &config);
        controller.observe(timeline.wait(500, true), &config);
        for _ in 0..6 {
            controller.observe(timeline.tick(88, true), &config);
        }

        // Danger gone, but not for long enough yet.
        for _ in 0..3 {
            assert_eq!(
                controller.observe(timeline.wait(1000, false), &config),
                Decision::Idle
            );
        }

        // Past return_delay_seconds it hands the strength back.
        assert!(is_press(
            controller.observe(
                timeline.wait(config.return_delay_seconds * 1000, false),
                &config
            ),
            Press::Agility
        ));
    }

    #[test]
    fn the_shift_back_never_happens_below_the_health_floor() {
        let mut controller = ShiftController::new();
        let config = config();
        let mut timeline = Timeline::new(2000);
        timeline.health_percent = config.return_min_health_percent as i64 - 1;

        controller.observe(timeline.tick(0, false), &config);
        controller.observe(timeline.tick(0, true), &config);
        controller.observe(timeline.wait(500, true), &config);
        for _ in 0..6 {
            controller.observe(timeline.tick(88, true), &config);
        }

        for _ in 0..12 {
            assert_eq!(
                controller.observe(timeline.wait(1000, false), &config),
                Decision::Idle,
                "hurt heroes keep their strength"
            );
        }
    }

    #[test]
    fn return_to_agility_can_be_switched_off() {
        let mut controller = ShiftController::new();
        let config = MorphlingConfig {
            return_to_agility: false,
            ..config()
        };
        let mut timeline = Timeline::new(2000);

        controller.observe(timeline.tick(0, false), &config);
        controller.observe(timeline.tick(0, true), &config);
        controller.observe(timeline.wait(500, true), &config);
        for _ in 0..6 {
            controller.observe(timeline.tick(88, true), &config);
        }

        for _ in 0..12 {
            assert_eq!(
                controller.observe(timeline.wait(1000, false), &config),
                Decision::Idle
            );
        }
    }

    /// Two ticks of movement with nothing of ours running is the player.
    #[test]
    fn a_manual_shift_stands_the_automation_down() {
        let mut controller = ShiftController::new();
        let config = config();
        let mut timeline = Timeline::new(2000);

        controller.observe(timeline.tick(0, false), &config);
        assert_eq!(
            controller.observe(timeline.tick(88, false), &config),
            Decision::Idle,
            "one move alone is an item or a level"
        );
        assert_eq!(
            controller.observe(timeline.tick(88, false), &config),
            Decision::Note("shift moving on its own — the player has the toggles")
        );

        // And danger during the override window is theirs to handle.
        assert_eq!(
            controller.observe(timeline.tick(88, true), &config),
            Decision::Idle
        );
    }

    #[test]
    fn an_empty_pool_is_noticed_instead_of_pressed_at() {
        let mut controller = ShiftController::new();
        let config = config();
        let mut timeline = Timeline::new(2000);

        controller.observe(timeline.tick(0, false), &config);
        controller.observe(timeline.tick(0, true), &config);

        // The press landed, but max_health never moves: the pool is empty.
        controller.observe(timeline.wait(600, true), &config);
        assert_eq!(
            controller.observe(timeline.wait(600, true), &config),
            Decision::Note("max_health stopped moving — strength pool is empty")
        );
        assert_eq!(
            controller.observe(timeline.wait(600, true), &config),
            Decision::Idle,
            "nothing more to press for at a ceiling"
        );
    }

    #[test]
    fn disabled_means_nothing_is_pressed() {
        let mut controller = ShiftController::new();
        let config = MorphlingConfig {
            enabled: false,
            ..config()
        };
        let mut timeline = Timeline::new(2000);

        for _ in 0..10 {
            assert_eq!(
                controller.observe(timeline.tick(0, true), &config),
                Decision::Idle
            );
        }
    }
}
