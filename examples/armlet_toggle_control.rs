//! Armlet / Sabatons toggle controller — the decision loop, proved offline.
//!
//! `sabatons_toggle_probe` measured what a toggle looks like in GSI. The app's
//! own logs (`%LOCALAPPDATA%\dota2-scripts\logs`, 2026-09-01 to 2026-11-21: 215
//! armlet episodes, 973 presses) showed what the old loop did with it. Pairing
//! each press with the HP the next one saw:
//!
//! | What the next press found | Presses | Why |
//! |---|---|---|
//! | the Strength still ramping in, HP pinned at ~240–270 | 354 | a 300ms cooldown against a 0.75–1.0s ramp: every chord cancelled the last |
//! | the toggle off, HP creeping up on regen | 319 | the chord assumes ON; from OFF it switches on and straight back off |
//! | a chord from OFF that left it on | 18 | one press of the two did not land, ~5% |
//! | a chord from ON that left it off at 1–15 HP | 10 | the same, the other way |
//!
//! 37 of the 111 single-press episodes ended in a death inside 3s, most of them
//! on the very next tick: the trick parks the hero at 1 HP until the Strength
//! starts coming back, and a re-fire parks it there again.
//!
//! Three parts:
//!
//! - `ToggleController` — the state machine, written to be lifted into
//!   `src/actions/armlet.rs` unchanged. It reads which way the toggle is from the
//!   sign and shape of `max_health` moves, never presses while the Strength is
//!   still coming in, checks every press against what `max_health` did next, and
//!   presses once instead of the chord when the toggle is off.
//! - `LegacyController` — the loop it replaces, exactly as it ran, so every
//!   scenario shows both.
//! - `World` — a simulator of the toggle wired with what capture 1
//!   (`logs/sabatons_toggle/probe_2026-11-18_21-33-50.jsonl`) measured:
//!
//!   | Constant | Measured | Used for |
//!   |---|---|---|
//!   | GSI tick | 223ms median (212–333) | how often the controller gets to think |
//!   | press → first `max_health` move | 73–287ms | when a press lands |
//!   | Strength ramp | the whole bonus over ~0.6s in ~100ms steps, HP added flat | the window a re-fire cancels |
//!   | switching off | instant; max and current HP fall by what was granted, current floors at 1 | the trick, and its 1 HP window |
//!   | bonus | Armlet +500, Sabatons +600 `max_health` | the size of a toggle |
//!   | death | respawns with the toggle off | state after a death |
//!   | one press of a chord lost | ~5% (18 of 337 chords from OFF, app logs) | modelled at 0% and 5% |
//!   | drain | not isolated: creep damage throughout the capture | modelled at 25 and 50 HP/s |
//!
//!   **The controller itself uses none of these.** It learns the size of a toggle
//!   from the moves it sees, and the two times it is configured with — how long a
//!   press may take to show, how close two presses may be — are in milliseconds.
//!
//! ## The rules it enforces
//!
//! 1. **Read the toggle from `max_health`.** A rise that keeps rising tick over
//!    tick is the Strength coming in: ON. A one-tick drop the size of a toggle is
//!    it switching off: OFF. Death, a fresh item, or a swapped item: OFF. The app
//!    starting with the item already held: unknown.
//! 2. **Never press while the Strength is coming in.** A chord mid-ramp throws
//!    away what the ramp already gave and parks the hero at 1 HP again.
//! 3. **The trick only from ON.** From OFF, one press turns it on: the whole
//!    bonus, and no 1 HP floor at all. With the state unknown the chord is the
//!    safe probe: from OFF it cancels itself, which also says it is OFF.
//! 4. **Check every press.** A chord that switched it off but never back on is
//!    finished with one more press at once. A press that moved nothing did not
//!    land: back off instead of pressing into nothing.
//! 5. **The player's toggles count.** A toggle we did not press holds the
//!    automation off for a moment, so a trick done by hand is never cut in half.
//!    From OFF it only turns the toggle on while HP is actually being lost.
//! 6. **Only press into Dota, and only when it can be used.** Not while stunned,
//!    hexed or muted, and not while another window has the keyboard.
//!
//! ## Running it
//!
//! ```powershell
//! cargo run --release --example armlet_toggle_control              # scenario suite
//! cargo run --release --example armlet_toggle_control -- --verbose # every tick of the scripted scenarios
//! cargo run --release --example armlet_toggle_control -- --seeds 400
//! ```
//!
//! The suite also replays capture 1 through the state tracker when the file is
//! present, and checks what it reads against what the probe measured.

use std::io::BufReader;

use serde_json::Value;

// ─────────────────────────── the controller ───────────────────────────
// Everything in this section is the candidate for src/actions/armlet.rs.

/// Which way the toggle is, as far as `max_health` has said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleState {
    /// Nothing seen yet says: the app started with the item already held.
    Unknown,
    Off,
    On,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// Off and straight back on, one chord. HP floors at 1 and the Strength ramps
    /// back in on top of it: a gain while HP is under the bonus, and only from
    /// ON — from OFF the chord switches it on and straight back off.
    Trick,
    /// One press, to turn it on from OFF: the whole bonus, no floor.
    TurnOn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observation {
    pub at_ms: u64,
    pub alive: bool,
    pub health: u32,
    pub max_health: u32,
    pub level: u32,
    /// Which toggle item is held, as its index in [`TOGGLE_ITEMS`]: Armlet sold
    /// for Sabatons reads as a new item with a new bonus.
    pub item: Option<usize>,
    /// Something besides the toggle went in or out of the inventory — a
    /// purchase, a sale, a neutral item — which moves `max_health` by itself.
    pub inventory_changed: bool,
    /// Stunned, hexed or muted: the item cannot be used, so a press is refused.
    pub disabled: bool,
}

/// What one event told the tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// One of ours landed and the Strength has finished coming in.
    Landed {
        press: Press,
        hp_before: u32,
        hp_low: u32,
        hp_after: u32,
        took_ms: u64,
    },
    /// The trick switched it off, but the press that turns it back on never
    /// showed: it is off, at whatever HP the floor left.
    SecondPressLost { hp: u32 },
    /// A single press switched it off: it was on after all.
    WasOn { hp: u32 },
    /// Nothing moved: the presses never landed, or a trick from OFF switched it
    /// on and straight back off.
    NoEffect { press: Press },
    /// The player toggled it themselves.
    PlayerToggled { on: bool },
    /// The hero died with one of ours still out or its Strength still coming in.
    Died {
        press: Press,
        hp_before: u32,
        hp_low: u32,
        after_ms: u64,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tracked {
    /// HP this event lost to the toggle switching off — ours or the player's.
    /// Not damage: the danger detector and Roshan's hit learning must not see it.
    pub self_inflicted: u32,
    pub outcome: Option<Outcome>,
}

/// What the rest of the app wants from this event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Want {
    /// HP is under the trigger, or Roshan mode says the next hit is lethal.
    pub trick: bool,
    /// The danger detector is up.
    pub in_danger: bool,
    /// Dota has the keyboard. A press anywhere else lands in another window.
    pub focused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// Never press twice inside this.
    pub cooldown_ms: u64,
    /// A press that has not moved `max_health` by now did not land.
    pub confirm_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OurPress {
    press: Press,
    at_ms: u64,
    hp_before: u32,
    hp_low: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Settled,
    /// One of ours is out and `max_health` has not answered yet.
    Pending {
        ours: OurPress,
        belief: ToggleState,
        /// `max_health` once the switch-off half of a trick showed, and when.
        dip: Option<(u32, u64)>,
        /// The hero could not use items at some point while it was out.
        refused: bool,
    },
    /// The Strength is coming in, `max_health` rising tick over tick.
    Ramping {
        floor: u32,
        top: u32,
        started_ms: u64,
        rising_ticks: u32,
        ours: Option<OurPress>,
        /// What we believed before it started, for a rise that turns out not
        /// to be the toggle.
        prior: ToggleState,
    },
}

pub struct ToggleController {
    state: ToggleState,
    phase: Phase,
    last: Option<Observation>,
    /// How far `max_health` moves for a whole toggle of the item held: the
    /// biggest clean move seen. Learned, never assumed.
    bonus: Option<u32>,
    bonus_kind: Option<usize>,
    last_press_ms: Option<u64>,
    /// Ours switched it off and the press turning it back on never showed.
    finish: bool,
    /// Tricks in a row that moved nothing while we believed it on.
    unanswered_tricks: u8,
    /// No pressing before this: the player is mid-toggle, or our presses are
    /// not landing.
    hold_until_ms: Option<u64>,
    last_damage_ms: Option<u64>,
    /// HP the last event lost to something other than the toggle.
    last_tick_loss: u32,
}

impl Default for ToggleController {
    fn default() -> Self {
        Self::new()
    }
}

impl ToggleController {
    pub fn new() -> Self {
        Self {
            state: ToggleState::Unknown,
            phase: Phase::Settled,
            last: None,
            bonus: None,
            bonus_kind: None,
            last_press_ms: None,
            finish: false,
            unanswered_tricks: 0,
            hold_until_ms: None,
            last_damage_ms: None,
            last_tick_loss: 0,
        }
    }

    pub fn state(&self) -> ToggleState {
        self.state
    }

    pub fn last_tick_loss(&self) -> u32 {
        self.last_tick_loss
    }

    pub fn bonus(&self) -> Option<u32> {
        self.bonus
    }

    /// True while a press of ours is out or the Strength is still coming in.
    pub fn busy(&self) -> bool {
        self.phase != Phase::Settled
    }

    fn forget(&mut self, state: ToggleState) {
        self.state = state;
        self.phase = Phase::Settled;
        self.finish = false;
        self.unanswered_tricks = 0;
        self.hold_until_ms = None;
    }

    /// The trick and a fresh ramp can stop short of the whole bonus, so keep
    /// the biggest seen: the gain check needs a lower bound, nothing more.
    fn learn_bonus(&mut self, moved: u32) {
        if moved > 0 {
            self.bonus = Some(self.bonus.map_or(moved, |bonus| bonus.max(moved)));
        }
    }

    /// Read one event. Runs on every event, whatever else is going on.
    pub fn track(&mut self, obs: &Observation, confirm_ms: u64) -> Tracked {
        let mut tracked = Tracked::default();
        let previous = self.last.replace(*obs);
        self.last_tick_loss = 0;

        // Death takes the toggle off: capture 1 respawned at the base max_health.
        if !obs.alive {
            let cut_short = match self.phase {
                Phase::Pending { ours, .. } | Phase::Ramping { ours: Some(ours), .. } => Some(ours),
                _ => None,
            };
            tracked.outcome = cut_short.map(|ours| Outcome::Died {
                press: ours.press,
                hp_before: ours.hp_before,
                hp_low: ours.hp_low,
                after_ms: obs.at_ms.saturating_sub(ours.at_ms),
            });
            self.forget(ToggleState::Off);
            return tracked;
        }
        // Sold, stashed or put in the backpack: it stops, and comes back off.
        let Some(item) = obs.item else {
            self.forget(ToggleState::Off);
            return tracked;
        };
        if self.bonus_kind != Some(item) {
            self.bonus = None;
            self.bonus_kind = Some(item);
        }
        // First sight with the item already held: no telling which way it is.
        let Some(previous) = previous else {
            self.forget(ToggleState::Unknown);
            return tracked;
        };
        // Respawned, or the item only just arrived: off either way.
        if !previous.alive || previous.item != Some(item) {
            self.forget(ToggleState::Off);
            return tracked;
        }

        let d_max = obs.max_health as i64 - previous.max_health as i64;
        // A toggle switching off is flat: current HP falls with max_health and
        // floors at 1, so what it took is the smaller of the two drops.
        let drop_cost = if d_max < 0 {
            (previous.health as i64 - obs.health as i64).clamp(0, -d_max) as u32
        } else {
            0
        };
        // A level or an item moves max_health by itself.
        let noisy = obs.level != previous.level || obs.inventory_changed;

        match self.phase {
            Phase::Pending { .. } => {
                self.track_pending(obs, &previous, d_max, drop_cost, noisy, confirm_ms, &mut tracked)
            }
            Phase::Ramping { .. } => {
                self.track_ramp(obs, d_max, drop_cost, noisy, confirm_ms, &mut tracked)
            }
            Phase::Settled => {
                self.track_settled(obs, &previous, d_max, drop_cost, noisy, confirm_ms, &mut tracked)
            }
        }

        self.last_tick_loss = previous
            .health
            .saturating_sub(obs.health)
            .saturating_sub(tracked.self_inflicted);
        if self.last_tick_loss > 0 {
            self.last_damage_ms = Some(obs.at_ms);
        }
        tracked
    }

    #[allow(clippy::too_many_arguments)]
    fn track_pending(
        &mut self,
        obs: &Observation,
        previous: &Observation,
        d_max: i64,
        drop_cost: u32,
        noisy: bool,
        confirm_ms: u64,
        tracked: &mut Tracked,
    ) {
        let Phase::Pending {
            mut ours,
            belief,
            mut dip,
            mut refused,
        } = self.phase
        else {
            return;
        };
        ours.hp_low = ours.hp_low.min(obs.health);
        refused |= obs.disabled;

        // A level never takes max_health down, so only a sale can muddy a drop.
        if d_max < 0 && !obs.inventory_changed {
            tracked.self_inflicted = drop_cost;
            match ours.press {
                Press::Trick => dip = Some((obs.max_health, obs.at_ms)),
                Press::TurnOn => {
                    // Our one press switched it off: it was on after all.
                    self.learn_bonus((-d_max) as u32);
                    self.state = ToggleState::Off;
                    self.phase = Phase::Settled;
                    self.finish = true;
                    tracked.outcome = Some(Outcome::WasOn { hp: obs.health });
                    return;
                }
            }
        } else if d_max > 0 && !noisy {
            // The Strength is coming in: ours landed.
            self.state = ToggleState::On;
            self.unanswered_tricks = 0;
            self.phase = Phase::Ramping {
                floor: dip.map_or(previous.max_health, |(max, _)| max),
                top: obs.max_health,
                started_ms: ours.at_ms,
                rising_ticks: 1,
                ours: Some(ours),
                prior: belief,
            };
            return;
        }

        // Both halves of the trick go out together, so a press back on that
        // landed is already rising by the tick after the switch-off showed.
        // Waiting out the whole confirm window would leave the hero at 1 HP for
        // it. A quarter of it is the guard against two ticks arriving together.
        let dip_unanswered = dip.is_some_and(|(_, at)| obs.at_ms.saturating_sub(at) >= confirm_ms / 4);
        if !dip_unanswered && obs.at_ms.saturating_sub(ours.at_ms) < confirm_ms {
            self.phase = Phase::Pending {
                ours,
                belief,
                dip,
                refused,
            };
            return;
        }

        self.phase = Phase::Settled;
        match (ours.press, dip) {
            (Press::Trick, Some(_)) => {
                self.state = ToggleState::Off;
                self.finish = true;
                tracked.outcome = Some(Outcome::SecondPressLost { hp: obs.health });
            }
            (Press::Trick, None) => {
                tracked.outcome = Some(Outcome::NoEffect { press: Press::Trick });
                if refused {
                    // Refused while stunned or hexed: the state stands.
                } else if belief == ToggleState::On {
                    // On was read from max_health rising, and switching off
                    // would have shown as a drop: the presses are not landing.
                    // Taunts and cyclones stop items without GSI saying so.
                    // Retry once for a lost press, then wait; a single press
                    // here would switch it off. Only a third miss reads as off.
                    self.unanswered_tricks += 1;
                    match self.unanswered_tricks {
                        1 => {}
                        2 => self.hold_until_ms = Some(obs.at_ms + confirm_ms),
                        _ => {
                            self.state = ToggleState::Off;
                            self.unanswered_tricks = 0;
                        }
                    }
                } else {
                    // From OFF the chord switches it on and straight back off,
                    // which leaves nothing to see.
                    self.state = ToggleState::Off;
                    self.unanswered_tricks = 0;
                }
            }
            (Press::TurnOn, _) => {
                tracked.outcome = Some(Outcome::NoEffect {
                    press: Press::TurnOn,
                });
                // Not landing: wait before pressing into nothing again.
                self.hold_until_ms = Some(obs.at_ms + confirm_ms);
            }
        }
    }

    fn track_ramp(
        &mut self,
        obs: &Observation,
        d_max: i64,
        drop_cost: u32,
        noisy: bool,
        confirm_ms: u64,
        tracked: &mut Tracked,
    ) {
        let Phase::Ramping {
            floor,
            mut top,
            started_ms,
            mut rising_ticks,
            mut ours,
            prior,
        } = self.phase
        else {
            return;
        };
        if let Some(press) = ours.as_mut() {
            press.hp_low = press.hp_low.min(obs.health);
        }

        // Still rising, unless it has been rising for far longer than any ramp.
        let overlong = obs.at_ms.saturating_sub(started_ms) > confirm_ms * 4;
        if d_max > 0 && !overlong {
            top = obs.max_health;
            rising_ticks += 1;
            self.phase = Phase::Ramping {
                floor,
                top,
                started_ms,
                rising_ticks,
                ours,
                prior,
            };
            return;
        }

        if d_max < 0 && !noisy {
            // Switched off before the Strength was all in. Never us: we do not
            // press during a ramp. The player may be mid-trick, so hold off.
            tracked.self_inflicted = drop_cost;
            self.learn_bonus((-d_max) as u32);
            self.state = ToggleState::Off;
            self.phase = Phase::Settled;
            self.hold_until_ms = Some(obs.at_ms + confirm_ms);
            tracked.outcome = Some(Outcome::PlayerToggled { on: false });
            return;
        }

        // Stopped rising: the Strength is in.
        let gained = top.saturating_sub(floor);
        self.phase = Phase::Settled;
        match ours {
            Some(press) => {
                self.state = ToggleState::On;
                self.learn_bonus(gained);
                tracked.outcome = Some(Outcome::Landed {
                    press: press.press,
                    hp_before: press.hp_before,
                    hp_low: press.hp_low,
                    hp_after: obs.health,
                    took_ms: obs.at_ms.saturating_sub(press.at_ms),
                });
            }
            None => {
                // A ramp spans several ticks; a one-tick rise is a buff, or an
                // attribute coming back. With the size known, it can also be a
                // ramp squeezed between two slow ticks.
                let toggle_sized = self.bonus.is_some_and(|bonus| gained >= bonus / 2);
                if rising_ticks >= 2 || toggle_sized {
                    self.state = ToggleState::On;
                    self.learn_bonus(gained);
                    tracked.outcome = Some(Outcome::PlayerToggled { on: true });
                } else {
                    self.state = prior;
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn track_settled(
        &mut self,
        obs: &Observation,
        previous: &Observation,
        d_max: i64,
        drop_cost: u32,
        noisy: bool,
        confirm_ms: u64,
        tracked: &mut Tracked,
    ) {
        if noisy || d_max == 0 {
            return;
        }
        if d_max > 0 {
            // Rising with nothing of ours out: the player switching it on, or a
            // buff. The shape of the rise decides.
            self.phase = Phase::Ramping {
                floor: previous.max_health,
                top: obs.max_health,
                started_ms: obs.at_ms,
                rising_ticks: 1,
                ours: None,
                prior: self.state,
            };
            return;
        }

        let drop = (-d_max) as u32;
        let toggle_sized = self
            .bonus
            .is_some_and(|bonus| drop >= bonus / 2 && drop <= bonus + bonus / 4);
        // Before any toggle has taught us the size, a drop that floored HP can
        // only be the toggle: the floor is its signature.
        let floored = obs.health <= 2 && previous.health > 2;
        if toggle_sized || floored {
            tracked.self_inflicted = drop_cost;
            self.state = ToggleState::Off;
            // Their press back on may be on its way.
            self.hold_until_ms = Some(obs.at_ms + confirm_ms);
            tracked.outcome = Some(Outcome::PlayerToggled { on: false });
        }
    }

    /// Decide whether to press, after `track` has read the same event.
    pub fn decide(
        &mut self,
        obs: &Observation,
        want: Want,
        timing: Timing,
    ) -> Option<(Press, &'static str)> {
        // GSI can say alive on the tick the hero dies, at 0 HP.
        if !obs.alive || obs.health == 0 || obs.item.is_none() || self.phase != Phase::Settled {
            return None;
        }
        if let Some(until) = self.hold_until_ms {
            if obs.at_ms < until {
                return None;
            }
            self.hold_until_ms = None;
        }
        if obs.disabled || !want.focused {
            return None;
        }

        let choice = if self.finish {
            Some((Press::TurnOn, "the trick left it off — turning it back on"))
        } else if !want.trick
            || self
                .last_press_ms
                .is_some_and(|at| obs.at_ms.saturating_sub(at) < timing.cooldown_ms)
        {
            None
        } else {
            match self.state {
                // Off at HP h, the trick leaves max(1, h - bonus) and adds the
                // bonus back: nothing to gain once h reaches the bonus.
                ToggleState::On if self.bonus.is_some_and(|bonus| obs.health >= bonus) => None,
                ToggleState::On => Some((Press::Trick, "low HP, toggle on")),
                ToggleState::Unknown => Some((
                    Press::Trick,
                    "low HP, toggle unknown — from off the trick cancels itself",
                )),
                ToggleState::Off
                    if want.in_danger
                        || self
                            .last_damage_ms
                            .is_some_and(|at| obs.at_ms.saturating_sub(at) <= timing.confirm_ms) =>
                {
                    Some((Press::TurnOn, "low HP under fire, toggle off"))
                }
                ToggleState::Off => None,
            }
        };

        if let Some((press, _)) = choice {
            self.finish = false;
            self.last_press_ms = Some(obs.at_ms);
            self.phase = Phase::Pending {
                ours: OurPress {
                    press,
                    at_ms: obs.at_ms,
                    hp_before: obs.health,
                    hp_low: obs.health,
                },
                belief: self.state,
                dip: None,
                refused: false,
            };
        }
        choice
    }
}

// ─────────────────────────── the loop it replaces ───────────────────────────

/// `src/actions/armlet.rs` before this: the chord whenever HP is under the
/// trigger and the cooldown has passed, plus a forced retry while HP stays
/// critical. It never asks which way the toggle is.
#[derive(Default)]
struct LegacyController {
    last_toggle_ms: Option<u64>,
    critical: Option<u32>,
}

impl LegacyController {
    fn decide(&mut self, obs: &Observation, threshold: u32, offset: u32, cooldown_ms: u64) -> Option<Press> {
        if !obs.alive || obs.item.is_none() {
            return None;
        }
        let ready = self
            .last_toggle_ms
            .is_none_or(|at| obs.at_ms.saturating_sub(at) >= cooldown_ms);
        if let Some(critical) = self.critical {
            if obs.health < threshold / 2 && obs.health <= critical && ready {
                self.critical = None;
                self.last_toggle_ms = Some(obs.at_ms);
                return Some(Press::Trick);
            }
        }
        if obs.health >= threshold + offset {
            self.critical = None;
            return None;
        }
        if obs.disabled || !ready {
            return None;
        }
        self.last_toggle_ms = Some(obs.at_ms);
        self.critical = (obs.health < threshold / 2).then_some(obs.health);
        Some(Press::Trick)
    }
}

// ─────────────────────────── the simulated world ───────────────────────────
// Wired with capture 1. None of it is known to the controller.

const STEP_MS: u64 = 5;
const TICK_MS: (u64, u64) = (212, 240);
const SLOW_TICK_MS: u64 = 333;
const PRESS_LATENCY_MS: (u64, u64) = (90, 290);
const RAMP_STEP_MS: u64 = 100;
const RAMP_STEPS: u32 = 6;
const RESPAWN_MS: u64 = 3000;
const ARMLET: usize = 0;
const SABATONS: usize = 1;
const ARMLET_BONUS: u32 = 500;
const SABATONS_BONUS: u32 = 600;
/// The app's order: Armlet wins if both are held.
const TOGGLE_ITEMS: &[&str] = &["item_armlet", "item_berserkers_sabatons"];

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo + 1)
    }

    fn chance(&mut self, p: f64) -> bool {
        ((self.next() >> 11) as f64 / (1u64 << 53) as f64) < p
    }
}

#[derive(Debug, Clone, Copy)]
struct Hits {
    from_ms: u64,
    to_ms: u64,
    every_ms: u64,
    damage: f64,
    next_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Who {
    Ours,
    Player,
}

#[derive(Debug, Clone, Copy)]
struct Model {
    drain: f64,
    press_loss: f64,
}

const MODELS: [Model; 4] = [
    Model { drain: 25.0, press_loss: 0.0 },
    Model { drain: 50.0, press_loss: 0.0 },
    Model { drain: 25.0, press_loss: 0.05 },
    Model { drain: 50.0, press_loss: 0.05 },
];

#[derive(Debug, Clone, Default)]
struct Stats {
    tricks: u32,
    turn_ons: u32,
    /// Pressed while the Strength was still coming in.
    mid_ramp_presses: u32,
    /// A chord sent while the toggle was off: it cancels itself.
    tricks_from_off: u32,
    /// Pressed while another window had the keyboard.
    lost_into_void: u32,
    /// A single press sent while the toggle was on: it switches it off.
    turn_ons_while_on: u32,
    /// Our switch-off left the hero at 1–2 HP.
    floor_trips: u32,
    deaths: u32,
    min_hp: u32,
    /// Time alive at 50 HP or less.
    ms_low: u64,
}

struct World {
    now: u64,
    rng_input: Rng,
    rng_ticks: Rng,
    alive: bool,
    respawn_at: Option<u64>,
    base_max: u32,
    level: u32,
    health: f64,
    regen: f64,
    drain: f64,
    item: Option<usize>,
    bonus: u32,
    on: bool,
    granted: u32,
    next_step_ms: u64,
    inflight: Vec<(u64, Who)>,
    press_loss: f64,
    /// Indices of our presses (counting each press of a chord) that never land.
    forced_losses: Vec<usize>,
    ours_sent: usize,
    hits: Vec<Hits>,
    stuns: Vec<(u64, u64)>,
    /// Item use stopped with nothing in GSI to say so: a taunt, a cyclone.
    taunts: Vec<(u64, u64)>,
    unfocused: Vec<(u64, u64)>,
    player_presses: Vec<u64>,
    level_ups: Vec<u64>,
    purchases: Vec<(u64, u32)>,
    swaps: Vec<(u64, usize, u32)>,
    inventory_dirty: bool,
    last_hit_ms: Option<u64>,
    stats: Stats,
}

impl World {
    fn new(seed: u64, model: Model, base_max: u32, kind: usize, bonus: u32) -> Self {
        Self {
            now: 0,
            rng_input: Rng::new(seed ^ 0xA5A5_A5A5),
            rng_ticks: Rng::new(seed ^ 0x5A5A_5A5A),
            alive: true,
            respawn_at: None,
            base_max,
            level: 12,
            health: base_max as f64,
            regen: 8.0,
            drain: model.drain,
            item: Some(kind),
            bonus,
            on: false,
            granted: 0,
            next_step_ms: 0,
            inflight: Vec::new(),
            press_loss: model.press_loss,
            forced_losses: Vec::new(),
            ours_sent: 0,
            hits: Vec::new(),
            stuns: Vec::new(),
            taunts: Vec::new(),
            unfocused: Vec::new(),
            player_presses: Vec::new(),
            level_ups: Vec::new(),
            purchases: Vec::new(),
            swaps: Vec::new(),
            inventory_dirty: false,
            last_hit_ms: None,
            stats: Stats {
                min_hp: u32::MAX,
                ..Stats::default()
            },
        }
    }

    /// Already on when the run starts, the Strength all in.
    fn starting_on(mut self) -> Self {
        self.on = true;
        self.granted = self.bonus;
        self.health += self.bonus as f64;
        self
    }

    fn at_health(mut self, health: f64) -> Self {
        self.health = health;
        self
    }

    fn hit(mut self, from_ms: u64, to_ms: u64, every_ms: u64, damage: f64, phase_ms: u64) -> Self {
        self.hits.push(Hits {
            from_ms,
            to_ms,
            every_ms,
            damage,
            next_ms: from_ms + phase_ms,
        });
        self
    }

    fn max_health(&self) -> u32 {
        self.base_max + self.granted
    }

    fn stunned(&self) -> bool {
        self.stuns.iter().any(|(from, to)| self.now >= *from && self.now < *to)
    }

    fn taunted(&self) -> bool {
        self.taunts.iter().any(|(from, to)| self.now >= *from && self.now < *to)
    }

    fn focused(&self) -> bool {
        !self.unfocused.iter().any(|(from, to)| self.now >= *from && self.now < *to)
    }

    fn in_danger(&self) -> bool {
        self.last_hit_ms.is_some_and(|at| self.now.saturating_sub(at) <= 2000)
    }

    fn ramping(&self) -> bool {
        self.on && self.granted < self.bonus
    }

    fn tick_interval(&mut self) -> u64 {
        if self.rng_ticks.chance(0.05) {
            SLOW_TICK_MS
        } else {
            self.rng_ticks.range(TICK_MS.0, TICK_MS.1)
        }
    }

    fn press(&mut self, press: Press) {
        if !self.focused() {
            self.stats.lost_into_void += 1;
            return;
        }
        match press {
            Press::Trick => {
                self.stats.tricks += 1;
                if !self.on {
                    self.stats.tricks_from_off += 1;
                }
            }
            Press::TurnOn => {
                self.stats.turn_ons += 1;
                if self.on {
                    self.stats.turn_ons_while_on += 1;
                }
            }
        }
        if self.ramping() {
            self.stats.mid_ramp_presses += 1;
        }
        let latency = self.rng_input.range(PRESS_LATENCY_MS.0, PRESS_LATENCY_MS.1);
        let count = match press {
            Press::Trick => 2,
            Press::TurnOn => 1,
        };
        for k in 0..count {
            let index = self.ours_sent;
            self.ours_sent += 1;
            let lost = self.forced_losses.contains(&index) || self.rng_input.chance(self.press_loss);
            if !lost {
                self.inflight.push((self.now + latency + k, Who::Ours));
            }
        }
    }

    fn toggle(&mut self, who: Who) {
        if !self.alive || self.item.is_none() || self.stunned() || self.taunted() {
            return;
        }
        if self.on {
            let before = self.health;
            self.health = (self.health - self.granted as f64).max(1.0);
            if who == Who::Ours && self.health <= 2.0 && before > 2.0 {
                self.stats.floor_trips += 1;
            }
            self.granted = 0;
            self.on = false;
        } else {
            self.on = true;
            self.granted = 0;
            self.next_step_ms = self.now + RAMP_STEP_MS;
        }
    }

    fn die(&mut self) {
        self.alive = false;
        self.health = 0.0;
        self.stats.deaths += 1;
        self.on = false;
        self.granted = 0;
        self.inflight.clear();
        self.respawn_at = Some(self.now + RESPAWN_MS);
    }

    fn step(&mut self) {
        self.now += STEP_MS;
        let now = self.now;

        if !self.alive {
            if self.respawn_at.is_some_and(|at| now >= at) {
                self.alive = true;
                self.respawn_at = None;
                self.on = false;
                self.granted = 0;
                self.health = self.base_max as f64;
            } else {
                return;
            }
        }

        while self.level_ups.first().is_some_and(|at| *at <= now) {
            self.level_ups.remove(0);
            self.base_max += 60;
            self.health += 60.0;
            self.level += 1;
        }
        while self.purchases.first().is_some_and(|(at, _)| *at <= now) {
            let (_, gain) = self.purchases.remove(0);
            self.base_max += gain;
            self.health += gain as f64;
            self.inventory_dirty = true;
        }
        while self.swaps.first().is_some_and(|(at, _, _)| *at <= now) {
            let (_, kind, bonus) = self.swaps.remove(0);
            // The old item leaves the inventory and takes what it granted.
            if self.on {
                self.health = (self.health - self.granted as f64).max(1.0);
            }
            self.on = false;
            self.granted = 0;
            self.item = Some(kind);
            self.bonus = bonus;
            self.inventory_dirty = true;
        }
        while self.player_presses.first().is_some_and(|at| *at <= now) {
            self.player_presses.remove(0);
            let latency = self.rng_input.range(PRESS_LATENCY_MS.0, PRESS_LATENCY_MS.1);
            self.inflight.push((now + latency, Who::Player));
        }

        let mut landed = Vec::new();
        self.inflight.retain(|(at, who)| {
            if *at <= now {
                landed.push(*who);
                false
            } else {
                true
            }
        });
        for who in landed {
            self.toggle(who);
        }

        if self.ramping() && now >= self.next_step_ms {
            let step = self.bonus.div_ceil(RAMP_STEPS).min(self.bonus - self.granted);
            self.granted += step;
            self.health += step as f64;
            self.next_step_ms += RAMP_STEP_MS;
        }

        let dt = STEP_MS as f64 / 1000.0;
        self.health += self.regen * dt;
        // The drain cannot kill.
        if self.on && self.health > 1.0 {
            self.health = (self.health - self.drain * dt).max(1.0);
        }
        self.health = self.health.min(self.max_health() as f64);

        for index in 0..self.hits.len() {
            let hit = self.hits[index];
            if now < hit.from_ms || now > hit.to_ms || now < hit.next_ms {
                continue;
            }
            self.hits[index].next_ms = now + hit.every_ms;
            self.health -= hit.damage;
            self.last_hit_ms = Some(now);
            if self.health <= 0.0 {
                self.die();
                return;
            }
        }

        if self.health <= 50.0 {
            self.stats.ms_low += STEP_MS;
        }
        self.stats.min_hp = self.stats.min_hp.min(self.health.max(0.0) as u32);
    }

    fn observation(&mut self) -> Observation {
        Observation {
            at_ms: self.now,
            alive: self.alive,
            health: if self.alive { self.health.max(0.0) as u32 } else { 0 },
            max_health: if self.alive { self.max_health() } else { self.base_max },
            level: self.level,
            item: self.item,
            inventory_changed: std::mem::take(&mut self.inventory_dirty),
            disabled: self.alive && self.stunned(),
        }
    }
}

// ─────────────────────────── running a scenario ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Driver {
    New,
    Legacy,
}

struct Setup {
    world: World,
    end_ms: u64,
    threshold: u32,
    offset: u32,
    cooldown_ms: u64,
    confirm_ms: u64,
}

impl Setup {
    /// Your Huskar settings: trick under 130 + 150 HP, 300ms between toggles.
    fn new(world: World, end_ms: u64) -> Self {
        Self {
            world,
            end_ms,
            threshold: 130,
            offset: 150,
            cooldown_ms: 300,
            confirm_ms: 600,
        }
    }

    fn trigger(mut self, threshold: u32, offset: u32) -> Self {
        self.threshold = threshold;
        self.offset = offset;
        self
    }
}

struct Run {
    stats: Stats,
    /// When, what, and the HP it was pressed at.
    presses: Vec<(u64, Press, u32)>,
    outcomes: Vec<(u64, Outcome)>,
    bonus: Option<u32>,
    world_on: bool,
}

impl Run {
    fn first_press_after(&self, at_ms: u64) -> Option<(u64, Press)> {
        self.presses
            .iter()
            .find(|(t, ..)| *t >= at_ms)
            .map(|(t, press, _)| (*t, *press))
    }

    fn presses_between(&self, from_ms: u64, to_ms: u64) -> usize {
        self.presses.iter().filter(|(t, ..)| *t >= from_ms && *t < to_ms).count()
    }
}

fn run(mut setup: Setup, driver: Driver, verbose: bool) -> Run {
    let mut controller = ToggleController::new();
    let mut legacy = LegacyController::default();
    let timing = Timing {
        cooldown_ms: setup.cooldown_ms,
        confirm_ms: setup.confirm_ms,
    };
    let trigger = setup.threshold + setup.offset;
    let mut out = Run {
        stats: Stats::default(),
        presses: Vec::new(),
        outcomes: Vec::new(),
        bonus: None,
        world_on: false,
    };

    let world = &mut setup.world;
    let mut next_tick = world.tick_interval();
    while world.now < setup.end_ms {
        world.step();
        if world.now < next_tick {
            continue;
        }
        next_tick = world.now + world.tick_interval();
        let obs = world.observation();
        let press = match driver {
            Driver::New => {
                let tracked = controller.track(&obs, setup.confirm_ms);
                if let Some(outcome) = tracked.outcome {
                    if verbose {
                        println!("[{:>6}ms] {:?}", obs.at_ms, outcome);
                    }
                    out.outcomes.push((obs.at_ms, outcome));
                }
                let want = Want {
                    trick: obs.health < trigger,
                    in_danger: world.in_danger(),
                    focused: world.focused(),
                };
                controller.decide(&obs, want, timing).map(|(press, why)| {
                    if verbose {
                        println!("[{:>6}ms] press {:?} — {} (hp {})", obs.at_ms, press, why, obs.health);
                    }
                    press
                })
            }
            Driver::Legacy => legacy.decide(&obs, setup.threshold, setup.offset, setup.cooldown_ms),
        };
        if verbose {
            println!(
                "[{:>6}ms] {:>4} / {:<4} {}{}{}",
                obs.at_ms,
                obs.health,
                obs.max_health,
                if world.on { "ON " } else { "off" },
                if obs.alive { "" } else { "  dead" },
                if driver == Driver::New {
                    format!("  believes {:?}", controller.state())
                } else {
                    String::new()
                }
            );
        }
        if let Some(press) = press {
            out.presses.push((obs.at_ms, press, obs.health));
            world.press(press);
        }
    }

    out.stats = world.stats.clone();
    out.bonus = controller.bonus();
    out.world_on = world.on;
    out
}

fn check(results: &mut Vec<(String, bool)>, label: &str, passed: bool) {
    println!("     {} {}", if passed { "PASS" } else { "FAIL" }, label);
    results.push((label.to_string(), passed));
}

fn header(name: &str) {
    println!("\n─── {name} ───");
}

fn summary_line(label: &str, run: &Run) {
    let s = &run.stats;
    println!(
        "     {label:<7} presses {:>3} (tricks {:>3}, on {:>2})  mid-ramp {:>3}  from-off {:>3}  into-void {:>3}  floor trips {:>3}  deaths {}  min hp {}",
        run.presses.len(),
        s.tricks,
        s.turn_ons,
        s.mid_ramp_presses,
        s.tricks_from_off,
        s.lost_into_void,
        s.floor_trips,
        s.deaths,
        if s.min_hp == u32::MAX { 0 } else { s.min_hp }
    );
}

/// Run the same world through both loops and print the pair.
fn both(name: &str, build: impl Fn() -> Setup, verbose: bool) -> (Run, Run) {
    header(name);
    let new = run(build(), Driver::New, verbose);
    let legacy = run(build(), Driver::Legacy, false);
    summary_line("new", &new);
    summary_line("legacy", &legacy);
    (new, legacy)
}

// ─────────────────────────── scripted scenarios ───────────────────────────

const STEADY: Model = Model {
    drain: 35.0,
    press_loss: 0.0,
};

/// A Huskar-sized hero with Armlet off; the player switches it on at 1s, so
/// the controller has seen one toggle before anything happens.
fn armlet_hero(seed: u64, model: Model) -> World {
    let mut world = World::new(seed, model, 1300, ARMLET, ARMLET_BONUS);
    world.player_presses.push(1000);
    world
}

fn scripted(results: &mut Vec<(String, bool)>, verbose: bool) {
    // 1. The ordinary trick under creep hits.
    let (new, legacy) = both(
        "trick under creep hits",
        || Setup::new(armlet_hero(1, STEADY).at_health(800.0).hit(3000, 11000, 700, 120.0, 0), 12000),
        verbose,
    );
    check(results, "never presses while the Strength is coming in", new.stats.mid_ramp_presses == 0);
    check(results, "every trick is from ON", new.stats.tricks_from_off == 0);
    check(
        results,
        "the old loop re-fires mid-ramp here, as the logs showed",
        legacy.stats.mid_ramp_presses > 0,
    );
    check(
        results,
        "survives the fight the old loop's re-fire died in",
        new.stats.deaths == 0 && legacy.stats.deaths > 0,
    );

    // 2. The chord's second press does not land.
    let (new, legacy) = both(
        "trick's second press lost",
        || {
            let mut world = armlet_hero(2, STEADY).at_health(700.0).hit(3000, 9000, 900, 120.0, 0);
            world.forced_losses = vec![1];
            Setup::new(world, 10000)
        },
        verbose,
    );
    let lost_at = new
        .outcomes
        .iter()
        .find(|(_, outcome)| matches!(outcome, Outcome::SecondPressLost { .. }))
        .map(|(t, _)| *t);
    check(results, "notices the trick left it off", lost_at.is_some());
    check(
        results,
        "turns it back on with one press, at once",
        lost_at.is_some_and(|t| new.first_press_after(t).is_some_and(|(at, press)| press == Press::TurnOn && at - t < 300)),
    );
    check(results, "ends on", new.world_on);
    check(
        results,
        "the old loop chords from OFF instead",
        legacy.stats.tricks_from_off > 0,
    );

    // 3. Dota takes none of the presses.
    let (new, legacy) = both(
        "no press lands",
        || {
            let world = World::new(3, Model { drain: 35.0, press_loss: 1.0 }, 1300, ARMLET, ARMLET_BONUS)
                .starting_on()
                .at_health(250.0);
            Setup::new(world, 6000)
        },
        verbose,
    );
    check(results, "backs off instead of pressing every other tick", new.presses.len() <= 7);
    check(results, "the old loop kept pressing", legacy.presses.len() > new.presses.len() * 2);

    // 4. The player does the trick by hand.
    let (new, _) = both(
        "player does the trick by hand",
        || {
            // Low enough that their switch-off floors HP under the trigger, so
            // the automation has every reason to step in and must not.
            let mut world = armlet_hero(4, STEADY).at_health(300.0).hit(2500, 9000, 1500, 120.0, 0);
            world.player_presses.extend([4000, 4150]);
            Setup::new(world, 7000)
        },
        verbose,
    );
    check(
        results,
        "presses nothing while the player's trick is going",
        new.presses_between(4000, 5200) == 0,
    );
    check(results, "the player's trick ends on", new.world_on);

    // 5. The player switches it off out of combat, low.
    let (new, _) = both(
        "player switches it off out of combat",
        || {
            // Their switch-off leaves it under the trigger.
            let mut world = armlet_hero(5, STEADY).at_health(300.0);
            world.player_presses.push(3000);
            Setup::new(world, 9000)
        },
        verbose,
    );
    check(results, "leaves it off while nothing is hitting", new.presses.is_empty());

    // 6. ...and then a fight starts.
    let (new, legacy) = both(
        "player switches it off, then a fight starts",
        || {
            let mut world = armlet_hero(6, STEADY).at_health(300.0).hit(6000, 12000, 800, 90.0, 0);
            world.player_presses.push(3000);
            Setup::new(world, 12000)
        },
        verbose,
    );
    check(
        results,
        "turns it on with one press, not the chord",
        new.first_press_after(0).is_some_and(|(_, press)| press == Press::TurnOn),
    );
    check(results, "the old loop chords from OFF", legacy.stats.tricks_from_off > 0);

    // 7. Death with it on, respawn, a new fight.
    let (new, legacy) = both(
        "dies with it on, respawns",
        || {
            let world = armlet_hero(7, STEADY)
                .hit(4000, 4000, 1000, 5000.0, 0)
                .hit(9000, 15000, 700, 120.0, 0);
            Setup::new(world, 15000)
        },
        verbose,
    );
    check(
        results,
        "after the respawn it is off: one press, not the chord",
        new.first_press_after(7000).is_some_and(|(_, press)| press == Press::TurnOn),
    );
    check(results, "the old loop chords from OFF", legacy.stats.tricks_from_off > 0);

    // 8. The app starts with it already on.
    let (new, _) = both(
        "app started, toggle on",
        || {
            let world = World::new(8, STEADY, 1300, ARMLET, ARMLET_BONUS)
                .starting_on()
                .at_health(700.0)
                .hit(2000, 8000, 700, 120.0, 0);
            Setup::new(world, 9000)
        },
        verbose,
    );
    check(
        results,
        "probes with the trick and it lands",
        new.presses.first().is_some_and(|(_, press, _)| *press == Press::Trick)
            && new
                .outcomes
                .iter()
                .any(|(_, outcome)| matches!(outcome, Outcome::Landed { press: Press::Trick, .. })),
    );

    // 9. The app starts with it off.
    let (new, legacy) = both(
        "app started, toggle off",
        || Setup::new(World::new(9, STEADY, 1300, ARMLET, ARMLET_BONUS).hit(2000, 12000, 700, 100.0, 0), 13000),
        verbose,
    );
    check(
        results,
        "one trick to probe, then one press to turn it on",
        new.presses.len() >= 2 && new.presses[0].1 == Press::Trick && new.presses[1].1 == Press::TurnOn,
    );
    check(results, "never a second trick from OFF", new.stats.tricks_from_off <= 1);
    check(results, "the old loop chords from OFF over and over", legacy.stats.tricks_from_off > 3);

    // 10. Levels and a purchase with it on.
    let (new, _) = both(
        "levels and a purchase, toggle on",
        || {
            let mut world = armlet_hero(10, STEADY).at_health(1000.0).hit(6000, 14000, 600, 150.0, 0);
            world.level_ups.extend([3000, 3500]);
            world.purchases.push((4000, 250));
            Setup::new(world, 15000)
        },
        verbose,
    );
    check(
        results,
        "still believes it on, so the fight gets the trick",
        new.first_press_after(0).is_some_and(|(_, press)| press == Press::Trick) && new.stats.tricks_from_off == 0,
    );

    // 11. Armlet sold for Sabatons.
    let (new, _) = both(
        "Armlet sold for Sabatons",
        || {
            let mut world = armlet_hero(11, STEADY).hit(6000, 14000, 600, 150.0, 0);
            world.swaps.push((3000, SABATONS, SABATONS_BONUS));
            world.player_presses.push(4000);
            Setup::new(world, 15000)
        },
        verbose,
    );
    check(results, "learns the new item's bonus", new.bonus == Some(SABATONS_BONUS));
    check(results, "and tricks from ON with it", new.stats.tricks_from_off == 0 && new.stats.tricks > 0);

    // 12. Stunned through the trigger.
    let (new, _) = both(
        "stunned through the trigger",
        || {
            let mut world = armlet_hero(12, STEADY).hit(3000, 9000, 600, 140.0, 0);
            world.stuns.push((5000, 6500));
            Setup::new(world, 9000)
        },
        verbose,
    );
    check(results, "presses nothing while stunned", new.presses_between(5000, 6500) == 0);

    // 13. Another window has the keyboard.
    let (new, legacy) = both(
        "Dota loses focus",
        || {
            let mut world = World::new(13, STEADY, 1300, ARMLET, ARMLET_BONUS).starting_on().at_health(260.0);
            world.unfocused.push((0, 6000));
            Setup::new(world, 8000)
        },
        verbose,
    );
    check(results, "presses nothing into another window", new.stats.lost_into_void == 0);
    check(results, "the old loop typed into it", legacy.stats.lost_into_void > 5);

    // 14. A trigger above what the trick can give.
    let (new, _) = both(
        "trigger above the bonus",
        || {
            let world = armlet_hero(14, STEADY).hit(3000, 12000, 700, 150.0, 0);
            Setup::new(world, 13000).trigger(600, 100)
        },
        verbose,
    );
    check(
        results,
        "no trick while HP is at or over the bonus",
        new.presses
            .iter()
            .all(|(_, press, hp)| *press != Press::Trick || *hp < ARMLET_BONUS)
            && new.stats.tricks > 0,
    );

    // 15. Drain at low HP, nobody around.
    let (new, legacy) = both(
        "drain at low HP out of combat",
        || Setup::new(World::new(15, STEADY, 1300, ARMLET, ARMLET_BONUS).starting_on().at_health(420.0), 30000),
        verbose,
    );
    check(results, "keeps it topped up without a single mid-ramp press", new.stats.mid_ramp_presses == 0 && new.stats.tricks >= 2);
    check(results, "the old loop needed more presses for it", legacy.presses.len() >= new.presses.len());

    // 16. Taunted while on, which GSI does not report (logged 2026-11-21).
    let (new, _) = both(
        "taunted while on",
        || {
            // The player switches it on, so the controller has seen it go on,
            // then one big hit lands them under the trigger mid-taunt.
            let mut world = World::new(16, STEADY, 1300, ARMLET, ARMLET_BONUS).at_health(300.0);
            world.player_presses.push(300);
            world.taunts.push((1400, 3400));
            Setup::new(world.hit(1500, 1600, 1000, 500.0, 0), 6000)
        },
        verbose,
    );
    check(
        results,
        "never a single press into it while it is on",
        new.stats.turn_ons_while_on == 0,
    );
    check(
        results,
        "the trick lands once the taunt ends",
        new.presses.iter().any(|(t, press, _)| *t >= 3400 && *press == Press::Trick) && new.world_on,
    );
}

// ─────────────────────────── fights, many seeds ───────────────────────────

/// Random sources of hits, the same for both loops on a given seed.
fn random_hits(world: World, seed: u64, sources: (u64, u64), damage: (u64, u64), every: (u64, u64), from_ms: u64, to_ms: u64) -> World {
    let mut rng = Rng::new(seed ^ 0x1234_5678);
    let count = rng.range(sources.0, sources.1);
    let mut world = world;
    for _ in 0..count {
        let every_ms = rng.range(every.0, every.1);
        let dmg = rng.range(damage.0, damage.1) as f64;
        let phase = rng.range(0, every_ms);
        world = world.hit(from_ms, to_ms, every_ms, dmg, phase);
    }
    world
}

#[derive(Default)]
struct Totals {
    runs: u32,
    presses: u64,
    mid_ramp: u64,
    from_off: u64,
    floor_trips: u64,
    deaths: u64,
    runs_with_death: u32,
    ms_low: u64,
    max_from_off: u32,
}

impl Totals {
    fn add(&mut self, run: &Run) {
        self.runs += 1;
        self.presses += run.presses.len() as u64;
        self.mid_ramp += run.stats.mid_ramp_presses as u64;
        self.from_off += run.stats.tricks_from_off as u64;
        self.floor_trips += run.stats.floor_trips as u64;
        self.deaths += run.stats.deaths as u64;
        if run.stats.deaths > 0 {
            self.runs_with_death += 1;
        }
        self.ms_low += run.stats.ms_low;
        self.max_from_off = self.max_from_off.max(run.stats.tricks_from_off);
    }

    fn line(&self, label: &str) {
        let n = self.runs.max(1) as f64;
        println!(
            "     {label:<7} presses {:>5.1}  mid-ramp {:>5.1}  from-off {:>5.1}  floor trips {:>4.1}  died in {:>5.1}% of runs  {:>5.0}ms at ≤50 HP",
            self.presses as f64 / n,
            self.mid_ramp as f64 / n,
            self.from_off as f64 / n,
            self.floor_trips as f64 / n,
            100.0 * self.runs_with_death as f64 / n,
            self.ms_low as f64 / n,
        );
    }
}

type Builder = fn(u64, Model) -> Setup;

fn lane_skirmish(seed: u64, model: Model) -> Setup {
    let world = random_hits(armlet_hero(seed, model), seed, (1, 3), (40, 110), (700, 1300), 3000, 13000);
    Setup::new(world, 15000)
}

fn teamfight(seed: u64, model: Model) -> Setup {
    let mut world = World::new(seed, model, 1500, SABATONS, SABATONS_BONUS);
    world.player_presses.push(1000);
    let world = random_hits(world, seed, (1, 2), (70, 130), (600, 900), 3000, 9000);
    let world = random_hits(world, seed ^ 0xBEEF, (1, 2), (200, 350), (1800, 2600), 3500, 9000);
    Setup::new(world, 12000)
}

fn slow_bleed(seed: u64, model: Model) -> Setup {
    let world = random_hits(armlet_hero(seed, model).at_health(900.0), seed, (1, 1), (45, 60), (900, 1100), 2000, 22000);
    Setup::new(world, 24000)
}

fn restarted_on(seed: u64, model: Model) -> Setup {
    let world = World::new(seed, model, 1300, ARMLET, ARMLET_BONUS).starting_on();
    let world = random_hits(world, seed, (1, 3), (40, 110), (700, 1300), 2000, 12000);
    Setup::new(world, 14000)
}

fn restarted_off(seed: u64, model: Model) -> Setup {
    let world = World::new(seed, model, 1300, ARMLET, ARMLET_BONUS);
    let world = random_hits(world, seed, (1, 3), (40, 110), (700, 1300), 2000, 12000);
    Setup::new(world, 14000)
}

fn fights(results: &mut Vec<(String, bool)>, seeds: u64) {
    let scenarios: [(&str, Builder, bool); 5] = [
        ("lane skirmish", lane_skirmish, true),
        ("teamfight (Sabatons, bursts)", teamfight, true),
        ("slow bleed with drain", slow_bleed, true),
        ("app restarted, toggle on", restarted_on, false),
        ("app restarted, toggle off", restarted_off, false),
    ];

    for (name, build, state_known) in scenarios {
        for model in MODELS {
            header(&format!(
                "{name} — drain {} HP/s, {}% of presses lost, {} seeds",
                model.drain,
                model.press_loss * 100.0,
                seeds
            ));
            let mut new = Totals::default();
            let mut legacy = Totals::default();
            for seed in 1..=seeds {
                new.add(&run(build(seed, model), Driver::New, false));
                legacy.add(&run(build(seed, model), Driver::Legacy, false));
            }
            new.line("new");
            legacy.line("legacy");
            check(results, &format!("{name} / {model:?}: no mid-ramp press"), new.mid_ramp == 0);
            if state_known {
                check(results, &format!("{name} / {model:?}: no trick from OFF"), new.from_off == 0);
            } else {
                check(
                    results,
                    &format!("{name} / {model:?}: at most one probing trick from OFF per run"),
                    new.max_from_off <= 1,
                );
            }
            check(
                results,
                &format!("{name} / {model:?}: dies no more often than the old loop"),
                new.runs_with_death <= legacy.runs_with_death,
            );
            check(
                results,
                &format!("{name} / {model:?}: fewer presses than the old loop"),
                new.presses <= legacy.presses,
            );
        }
    }
}

// ─────────────────────────── capture 1, replayed ───────────────────────────

const CAPTURE: &str = "logs/sabatons_toggle/probe_2026-11-18_21-33-50.jsonl";

fn observation_from_payload(ts_ms: u64, payload: &Value, previous_names: &mut Option<Vec<String>>) -> Option<Observation> {
    let hero = payload.get("hero")?;
    let name = hero.get("name")?.as_str()?;
    if name.is_empty() || name == "empty" {
        return None;
    }
    let items = payload.get("items");
    let names: Vec<String> = (0..6)
        .map(|index| format!("slot{index}"))
        .chain(std::iter::once(String::from("neutral0")))
        .map(|slot| {
            items
                .and_then(|items| items.get(&slot))
                .and_then(|item| item.get("name"))
                .and_then(|name| name.as_str())
                .unwrap_or("empty")
                .to_string()
        })
        .collect();
    let item = TOGGLE_ITEMS
        .iter()
        .position(|wanted| names.iter().any(|name| name == wanted));
    let mut sorted = names;
    sorted.sort();
    let inventory_changed = previous_names.as_ref().is_some_and(|previous| *previous != sorted);
    *previous_names = Some(sorted);

    let number = |field: &str| hero.get(field).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let flag = |field: &str| hero.get(field).and_then(|v| v.as_bool()).unwrap_or(false);
    Some(Observation {
        at_ms: ts_ms,
        alive: flag("alive"),
        health: number("health"),
        max_health: number("max_health"),
        level: number("level"),
        item,
        inventory_changed,
        disabled: flag("stunned") || flag("hexed") || flag("muted"),
    })
}

fn replay_capture(path: &str, results: &mut Vec<(String, bool)>) {
    header(&format!("capture 1 replayed through the tracker ({path})"));
    let Ok(file) = std::fs::File::open(path) else {
        println!("     capture not found — skipped (run from the repo root after recording it)");
        return;
    };

    let mut controller = ToggleController::new();
    let mut previous_names = None;
    let mut base: Option<u64> = None;
    let mut timeline: Vec<(f64, ToggleState, Option<u32>)> = Vec::new();
    let mut self_inflicted: Vec<(f64, u32)> = Vec::new();

    let stream = serde_json::Deserializer::from_reader(BufReader::new(file)).into_iter::<Value>();
    for value in stream.flatten() {
        let Some(ts) = value.get("probe_ts_ms").and_then(|ts| ts.as_u64()) else {
            continue;
        };
        let payload = value.get("payload");
        let observation = payload.and_then(|payload| observation_from_payload(ts, payload, &mut previous_names));
        // The probe's clock starts at the first key or hero payload.
        if base.is_none() && (value.get("key").is_some() || observation.is_some()) {
            base = Some(ts);
        }
        let Some(obs) = observation else {
            continue;
        };
        let at = (ts - base.unwrap_or(ts)) as f64 / 1000.0;
        let tracked = controller.track(&obs, 600);
        if let Some(outcome) = tracked.outcome {
            println!("     [{at:>6.2}s] {outcome:?}  (max {}, hp {})", obs.max_health, obs.health);
        }
        if tracked.self_inflicted > 0 {
            self_inflicted.push((at, tracked.self_inflicted));
        }
        timeline.push((at, controller.state(), controller.bonus()));
    }

    let state_at = |at: f64| timeline.iter().rev().find(|(t, ..)| *t <= at).map(|(_, state, bonus)| (*state, *bonus));
    // What the probe measured at each point (see its report for capture 1).
    let expected: [(f64, ToggleState, &str); 14] = [
        (21.0, ToggleState::On, "Armlet on, ramp done"),
        (36.0, ToggleState::Off, "dead"),
        (40.0, ToggleState::Off, "respawned"),
        (42.6, ToggleState::On, "pressed on again"),
        (44.0, ToggleState::Off, "pressed off"),
        (47.5, ToggleState::On, "on, from a press the probe did not see"),
        (49.0, ToggleState::Off, "Sabatons in, Armlet gone"),
        (50.8, ToggleState::Off, "switched off mid-ramp"),
        (53.0, ToggleState::Off, "switched off as the ramp topped out"),
        (56.6, ToggleState::Off, "double tap from OFF: nothing moved"),
        (58.6, ToggleState::On, "on"),
        (60.1, ToggleState::On, "double tap from ON: the trick"),
        (61.5, ToggleState::On, "the trick again"),
        (65.0, ToggleState::Off, "died at 1 HP mid-trick"),
    ];
    for (at, state, what) in expected {
        let seen = state_at(at);
        check(
            results,
            &format!("capture {at:>5.1}s {what}: reads {:?}", state),
            seen.is_some_and(|(read, _)| read == state),
        );
    }
    check(
        results,
        "capture: learns Armlet's bonus as 500",
        state_at(30.0).is_some_and(|(_, bonus)| bonus == Some(ARMLET_BONUS)),
    );
    check(
        results,
        "capture: learns Sabatons' bonus as 600",
        state_at(58.6).is_some_and(|(_, bonus)| bonus == Some(SABATONS_BONUS)),
    );
    check(
        results,
        "capture: the 42.9s switch-off is 500 HP of self-inflicted cost, not damage",
        self_inflicted.iter().any(|(at, hp)| (*at - 42.89).abs() < 0.1 && *hp == 500),
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Usage: cargo run --release --example armlet_toggle_control -- [--verbose] [--seeds N] [--capture PATH]");
        return;
    }
    let verbose = args.iter().any(|arg| arg == "--verbose" || arg == "-v");
    let value_of = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|index| args.get(index + 1))
            .cloned()
    };
    let seeds: u64 = value_of("--seeds").and_then(|v| v.parse().ok()).unwrap_or(150);
    let capture = value_of("--capture").unwrap_or_else(|| String::from(CAPTURE));

    println!("Armlet toggle controller — simulated against capture 1, compared with the loop it replaces");
    println!(
        "tick {}–{}ms (5% at {}ms), press lands in {}–{}ms, ramp {}×{}ms, Armlet +{}, Sabatons +{}",
        TICK_MS.0, TICK_MS.1, SLOW_TICK_MS, PRESS_LATENCY_MS.0, PRESS_LATENCY_MS.1, RAMP_STEPS, RAMP_STEP_MS, ARMLET_BONUS, SABATONS_BONUS
    );

    let mut results: Vec<(String, bool)> = Vec::new();
    scripted(&mut results, verbose);
    fights(&mut results, seeds);
    replay_capture(&capture, &mut results);

    let failed: Vec<&(String, bool)> = results.iter().filter(|(_, passed)| !passed).collect();
    println!("\n═══ {} checks, {} failed ═══", results.len(), failed.len());
    for (label, _) in &failed {
        println!("  FAIL {label}");
    }
    if !failed.is_empty() {
        std::process::exit(1);
    }
}
