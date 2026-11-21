//! Armlet of Mordiggian and Sabatons of Mordiggian: the toggle trick, driven by
//! what `max_health` says the toggle is doing.
//!
//! ## The trick
//!
//! Both items drain HP for Strength. Switching one off takes the HP it granted
//! with it, but cannot kill: HP floors at 1. Switching it back on ramps the
//! Strength in over about 0.6s and adds that HP flat. So at low HP, off and
//! straight back on in one chord trades a moment at 1 HP for up to the whole
//! bonus.
//!
//! ## Why it reads `max_health`
//!
//! GSI has no field for the toggle: in capture 1 of
//! `examples/sabatons_toggle_probe.rs` the item JSON is identical on and off.
//! The loop this replaced never asked. It chorded whenever HP was under the
//! trigger and 300ms had passed, and the app's logs from September to November
//! (215 episodes, 973 presses) show what that cost:
//!
//! | What the next press found | Presses |
//! |---|---|
//! | the Strength still ramping in: the chord threw it away and parked the hero at 1 HP again | 354 |
//! | the toggle off: the chord switched it on and straight back off | 319 |
//! | a chord that only half landed | 28 |
//!
//! `max_health` does say, though. The Strength coming in raises it over several
//! ticks, and switching off drops it by the bonus in one. [`ToggleController`]
//! reads that, and:
//!
//! 1. never presses while the Strength is still coming in;
//! 2. chords only from ON, and presses once to turn it on from OFF — the whole
//!    bonus, with no 1 HP floor at all;
//! 3. checks every press against what `max_health` did next: a chord that
//!    switched it off but never back on is finished at once, and a press that
//!    moved nothing backs off instead of pressing into nothing. A trick that
//!    moves nothing while on never turns into a single press: that would switch
//!    it off, and GSI does not report the taunts and cyclones that stop items;
//! 4. treats a toggle it did not press as the player's, and holds off long
//!    enough not to cut a trick done by hand in half;
//! 5. presses only while Dota has the keyboard and the hero can use items.
//!
//! The HP a switch-off takes is declared to the danger detector, which used to
//! read every trick as a 500 HP hit.
//!
//! | Measured (capture 1) | Armlet | Sabatons |
//! |---|---|---|
//! | `max_health` per toggle | +500 | +600 |
//! | press → first move | 135–287ms | 89–208ms |
//! | press → settled | 730–794ms | 840–1000ms |
//!
//! **None of those numbers appear below.** The size of a toggle is learned from
//! the moves the controller sees, and the two configured times are in
//! milliseconds. The loop was proved against a simulator wired with them before
//! any of it ran in a game — `examples/armlet_toggle_control.rs`, which also
//! replays capture 1 through the tracker and runs the old loop beside it.

use crate::config::settings::{ArmletRoshanConfig, EffectiveArmletConfig};
use crate::config::Settings;
use crate::input::binding::{KeyBinding, PressableKey};
use crate::input::simulation::{armlet_chord, armlet_press, ModifierKey};
use crate::models::gsi_event::Item;
use crate::models::GsiWebhookEvent;
use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use tracing::{debug, info, trace, warn};

lazy_static! {
    static ref RUNTIME: Mutex<ArmletRuntime> = Mutex::new(ArmletRuntime::default());
    static ref ARMLET_ROSHAN_STATE: Mutex<ArmletRoshanState> =
        Mutex::new(ArmletRoshanState::default());
}

static ARMLET_ROSHAN_MODE_ARMED: AtomicBool = AtomicBool::new(false);

/// The tracker, and what it needs to remember from the previous event.
#[derive(Default)]
struct ArmletRuntime {
    controller: ToggleController,
    /// Inventory names last event, sorted: a purchase moves `max_health` too.
    inventory: Option<Vec<String>>,
}

/// Milliseconds on a monotonic clock, for the controller's timings.
fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

// ─────────────────────────── the toggle controller ───────────────────────────
// Proved in examples/armlet_toggle_control.rs before it moved here.

/// Which way the toggle is, as far as `max_health` has said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToggleState {
    /// Nothing seen yet says: the app started with the item already held.
    Unknown,
    Off,
    On,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Press {
    /// Off and straight back on, one chord. HP floors at 1 and the Strength
    /// ramps back in on top of it: a gain while HP is under the bonus, and only
    /// from ON — from OFF the chord switches it on and straight back off.
    Trick,
    /// One press, to turn it on from OFF: the whole bonus, no floor.
    TurnOn,
}

impl Press {
    fn label(self) -> &'static str {
        match self {
            Self::Trick => "trick",
            Self::TurnOn => "turn-on",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Observation {
    at_ms: u64,
    alive: bool,
    health: u32,
    max_health: u32,
    level: u32,
    /// Which toggle item is held, as its index in [`TOGGLE_DRAIN_ITEMS`]:
    /// Armlet sold for Sabatons reads as a new item with a new bonus.
    item: Option<usize>,
    /// Something besides the toggle went in or out of the inventory — a
    /// purchase, a sale, a neutral item — which moves `max_health` by itself.
    inventory_changed: bool,
    /// Stunned, hexed or muted: the item cannot be used, so a press is refused.
    disabled: bool,
}

/// What one event told the tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
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
struct Tracked {
    /// HP this event lost to the toggle switching off — ours or the player's.
    /// Not damage: the danger detector and Roshan's hit learning must not see it.
    self_inflicted: u32,
    outcome: Option<Outcome>,
}

/// What the rest of the app wants from this event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Want {
    /// HP is under the trigger, or Roshan mode says the next hit is lethal.
    trick: bool,
    /// The danger detector is up.
    in_danger: bool,
    /// Dota has the keyboard. A press anywhere else lands in another window.
    focused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Timing {
    /// Never press twice inside this.
    cooldown_ms: u64,
    /// A press that has not moved `max_health` by now did not land.
    confirm_ms: u64,
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

struct ToggleController {
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
}

impl ToggleController {
    fn state(&self) -> ToggleState {
        self.state
    }

    fn last_tick_loss(&self) -> u32 {
        self.last_tick_loss
    }

    /// True while a press of ours is out or the Strength is still coming in.
    fn busy(&self) -> bool {
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
    fn track(&mut self, obs: &Observation, confirm_ms: u64) -> Tracked {
        let mut tracked = Tracked::default();
        let previous = self.last.replace(*obs);
        self.last_tick_loss = 0;

        // Death takes the toggle off: capture 1 respawned at the base max_health.
        if !obs.alive {
            let cut_short = match self.phase {
                Phase::Pending { ours, .. } | Phase::Ramping { ours: Some(ours), .. } => {
                    Some(ours)
                }
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
            Phase::Pending { .. } => self.track_pending(
                obs,
                &previous,
                d_max,
                drop_cost,
                noisy,
                confirm_ms,
                &mut tracked,
            ),
            Phase::Ramping { .. } => {
                self.track_ramp(obs, d_max, drop_cost, noisy, confirm_ms, &mut tracked)
            }
            Phase::Settled => self.track_settled(
                obs,
                &previous,
                d_max,
                drop_cost,
                noisy,
                confirm_ms,
                &mut tracked,
            ),
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
        let dip_unanswered =
            dip.is_some_and(|(_, at)| obs.at_ms.saturating_sub(at) >= confirm_ms / 4);
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
    fn decide(
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
                        || self.last_damage_ms.is_some_and(|at| {
                            obs.at_ms.saturating_sub(at) <= timing.confirm_ms
                        }) =>
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

// ─────────────────────────── Roshan mode ───────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
struct RoshanHitSample {
    at_ms: u64,
    damage: u32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ArmletRoshanState {
    last_health: Option<u32>,
    latest_observed_damage: Option<u32>,
    samples: VecDeque<RoshanHitSample>,
    was_stunned_last_tick: bool,
    awaiting_post_stun_hit: bool,
    stun_recovery_estimate_damage: Option<u32>,
    stun_recovery_started_at_ms: Option<u64>,
}

impl ArmletRoshanState {
    #[cfg_attr(not(test), allow(dead_code))]
    fn armed() -> Self {
        Self::default()
    }

    fn reset_learning(&mut self) {
        self.last_health = None;
        self.latest_observed_damage = None;
        self.samples.clear();
        self.was_stunned_last_tick = false;
        self.awaiting_post_stun_hit = false;
        self.stun_recovery_estimate_damage = None;
        self.stun_recovery_started_at_ms = None;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoshanArmletTrigger {
    EmergencyFallback {
        observed_damage: u32,
        lethal_zone: u32,
    },
    LearnedHit {
        predicted_damage: u32,
        lethal_zone: u32,
        sample_count: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoshanRecoveryAction {
    None,
    ProtectNow,
    AwaitNextHit { predicted_damage: u32 },
    TriggerDeferredHit { observed_damage: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoshanResetReason {
    HeroDied,
    ArmletDisabled,
    ArmletMissing,
    RoshanModeDisarmed,
    ModeToggled,
}

impl RoshanResetReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::HeroDied => "hero died",
            Self::ArmletDisabled => "armlet automation disabled",
            Self::ArmletMissing => "armlet item missing",
            Self::RoshanModeDisarmed => "roshan mode inactive",
            Self::ModeToggled => "roshan mode toggled",
        }
    }
}

/// Why Roshan mode is not asking for a trick this event, for its diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoshanSkip {
    /// HP is still over the ordinary trigger.
    Safe,
    Stunned,
    /// Under the trigger, but a press of ours is out, the Strength is still
    /// coming in, or the hero cannot use items.
    Busy,
}

fn should_log_roshan_skip_context(
    health: u32,
    trigger_point: u32,
    observed_damage: Option<u32>,
    predicted_damage: Option<u32>,
    emergency_margin_hp: u32,
) -> bool {
    let lethal_zone = observed_damage
        .into_iter()
        .chain(predicted_damage)
        .map(|damage| damage.saturating_add(emergency_margin_hp))
        .max()
        .unwrap_or_default();
    let near_danger_floor = trigger_point.max(lethal_zone);

    health <= near_danger_floor.saturating_add(120)
}

pub fn is_roshan_mode_armed() -> bool {
    ARMLET_ROSHAN_MODE_ARMED.load(Ordering::SeqCst)
}

pub fn set_roshan_mode_armed(armed: bool) -> bool {
    let previous = ARMLET_ROSHAN_MODE_ARMED.swap(armed, Ordering::SeqCst);
    if previous != armed {
        if let Ok(mut state) = ARMLET_ROSHAN_STATE.lock() {
            clear_roshan_learning_state_with_reason(&mut state, RoshanResetReason::ModeToggled);
        }
        info!(
            "Armlet Roshan mode {} (reason: {})",
            if armed { "armed" } else { "disarmed" },
            RoshanResetReason::ModeToggled.as_str()
        );
    }

    armed
}

pub fn toggle_roshan_mode() -> bool {
    set_roshan_mode_armed(!is_roshan_mode_armed())
}

fn prune_stale_roshan_samples(
    state: &mut ArmletRoshanState,
    now_ms: u64,
    config: &ArmletRoshanConfig,
) {
    if let Some(last_sample) = state.samples.back() {
        if now_ms.saturating_sub(last_sample.at_ms) > config.stale_reset_ms {
            state.reset_learning();
            return;
        }
    }

    while let Some(sample) = state.samples.front() {
        if now_ms.saturating_sub(sample.at_ms) > config.learning_window_ms {
            state.samples.pop_front();
        } else {
            break;
        }
    }
}

/// Record this event's HP loss as a possible Roshan hit. `self_inflicted` is
/// what our own toggle took in the same event: a trick's switch-off is not a
/// Roshan hit, and learning it as one would set the lethal zone at the HP the
/// trick itself throws away.
fn record_roshan_health_sample(
    state: &mut ArmletRoshanState,
    now_ms: u64,
    health: u32,
    self_inflicted: u32,
    config: &ArmletRoshanConfig,
) -> Option<u32> {
    prune_stale_roshan_samples(state, now_ms, config);

    let observed_damage = state
        .last_health
        .and_then(|last_health| last_health.checked_sub(health))
        .map(|lost| lost.saturating_sub(self_inflicted))
        .filter(|damage| *damage >= config.min_sample_damage);

    state.last_health = Some(health);
    state.latest_observed_damage = observed_damage;

    if let Some(damage) = observed_damage {
        state.samples.push_back(RoshanHitSample {
            at_ms: now_ms,
            damage,
        });
    }

    observed_damage
}

fn evaluate_roshan_trigger(
    health: u32,
    now_ms: u64,
    state: &ArmletRoshanState,
    config: &ArmletRoshanConfig,
) -> Option<RoshanArmletTrigger> {
    if !config.enabled {
        return None;
    }

    let recent_samples: Vec<_> = state
        .samples
        .iter()
        .filter(|sample| now_ms.saturating_sub(sample.at_ms) <= config.learning_window_ms)
        .collect();

    if recent_samples.len() >= config.min_confidence_hits {
        let predicted_damage = recent_samples.iter().map(|sample| sample.damage).max()?;
        let lethal_zone = predicted_damage.saturating_add(config.emergency_margin_hp);
        if health <= lethal_zone {
            return Some(RoshanArmletTrigger::LearnedHit {
                predicted_damage,
                lethal_zone,
                sample_count: recent_samples.len(),
            });
        }
    }

    let observed_damage = state.latest_observed_damage?;
    let lethal_zone = observed_damage.saturating_add(config.emergency_margin_hp);
    if health <= lethal_zone {
        Some(RoshanArmletTrigger::EmergencyFallback {
            observed_damage,
            lethal_zone,
        })
    } else {
        None
    }
}

fn best_roshan_hit_estimate(
    state: &ArmletRoshanState,
    now_ms: u64,
    config: &ArmletRoshanConfig,
) -> Option<u32> {
    let recent_max = state
        .samples
        .iter()
        .filter(|sample| now_ms.saturating_sub(sample.at_ms) <= config.learning_window_ms)
        .map(|sample| sample.damage)
        .max();

    recent_max.or(state.latest_observed_damage)
}

fn clear_roshan_recovery_defer(state: &mut ArmletRoshanState) {
    state.awaiting_post_stun_hit = false;
    state.stun_recovery_estimate_damage = None;
    state.stun_recovery_started_at_ms = None;
}

fn evaluate_roshan_stun_recovery(
    state: &mut ArmletRoshanState,
    health: u32,
    is_stunned: bool,
    just_recovered_from_stun: bool,
    now_ms: u64,
    config: &ArmletRoshanConfig,
) -> RoshanRecoveryAction {
    if is_stunned {
        clear_roshan_recovery_defer(state);
        state.was_stunned_last_tick = true;
        return RoshanRecoveryAction::None;
    }

    if state.awaiting_post_stun_hit {
        if let Some(observed_damage) = state.latest_observed_damage {
            if observed_damage >= config.min_sample_damage {
                clear_roshan_recovery_defer(state);
                state.was_stunned_last_tick = false;
                return RoshanRecoveryAction::TriggerDeferredHit { observed_damage };
            }
        }

        state.was_stunned_last_tick = false;
        return RoshanRecoveryAction::None;
    }

    if !just_recovered_from_stun {
        state.was_stunned_last_tick = false;
        return RoshanRecoveryAction::None;
    }

    let Some(predicted_damage) = best_roshan_hit_estimate(state, now_ms, config) else {
        state.was_stunned_last_tick = false;
        return RoshanRecoveryAction::None;
    };

    state.was_stunned_last_tick = false;
    if health <= predicted_damage.saturating_add(config.emergency_margin_hp) {
        RoshanRecoveryAction::ProtectNow
    } else {
        state.awaiting_post_stun_hit = true;
        state.stun_recovery_estimate_damage = Some(predicted_damage);
        state.stun_recovery_started_at_ms = Some(now_ms);
        RoshanRecoveryAction::AwaitNextHit { predicted_damage }
    }
}

fn clear_roshan_learning_state(state: &mut ArmletRoshanState) {
    state.reset_learning();
}

fn clear_roshan_learning_state_with_reason(
    state: &mut ArmletRoshanState,
    reason: RoshanResetReason,
) {
    if *state != ArmletRoshanState::default() {
        info!(
            "Clearing Armlet Roshan learning state ({})",
            reason.as_str()
        );
    }
    clear_roshan_learning_state(state);
}

fn clear_roshan_learning(reason: RoshanResetReason) {
    if let Ok(mut roshan_state) = ARMLET_ROSHAN_STATE.lock() {
        clear_roshan_learning_state_with_reason(&mut roshan_state, reason);
    }
}

fn roshan_prediction_summary(
    state: &ArmletRoshanState,
    now_ms: u64,
    config: &ArmletRoshanConfig,
) -> (Option<u32>, usize) {
    let mut predicted_damage = None;
    let mut recent_sample_count = 0;

    for sample in state
        .samples
        .iter()
        .filter(|sample| now_ms.saturating_sub(sample.at_ms) <= config.learning_window_ms)
    {
        recent_sample_count += 1;
        predicted_damage = Some(predicted_damage.map_or(sample.damage, |current_max: u32| {
            current_max.max(sample.damage)
        }));
    }

    (predicted_damage, recent_sample_count)
}

#[allow(clippy::too_many_arguments)]
fn maybe_log_roshan_skip_context(
    skip: RoshanSkip,
    health: u32,
    threshold: u32,
    predictive_offset: u32,
    trigger_point: u32,
    is_stunned: bool,
    state: &ArmletRoshanState,
    recovery_action: RoshanRecoveryAction,
    now_ms: u64,
    config: &ArmletRoshanConfig,
) {
    let observed_damage = state.latest_observed_damage;
    let (predicted_damage, recent_sample_count) = roshan_prediction_summary(state, now_ms, config);
    let predicted_context_damage = state.stun_recovery_estimate_damage.or(predicted_damage);

    if !should_log_roshan_skip_context(
        health,
        trigger_point,
        observed_damage,
        predicted_context_damage,
        config.emergency_margin_hp,
    ) {
        return;
    }

    debug!(
        "Armlet Roshan no-toggle context: hp={}, threshold={}, offset={}, trigger={}, observed_hit={:?}, predicted_hit={:?}, samples={}, awaiting_post_stun_hit={}, stunned={}",
        health,
        threshold,
        predictive_offset,
        trigger_point,
        observed_damage,
        predicted_context_damage,
        recent_sample_count,
        state.awaiting_post_stun_hit,
        is_stunned
    );

    match skip {
        RoshanSkip::Busy => {
            debug!("Skipping Armlet Roshan protection near danger: a toggle is already in flight");
        }
        RoshanSkip::Stunned => {
            debug!("Skipping Armlet Roshan protection near danger: still stunned");
        }
        RoshanSkip::Safe => match recovery_action {
            RoshanRecoveryAction::AwaitNextHit { predicted_damage } => {
                debug!(
                    "Skipping Armlet Roshan protection near danger: waiting for deferred post-stun hit re-sync (predicted hit: {})",
                    predicted_damage
                );
            }
            RoshanRecoveryAction::None if state.awaiting_post_stun_hit => {
                debug!(
                    "Skipping Armlet Roshan protection near danger: deferred post-stun wait still active"
                );
            }
            RoshanRecoveryAction::None => {
                if recent_sample_count < config.min_confidence_hits && observed_damage.is_none() {
                    debug!(
                        "Skipping Armlet Roshan protection near danger: insufficient sample confidence (samples: {}/{}, predicted hit: {:?})",
                        recent_sample_count,
                        config.min_confidence_hits,
                        predicted_damage
                    );
                } else if recent_sample_count >= config.min_confidence_hits {
                    if let Some(predicted_damage) = predicted_damage {
                        debug!(
                            "Skipping Armlet Roshan protection near danger: HP {} still above learned lethal zone {} (predicted hit: {}, samples: {})",
                            health,
                            predicted_damage.saturating_add(config.emergency_margin_hp),
                            predicted_damage,
                            recent_sample_count
                        );
                    }
                } else if let Some(observed_damage) = observed_damage {
                    debug!(
                        "Skipping Armlet Roshan protection near danger: HP {} still above emergency fallback zone {} (observed hit: {})",
                        health,
                        observed_damage.saturating_add(config.emergency_margin_hp),
                        observed_damage
                    );
                } else {
                    debug!(
                        "Skipping Armlet Roshan protection near danger: insufficient sample confidence (samples: {}/{})",
                        recent_sample_count, config.min_confidence_hits
                    );
                }
            }
            _ => {}
        },
    }
}

/// Roshan mode's say: whether the next hit looks lethal while HP is still over
/// the ordinary trigger. Keeps its hit learning fed either way.
fn roshan_wants_trick(
    event: &GsiWebhookEvent,
    resolved: &EffectiveArmletConfig,
    trigger_point: u32,
    below_trigger: bool,
    cannot_press: bool,
    self_inflicted: u32,
) -> bool {
    let Ok(mut roshan_state) = ARMLET_ROSHAN_STATE.lock() else {
        return false;
    };
    if !(resolved.roshan.enabled && is_roshan_mode_armed()) {
        clear_roshan_learning_state_with_reason(
            &mut roshan_state,
            RoshanResetReason::RoshanModeDisarmed,
        );
        return false;
    }

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default();
    let health = event.hero.health;
    let threshold = resolved.toggle_threshold;
    let offset = resolved.predictive_offset;
    let stunned = event.hero.is_stunned();
    let was_stunned_last_tick = roshan_state.was_stunned_last_tick;
    record_roshan_health_sample(
        &mut roshan_state,
        now_ms,
        health,
        self_inflicted,
        &resolved.roshan,
    );

    if below_trigger && !cannot_press {
        // The ordinary trigger has this one; keep the stun bookkeeping current.
        roshan_state.was_stunned_last_tick = stunned;
        if stunned {
            clear_roshan_recovery_defer(&mut roshan_state);
        }
        return false;
    }

    let recovery = evaluate_roshan_stun_recovery(
        &mut roshan_state,
        health,
        stunned,
        was_stunned_last_tick && !stunned,
        now_ms,
        &resolved.roshan,
    );
    let (predicted_damage, recent_sample_count) =
        roshan_prediction_summary(&roshan_state, now_ms, &resolved.roshan);

    if below_trigger {
        let skip = if stunned {
            RoshanSkip::Stunned
        } else {
            RoshanSkip::Busy
        };
        maybe_log_roshan_skip_context(
            skip,
            health,
            threshold,
            offset,
            trigger_point,
            stunned,
            &roshan_state,
            recovery,
            now_ms,
            &resolved.roshan,
        );
        trace!(
            "Roshan recovery state updated without a toggle decision: {:?}",
            recovery
        );
        return false;
    }

    match recovery {
        RoshanRecoveryAction::ProtectNow => {
            info!(
                "Roshan protection after stun recovery (HP: {}, trigger: {}, predicted hit: {:?}, samples: {})",
                health, trigger_point, predicted_damage, recent_sample_count
            );
            true
        }
        RoshanRecoveryAction::AwaitNextHit { predicted_damage } => {
            maybe_log_roshan_skip_context(
                RoshanSkip::Safe,
                health,
                threshold,
                offset,
                trigger_point,
                stunned,
                &roshan_state,
                recovery,
                now_ms,
                &resolved.roshan,
            );
            debug!(
                "Deferring Roshan toggle after stun recovery; waiting for next hit (HP: {}, trigger: {}, predicted damage: {}, samples: {})",
                health, trigger_point, predicted_damage, recent_sample_count
            );
            false
        }
        RoshanRecoveryAction::TriggerDeferredHit { observed_damage } => {
            info!(
                "Roshan re-sync after the post-stun hit (HP: {}, trigger: {}, observed damage: {}, predicted hit: {:?}, samples: {})",
                health, trigger_point, observed_damage, predicted_damage, recent_sample_count
            );
            true
        }
        RoshanRecoveryAction::None => {
            match evaluate_roshan_trigger(health, now_ms, &roshan_state, &resolved.roshan) {
                Some(RoshanArmletTrigger::EmergencyFallback {
                    observed_damage,
                    lethal_zone,
                }) => {
                    info!(
                        "Roshan emergency fallback (HP: {} <= lethal zone: {}, trigger: {}, observed hit: {}, samples: {})",
                        health, lethal_zone, trigger_point, observed_damage, recent_sample_count
                    );
                    true
                }
                Some(RoshanArmletTrigger::LearnedHit {
                    predicted_damage,
                    lethal_zone,
                    sample_count,
                }) => {
                    info!(
                        "Roshan learned-hit protection (HP: {} <= lethal zone: {}, trigger: {}, predicted hit: {}, samples: {})",
                        health, lethal_zone, trigger_point, predicted_damage, sample_count
                    );
                    true
                }
                None => {
                    maybe_log_roshan_skip_context(
                        RoshanSkip::Safe,
                        health,
                        threshold,
                        offset,
                        trigger_point,
                        stunned,
                        &roshan_state,
                        recovery,
                        now_ms,
                        &resolved.roshan,
                    );
                    false
                }
            }
        }
    }
}

// ─────────────────────────── pressing ───────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArmletTriggerStep {
    QuickCast(PressableKey),
    ModifierDown(ModifierKey),
    ModifierUp(ModifierKey),
}

fn parse_cast_modifier(raw: &str) -> Option<ModifierKey> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "alt" => Some(ModifierKey::Alt),
        "ctrl" | "control" => Some(ModifierKey::Control),
        "shift" => Some(ModifierKey::Shift),
        _ => None,
    }
}

fn resolve_cast_modifier(config: &EffectiveArmletConfig) -> ModifierKey {
    parse_cast_modifier(&config.cast_modifier).unwrap_or_else(|| {
        warn!(
            "Unknown armlet cast modifier {:?}; defaulting to Alt",
            config.cast_modifier
        );
        ModifierKey::Alt
    })
}

fn plan_dual_trigger_sequence(
    slot_key: PressableKey,
    cast_modifier: ModifierKey,
) -> [ArmletTriggerStep; 4] {
    [
        ArmletTriggerStep::QuickCast(slot_key),
        ArmletTriggerStep::ModifierDown(cast_modifier),
        ArmletTriggerStep::QuickCast(slot_key),
        ArmletTriggerStep::ModifierUp(cast_modifier),
    ]
}

fn execute_dual_trigger(slot_key: &KeyBinding, cast_modifier: ModifierKey) {
    let started = Instant::now();
    if let Some(target) = slot_key.pressable() {
        debug!(
            "Armlet dual-trigger starting for '{}' with {:?}: {:?}",
            slot_key,
            cast_modifier,
            plan_dual_trigger_sequence(target, cast_modifier)
        );
    }

    armlet_chord(slot_key, cast_modifier);

    debug!(
        "Armlet dual-trigger finished in {}ms via dedicated worker chord",
        started.elapsed().as_millis()
    );
}

/// HP-draining Strength toggles this automation drives, in order of preference.
///
/// Sabatons of Mordiggian (7.42, built from Armlet) measured the same as Armlet in
/// every way the decision relies on: a toggle shows only as a `max_health` move, GSI
/// reports no on/off state, and switching off at low HP leaves 1 HP instead of
/// killing. See capture 1 of `examples/sabatons_toggle_probe.rs`. Armlet wins if a
/// hero somehow carries both.
const TOGGLE_DRAIN_ITEMS: &[&str] = &["item_armlet", "item_berserkers_sabatons"];

/// The toggle held, as its index in [`TOGGLE_DRAIN_ITEMS`] and its slot.
fn find_toggle_drain_slot<'a>(slots: &[(&'a str, &Item)]) -> Option<(usize, &'a str)> {
    TOGGLE_DRAIN_ITEMS.iter().enumerate().find_map(|(kind, wanted)| {
        slots
            .iter()
            .find(|(_, item)| item.name == *wanted)
            .map(|(slot, _)| (kind, *slot))
    })
}

fn inventory_names(slots: &[(&str, &Item)]) -> Vec<String> {
    let mut names: Vec<String> = slots.iter().map(|(_, item)| item.name.clone()).collect();
    names.sort();
    names
}

fn log_outcome(outcome: Outcome, obs: &Observation) {
    match outcome {
        Outcome::Landed {
            press,
            hp_before,
            hp_low,
            hp_after,
            took_ms,
        } => info!(
            "Armlet {} landed: HP {} → {} → {} in {}ms",
            press.label(),
            hp_before,
            hp_low,
            hp_after,
            took_ms
        ),
        Outcome::SecondPressLost { hp } => warn!(
            "Armlet trick switched it off but the press back on never showed (HP {}) — turning it back on",
            hp
        ),
        Outcome::WasOn { hp } => warn!(
            "Armlet was on after all: the press switched it off (HP {}) — turning it back on",
            hp
        ),
        Outcome::NoEffect { press: Press::Trick } => info!(
            "Armlet trick moved nothing: the toggle was off, or the presses did not land (HP {})",
            obs.health
        ),
        Outcome::NoEffect {
            press: Press::TurnOn,
        } => warn!(
            "Armlet press moved nothing (HP {}) — Dota is not taking it; backing off",
            obs.health
        ),
        Outcome::PlayerToggled { on } => debug!(
            "Armlet switched {} by hand (max HP {}, HP {})",
            if on { "on" } else { "off" },
            obs.max_health,
            obs.health
        ),
        Outcome::Died {
            press,
            hp_before,
            hp_low,
            after_ms,
        } => info!(
            "Armlet {} cut short: died {}ms after the press (HP {} → {})",
            press.label(),
            after_ms,
            hp_before,
            hp_low
        ),
    }
}

pub fn maybe_toggle(event: &GsiWebhookEvent, settings: &Settings) {
    let resolved = settings.resolve_armlet_config(&event.hero.name);
    let timing = Timing {
        cooldown_ms: resolved.toggle_cooldown_ms,
        confirm_ms: resolved.press_confirm_ms,
    };
    let slots = event.items.all_slots();
    let held = find_toggle_drain_slot(&slots);

    let mut runtime = RUNTIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let names = inventory_names(&slots);
    let inventory_changed = runtime
        .inventory
        .as_ref()
        .is_some_and(|previous| *previous != names);
    runtime.inventory = Some(names);

    let hero = &event.hero;
    let obs = Observation {
        at_ms: now_ms(),
        alive: hero.is_alive(),
        health: hero.health,
        max_health: hero.max_health,
        level: hero.level,
        item: held.map(|(kind, _)| kind),
        inventory_changed,
        disabled: hero.stunned || hero.hexed || hero.muted,
    };
    let tracked = runtime.controller.track(&obs, timing.confirm_ms);

    // What the toggle took is ours, not an enemy's. A manual toggle reads the
    // same, so this runs with the automation off too.
    if tracked.self_inflicted > 0 {
        crate::actions::danger_detector::note_self_damage(tracked.self_inflicted, "Armlet toggle");
    }
    if let Some(outcome) = tracked.outcome {
        log_outcome(outcome, &obs);
    }

    if !hero.is_alive() {
        clear_roshan_learning(RoshanResetReason::HeroDied);
        return;
    }
    if !resolved.enabled {
        clear_roshan_learning(RoshanResetReason::ArmletDisabled);
        return;
    }
    let Some((_, slot)) = held else {
        clear_roshan_learning(RoshanResetReason::ArmletMissing);
        return;
    };
    let Some(slot_key) = settings.get_key_for_slot(slot) else {
        return;
    };

    let trigger_point = resolved
        .toggle_threshold
        .saturating_add(resolved.predictive_offset);
    let below_trigger = hero.health < trigger_point;
    let roshan_wants = roshan_wants_trick(
        event,
        &resolved,
        trigger_point,
        below_trigger,
        obs.disabled || runtime.controller.busy(),
        tracked.self_inflicted,
    );
    let focused = !crate::observability::minimap_capture_backend::is_dota2_backgrounded();
    if below_trigger && !focused {
        debug!("Armlet holding off: another window has the keyboard (HP {})", hero.health);
    }

    let want = Want {
        trick: below_trigger || roshan_wants,
        in_danger: crate::actions::danger_detector::is_in_danger(),
        focused,
    };
    let decision = runtime.controller.decide(&obs, want, timing);
    let believed = runtime.controller.state();
    let last_tick_loss = runtime.controller.last_tick_loss();
    drop(runtime);

    let Some((press, why)) = decision else {
        return;
    };
    info!(
        "Armlet {}: {} (HP {}/{}, lost {} last tick, trigger {}{}, toggle read as {:?})",
        press.label(),
        why,
        hero.health,
        hero.max_health,
        last_tick_loss,
        trigger_point,
        if roshan_wants && !below_trigger {
            ", Roshan"
        } else {
            ""
        },
        believed
    );
    match press {
        Press::Trick => execute_dual_trigger(&slot_key, resolve_cast_modifier(&resolved)),
        Press::TurnOn => armlet_press(&slot_key),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        clear_roshan_learning_state, evaluate_roshan_stun_recovery, evaluate_roshan_trigger,
        find_toggle_drain_slot, parse_cast_modifier, plan_dual_trigger_sequence,
        record_roshan_health_sample, resolve_cast_modifier, should_log_roshan_skip_context,
        ArmletRoshanConfig, ArmletRoshanState, ArmletTriggerStep, Observation, Outcome, Press,
        RoshanArmletTrigger, RoshanRecoveryAction, RoshanResetReason, Timing, ToggleController,
        ToggleState, Want,
    };
    use crate::config::{
        settings::{ArmletAutomationConfig, EffectiveArmletConfig, HeroArmletOverrideConfig},
        Settings,
    };
    use crate::input::simulation::ModifierKey;

    const TIMING: Timing = Timing {
        cooldown_ms: 300,
        confirm_ms: 600,
    };
    const ARMLET: usize = 0;
    const SABATONS: usize = 1;

    /// Feeds a controller events a tick apart, the way GSI does.
    struct Feed {
        controller: ToggleController,
        at_ms: u64,
        obs: Observation,
    }

    impl Feed {
        /// Armlet held, alive, at `health` of `max_health`, already seen once.
        fn new(health: u32, max_health: u32) -> Self {
            let obs = Observation {
                at_ms: 0,
                alive: true,
                health,
                max_health,
                level: 12,
                item: Some(ARMLET),
                inventory_changed: false,
                disabled: false,
            };
            let mut feed = Self {
                controller: ToggleController::default(),
                at_ms: 0,
                obs,
            };
            feed.controller.track(&obs, TIMING.confirm_ms);
            feed
        }

        /// A controller that has watched the player switch Armlet on: state on,
        /// bonus learned as `bonus`.
        fn on(health: u32, base_max: u32, bonus: u32) -> Self {
            let mut feed = Self::new(health, base_max);
            let steps = [bonus / 3, bonus / 3, bonus - 2 * (bonus / 3)];
            let mut granted = 0;
            for step in steps {
                granted += step;
                feed.tick(health + granted, base_max + granted);
            }
            feed.tick(health + bonus, base_max + bonus);
            assert_eq!(feed.controller.state(), ToggleState::On);
            assert_eq!(feed.controller.bonus, Some(bonus));
            feed
        }

        fn tick(&mut self, health: u32, max_health: u32) -> super::Tracked {
            self.at_ms += 225;
            self.obs.at_ms = self.at_ms;
            self.obs.health = health;
            self.obs.max_health = max_health;
            self.obs.inventory_changed = false;
            self.controller.track(&self.obs, TIMING.confirm_ms)
        }

        fn decide(&mut self, want: Want) -> Option<Press> {
            self.controller
                .decide(&self.obs, want, TIMING)
                .map(|(press, _)| press)
        }
    }

    fn low() -> Want {
        Want {
            trick: true,
            in_danger: true,
            focused: true,
        }
    }

    fn calm() -> Want {
        Want {
            trick: true,
            in_danger: false,
            focused: true,
        }
    }

    #[test]
    fn a_rise_over_several_ticks_reads_as_switched_on_and_teaches_the_bonus() {
        let feed = Feed::on(800, 1300, 500);
        assert_eq!(feed.controller.state(), ToggleState::On);
        assert_eq!(feed.controller.bonus, Some(500));
    }

    #[test]
    fn a_one_tick_rise_is_not_the_toggle() {
        let mut feed = Feed::new(800, 1300);
        feed.tick(880, 1380);
        feed.tick(880, 1380);
        assert_eq!(feed.controller.state(), ToggleState::Unknown);
        assert_eq!(feed.controller.bonus, None);
    }

    #[test]
    fn a_level_up_is_not_the_toggle() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.obs.level += 1;
        feed.tick(1360, 1860);
        feed.tick(1360, 1860);
        assert_eq!(feed.controller.state(), ToggleState::On);
    }

    #[test]
    fn a_purchase_is_not_the_toggle() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.at_ms += 225;
        feed.obs.at_ms = feed.at_ms;
        feed.obs.health = 1550;
        feed.obs.max_health = 2050;
        feed.obs.inventory_changed = true;
        feed.controller.track(&feed.obs, TIMING.confirm_ms);
        feed.tick(1550, 2050);
        assert_eq!(feed.controller.state(), ToggleState::On);
    }

    #[test]
    fn a_bonus_sized_drop_reads_as_switched_off_and_is_not_damage() {
        let mut feed = Feed::on(800, 1300, 500);
        let tracked = feed.tick(800, 1300);
        assert_eq!(feed.controller.state(), ToggleState::Off);
        assert_eq!(tracked.self_inflicted, 500);
        assert_eq!(tracked.outcome, Some(Outcome::PlayerToggled { on: false }));
    }

    #[test]
    fn a_small_drop_is_a_debuff_not_the_toggle() {
        let mut feed = Feed::on(800, 1300, 500);
        let tracked = feed.tick(1220, 1720);
        assert_eq!(feed.controller.state(), ToggleState::On);
        assert_eq!(tracked.self_inflicted, 0);
    }

    #[test]
    fn a_drop_that_floors_hp_is_the_toggle_even_before_its_size_is_known() {
        let mut feed = Feed::new(250, 1800);
        let tracked = feed.tick(1, 1300);
        assert_eq!(feed.controller.state(), ToggleState::Off);
        assert_eq!(tracked.self_inflicted, 249);
    }

    #[test]
    fn first_sight_is_unknown_and_death_respawn_and_a_fresh_item_are_off() {
        let feed = Feed::new(800, 1300);
        assert_eq!(feed.controller.state(), ToggleState::Unknown);

        let mut feed = Feed::on(800, 1300, 500);
        feed.obs.alive = false;
        feed.tick(0, 1800);
        assert_eq!(feed.controller.state(), ToggleState::Off);
        feed.obs.alive = true;
        feed.tick(1300, 1300);
        assert_eq!(feed.controller.state(), ToggleState::Off);

        let mut feed = Feed::on(800, 1300, 500);
        feed.obs.item = None;
        feed.tick(800, 1300);
        feed.obs.item = Some(ARMLET);
        feed.tick(800, 1300);
        assert_eq!(feed.controller.state(), ToggleState::Off);
    }

    #[test]
    fn swapping_armlet_for_sabatons_reads_off_and_forgets_the_bonus() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.obs.item = Some(SABATONS);
        feed.tick(800, 1300);
        assert_eq!(feed.controller.state(), ToggleState::Off);
        assert_eq!(feed.controller.bonus, None);
    }

    #[test]
    fn low_hp_with_it_on_gets_the_trick_and_nothing_while_it_lands() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.tick(270, 1800);
        assert_eq!(feed.decide(low()), Some(Press::Trick));

        // The switch-off shows: max down by the bonus, HP floored.
        let tracked = feed.tick(1, 1300);
        assert_eq!(tracked.self_inflicted, 269);
        assert_eq!(feed.decide(low()), None);
        // The Strength comes back in, tick over tick: still nothing.
        for (health, max) in [(170, 1470), (340, 1640), (501, 1800)] {
            feed.tick(health, max);
            assert_eq!(feed.decide(low()), None, "pressed mid-ramp at {health}");
        }
        let tracked = feed.tick(495, 1800);
        assert!(matches!(
            tracked.outcome,
            Some(Outcome::Landed {
                press: Press::Trick,
                hp_before: 270,
                hp_low: 1,
                hp_after: 495,
                ..
            })
        ));
        assert_eq!(feed.controller.state(), ToggleState::On);
    }

    #[test]
    fn a_trick_whose_second_press_never_shows_is_finished_with_one_press() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.tick(270, 1800);
        assert_eq!(feed.decide(low()), Some(Press::Trick));
        feed.tick(1, 1300);
        let tracked = feed.tick(2, 1300);
        assert_eq!(tracked.outcome, Some(Outcome::SecondPressLost { hp: 2 }));
        assert_eq!(feed.controller.state(), ToggleState::Off);
        // Straight away, whatever the cooldown and whether or not HP is low.
        assert_eq!(
            feed.decide(Want {
                trick: false,
                ..low()
            }),
            Some(Press::TurnOn)
        );
    }

    /// Logged 2026-11-21: two tricks moved nothing with max_health still
    /// carrying the bonus, and a single press went out that would have
    /// switched it off at 43 HP.
    #[test]
    fn tricks_that_move_nothing_while_on_retry_then_wait_and_read_off_only_on_a_third_miss() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.tick(270, 1800);
        assert_eq!(feed.decide(low()), Some(Press::Trick));
        for _ in 0..3 {
            feed.tick(270, 1800);
        }
        // First miss: a lost press, straight back in.
        assert_eq!(feed.controller.state(), ToggleState::On);
        assert_eq!(feed.decide(low()), Some(Press::Trick));
        for _ in 0..3 {
            feed.tick(270, 1800);
        }
        // Second miss: still on, and no single press into it. Wait.
        assert_eq!(feed.controller.state(), ToggleState::On);
        assert_eq!(feed.decide(low()), None);
        feed.tick(270, 1800);
        assert_eq!(feed.decide(low()), None);
        feed.tick(270, 1800);
        feed.tick(270, 1800);
        assert_eq!(feed.decide(low()), Some(Press::Trick));
        for _ in 0..3 {
            feed.tick(270, 1800);
        }
        // Third miss: the reading is what is wrong.
        assert_eq!(feed.controller.state(), ToggleState::Off);
        assert_eq!(feed.decide(low()), Some(Press::TurnOn));
    }

    #[test]
    fn a_landed_trick_clears_the_misses() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.tick(270, 1800);
        assert_eq!(feed.decide(low()), Some(Press::Trick));
        for _ in 0..3 {
            feed.tick(270, 1800);
        }
        assert_eq!(feed.controller.unanswered_tricks, 1);
        assert_eq!(feed.decide(low()), Some(Press::Trick));
        feed.tick(1, 1300);
        feed.tick(250, 1550);
        assert_eq!(feed.controller.unanswered_tricks, 0);
    }

    /// Logged 2026-11-21: GSI said alive at 0 HP and a trick went out.
    #[test]
    fn no_press_at_zero_hp() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.tick(0, 1800);
        assert_eq!(feed.decide(low()), None);
    }

    #[test]
    fn dying_before_a_trick_lands_is_reported_once() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.tick(270, 1800);
        assert_eq!(feed.decide(low()), Some(Press::Trick));
        feed.tick(1, 1300);
        feed.obs.alive = false;
        let tracked = feed.tick(0, 1300);
        assert_eq!(
            tracked.outcome,
            Some(Outcome::Died {
                press: Press::Trick,
                hp_before: 270,
                hp_low: 1,
                after_ms: 450,
            })
        );
        assert_eq!(feed.tick(0, 1300).outcome, None);
    }

    #[test]
    fn last_tick_loss_leaves_out_what_the_toggle_took() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.tick(1200, 1800);
        assert_eq!(feed.controller.last_tick_loss(), 100);
        // Switched off: 500 of the 550 is the toggle, 50 a hit.
        feed.tick(650, 1300);
        assert_eq!(feed.controller.last_tick_loss(), 50);
        feed.tick(660, 1300);
        assert_eq!(feed.controller.last_tick_loss(), 0);
    }

    #[test]
    fn unknown_probes_with_the_trick_and_turns_it_on_once_it_reads_off() {
        let mut feed = Feed::new(270, 1300);
        assert_eq!(feed.decide(low()), Some(Press::Trick));
        let mut outcome = None;
        for _ in 0..3 {
            outcome = feed.tick(265, 1300).outcome.or(outcome);
        }
        assert_eq!(outcome, Some(Outcome::NoEffect { press: Press::Trick }));
        assert_eq!(feed.controller.state(), ToggleState::Off);
        assert_eq!(feed.decide(low()), Some(Press::TurnOn));
        feed.tick(345, 1380);
        assert_eq!(feed.controller.state(), ToggleState::On);
    }

    #[test]
    fn off_and_out_of_combat_is_left_alone() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.tick(760, 1800); // the last hit of a fight
        for _ in 0..4 {
            feed.tick(760, 1800);
        }
        feed.tick(260, 1300); // the player switches it off, nothing hitting
        for health in [262, 264, 266, 268, 270] {
            feed.tick(health, 1300);
            assert_eq!(feed.decide(calm()), None);
        }
    }

    #[test]
    fn off_under_fire_gets_one_press_not_the_chord() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.tick(400, 1300); // the player switches it off
        for health in [400, 400, 400] {
            feed.tick(health, 1300); // the hold for their press back on
        }
        feed.tick(270, 1300);
        assert_eq!(feed.decide(low()), Some(Press::TurnOn));
    }

    #[test]
    fn a_players_switch_off_holds_the_automation_for_their_press_back_on() {
        let mut feed = Feed::on(400, 1300, 500);
        feed.tick(1, 1300); // their trick, first half
        assert_eq!(feed.decide(low()), None);
        feed.tick(170, 1470); // their second half lands
        assert_eq!(feed.decide(low()), None);
    }

    #[test]
    fn a_single_press_that_switches_it_off_is_finished_with_another() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.controller.state = ToggleState::Off; // a wrong belief
        feed.tick(270, 1800);
        assert_eq!(feed.decide(low()), Some(Press::TurnOn));
        let tracked = feed.tick(1, 1300);
        assert_eq!(tracked.outcome, Some(Outcome::WasOn { hp: 1 }));
        assert_eq!(feed.decide(low()), Some(Press::TurnOn));
    }

    #[test]
    fn a_press_that_moves_nothing_backs_off_for_the_confirm_window() {
        let mut feed = Feed::new(270, 1300);
        feed.controller.state = ToggleState::Off;
        assert_eq!(feed.decide(low()), Some(Press::TurnOn));
        for _ in 0..3 {
            feed.tick(270, 1300);
        }
        assert_eq!(feed.decide(low()), None);
        feed.tick(270, 1300);
        assert_eq!(feed.decide(low()), None);
        for _ in 0..2 {
            feed.tick(270, 1300);
        }
        assert_eq!(feed.decide(low()), Some(Press::TurnOn));
    }

    #[test]
    fn no_press_while_disabled_and_a_refused_trick_keeps_the_state() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.obs.disabled = true;
        feed.tick(270, 1800);
        assert_eq!(feed.decide(low()), None);

        feed.obs.disabled = false;
        feed.tick(270, 1800);
        assert_eq!(feed.decide(low()), Some(Press::Trick));
        feed.obs.disabled = true; // stunned as the presses go out
        for _ in 0..3 {
            feed.tick(270, 1800);
        }
        assert_eq!(feed.controller.state(), ToggleState::On);
    }

    #[test]
    fn nothing_goes_into_another_window_and_a_pending_finish_waits_for_focus() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.tick(270, 1800);
        let unfocused = Want {
            focused: false,
            ..low()
        };
        assert_eq!(feed.decide(unfocused), None);
        assert_eq!(feed.decide(low()), Some(Press::Trick));
        feed.tick(1, 1300);
        feed.tick(2, 1300); // the press back on never shows
        assert_eq!(feed.decide(unfocused), None);
        assert_eq!(feed.decide(low()), Some(Press::TurnOn));
    }

    #[test]
    fn no_trick_once_hp_reaches_the_bonus() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.tick(500, 1800);
        assert_eq!(feed.decide(low()), None);
        feed.tick(499, 1800);
        assert_eq!(feed.decide(low()), Some(Press::Trick));
    }

    #[test]
    fn presses_keep_the_cooldown_apart() {
        let mut feed = Feed::on(800, 1300, 500);
        feed.tick(270, 1800);
        assert_eq!(feed.decide(low()), Some(Press::Trick));
        // A ramp that is over before the cooldown: the cooldown still holds.
        feed.controller.phase = super::Phase::Settled;
        feed.at_ms += 100;
        feed.obs.at_ms = feed.at_ms;
        assert_eq!(feed.decide(low()), None);
    }

    #[test]
    fn the_confirm_window_runs_from_the_press_not_from_the_last_move() {
        // max_health sits still for minutes at a time; timing from its last
        // move would call every press lost on the first tick after it.
        let mut feed = Feed::on(800, 1300, 500);
        for _ in 0..40 {
            feed.tick(800, 1800);
        }
        feed.tick(270, 1800);
        assert_eq!(feed.decide(low()), Some(Press::Trick));
        let tracked = feed.tick(270, 1800);
        assert_eq!(tracked.outcome, None);
        assert!(feed.controller.busy());
    }

    #[test]
    fn toggle_drain_slot_finds_armlet_or_sabatons() {
        let empty = item("empty");
        let armlet = item("item_armlet");
        let sabatons = item("item_berserkers_sabatons");

        assert_eq!(
            find_toggle_drain_slot(&[("slot0", &empty), ("slot1", &armlet)]),
            Some((0, "slot1"))
        );
        assert_eq!(
            find_toggle_drain_slot(&[("slot0", &empty), ("slot3", &sabatons)]),
            Some((1, "slot3"))
        );
        assert_eq!(find_toggle_drain_slot(&[("slot0", &empty)]), None);
    }

    #[test]
    fn toggle_drain_slot_prefers_armlet_when_both_are_held() {
        let armlet = item("item_armlet");
        let sabatons = item("item_berserkers_sabatons");

        assert_eq!(
            find_toggle_drain_slot(&[("slot0", &sabatons), ("slot4", &armlet)]),
            Some((0, "slot4"))
        );
    }

    fn item(name: &str) -> crate::models::gsi_event::Item {
        crate::models::gsi_event::Item {
            name: name.to_string(),
            ..Default::default()
        }
    }

    fn roshan_test_config() -> ArmletRoshanConfig {
        ArmletRoshanConfig {
            enabled: true,
            toggle_key: "Insert".to_string(),
            emergency_margin_hp: 60,
            learning_window_ms: 5_000,
            min_confidence_hits: 2,
            min_sample_damage: 80,
            stale_reset_ms: 6_000,
        }
    }

    #[test]
    fn dual_trigger_plan_uses_quick_cast_then_modified_cast() {
        let slot = super::KeyBinding::Char('x').pressable().unwrap();

        assert_eq!(
            plan_dual_trigger_sequence(slot, ModifierKey::Alt),
            [
                ArmletTriggerStep::QuickCast(slot),
                ArmletTriggerStep::ModifierDown(ModifierKey::Alt),
                ArmletTriggerStep::QuickCast(slot),
                ArmletTriggerStep::ModifierUp(ModifierKey::Alt),
            ]
        );
    }

    #[test]
    fn parse_cast_modifier_supports_common_aliases() {
        assert_eq!(parse_cast_modifier("Alt"), Some(ModifierKey::Alt));
        assert_eq!(parse_cast_modifier("ctrl"), Some(ModifierKey::Control));
        assert_eq!(parse_cast_modifier("Control"), Some(ModifierKey::Control));
        assert_eq!(parse_cast_modifier("Shift"), Some(ModifierKey::Shift));
    }

    #[test]
    fn invalid_modifier_falls_back_to_alt() {
        let config = EffectiveArmletConfig {
            enabled: true,
            cast_modifier: "weird".to_string(),
            toggle_threshold: 320,
            predictive_offset: 30,
            toggle_cooldown_ms: 250,
            press_confirm_ms: 600,
            roshan: ArmletRoshanConfig::default(),
        };

        assert_eq!(resolve_cast_modifier(&config), ModifierKey::Alt);
    }

    #[test]
    fn roshan_mode_triggers_on_first_large_hit_when_health_enters_emergency_margin() {
        let mut state = ArmletRoshanState::armed();
        let config = roshan_test_config();

        assert_eq!(
            record_roshan_health_sample(&mut state, 1_000, 320, 0, &config),
            None
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 1_200, 120, 0, &config),
            Some(200)
        );

        assert_eq!(
            evaluate_roshan_trigger(120, 1_200, &state, &config),
            Some(RoshanArmletTrigger::EmergencyFallback {
                observed_damage: 200,
                lethal_zone: 260,
            })
        );
    }

    #[test]
    fn roshan_learning_ignores_what_our_own_toggle_took() {
        let mut state = ArmletRoshanState::armed();
        let config = roshan_test_config();

        record_roshan_health_sample(&mut state, 1_000, 270, 0, &config);
        // The trick's switch-off floors 270 HP to 1: 269 of it was ours.
        assert_eq!(
            record_roshan_health_sample(&mut state, 1_200, 1, 269, &config),
            None
        );
        assert!(state.samples.is_empty());
        // A real hit landing in the same event still counts, net of ours.
        record_roshan_health_sample(&mut state, 1_400, 500, 0, &config);
        assert_eq!(
            record_roshan_health_sample(&mut state, 1_600, 1, 300, &config),
            Some(199)
        );
    }

    #[test]
    fn roshan_mode_uses_largest_recent_hit_once_confident() {
        let mut state = ArmletRoshanState::armed();
        let config = roshan_test_config();

        assert_eq!(
            record_roshan_health_sample(&mut state, 1_000, 500, 0, &config),
            None
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 1_200, 360, 0, &config),
            Some(140)
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 2_200, 210, 0, &config),
            Some(150)
        );

        assert_eq!(
            evaluate_roshan_trigger(190, 2_200, &state, &config),
            Some(RoshanArmletTrigger::LearnedHit {
                predicted_damage: 150,
                lethal_zone: 210,
                sample_count: 2,
            })
        );
    }

    #[test]
    fn clearing_roshan_learning_state_resets_stale_health_and_samples() {
        let mut state = ArmletRoshanState::armed();
        let config = roshan_test_config();

        assert_eq!(
            record_roshan_health_sample(&mut state, 1_000, 500, 0, &config),
            None
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 1_200, 360, 0, &config),
            Some(140)
        );
        assert_eq!(state.last_health, Some(360));
        assert_eq!(state.latest_observed_damage, Some(140));
        assert_eq!(state.samples.len(), 1);

        clear_roshan_learning_state(&mut state);

        assert_eq!(state.last_health, None);
        assert_eq!(state.latest_observed_damage, None);
        assert!(state.samples.is_empty());
    }

    #[test]
    fn roshan_stun_recovery_defers_toggle_until_next_valid_hit_when_one_more_hit_is_survivable() {
        let config = roshan_test_config();
        let mut state = ArmletRoshanState::armed();

        assert_eq!(
            record_roshan_health_sample(&mut state, 1_000, 520, 0, &config),
            None
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 1_200, 380, 0, &config),
            Some(140)
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 2_200, 230, 0, &config),
            Some(150)
        );

        let recovery = evaluate_roshan_stun_recovery(&mut state, 230, false, true, 2_300, &config);

        assert_eq!(
            recovery,
            RoshanRecoveryAction::AwaitNextHit {
                predicted_damage: 150
            }
        );
        assert!(state.awaiting_post_stun_hit);
    }

    #[test]
    fn roshan_stun_recovery_resyncs_on_first_valid_hit_after_deferral() {
        let config = roshan_test_config();
        let mut state = ArmletRoshanState::armed();
        state.awaiting_post_stun_hit = true;
        state.stun_recovery_estimate_damage = Some(150);

        assert_eq!(
            record_roshan_health_sample(&mut state, 3_000, 230, 0, &config),
            None
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 3_250, 80, 0, &config),
            Some(150)
        );

        let recovery = evaluate_roshan_stun_recovery(&mut state, 80, false, false, 3_250, &config);

        assert_eq!(
            recovery,
            RoshanRecoveryAction::TriggerDeferredHit {
                observed_damage: 150
            }
        );
        assert!(!state.awaiting_post_stun_hit);
    }

    #[test]
    fn roshan_stun_recovery_does_not_defer_when_recovery_is_already_lethal() {
        let config = roshan_test_config();
        let mut state = ArmletRoshanState::armed();

        assert_eq!(
            record_roshan_health_sample(&mut state, 1_000, 500, 0, &config),
            None
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 1_200, 360, 0, &config),
            Some(140)
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 2_200, 210, 0, &config),
            Some(150)
        );

        let recovery = evaluate_roshan_stun_recovery(&mut state, 200, false, true, 2_300, &config);

        assert_eq!(recovery, RoshanRecoveryAction::ProtectNow);
        assert!(!state.awaiting_post_stun_hit);
    }

    #[test]
    fn roshan_stun_recovery_ignores_small_damage_while_waiting_for_next_hit() {
        let config = roshan_test_config();
        let mut state = ArmletRoshanState::armed();
        state.awaiting_post_stun_hit = true;
        state.stun_recovery_estimate_damage = Some(150);

        assert_eq!(
            record_roshan_health_sample(&mut state, 4_000, 220, 0, &config),
            None
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 4_200, 180, 0, &config),
            None
        );

        let recovery = evaluate_roshan_stun_recovery(&mut state, 180, false, false, 4_200, &config);

        assert_eq!(recovery, RoshanRecoveryAction::None);
        assert!(state.awaiting_post_stun_hit);
    }

    #[test]
    fn clearing_roshan_learning_state_resets_stun_recovery_defer_fields() {
        let mut state = ArmletRoshanState::armed();
        state.awaiting_post_stun_hit = true;
        state.stun_recovery_estimate_damage = Some(150);
        state.stun_recovery_started_at_ms = Some(2_300);
        state.was_stunned_last_tick = true;

        clear_roshan_learning_state(&mut state);

        assert_eq!(state.last_health, None);
        assert_eq!(state.latest_observed_damage, None);
        assert!(state.samples.is_empty());
        assert!(!state.awaiting_post_stun_hit);
        assert_eq!(state.stun_recovery_estimate_damage, None);
        assert_eq!(state.stun_recovery_started_at_ms, None);
        assert!(!state.was_stunned_last_tick);
    }

    #[test]
    fn roshan_reset_reason_strings_match_expected_logs() {
        assert_eq!(RoshanResetReason::HeroDied.as_str(), "hero died");
        assert_eq!(
            RoshanResetReason::ArmletDisabled.as_str(),
            "armlet automation disabled"
        );
        assert_eq!(
            RoshanResetReason::ArmletMissing.as_str(),
            "armlet item missing"
        );
        assert_eq!(
            RoshanResetReason::RoshanModeDisarmed.as_str(),
            "roshan mode inactive"
        );
        assert_eq!(
            RoshanResetReason::ModeToggled.as_str(),
            "roshan mode toggled"
        );
    }

    #[test]
    fn roshan_skip_logging_window_stays_quiet_when_health_is_far_above_danger() {
        assert!(!should_log_roshan_skip_context(
            620,
            270,
            None,
            Some(140),
            60,
        ));
    }

    #[test]
    fn roshan_skip_logging_window_turns_on_near_lethal_zone() {
        assert!(should_log_roshan_skip_context(
            340,
            270,
            Some(120),
            None,
            60,
        ));
    }

    #[test]
    fn roshan_skip_logging_window_uses_predicted_damage_when_present() {
        assert!(should_log_roshan_skip_context(315, 270, None, Some(80), 60,));
    }

    #[test]
    fn roshan_skip_logging_window_stays_quiet_until_trigger_band_is_close() {
        assert!(!should_log_roshan_skip_context(
            500,
            270,
            Some(120),
            None,
            60,
        ));
    }

    #[test]
    fn resolve_armlet_config_uses_shared_defaults_for_unknown_hero() {
        let mut settings = Settings::default();
        settings.armlet = ArmletAutomationConfig {
            enabled: true,
            cast_modifier: "Shift".to_string(),
            toggle_threshold: 350,
            predictive_offset: 40,
            toggle_cooldown_ms: 280,
            press_confirm_ms: 700,
            roshan: ArmletRoshanConfig::default(),
        };

        let resolved = settings.resolve_armlet_config("npc_dota_hero_kunkka");

        assert_eq!(resolved.enabled, true);
        assert_eq!(resolved.cast_modifier, "Shift");
        assert_eq!(resolved.toggle_threshold, 350);
        assert_eq!(resolved.predictive_offset, 40);
        assert_eq!(resolved.toggle_cooldown_ms, 280);
        assert_eq!(resolved.press_confirm_ms, 700);
    }

    #[test]
    fn resolve_armlet_config_uses_huskar_legacy_fields_when_nested_override_is_empty() {
        let mut settings = Settings::default();
        settings.armlet.toggle_threshold = 320;
        settings.armlet.predictive_offset = 30;
        settings.armlet.toggle_cooldown_ms = 250;
        settings.heroes.huskar.armlet_toggle_threshold = 120;
        settings.heroes.huskar.armlet_predictive_offset = 150;
        settings.heroes.huskar.armlet_toggle_cooldown_ms = 300;

        let resolved = settings.resolve_armlet_config("npc_dota_hero_huskar");

        assert_eq!(resolved.toggle_threshold, 120);
        assert_eq!(resolved.predictive_offset, 150);
        assert_eq!(resolved.toggle_cooldown_ms, 300);
    }

    #[test]
    fn resolve_armlet_config_prefers_nested_override_and_falls_back_to_shared_defaults() {
        let mut settings = Settings::default();
        settings.armlet.toggle_threshold = 320;
        settings.armlet.predictive_offset = 30;
        settings.armlet.toggle_cooldown_ms = 250;
        settings.heroes.huskar.armlet = HeroArmletOverrideConfig {
            enabled: Some(false),
            toggle_threshold: Some(110),
            predictive_offset: None,
            toggle_cooldown_ms: Some(190),
        };
        settings.heroes.huskar.armlet_toggle_threshold = 120;
        settings.heroes.huskar.armlet_predictive_offset = 150;
        settings.heroes.huskar.armlet_toggle_cooldown_ms = 300;

        let resolved = settings.resolve_armlet_config("npc_dota_hero_huskar");

        assert_eq!(resolved.enabled, false);
        assert_eq!(resolved.toggle_threshold, 110);
        assert_eq!(resolved.predictive_offset, 30);
        assert_eq!(resolved.toggle_cooldown_ms, 190);
    }
}
