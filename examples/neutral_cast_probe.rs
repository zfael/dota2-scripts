//! Neutral item cast probe — what does each 7.42 active neutral do when pressed
//! the way the automation would press it?
//!
//! Nothing in `src/` depends on this. The 7.42 neutral pool brought actives the
//! automation wants to use, but some answers only a live game gives:
//!
//! 1. **Silence dispel.** Does a Flicker, Minotaur Horn or Witchbane cast clear
//!    `hero.silenced`, and how long after the cast tick? That decides whether they
//!    join Manta and Lotus in `src/actions/dispel.rs`.
//! 2. **Cast detection.** Does the `neutral0` cooldown edge (0 → N) show every
//!    cast, and how long after the press? The dispel planner settles on it.
//! 3. **Flicker's blink.** Flicker blinks forward at random, or in place if the
//!    hero took player damage in the last 3s. How far did each cast move us, next
//!    to how much damage came in during the 3s before?
//! 4. **Double-tap self-cast.** The automation casts self-cast items by tapping
//!    the item key twice. Does that cast Pollywog Charm on yourself (regen goes
//!    up), drop Seeds of Serenity at your feet (regen goes up), and put Witchbane
//!    on yourself (your silence clears)?
//! 5. **Cast cost.** Riftshadow Prism costs 4% of *current* HP. What does the
//!    cast tick take?
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
//! cargo run --example neutral_cast_probe -- --keys 0
//! ```
//!
//! `--keys` is your neutral slot key (`[keybindings] neutral0`). You need a way
//! to be silenced: an enemy hero with a silence (Drow Ranger's Gust, Silencer,
//! Skywrath Mage, Night Stalker) in a cheats lobby, with `-levelbots 30` or under
//! your control. Give yourself each neutral with `-item <name>`; it lands in the
//! neutral slot. Wait ~3s between steps:
//!
//! | # | Item | Do this | What it measures |
//! |---|---|---|---|
//! | 1 | — | Stand still 5s | baseline, tick interval, regen |
//! | 2 | `item_flicker` | Cast it, untouched for 4s+ | cast edge, latency, a forward blink |
//! | 3 | `item_flicker` | Take a hit, then cast it within 3s | the blink-in-place rule |
//! | 4 | `item_flicker` | Get silenced, cast it | **does it clear silence** |
//! | 5 | `item_minotaur_horn` | Get silenced, cast it (double tap) | **does it clear silence**; magic immunity |
//! | 6 | `item_heavy_blade` (Witchbane) | Get silenced, double tap | **does a double tap land on you and clear silence** |
//! | 7 | `item_heavy_blade` | Not silenced, double tap | does a double tap cast at all |
//! | 8 | `item_polliwog_charm` | Lose some HP, double tap, stand still | **self-cast lands**: regen before vs after |
//! | 9 | `item_seeds_of_serenity` | Lose some HP, double tap, stand still | **does it drop at your feet**: regen before vs after |
//! | 10 | `item_riftshadow_prism` | Cast at full HP, then at half HP | cast cost against current HP |
//! | 11 | `item_demonicon` (Book of the Dead) | Cast it | cast edge only |
//! | 12 | any | Get silenced and wait it out, no cast | a silence that expires on its own |
//!
//! Every payload is written verbatim to `logs/neutral_cast/`, so the session can
//! be re-analysed without playing again:
//!
//! ```powershell
//! cargo run --example neutral_cast_probe -- --replay logs/neutral_cast/probe_....jsonl --verbose
//! ```

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{extract::State, routing::post, Router};
use serde_json::Value;

/// Presses closer together than this are one action: a double tap.
const DEFAULT_MERGE_MS: u128 = 500;

/// A press further back than this did not cause the cast.
const PRESS_ATTRIBUTION_MS: u128 = 2000;

/// How long after a cast its effects are still read: the regen after, a silence
/// clearing, the blink distance.
const DEFAULT_FOLLOW_MS: u128 = 3000;

/// A silence that ends this soon after a cast was ended by the cast.
const DISPEL_ATTRIBUTION_MS: u128 = 1500;

/// Flicker's rule looks back this far for player damage.
const RECENT_DAMAGE_MS: u128 = 3000;

/// How much tick history is kept for the "before" readings.
const HISTORY_MS: u128 = 6000;

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
struct Tick {
    ts_ms: u128,
    hero: String,
    alive: bool,
    health: i64,
    max_health: i64,
    xpos: i64,
    ypos: i64,
    silenced: bool,
    muted: bool,
    hexed: bool,
    stunned: bool,
    magicimmune: bool,
    has_debuff: bool,
    neutral: String,
    cooldown: i64,
    charges: i64,
    can_cast: bool,
}

/// Pull the fields we care about out of one raw GSI payload. Returns `None` for
/// the menu / draft / spectator payloads that carry no hero.
fn parse_tick(ts_ms: u128, payload: &Value) -> Option<Tick> {
    let hero = payload.get("hero")?;
    let name = hero.get("name")?.as_str()?.to_string();
    if name.is_empty() || name == "empty" {
        return None;
    }
    let neutral = payload.get("items").and_then(|items| items.get("neutral0"));

    Some(Tick {
        ts_ms,
        hero: name,
        alive: as_bool(hero.get("alive")),
        health: as_i64(hero.get("health")),
        max_health: as_i64(hero.get("max_health")),
        xpos: as_i64(hero.get("xpos")),
        ypos: as_i64(hero.get("ypos")),
        silenced: as_bool(hero.get("silenced")),
        muted: as_bool(hero.get("muted")),
        hexed: as_bool(hero.get("hexed")),
        stunned: as_bool(hero.get("stunned")),
        magicimmune: as_bool(hero.get("magicimmune")),
        has_debuff: as_bool(hero.get("has_debuff")),
        neutral: neutral
            .and_then(|item| item.get("name"))
            .and_then(|name| name.as_str())
            .unwrap_or("empty")
            .to_string(),
        cooldown: as_i64(neutral.and_then(|item| item.get("cooldown"))),
        charges: as_i64(neutral.and_then(|item| item.get("charges"))),
        can_cast: as_bool(neutral.and_then(|item| item.get("can_cast"))),
    })
}

fn distance(a: &Tick, b: &Tick) -> f64 {
    let dx = (a.xpos - b.xpos) as f64;
    let dy = (a.ypos - b.ypos) as f64;
    (dx * dx + dy * dy).sqrt()
}

/// HP per second across consecutive ticks where `max_health` did not move, so
/// a level or an item is not read as regen. Damage still counts, which is why
/// the protocol asks you to stand still.
fn steady_rate<'a>(ticks: impl Iterator<Item = &'a Tick>) -> Option<f64> {
    let mut hp = 0i64;
    let mut ms = 0u128;
    let mut previous: Option<&Tick> = None;
    for tick in ticks {
        if let Some(before) = previous {
            if before.alive && tick.alive && before.max_health == tick.max_health {
                hp += tick.health - before.health;
                ms += tick.ts_ms.saturating_sub(before.ts_ms);
            }
        }
        previous = Some(tick);
    }
    (ms >= 500).then(|| hp as f64 * 1000.0 / ms as f64)
}

#[derive(Clone, Debug)]
struct PressBurst {
    first_ms: u128,
    last_ms: u128,
    presses: u32,
    with_modifier: bool,
    item: String,
    consumed: bool,
}

/// One neutral cast, from the cooldown edge to the end of its follow window.
#[derive(Debug)]
struct Cast {
    item: String,
    cast_ms: u128,
    presses: Option<u32>,
    press_gap_ms: Option<u128>,
    with_modifier: bool,
    latency_ms: Option<u128>,
    before: Tick,
    hp_cost: i64,
    recent_damage: i64,
    regen_before: Option<f64>,
    follow: Vec<Tick>,
}

impl Cast {
    fn silence_cleared_ms(&self) -> Option<u128> {
        if !self.before.silenced {
            return None;
        }
        self.follow
            .iter()
            .find(|tick| !tick.silenced)
            .map(|tick| tick.ts_ms.saturating_sub(self.cast_ms))
    }

    fn debuff_cleared_ms(&self) -> Option<u128> {
        if !self.before.has_debuff {
            return None;
        }
        self.follow
            .iter()
            .find(|tick| !tick.has_debuff)
            .map(|tick| tick.ts_ms.saturating_sub(self.cast_ms))
    }

    fn magic_immune_after(&self) -> bool {
        !self.before.magicimmune && self.follow.iter().any(|tick| tick.magicimmune)
    }

    /// The farthest the hero got from where it stood before the press, in the
    /// first second after the cast. A blink shows as a single big jump.
    fn moved(&self) -> f64 {
        self.follow
            .iter()
            .take_while(|tick| tick.ts_ms.saturating_sub(self.cast_ms) <= 1000)
            .map(|tick| distance(&self.before, tick))
            .fold(0.0, f64::max)
    }

    fn regen_after(&self) -> Option<f64> {
        steady_rate(self.follow.iter())
    }
}

struct Analyzer {
    verbose: bool,
    merge_ms: u128,
    follow_ms: u128,
    neutral_keys: Vec<String>,
    modifier_held: bool,
    session_start_ms: Option<u128>,
    history: VecDeque<Tick>,
    burst: Option<PressBurst>,
    open_casts: Vec<Cast>,
    casts: Vec<Cast>,
    silence_started_ms: Option<u128>,
    silences: Vec<(u128, u128, Option<String>)>,
    bursts_without_cast: Vec<PressBurst>,
    intervals_ms: Vec<u128>,
    items_seen: BTreeSet<String>,
    ticks_seen: u64,
}

impl Analyzer {
    fn new(verbose: bool, merge_ms: u128, follow_ms: u128, neutral_keys: Vec<String>) -> Self {
        Self {
            verbose,
            merge_ms,
            follow_ms,
            neutral_keys,
            modifier_held: false,
            session_start_ms: None,
            history: VecDeque::new(),
            burst: None,
            open_casts: Vec::new(),
            casts: Vec::new(),
            silence_started_ms: None,
            silences: Vec::new(),
            bursts_without_cast: Vec::new(),
            intervals_ms: Vec::new(),
            items_seen: BTreeSet::new(),
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
        if !self.neutral_keys.iter().any(|key| key == label) {
            return;
        }

        let item = self
            .history
            .back()
            .map(|tick| tick.neutral.clone())
            .unwrap_or_else(|| String::from("?"));
        let modifier = self.modifier_held;
        let elapsed = self.elapsed(ts_ms);

        if let Some(burst) = self.burst.as_mut() {
            if ts_ms.saturating_sub(burst.last_ms) <= self.merge_ms {
                burst.presses += 1;
                burst.last_ms = ts_ms;
                burst.with_modifier |= modifier;
                println!(
                    "[{:>7.2}s] PRESS {} again — {} taps",
                    elapsed, label, burst.presses
                );
                return;
            }
        }

        self.retire_burst();
        println!(
            "[{:>7.2}s] PRESS {}{} — neutral slot holds {}",
            elapsed,
            label,
            if modifier { " with modifier" } else { "" },
            item
        );
        self.burst = Some(PressBurst {
            first_ms: ts_ms,
            last_ms: ts_ms,
            presses: 1,
            with_modifier: modifier,
            item,
            consumed: false,
        });
    }

    /// A burst that never produced a cast is a finding too: a double tap that
    /// only armed a ground-target item, or a press while muted.
    fn retire_burst(&mut self) {
        if let Some(burst) = self.burst.take() {
            if !burst.consumed {
                println!(
                    "            ↳ {}× tap on {} produced no cast",
                    burst.presses, burst.item
                );
                self.bursts_without_cast.push(burst);
            }
        }
    }

    fn on_tick(&mut self, tick: Tick) {
        self.session_start_ms.get_or_insert(tick.ts_ms);
        self.ticks_seen += 1;
        if tick.neutral != "empty" {
            self.items_seen.insert(tick.neutral.clone());
        }

        let Some(previous) = self.history.back().cloned() else {
            println!(
                "[{:>7.2}s] START hero {} — hp {} / {}, neutral {}, silenced {}",
                self.elapsed(tick.ts_ms),
                tick.hero,
                tick.health,
                tick.max_health,
                tick.neutral,
                tick.silenced
            );
            self.history.push_back(tick);
            return;
        };

        let interval = tick.ts_ms.saturating_sub(previous.ts_ms);
        if interval > 0 {
            self.intervals_ms.push(interval);
        }
        let elapsed = self.elapsed(tick.ts_ms);

        if self.verbose {
            println!(
                "[{:>7.2}s] tick  hp {:>5} ({:>+4})  pos {:>6},{:>6}  {} cd {} ch {}{}{}{}{}",
                elapsed,
                tick.health,
                tick.health - previous.health,
                tick.xpos,
                tick.ypos,
                tick.neutral,
                tick.cooldown,
                tick.charges,
                if tick.silenced { " SILENCED" } else { "" },
                if tick.muted { " MUTED" } else { "" },
                if tick.magicimmune { " IMMUNE" } else { "" },
                if tick.has_debuff { " debuff" } else { "" }
            );
        }

        if tick.neutral != previous.neutral {
            println!(
                "[{:>7.2}s] ITEM  neutral slot {} → {} (cd {}, charges {}, can_cast {})",
                elapsed, previous.neutral, tick.neutral, tick.cooldown, tick.charges, tick.can_cast
            );
        }

        // Status transitions worth seeing live.
        for (label, was, is) in [
            ("silenced", previous.silenced, tick.silenced),
            ("muted", previous.muted, tick.muted),
            ("hexed", previous.hexed, tick.hexed),
            ("stunned", previous.stunned, tick.stunned),
            ("magicimmune", previous.magicimmune, tick.magicimmune),
            ("has_debuff", previous.has_debuff, tick.has_debuff),
        ] {
            if was != is {
                println!("[{:>7.2}s] HERO  {} {} → {}", elapsed, label, was, is);
            }
        }
        for cast in self.open_casts.iter_mut() {
            cast.follow.push(tick.clone());
        }

        // The cast edge: cooldown starts, or a charge is spent, on the same item.
        // Read before the silence transition below: a dispel usually clears the
        // silence on the very tick the cooldown appears.
        let same_item = tick.neutral == previous.neutral && tick.neutral != "empty";
        let cooldown_edge = previous.cooldown == 0 && tick.cooldown > 0;
        let charge_spent = tick.charges < previous.charges;
        if same_item && (cooldown_edge || charge_spent) {
            self.open_cast(&previous, &tick);
        }

        if !previous.silenced && tick.silenced {
            self.silence_started_ms = Some(tick.ts_ms);
        }
        if previous.silenced && !tick.silenced {
            let started = self.silence_started_ms.take().unwrap_or(previous.ts_ms);
            let cleared_by = self
                .open_casts
                .iter()
                .chain(self.casts.iter())
                .filter(|cast| {
                    cast.before.silenced
                        && tick.ts_ms.saturating_sub(cast.cast_ms) <= DISPEL_ATTRIBUTION_MS
                })
                .map(|cast| cast.item.clone())
                .last();
            println!(
                "            ↳ silence lasted {:.2}s — {}",
                tick.ts_ms.saturating_sub(started) as f64 / 1000.0,
                match &cleared_by {
                    Some(item) => format!("cleared by {item}"),
                    None => String::from("expired (no cast just before)"),
                }
            );
            self.silences.push((started, tick.ts_ms, cleared_by));
        }

        self.history.push_back(tick);
        while self
            .history
            .front()
            .zip(self.history.back())
            .is_some_and(|(front, back)| back.ts_ms.saturating_sub(front.ts_ms) > HISTORY_MS)
        {
            self.history.pop_front();
        }

        let now = self.history.back().map(|tick| tick.ts_ms).unwrap_or(0);
        if self
            .burst
            .as_ref()
            .is_some_and(|burst| now.saturating_sub(burst.last_ms) > PRESS_ATTRIBUTION_MS)
        {
            self.retire_burst();
        }

        let follow_ms = self.follow_ms;
        let (done, open): (Vec<Cast>, Vec<Cast>) = std::mem::take(&mut self.open_casts)
            .into_iter()
            .partition(|cast| now.saturating_sub(cast.cast_ms) >= follow_ms);
        self.open_casts = open;
        for cast in done {
            self.close_cast(cast);
        }
    }

    fn open_cast(&mut self, previous: &Tick, tick: &Tick) {
        let burst = self.burst.as_mut().filter(|burst| {
            !burst.consumed && tick.ts_ms.saturating_sub(burst.first_ms) <= PRESS_ATTRIBUTION_MS
        });
        let (presses, gap, modifier, latency, reference_ms) = match burst {
            Some(burst) => {
                burst.consumed = true;
                (
                    Some(burst.presses),
                    Some(burst.last_ms - burst.first_ms),
                    burst.with_modifier,
                    Some(tick.ts_ms.saturating_sub(burst.first_ms)),
                    burst.first_ms,
                )
            }
            None => (None, None, false, None, previous.ts_ms),
        };

        // "Before" is the last tick before the press, so the cast's own effects
        // (a blink, a dispel) are never inside it.
        let before = self
            .history
            .iter()
            .rev()
            .find(|candidate| candidate.ts_ms <= reference_ms)
            .unwrap_or(previous)
            .clone();
        let window: Vec<&Tick> = self
            .history
            .iter()
            .filter(|candidate| {
                candidate.ts_ms <= reference_ms
                    && reference_ms.saturating_sub(candidate.ts_ms) <= RECENT_DAMAGE_MS
            })
            .collect();
        let recent_damage = window
            .windows(2)
            .filter(|pair| pair[0].max_health == pair[1].max_health)
            .map(|pair| (pair[0].health - pair[1].health).max(0))
            .sum();
        let regen_before = steady_rate(window.into_iter());

        println!(
            "[{:>7.2}s] CAST  {} — {}, hp {} → {} on the cast tick{}",
            self.elapsed(tick.ts_ms),
            tick.neutral,
            match (presses, latency) {
                (Some(presses), Some(latency)) => format!("{presses}× tap, {latency}ms after the first"),
                _ => String::from("no press seen"),
            },
            previous.health,
            tick.health,
            if before.silenced { ", while SILENCED" } else { "" }
        );

        self.open_casts.push(Cast {
            item: tick.neutral.clone(),
            cast_ms: tick.ts_ms,
            presses,
            press_gap_ms: gap,
            with_modifier: modifier,
            latency_ms: latency,
            hp_cost: previous.health - tick.health,
            before,
            recent_damage,
            regen_before,
            follow: vec![tick.clone()],
        });
    }

    fn close_cast(&mut self, cast: Cast) {
        println!(
            "[{:>7.2}s] DONE  {} — moved {:.0}u, recent damage {}, regen {} → {} HP/s{}{}{}",
            self.elapsed(cast.cast_ms + self.follow_ms),
            cast.item,
            cast.moved(),
            cast.recent_damage,
            format_rate(cast.regen_before),
            format_rate(cast.regen_after()),
            match (cast.before.silenced, cast.silence_cleared_ms()) {
                (true, Some(ms)) => format!(", silence cleared +{ms}ms"),
                (true, None) => String::from(", SILENCE NOT CLEARED"),
                _ => String::new(),
            },
            match (cast.before.has_debuff, cast.debuff_cleared_ms()) {
                (true, Some(ms)) => format!(", debuffs cleared +{ms}ms"),
                (true, None) => String::from(", debuffs remain"),
                _ => String::new(),
            },
            if cast.magic_immune_after() { ", magic immune" } else { "" }
        );
        self.casts.push(cast);
    }

    fn report(&mut self) {
        for cast in std::mem::take(&mut self.open_casts) {
            self.close_cast(cast);
        }
        self.retire_burst();

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
        println!(
            "neutral items seen: {}",
            self.items_seen.iter().cloned().collect::<Vec<_>>().join(", ")
        );

        println!("\ncasts ({}):", self.casts.len());
        println!(
            "  {:>8}  {:<24}  {:>5}  {:>7}  {:>7}  {:>6}  {:>6}  {:>6}  {:>7}  {:>7}  {}",
            "at", "item", "taps", "latency", "silence", "moved", "dmg3s", "Δhp", "regen0", "regen1", "notes"
        );
        for cast in &self.casts {
            let mut notes = Vec::new();
            if cast.magic_immune_after() {
                notes.push(String::from("magic immune"));
            }
            match (cast.before.has_debuff, cast.debuff_cleared_ms()) {
                (true, Some(ms)) => notes.push(format!("debuffs cleared +{ms}ms")),
                (true, None) => notes.push(String::from("debuffs remain")),
                _ => {}
            }
            if cast.with_modifier {
                notes.push(String::from("modifier held"));
            }
            if let Some(gap) = cast.press_gap_ms.filter(|gap| *gap > 0) {
                notes.push(format!("taps {gap}ms apart"));
            }
            println!(
                "  {:>7.2}s  {:<24}  {:>5}  {:>7}  {:>7}  {:>5.0}u  {:>6}  {:>+6}  {:>7}  {:>7}  {}",
                self.elapsed(cast.cast_ms),
                cast.item,
                cast.presses.map_or_else(|| String::from("—"), |p| p.to_string()),
                cast.latency_ms
                    .map_or_else(|| String::from("—"), |ms| format!("{ms}ms")),
                match (cast.before.silenced, cast.silence_cleared_ms()) {
                    (true, Some(ms)) => format!("+{ms}ms"),
                    (true, None) => String::from("KEPT"),
                    _ => String::from("—"),
                },
                cast.moved(),
                cast.recent_damage,
                -cast.hp_cost,
                format_rate(cast.regen_before),
                format_rate(cast.regen_after()),
                notes.join(", ")
            );
        }

        // Question 1: per item, how many silenced casts cleared the silence.
        let mut dispels: BTreeMap<&str, (u32, u32)> = BTreeMap::new();
        for cast in self.casts.iter().filter(|cast| cast.before.silenced) {
            let entry = dispels.entry(cast.item.as_str()).or_insert((0, 0));
            entry.0 += 1;
            if cast
                .silence_cleared_ms()
                .is_some_and(|ms| ms <= DISPEL_ATTRIBUTION_MS)
            {
                entry.1 += 1;
            }
        }
        println!("\nsilence dispel (casts while silenced → silence gone within {DISPEL_ATTRIBUTION_MS}ms):");
        if dispels.is_empty() {
            println!("  no cast happened while silenced");
        }
        for (item, (casts, cleared)) in &dispels {
            println!(
                "  {:<24} {}/{} {}",
                item,
                cleared,
                casts,
                if cleared == casts { "→ dispels silence" } else if *cleared == 0 { "→ does NOT dispel silence" } else { "→ mixed; look at the timeline" }
            );
        }
        let expired = self.silences.iter().filter(|(_, _, by)| by.is_none()).count();
        println!("silences that expired on their own: {expired}");

        // Question 3: Flicker distance against recent damage.
        let flickers: Vec<&Cast> = self
            .casts
            .iter()
            .filter(|cast| cast.item == "item_flicker")
            .collect();
        if !flickers.is_empty() {
            println!("\nFlicker blinks:");
            for cast in flickers {
                println!(
                    "  moved {:>5.0}u after {:>4} damage in the 3s before{}",
                    cast.moved(),
                    cast.recent_damage,
                    if cast.before.silenced { " (silenced)" } else { "" }
                );
            }
        }

        // Question 4: did multi-tap casts land on the caster?
        println!("\ndouble-tap casts (regen before → after; silence):");
        let mut any = false;
        for cast in self.casts.iter().filter(|cast| cast.presses.unwrap_or(0) >= 2) {
            any = true;
            println!(
                "  {:<24} regen {} → {} HP/s{}",
                cast.item,
                format_rate(cast.regen_before),
                format_rate(cast.regen_after()),
                match (cast.before.silenced, cast.silence_cleared_ms()) {
                    (true, Some(ms)) => format!(", own silence cleared +{ms}ms"),
                    (true, None) => String::from(", own silence KEPT"),
                    _ => String::new(),
                }
            );
        }
        if !any {
            println!("  none");
        }
        if !self.bursts_without_cast.is_empty() {
            println!("\npresses that produced no cast:");
            for burst in &self.bursts_without_cast {
                println!(
                    "  {:>7.2}s  {}× tap on {}",
                    self.elapsed(burst.first_ms),
                    burst.presses,
                    burst.item
                );
            }
        }
    }
}

fn format_rate(rate: Option<f64>) -> String {
    rate.map_or_else(|| String::from("—"), |rate| format!("{:+.1}", rate))
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

/// Watch the neutral key and modifiers with a plain listener — `rdev::listen`
/// observes, unlike the app's `rdev::grab`, so this never swallows a key.
fn watch_keys(neutral_keys: Vec<String>, shared: Arc<Mutex<Shared>>) {
    std::thread::spawn(move || {
        let mut held: BTreeSet<String> = BTreeSet::new();
        let result = rdev::listen(move |event| {
            let (label, pressed) = match event.event_type {
                rdev::EventType::KeyPress(key) => (format!("{:?}", key), true),
                rdev::EventType::KeyRelease(key) => (format!("{:?}", key), false),
                _ => return,
            };
            let watched =
                neutral_keys.contains(&label) || MODIFIER_LABELS.contains(&label.as_str());
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
    follow_ms: u128,
}

impl Args {
    fn parse() -> Self {
        let mut args = Self {
            port: 3000,
            keys: vec![String::from("0")],
            replay: None,
            output_dir: String::from("logs/neutral_cast"),
            verbose: false,
            merge_ms: DEFAULT_MERGE_MS,
            follow_ms: DEFAULT_FOLLOW_MS,
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
                "--follow-ms" => {
                    index += 1;
                    if let Some(value) = raw.get(index).and_then(|v| v.parse().ok()) {
                        args.follow_ms = value;
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
    println!("Neutral item cast probe — records GSI and reports what each neutral cast did");
    println!();
    println!("Usage:");
    println!("  cargo run --example neutral_cast_probe -- [--port 3000] [--keys 0]");
    println!("  cargo run --example neutral_cast_probe -- --replay <file.jsonl> [--verbose]");
    println!();
    println!("Options:");
    println!("  --port <N>           GSI listen port (default: 3000, the app's; close the app first)");
    println!("  --keys <a,...>       Neutral slot key(s) to timestamp (default: 0)");
    println!("  --no-keys            Skip the keyboard listener");
    println!("  --replay <PATH>      Analyse a capture instead of listening");
    println!("  --output <DIR>       Capture directory (default: logs/neutral_cast)");
    println!("  --merge-ms <N>       Taps closer than this are one double tap (default: 500)");
    println!("  --follow-ms <N>      How long a cast's effects are read (default: 3000)");
    println!("  --verbose, -v        Print every tick");
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let neutral_keys: Vec<String> = args.keys.iter().map(|key| rdev_label(key)).collect();
    let analyzer = Analyzer::new(args.verbose, args.merge_ms, args.follow_ms, neutral_keys.clone());

    if let Some(path) = &args.replay {
        replay(path, analyzer);
        return;
    }

    let shared = Arc::new(Mutex::new(Shared {
        analyzer,
        recorder: Recorder::create(&args.output_dir),
    }));

    if !neutral_keys.is_empty() {
        watch_keys(neutral_keys.clone(), Arc::clone(&shared));
        println!("watching neutral key(s): {}", args.keys.join(", "));
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
