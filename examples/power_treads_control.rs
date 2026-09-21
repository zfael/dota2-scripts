//! Power Treads control loop — the real state machine from
//! `src/actions/power_treads.rs`, run against a simulated game. It proved the
//! loop before anything could press a key in a real one, and it is the re-test
//! after any change to the machine.
//!
//! Nothing in `src/` depends on this. Stage 2 of `docs/workflows/probe-first.md`.
//!
//! ## What the probe measured (`examples/power_treads_probe.rs`, 2026-09-21)
//!
//! Captures `logs/power_treads/probe_2026-09-21_14-0*.jsonl`, on Earth Spirit,
//! Slark, Ogre Magi, Void Spirit and Outworld Destroyer:
//!
//! | Finding | Value |
//! |---|---|
//! | Treads attribute in GSI | none — the item object is identical in all three states |
//! | Start state after purchase | STR→STR, AGI→AGI, INT→INT, universal→STR |
//! | One switch, ordinary hero | 220 max HP / 120 max mana, always |
//! | HP% / mana% across a switch | preserved, 46/46 |
//! | Press → change visible in GSI | 13–251ms; GSI tick 208–584ms, median ~228ms |
//! | Two presses inside one tick | read as one two-step switch, 3/3 |
//! | Ogre Magi | STR also grants mana (+60), INT grants none: INT and AGI look identical |
//! | Outworld Destroyer | Astral Imprisonment: +1250 max mana, no press — mimics AGI→INT by sign |
//! | Press during cast point / channel | does not cancel it (reported by the tester) |
//! | Press while stunned | does nothing (reported by the tester) |
//!
//! ## What those findings did to the design
//!
//! * **Exact deltas, not signs.** OD's buff has the sign of AGI→INT. A change
//!   only counts as a switch when it equals a learned switch vector exactly.
//!   Signs are only the seed used before anything is learned.
//! * **Learned per-hero offsets.** Ogre breaks the ordinary hero's pattern, so
//!   the machine learns each attribute's `(max_hp, max_mana)` offset relative
//!   to STR, from a verify cycle or from its own single presses. Targets come
//!   from the learned offsets: "most mana" is INT for most heroes and STR for
//!   Ogre.
//! * **Verify cycle.** Three presses, one per tick, back to the start. The
//!   attribute with the highest max HP is STR — the only assumption, true for
//!   every hero — and the fixed STR→INT→AGI order names the other two.
//!
//! ## Unmeasured, so modelled both ways
//!
//! * press latency: `fast` (5ms) and `slow` (90ms) to the server
//! * which statuses drop an item press: `lenient` (stun only, as reported) and
//!   `strict` (stun, hex, mute)
//! * heals: the Bottle / Salve steps were not captured. The world heals a flat
//!   amount per second into whichever pool is active, and Salve-like heals end
//!   on damage — the Dota rules, but not yet seen through GSI.
//!
//! Measured constants appear in the world below and nowhere in the machine.
//!
//! ```powershell
//! cargo run --example power_treads_control
//! cargo run --example power_treads_control -- --scenario ogre --verbose
//! ```

use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

/// `--raw-danger`: model the danger detector as it was before it rescaled HP
/// across a max-HP change. Reproduces the first real game's STR loop.
static RAW_DANGER: AtomicBool = AtomicBool::new(false);

// The machine is the real one from `src/` — the simulator exercises exactly
// what ships.
use dota2_scripts::actions::power_treads::{
    Attr, Confidence, HealKind, Intent, Observation, TreadsMachine, TreadsSlot,
};
use dota2_scripts::config::settings::PowerTreadsConfig as TreadsConfig;

// ═══════════════════════════════════════════════════════════════════════════
// The world — measured constants live here, and only here.
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug)]
struct HeroKind {
    name: &'static str,
    /// Primary attribute, universal already resolved to STR.
    primary: Attr,
    /// `(max_hp, max_mana)` the treads add in each state, indexed STR/INT/AGI.
    bonus: [(i64, i64); 3],
}

const ORDINARY: [(i64, i64); 3] = [(220, 0), (0, 120), (0, 0)];
const OGRE: [(i64, i64); 3] = [(220, 60), (0, 0), (0, 0)];

const fn hero(name: &'static str, primary: Attr) -> HeroKind {
    HeroKind {
        name,
        primary,
        bonus: ORDINARY,
    }
}

const EARTH_SPIRIT: HeroKind = hero("npc_dota_hero_earth_spirit", Attr::Str);
const SLARK: HeroKind = hero("npc_dota_hero_slark", Attr::Agi);
const OD: HeroKind = hero("npc_dota_hero_obsidian_destroyer", Attr::Int);
const VOID_SPIRIT: HeroKind = hero("npc_dota_hero_void_spirit", Attr::Str);
const OGRE_MAGI: HeroKind = HeroKind {
    name: "npc_dota_hero_ogre_magi",
    primary: Attr::Str,
    bonus: OGRE,
};

#[derive(Clone, Copy, Debug)]
struct HealSpec {
    kind: HealKind,
    hp_per_s: f64,
    mana_per_s: f64,
    duration_ms: u64,
    ends_on_damage: bool,
}

const BOTTLE: HealSpec = HealSpec {
    kind: HealKind::Both,
    hp_per_s: 40.0,
    mana_per_s: 26.0,
    duration_ms: 2700,
    ends_on_damage: false,
};

const SALVE: HealSpec = HealSpec {
    kind: HealKind::Health,
    hp_per_s: 50.0,
    mana_per_s: 0.0,
    duration_ms: 8000,
    ends_on_damage: true,
};

#[derive(Clone, Copy, Debug)]
enum Act {
    Cast {
        cost: i64,
        cast_point_ms: u64,
    },
    Heal(HealSpec),
    /// Treads key, seen by the keyboard hook.
    KeyPress,
    /// Treads clicked with the mouse: the machine is not told.
    Click,
    Stun(u64),
    Hex(u64),
    Damage(f64),
    BuffMaxMana(i64),
    LevelUp,
    Die,
    Respawn,
    Danger(bool),
    Buy,
    ToBackpack,
    FromBackpack,
    Mark(&'static str),
}

#[derive(Clone, Copy, Debug)]
enum Input {
    Treads,
    Cast { cost: i64, cast_point_ms: u64 },
    Heal(HealSpec),
}

#[derive(Clone, Copy, Debug)]
struct CastRecord {
    landed: bool,
    attr: Attr,
}

struct HealEffect {
    spec: HealSpec,
    until_ms: u64,
}

struct World {
    kind: HeroKind,
    truth: Attr,
    treads: TreadsSlot,
    base_hp: i64,
    base_mana: i64,
    extra_hp: i64,
    extra_mana: i64,
    level: u32,
    hp: f64,
    mana: f64,
    alive: bool,
    stunned_until: u64,
    hexed_until: u64,
    danger: bool,
    inventory_signature: u64,
    strict_drops: bool,
    inflight: Vec<(u64, Input)>,
    casting: Vec<(u64, i64)>,
    heals: Vec<HealEffect>,
    casts: Vec<CastRecord>,
    /// HP and mana healed while in each attribute.
    healed: [(f64, f64); 3],
    presses_dropped: u32,
}

impl World {
    fn new(kind: HeroKind, owned: Option<Attr>, strict_drops: bool) -> Self {
        let mut world = Self {
            kind,
            truth: owned.unwrap_or(kind.primary),
            treads: if owned.is_some() {
                TreadsSlot::Active
            } else {
                TreadsSlot::Missing
            },
            base_hp: 2400,
            base_mana: 1200,
            extra_hp: 0,
            extra_mana: 0,
            level: 20,
            hp: 0.0,
            mana: 0.0,
            alive: true,
            stunned_until: 0,
            hexed_until: 0,
            danger: false,
            inventory_signature: 1,
            strict_drops,
            inflight: Vec::new(),
            casting: Vec::new(),
            heals: Vec::new(),
            casts: Vec::new(),
            healed: [(0.0, 0.0); 3],
            presses_dropped: 0,
        };
        world.hp = world.max_hp() as f64;
        world.mana = world.max_mana() as f64;
        world
    }

    fn bonus(&self) -> (i64, i64) {
        if self.treads == TreadsSlot::Active {
            self.kind.bonus[self.truth.index()]
        } else {
            (0, 0)
        }
    }

    fn max_hp(&self) -> i64 {
        self.base_hp + self.extra_hp + self.bonus().0
    }

    fn max_mana(&self) -> i64 {
        self.base_mana + self.extra_mana + self.bonus().1
    }

    /// Change something that moves the treads bonus. Measured: the fraction
    /// of both pools is preserved.
    fn scaled(&mut self, change: impl FnOnce(&mut World)) {
        let (old_hp, old_mana) = (self.max_hp() as f64, self.max_mana() as f64);
        change(self);
        if old_hp > 0.0 {
            self.hp *= self.max_hp() as f64 / old_hp;
        }
        if old_mana > 0.0 {
            self.mana *= self.max_mana() as f64 / old_mana;
        }
    }

    fn items_blocked(&self, now: u64) -> bool {
        !self.alive || now < self.stunned_until || (self.strict_drops && now < self.hexed_until)
    }

    fn step(&mut self, now: u64, dt_ms: u64) {
        let mut arrived = Vec::new();
        self.inflight.retain(|(at, input)| {
            if *at <= now {
                arrived.push(*input);
                false
            } else {
                true
            }
        });
        for input in arrived {
            match input {
                Input::Treads => {
                    if self.items_blocked(now) || self.treads != TreadsSlot::Active {
                        self.presses_dropped += 1;
                    } else {
                        self.scaled(|w| w.truth = w.truth.advance(1));
                    }
                }
                Input::Cast {
                    cost,
                    cast_point_ms,
                } => {
                    // Spells: stun and hex both stop them, in either model.
                    if !self.alive || now < self.stunned_until || now < self.hexed_until {
                        self.casts.push(CastRecord {
                            landed: false,
                            attr: self.truth,
                        });
                    } else {
                        self.casting.push((now + cast_point_ms, cost));
                    }
                }
                Input::Heal(spec) => {
                    if !self.items_blocked(now) {
                        self.heals.push(HealEffect {
                            spec,
                            until_ms: now + spec.duration_ms,
                        });
                    }
                }
            }
        }

        let mut resolved = Vec::new();
        self.casting.retain(|(at, cost)| {
            if *at <= now {
                resolved.push(*cost);
                false
            } else {
                true
            }
        });
        for cost in resolved {
            let landed = self.alive && self.mana >= cost as f64;
            if landed {
                self.mana -= cost as f64;
            }
            self.casts.push(CastRecord {
                landed,
                attr: self.truth,
            });
        }

        if !self.alive {
            return;
        }
        let seconds = dt_ms as f64 / 1000.0;
        let (max_hp, max_mana) = (self.max_hp() as f64, self.max_mana() as f64);
        let mut hp_gain = 3.0 * seconds;
        let mut mana_gain = 2.0 * seconds;
        self.heals.retain(|heal| heal.until_ms > now);
        for heal in &self.heals {
            hp_gain += heal.spec.hp_per_s * seconds;
            mana_gain += heal.spec.mana_per_s * seconds;
        }
        let before = (self.hp, self.mana);
        self.hp = (self.hp + hp_gain).min(max_hp);
        self.mana = (self.mana + mana_gain).min(max_mana);
        if !self.heals.is_empty() {
            let slot = &mut self.healed[self.truth.index()];
            slot.0 += self.hp - before.0;
            slot.1 += self.mana - before.1;
        }
    }

    fn apply(&mut self, now: u64, act: Act) {
        match act {
            Act::Stun(ms) => self.stunned_until = now + ms,
            Act::Hex(ms) => self.hexed_until = now + ms,
            Act::Damage(amount) => {
                self.hp = (self.hp - amount).max(1.0);
                self.heals.retain(|heal| !heal.spec.ends_on_damage);
            }
            Act::BuffMaxMana(amount) => self.extra_mana += amount,
            Act::LevelUp => {
                self.level += 1;
                self.extra_hp += 80;
                self.extra_mana += 50;
            }
            Act::Die => {
                self.alive = false;
                self.hp = 0.0;
                self.heals.clear();
                self.casting.clear();
            }
            Act::Respawn => {
                self.alive = true;
                self.hp = self.max_hp() as f64;
                self.mana = self.max_mana() as f64;
            }
            Act::Danger(on) => self.danger = on,
            Act::Buy => {
                self.inventory_signature += 1;
                let start = self.kind.primary;
                self.scaled(|w| {
                    w.treads = TreadsSlot::Active;
                    w.truth = start;
                });
            }
            Act::ToBackpack => {
                self.inventory_signature += 1;
                self.scaled(|w| w.treads = TreadsSlot::Inactive);
            }
            Act::FromBackpack => {
                self.inventory_signature += 1;
                self.scaled(|w| w.treads = TreadsSlot::Active);
            }
            Act::Cast { .. } | Act::Heal(_) | Act::KeyPress | Act::Click | Act::Mark(_) => {}
        }
    }

    fn observe(&self, now: u64) -> Observation {
        Observation {
            now_ms: now,
            hero: self.kind.name.to_string(),
            primary: self.kind.primary,
            alive: self.alive,
            level: self.level,
            health: self.hp.floor() as i64,
            max_health: self.max_hp(),
            mana: self.mana.floor() as i64,
            max_mana: self.max_mana(),
            stunned: now < self.stunned_until,
            hexed: now < self.hexed_until,
            muted: false,
            treads: self.treads,
            inventory_signature: self.inventory_signature,
            in_danger: self.danger,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Harness
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy)]
struct Model {
    label: &'static str,
    lag_ms: u64,
    strict_drops: bool,
}

const MODELS: [Model; 4] = [
    Model {
        label: "fast/lenient",
        lag_ms: 5,
        strict_drops: false,
    },
    Model {
        label: "fast/strict",
        lag_ms: 5,
        strict_drops: true,
    },
    Model {
        label: "slow/lenient",
        lag_ms: 90,
        strict_drops: false,
    },
    Model {
        label: "slow/strict",
        lag_ms: 90,
        strict_drops: true,
    },
];

struct Run {
    world: World,
    machine: TreadsMachine,
    key_presses: u32,
    tick_presses: u32,
    /// Ticks where the machine was confident, idle, and wrong.
    wrong_ticks: Vec<String>,
    marks: Vec<(&'static str, Attr, Option<Attr>, Confidence)>,
    /// Times the modelled danger detector fired.
    danger_trips: u32,
}

impl Run {
    fn mark(&self, label: &str) -> Option<(Attr, Option<Attr>, Confidence)> {
        self.marks
            .iter()
            .find(|(name, ..)| *name == label)
            .map(|(_, truth, estimate, confidence)| (*truth, *estimate, *confidence))
    }

    fn settled_on(&self, attr: Attr) -> bool {
        self.world.truth == attr
            && self.machine.estimate == Some(attr)
            && self.machine.confidence == Confidence::Confident
            && self.machine.pending() == 0
    }

    fn cast_attrs(&self) -> Vec<Attr> {
        self.world
            .casts
            .iter()
            .filter(|c| c.landed)
            .map(|c| c.attr)
            .collect()
    }
}

type Check = fn(&Run) -> Vec<(String, bool)>;

struct Scenario {
    name: &'static str,
    about: &'static str,
    hero: HeroKind,
    /// Treads already owned (and in this state) when the app starts.
    owned: Option<Attr>,
    duration_ms: u64,
    configure: fn(&mut TreadsConfig),
    actions: Vec<(u64, Act)>,
    check: Check,
}

fn keep_defaults(_: &mut TreadsConfig) {}

/// GSI tick intervals shaped like the capture: ~210–250ms, the odd ~450ms.
struct Ticks(u64);

impl Ticks {
    fn next_interval(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let roll = (self.0 >> 33) % 100;
        210 + (self.0 >> 40) % 40 + if roll < 6 { 230 } else { 0 }
    }
}

fn run(scenario: &Scenario, model: Model) -> Run {
    // The shipped default is off; every scenario runs with it on unless it
    // says otherwise.
    let mut config = TreadsConfig {
        enabled: true,
        ..TreadsConfig::default()
    };
    (scenario.configure)(&mut config);
    let gap = config.press_gap_ms;
    let mut world = World::new(scenario.hero, scenario.owned, model.strict_drops);
    let mut machine = TreadsMachine::new(config);
    let mut outbox: Vec<(u64, Input)> = Vec::new();
    let mut ticks = Ticks(0x5eed ^ scenario.name.len() as u64);
    let mut next_tick = 0u64;
    let mut key_presses = 0u32;
    let mut tick_presses = 0u32;
    let mut wrong_ticks = Vec::new();
    let mut marks = Vec::new();
    let mut previous_obs: Option<Observation> = None;
    let mut detector_until = 0u64;
    let mut danger_trips = 0u32;

    let mut actions = scenario.actions.clone();
    actions.sort_by_key(|(at, _)| *at);
    let mut next_action = 0;

    let step = 5u64;
    let mut now = 0u64;
    while now <= scenario.duration_ms {
        while next_action < actions.len() && actions[next_action].0 <= now {
            let act = actions[next_action].1;
            next_action += 1;
            match act {
                Act::Cast {
                    cost,
                    cast_point_ms,
                } => {
                    let presses = machine.on_key(now, Intent::SpendMana);
                    key_presses += presses as u32;
                    for i in 0..presses as u64 {
                        outbox.push((now + i * gap, Input::Treads));
                    }
                    outbox.push((
                        now + presses as u64 * gap,
                        Input::Cast {
                            cost,
                            cast_point_ms,
                        },
                    ));
                }
                Act::Heal(spec) => {
                    let presses = machine.on_key(now, Intent::Heal(spec.kind));
                    key_presses += presses as u32;
                    for i in 0..presses as u64 {
                        outbox.push((now + i * gap, Input::Treads));
                    }
                    outbox.push((now + presses as u64 * gap, Input::Heal(spec)));
                }
                Act::KeyPress => {
                    machine.on_manual_press(now);
                    outbox.push((now, Input::Treads));
                }
                Act::Click => outbox.push((now, Input::Treads)),
                Act::Mark(label) => {
                    marks.push((label, world.truth, machine.estimate, machine.confidence));
                }
                other => world.apply(now, other),
            }
        }

        outbox.retain(|(at, input)| {
            if *at <= now {
                world.inflight.push((now + model.lag_ms, *input));
                false
            } else {
                true
            }
        });

        world.step(now, step);

        if now >= next_tick {
            let mut obs = world.observe(now);
            // The danger detector, as src/actions/danger_detector.rs runs it:
            // a loss over 100 HP in one tick is danger for 3s. Scripted danger
            // alone could never show the loop the first real game hit — our
            // own switch out of STR reading as damage — so it is derived here.
            if let Some(previous) = &previous_obs {
                let comparable = if RAW_DANGER.load(Ordering::Relaxed)
                    || previous.max_health == obs.max_health
                    || previous.max_health <= 0
                {
                    previous.health
                } else {
                    previous.health * obs.max_health / previous.max_health
                };
                if obs.alive && previous.alive && comparable - obs.health > 100 {
                    if now >= detector_until {
                        danger_trips += 1;
                    }
                    detector_until = now + 3000;
                }
            }
            obs.in_danger = obs.in_danger || now < detector_until;
            previous_obs = Some(obs.clone());
            let presses = machine.on_tick(&obs);
            tick_presses += presses as u32;
            for i in 0..presses as u64 {
                outbox.push((now + i * gap, Input::Treads));
            }
            if machine.confidence == Confidence::Confident
                && machine.pending() == 0
                && machine.estimate != Some(world.truth)
                && world.treads == TreadsSlot::Active
                && world.alive
            {
                wrong_ticks.push(format!(
                    "{}ms: confident {:?}, truth {}",
                    now,
                    machine.estimate,
                    world.truth.label()
                ));
            }
            next_tick = now + ticks.next_interval();
        }
        now += step;
    }

    Run {
        world,
        machine,
        key_presses,
        tick_presses,
        wrong_ticks,
        marks,
        danger_trips,
    }
}

fn check(label: impl Into<String>, ok: bool) -> (String, bool) {
    (label.into(), ok)
}

fn never_wrong(run: &Run) -> (String, bool) {
    check(
        format!(
            "never confidently wrong{}",
            run.wrong_ticks
                .first()
                .map(|w| format!(" (first: {w})"))
                .unwrap_or_default()
        ),
        run.wrong_ticks.is_empty(),
    )
}

const CAST: Act = Act::Cast {
    cost: 150,
    cast_point_ms: 300,
};

fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "agi-cast",
            about: "AGI hero buys treads, casts: the cast lands on INT, then back to AGI",
            hero: SLARK,
            owned: None,
            duration_ms: 12_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (3000, CAST)],
            check: |r| {
                vec![
                    check("cast landed on INT", r.cast_attrs() == vec![Attr::Int]),
                    check("back on AGI", r.settled_on(Attr::Agi)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "str-cast",
            about: "STR hero: one press to INT, two back",
            hero: EARTH_SPIRIT,
            owned: None,
            duration_ms: 12_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (3000, CAST)],
            check: |r| {
                vec![
                    check("cast landed on INT", r.cast_attrs() == vec![Attr::Int]),
                    check("one press before the cast", r.key_presses == 1),
                    check("back on STR", r.settled_on(Attr::Str)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "int-cast",
            about: "INT hero: already there, no presses for the cast",
            hero: OD,
            owned: None,
            duration_ms: 12_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (3000, CAST)],
            check: |r| {
                vec![
                    check("cast landed on INT", r.cast_attrs() == vec![Attr::Int]),
                    check("no presses before the cast", r.key_presses == 0),
                    check("still on INT", r.settled_on(Attr::Int)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "lich-no-danger-loop",
            about: "first real game: INT hero, our own switch out of STR must not read as danger",
            hero: hero("npc_dota_hero_lich", Attr::Int),
            owned: None,
            duration_ms: 30_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (15_000, CAST)],
            check: |r| {
                vec![
                    check(
                        format!("danger detector never fired ({} trips)", r.danger_trips),
                        r.danger_trips == 0,
                    ),
                    check(
                        format!("only the learning verify pressed on ticks ({})", r.tick_presses),
                        r.tick_presses <= 3,
                    ),
                    check("cast landed on INT", r.cast_attrs() == vec![Attr::Int]),
                    check("on INT", r.settled_on(Attr::Int)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "universal",
            about: "universal hero starts and returns to STR",
            hero: VOID_SPIRIT,
            owned: None,
            duration_ms: 12_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (3000, CAST)],
            check: |r| {
                vec![
                    check("cast landed on INT", r.cast_attrs() == vec![Attr::Int]),
                    check("back on STR", r.settled_on(Attr::Str)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "chain",
            about: "three casts in a row all pay on INT, one round trip",
            hero: SLARK,
            owned: None,
            duration_ms: 12_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (3000, CAST), (3500, CAST), (4000, CAST)],
            check: |r| {
                vec![
                    check("all three on INT", r.cast_attrs() == vec![Attr::Int; 3]),
                    check("two presses before the first cast only", r.key_presses == 2),
                    check("back on AGI", r.settled_on(Attr::Agi)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "cast-during-revert",
            about: "a second cast arrives while the machine is walking back to AGI",
            hero: SLARK,
            owned: None,
            duration_ms: 14_000,
            configure: keep_defaults,
            actions: {
                let mut acts = vec![(500, Act::Buy), (3000, CAST)];
                // The revert fires somewhere after spend + linger; hit the
                // whole window so one of them lands mid-revert.
                for at in (4200..5400).step_by(120) {
                    acts.push((
                        at,
                        Act::Cast {
                            cost: 20,
                            cast_point_ms: 0,
                        },
                    ));
                }
                acts
            },
            check: |r| {
                vec![
                    check("every cast landed", r.world.casts.iter().all(|c| c.landed)),
                    check(
                        "most casts on INT",
                        r.cast_attrs().iter().filter(|a| **a == Attr::Int).count() >= 9,
                    ),
                    check("back on AGI", r.settled_on(Attr::Agi)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "manual-key",
            about: "the player presses the treads key: followed, and it becomes home",
            hero: EARTH_SPIRIT,
            owned: None,
            duration_ms: 12_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (3000, Act::KeyPress), (6000, CAST)],
            check: |r| {
                vec![
                    check("cast landed on INT", r.cast_attrs() == vec![Attr::Int]),
                    check("no presses for the cast", r.key_presses == 0),
                    check("stays on INT (new home)", r.settled_on(Attr::Int)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "click-mid-sequence",
            about: "an unannounced click lands while our presses are in flight",
            hero: SLARK,
            owned: None,
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (3000, CAST), (3010, Act::Click)],
            check: |r| {
                vec![
                    check("back on AGI", r.settled_on(Attr::Agi)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "click-idle",
            about: "a mouse click on the treads while idle is read and kept as the new home",
            hero: SLARK,
            owned: None,
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![
                (500, Act::Buy),
                (8000, Act::Click),
                (8600, Act::Mark("after click")),
            ],
            check: |r| {
                vec![
                    check(
                        "estimate followed the click",
                        r.mark("after click").is_some_and(|(t, e, _)| e == Some(t)),
                    ),
                    check("stays on STR", r.settled_on(Attr::Str)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "stun-drops-presses",
            about: "stun lands after the last tick: our presses and the cast are dropped",
            hero: SLARK,
            owned: None,
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (2990, Act::Stun(1500)), (3000, CAST)],
            check: |r| {
                vec![
                    check("ends on AGI, confident", r.settled_on(Attr::Agi)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "stun-visible",
            about: "stunned on the last tick: no presses at all",
            hero: SLARK,
            owned: None,
            duration_ms: 12_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (2000, Act::Stun(2000)), (3000, CAST)],
            check: |r| {
                vec![
                    check("no presses before the cast", r.key_presses == 0),
                    check("nothing dropped", r.world.presses_dropped == 0),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "hex-visible",
            about: "hexed: no presses (dropped in the strict model if we tried)",
            hero: SLARK,
            owned: None,
            duration_ms: 12_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (2000, Act::Hex(2500)), (3000, CAST)],
            check: |r| {
                vec![
                    check("nothing dropped", r.world.presses_dropped == 0),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "od-buff",
            about: "+1250 max mana from a spell while idle is not a switch",
            hero: OD,
            owned: None,
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![
                (500, Act::Buy),
                (8000, Act::BuffMaxMana(1250)),
                (15_000, Act::BuffMaxMana(-1250)),
            ],
            check: |r| {
                vec![
                    check("ends on INT, confident", r.settled_on(Attr::Int)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "buff-on-switch-tick",
            about: "a max-mana buff lands in the same tick as our switch",
            hero: SLARK,
            owned: None,
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (3000, CAST), (3010, Act::BuffMaxMana(300))],
            check: |r| {
                vec![
                    check("ends on AGI, confident", r.settled_on(Attr::Agi)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "level-on-switch-tick",
            about: "a level-up lands in the same tick as our switch",
            hero: SLARK,
            owned: None,
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (3000, CAST), (3010, Act::LevelUp)],
            check: |r| {
                vec![
                    check("ends on AGI, confident", r.settled_on(Attr::Agi)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "ogre",
            about: "Ogre Magi: offsets learned, then casts stay on STR (most mana)",
            hero: OGRE_MAGI,
            owned: None,
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (12_000, CAST)],
            check: |r| {
                vec![
                    check("cast landed on STR", r.cast_attrs() == vec![Attr::Str]),
                    check("no presses for the cast", r.key_presses == 0),
                    check("on STR", r.settled_on(Attr::Str)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "ogre-first-cast",
            about: "Ogre casts before anything is learned: one press, then it learns",
            hero: OGRE_MAGI,
            owned: None,
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (1500, CAST), (14_000, CAST)],
            check: |r| {
                vec![
                    check(
                        "second cast on STR",
                        r.cast_attrs().get(1) == Some(&Attr::Str),
                    ),
                    check("on STR", r.settled_on(Attr::Str)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "unknown-start",
            about: "app starts with treads already on STR: verify finds it, returns home",
            hero: SLARK,
            owned: Some(Attr::Str),
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![(12_000, CAST)],
            check: |r| {
                vec![
                    check("cast landed on INT", r.cast_attrs() == vec![Attr::Int]),
                    check("back on AGI", r.settled_on(Attr::Agi)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "unknown-start-ogre",
            about: "Ogre with treads on INT at startup: the invisible step does not fool verify",
            hero: OGRE_MAGI,
            owned: Some(Attr::Int),
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![],
            check: |r| vec![check("on STR", r.settled_on(Attr::Str)), never_wrong(r)],
        },
        Scenario {
            name: "verify-interrupted",
            about: "a cast arrives mid-verify on an unknown state",
            hero: SLARK,
            owned: Some(Attr::Str),
            duration_ms: 25_000,
            configure: keep_defaults,
            actions: vec![(3400, CAST)],
            check: |r| {
                vec![
                    check("ends on AGI, confident", r.settled_on(Attr::Agi)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "periodic-verify",
            about: "confident and idle: verify still runs on its interval",
            hero: EARTH_SPIRIT,
            owned: None,
            duration_ms: 40_000,
            configure: |c| c.verify_interval_ms = 10_000,
            actions: vec![(500, Act::Buy)],
            check: |r| {
                vec![
                    check(
                        "verified more than once",
                        r.machine
                            .log
                            .iter()
                            .filter(|l| l.contains("verified"))
                            .count()
                            >= 2,
                    ),
                    check("on STR", r.settled_on(Attr::Str)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "low-hp",
            about: "STR hero under 30% HP does not leave STR to cast",
            hero: EARTH_SPIRIT,
            owned: None,
            duration_ms: 12_000,
            configure: keep_defaults,
            actions: vec![(500, Act::Buy), (2000, Act::Damage(2000.0)), (3000, CAST)],
            check: |r| {
                vec![
                    check("cast landed on STR", r.cast_attrs() == vec![Attr::Str]),
                    check("no presses before the cast", r.key_presses == 0),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "disabled",
            about: "feature off: nothing is ever pressed",
            hero: SLARK,
            owned: None,
            duration_ms: 20_000,
            configure: |c| c.enabled = false,
            actions: vec![(500, Act::Buy), (3000, CAST), (6000, Act::Heal(BOTTLE))],
            check: |r| vec![check("zero presses", r.key_presses + r.tick_presses == 0)],
        },
        Scenario {
            name: "danger-str",
            about: "AGI hero in danger: STR while it lasts, casts stay there, AGI after",
            hero: SLARK,
            owned: None,
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![
                (500, Act::Buy),
                (5000, Act::Danger(true)),
                (6500, Act::Mark("in danger")),
                (7000, CAST),
                (9000, Act::Danger(false)),
            ],
            check: |r| {
                vec![
                    check(
                        "on STR during danger",
                        r.mark("in danger").is_some_and(|(t, _, _)| t == Attr::Str),
                    ),
                    check(
                        "cast in danger stayed on STR",
                        r.cast_attrs() == vec![Attr::Str],
                    ),
                    check("back on AGI", r.settled_on(Attr::Agi)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "danger-switch-off",
            about: "danger_switch off: danger moves nothing, the cast still goes to INT",
            hero: SLARK,
            owned: None,
            duration_ms: 20_000,
            configure: |c| c.danger_switch = false,
            actions: vec![
                (500, Act::Buy),
                (5000, Act::Danger(true)),
                (6500, Act::Mark("in danger")),
                (7000, CAST),
                (9000, Act::Danger(false)),
            ],
            check: |r| {
                vec![
                    check(
                        "still AGI in danger",
                        r.mark("in danger").is_some_and(|(t, _, _)| t == Attr::Agi),
                    ),
                    check("cast on INT", r.cast_attrs() == vec![Attr::Int]),
                    check("back on AGI", r.settled_on(Attr::Agi)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "cast-switch-off",
            about: "cast_switch off: casts are left alone",
            hero: SLARK,
            owned: None,
            duration_ms: 20_000,
            configure: |c| c.cast_switch = false,
            actions: vec![(500, Act::Buy), (8000, CAST)],
            check: |r| {
                vec![
                    check("cast on AGI", r.cast_attrs() == vec![Attr::Agi]),
                    check("no presses for the cast", r.key_presses == 0),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "heal-switch-off",
            about: "heal_switch off: a Bottle is left alone",
            hero: EARTH_SPIRIT,
            owned: None,
            duration_ms: 20_000,
            configure: |c| c.heal_switch = false,
            actions: vec![
                (500, Act::Buy),
                (6000, Act::Damage(1200.0)),
                (10_000, Act::Heal(BOTTLE)),
            ],
            check: |r| {
                vec![
                    check("no presses for the heal", r.key_presses == 0),
                    check("healed on STR", r.world.healed[Attr::Str.index()].0 > 0.0),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "auto-verify-off",
            about: "auto_verify off: unknown at startup stays hands-off until the player switches",
            hero: SLARK,
            owned: Some(Attr::Str),
            duration_ms: 20_000,
            configure: |c| {
                c.auto_verify = false;
                c.verify_interval_ms = 0;
            },
            actions: vec![(6000, CAST), (10_000, Act::KeyPress), (15_000, CAST)],
            check: |r| {
                vec![
                    check(
                        "first cast untouched on STR",
                        r.cast_attrs().first() == Some(&Attr::Str),
                    ),
                    check(
                        "no verify presses",
                        r.tick_presses == 0
                            || r.machine.log.iter().all(|l| !l.contains("verify: start")),
                    ),
                    check(
                        "second cast on INT once the switch was read",
                        r.cast_attrs().get(1) == Some(&Attr::Int),
                    ),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "death-mid-switch",
            about: "dies right after our presses go out",
            hero: SLARK,
            owned: None,
            duration_ms: 25_000,
            configure: keep_defaults,
            actions: vec![
                (500, Act::Buy),
                (3000, CAST),
                (3002, Act::Die),
                (8000, Act::Respawn),
            ],
            check: |r| {
                vec![
                    check("ends on AGI, confident", r.settled_on(Attr::Agi)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "backpack",
            about: "treads to the backpack and back",
            hero: SLARK,
            owned: None,
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![
                (500, Act::Buy),
                (6000, Act::ToBackpack),
                (8000, Act::FromBackpack),
            ],
            check: |r| {
                vec![
                    check("ends on AGI, confident", r.settled_on(Attr::Agi)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "bottle",
            about: "Bottle on a STR hero: healed on AGI, back to STR after",
            hero: EARTH_SPIRIT,
            owned: None,
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![
                (500, Act::Buy),
                (6000, Act::Damage(1200.0)),
                (10_000, Act::Heal(BOTTLE)),
            ],
            check: |r| {
                let total: f64 = r.world.healed.iter().map(|h| h.0).sum();
                let on_agi = r.world.healed[Attr::Agi.index()].0;
                vec![
                    check(
                        format!("≥75% of the heal on AGI ({:.0}/{:.0})", on_agi, total),
                        total > 0.0 && on_agi / total >= 0.75,
                    ),
                    check("back on STR", r.settled_on(Attr::Str)),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "salve-interrupted",
            about: "Salve on a STR hero, damage cancels it: back to STR promptly",
            hero: EARTH_SPIRIT,
            owned: None,
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![
                (500, Act::Buy),
                (6000, Act::Damage(1200.0)),
                (10_000, Act::Heal(SALVE)),
                (12_000, Act::Damage(100.0)),
                (13_500, Act::Mark("1.5s after the hit")),
            ],
            check: |r| {
                vec![
                    check(
                        "on STR 1.5s after the hit",
                        r.mark("1.5s after the hit")
                            .is_some_and(|(t, _, _)| t == Attr::Str),
                    ),
                    check(
                        "healed off STR before the hit",
                        r.world.healed[Attr::Int.index()].0 + r.world.healed[Attr::Agi.index()].0
                            > 0.0,
                    ),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "heal-in-danger",
            about: "no heal switch while the danger detector is firing",
            hero: EARTH_SPIRIT,
            owned: None,
            duration_ms: 12_000,
            configure: keep_defaults,
            actions: vec![
                (500, Act::Buy),
                (2000, Act::Damage(1200.0)),
                (2500, Act::Danger(true)),
                (3000, Act::Heal(BOTTLE)),
            ],
            check: |r| {
                vec![
                    check("no presses for the heal", r.key_presses == 0),
                    never_wrong(r),
                ]
            },
        },
        Scenario {
            name: "cast-during-heal",
            about: "a cast during a Bottle: INT for the cast, then back to the heal attribute",
            hero: EARTH_SPIRIT,
            owned: None,
            duration_ms: 20_000,
            configure: keep_defaults,
            actions: vec![
                (500, Act::Buy),
                (6000, Act::Damage(1200.0)),
                (10_000, Act::Heal(SALVE)),
                (10_500, CAST),
            ],
            check: |r| {
                vec![
                    check("cast landed on INT", r.cast_attrs() == vec![Attr::Int]),
                    check("back on STR", r.settled_on(Attr::Str)),
                    never_wrong(r),
                ]
            },
        },
    ]
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");
    RAW_DANGER.store(args.iter().any(|a| a == "--raw-danger"), Ordering::Relaxed);
    let filter = args
        .iter()
        .position(|a| a == "--scenario")
        .and_then(|i| args.get(i + 1))
        .cloned();

    let mut failures = 0;
    let mut total = 0;
    for scenario in scenarios() {
        if filter
            .as_ref()
            .is_some_and(|f| !scenario.name.contains(f.as_str()))
        {
            continue;
        }
        println!("\n▶ {} — {}", scenario.name, scenario.about);
        for model in MODELS {
            let run = run(&scenario, model);
            let results = (scenario.check)(&run);
            let passed = results.iter().all(|(_, ok)| *ok);
            total += 1;
            if !passed {
                failures += 1;
            }
            println!(
                "  {} {:<13} presses: {} for keys, {} on ticks; casts {:?}",
                if passed { "PASS" } else { "FAIL" },
                model.label,
                run.key_presses,
                run.tick_presses,
                run.cast_attrs()
                    .iter()
                    .map(|a| a.label())
                    .collect::<Vec<_>>()
            );
            for (label, ok) in &results {
                if !ok || verbose {
                    println!("       {} {}", if *ok { "✓" } else { "✗" }, label);
                }
            }
            if !passed || verbose {
                for line in &run.machine.log {
                    println!("         {line}");
                }
            }
        }
    }

    println!("\n{} / {} scenario runs passed", total - failures, total);
    if failures > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
