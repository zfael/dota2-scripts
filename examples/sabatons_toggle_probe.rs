//! Sabatons of Mordiggian toggle probe — what does a Sabatons (or Armlet) toggle
//! look like in GSI, and does the Armlet trick carry over?
//!
//! Nothing in `src/` depends on this. 7.42 added `item_berserkers_sabatons`, built
//! from Armlet: a toggle that drains HP for +30 Strength, +40 damage and +5 armor.
//! The Armlet automation (`src/actions/armlet.rs`) only looks for `item_armlet`.
//! Before it learns the new item, these have to be measured, not assumed:
//!
//! 1. **The trick.** Armlet's automation double-taps the item (off, then on) to
//!    gain HP at low health. Does an off-on double tap gain HP with Sabatons too,
//!    and how much, a second after the press?
//! 2. **Ramp.** How long from the press until `max_health` stops moving? That is
//!    how long the Strength takes to come in, and the floor on the toggle cooldown.
//! 3. **Lethality.** The patch notes say neither the drain nor the Strength loss
//!    can kill. Does HP floor at 1, and does a toggle at a sliver ever kill?
//! 4. **Toggle state.** Does any item or hero field differ between on and off?
//!    Power Treads showed nothing, so the expectation is no, and the state has to
//!    be inferred from the sign of the `max_health` change.
//! 5. **Drain.** HP per second lost while on and standing still, next to the
//!    regen while off. 7.42 also made Armlet's drain show as negative regen.
//!
//! The probe never presses anything. It listens.
//!
//! ## Capture protocol
//!
//! Close the app first: the probe listens on the app's GSI port (3000), so it
//! uses the cfg already installed, and the two cannot both bind it. To run it
//! beside the app instead, add a second cfg in
//! `steamapps/common/dota 2 beta/game/dota/cfg/gamestate_integration/` with
//! `"uri" "http://127.0.0.1:3100/"` and its own `"name"`, and pass `--port 3100`.
//!
//! ```powershell
//! cargo run --example sabatons_toggle_probe -- --keys z,x,c,v,b,n
//! ```
//!
//! `--keys` are your item slot keys, slot0 to slot5, the same as `[keybindings]`.
//! Take a Strength hero into a demo game, buy Sabatons of Mordiggian, and run
//! this script. Wait ~3s between steps so the episodes stay separable:
//!
//! | # | Do this | What it measures |
//! |---|---|---|
//! | 1 | Stand still 5s, Sabatons off | baseline, tick interval, regen while off |
//! | 2 | Toggle on, stand still 6s | latency, ramp, drain rate while on |
//! | 3 | Toggle off, wait 4s | Strength loss: does current HP fall with max HP? |
//! | 4 | Toggle on, then double tap (off-on) at full HP | the trick at full HP |
//! | 5 | Let creeps or the demo's damage bring you under 300 HP, then double tap | **the trick at low HP: the number the automation lives on** |
//! | 6 | Stay on at low HP until the drain stops | does the drain floor at 1 HP |
//! | 7 | At a sliver, toggle off | can the Strength loss kill |
//! | 8 | Level up, then buy an HP item | non-toggle `max_health` moves |
//! | 9 | Sell Sabatons, buy Armlet, repeat 2, 4 and 5 | 7.42 Armlet, for comparison |
//!
//! Every payload is written verbatim to `logs/sabatons_toggle/`, so the session
//! can be re-analysed without playing again:
//!
//! ```powershell
//! cargo run --example sabatons_toggle_probe -- --replay logs/sabatons_toggle/probe_....jsonl --verbose
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{extract::State, routing::post, Router};
use serde_json::{Map, Value};

/// The toggles this probe follows, Sabatons first if somehow both are held.
const TOGGLE_ITEMS: &[&str] = &["item_berserkers_sabatons", "item_armlet"];

/// Presses closer together than this are one action: a double tap, or the app's
/// quick-cast-then-modifier chord.
const DEFAULT_MERGE_MS: u128 = 500;

/// How long after an action its effect is still attributed to it.
const DEFAULT_WINDOW_MS: u128 = 4000;

/// When the "a second later" HP reading is taken for the trick.
const TRICK_READ_MS: u128 = 1000;

/// Hero fields that move every tick for reasons that have nothing to do with the
/// toggle. Everything else is diffed, in case one of them is the toggle state.
const NOISY_HERO_FIELDS: &[&str] = &[
    "xpos",
    "ypos",
    "health",
    "max_health",
    "health_percent",
    "mana",
    "max_mana",
    "mana_percent",
    "xp",
    "buyback_cost",
    "buyback_cooldown",
    "respawn_seconds",
];

const MODIFIER_LABELS: &[&str] = &[
    "Alt",
    "AltGr",
    "ControlLeft",
    "ControlRight",
    "ShiftLeft",
    "ShiftRight",
];

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn as_i64(value: Option<&Value>) -> i64 {
    value.and_then(|v| v.as_i64()).unwrap_or(0)
}

fn as_bool(value: Option<&Value>) -> bool {
    value.and_then(|v| v.as_bool()).unwrap_or(false)
}

#[derive(Clone, Debug)]
struct ToggleItem {
    name: String,
    slot: String,
    raw: Map<String, Value>,
}

#[derive(Clone, Debug)]
struct Tick {
    ts_ms: u128,
    clock_time: i64,
    hero: String,
    alive: bool,
    health: i64,
    max_health: i64,
    health_percent: i64,
    hero_fields: Map<String, Value>,
    item: Option<ToggleItem>,
}

/// Pull the fields we care about out of one raw GSI payload. Returns `None` for
/// the menu / draft / spectator payloads that carry no hero.
fn parse_tick(ts_ms: u128, payload: &Value) -> Option<Tick> {
    let hero = payload.get("hero")?;
    let name = hero.get("name")?.as_str()?.to_string();
    if name.is_empty() || name == "empty" {
        return None;
    }

    let mut hero_fields = hero.as_object().cloned().unwrap_or_default();
    for field in NOISY_HERO_FIELDS {
        hero_fields.remove(*field);
    }

    // Inventory and backpack only: a toggle in the stash is not a toggle.
    let item = payload
        .get("items")
        .and_then(|items| items.as_object())
        .and_then(|items| {
            TOGGLE_ITEMS.iter().find_map(|wanted| {
                (0..9).find_map(|index| {
                    let slot = format!("slot{index}");
                    let raw = items.get(&slot)?.as_object()?;
                    (raw.get("name")?.as_str()? == *wanted).then(|| ToggleItem {
                        name: wanted.to_string(),
                        slot,
                        raw: raw.clone(),
                    })
                })
            })
        });

    Some(Tick {
        ts_ms,
        clock_time: as_i64(payload.get("map").and_then(|m| m.get("clock_time"))),
        hero: name,
        alive: as_bool(hero.get("alive")),
        health: as_i64(hero.get("health")),
        max_health: as_i64(hero.get("max_health")),
        health_percent: as_i64(hero.get("health_percent")),
        hero_fields,
        item,
    })
}

/// One press of the toggle item, or a burst of presses close enough to be one
/// double tap, and everything that happened in the window after it.
#[derive(Debug)]
struct Action {
    item: String,
    first_press_ms: u128,
    last_press_ms: u128,
    presses: u32,
    with_modifier: bool,
    hp_before: i64,
    max_before: i64,
    pct_before: i64,
    /// `(ts, health, max_health, health_percent)` for every tick in the window.
    samples: Vec<(u128, i64, i64, i64)>,
    died: bool,
}

/// What an action did, once its window closed.
#[derive(Debug)]
struct ActionResult {
    item: String,
    at_ms: u128,
    presses: u32,
    press_gap_ms: u128,
    with_modifier: bool,
    hp_before: i64,
    hp_after_1s: Option<i64>,
    hp_min: i64,
    hp_end: i64,
    max_before: i64,
    max_min: i64,
    max_end: i64,
    pct_before: i64,
    pct_end: i64,
    latency_ms: Option<u128>,
    settle_ms: Option<u128>,
    died: bool,
}

impl ActionResult {
    fn net_max(&self) -> i64 {
        self.max_end - self.max_before
    }

    /// The direction the toggle went, read the only way GSI allows: a gain in
    /// `max_health` is Strength coming in, a loss is it leaving. A double tap that
    /// ends where it started tells us nothing about the final state.
    fn inferred_state(&self) -> Option<bool> {
        match self.net_max().cmp(&0) {
            std::cmp::Ordering::Greater => Some(true),
            std::cmp::Ordering::Less => Some(false),
            std::cmp::Ordering::Equal => None,
        }
    }

    fn trick_gain(&self) -> Option<i64> {
        self.hp_after_1s.map(|hp| hp - self.hp_before)
    }
}

/// A stretch of ticks between two actions, used for the drain and regen rates.
#[derive(Debug)]
struct Segment {
    item: Option<String>,
    state: Option<bool>,
    start_ms: u128,
    end_ms: u128,
    hp_start: i64,
    hp_end: i64,
    hp_min: i64,
    /// Sum of health deltas and of elapsed time over ticks where `max_health`
    /// did not move, so a level or an item doesn't count as regen.
    steady_hp: i64,
    steady_ms: u128,
    at_one_hp_ticks: u32,
}

impl Segment {
    fn rate_hp_per_sec(&self) -> Option<f64> {
        (self.steady_ms >= 500).then(|| self.steady_hp as f64 * 1000.0 / self.steady_ms as f64)
    }
}

/// End-of-window snapshots of the item and hero JSON, keyed by the state the
/// action left the toggle in. If a field holds different values in the two
/// groups, GSI does report the toggle state after all.
#[derive(Default)]
struct StateSnapshots {
    on: Vec<Map<String, Value>>,
    off: Vec<Map<String, Value>>,
}

impl StateSnapshots {
    fn record(&mut self, on: bool, snapshot: Map<String, Value>) {
        if on {
            self.on.push(snapshot);
        } else {
            self.off.push(snapshot);
        }
    }

    fn separating_fields(&self) -> Vec<String> {
        if self.on.is_empty() || self.off.is_empty() {
            return Vec::new();
        }
        let mut fields: BTreeSet<String> = BTreeSet::new();
        for snapshot in self.on.iter().chain(self.off.iter()) {
            fields.extend(snapshot.keys().cloned());
        }
        fields
            .into_iter()
            .filter(|field| {
                let values = |group: &Vec<Map<String, Value>>| -> BTreeSet<String> {
                    group
                        .iter()
                        .map(|snapshot| {
                            snapshot
                                .get(field)
                                .map(|value| value.to_string())
                                .unwrap_or_else(|| String::from("<absent>"))
                        })
                        .collect()
                };
                values(&self.on).is_disjoint(&values(&self.off))
            })
            .collect()
    }
}

struct Analyzer {
    verbose: bool,
    merge_ms: u128,
    window_ms: u128,
    /// rdev labels of the slot0..slot5 keys.
    slot_keys: Vec<String>,
    modifier_held: bool,
    session_start_ms: Option<u128>,
    last: Option<Tick>,
    pending: Option<Action>,
    results: Vec<ActionResult>,
    segment: Option<Segment>,
    segments: Vec<Segment>,
    state: Option<bool>,
    item_snapshots: StateSnapshots,
    hero_snapshots: StateSnapshots,
    item_field_changes: BTreeMap<String, u32>,
    hero_field_changes: BTreeMap<String, u32>,
    intervals_ms: Vec<u128>,
    deaths: Vec<(u128, i64, Option<bool>)>,
    min_hp_with_item: Option<i64>,
    ticks_seen: u64,
}

impl Analyzer {
    fn new(verbose: bool, merge_ms: u128, window_ms: u128, slot_keys: Vec<String>) -> Self {
        Self {
            verbose,
            merge_ms,
            window_ms,
            slot_keys,
            modifier_held: false,
            session_start_ms: None,
            last: None,
            pending: None,
            results: Vec::new(),
            segment: None,
            segments: Vec::new(),
            state: None,
            item_snapshots: StateSnapshots::default(),
            hero_snapshots: StateSnapshots::default(),
            item_field_changes: BTreeMap::new(),
            hero_field_changes: BTreeMap::new(),
            intervals_ms: Vec::new(),
            deaths: Vec::new(),
            min_hp_with_item: None,
            ticks_seen: 0,
        }
    }

    fn elapsed(&self, ts_ms: u128) -> f64 {
        match self.session_start_ms {
            Some(start) => ts_ms.saturating_sub(start) as f64 / 1000.0,
            None => 0.0,
        }
    }

    fn on_key_up(&mut self, label: &str) {
        if MODIFIER_LABELS.contains(&label) {
            self.modifier_held = false;
        }
    }

    fn on_key(&mut self, ts_ms: u128, label: &str) {
        self.session_start_ms.get_or_insert(ts_ms);
        if MODIFIER_LABELS.contains(&label) {
            self.modifier_held = true;
            return;
        }

        let Some(slot_index) = self.slot_keys.iter().position(|key| key == label) else {
            return;
        };
        let wanted_slot = format!("slot{slot_index}");
        let Some((item_name, hp, max, pct)) = self.last.as_ref().and_then(|tick| {
            let item = tick.item.as_ref()?;
            (item.slot == wanted_slot)
                .then(|| (item.name.clone(), tick.health, tick.max_health, tick.health_percent))
        }) else {
            println!(
                "[{:>7.2}s] KEY   {} (slot{}, not the toggle item)",
                self.elapsed(ts_ms),
                label,
                slot_index
            );
            return;
        };

        let modifier = self.modifier_held;
        let elapsed = self.elapsed(ts_ms);
        if let Some(action) = self.pending.as_mut() {
            if ts_ms.saturating_sub(action.last_press_ms) <= self.merge_ms {
                action.presses += 1;
                action.last_press_ms = ts_ms;
                action.with_modifier |= modifier;
                println!(
                    "[{:>7.2}s] PRESS {} again{} — {} presses in this action",
                    elapsed,
                    label,
                    if modifier { " with modifier" } else { "" },
                    action.presses
                );
                return;
            }
        }

        self.close_action();
        self.close_segment();
        println!(
            "[{:>7.2}s] PRESS {} → {}{} — hp {} / {} ({}%)",
            self.elapsed(ts_ms),
            label,
            item_name,
            if modifier { " with modifier" } else { "" },
            hp,
            max,
            pct
        );
        self.pending = Some(Action {
            item: item_name,
            first_press_ms: ts_ms,
            last_press_ms: ts_ms,
            presses: 1,
            with_modifier: modifier,
            hp_before: hp,
            max_before: max,
            pct_before: pct,
            samples: Vec::new(),
            died: false,
        });
    }

    fn on_tick(&mut self, tick: Tick) {
        self.session_start_ms.get_or_insert(tick.ts_ms);
        self.ticks_seen += 1;

        let Some(previous) = self.last.clone() else {
            println!(
                "[{:>7.2}s] START hero {} — hp {} / {} ({}%), clock {}",
                self.elapsed(tick.ts_ms),
                tick.hero,
                tick.health,
                tick.max_health,
                tick.health_percent,
                tick.clock_time
            );
            self.print_item(&tick);
            self.open_segment(&tick);
            self.last = Some(tick);
            return;
        };

        let interval = tick.ts_ms.saturating_sub(previous.ts_ms);
        if interval > 0 {
            self.intervals_ms.push(interval);
        }

        self.diff_fields(&previous, &tick);

        if previous.item.as_ref().map(|item| (&item.name, &item.slot))
            != tick.item.as_ref().map(|item| (&item.name, &item.slot))
        {
            self.print_item(&tick);
            // A new toggle starts off; whatever we knew about the old one is gone.
            self.state = None;
        }

        if tick.item.is_some() {
            self.min_hp_with_item = Some(
                self.min_hp_with_item
                    .map_or(tick.health, |min| min.min(tick.health)),
            );
        }

        if previous.alive && !tick.alive {
            println!(
                "[{:>7.2}s] DIED  last hp {} — toggle was {}",
                self.elapsed(tick.ts_ms),
                previous.health,
                state_label(self.state)
            );
            self.deaths.push((tick.ts_ms, previous.health, self.state));
            if let Some(action) = self.pending.as_mut() {
                action.died = true;
            }
        }

        if self.verbose {
            println!(
                "[{:>7.2}s] tick  hp {:>5} ({:>+4})  max {:>5} ({:>+4})  {:>3}%  {}",
                self.elapsed(tick.ts_ms),
                tick.health,
                tick.health - previous.health,
                tick.max_health,
                tick.max_health - previous.max_health,
                tick.health_percent,
                state_label(self.state)
            );
        }

        if let Some(action) = self.pending.as_mut() {
            action.samples.push((
                tick.ts_ms,
                tick.health,
                tick.max_health,
                tick.health_percent,
            ));
        }
        let window_over = self
            .pending
            .as_ref()
            .is_some_and(|action| tick.ts_ms.saturating_sub(action.first_press_ms) >= self.window_ms);

        if let Some(segment) = self.segment.as_mut() {
            segment.end_ms = tick.ts_ms;
            segment.hp_end = tick.health;
            segment.hp_min = segment.hp_min.min(tick.health);
            if tick.alive && previous.alive && tick.max_health == previous.max_health {
                segment.steady_hp += tick.health - previous.health;
                segment.steady_ms += interval;
            }
            if tick.alive && tick.health == 1 {
                segment.at_one_hp_ticks += 1;
            }
        }

        self.last = Some(tick);
        if window_over {
            self.close_action();
            if let Some(tick) = self.last.clone() {
                self.open_segment(&tick);
            }
        }
    }

    fn diff_fields(&mut self, previous: &Tick, tick: &Tick) {
        let elapsed = self.elapsed(tick.ts_ms);
        if let (Some(before), Some(after)) = (&previous.item, &tick.item) {
            if before.name == after.name {
                for (field, change) in changed_fields(&before.raw, &after.raw) {
                    println!("[{:>7.2}s] ITEM  {}.{} {}", elapsed, after.name, field, change);
                    *self.item_field_changes.entry(field).or_insert(0) += 1;
                }
            }
        }
        for (field, change) in changed_fields(&previous.hero_fields, &tick.hero_fields) {
            println!("[{:>7.2}s] HERO  {} {}", elapsed, field, change);
            *self.hero_field_changes.entry(field).or_insert(0) += 1;
        }
    }

    fn print_item(&self, tick: &Tick) {
        match &tick.item {
            Some(item) => println!(
                "            toggle item: {} in {} — {}",
                item.name,
                item.slot,
                Value::Object(item.raw.clone())
            ),
            None => println!("            toggle item: none held (Sabatons or Armlet)"),
        }
    }

    fn open_segment(&mut self, tick: &Tick) {
        self.segment = Some(Segment {
            item: tick.item.as_ref().map(|item| item.name.clone()),
            state: self.state,
            start_ms: tick.ts_ms,
            end_ms: tick.ts_ms,
            hp_start: tick.health,
            hp_end: tick.health,
            hp_min: tick.health,
            steady_hp: 0,
            steady_ms: 0,
            at_one_hp_ticks: 0,
        });
    }

    fn close_segment(&mut self) {
        if let Some(segment) = self.segment.take() {
            if segment.end_ms > segment.start_ms {
                self.segments.push(segment);
            }
        }
    }

    fn close_action(&mut self) {
        let Some(action) = self.pending.take() else {
            return;
        };

        let mut latency_ms = None;
        let mut settle_ms = None;
        let mut previous_max = action.max_before;
        for (ts, _, max, _) in &action.samples {
            if *max != previous_max {
                let since_press = ts.saturating_sub(action.first_press_ms);
                latency_ms.get_or_insert(since_press);
                settle_ms = Some(since_press);
            }
            previous_max = *max;
        }

        let (hp_end, max_end, pct_end) = action
            .samples
            .last()
            .map(|(_, hp, max, pct)| (*hp, *max, *pct))
            .unwrap_or((action.hp_before, action.max_before, action.pct_before));
        let result = ActionResult {
            item: action.item,
            at_ms: action.first_press_ms,
            presses: action.presses,
            press_gap_ms: action.last_press_ms - action.first_press_ms,
            with_modifier: action.with_modifier,
            hp_before: action.hp_before,
            hp_after_1s: action
                .samples
                .iter()
                .find(|(ts, ..)| ts.saturating_sub(action.first_press_ms) >= TRICK_READ_MS)
                .map(|(_, hp, ..)| *hp),
            hp_min: action
                .samples
                .iter()
                .map(|(_, hp, ..)| *hp)
                .min()
                .unwrap_or(action.hp_before),
            hp_end,
            max_before: action.max_before,
            max_min: action
                .samples
                .iter()
                .map(|(_, _, max, _)| *max)
                .min()
                .unwrap_or(action.max_before),
            max_end,
            pct_before: action.pct_before,
            pct_end,
            latency_ms,
            settle_ms,
            died: action.died,
        };

        if let Some(state) = result.inferred_state() {
            self.state = Some(state);
            if let Some(tick) = &self.last {
                if let Some(item) = &tick.item {
                    self.item_snapshots.record(state, item.raw.clone());
                    self.hero_snapshots.record(state, tick.hero_fields.clone());
                }
            }
        }

        println!(
            "[{:>7.2}s] DONE  {}× press{} — max {} → {} ({:+}, lowest {}), hp {} → {} after 1s → {} ({}% → {}%), first move {}, settled {} → toggle {}",
            self.elapsed(result.at_ms + self.window_ms),
            result.presses,
            if result.with_modifier { " +modifier" } else { "" },
            result.max_before,
            result.max_end,
            result.net_max(),
            result.max_min,
            result.hp_before,
            result
                .hp_after_1s
                .map_or_else(|| String::from("?"), |hp| hp.to_string()),
            result.hp_end,
            result.pct_before,
            result.pct_end,
            format_ms(result.latency_ms),
            format_ms(result.settle_ms),
            state_label(result.inferred_state().or(self.state))
        );
        if result.died {
            println!("            ⚠ died inside this action's window");
        }

        self.results.push(result);
    }

    fn report(&mut self) {
        self.close_action();
        self.close_segment();

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

        println!("\nactions ({}):", self.results.len());
        println!(
            "  {:>8}  {:<24}  {:>7}  {:>6}  {:>6}  {:>7}  {:>6}  {:>6}  {:>6}  {:>7}  {:>8}  {:>8}  {}",
            "at", "item", "presses", "max0", "max1", "Δmax", "hp0", "hp+1s", "gain", "pct", "1st move", "settled", "state"
        );
        for result in &self.results {
            println!(
                "  {:>7.2}s  {:<24}  {:>4}{:<3}  {:>6}  {:>6}  {:>+7}  {:>6}  {:>6}  {:>6}  {:>3}→{:<3}  {:>8}  {:>8}  {}{}",
                self.elapsed(result.at_ms),
                result.item,
                result.presses,
                if result.with_modifier { "+m" } else { "" },
                result.max_before,
                result.max_end,
                result.net_max(),
                result.hp_before,
                result
                    .hp_after_1s
                    .map_or_else(|| String::from("?"), |hp| hp.to_string()),
                result
                    .trick_gain()
                    .map_or_else(|| String::from("?"), |gain| format!("{:+}", gain)),
                result.pct_before,
                result.pct_end,
                format_ms(result.latency_ms),
                format_ms(result.settle_ms),
                state_label(result.inferred_state()),
                if result.died { "  DIED" } else { "" }
            );
            if result.presses > 1 {
                println!(
                    "            double tap over {}ms; max dipped to {} before settling",
                    result.press_gap_ms, result.max_min
                );
            }
        }

        // Question 1: the trick, per item, split by how low HP was.
        for item in TOGGLE_ITEMS {
            let taps: Vec<&ActionResult> = self
                .results
                .iter()
                .filter(|result| result.item == *item && result.presses > 1)
                .collect();
            if taps.is_empty() {
                continue;
            }
            println!("\n{item}: off-on double taps");
            for tap in taps {
                println!(
                    "  at hp {:>5} ({:>3}%): {:+} hp a second later, lowest {}",
                    tap.hp_before,
                    tap.pct_before,
                    tap.trick_gain().unwrap_or(0),
                    tap.hp_min
                );
            }
        }

        // Questions 2 and 5: drain and regen, by inferred state.
        println!("\nsteady stretches between actions (max_health still):");
        println!(
            "  {:>8}  {:>6}  {:<24}  {:>5}  {:>6}  {:>6}  {:>6}  {:>8}  {}",
            "at", "length", "item", "state", "hp0", "hp1", "min", "HP/s", "ticks at 1 HP"
        );
        for segment in &self.segments {
            println!(
                "  {:>7.2}s  {:>5.1}s  {:<24}  {:>5}  {:>6}  {:>6}  {:>6}  {:>8}  {}",
                self.elapsed(segment.start_ms),
                segment.end_ms.saturating_sub(segment.start_ms) as f64 / 1000.0,
                segment.item.as_deref().unwrap_or("—"),
                state_label(segment.state),
                segment.hp_start,
                segment.hp_end,
                segment.hp_min,
                segment
                    .rate_hp_per_sec()
                    .map_or_else(|| String::from("—"), |rate| format!("{:+.1}", rate)),
                segment.at_one_hp_ticks
            );
        }

        // Question 3: lethality.
        println!(
            "\nlowest hp while holding a toggle item: {}",
            self.min_hp_with_item
                .map_or_else(|| String::from("—"), |hp| hp.to_string())
        );
        if self.deaths.is_empty() {
            println!("deaths: none");
        } else {
            for (at, last_hp, state) in &self.deaths {
                println!(
                    "death at {:.2}s — last hp {}, toggle {}",
                    self.elapsed(*at),
                    last_hp,
                    state_label(*state)
                );
            }
        }

        // Question 4: does anything in the payload carry the toggle state?
        println!(
            "\nitem fields that ever changed: {}",
            format_counts(&self.item_field_changes)
        );
        println!(
            "hero fields that ever changed: {}",
            format_counts(&self.hero_field_changes)
        );
        let item_fields = self.item_snapshots.separating_fields();
        let hero_fields = self.hero_snapshots.separating_fields();
        println!(
            "snapshots after a toggle: {} on, {} off",
            self.item_snapshots.on.len(),
            self.item_snapshots.off.len()
        );
        if self.item_snapshots.on.is_empty() || self.item_snapshots.off.is_empty() {
            println!("→ need at least one toggle each way to compare states");
        } else if item_fields.is_empty() && hero_fields.is_empty() {
            println!("→ no item or hero field tells on from off; infer it from the max_health sign");
        } else {
            println!(
                "→ fields that DO separate on from off: item {:?}, hero {:?}",
                item_fields, hero_fields
            );
        }
    }
}

fn changed_fields(before: &Map<String, Value>, after: &Map<String, Value>) -> Vec<(String, String)> {
    let mut keys: BTreeSet<&String> = before.keys().collect();
    keys.extend(after.keys());
    keys.into_iter()
        .filter_map(|key| {
            let old = before.get(key);
            let new = after.get(key);
            (old != new).then(|| {
                (
                    key.clone(),
                    format!(
                        "{} → {}",
                        old.map_or_else(|| String::from("<absent>"), |v| v.to_string()),
                        new.map_or_else(|| String::from("<absent>"), |v| v.to_string())
                    ),
                )
            })
        })
        .collect()
}

fn state_label(state: Option<bool>) -> &'static str {
    match state {
        Some(true) => "ON",
        Some(false) => "OFF",
        None => "?",
    }
}

fn format_ms(ms: Option<u128>) -> String {
    ms.map_or_else(|| String::from("—"), |ms| format!("{ms}ms"))
}

fn format_counts(counts: &BTreeMap<String, u32>) -> String {
    if counts.is_empty() {
        return String::from("—");
    }
    counts
        .iter()
        .map(|(name, count)| format!("{}×{}", name, count))
        .collect::<Vec<_>>()
        .join(", ")
}

/// rdev's name for a configured key: `z` → `KeyZ`, `0` → `Num0`.
fn rdev_label(key: &str) -> String {
    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => format!("Key{}", c.to_ascii_uppercase()),
        (Some(c), None) if c.is_ascii_digit() => format!("Num{c}"),
        _ => key.to_string(),
    }
}

/// The capture file: every payload verbatim, plus the key presses, so a session
/// can be replayed exactly.
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
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&self.path) {
            let _ = writeln!(file, "{}", line);
        }
    }

    /// Dota pretty-prints its payloads; re-serialising compacts each back to one
    /// line. Nothing is dropped.
    fn record_payload(&self, ts_ms: u128, payload: &Value) {
        self.write(&format!(
            "{{\"probe_ts_ms\":{},\"payload\":{}}}",
            ts_ms,
            serde_json::to_string(payload).unwrap_or_else(|_| String::from("null"))
        ));
    }

    fn record_key(&self, ts_ms: u128, field: &str, key: &str) {
        self.write(&format!(
            "{{\"probe_ts_ms\":{},\"{}\":\"{}\"}}",
            ts_ms, field, key
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

/// Watch the slot keys and modifiers with a plain listener — `rdev::listen`
/// observes, unlike the app's `rdev::grab`, so this never swallows a key.
fn watch_keys(slot_keys: Vec<String>, shared: Arc<Mutex<Shared>>) {
    std::thread::spawn(move || {
        let mut held: BTreeSet<String> = BTreeSet::new();
        let result = rdev::listen(move |event| {
            let (label, pressed) = match event.event_type {
                rdev::EventType::KeyPress(key) => (format!("{:?}", key), true),
                rdev::EventType::KeyRelease(key) => (format!("{:?}", key), false),
                _ => return,
            };
            let watched =
                slot_keys.contains(&label) || MODIFIER_LABELS.contains(&label.as_str());
            if !watched {
                return;
            }

            // Held keys autorepeat; only the first press of a hold is a press.
            if pressed && !held.insert(label.clone()) {
                return;
            }
            if !pressed {
                held.remove(&label);
            }

            let ts_ms = now_ms();
            if let Ok(mut shared) = shared.lock() {
                if let Some(recorder) = &shared.recorder {
                    recorder.record_key(ts_ms, if pressed { "key" } else { "key_up" }, &label);
                }
                if pressed {
                    shared.analyzer.on_key(ts_ms, &label);
                } else {
                    shared.analyzer.on_key_up(&label);
                }
            }
        });
        if let Err(error) = result {
            eprintln!("key watch unavailable ({:?}) — running without key timing", error);
        }
    });
}

fn replay(path: &str, mut analyzer: Analyzer) {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("could not open {path}: {error}");
            return;
        }
    };

    let mut synthetic_ts = 0u128;
    // A stream of JSON values, not a value per line: Dota pretty-prints, and the
    // app's own logs keep that layout.
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
                } else if let Some(key) = value.get("key_up").and_then(|key| key.as_str()) {
                    analyzer.on_key_up(key);
                } else if let Some(payload) = value.get("payload") {
                    if let Some(tick) = parse_tick(ts_ms, payload) {
                        analyzer.on_tick(tick);
                    }
                }
            }
            // The app's own gsi_events log: bare payloads, no timing, no keys.
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
    analyzer.report();
}

struct Args {
    port: u16,
    keys: Vec<String>,
    replay: Option<String>,
    output_dir: String,
    verbose: bool,
    merge_ms: u128,
    window_ms: u128,
}

impl Args {
    fn parse() -> Self {
        let mut args = Self {
            port: 3000,
            keys: ["z", "x", "c", "v", "b", "n"].map(String::from).to_vec(),
            replay: None,
            output_dir: String::from("logs/sabatons_toggle"),
            verbose: false,
            merge_ms: DEFAULT_MERGE_MS,
            window_ms: DEFAULT_WINDOW_MS,
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
                "--keys" => {
                    index += 1;
                    if let Some(value) = raw.get(index) {
                        args.keys = value
                            .split(',')
                            .map(|part| part.trim().to_string())
                            .filter(|part| !part.is_empty())
                            .collect();
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
                "--merge-ms" => {
                    index += 1;
                    if let Some(value) = raw.get(index).and_then(|v| v.parse().ok()) {
                        args.merge_ms = value;
                    }
                }
                "--window-ms" => {
                    index += 1;
                    if let Some(value) = raw.get(index).and_then(|v| v.parse().ok()) {
                        args.window_ms = value;
                    }
                }
                "--no-keys" => args.keys.clear(),
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
    println!("Sabatons / Armlet toggle probe — records GSI and reports what a toggle looks like");
    println!();
    println!("Usage:");
    println!("  cargo run --example sabatons_toggle_probe -- [--port 3000] [--keys z,x,c,v,b,n]");
    println!("  cargo run --example sabatons_toggle_probe -- --replay <file.jsonl> [--verbose]");
    println!();
    println!("Options:");
    println!("  --port <N>           GSI listen port (default: 3000, the app's; close the app first)");
    println!("  --keys <a,b,...>     Item slot keys slot0..slot5 (default: z,x,c,v,b,n)");
    println!("  --no-keys            Skip the keyboard listener");
    println!("  --replay <PATH>      Analyse a capture instead of listening");
    println!("  --output <DIR>       Capture directory (default: logs/sabatons_toggle)");
    println!("  --merge-ms <N>       Presses closer than this are one action (default: 500)");
    println!("  --window-ms <N>      How long an action's effect is measured (default: 4000)");
    println!("  --verbose, -v        Print every tick");
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let slot_keys: Vec<String> = args.keys.iter().map(|key| rdev_label(key)).collect();
    let analyzer = Analyzer::new(args.verbose, args.merge_ms, args.window_ms, slot_keys.clone());

    if let Some(path) = &args.replay {
        replay(path, analyzer);
        return;
    }

    let shared = Arc::new(Mutex::new(Shared {
        analyzer,
        recorder: Recorder::create(&args.output_dir),
    }));

    if !slot_keys.is_empty() {
        watch_keys(slot_keys.clone(), Arc::clone(&shared));
        println!("watching slot keys: {}", args.keys.join(", "));
    }

    let app = Router::new()
        .route("/", post(receive))
        .with_state(Arc::clone(&shared));

    let address = format!("127.0.0.1:{}", args.port);
    let listener = match tokio::net::TcpListener::bind(&address).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("could not bind {address}: {error}");
            eprintln!("is the app (or a stray dota2-scripts.exe) still running on that port?");
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
