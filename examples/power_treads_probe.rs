//! Power Treads probe — can we tell which attribute the treads are on from
//! nothing but `max_health` and `max_mana`?
//!
//! Nothing in `src/` depends on this. GSI lists `item_power_treads` in the
//! inventory but, as far as the fixtures show, nothing that says STR, INT or
//! AGI. The automation's plan is to infer the state from how the hero's pools
//! move when the treads switch:
//!
//! | Switch | presses | `max_health` | `max_mana` |
//! |---|---|---|---|
//! | STR → INT | 1 | − | + |
//! | INT → AGI | 1 | 0 | − |
//! | AGI → STR | 1 | + | 0 |
//! | STR → AGI | 2 | − | 0 |
//! | INT → STR | 2 | + | − |
//! | AGI → INT | 2 | 0 | + |
//! | back to start | 3 | 0 | 0 |
//!
//! Every switch has its own sign pair, so one observed switch names both ends.
//! This probe checks that table against the game and answers what the table
//! cannot:
//!
//! 1. **Starting state.** Treads bought on a hero start on its primary
//!    attribute (the generated `hero_attributes.rs`)? What do universal heroes
//!    get?
//! 2. **Hidden state field.** Does any field of the treads item object change
//!    with the attribute? If one does, it beats all of this inference.
//! 3. **Percent preservation.** Does a switch keep `health_percent` and
//!    `mana_percent`, scaling current HP/mana with the pool? The cast trick and
//!    the heal trick both depend on it.
//! 4. **Merging.** Two quick presses between ticks — do they show up as one
//!    two-step signature? Do three show up as nothing?
//! 5. **Latency.** Treads key press → first tick showing the change.
//! 6. **Dropped presses.** Does a press while stunned / hexed / dead do nothing?
//!    A press that does nothing is how the state estimate goes wrong.
//! 7. **Confounders.** Level up, Vitality Booster, Energy Booster, Bracer —
//!    what do they look like next to a switch?
//! 8. **Casts.** Does pressing treads during a cast point or channel cancel the
//!    cast? The probe prints every ability cooldown `0 → N` edge; a cast that
//!    was cancelled starts no cooldown.
//!
//! The probe never presses anything. It listens.
//!
//! ## Capture protocol
//!
//! Run it on its own port next to the real app: drop a second file in
//! `steamapps/common/dota 2 beta/game/dota/cfg/gamestate_integration/`, say
//! `gamestate_integration_treads_probe.cfg`, identical to the app's own but
//! with `"uri" "http://127.0.0.1:3100/"` and its own `"name"`. (The Morphling
//! probe's cfg is already exactly that — reuse it.) With the app closed, pass
//! `--port 3000` instead and the app's own cfg feeds the probe.
//!
//! ```powershell
//! cargo run --example power_treads_probe -- --slot-keys z,x,c,v,b,n
//! ```
//!
//! Use a demo / `-wait 0` lobby with cheats (`-gold`, `-lvlup`, `-refresh`).
//! Pause ~3s between steps so every switch lands on its own ticks.
//!
//! | # | Do this | What it measures |
//! |---|---|---|
//! | 1 | Pick an **AGI** hero, buy Power Treads, stand still 5s | starting state vs primary attribute, baseline tick interval |
//! | 2 | Press treads 6 times, ~2s apart | the single-step signatures, latency, whether the item object changes |
//! | 3 | Take some damage and spend some mana (not full), press treads 3 times slowly | percent preservation at partial HP / mana |
//! | 4 | Double-tap treads fast (<100ms), wait 2s, repeat twice | two-step signatures |
//! | 5 | Triple-tap treads fast | three presses → no visible change? |
//! | 6 | `-lvlup 1`; buy Vitality Booster, Energy Booster, Bracer; sell them | confounders |
//! | 7 | Cast a spell with a cast point and press treads during the cast point; then a channelled spell (or TP) and press treads mid-channel | does the switch cancel either |
//! | 8 | Get stunned (neutral / `-createhero` ally stun) and press treads while stunned | dropped press |
//! | 9 | Drink a Bottle charge and use a Salve on AGI, then again on STR (switch to STR after the salve ends) | heal numbers for the heal trick |
//! | 10 | Take damage while a Salve is running | regen cancellation visible? |
//! | 11 | Move treads to the backpack and back; die and respawn | state across those |
//! | 12 | Repeat steps 1–2 on a **STR**, an **INT**, and a **universal** hero | starting state for every attribute |
//!
//! Every payload is written verbatim to `logs/power_treads/`, so a session can
//! be re-analysed without playing again:
//!
//! ```powershell
//! cargo run --example power_treads_probe -- --replay logs/power_treads/probe_....jsonl --verbose
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{extract::State, routing::post, Router};
use dota2_scripts::actions::hero_attributes::{primary_attribute, PrimaryAttribute};
use serde_json::Value;

const TREADS: &str = "item_power_treads";

/// A treads press older than this with no change after it is reported as
/// dropped (or a silent three-press cycle).
const DEFAULT_PRESS_TIMEOUT_MS: u128 = 1500;

/// Status flags GSI reports on `hero`. A press while any is set may be dropped.
const STATUS_FLAGS: &[&str] = &[
    "stunned",
    "silenced",
    "hexed",
    "muted",
    "disarmed",
    "break",
    "magicimmune",
];

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Attr {
    Str,
    Int,
    Agi,
}

impl Attr {
    fn label(self) -> &'static str {
        match self {
            Attr::Str => "STR",
            Attr::Int => "INT",
            Attr::Agi => "AGI",
        }
    }
}

/// What one tick's `(Δmax_health, Δmax_mana)` sign pair says about the treads.
/// `None` for the pairs no switch produces — `(+,+)` is a level or a stat item.
fn signature(d_max_hp: i64, d_max_mana: i64) -> Option<(Attr, Attr, u8)> {
    match (d_max_hp.signum(), d_max_mana.signum()) {
        (-1, 1) => Some((Attr::Str, Attr::Int, 1)),
        (0, -1) => Some((Attr::Int, Attr::Agi, 1)),
        (1, 0) => Some((Attr::Agi, Attr::Str, 1)),
        (-1, 0) => Some((Attr::Str, Attr::Agi, 2)),
        (1, -1) => Some((Attr::Int, Attr::Str, 2)),
        (0, 1) => Some((Attr::Agi, Attr::Int, 2)),
        _ => None,
    }
}

#[derive(Clone, Debug)]
struct Tick {
    ts_ms: u128,
    clock_time: i64,
    hero: String,
    alive: bool,
    level: i64,
    health: i64,
    max_health: i64,
    health_percent: i64,
    mana: i64,
    max_mana: i64,
    mana_percent: i64,
    status: BTreeSet<String>,
    /// Slot key (`slot3`, `slot7`, `stash0`...) and the whole item object, so
    /// any field that tracks the attribute shows up in a diff.
    treads: Option<(String, Value)>,
    /// Every item slot → item name, for purchase / sale / move detection.
    inventory: BTreeMap<String, String>,
    /// Item slot → charges, so a Bottle / Salve use is visible.
    charges: BTreeMap<String, i64>,
    /// Ability name → cooldown, for cast detection.
    ability_cooldowns: BTreeMap<String, i64>,
}

fn as_i64(value: Option<&Value>) -> i64 {
    value.and_then(|v| v.as_i64()).unwrap_or(0)
}

fn as_bool(value: Option<&Value>) -> bool {
    value.and_then(|v| v.as_bool()).unwrap_or(false)
}

fn parse_tick(ts_ms: u128, payload: &Value) -> Option<Tick> {
    let hero = payload.get("hero")?;
    let name = hero.get("name")?.as_str()?.to_string();
    if name.is_empty() || name == "empty" {
        return None;
    }

    let status = STATUS_FLAGS
        .iter()
        .filter(|flag| as_bool(hero.get(**flag)))
        .map(|flag| flag.to_string())
        .collect();

    let mut treads = None;
    let mut inventory = BTreeMap::new();
    let mut charges = BTreeMap::new();
    if let Some(items) = payload.get("items").and_then(|i| i.as_object()) {
        for (slot, item) in items {
            let Some(item_name) = item.get("name").and_then(|n| n.as_str()) else {
                continue;
            };
            if item_name == "empty" {
                continue;
            }
            if item_name == TREADS && treads.is_none() {
                treads = Some((slot.clone(), item.clone()));
            }
            inventory.insert(slot.clone(), item_name.to_string());
            if let Some(count) = item.get("charges").and_then(|c| c.as_i64()) {
                charges.insert(slot.clone(), count);
            }
        }
    }

    let mut ability_cooldowns = BTreeMap::new();
    if let Some(block) = payload.get("abilities").and_then(|a| a.as_object()) {
        for ability in block.values() {
            if let Some(ability_name) = ability.get("name").and_then(|n| n.as_str()) {
                ability_cooldowns.insert(ability_name.to_string(), as_i64(ability.get("cooldown")));
            }
        }
    }

    Some(Tick {
        ts_ms,
        clock_time: as_i64(payload.get("map").and_then(|m| m.get("clock_time"))),
        hero: name,
        alive: as_bool(hero.get("alive")),
        level: as_i64(hero.get("level")),
        health: as_i64(hero.get("health")),
        max_health: as_i64(hero.get("max_health")),
        health_percent: as_i64(hero.get("health_percent")),
        mana: as_i64(hero.get("mana")),
        max_mana: as_i64(hero.get("max_mana")),
        mana_percent: as_i64(hero.get("mana_percent")),
        status,
        treads,
        inventory,
        charges,
        ability_cooldowns,
    })
}

/// One tick where `max_health` or `max_mana` moved.
#[derive(Debug)]
struct Change {
    ts_ms: u128,
    d_max_hp: i64,
    d_max_mana: i64,
    /// `(from, to, presses)` if the sign pair is a treads switch.
    signature: Option<(Attr, Attr, u8)>,
    /// Level up or inventory change on the same tick — the pair cannot be
    /// trusted even if it happens to match.
    confounded: Option<String>,
    treads_present: bool,
    /// Treads key presses since the previous change, and the first one's lead.
    presses: u32,
    latency_ms: Option<u128>,
    /// The running estimate before this tick, and whether the signature's
    /// `from` agreed with it.
    estimate_before: Option<Attr>,
    hp: (i64, i64, i64, i64),   // health, max, → health, max
    mana: (i64, i64, i64, i64), // mana, max, → mana, max
    pct: (i64, i64, i64, i64),  // hp%, → hp%, mana%, → mana%
}

impl Change {
    fn is_switch(&self) -> bool {
        self.signature.is_some() && self.confounded.is_none() && self.treads_present
    }

    fn agrees(&self) -> Option<bool> {
        if self.confounded.is_some() {
            return None;
        }
        let (from, _, _) = self.signature?;
        self.estimate_before.map(|estimate| estimate == from)
    }
}

struct Analyzer {
    verbose: bool,
    slot_keys: Vec<String>,
    press_timeout_ms: u128,
    session_start_ms: Option<u128>,
    last: Option<Tick>,
    ticks_seen: u64,
    intervals_ms: Vec<u128>,
    /// Treads presses not yet explained by an observed change.
    pending_presses: Vec<u128>,
    /// Status flags active at each pending press, to explain drops.
    pending_status: BTreeSet<String>,
    estimate: Option<Attr>,
    changes: Vec<Change>,
    dropped: Vec<(u128, u32, BTreeSet<String>, bool)>,
    /// Distinct treads item objects seen while the estimate named each state.
    treads_variants: BTreeMap<String, BTreeSet<String>>,
    starts: Vec<(String, Option<PrimaryAttribute>, String)>,
    casts: Vec<(u128, String, u32)>,
}

impl Analyzer {
    fn new(verbose: bool, slot_keys: Vec<String>, press_timeout_ms: u128) -> Self {
        Self {
            verbose,
            slot_keys,
            press_timeout_ms,
            session_start_ms: None,
            last: None,
            ticks_seen: 0,
            intervals_ms: Vec::new(),
            pending_presses: Vec::new(),
            pending_status: BTreeSet::new(),
            estimate: None,
            changes: Vec::new(),
            dropped: Vec::new(),
            treads_variants: BTreeMap::new(),
            starts: Vec::new(),
            casts: Vec::new(),
        }
    }

    fn elapsed(&self, ts_ms: u128) -> f64 {
        match self.session_start_ms {
            Some(start) => ts_ms.saturating_sub(start) as f64 / 1000.0,
            None => 0.0,
        }
    }

    /// The key currently bound to the treads' slot, if they sit in `slot0-5`.
    fn treads_key(&self) -> Option<&str> {
        let (slot, _) = self.last.as_ref()?.treads.as_ref()?;
        let index: usize = slot.strip_prefix("slot")?.parse().ok()?;
        self.slot_keys.get(index).map(|key| key.as_str())
    }

    fn on_key(&mut self, ts_ms: u128, key: &str) {
        self.session_start_ms.get_or_insert(ts_ms);
        let is_treads = self
            .treads_key()
            .is_some_and(|treads| key.eq_ignore_ascii_case(treads));
        println!(
            "[{:>7.2}s] KEY   {}{}",
            self.elapsed(ts_ms),
            key,
            if is_treads { "  ← treads" } else { "" }
        );
        if is_treads {
            if let Some(tick) = &self.last {
                self.pending_status.extend(tick.status.iter().cloned());
                if !tick.alive {
                    self.pending_status.insert(String::from("dead"));
                }
            }
            self.pending_presses.push(ts_ms);
        }
    }

    fn on_tick(&mut self, tick: Tick) {
        self.session_start_ms.get_or_insert(tick.ts_ms);
        self.ticks_seen += 1;

        let Some(previous) = self.last.clone() else {
            println!(
                "[{:>7.2}s] START {} (primary {:?}) — hp {}/{} mana {}/{} level {} clock {}",
                self.elapsed(tick.ts_ms),
                tick.hero,
                primary_attribute(&tick.hero),
                tick.health,
                tick.max_health,
                tick.mana,
                tick.max_mana,
                tick.level,
                tick.clock_time
            );
            if let Some((slot, item)) = &tick.treads {
                println!("            treads already in {slot}: {item} — state unknown until the first switch");
            }
            self.last = Some(tick);
            return;
        };

        let interval = tick.ts_ms.saturating_sub(previous.ts_ms);
        if interval > 0 {
            self.intervals_ms.push(interval);
        }

        if tick.hero != previous.hero {
            println!(
                "[{:>7.2}s] HERO  {} → {}",
                self.elapsed(tick.ts_ms),
                previous.hero,
                tick.hero
            );
            self.estimate = None;
        }

        self.report_side_events(&previous, &tick);

        let d_max_hp = tick.max_health - previous.max_health;
        let d_max_mana = tick.max_mana - previous.max_mana;
        let inventory_changed = tick.inventory != previous.inventory;
        let level_changed = tick.level != previous.level;

        // Treads appearing is the purchase (or a pick-up from the stash).
        if previous.treads.is_none() {
            if let Some((slot, item)) = &tick.treads {
                let primary = primary_attribute(&tick.hero);
                let predicted = match primary {
                    Some(PrimaryAttribute::Strength) => Some(Attr::Str),
                    Some(PrimaryAttribute::Agility) => Some(Attr::Agi),
                    Some(PrimaryAttribute::Intelligence) => Some(Attr::Int),
                    _ => None,
                };
                println!(
                    "[{:>7.2}s] TREADS appear in {} on {} (primary {:?}) — predicted start {}; Δmax_hp {:+} Δmax_mana {:+} this tick",
                    self.elapsed(tick.ts_ms),
                    slot,
                    tick.hero,
                    primary,
                    predicted.map(Attr::label).unwrap_or("unknown"),
                    d_max_hp,
                    d_max_mana
                );
                println!("            item object: {item}");
                self.starts.push((
                    tick.hero.clone(),
                    primary,
                    format!("Δmax_hp {:+}, Δmax_mana {:+}", d_max_hp, d_max_mana),
                ));
                self.estimate = predicted;
            }
        } else if tick.treads.is_none() {
            println!(
                "[{:>7.2}s] TREADS gone from the inventory",
                self.elapsed(tick.ts_ms)
            );
        }

        // Question 2: does the item object carry the state?
        if let (Some((prev_slot, prev_item)), Some((slot, item))) = (&previous.treads, &tick.treads)
        {
            if prev_item != item || prev_slot != slot {
                println!(
                    "[{:>7.2}s] ITEM  treads object changed: {prev_slot} {prev_item}\n                                          → {slot} {item}",
                    self.elapsed(tick.ts_ms)
                );
            }
        }

        if self.verbose {
            println!(
                "[{:>7.2}s] tick  hp {:>5}/{:<5} ({:>3}%) mana {:>5}/{:<5} ({:>3}%)  Δmax {:+}/{:+}  lvl {}  est {}  {}",
                self.elapsed(tick.ts_ms),
                tick.health,
                tick.max_health,
                tick.health_percent,
                tick.mana,
                tick.max_mana,
                tick.mana_percent,
                d_max_hp,
                d_max_mana,
                tick.level,
                self.estimate.map(Attr::label).unwrap_or("?"),
                tick.status.iter().cloned().collect::<Vec<_>>().join(",")
            );
        }

        if d_max_hp != 0 || d_max_mana != 0 {
            self.on_change(
                &previous,
                &tick,
                d_max_hp,
                d_max_mana,
                inventory_changed,
                level_changed,
            );
        } else if let Some(first) = self.pending_presses.first().copied() {
            if tick.ts_ms.saturating_sub(first) >= self.press_timeout_ms {
                let count = self.pending_presses.len() as u32;
                let status = std::mem::take(&mut self.pending_status);
                println!(
                    "[{:>7.2}s] ⚠ {} treads press(es) and no change after {}ms — {}",
                    self.elapsed(tick.ts_ms),
                    count,
                    tick.ts_ms.saturating_sub(first),
                    if count % 3 == 0 {
                        String::from("a full cycle, back to the start?")
                    } else if status.is_empty() {
                        String::from("dropped with no status flag set")
                    } else {
                        format!(
                            "dropped while {}",
                            status.iter().cloned().collect::<Vec<_>>().join(",")
                        )
                    }
                );
                self.dropped.push((first, count, status, count % 3 == 0));
                self.pending_presses.clear();
            }
        }

        if let (Some(estimate), Some((_, item))) = (self.estimate, &tick.treads) {
            self.treads_variants
                .entry(estimate.label().to_string())
                .or_default()
                .insert(item.to_string());
        }

        self.last = Some(tick);
    }

    fn on_change(
        &mut self,
        previous: &Tick,
        tick: &Tick,
        d_max_hp: i64,
        d_max_mana: i64,
        inventory_changed: bool,
        level_changed: bool,
    ) {
        let signature = signature(d_max_hp, d_max_mana);
        let confounded = match (level_changed, inventory_changed) {
            (true, true) => Some(String::from("level + inventory changed")),
            (true, false) => Some(String::from("level changed")),
            (false, true) => Some(String::from("inventory changed")),
            _ => None,
        };
        let presses = self.pending_presses.len() as u32;
        let latency_ms = self
            .pending_presses
            .first()
            .map(|first| tick.ts_ms.saturating_sub(*first));
        self.pending_presses.clear();
        self.pending_status.clear();

        let change = Change {
            ts_ms: tick.ts_ms,
            d_max_hp,
            d_max_mana,
            signature,
            confounded,
            treads_present: tick.treads.is_some() || previous.treads.is_some(),
            presses,
            latency_ms,
            estimate_before: self.estimate,
            hp: (
                previous.health,
                previous.max_health,
                tick.health,
                tick.max_health,
            ),
            mana: (previous.mana, previous.max_mana, tick.mana, tick.max_mana),
            pct: (
                previous.health_percent,
                tick.health_percent,
                previous.mana_percent,
                tick.mana_percent,
            ),
        };

        let verdict = match (&change.signature, &change.confounded) {
            (_, Some(reason)) => format!("not a switch — {reason}"),
            (None, None) => String::from("no switch has this sign pair"),
            (Some((from, to, steps)), None) => format!(
                "{}→{} ({} press{}){}",
                from.label(),
                to.label(),
                steps,
                if *steps == 1 { "" } else { "es" },
                match change.agrees() {
                    Some(true) => "  ✓ matches estimate",
                    Some(false) => "  ✗ estimate said otherwise",
                    None => "  (no prior estimate)",
                }
            ),
        };

        println!(
            "[{:>7.2}s] MOVE  Δmax_hp {:+} Δmax_mana {:+} — {}",
            self.elapsed(tick.ts_ms),
            d_max_hp,
            d_max_mana,
            verdict
        );
        println!(
            "            hp {}/{} ({}%) → {}/{} ({}%)   mana {}/{} ({}%) → {}/{} ({}%)   treads presses {} latency {}",
            change.hp.0,
            change.hp.1,
            change.pct.0,
            change.hp.2,
            change.hp.3,
            change.pct.1,
            change.mana.0,
            change.mana.1,
            change.pct.2,
            change.mana.2,
            change.mana.3,
            change.pct.3,
            change.presses,
            change
                .latency_ms
                .map(|ms| format!("{ms}ms"))
                .unwrap_or_else(|| String::from("—"))
        );

        if change.is_switch() {
            if let Some((_, to, _)) = change.signature {
                self.estimate = Some(to);
            }
        }
        self.changes.push(change);
    }

    fn report_side_events(&mut self, previous: &Tick, tick: &Tick) {
        let at = self.elapsed(tick.ts_ms);

        if tick.alive != previous.alive {
            println!(
                "[{at:>7.2}s] {}",
                if tick.alive { "RESPAWN" } else { "DIED" }
            );
        }
        if tick.level != previous.level {
            println!("[{at:>7.2}s] LEVEL {} → {}", previous.level, tick.level);
        }
        if tick.status != previous.status {
            println!(
                "[{at:>7.2}s] STATUS {}",
                if tick.status.is_empty() {
                    String::from("clear")
                } else {
                    tick.status.iter().cloned().collect::<Vec<_>>().join(",")
                }
            );
        }

        let slots: BTreeSet<&String> = previous
            .inventory
            .keys()
            .chain(tick.inventory.keys())
            .collect();
        for slot in slots {
            let before = previous.inventory.get(slot);
            let after = tick.inventory.get(slot);
            if before != after {
                println!(
                    "[{at:>7.2}s] INV   {slot}: {} → {}",
                    before.map(String::as_str).unwrap_or("—"),
                    after.map(String::as_str).unwrap_or("—")
                );
            }
        }
        for (slot, count) in &tick.charges {
            if let Some(before) = previous.charges.get(slot) {
                if before != count && previous.inventory.get(slot) == tick.inventory.get(slot) {
                    println!(
                        "[{at:>7.2}s] USE   {} charges {} → {}",
                        tick.inventory.get(slot).map(String::as_str).unwrap_or(slot),
                        before,
                        count
                    );
                }
            }
        }

        // Question 8: a cast that completed starts a cooldown.
        for (name, cooldown) in &tick.ability_cooldowns {
            let before = previous.ability_cooldowns.get(name).copied().unwrap_or(0);
            if before == 0 && *cooldown > 0 {
                let presses_in_flight = self.pending_presses.len() as u32;
                println!(
                    "[{at:>7.2}s] CAST  {name} (cooldown {cooldown}s){}",
                    if presses_in_flight > 0 {
                        format!("  — {presses_in_flight} treads press(es) still unexplained")
                    } else {
                        String::new()
                    }
                );
                self.casts
                    .push((tick.ts_ms, name.clone(), presses_in_flight));
            }
        }
    }

    fn report(&mut self) {
        println!("\n═══ summary ═══");
        println!("ticks: {}", self.ticks_seen);
        if self.ticks_seen == 0 {
            println!("nothing captured — was a hero in play, and is the cfg pointed at this port?");
            return;
        }

        if !self.intervals_ms.is_empty() {
            let mut sorted = self.intervals_ms.clone();
            sorted.sort_unstable();
            let sum: u128 = sorted.iter().sum();
            println!(
                "tick interval: min {}ms, median {}ms, mean {}ms, max {}ms",
                sorted[0],
                sorted[sorted.len() / 2],
                sum / sorted.len() as u128,
                sorted[sorted.len() - 1]
            );
        }

        // Question 1.
        println!("\ntreads purchases:");
        if self.starts.is_empty() {
            println!("  none seen — buy treads while the probe runs");
        }
        for (hero, primary, delta) in &self.starts {
            println!("  {hero:<32} primary {primary:?}, {delta}");
        }
        println!(
            "  → the first switch after each purchase names the real start; check its ✓/✗ below"
        );

        println!("\nmax_health / max_mana moves ({}):", self.changes.len());
        println!(
            "  {:>8}  {:>7}  {:>7}  {:<22}  {:>7}  {:>8}  {:>9}  {:>11}  {}",
            "at", "Δmaxhp", "Δmaxmp", "reading", "presses", "latency", "hp%", "mana%", "estimate"
        );
        for change in &self.changes {
            let reading = match (&change.signature, &change.confounded) {
                (_, Some(reason)) => reason.clone(),
                (None, None) => String::from("not a switch pair"),
                (Some((from, to, steps)), None) => {
                    format!("{}→{} ×{}", from.label(), to.label(), steps)
                }
            };
            println!(
                "  {:>7.2}s  {:>+7}  {:>+7}  {:<22}  {:>7}  {:>8}  {:>4}→{:<4}  {:>4}→{:<5}  {}",
                self.elapsed(change.ts_ms),
                change.d_max_hp,
                change.d_max_mana,
                reading,
                change.presses,
                change
                    .latency_ms
                    .map(|ms| format!("{ms}ms"))
                    .unwrap_or_else(|| String::from("—")),
                change.pct.0,
                change.pct.1,
                change.pct.2,
                change.pct.3,
                match change.agrees() {
                    Some(true) => "✓",
                    Some(false) => "✗",
                    None => "—",
                }
            );
        }

        let switches: Vec<&Change> = self.changes.iter().filter(|c| c.is_switch()).collect();
        if !switches.is_empty() {
            // Question 4: presses seen vs steps the signature implies.
            let press_matches = switches
                .iter()
                .filter(|c| {
                    c.signature
                        .is_some_and(|(_, _, steps)| c.presses == steps as u32)
                })
                .count();
            println!(
                "\npresses vs signature steps: {}/{} switches agree (a mismatch is a merged tick, an unwatched key, or a manual click)",
                press_matches,
                switches.len()
            );

            // Question 3: within ±1 point, since GSI rounds the percent.
            let preserved = switches
                .iter()
                .filter(|c| (c.pct.0 - c.pct.1).abs() <= 1 && (c.pct.2 - c.pct.3).abs() <= 1)
                .count();
            println!(
                "percent preserved (±1%) across switches: {}/{}",
                preserved,
                switches.len()
            );

            // Question 5.
            let mut latencies: Vec<u128> = switches.iter().filter_map(|c| c.latency_ms).collect();
            latencies.sort_unstable();
            if !latencies.is_empty() {
                println!(
                    "press → change latency: min {}ms, median {}ms, max {}ms",
                    latencies[0],
                    latencies[latencies.len() / 2],
                    latencies[latencies.len() - 1]
                );
            }

            let hp_sizes: BTreeSet<i64> = switches
                .iter()
                .map(|c| c.d_max_hp.abs())
                .filter(|d| *d > 0)
                .collect();
            let mana_sizes: BTreeSet<i64> = switches
                .iter()
                .map(|c| c.d_max_mana.abs())
                .filter(|d| *d > 0)
                .collect();
            println!(
                "switch sizes seen: |Δmax_hp| {:?}, |Δmax_mana| {:?}",
                hp_sizes, mana_sizes
            );

            let disagreements = switches
                .iter()
                .filter(|c| c.agrees() == Some(false))
                .count();
            println!(
                "estimate contradicted by a switch: {} time(s)",
                disagreements
            );
        }

        let impostors: Vec<&Change> = self
            .changes
            .iter()
            .filter(|c| c.signature.is_some() && c.confounded.is_some())
            .collect();
        if !impostors.is_empty() {
            println!("\nnon-switches that share a switch's sign pair (the discriminator must reject these):");
            for change in impostors {
                println!(
                    "  {:>7.2}s  Δ{:+}/{:+}  {}",
                    self.elapsed(change.ts_ms),
                    change.d_max_hp,
                    change.d_max_mana,
                    change.confounded.as_deref().unwrap_or("")
                );
            }
        }

        if !self.dropped.is_empty() {
            println!("\npresses with no change:");
            for (at, count, status, cycle) in &self.dropped {
                println!(
                    "  {:>7.2}s  {} press(es)  {}{}",
                    self.elapsed(*at),
                    count,
                    if status.is_empty() {
                        String::from("no status flag")
                    } else {
                        status.iter().cloned().collect::<Vec<_>>().join(",")
                    },
                    if *cycle { "  (multiple of 3)" } else { "" }
                );
            }
        }

        // Question 2.
        println!("\ntreads item object per estimated state:");
        if self.treads_variants.is_empty() {
            println!("  —");
        }
        for (state, variants) in &self.treads_variants {
            println!("  {state}: {} distinct", variants.len());
            for variant in variants {
                println!("      {variant}");
            }
        }
        let all: BTreeSet<&String> = self.treads_variants.values().flatten().collect();
        if self.treads_variants.len() > 1 && all.len() == 1 {
            println!("  → identical in every state: GSI does not expose the treads attribute");
        }

        if !self.casts.is_empty() {
            println!("\ncasts (cooldown 0 → N): {}", self.casts.len());
            for (at, name, in_flight) in &self.casts {
                println!(
                    "  {:>7.2}s  {name}{}",
                    self.elapsed(*at),
                    if *in_flight > 0 {
                        "  (treads pressed during it)"
                    } else {
                        ""
                    }
                );
            }
        }
    }
}

/// The capture file: every payload verbatim, plus the key presses.
struct Recorder {
    path: PathBuf,
}

impl Recorder {
    fn create(dir: &str) -> Option<Self> {
        if let Err(error) = fs::create_dir_all(dir) {
            eprintln!("could not create {dir}: {error}");
            return None;
        }
        let path = PathBuf::from(dir).join(format!(
            "probe_{}.jsonl",
            chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
        ));
        println!("recording to {}", path.display());
        Some(Self { path })
    }

    fn write(&self, line: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(file, "{}", line);
        }
    }

    fn record_payload(&self, ts_ms: u128, payload: &Value) {
        self.write(&format!(
            "{{\"probe_ts_ms\":{},\"payload\":{}}}",
            ts_ms,
            serde_json::to_string(payload).unwrap_or_else(|_| String::from("null"))
        ));
    }

    fn record_key(&self, ts_ms: u128, key: &str) {
        self.write(&format!(
            "{{\"probe_ts_ms\":{},\"key\":\"{}\"}}",
            ts_ms, key
        ));
    }
}

struct Shared {
    analyzer: Analyzer,
    recorder: Option<Recorder>,
}

async fn receive(State(shared): State<Arc<Mutex<Shared>>>, body: String) -> &'static str {
    let ts_ms = now_ms();
    let Ok(payload) = serde_json::from_str::<Value>(&body) else {
        eprintln!("payload did not parse as JSON ({} bytes)", body.len());
        return "bad json";
    };

    if let Ok(mut shared) = shared.lock() {
        if let Some(recorder) = &shared.recorder {
            recorder.record_payload(ts_ms, &payload);
        }
        if let Some(tick) = parse_tick(ts_ms, &payload) {
            shared.analyzer.on_tick(tick);
        }
    }
    "ok"
}

/// `KeyZ` → `z`, `Num3` → `3`; mouse buttons as `Mouse4` / `Mouse5` / `Mouse3`,
/// matching the names `[keybindings]` accepts.
fn key_label(event: &rdev::EventType) -> Option<String> {
    match event {
        rdev::EventType::KeyPress(key) => {
            let debug = format!("{:?}", key);
            if let Some(letter) = debug.strip_prefix("Key") {
                Some(letter.to_ascii_lowercase())
            } else if let Some(digit) = debug.strip_prefix("Num") {
                Some(digit.to_string())
            } else {
                Some(debug)
            }
        }
        rdev::EventType::ButtonPress(button) => match button {
            rdev::Button::Left | rdev::Button::Right => None,
            rdev::Button::Middle => Some(String::from("Mouse3")),
            rdev::Button::Unknown(1) => Some(String::from("Mouse4")),
            rdev::Button::Unknown(2) => Some(String::from("Mouse5")),
            other => Some(format!("{:?}", other)),
        },
        _ => None,
    }
}

/// Watch the keys that matter — the item slot keys and the ability keys — with
/// a plain listener. `rdev::listen` observes, unlike the app's `rdev::grab`, so
/// this never swallows a key even with the real app running alongside.
fn watch_keys(watched: BTreeSet<String>, shared: Arc<Mutex<Shared>>) {
    std::thread::spawn(move || {
        let mut last_press: BTreeMap<String, u128> = BTreeMap::new();
        let result = rdev::listen(move |event| {
            let Some(label) = key_label(&event.event_type) else {
                return;
            };
            if !watched.iter().any(|key| key.eq_ignore_ascii_case(&label)) {
                return;
            }

            // Held keys autorepeat at ~30ms; a deliberate double-tap is slower
            // than that, and step 4 of the protocol needs double-taps kept.
            let ts_ms = now_ms();
            let repeat = last_press
                .get(&label)
                .is_some_and(|previous| ts_ms.saturating_sub(*previous) < 40);
            last_press.insert(label.clone(), ts_ms);
            if repeat {
                return;
            }

            if let Ok(mut shared) = shared.lock() {
                if let Some(recorder) = &shared.recorder {
                    recorder.record_key(ts_ms, &label);
                }
                shared.analyzer.on_key(ts_ms, &label);
            }
        });
        if let Err(error) = result {
            eprintln!(
                "key watch unavailable ({:?}) — running without key timing",
                error
            );
        }
    });
}

fn replay(path: &str, analyzer: &mut Analyzer) {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("could not open {path}: {error}");
            return;
        }
    };

    let mut synthetic_ts = 0u128;
    // A stream of JSON values, not a value per line: Dota pretty-prints.
    let stream = serde_json::Deserializer::from_reader(BufReader::new(file)).into_iter::<Value>();
    for (index, value) in stream.enumerate() {
        let value = match value {
            Ok(value) => value,
            Err(error) => {
                eprintln!("stopped after {} values: {}", index, error);
                break;
            }
        };

        match value.get("probe_ts_ms").and_then(|ts| ts.as_u64()) {
            Some(ts) => {
                let ts_ms = ts as u128;
                if let Some(key) = value.get("key").and_then(|key| key.as_str()) {
                    analyzer.on_key(ts_ms, key);
                } else if let Some(payload) = value.get("payload") {
                    if let Some(tick) = parse_tick(ts_ms, payload) {
                        analyzer.on_tick(tick);
                    }
                }
            }
            // The app's own gsi_events log: bare payloads, no timing.
            None => {
                synthetic_ts += 100;
                if let Some(tick) = parse_tick(synthetic_ts, &value) {
                    analyzer.on_tick(tick);
                }
            }
        }
    }

    if analyzer.session_start_ms.is_none() {
        println!("no hero payloads in {path}");
    }
}

struct Args {
    port: u16,
    slot_keys: Vec<String>,
    ability_keys: Vec<String>,
    replay: Option<String>,
    output_dir: String,
    verbose: bool,
    press_timeout_ms: u128,
    listen_keys: bool,
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

impl Args {
    fn parse() -> Self {
        let mut args = Self {
            port: 3100,
            slot_keys: split_list("z,x,c,v,b,n"),
            ability_keys: split_list("q,w,e,r,d,f"),
            replay: None,
            output_dir: String::from("logs/power_treads"),
            verbose: false,
            press_timeout_ms: DEFAULT_PRESS_TIMEOUT_MS,
            listen_keys: true,
        };

        let raw: Vec<String> = std::env::args().skip(1).collect();
        let mut index = 0;
        while index < raw.len() {
            match raw[index].as_str() {
                "--port" => {
                    index += 1;
                    if let Some(value) = raw.get(index).and_then(|v| v.parse().ok()) {
                        args.port = value;
                    }
                }
                "--slot-keys" => {
                    index += 1;
                    if let Some(value) = raw.get(index) {
                        args.slot_keys = split_list(value);
                    }
                }
                "--ability-keys" => {
                    index += 1;
                    if let Some(value) = raw.get(index) {
                        args.ability_keys = split_list(value);
                    }
                }
                "--replay" => {
                    index += 1;
                    args.replay = raw.get(index).cloned();
                }
                "--output" => {
                    index += 1;
                    if let Some(value) = raw.get(index) {
                        args.output_dir = value.clone();
                    }
                }
                "--press-timeout-ms" => {
                    index += 1;
                    if let Some(value) = raw.get(index).and_then(|v| v.parse().ok()) {
                        args.press_timeout_ms = value;
                    }
                }
                "--no-keys" => args.listen_keys = false,
                "--verbose" | "-v" => args.verbose = true,
                "--help" | "-h" => {
                    usage();
                    std::process::exit(0);
                }
                other => eprintln!("ignoring unknown argument {other}"),
            }
            index += 1;
        }
        args
    }
}

fn usage() {
    println!("Power Treads probe — records GSI and reads treads switches from max HP / max mana");
    println!();
    println!("Usage:");
    println!("  cargo run --example power_treads_probe -- [--port 3100] [--slot-keys z,x,c,v,b,n]");
    println!("  cargo run --example power_treads_probe -- --replay <file.jsonl> [--verbose]");
    println!();
    println!("Options:");
    println!("  --port <N>              Listen port for a second GSI cfg (default: 3100)");
    println!("  --slot-keys <a,b,..>    Item slot keys slot0..slot5, as in [keybindings] (default: z,x,c,v,b,n)");
    println!("  --ability-keys <a,..>   Ability keys to timestamp (default: q,w,e,r,d,f)");
    println!("  --no-keys               Skip the keyboard listener");
    println!("  --replay <PATH>         Analyse a capture instead of listening");
    println!("  --output <DIR>          Capture directory (default: logs/power_treads)");
    println!("  --press-timeout-ms <N>  A treads press with no change after this is 'dropped' (default: 1500)");
    println!("  --verbose, -v           Print every tick");
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut analyzer = Analyzer::new(args.verbose, args.slot_keys.clone(), args.press_timeout_ms);

    if let Some(path) = &args.replay {
        replay(path, &mut analyzer);
        analyzer.report();
        return;
    }

    let shared = Arc::new(Mutex::new(Shared {
        analyzer,
        recorder: Recorder::create(&args.output_dir),
    }));

    if args.listen_keys {
        let watched: BTreeSet<String> = args
            .slot_keys
            .iter()
            .chain(args.ability_keys.iter())
            .cloned()
            .collect();
        println!(
            "watching keys: {}",
            watched.iter().cloned().collect::<Vec<_>>().join(", ")
        );
        watch_keys(watched, Arc::clone(&shared));
    }

    let app = Router::new()
        .route("/", post(receive))
        .with_state(Arc::clone(&shared));

    let address = format!("127.0.0.1:{}", args.port);
    let listener = match tokio::net::TcpListener::bind(&address).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("could not bind {address}: {error}");
            return;
        }
    };

    println!("listening on http://{address}/ — point a gamestate_integration cfg at it");
    println!("Ctrl-C to stop and print the summary\n");

    let server = axum::serve(listener, app);
    tokio::select! {
        result = server => {
            if let Err(error) = result {
                eprintln!("server stopped: {error}");
            }
        }
        _ = tokio::signal::ctrl_c() => {}
    }

    if let Ok(mut locked) = shared.lock() {
        locked.analyzer.report();
    };
}
