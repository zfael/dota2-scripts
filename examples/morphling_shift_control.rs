//! Morphling Attribute Shift controller — the decision loop, proved offline.
//!
//! `morphling_shift_probe` established what the game reports. This one
//! establishes what to do about it, before a single key press reaches Dota.
//!
//! Two halves:
//!
//! - `ShiftController` — the state machine. It sees GSI observations and the
//!   danger flag, and returns presses. It is written to be lifted into
//!   `src/actions/heroes/morphling.rs` unchanged.
//! - `World` — a simulator of Morphling's shift, wired with the constants
//!   measured from `logs/morphling_shift/probe_2026-09-05_19-34-54.jsonl`:
//!
//!   | Constant | Measured | Used for |
//!   |---|---|---|
//!   | tick interval | 226ms median (211–313) | how often the controller gets to think |
//!   | shift rate | ~345 HP/s ≈ 15.6 points/s | how fast the world moves |
//!   | HP per point | 22 | quantising the world's steps |
//!   | press → effect | 95–391ms, median ~250 | why a stop press overshoots |
//!   | pool | 4378 HP between full agility and full strength | exhaustion plateaus |
//!
//!   **The controller itself uses none of these.** They describe the world it
//!   is tested against, not the rules it follows — a patch that changes the
//!   rate or the HP per point changes this simulator's numbers and nothing in
//!   the state machine. The only thing it reads is the sign and size of the
//!   `max_health` delta it just saw.
//!
//! ## The rules it enforces
//!
//! 1. Danger means strength. Press once, stop when `max_health` has risen by
//!    the configured HP — projecting one tick ahead, because the stop press
//!    lands a quarter-second late.
//! 2. Never press twice inside the settle window. A second press of the same
//!    key *stops* the shift (measured: F at 8.39s, flat by 8.57s), so an
//!    impatient controller toggles itself off and stands there in a fight.
//! 3. A plateau is exhaustion, not a stuck shift. Stop deciding, do not press.
//! 4. Shifting back to agility is opt-in, gated on a quiet period *and* an HP
//!    floor, and abandoned the instant danger returns — the opposite key
//!    reverses a running shift with no stop press needed (measured at 15.59s).
//! 5. A shift key you pressed yourself hands control back to you.
//!
//! ## Running it
//!
//! ```powershell
//! cargo run --example morphling_shift_control              # scenario suite
//! cargo run --example morphling_shift_control -- --verbose # every tick
//! cargo run --example morphling_shift_control -- --hp-model clamped
//! ```
//!
//! `--hp-model` exists because one thing is still unmeasured: the capture ran
//! at full HP throughout, so it cannot say whether losing max HP to an agility
//! shift drags current HP down with it (`pessimistic`, the default here) or
//! merely clamps it (`clamped`). The controller is tested against the harsher
//! of the two.

use std::collections::VecDeque;

// ─────────────────────────── the controller ───────────────────────────
// Everything in this section is the candidate for src/actions/heroes/morphling.rs.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Strength,
    Agility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Nothing to do this tick.
    Idle,
    /// Press a shift key. Starting, stopping and reversing are all this — the
    /// key does whatever the current toggle state makes it do.
    Press(Press, &'static str),
    /// No press, but something happened worth a log line.
    Note(&'static str),
}

#[derive(Debug, Clone)]
pub struct ShiftConfig {
    pub enabled: bool,
    /// How much `max_health` to gain when danger hits. In HP, not strength
    /// points: HP is what GSI reports, so nothing here depends on what a point
    /// is worth this patch.
    pub target_hp_gain: i64,
    /// Whether to shift back to agility at all, or leave that to the player.
    pub return_to_agility: bool,
    /// Quiet period after danger clears before shifting back. Deliberately
    /// separate from the danger detector's own clear delay: not taking damage
    /// for a moment is not the same as the fight being over.
    pub return_delay_ms: u128,
    /// Never give up strength below this much health.
    pub return_min_health_percent: i64,
    /// Hard stop on any single shift, however the rest of the logic feels.
    pub max_shift_ms: u128,
    /// `max_health` standing still this long means the shift is done — either
    /// stopped or out of pool.
    pub plateau_ms: u128,
    /// How long a press needs to land before its effect is believable. The
    /// probe measured 95–391ms; this is the ceiling with room to spare.
    pub press_settle_ms: u128,
    /// How long a manual shift key takes the automation out of the loop.
    pub manual_override_ms: u128,
}

impl Default for ShiftConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            // ~15 strength at the 22 HP/point the probe measured — about a
            // second of shifting.
            target_hp_gain: 330,
            return_to_agility: true,
            return_delay_ms: 5000,
            return_min_health_percent: 70,
            max_shift_ms: 4000,
            plateau_ms: 700,
            press_settle_ms: 500,
            manual_override_ms: 5000,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Observation {
    pub ts_ms: u128,
    pub alive: bool,
    pub max_health: i64,
    pub health_percent: i64,
    pub in_danger: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Idle,
    /// Shifting toward strength, aiming at `target_max_health`.
    Gaining {
        target_max_health: i64,
        started_ms: u128,
    },
    /// Holding the strength we bought, waiting out the fight.
    Holding {
        danger_clear_since: Option<u128>,
    },
    /// Giving it back, aiming at the `max_health` we started from.
    Returning {
        target_max_health: i64,
        started_ms: u128,
    },
}

pub struct ShiftController {
    config: ShiftConfig,
    phase: Phase,
    /// `max_health` before the first shift of this episode — what "back to
    /// normal" means. Cleared when the player takes over.
    baseline_max_health: Option<i64>,
    last: Option<Observation>,
    /// What `max_health` this danger episode is aiming at, fixed when the
    /// episode opens. Held on the controller rather than in the phase so a
    /// second push later in the same fight aims at the same number instead of
    /// stacking another target on top of what it already gained — which is how
    /// a fixed target quietly turns into a full shift.
    episode_target: Option<i64>,
    /// The most recent non-zero `max_health` step, which is how far the next
    /// tick is expected to move. Measured, never assumed.
    last_step: i64,
    last_move_ms: u128,
    /// When our last press was issued, so we do not press through it.
    last_press_ms: Option<u128>,
    /// Set when a shift ran out of pool; cleared once `max_health` moves the
    /// other way. Stops us pressing at a ceiling that cannot move.
    exhausted: bool,
    /// Consecutive ticks where `max_health` moved while we believed nothing was
    /// shifting. Two in a row is the player working the toggles themselves —
    /// one alone is an item or a level, which move `max_health` once and stop.
    unexplained_moves: u32,
    /// Whether the previous observation was in danger, so a new fight can be
    /// told apart from the same one still going.
    danger_was: bool,
    /// A danger that started since we last got to decide. Latched rather than
    /// read live: the edge can land inside the settle window, and losing it
    /// there would mean sitting out the fight that just started.
    danger_edge_pending: bool,
    manual_until_ms: u128,
}

impl ShiftController {
    pub fn new(config: ShiftConfig) -> Self {
        Self {
            config,
            phase: Phase::Idle,
            baseline_max_health: None,
            episode_target: None,
            last: None,
            last_step: 0,
            last_move_ms: 0,
            last_press_ms: None,
            exhausted: false,
            unexplained_moves: 0,
            danger_was: false,
            danger_edge_pending: false,
            manual_until_ms: 0,
        }
    }

    /// A shift key the player pressed themselves. The automation gets out of
    /// the way rather than fighting them for the toggle.
    pub fn note_manual_press(&mut self, ts_ms: u128) {
        self.manual_until_ms = ts_ms + self.config.manual_override_ms;
        self.phase = Phase::Idle;
        self.baseline_max_health = None;
        self.episode_target = None;
        self.last_press_ms = None;
    }

    fn reset(&mut self) {
        self.phase = Phase::Idle;
        self.baseline_max_health = None;
        self.episode_target = None;
        self.last_press_ms = None;
        self.exhausted = false;
        self.danger_edge_pending = false;
    }

    /// How far `max_health` is expected to travel between deciding to stop and
    /// the shift actually stopping: one tick to see the state we are deciding
    /// on, plus one for the press to land (measured at 95–391ms against a
    /// 226ms tick). Both terms are the step the game last actually moved, so
    /// this tracks the patch rather than predicting it.
    fn stopping_distance(&self) -> i64 {
        self.last_step.abs() * 2
    }

    fn press(&mut self, ts_ms: u128, press: Press, why: &'static str) -> Decision {
        self.last_press_ms = Some(ts_ms);
        Decision::Press(press, why)
    }

    /// Open a fresh strength episode, aimed at one target's worth above wherever
    /// we are now. The baseline is left alone: however many fights it takes,
    /// "back to normal" still means the build the player set.
    fn start_gaining(&mut self, obs: Observation, why: &'static str) -> Decision {
        self.baseline_max_health.get_or_insert(obs.max_health);
        let target = obs.max_health + self.config.target_hp_gain;
        self.episode_target = Some(target);
        self.phase = Phase::Gaining {
            target_max_health: target,
            started_ms: obs.ts_ms,
        };
        self.press(obs.ts_ms, Press::Strength, why)
    }

    /// True while `max_health` has stood still long enough to call the shift
    /// over, measured from the later of "last moved" and "this phase started".
    ///
    /// `max_health` sits still for most of a game, so timing from the last
    /// movement alone declares the pool empty on the first tick after any press
    /// — the last move having been minutes ago. This simulator only ever
    /// escaped that because its worlds start moving on the same tick the settle
    /// window expires; the unit tests in `src/actions/heroes/morphling.rs`
    /// caught it.
    fn plateaued(&self, obs: &Observation, started_ms: u128) -> bool {
        let since = self.last_move_ms.max(started_ms);
        obs.ts_ms.saturating_sub(since) >= self.config.plateau_ms
            && obs.ts_ms.saturating_sub(started_ms) >= self.config.press_settle_ms
    }

    pub fn observe(&mut self, obs: Observation) -> Decision {
        if !self.config.enabled {
            self.last = Some(obs);
            return Decision::Idle;
        }

        // Death ends everything: the shift is gone and so is the baseline.
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

        // Track how far max_health moved, and when it last did. This is the
        // whole of what the controller knows about the shift.
        // The first observation has nothing to compare against, so it is not
        // movement — counting it as such would let one real move afterwards look
        // like the two-in-a-row that means the player is shifting by hand.
        let mut moved = false;
        match self.last {
            Some(previous) => {
                let step = obs.max_health - previous.max_health;
                if step != 0 {
                    self.last_step = step;
                    self.last_move_ms = obs.ts_ms;
                    moved = true;
                    if step < 0 {
                        // Moving down means the pool has room again.
                        self.exhausted = false;
                    }
                }
            }
            None => self.last_move_ms = obs.ts_ms,
        }
        self.last = Some(obs);

        // A fight that starts is not a fight that is still going. Latch the
        // edge here, before any guard can return early and lose it.
        if obs.in_danger && !self.danger_was {
            self.danger_edge_pending = true;
        }
        self.danger_was = obs.in_danger;

        // While the player is driving, stay out of it — and keep staying out of
        // it for as long as the shift keeps moving, rather than cutting back in
        // mid-shift when the window happens to expire.
        if obs.ts_ms < self.manual_until_ms {
            self.danger_edge_pending = false;
            if moved {
                self.manual_until_ms = obs.ts_ms + self.config.manual_override_ms;
            }
            return Decision::Idle;
        }

        // Rule 5, without a keyboard hook: a shift moving while we believe
        // nothing is shifting is the player working the toggles. Only counted
        // once our own last press has had time to land, so the tail of our own
        // stop is never mistaken for them, and only from the second tick, since
        // an item or a level moves max_health once and stops.
        let believes_shifting = matches!(self.phase, Phase::Gaining { .. } | Phase::Returning { .. });
        let settled = self
            .last_press_ms
            .is_none_or(|at| obs.ts_ms.saturating_sub(at) >= self.config.press_settle_ms);
        if moved && !believes_shifting && settled {
            self.unexplained_moves += 1;
            if self.unexplained_moves >= 2 {
                self.note_manual_press(obs.ts_ms);
                self.unexplained_moves = 0;
                return Decision::Note("shift moving on its own — the player has the toggles");
            }
        } else if !moved {
            self.unexplained_moves = 0;
        }

        // Rule 2: never press through a press that has not landed yet.
        if let Some(pressed_at) = self.last_press_ms {
            if obs.ts_ms.saturating_sub(pressed_at) < self.config.press_settle_ms {
                return Decision::Idle;
            }
        }

        // A new fight always buys another slice, whatever we were doing —
        // holding the strength from the last one, or halfway through handing it
        // back. Only a sustained fight is capped at a single target, so that
        // one long teamfight does not ratchet into a full shift.
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
            return self.start_gaining(obs, why);
        }

        match self.phase {
            Phase::Idle => self.decide_idle(obs),
            Phase::Gaining {
                target_max_health,
                started_ms,
            } => self.decide_gaining(obs, target_max_health, started_ms),
            Phase::Holding { danger_clear_since } => self.decide_holding(obs, danger_clear_since),
            Phase::Returning {
                target_max_health,
                started_ms,
            } => self.decide_returning(obs, target_max_health, started_ms),
        }
    }

    fn decide_idle(&mut self, obs: Observation) -> Decision {
        if !obs.in_danger {
            return Decision::Idle;
        }
        if self.exhausted {
            return Decision::Note("in danger, but already at full strength");
        }

        self.start_gaining(obs, "danger — shifting to strength")
    }

    fn decide_gaining(
        &mut self,
        obs: Observation,
        target_max_health: i64,
        started_ms: u128,
    ) -> Decision {
        // Rule 1: the stop press lands late, so stop early by however far the
        // game is currently moving per tick.
        //
        // One tick of lookahead here against two on the way back, on purpose.
        // Both errors are then in the same direction — a little more strength
        // than asked for. Overshooting into strength during a fight costs some
        // damage; undershooting costs the HP the shift was for.
        let projected = obs.max_health + self.last_step.max(0);
        if obs.max_health >= target_max_health || projected >= target_max_health {
            self.phase = Phase::Holding {
                danger_clear_since: (!obs.in_danger).then_some(obs.ts_ms),
            };
            return self.press(obs.ts_ms, Press::Strength, "target reached — stopping");
        }

        // Rule 3: a ceiling is not a stuck shift. Nothing left to press for.
        if self.plateaued(&obs, started_ms) {
            self.exhausted = true;
            self.phase = Phase::Holding {
                danger_clear_since: (!obs.in_danger).then_some(obs.ts_ms),
            };
            return Decision::Note("max_health stopped moving — strength pool is empty");
        }

        if obs.ts_ms.saturating_sub(started_ms) >= self.config.max_shift_ms {
            self.phase = Phase::Holding {
                danger_clear_since: (!obs.in_danger).then_some(obs.ts_ms),
            };
            return self.press(obs.ts_ms, Press::Strength, "shift cap reached — stopping");
        }

        Decision::Idle
    }

    fn decide_holding(&mut self, obs: Observation, danger_clear_since: Option<u128>) -> Decision {
        if obs.in_danger {
            self.phase = Phase::Holding {
                danger_clear_since: None,
            };

            // Still in it. Top up only if we are genuinely short of the target
            // this episode set — a stop press that landed early, or HP handed
            // back before the fight restarted. "Short" has to mean more than a
            // tick's worth, or a sustained fight becomes a press every settle
            // window and ratchets to a full shift the player never asked for.
            let target = self.episode_target.unwrap_or(obs.max_health);
            let slack = self.stopping_distance().max(self.config.target_hp_gain / 4);
            if !self.exhausted && obs.max_health + slack < target {
                self.phase = Phase::Gaining {
                    target_max_health: target,
                    started_ms: obs.ts_ms,
                };
                return self.press(
                    obs.ts_ms,
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
                    danger_clear_since: Some(obs.ts_ms),
                };
                return Decision::Idle;
            }
        };

        if !self.config.return_to_agility {
            return Decision::Idle;
        }

        // Rule 4, first half: the fight has to have been over for a while, and
        // we have to be healthy enough to hand the HP back.
        if obs.ts_ms.saturating_sub(clear_since) < self.config.return_delay_ms {
            return Decision::Idle;
        }
        if obs.health_percent < self.config.return_min_health_percent {
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
            started_ms: obs.ts_ms,
        };
        self.press(obs.ts_ms, Press::Agility, "quiet — shifting back to agility")
    }

    fn decide_returning(
        &mut self,
        obs: Observation,
        target_max_health: i64,
        started_ms: u128,
    ) -> Decision {
        // Rule 4, second half. The opposite key reverses a running shift on its
        // own, so danger costs one press, not two.
        if obs.in_danger {
            let target = obs.max_health + self.config.target_hp_gain;
            self.episode_target = Some(target);
            self.phase = Phase::Gaining {
                target_max_health: target,
                started_ms: obs.ts_ms,
            };
            return self.press(
                obs.ts_ms,
                Press::Strength,
                "danger returned — reversing to strength",
            );
        }

        // Handing back strength costs current HP. Below the floor, stop.
        if obs.health_percent < self.config.return_min_health_percent {
            self.phase = Phase::Holding {
                danger_clear_since: Some(obs.ts_ms),
            };
            return self.press(obs.ts_ms, Press::Agility, "health dropped — keeping the strength");
        }

        // Stopping short leaves a little unspent strength. Stopping late spends
        // agility the player never had, putting the build past where they set
        // it — so the error belongs on the near side of the baseline.
        let projected = obs.max_health - self.stopping_distance();
        if obs.max_health <= target_max_health || projected <= target_max_health {
            self.phase = Phase::Idle;
            self.baseline_max_health = None;
            self.episode_target = None;
            return self.press(obs.ts_ms, Press::Agility, "back to baseline — stopping");
        }

        if self.plateaued(&obs, started_ms) {
            self.phase = Phase::Idle;
            self.baseline_max_health = None;
            self.episode_target = None;
            return Decision::Note("max_health stopped moving — agility pool is empty");
        }

        if obs.ts_ms.saturating_sub(started_ms) >= self.config.max_shift_ms {
            self.phase = Phase::Idle;
            self.baseline_max_health = None;
            self.episode_target = None;
            return self.press(obs.ts_ms, Press::Agility, "shift cap reached — stopping");
        }

        Decision::Idle
    }
}

// ─────────────────────────── the simulated world ───────────────────────────
// Measured from the live capture. None of it is known to the controller.

const TICK_MS: u128 = 226;
const POINTS_PER_SECOND: f64 = 15.6;
const HP_PER_POINT: i64 = 22;
const PRESS_LATENCY_MS: u128 = 250;

#[derive(Debug, Clone, Copy, PartialEq)]
enum HpModel {
    /// Losing max HP drags current HP down with it.
    Pessimistic,
    /// Current HP only falls once max HP passes under it.
    Clamped,
}

struct World {
    ts_ms: u128,
    max_health: i64,
    health: i64,
    floor_max_health: i64,
    ceiling_max_health: i64,
    /// `-1` agility-ward, `0` stopped, `+1` strength-ward.
    shift: i8,
    pending: VecDeque<(u128, Press)>,
    /// Fractional attribute points carried between ticks, so steps come out as
    /// the 66/88 mix the capture shows rather than a flat number.
    carry: f64,
    hp_model: HpModel,
    danger_windows: Vec<(u128, u128)>,
}

impl World {
    fn new(max_health: i64, health: i64, hp_model: HpModel) -> Self {
        Self {
            ts_ms: 0,
            max_health,
            health,
            // The measured pool: 4378 HP between the two ends.
            floor_max_health: max_health - 2000,
            ceiling_max_health: max_health + 2378,
            shift: 0,
            pending: VecDeque::new(),
            carry: 0.0,
            hp_model,
            danger_windows: Vec::new(),
        }
    }

    fn with_pool(mut self, floor: i64, ceiling: i64) -> Self {
        self.floor_max_health = floor;
        self.ceiling_max_health = ceiling;
        self
    }

    fn with_danger(mut self, windows: &[(u128, u128)]) -> Self {
        self.danger_windows = windows.to_vec();
        self
    }

    fn in_danger(&self) -> bool {
        self.danger_windows
            .iter()
            .any(|(from, to)| self.ts_ms >= *from && self.ts_ms < *to)
    }

    fn press(&mut self, press: Press) {
        self.pending.push_back((self.ts_ms + PRESS_LATENCY_MS, press));
    }

    /// The toggle rule the capture established: the same key again stops the
    /// shift, the other key reverses it.
    fn apply(&mut self, press: Press) {
        let wanted = match press {
            Press::Strength => 1i8,
            Press::Agility => -1i8,
        };
        self.shift = if self.shift == wanted { 0 } else { wanted };
    }

    fn tick(&mut self) {
        self.ts_ms += TICK_MS;

        while matches!(self.pending.front(), Some((at, _)) if *at <= self.ts_ms) {
            let (_, press) = self.pending.pop_front().expect("front just matched");
            self.apply(press);
        }

        if self.shift == 0 {
            self.carry = 0.0;
            return;
        }

        self.carry += POINTS_PER_SECOND * (TICK_MS as f64 / 1000.0);
        let points = self.carry.floor() as i64;
        if points == 0 {
            return;
        }
        self.carry -= points as f64;

        let requested = points * HP_PER_POINT * self.shift as i64;
        let next = (self.max_health + requested).clamp(self.floor_max_health, self.ceiling_max_health);
        let applied = next - self.max_health;
        self.max_health = next;

        if applied > 0 {
            // Strength hands over the HP it buys.
            self.health += applied;
        } else if applied < 0 {
            self.health = match self.hp_model {
                HpModel::Pessimistic => (self.health + applied).max(1),
                HpModel::Clamped => self.health.min(self.max_health),
            };
        }
        self.health = self.health.min(self.max_health).max(1);
    }

    fn health_percent(&self) -> i64 {
        if self.max_health == 0 {
            return 0;
        }
        (self.health * 100) / self.max_health
    }

    fn observation(&self) -> Observation {
        Observation {
            ts_ms: self.ts_ms,
            alive: true,
            max_health: self.max_health,
            health_percent: self.health_percent(),
            in_danger: self.in_danger(),
        }
    }
}

// ─────────────────────────── scenarios ───────────────────────────

struct Run {
    presses: Vec<(u128, Press, &'static str)>,
    notes: Vec<(u128, &'static str)>,
    peak_max_health: i64,
    final_max_health: i64,
    start_max_health: i64,
    min_health_percent: i64,
}

impl Run {
    fn presses_within(&self, window_ms: u128) -> Option<(u128, u128)> {
        self.presses
            .windows(2)
            .map(|pair| (pair[0].0, pair[1].0))
            .find(|(first, second)| second.saturating_sub(*first) < window_ms)
    }

    fn count(&self, press: Press) -> usize {
        self.presses.iter().filter(|(_, kind, _)| *kind == press).count()
    }
}

fn run(name: &str, mut world: World, config: ShiftConfig, ticks: usize, verbose: bool) -> Run {
    println!("\n─── {name} ───");
    let mut controller = ShiftController::new(config);
    let mut run = Run {
        presses: Vec::new(),
        notes: Vec::new(),
        peak_max_health: world.max_health,
        final_max_health: world.max_health,
        start_max_health: world.max_health,
        min_health_percent: world.health_percent(),
    };

    let mut danger_was = false;
    for _ in 0..ticks {
        world.tick();
        let observation = world.observation();

        if observation.in_danger != danger_was {
            println!(
                "[{:>6}ms] {}",
                observation.ts_ms,
                if observation.in_danger {
                    "⚠ danger"
                } else {
                    "✓ danger cleared"
                }
            );
            danger_was = observation.in_danger;
        }

        match controller.observe(observation) {
            Decision::Press(press, why) => {
                println!(
                    "[{:>6}ms] press {:?}  max_hp {} ({}%) — {}",
                    observation.ts_ms,
                    press,
                    observation.max_health,
                    observation.health_percent,
                    why
                );
                world.press(press);
                run.presses.push((observation.ts_ms, press, why));
            }
            Decision::Note(note) => {
                println!("[{:>6}ms] {} (max_hp {})", observation.ts_ms, note, observation.max_health);
                run.notes.push((observation.ts_ms, note));
            }
            Decision::Idle => {
                if verbose {
                    println!(
                        "[{:>6}ms] .     max_hp {} ({}%)",
                        observation.ts_ms, observation.max_health, observation.health_percent
                    );
                }
            }
        }

        run.peak_max_health = run.peak_max_health.max(world.max_health);
        run.min_health_percent = run.min_health_percent.min(world.health_percent());
    }

    run.final_max_health = world.max_health;
    println!(
        "     → max_hp {} → peak {} → final {}, presses {}",
        run.start_max_health,
        run.peak_max_health,
        run.final_max_health,
        run.presses.len()
    );
    run
}

fn check(results: &mut Vec<(String, bool)>, label: &str, passed: bool) {
    println!("     {} {}", if passed { "PASS" } else { "FAIL" }, label);
    results.push((label.to_string(), passed));
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verbose = args.iter().any(|arg| arg == "--verbose" || arg == "-v");
    let hp_model = if args.iter().any(|arg| arg == "clamped") {
        HpModel::Clamped
    } else {
        HpModel::Pessimistic
    };
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Usage: cargo run --example morphling_shift_control -- [--verbose] [--hp-model clamped|pessimistic]");
        return;
    }

    println!("Morphling shift controller — simulated against the measured capture");
    println!(
        "tick {}ms, {:.1} points/s, {} HP/point, press lands in {}ms, HP model {:?}",
        TICK_MS, POINTS_PER_SECOND, HP_PER_POINT, PRESS_LATENCY_MS, hp_model
    );

    let mut results: Vec<(String, bool)> = Vec::new();
    let config = ShiftConfig::default();

    // 1. The ordinary case: burst at full HP, shift, hold, hand it back.
    let world = World::new(2000, 2000, hp_model).with_danger(&[(500, 3000)]);
    let run1 = run("burst at full HP", world, config.clone(), 60, verbose);
    let gained = run1.peak_max_health - run1.start_max_health;
    check(
        &mut results,
        "gains roughly the configured target, overshoot under one tick",
        gained >= config.target_hp_gain && gained < config.target_hp_gain + 200,
    );
    check(
        &mut results,
        "never presses twice inside the settle window",
        run1.presses_within(config.press_settle_ms).is_none(),
    );
    // Landing a little short of the baseline is the safe error: it leaves a
    // sliver of strength unspent. Landing past it would spend agility the
    // player never had.
    check(
        &mut results,
        "returns to the baseline without shifting past it",
        run1.final_max_health >= run1.start_max_health
            && run1.final_max_health <= run1.start_max_health + 3 * HP_PER_POINT * 4,
    );

    // 2. A long fight: it must not hand strength back mid-fight.
    let world = World::new(2000, 1400, hp_model).with_danger(&[(500, 12000)]);
    let run2 = run("sustained fight", world, config.clone(), 70, verbose);
    check(
        &mut results,
        "no agility press while danger is up",
        !run2
            .presses
            .iter()
            .any(|(at, press, _)| *press == Press::Agility && *at < 12000),
    );

    // 3. Already near full strength: the pool runs out mid-shift.
    let world = World::new(4000, 4000, hp_model)
        .with_pool(1800, 4100)
        .with_danger(&[(500, 4000)]);
    let run3 = run("strength pool nearly empty", world, config.clone(), 40, verbose);
    check(
        &mut results,
        "notices the pool is empty instead of pressing at a ceiling",
        run3.notes.iter().any(|(_, note)| note.contains("pool is empty")),
    );
    check(
        &mut results,
        "does not keep pressing once exhausted",
        run3.count(Press::Strength) <= 2,
    );

    // 4. Danger returns while handing strength back. The target is large here
    // so the return is still running when the second fight starts — with the
    // default 330 the shift back is over in under a second and the scenario
    // tests nothing.
    let world = World::new(2000, 2000, hp_model).with_danger(&[(500, 2500), (9000, 14000)]);
    let run4 = run(
        "danger returns mid-return",
        world,
        ShiftConfig {
            target_hp_gain: 1000,
            ..config.clone()
        },
        90,
        verbose,
    );
    check(
        &mut results,
        "reverses with a single strength press",
        run4.presses
            .iter()
            .any(|(at, press, why)| *at >= 9000 && *press == Press::Strength && why.contains("reversing")),
    );
    check(
        &mut results,
        "ends the run holding more strength than it started with",
        run4.final_max_health > run4.start_max_health,
    );

    // 5. Jumped again before the shift back was ever due. A second fight has
    // to buy a second slice, even though the first one never got handed back.
    let world = World::new(2000, 2000, hp_model).with_danger(&[(500, 2500), (5000, 7500)]);
    let run5 = run(
        "second fight before the return timeout",
        world,
        config.clone(),
        60,
        verbose,
    );
    check(
        &mut results,
        "a fresh fight starts a second shift",
        run5
            .presses
            .iter()
            .filter(|(_, press, why)| *press == Press::Strength && why.starts_with("danger —"))
            .count()
            == 2,
    );
    check(
        &mut results,
        "ends up with roughly two targets' worth of strength",
        run5.peak_max_health - run5.start_max_health >= 2 * config.target_hp_gain,
    );

    // 6. The player shifts on their own, with nothing telling the controller.
    // It has to work this out from max_health alone.
    println!("\n─── player shifts manually, unannounced ───");
    let mut world = World::new(2000, 2000, hp_model);
    let mut controller = ShiftController::new(config.clone());
    let mut unannounced_presses = 0;
    let mut inferred = false;
    for _ in 0..40 {
        world.tick();
        // They press F at ~2s and F again at ~4s, exactly as a player would.
        if world.ts_ms >= 2000 && world.ts_ms < 2000 + TICK_MS {
            println!("[{:>6}ms] player presses strength themselves", world.ts_ms);
            world.press(Press::Strength);
        }
        if world.ts_ms >= 4000 && world.ts_ms < 4000 + TICK_MS {
            println!("[{:>6}ms] player stops it themselves", world.ts_ms);
            world.press(Press::Strength);
        }
        match controller.observe(world.observation()) {
            Decision::Press(press, why) => {
                println!("[{:>6}ms] press {:?} — {}", world.ts_ms, press, why);
                world.press(press);
                unannounced_presses += 1;
            }
            Decision::Note(note) => {
                println!("[{:>6}ms] {}", world.ts_ms, note);
                if note.contains("player has the toggles") {
                    inferred = true;
                }
            }
            Decision::Idle => {}
        }
    }
    check(
        &mut results,
        "infers a manual shift from max_health alone",
        inferred,
    );
    check(
        &mut results,
        "presses nothing while the player is driving",
        unannounced_presses == 0,
    );

    // 7. The player takes over mid-plan, announced.
    println!("\n─── player presses a shift key mid-plan ───");
    let mut world = World::new(2000, 2000, hp_model).with_danger(&[(500, 3000)]);
    let mut controller = ShiftController::new(config.clone());
    let mut presses_after_manual = 0;
    for _ in 0..40 {
        world.tick();
        if world.ts_ms >= 2000 && world.ts_ms < 2000 + TICK_MS {
            println!("[{:>6}ms] player presses agility themselves", world.ts_ms);
            controller.note_manual_press(world.ts_ms);
            world.press(Press::Agility);
        }
        if let Decision::Press(press, why) = controller.observe(world.observation()) {
            println!("[{:>6}ms] press {:?} — {}", world.ts_ms, press, why);
            world.press(press);
            if world.ts_ms > 2000 {
                presses_after_manual += 1;
            }
        }
    }
    check(
        &mut results,
        "hands control back for the override window",
        presses_after_manual == 0,
    );

    // 6. Hurt after the fight: holding strength beats handing it back.
    let world = World::new(2000, 900, hp_model).with_danger(&[(500, 3000)]);
    let run6 = run("still hurt when the fight ends", world, config.clone(), 80, verbose);
    check(
        &mut results,
        "keeps the strength while health is under the floor",
        run6.count(Press::Agility) == 0 && run6.final_max_health > run6.start_max_health,
    );

    // 7. The toggle the player asked for.
    let world = World::new(2000, 2000, hp_model).with_danger(&[(500, 3000)]);
    let run7 = run(
        "return_to_agility = false",
        world,
        ShiftConfig {
            return_to_agility: false,
            ..config.clone()
        },
        80,
        verbose,
    );
    check(
        &mut results,
        "never shifts back on its own",
        run7.count(Press::Agility) == 0,
    );

    // 8. Disabled means disabled.
    let world = World::new(2000, 2000, hp_model).with_danger(&[(500, 3000)]);
    let run8 = run(
        "enabled = false",
        world,
        ShiftConfig {
            enabled: false,
            ..config.clone()
        },
        40,
        verbose,
    );
    check(&mut results, "presses nothing at all", run8.presses.is_empty());

    let failed = results.iter().filter(|(_, passed)| !passed).count();
    println!("\n═══ {} checks, {} failed ═══", results.len(), failed);
    for (label, passed) in results.iter().filter(|(_, passed)| !passed) {
        println!("  FAIL {label}");
        let _ = passed;
    }
    if failed > 0 {
        std::process::exit(1);
    }
}
