//! Power Treads switching: INT before a cast, STR in danger, the smallest pool
//! for a heal, and home otherwise.
//!
//! GSI lists `item_power_treads` but never says which attribute it is on, so
//! the state is inferred from how `max_health` and `max_mana` move when the
//! treads switch. Everything here was measured before it was modelled
//! (`docs/workflows/probe-first.md`):
//!
//! | Finding (2026-09-21, `examples/power_treads_probe.rs`) | Value |
//! |---|---|
//! | Treads attribute in GSI | none — the item object is identical in all three states |
//! | Start state after purchase | STR→STR, AGI→AGI, INT→INT, universal→STR |
//! | One switch, ordinary hero | 220 max HP / 120 max mana |
//! | HP% / mana% across a switch | preserved, 46/46 |
//! | Press → change visible in GSI | 13–251ms; GSI tick ~228ms median |
//! | Ogre Magi | STR also gives mana, INT none: INT and AGI look identical |
//! | Outworld Destroyer | Astral Imprisonment: +1250 max mana, no press |
//! | Press during cast point / channel | does not cancel it |
//! | Press while stunned | does nothing |
//!
//! Those findings shape the design:
//!
//! * **Exact deltas, not signs.** OD's buff has the sign of AGI→INT. A change
//!   counts as a switch only when it equals a learned switch exactly; signs are
//!   the seed before anything is learned, and a sign-only match is "suspect"
//!   until a verify confirms it.
//! * **Learned per-hero offsets.** Each attribute's `(max_hp, max_mana)` offset
//!   relative to STR is learned in game. Targets follow from them: "most mana"
//!   is INT for most heroes and STR for Ogre Magi.
//! * **Verify cycle.** Three presses, one per tick, back to the start. The
//!   attribute with the most HP is STR on every hero, and the fixed
//!   STR→INT→AGI order names the other two.
//!
//! No measured number appears in the logic below — the simulator
//! (`examples/power_treads_control.rs`) holds them, and runs this module's
//! [`TreadsMachine`] against them.

use crate::actions::hero_attributes::{primary_attribute, PrimaryAttribute};
use crate::config::settings::PowerTreadsConfig;
use crate::config::Settings;
use crate::input::binding::KeyBinding;
use crate::models::GsiWebhookEvent;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info};

pub const POWER_TREADS: &str = "item_power_treads";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attr {
    Str,
    Int,
    Agi,
}

impl Attr {
    pub const ALL: [Attr; 3] = [Attr::Str, Attr::Int, Attr::Agi];

    pub fn index(self) -> usize {
        match self {
            Attr::Str => 0,
            Attr::Int => 1,
            Attr::Agi => 2,
        }
    }

    fn from_index(index: usize) -> Attr {
        Attr::ALL[index % 3]
    }

    /// Treads cycle STR → INT → AGI → STR, one step per press.
    pub fn advance(self, presses: u8) -> Attr {
        Attr::from_index(self.index() + presses as usize)
    }

    pub fn presses_to(self, target: Attr) -> u8 {
        ((target.index() + 3 - self.index()) % 3) as u8
    }

    pub fn label(self) -> &'static str {
        match self {
            Attr::Str => "STR",
            Attr::Int => "INT",
            Attr::Agi => "AGI",
        }
    }

    /// Where treads bought on this hero start, and the default home.
    /// Universal heroes measured STR; an unknown hero guesses the same.
    pub fn for_hero(hero_name: &str) -> Attr {
        match primary_attribute(hero_name) {
            Some(PrimaryAttribute::Agility) => Attr::Agi,
            Some(PrimaryAttribute::Intelligence) => Attr::Int,
            Some(PrimaryAttribute::Strength) | Some(PrimaryAttribute::Universal) | None => {
                Attr::Str
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confidence {
    Confident,
    Suspect,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealKind {
    Health,
    Mana,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    SpendMana,
    Heal(HealKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreadsSlot {
    Missing,
    /// Backpack or stash: owned, but no bonus and cannot be toggled.
    Inactive,
    Active,
}

/// What the machine needs from one GSI event.
#[derive(Clone, Debug)]
pub struct Observation {
    pub now_ms: u64,
    pub hero: String,
    /// Where treads start on this hero — see [`Attr::for_hero`].
    pub primary: Attr,
    pub alive: bool,
    pub level: u32,
    pub health: i64,
    pub max_health: i64,
    pub mana: i64,
    pub max_mana: i64,
    pub stunned: bool,
    pub hexed: bool,
    pub muted: bool,
    pub treads: TreadsSlot,
    /// Changes whenever any item slot changes.
    pub inventory_signature: u64,
    pub in_danger: bool,
}

impl Observation {
    fn health_fraction(&self) -> f64 {
        if self.max_health <= 0 {
            0.0
        } else {
            self.health as f64 / self.max_health as f64
        }
    }

    fn mana_fraction(&self) -> f64 {
        if self.max_mana <= 0 {
            0.0
        } else {
            self.mana as f64 / self.max_mana as f64
        }
    }

    fn health_percent(&self) -> i64 {
        self.health * 100 / self.max_health.max(1)
    }
}

/// Which heals the heal switch applies to, and which pool each fills.
///
/// Hand-kept: unlike mana costs, dotaconstants has no field for "restores
/// health". Items that also cost mana (Mekansm) are left to the cast switch.
pub fn heal_kind_for_item(item_name: &str) -> Option<HealKind> {
    match item_name {
        "item_flask"
        | "item_tango"
        | "item_tango_single"
        | "item_faerie_fire"
        | "item_greater_faerie_fire" => Some(HealKind::Health),
        "item_clarity" | "item_enchanted_mango" | "item_arcane_boots" => Some(HealKind::Mana),
        "item_bottle"
        | "item_magic_stick"
        | "item_magic_wand"
        | "item_cheese"
        | "item_famango"
        | "item_great_famango"
        | "item_royal_jelly"
        | "item_holy_locket"
        | "item_guardian_greaves" => Some(HealKind::Both),
        _ => None,
    }
}

/// Before anything is learned: an ordinary hero's switch, as signs only.
/// STR adds HP, INT adds mana, AGI adds neither.
fn seed_offset(attr: Attr) -> (i64, i64) {
    match attr {
        Attr::Str => (0, 0),
        Attr::Int => (-1, 1),
        Attr::Agi => (-1, 0),
    }
}

fn seed_signature(from: Attr, to: Attr) -> (i64, i64) {
    let (a, b) = (seed_offset(from), seed_offset(to));
    ((b.0 - a.0).signum(), (b.1 - a.1).signum())
}

fn signs(delta: (i64, i64)) -> (i64, i64) {
    (delta.0.signum(), delta.1.signum())
}

struct CastWindow {
    target: Attr,
    last_key_ms: u64,
    spent_ms: Option<u64>,
}

struct HealWindow {
    kind: HealKind,
    target: Attr,
    started_ms: u64,
    last_rising_ms: u64,
}

struct Verify {
    prior: Option<Attr>,
    prior_confident: bool,
    deltas: Vec<(i64, i64)>,
    awaiting: bool,
    step_sent_ms: u64,
}

pub struct TreadsMachine {
    pub config: PowerTreadsConfig,
    last: Option<Observation>,
    pub estimate: Option<Attr>,
    pub confidence: Confidence,
    pub home: Option<Attr>,
    /// `(max_hp, max_mana)` of each attribute relative to STR. STR is `(0, 0)`
    /// by definition; the other two are learned.
    offsets: [Option<(i64, i64)>; 3],
    pending: u8,
    pending_since_ms: u64,
    manual_pending: u8,
    verify: Option<Verify>,
    last_verify_ms: u64,
    verify_not_before_ms: u64,
    cast: Option<CastWindow>,
    heal: Option<HealWindow>,
    last_key_ms: Option<u64>,
    hp_rate_baseline: f64,
    mana_rate_baseline: f64,
    /// Decisions, newest last. Bounded; read by the simulator and debug logs.
    pub log: Vec<String>,
}

const LOG_LIMIT: usize = 200;

impl TreadsMachine {
    pub fn new(config: PowerTreadsConfig) -> Self {
        Self {
            config,
            last: None,
            estimate: None,
            confidence: Confidence::Unknown,
            home: None,
            offsets: [Some((0, 0)), None, None],
            pending: 0,
            pending_since_ms: 0,
            manual_pending: 0,
            verify: None,
            last_verify_ms: 0,
            verify_not_before_ms: 0,
            cast: None,
            heal: None,
            last_key_ms: None,
            hp_rate_baseline: 0.0,
            mana_rate_baseline: 0.0,
            log: Vec::new(),
        }
    }

    /// Presses sent but not yet seen in GSI. Read by the simulator.
    #[allow(dead_code)]
    pub fn pending(&self) -> u8 {
        self.pending
    }

    pub fn offsets_complete(&self) -> bool {
        self.offsets.iter().all(Option::is_some)
    }

    fn note(&mut self, now_ms: u64, message: String) {
        debug!("👟 {message}");
        if self.log.len() >= LOG_LIMIT {
            self.log.remove(0);
        }
        self.log.push(format!("[{:>6}ms] {}", now_ms, message));
    }

    fn forget(&mut self, now_ms: u64, why: &str) {
        self.estimate = None;
        self.confidence = Confidence::Unknown;
        self.offsets = [Some((0, 0)), None, None];
        self.pending = 0;
        self.manual_pending = 0;
        self.verify = None;
        self.cast = None;
        self.heal = None;
        self.note(now_ms, format!("forget: {why}"));
    }

    fn suspect(&mut self, now_ms: u64, why: &str) {
        if self.confidence == Confidence::Confident {
            self.confidence = Confidence::Suspect;
        }
        self.note(now_ms, format!("suspect: {why}"));
    }

    fn can_press(obs: &Observation) -> bool {
        obs.alive && !obs.stunned && !obs.hexed && !obs.muted && obs.treads == TreadsSlot::Active
    }

    fn transition(&self, from: Attr, to: Attr) -> Option<(i64, i64)> {
        let a = self.offsets[from.index()]?;
        let b = self.offsets[to.index()]?;
        Some((b.0 - a.0, b.1 - a.1))
    }

    fn learn(&mut self, now_ms: u64, from: Attr, to: Attr, delta: (i64, i64)) {
        match (self.offsets[from.index()], self.offsets[to.index()]) {
            (Some(a), None) => {
                self.offsets[to.index()] = Some((a.0 + delta.0, a.1 + delta.1));
                self.note(
                    now_ms,
                    format!(
                        "learned {} offset {:?}",
                        to.label(),
                        self.offsets[to.index()]
                    ),
                );
            }
            (None, Some(b)) => {
                self.offsets[from.index()] = Some((b.0 - delta.0, b.1 - delta.1));
                self.note(
                    now_ms,
                    format!(
                        "learned {} offset {:?}",
                        from.label(),
                        self.offsets[from.index()]
                    ),
                );
            }
            _ => {}
        }
    }

    /// How many presses from the estimate explain `delta`, if any.
    fn explain(&self, delta: (i64, i64), fresh: bool) -> Option<u32> {
        let estimate = self.estimate?;
        let pending = self.pending as u32;
        let mut candidates: Vec<u32> = Vec::new();
        if pending > 0 {
            candidates.push(pending);
            candidates.extend(1..pending);
        }
        candidates.extend(pending + 1..=pending + 2);

        for &count in &candidates {
            if count % 3 == 0 {
                continue;
            }
            let to = estimate.advance((count % 3) as u8);
            let matched = match self.transition(estimate, to) {
                Some(expected) => expected == delta,
                None => signs(delta) == seed_signature(estimate, to),
            };
            if matched {
                return Some(count);
            }
        }

        // Nothing fits the learned or seeded pattern. Our own single press
        // landing on time is still strong evidence — this is how Ogre Magi's
        // odd offsets get learned.
        if pending == 1 && fresh {
            let to = estimate.advance(1);
            if self.transition(estimate, to).is_none() {
                return Some(1);
            }
        }
        None
    }

    fn most_mana(&self, from: Attr) -> Attr {
        let pick = |offset: &dyn Fn(Attr) -> (i64, i64)| {
            *Attr::ALL
                .iter()
                .max_by_key(|a| (offset(**a).1, -(from.presses_to(**a) as i64), offset(**a).0))
                .unwrap_or(&from)
        };
        if self.offsets_complete() {
            return pick(&|a| self.offsets[a.index()].unwrap_or((0, 0)));
        }
        let seeded = pick(&seed_offset);
        // The seed describes an ordinary hero. If what we did learn says the
        // seeded target is no better, trust the learning and stay.
        if let (Some(t), Some(f)) = (self.offsets[seeded.index()], self.offsets[from.index()]) {
            if t.1 <= f.1 {
                return from;
            }
        }
        seeded
    }

    fn least_pool(&self, kind: HealKind, from: Attr) -> Attr {
        let key = |offset: (i64, i64), attr: Attr| -> (i64, i64, i64) {
            let presses = from.presses_to(attr) as i64;
            match kind {
                HealKind::Health => (offset.0, presses, offset.1),
                HealKind::Mana => (offset.1, presses, -offset.0),
                HealKind::Both => (offset.0, offset.1, presses),
            }
        };
        let source = |attr: Attr| -> (i64, i64) {
            if self.offsets_complete() {
                self.offsets[attr.index()].unwrap_or((0, 0))
            } else {
                seed_offset(attr)
            }
        };
        *Attr::ALL
            .iter()
            .min_by_key(|a| key(source(**a), **a))
            .unwrap_or(&from)
    }

    /// The player pressed the treads key themselves.
    pub fn on_manual_press(&mut self, now_ms: u64) {
        self.abort_verify(now_ms, "manual press");
        self.last_key_ms = Some(now_ms);
        self.pending += 1;
        self.manual_pending += 1;
        self.pending_since_ms = now_ms;
    }

    /// An ability or item key is about to go out. Returns how many treads
    /// presses to send first.
    pub fn on_key(&mut self, now_ms: u64, intent: Intent) -> u8 {
        self.last_key_ms = Some(now_ms);
        if !self.config.enabled {
            return 0;
        }
        self.abort_verify(now_ms, "key pressed");

        let Some(obs) = self.last.clone() else {
            return 0;
        };
        if self.confidence != Confidence::Confident || !Self::can_press(&obs) {
            return 0;
        }
        let Some(current) = self.estimate.map(|e| e.advance(self.pending)) else {
            return 0;
        };

        let target = match intent {
            Intent::SpendMana => {
                if !self.config.cast_switch || (self.config.danger_switch && obs.in_danger) {
                    return 0;
                }
                let target = self.most_mana(current);
                let loses_hp = match (self.offsets[target.index()], self.offsets[current.index()]) {
                    (Some(t), Some(c)) => t.0 < c.0,
                    _ => seed_offset(target).0 < seed_offset(current).0,
                };
                let health_percent = obs.health_percent();
                if loses_hp && health_percent < self.config.min_health_percent as i64 {
                    self.note(
                        now_ms,
                        format!(
                            "cast: stay on {} at {}% HP",
                            current.label(),
                            health_percent
                        ),
                    );
                    return 0;
                }
                match self.cast.as_mut() {
                    Some(window) => {
                        window.last_key_ms = now_ms;
                        window.spent_ms = None;
                        window.target = target;
                    }
                    None => {
                        self.cast = Some(CastWindow {
                            target,
                            last_key_ms: now_ms,
                            spent_ms: None,
                        })
                    }
                }
                target
            }
            Intent::Heal(kind) => {
                if !self.config.heal_switch || obs.in_danger {
                    return 0;
                }
                let target = self.least_pool(kind, current);
                self.heal = Some(HealWindow {
                    kind,
                    target,
                    started_ms: now_ms,
                    last_rising_ms: now_ms,
                });
                // A cast in progress keeps its attribute; the heal target takes
                // over when the cast window closes.
                if self.cast.is_some() {
                    return 0;
                }
                target
            }
        };

        let presses = current.presses_to(target);
        if presses > 0 {
            self.pending += presses;
            self.pending_since_ms = now_ms;
            self.note(
                now_ms,
                format!(
                    "{:?}: {} → {} ({} press)",
                    intent,
                    current.label(),
                    target.label(),
                    presses
                ),
            );
        }
        presses
    }

    fn abort_verify(&mut self, now_ms: u64, why: &str) {
        let Some(verify) = self.verify.take() else {
            return;
        };
        // A confident verify whose steps all matched the learned switches is
        // still confident: we know exactly where it stopped.
        let consistent = verify.prior_confident
            && verify.prior.is_some_and(|prior| {
                verify.deltas.iter().enumerate().all(|(i, delta)| {
                    let from = prior.advance(i as u8);
                    self.transition(from, from.advance(1)) == Some(*delta)
                })
            });
        let done = verify.deltas.len() as u8;
        if consistent {
            self.estimate = verify.prior.map(|p| p.advance(done));
            self.confidence = Confidence::Confident;
            self.pending = u8::from(verify.awaiting);
        } else {
            self.estimate = None;
            self.confidence = Confidence::Unknown;
            self.pending = 0;
        }
        self.verify_not_before_ms = now_ms + self.config.verify_retry_ms;
        self.note(
            now_ms,
            format!(
                "verify aborted ({why}), {}",
                if consistent {
                    "still confident"
                } else {
                    "state unknown"
                }
            ),
        );
    }

    fn fail_verify(&mut self, now_ms: u64, why: &str) {
        self.verify = None;
        self.estimate = None;
        self.confidence = Confidence::Unknown;
        self.pending = 0;
        self.verify_not_before_ms = now_ms + self.config.verify_retry_ms;
        self.note(now_ms, format!("verify failed: {why}"));
    }

    fn finish_verify(&mut self, now_ms: u64) {
        let Some(verify) = self.verify.take() else {
            return;
        };
        let d = &verify.deltas;
        let sum = (d[0].0 + d[1].0 + d[2].0, d[0].1 + d[1].1 + d[2].1);
        if sum != (0, 0) {
            self.fail_verify(now_ms, "three presses did not return to the start");
            return;
        }
        let positions = [(0, 0), d[0], (d[0].0 + d[1].0, d[0].1 + d[1].1)];
        let top = positions.iter().map(|p| p.0).max().unwrap_or(0);
        let tops: Vec<usize> = (0..3).filter(|i| positions[*i].0 == top).collect();
        if tops.len() != 1 {
            self.fail_verify(now_ms, "no single attribute has the most HP");
            return;
        }
        let str_position = tops[0];
        let start = Attr::Str.advance(((3 - str_position) % 3) as u8);
        for (position, offset) in positions.iter().enumerate() {
            let attr = start.advance(position as u8);
            self.offsets[attr.index()] = Some((
                offset.0 - positions[str_position].0,
                offset.1 - positions[str_position].1,
            ));
        }
        if verify.prior_confident && verify.prior != Some(start) {
            self.note(
                now_ms,
                format!("verify corrected {:?} → {}", verify.prior, start.label()),
            );
        }
        self.estimate = Some(start);
        self.confidence = Confidence::Confident;
        self.pending = 0;
        self.last_verify_ms = now_ms;
        self.note(
            now_ms,
            format!("verified {} — offsets {:?}", start.label(), self.offsets),
        );
    }

    fn start_verify(&mut self, now_ms: u64) -> u8 {
        self.verify = Some(Verify {
            prior: self.estimate,
            prior_confident: self.confidence == Confidence::Confident,
            deltas: Vec::new(),
            awaiting: true,
            step_sent_ms: now_ms,
        });
        self.pending = 1;
        self.pending_since_ms = now_ms;
        self.note(now_ms, String::from("verify: start"));
        1
    }

    fn verify_safe(&self, obs: &Observation) -> bool {
        let quiet = self
            .last_key_ms
            .is_none_or(|at| obs.now_ms.saturating_sub(at) >= self.config.verify_quiet_ms);
        Self::can_press(obs)
            && !obs.in_danger
            && quiet
            && self.cast.is_none()
            && self.heal.is_none()
            && obs.health_percent() >= self.config.min_health_percent as i64
    }

    /// One GSI event. Returns how many treads presses to send now.
    pub fn on_tick(&mut self, obs: &Observation) -> u8 {
        let now = obs.now_ms;
        let Some(prev) = self.last.replace(obs.clone()) else {
            self.home = Some(obs.primary);
            self.last_verify_ms = now;
            if obs.treads != TreadsSlot::Missing {
                self.note(now, String::from("treads already owned — state unknown"));
            }
            return 0;
        };

        if obs.hero != prev.hero {
            self.forget(now, "hero changed");
            self.home = Some(obs.primary);
            return 0;
        }

        if prev.treads == TreadsSlot::Missing && obs.treads != TreadsSlot::Missing {
            self.forget(now, "treads bought");
            self.estimate = Some(obs.primary);
            self.confidence = Confidence::Confident;
            self.home = Some(obs.primary);
            self.last_verify_ms = now;
            // Bought at the shop, likely mid-action: the same quiet period as a
            // key before the learning verify runs.
            self.last_key_ms = Some(now);
            return 0;
        }
        match obs.treads {
            TreadsSlot::Missing => {
                if prev.treads != TreadsSlot::Missing {
                    self.forget(now, "treads gone");
                }
                return 0;
            }
            TreadsSlot::Inactive => {
                if prev.treads == TreadsSlot::Active {
                    self.abort_verify(now, "treads left the inventory");
                    self.pending = 0;
                    self.cast = None;
                    self.heal = None;
                    self.suspect(now, "treads moved out of the inventory");
                }
                return 0;
            }
            TreadsSlot::Active => {}
        }

        if !obs.alive {
            if self.pending > 0 || self.verify.is_some() {
                self.abort_verify(now, "died");
                self.pending = 0;
                self.suspect(now, "died with presses in flight");
            }
            self.cast = None;
            self.heal = None;
            return 0;
        }

        let delta = (
            obs.max_health - prev.max_health,
            obs.max_mana - prev.max_mana,
        );
        let confounded = obs.level != prev.level
            || obs.inventory_signature != prev.inventory_signature
            || !prev.alive
            || prev.treads != TreadsSlot::Active;

        self.track_rates(&prev, obs, delta);

        if self.verify.is_some() {
            return self.step_verify(obs, delta, confounded);
        }

        if delta != (0, 0) {
            self.on_change(now, delta, confounded, obs.in_danger);
        } else if self.pending > 0
            && now.saturating_sub(self.pending_since_ms) >= self.config.confirm_timeout_ms
        {
            self.on_silent_timeout(now);
        }
        if self.pending == 0 && self.manual_pending > 0 {
            self.manual_pending = 0;
            // Suspect is fine here: the estimate followed the player's own key,
            // and a verify is about to confirm it.
            if self.config.manual_switch_sets_home
                && self.cast.is_none()
                && self.heal.is_none()
                && !obs.in_danger
                && self.estimate.is_some()
            {
                self.home = self.estimate;
                self.note(now, format!("manual switch — home is now {:?}", self.home));
            }
        }

        self.update_cast(&prev, obs, delta);
        self.update_heal(&prev, obs);

        if self.pending > 0 || !self.config.enabled || !Self::can_press(obs) {
            return 0;
        }

        if self.confidence == Confidence::Confident {
            if let (Some(current), Some(home)) = (self.estimate, self.home) {
                // STR has the most HP on every hero — it is how verify tells
                // the attributes apart.
                let desired = if self.config.danger_switch && obs.in_danger {
                    Attr::Str
                } else {
                    match (&self.cast, &self.heal) {
                        (Some(cast), _) => cast.target,
                        (None, Some(heal)) => heal.target,
                        (None, None) => home,
                    }
                };
                if current != desired {
                    let presses = current.presses_to(desired);
                    self.pending = presses;
                    self.pending_since_ms = now;
                    self.note(
                        now,
                        format!(
                            "move {} → {} ({} press)",
                            current.label(),
                            desired.label(),
                            presses
                        ),
                    );
                    return presses;
                }
            }
        }

        let periodic = self.config.verify_interval_ms > 0
            && now.saturating_sub(self.last_verify_ms) >= self.config.verify_interval_ms;
        let due = self.confidence != Confidence::Confident || !self.offsets_complete() || periodic;
        if self.config.auto_verify
            && due
            && now >= self.verify_not_before_ms
            && self.verify_safe(obs)
        {
            return self.start_verify(now);
        }
        0
    }

    fn on_change(&mut self, now: u64, delta: (i64, i64), confounded: bool, in_danger: bool) {
        if confounded {
            if self.pending > 0 {
                self.pending = 0;
                self.suspect(now, "switch landed on a level-up / inventory tick");
            }
            return;
        }

        let Some(estimate) = self.estimate else {
            // Unknown: a change we can pin to exactly one switch is a free resync.
            let exact: Vec<(Attr, Attr)> = Attr::ALL
                .iter()
                .flat_map(|f| Attr::ALL.iter().map(move |t| (*f, *t)))
                .filter(|(f, t)| f != t && self.transition(*f, *t) == Some(delta))
                .collect();
            if exact.len() == 1 {
                self.estimate = Some(exact[0].1);
                self.confidence = Confidence::Confident;
                self.note(now, format!("resync from a switch: {}", exact[0].1.label()));
            } else {
                let seeded: Vec<(Attr, Attr)> = Attr::ALL
                    .iter()
                    .flat_map(|f| Attr::ALL.iter().map(move |t| (*f, *t)))
                    .filter(|(f, t)| f != t && seed_signature(*f, *t) == signs(delta))
                    .collect();
                if seeded.len() == 1 {
                    // Unprompted, a sign match could be a buff (OD's +1250
                    // mana). Right after a press we saw go out, it is the press.
                    let (from, to) = seeded[0];
                    let prompted = self.pending > 0
                        && now.saturating_sub(self.pending_since_ms)
                            <= self.config.confirm_timeout_ms;
                    self.estimate = Some(to);
                    if prompted {
                        self.confidence = Confidence::Confident;
                        self.learn(now, from, to, delta);
                        self.note(now, format!("resync from a press: {}", to.label()));
                    } else {
                        self.confidence = Confidence::Suspect;
                        self.note(now, format!("probably {} (by sign only)", to.label()));
                    }
                }
            }
            self.pending = 0;
            return;
        };

        let fresh = self.pending > 0
            && now.saturating_sub(self.pending_since_ms) <= self.config.confirm_timeout_ms;
        match self.explain(delta, fresh) {
            Some(count) => {
                let to = estimate.advance((count % 3) as u8);
                let exact = self.transition(estimate, to).is_some();
                self.learn(now, estimate, to, delta);
                self.estimate = Some(to);
                if count <= self.pending as u32 {
                    self.pending -= count as u8;
                    self.pending_since_ms = now;
                } else {
                    self.note(
                        now,
                        format!("{} unannounced press(es)", count - self.pending as u32),
                    );
                    self.pending = 0;
                    // A switch the player made with the mouse while idle is as
                    // deliberate as one made with the key.
                    if self.config.manual_switch_sets_home
                        && self.cast.is_none()
                        && self.heal.is_none()
                        && !in_danger
                    {
                        self.home = Some(to);
                        self.note(now, format!("player switched — home is now {}", to.label()));
                    }
                }
                // Matched by sign or by press count only: a buff landing in the
                // same tick looks the same. Verify before trusting it, unless
                // verify is off and nothing else ever could.
                if !exact && self.config.auto_verify {
                    self.suspect(now, "matched before the switch sizes were learned");
                }
            }
            None => {
                self.pending = 0;
                self.suspect(now, &format!("unexplained change {:?}", delta));
            }
        }
    }

    fn on_silent_timeout(&mut self, now: u64) {
        let Some(estimate) = self.estimate else {
            self.pending = 0;
            return;
        };
        let to = estimate.advance(self.pending % 3);
        if self.pending % 3 == 0 || self.transition(estimate, to) == Some((0, 0)) {
            self.estimate = Some(to);
            self.pending = 0;
            return;
        }
        self.pending = 0;
        self.suspect(now, "press(es) showed nothing — dropped");
    }

    fn step_verify(&mut self, obs: &Observation, delta: (i64, i64), confounded: bool) -> u8 {
        let now = obs.now_ms;
        if confounded {
            self.fail_verify(now, "level-up / inventory change mid-verify");
            return 0;
        }
        let Some(verify) = self.verify.as_mut() else {
            return 0;
        };
        if verify.awaiting {
            if delta != (0, 0) {
                verify.deltas.push(delta);
                verify.awaiting = false;
            } else if now.saturating_sub(verify.step_sent_ms) >= self.config.confirm_timeout_ms {
                verify.deltas.push((0, 0));
                verify.awaiting = false;
            } else {
                return 0;
            }
            self.pending = 0;
        } else if delta != (0, 0) {
            self.fail_verify(now, "change between verify steps");
            return 0;
        }

        if self.verify.as_ref().is_some_and(|v| v.deltas.len() == 3) {
            self.finish_verify(now);
            return 0;
        }
        if !self.verify_safe(obs) {
            self.abort_verify(now, "no longer safe");
            return 0;
        }
        if let Some(verify) = self.verify.as_mut() {
            verify.awaiting = true;
            verify.step_sent_ms = now;
        }
        self.pending = 1;
        self.pending_since_ms = now;
        1
    }

    fn update_cast(&mut self, prev: &Observation, obs: &Observation, delta: (i64, i64)) {
        let now = obs.now_ms;
        let linger = self.config.cast_linger_ms;
        let timeout = self.config.cast_timeout_ms;
        let Some(cast) = self.cast.as_mut() else {
            return;
        };
        // A switch preserves the mana fraction, so the mana we should see
        // without a spend is the old fraction of the new pool.
        let expected = if delta.1 != 0 && prev.max_mana > 0 {
            prev.mana * obs.max_mana / prev.max_mana
        } else {
            prev.mana
        };
        if obs.mana + 2 < expected && cast.spent_ms.is_none() {
            cast.spent_ms = Some(now);
        }
        let done = match cast.spent_ms {
            Some(spent) => now.saturating_sub(spent) >= linger,
            None => now.saturating_sub(cast.last_key_ms) >= timeout,
        };
        if done {
            self.cast = None;
            self.note(now, String::from("cast window closed"));
        }
    }

    fn track_rates(&mut self, prev: &Observation, obs: &Observation, delta: (i64, i64)) {
        let seconds = obs.now_ms.saturating_sub(prev.now_ms) as f64 / 1000.0;
        if seconds <= 0.0 || self.heal.is_some() || delta != (0, 0) {
            return;
        }
        let hp_rate = (obs.health_fraction() - prev.health_fraction()) / seconds;
        let mana_rate = (obs.mana_fraction() - prev.mana_fraction()) / seconds;
        if hp_rate >= 0.0 {
            self.hp_rate_baseline = self.hp_rate_baseline * 0.7 + hp_rate * 0.3;
        }
        if mana_rate >= 0.0 {
            self.mana_rate_baseline = self.mana_rate_baseline * 0.7 + mana_rate * 0.3;
        }
    }

    fn update_heal(&mut self, prev: &Observation, obs: &Observation) {
        let now = obs.now_ms;
        let Some(heal) = self.heal.as_mut() else {
            return;
        };
        let seconds = now.saturating_sub(prev.now_ms) as f64 / 1000.0;
        if seconds <= 0.0 {
            return;
        }
        // Fractions survive a switch unchanged, so they are the only rates
        // comparable across one.
        let hp_rate = (obs.health_fraction() - prev.health_fraction()) / seconds;
        let mana_rate = (obs.mana_fraction() - prev.mana_fraction()) / seconds;
        let hp_rising = hp_rate > self.hp_rate_baseline * 2.0 + 0.002;
        let mana_rising = mana_rate > self.mana_rate_baseline * 2.0 + 0.002;
        let rising = match heal.kind {
            HealKind::Health => hp_rising,
            HealKind::Mana => mana_rising,
            HealKind::Both => hp_rising || mana_rising,
        };
        if rising {
            heal.last_rising_ms = now;
        }
        let full = match heal.kind {
            HealKind::Health => obs.health >= obs.max_health,
            HealKind::Mana => obs.mana >= obs.max_mana,
            HealKind::Both => obs.health >= obs.max_health && obs.mana >= obs.max_mana,
        };
        let damaged = obs.health_fraction() + 0.002 < prev.health_fraction();
        let reason = if damaged {
            Some("took damage")
        } else if obs.in_danger {
            Some("danger")
        } else if full {
            Some("pool full")
        } else if now.saturating_sub(heal.started_ms) >= self.config.heal_max_ms {
            Some("heal_max_ms")
        } else if now.saturating_sub(heal.started_ms) >= self.config.heal_grace_ms
            && now.saturating_sub(heal.last_rising_ms) >= self.config.heal_settle_ms
        {
            Some("stopped rising")
        } else {
            None
        };
        if let Some(reason) = reason {
            self.heal = None;
            self.note(now, format!("heal window closed: {reason}"));
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Runtime glue: GSI in, key presses out.
// ═══════════════════════════════════════════════════════════════════════════

struct Runtime {
    machine: TreadsMachine,
    /// The binding for whichever slot holds the treads, if one of slot0-5.
    treads_key: Option<KeyBinding>,
}

static RUNTIME: LazyLock<Mutex<Runtime>> = LazyLock::new(|| {
    Mutex::new(Runtime {
        machine: TreadsMachine::new(PowerTreadsConfig::default()),
        treads_key: None,
    })
});

static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

fn now_ms() -> u64 {
    EPOCH.elapsed().as_millis() as u64
}

/// Treads presses to send ahead of a key the player pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreadsPrefix {
    pub binding: KeyBinding,
    pub presses: u8,
    pub gap_ms: u64,
}

fn locate_treads(
    items: &crate::models::gsi_event::Items,
    settings: &Settings,
) -> (TreadsSlot, Option<KeyBinding>) {
    let active = [
        ("slot0", &items.slot0),
        ("slot1", &items.slot1),
        ("slot2", &items.slot2),
        ("slot3", &items.slot3),
        ("slot4", &items.slot4),
        ("slot5", &items.slot5),
    ];
    if let Some((slot, _)) = active.iter().find(|(_, item)| item.name == POWER_TREADS) {
        return (TreadsSlot::Active, settings.get_key_for_slot(slot));
    }
    let inactive = [
        &items.slot6,
        &items.slot7,
        &items.slot8,
        &items.stash0,
        &items.stash1,
        &items.stash2,
        &items.stash3,
        &items.stash4,
        &items.stash5,
    ];
    if inactive.iter().any(|item| item.name == POWER_TREADS) {
        return (TreadsSlot::Inactive, None);
    }
    (TreadsSlot::Missing, None)
}

fn inventory_signature(items: &crate::models::gsi_event::Items) -> u64 {
    let mut hasher = DefaultHasher::new();
    for item in [
        &items.slot0,
        &items.slot1,
        &items.slot2,
        &items.slot3,
        &items.slot4,
        &items.slot5,
        &items.slot6,
        &items.slot7,
        &items.slot8,
        &items.neutral0,
        &items.stash0,
        &items.stash1,
        &items.stash2,
        &items.stash3,
        &items.stash4,
        &items.stash5,
    ] {
        item.name.hash(&mut hasher);
    }
    hasher.finish()
}

pub fn observation_from_event(
    event: &GsiWebhookEvent,
    treads: TreadsSlot,
    now_ms: u64,
    in_danger: bool,
) -> Observation {
    let hero = &event.hero;
    Observation {
        now_ms,
        hero: hero.name.clone(),
        primary: Attr::for_hero(&hero.name),
        alive: hero.alive,
        level: hero.level,
        health: hero.health as i64,
        max_health: hero.max_health as i64,
        mana: hero.mana as i64,
        max_mana: hero.max_mana as i64,
        stunned: hero.stunned,
        hexed: hero.hexed,
        muted: hero.muted,
        treads,
        inventory_signature: inventory_signature(&event.items),
        in_danger,
    }
}

fn send_presses(binding: KeyBinding, presses: u8, gap_ms: u64) {
    std::thread::spawn(move || {
        for index in 0..presses {
            if index > 0 {
                std::thread::sleep(Duration::from_millis(gap_ms));
            }
            crate::input::simulation::press_binding(&binding);
        }
    });
}

/// Feed one GSI event to the machine, and press whatever it asks for.
///
/// Runs on every event, even with the feature off: the state has to be
/// tracked from the purchase onward to be known when the switch is flipped on.
pub fn observe_event(event: &GsiWebhookEvent, settings: &Settings) {
    let (treads, key) = locate_treads(&event.items, settings);
    let obs = observation_from_event(
        event,
        treads,
        now_ms(),
        crate::actions::danger_detector::is_in_danger(),
    );

    let mut runtime = RUNTIME.lock().unwrap();
    runtime.machine.config = settings.power_treads.clone();
    runtime.treads_key = key.clone();
    let presses = runtime.machine.on_tick(&obs);
    drop(runtime);

    if presses > 0 {
        if let Some(binding) = key {
            info!("👟 Power Treads: {} press(es)", presses);
            send_presses(binding, presses, settings.power_treads.press_gap_ms);
        }
    }
}

/// What the key the player just pressed is about to do, as far as treads care.
fn intent_for_key(binding: &KeyBinding) -> Option<Intent> {
    let state = crate::actions::soul_ring::SOUL_RING_STATE.lock().unwrap();
    if let KeyBinding::Char(key_char) = binding {
        // A silence drops ability presses, so there is nothing to prepare for.
        if !state.cast_blocked && state.ability_spend_for_key(*key_char).warrants_soul_ring() {
            return Some(Intent::SpendMana);
        }
    }
    let item = state.get_item_for_key(binding)?;
    if let Some(kind) = heal_kind_for_item(item) {
        return Some(Intent::Heal(kind));
    }
    state
        .item_spend_for_key(binding)
        .warrants_soul_ring()
        .then_some(Intent::SpendMana)
}

/// Called by the keyboard hook for every key the player presses. Returns the
/// treads presses to send before replaying the key, if any.
///
/// The treads key itself is reported to the machine and passed through.
pub fn on_key_press(binding: &KeyBinding) -> Option<TreadsPrefix> {
    {
        let mut runtime = RUNTIME.lock().unwrap();
        if runtime.treads_key.as_ref() == Some(binding) {
            runtime.machine.on_manual_press(now_ms());
            return None;
        }
        if !runtime.machine.config.enabled {
            return None;
        }
    }

    // Soul Ring's state is read without ours held, so the two locks are never
    // taken in both orders.
    let intent = intent_for_key(binding)?;

    let mut runtime = RUNTIME.lock().unwrap();
    let presses = runtime.machine.on_key(now_ms(), intent);
    let treads_key = runtime.treads_key.clone()?;
    (presses > 0).then(|| TreadsPrefix {
        binding: treads_key,
        presses,
        gap_ms: runtime.machine.config.press_gap_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PowerTreadsConfig {
        PowerTreadsConfig {
            enabled: true,
            ..PowerTreadsConfig::default()
        }
    }

    /// An AGI hero, treads active, full pools, at `now`.
    fn obs(now_ms: u64, max_health: i64, max_mana: i64) -> Observation {
        Observation {
            now_ms,
            hero: String::from("npc_dota_hero_slark"),
            primary: Attr::Agi,
            alive: true,
            level: 20,
            health: max_health,
            max_health,
            mana: max_mana,
            max_mana,
            stunned: false,
            hexed: false,
            muted: false,
            treads: TreadsSlot::Active,
            inventory_signature: 1,
            in_danger: false,
        }
    }

    /// Slark buys treads at t=0 and sits on AGI (2400 HP / 1200 mana).
    fn bought_on_agi() -> TreadsMachine {
        let mut machine = TreadsMachine::new(config());
        let mut before = obs(0, 2400, 1200);
        before.treads = TreadsSlot::Missing;
        before.inventory_signature = 0;
        machine.on_tick(&before);
        machine.on_tick(&obs(200, 2400, 1200));
        machine
    }

    #[test]
    fn treads_cycle_str_int_agi() {
        assert_eq!(Attr::Str.advance(1), Attr::Int);
        assert_eq!(Attr::Int.advance(1), Attr::Agi);
        assert_eq!(Attr::Agi.advance(1), Attr::Str);
        assert_eq!(Attr::Agi.presses_to(Attr::Int), 2);
        assert_eq!(Attr::Int.presses_to(Attr::Int), 0);
    }

    #[test]
    fn purchase_starts_on_the_primary_attribute() {
        let machine = bought_on_agi();
        assert_eq!(machine.estimate, Some(Attr::Agi));
        assert_eq!(machine.confidence, Confidence::Confident);
        assert_eq!(machine.home, Some(Attr::Agi));
    }

    #[test]
    fn universal_and_unknown_heroes_start_on_str() {
        assert_eq!(Attr::for_hero("npc_dota_hero_void_spirit"), Attr::Str);
        assert_eq!(Attr::for_hero("npc_dota_hero_not_released_yet"), Attr::Str);
        assert_eq!(
            Attr::for_hero("npc_dota_hero_obsidian_destroyer"),
            Attr::Int
        );
    }

    #[test]
    fn a_cast_from_agi_takes_two_presses_to_int() {
        let mut machine = bought_on_agi();
        assert_eq!(machine.on_key(300, Intent::SpendMana), 2);
    }

    #[test]
    fn master_switch_off_presses_nothing() {
        let mut machine = bought_on_agi();
        machine.config.enabled = false;
        assert_eq!(machine.on_key(300, Intent::SpendMana), 0);
    }

    #[test]
    fn cast_switch_off_leaves_casts_alone() {
        let mut machine = bought_on_agi();
        machine.config.cast_switch = false;
        assert_eq!(machine.on_key(300, Intent::SpendMana), 0);
    }

    #[test]
    fn a_cast_in_danger_stays_put_when_danger_switch_is_on() {
        let mut machine = bought_on_agi();
        let mut danger = obs(400, 2400, 1200);
        danger.in_danger = true;
        // The tick itself moves to STR.
        assert_eq!(machine.on_tick(&danger), 1);
        assert_eq!(machine.on_key(450, Intent::SpendMana), 0);
    }

    #[test]
    fn an_unexplained_buff_does_not_move_a_learned_estimate() {
        let mut machine = bought_on_agi();
        machine.offsets = [Some((0, 0)), Some((-220, 120)), Some((-220, 0))];
        // OD's Astral Imprisonment: +1250 max mana, no press.
        machine.on_tick(&obs(400, 2400, 2450));
        assert_eq!(machine.estimate, Some(Attr::Agi));
        assert_eq!(machine.confidence, Confidence::Suspect);
    }

    #[test]
    fn a_sign_only_match_is_suspect_until_verified() {
        let mut machine = bought_on_agi();
        assert_eq!(machine.on_key(300, Intent::SpendMana), 2);
        // AGI → INT arrives, but nothing is learned yet: sign match only.
        machine.on_tick(&obs(500, 2400, 1320));
        assert_eq!(machine.estimate, Some(Attr::Int));
        assert_eq!(machine.confidence, Confidence::Suspect);
    }

    #[test]
    fn verify_reads_an_ordinary_hero_from_agi() {
        let mut machine = TreadsMachine::new(config());
        machine.on_tick(&obs(0, 2400, 1200));
        // Treads owned at startup, state unknown; idle long enough to verify.
        assert_eq!(machine.on_tick(&obs(4000, 2400, 1200)), 1);
        assert_eq!(machine.on_tick(&obs(4250, 2620, 1200)), 1); // AGI → STR
        assert_eq!(machine.on_tick(&obs(4500, 2400, 1320)), 1); // STR → INT
        assert_eq!(machine.on_tick(&obs(4750, 2400, 1200)), 0); // INT → AGI
        assert_eq!(machine.estimate, Some(Attr::Agi));
        assert_eq!(machine.confidence, Confidence::Confident);
        assert!(machine.offsets_complete());
    }

    #[test]
    fn verify_reads_ogre_magi_through_the_invisible_step() {
        let mut machine = TreadsMachine::new(config());
        let ogre = |now, hp, mana| {
            let mut o = obs(now, hp, mana);
            o.hero = String::from("npc_dota_hero_ogre_magi");
            o.primary = Attr::Str;
            o
        };
        machine.on_tick(&ogre(0, 2400, 1200));
        // Truth: INT. INT → AGI moves nothing, AGI → STR adds HP and mana.
        assert_eq!(machine.on_tick(&ogre(4000, 2400, 1200)), 1);
        assert_eq!(machine.on_tick(&ogre(4250, 2400, 1200)), 0); // step not visible yet
        assert_eq!(machine.on_tick(&ogre(4800, 2400, 1200)), 1); // timed out: zero step
        assert_eq!(machine.on_tick(&ogre(5050, 2620, 1260)), 1); // AGI → STR
        assert_eq!(machine.on_tick(&ogre(5300, 2400, 1200)), 0); // STR → INT
        assert_eq!(machine.estimate, Some(Attr::Int));
        assert_eq!(machine.confidence, Confidence::Confident);
        // Most mana is STR for Ogre: casting from STR moves nothing.
        assert_eq!(machine.most_mana(Attr::Str), Attr::Str);
    }

    #[test]
    fn a_verify_that_does_not_return_to_the_start_fails() {
        let mut machine = TreadsMachine::new(config());
        machine.on_tick(&obs(0, 2400, 1200));
        assert_eq!(machine.on_tick(&obs(4000, 2400, 1200)), 1);
        assert_eq!(machine.on_tick(&obs(4250, 2620, 1200)), 1);
        // Second press dropped: timed out as a zero step.
        assert_eq!(machine.on_tick(&obs(5000, 2620, 1200)), 1);
        assert_eq!(machine.on_tick(&obs(5250, 2400, 1320)), 0);
        assert_eq!(machine.estimate, None);
        assert_eq!(machine.confidence, Confidence::Unknown);
    }

    #[test]
    fn heal_targets_the_smallest_pool() {
        let mut machine = TreadsMachine::new(config());
        machine.offsets = [Some((0, 0)), Some((-220, 120)), Some((-220, 0))];
        assert_eq!(machine.least_pool(HealKind::Both, Attr::Str), Attr::Agi);
        assert_eq!(machine.least_pool(HealKind::Health, Attr::Str), Attr::Int);
        assert_eq!(machine.least_pool(HealKind::Mana, Attr::Int), Attr::Agi);
    }

    #[test]
    fn heal_items_are_classified() {
        assert_eq!(heal_kind_for_item("item_bottle"), Some(HealKind::Both));
        assert_eq!(heal_kind_for_item("item_flask"), Some(HealKind::Health));
        assert_eq!(heal_kind_for_item("item_clarity"), Some(HealKind::Mana));
        assert_eq!(heal_kind_for_item("item_mekansm"), None);
        assert_eq!(heal_kind_for_item("item_power_treads"), None);
    }

    #[test]
    fn low_hp_keeps_a_str_hero_on_str_for_a_cast() {
        let mut machine = TreadsMachine::new(config());
        let es = |now, hp| {
            let mut o = obs(now, 2620, 1200);
            o.hero = String::from("npc_dota_hero_earth_spirit");
            o.primary = Attr::Str;
            o.health = hp;
            o
        };
        let mut before = es(0, 2620);
        before.treads = TreadsSlot::Missing;
        before.inventory_signature = 0;
        machine.on_tick(&before);
        machine.on_tick(&es(200, 2620));
        machine.on_tick(&es(400, 500));
        assert_eq!(machine.on_key(500, Intent::SpendMana), 0);
    }
}
