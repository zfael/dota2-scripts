//! Morphling Attribute Shift probe — can GSI tell us which shift is running,
//! and how far it has gone?
//!
//! Nothing in `src/` depends on this. It exists to answer, with measured
//! numbers from a live game, the questions the automation needs settled before
//! any of it is written:
//!
//! 1. **Direction.** `hero.max_health` is the only GSI field a shift moves:
//!    strength gain raises it, agility gain lowers it. Is that delta visible
//!    per tick at the 0.1s GSI throttle, and is it monotonic enough to read as
//!    "a strength shift is running right now"?
//! 2. **Toggle state.** Does `abilities.abilityN.ability_active` flip for
//!    `morphling_morph_agi` / `morphling_morph_str` while the toggle is on? If
//!    it does, it beats the delta signal — it has no latency and survives a
//!    plateau. If it does not, the delta is all we get.
//! 3. **Latency.** How long between the key press and the first tick that
//!    shows movement? That is the floor on how fast the automation can react
//!    and how fast it can know the shift actually started.
//! 4. **Granularity.** What HP does one strength point buy? The distinct
//!    per-tick deltas answer it (they should all be multiples of one constant),
//!    which is what turns a "shift me +N strength" config into an HP target.
//! 5. **Exhaustion.** When the attribute pool runs out, does `max_health` just
//!    plateau? That plateau is the stop signal when no HP target is set.
//! 6. **False positives.** A level up, an item, or a Power Treads swap also
//!    moves `max_health`. How do those look next to a real shift?
//!
//! The probe never presses anything. It listens.
//!
//! ## Capture protocol
//!
//! Run it on its own port next to the real app, so both get the same stream:
//! drop a second file in
//! `steamapps/common/dota 2 beta/game/dota/cfg/gamestate_integration/`, say
//! `gamestate_integration_morph_probe.cfg`, identical to the app's own but with
//! `"uri" "http://127.0.0.1:3100/"` and its own `"name"`.
//!
//! ```powershell
//! cargo run --example morphling_shift_probe -- --port 3100 --keys d,f
//! ```
//!
//! Then take Morphling into a demo game (`-wait 0` lobby or hero demo) and run
//! this script, pausing ~3s between steps so the episodes stay separable:
//!
//! | # | Do this | What it measures |
//! |---|---|---|
//! | 1 | Stand still 5s | baseline, tick interval |
//! | 2 | `F` (strength) for ~3s, then `F` again to stop | direction, rate, latency, whether a second press stops it |
//! | 3 | `D` (agility) until `max_health` is back, stop | the reverse, and whether current HP falls with it |
//! | 4 | `F` and let it run to exhaustion | the plateau |
//! | 5 | `D` and let it run to exhaustion | the other plateau; the size of the whole pool |
//! | 6 | Level up, then buy any HP item | non-shift `max_health` moves |
//! | 7 | Let creeps hit you mid-shift | damage on top of a shift |
//! | 8 | Start `F`, then press `D` without stopping first | does one toggle cancel the other |
//!
//! Every payload is written verbatim to `logs/morphling_shift/`, so the session
//! can be re-analysed without playing again:
//!
//! ```powershell
//! cargo run --example morphling_shift_probe -- --replay logs/morphling_shift/probe_....jsonl --verbose
//! ```
//!
//! `--replay` also accepts the app's own `logs/gsi_events/*.jsonl` (those carry
//! no timestamps, so ticks are assumed 100ms apart and no key data exists).

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{extract::State, routing::post, Router};
use serde_json::Value;

/// Distinct `max_health` values stop counting as one episode after this long
/// with no movement. Two GSI ticks at the documented 0.1s throttle, with room
/// for a slow one.
const DEFAULT_FLAT_GAP_MS: u128 = 700;

/// A key press further back than this is not what started the episode.
const KEY_ATTRIBUTION_MS: u128 = 3000;

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[derive(Clone, Debug)]
struct AbilitySnapshot {
    name: String,
    active: bool,
    can_cast: bool,
    cooldown: i64,
    level: i64,
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
    level: i64,
    abilities: Vec<AbilitySnapshot>,
}

fn as_i64(value: Option<&Value>) -> i64 {
    value.and_then(|v| v.as_i64()).unwrap_or(0)
}

fn as_bool(value: Option<&Value>) -> bool {
    value.and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Pull the fields we care about out of one raw GSI payload. Returns `None` for
/// the menu / draft / spectator payloads that carry no hero.
fn parse_tick(ts_ms: u128, payload: &Value) -> Option<Tick> {
    let hero = payload.get("hero")?;
    let name = hero.get("name")?.as_str()?.to_string();
    if name.is_empty() || name == "empty" {
        return None;
    }

    let mut abilities = Vec::new();
    if let Some(block) = payload.get("abilities").and_then(|a| a.as_object()) {
        // The ability block is an object keyed `ability0`..`abilityN`, and the
        // key order is the ability panel order — worth keeping, since it tells
        // us which slot each morph sits in.
        let mut keys: Vec<&String> = block.keys().collect();
        keys.sort();
        for key in keys {
            let ability = &block[key];
            let Some(ability_name) = ability.get("name").and_then(|n| n.as_str()) else {
                continue;
            };
            if ability_name.is_empty() {
                continue;
            }
            abilities.push(AbilitySnapshot {
                name: ability_name.to_string(),
                active: as_bool(ability.get("ability_active")),
                can_cast: as_bool(ability.get("can_cast")),
                cooldown: as_i64(ability.get("cooldown")),
                level: as_i64(ability.get("level")),
            });
        }
    }

    Some(Tick {
        ts_ms,
        clock_time: as_i64(payload.get("map").and_then(|m| m.get("clock_time"))),
        hero: name,
        alive: as_bool(hero.get("alive")),
        health: as_i64(hero.get("health")),
        max_health: as_i64(hero.get("max_health")),
        health_percent: as_i64(hero.get("health_percent")),
        level: as_i64(hero.get("level")),
        abilities,
    })
}

/// One run of `max_health` moving in a single direction.
#[derive(Debug)]
struct Episode {
    /// `+1` strength-ward (max HP rising), `-1` agility-ward.
    direction: i8,
    start_ms: u128,
    end_ms: u128,
    start_max_hp: i64,
    end_max_hp: i64,
    /// Current HP at both ends — a strength shift should carry current HP up
    /// with max, and the agility shift is the one that can leave us at a sliver.
    start_hp: i64,
    end_hp: i64,
    ticks: u32,
    /// Every non-zero per-tick step, for the granularity histogram.
    steps: Vec<i64>,
    /// How many ticks inside the episode reported each ability as active.
    active_counts: BTreeMap<String, u32>,
    /// The key press that plausibly started this, and how long before the first
    /// visible movement it landed.
    key_lead: Option<(String, u128)>,
    level_changed: bool,
}

impl Episode {
    fn hp_delta(&self) -> i64 {
        self.end_max_hp - self.start_max_hp
    }

    fn duration_ms(&self) -> u128 {
        self.end_ms.saturating_sub(self.start_ms)
    }

    fn rate_hp_per_sec(&self) -> f64 {
        let seconds = self.duration_ms() as f64 / 1000.0;
        if seconds <= 0.0 {
            return 0.0;
        }
        self.hp_delta().abs() as f64 / seconds
    }

    /// A shift moves `max_health` a little at a time over many ticks. An item,
    /// a level, or a Power Treads swap moves it once, by a lot. Only the former
    /// belongs in the rate and granularity numbers.
    fn looks_like_a_shift(&self) -> bool {
        self.ticks >= 3
    }
}

struct Analyzer {
    verbose: bool,
    flat_gap_ms: u128,
    session_start_ms: Option<u128>,
    last: Option<Tick>,
    current: Option<Episode>,
    episodes: Vec<Episode>,
    last_key: Option<(String, u128)>,
    intervals_ms: Vec<u128>,
    ability_names: BTreeMap<String, u32>,
    /// Every distinct `max_health` seen, so the full shift pool is visible.
    min_max_hp: i64,
    max_max_hp: i64,
    ticks_seen: u64,
}

impl Analyzer {
    fn new(verbose: bool, flat_gap_ms: u128) -> Self {
        Self {
            verbose,
            flat_gap_ms,
            session_start_ms: None,
            last: None,
            current: None,
            episodes: Vec::new(),
            last_key: None,
            intervals_ms: Vec::new(),
            ability_names: BTreeMap::new(),
            min_max_hp: i64::MAX,
            max_max_hp: 0,
            ticks_seen: 0,
        }
    }

    fn elapsed(&self, ts_ms: u128) -> f64 {
        match self.session_start_ms {
            Some(start) => ts_ms.saturating_sub(start) as f64 / 1000.0,
            None => 0.0,
        }
    }

    fn on_key(&mut self, ts_ms: u128, key: &str) {
        self.session_start_ms.get_or_insert(ts_ms);
        println!("[{:>7.2}s] KEY   {}", self.elapsed(ts_ms), key);
        self.last_key = Some((key.to_string(), ts_ms));
    }

    fn on_tick(&mut self, tick: Tick) {
        self.session_start_ms.get_or_insert(tick.ts_ms);
        self.ticks_seen += 1;
        self.min_max_hp = self.min_max_hp.min(tick.max_health);
        self.max_max_hp = self.max_max_hp.max(tick.max_health);
        for ability in &tick.abilities {
            *self.ability_names.entry(ability.name.clone()).or_insert(0) += 1;
        }

        if !tick.alive {
            self.close_episode("hero died");
            self.last = Some(tick);
            return;
        }

        let Some(previous) = self.last.clone() else {
            println!(
                "[{:>7.2}s] START hero {} — max_hp {}, hp {} ({}%), level {}, clock {}",
                self.elapsed(tick.ts_ms),
                tick.hero,
                tick.max_health,
                tick.health,
                tick.health_percent,
                tick.level,
                tick.clock_time
            );
            self.print_ability_panel(&tick);
            self.last = Some(tick);
            return;
        };

        let interval = tick.ts_ms.saturating_sub(previous.ts_ms);
        if interval > 0 {
            self.intervals_ms.push(interval);
        }

        let step = tick.max_health - previous.max_health;
        let direction = match step.cmp(&0) {
            std::cmp::Ordering::Greater => 1i8,
            std::cmp::Ordering::Less => -1i8,
            std::cmp::Ordering::Equal => 0,
        };

        if self.verbose {
            println!(
                "[{:>7.2}s] tick  max_hp {:>6} ({:>+5})  hp {:>6} ({:>+5})  {}%  {}",
                self.elapsed(tick.ts_ms),
                tick.max_health,
                step,
                tick.health,
                tick.health - previous.health,
                tick.health_percent,
                active_ability_list(&tick)
            );
        }

        if direction == 0 {
            // A plateau long enough to mean the shift stopped — either toggled
            // off or the pool ran dry.
            let idle_for = self
                .current
                .as_ref()
                .map(|episode| tick.ts_ms.saturating_sub(episode.end_ms))
                .unwrap_or(0);
            if self.current.is_some() && idle_for >= self.flat_gap_ms {
                self.close_episode("max_health went flat");
            }
            self.last = Some(tick);
            return;
        }

        let same_direction = self
            .current
            .as_ref()
            .is_some_and(|episode| episode.direction == direction);
        if !same_direction {
            self.close_episode("direction reversed");
            self.open_episode(direction, &previous, &tick);
        }

        if let Some(episode) = self.current.as_mut() {
            episode.end_ms = tick.ts_ms;
            episode.end_max_hp = tick.max_health;
            episode.end_hp = tick.health;
            episode.ticks += 1;
            episode.steps.push(step);
            episode.level_changed |= tick.level != previous.level;
            for ability in &tick.abilities {
                if ability.active {
                    *episode
                        .active_counts
                        .entry(ability.name.clone())
                        .or_insert(0) += 1;
                }
            }
        }

        self.last = Some(tick);
    }

    fn open_episode(&mut self, direction: i8, previous: &Tick, tick: &Tick) {
        let key_lead = self.last_key.as_ref().and_then(|(key, at)| {
            let lead = tick.ts_ms.saturating_sub(*at);
            (lead <= KEY_ATTRIBUTION_MS).then(|| (key.clone(), lead))
        });

        println!(
            "[{:>7.2}s] SHIFT {} begins — max_hp {}, game clock {}{}",
            self.elapsed(tick.ts_ms),
            arrow(direction),
            previous.max_health,
            tick.clock_time,
            match &key_lead {
                Some((key, lead)) => format!("  (after {} {}ms earlier)", key, lead),
                None => String::from("  (no key seen)"),
            }
        );

        self.current = Some(Episode {
            direction,
            start_ms: previous.ts_ms,
            end_ms: tick.ts_ms,
            start_max_hp: previous.max_health,
            end_max_hp: tick.max_health,
            start_hp: previous.health,
            end_hp: tick.health,
            ticks: 0,
            steps: Vec::new(),
            active_counts: BTreeMap::new(),
            key_lead,
            level_changed: false,
        });
    }

    fn close_episode(&mut self, reason: &str) {
        let Some(episode) = self.current.take() else {
            return;
        };

        println!(
            "[{:>7.2}s] SHIFT {} ends   — max_hp {} → {} ({:+}) in {:.2}s over {} ticks, {:.0} HP/s — {}",
            self.elapsed(episode.end_ms),
            arrow(episode.direction),
            episode.start_max_hp,
            episode.end_max_hp,
            episode.hp_delta(),
            episode.duration_ms() as f64 / 1000.0,
            episode.ticks,
            episode.rate_hp_per_sec(),
            reason
        );
        println!(
            "            current hp {} → {}, steps {}, active while shifting: {}",
            episode.start_hp,
            episode.end_hp,
            step_histogram(&episode.steps),
            if episode.active_counts.is_empty() {
                String::from("(none reported)")
            } else {
                episode
                    .active_counts
                    .iter()
                    .map(|(name, count)| format!("{}×{}", name, count))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        );
        if episode.level_changed {
            println!("            ⚠ hero level changed inside this episode — not a pure shift");
        }
        if !episode.looks_like_a_shift() {
            println!(
                "            ⚠ only {} tick(s) — an item, a level, or a treads swap, not a shift",
                episode.ticks
            );
        }

        self.episodes.push(episode);
    }

    fn print_ability_panel(&self, tick: &Tick) {
        println!("            ability panel:");
        for (index, ability) in tick.abilities.iter().enumerate() {
            println!(
                "              ability{}: {:<34} level {} active {} can_cast {} cd {}",
                index, ability.name, ability.level, ability.active, ability.can_cast, ability.cooldown
            );
        }
    }

    fn report(&mut self) {
        self.close_episode("session ended");

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
            "max_health range: {} … {} (spread {})",
            self.min_max_hp,
            self.max_max_hp,
            self.max_max_hp - self.min_max_hp
        );

        let shifts = self
            .episodes
            .iter()
            .filter(|episode| episode.looks_like_a_shift())
            .count();
        println!(
            "\nepisodes ({}, of which {} look like shifts):",
            self.episodes.len(),
            shifts
        );
        println!(
            "  {:>8}  {:>3}  {:>7}  {:>7}  {:>7}  {:>6}  {:>7}  {:>12}  {}",
            "at", "dir", "from", "to", "delta", "ticks", "HP/s", "key lead", "verdict"
        );
        for episode in &self.episodes {
            println!(
                "  {:>7.2}s  {:>3}  {:>7}  {:>7}  {:>+7}  {:>6}  {:>7.0}  {:>12}  {}",
                self.elapsed(episode.start_ms),
                arrow(episode.direction),
                episode.start_max_hp,
                episode.end_max_hp,
                episode.hp_delta(),
                episode.ticks,
                episode.rate_hp_per_sec(),
                match &episode.key_lead {
                    Some((key, lead)) => format!("{} {}ms", key, lead),
                    None => String::from("—"),
                },
                if episode.looks_like_a_shift() {
                    "shift"
                } else {
                    "one-off jump"
                }
            );
        }

        // Question 4: every per-tick step of a real shift should be a multiple
        // of the HP one strength point is worth. One-off jumps are items and
        // levels, and including them only poisons the divisor.
        let mut all_steps: Vec<i64> = Vec::new();
        for episode in self
            .episodes
            .iter()
            .filter(|episode| episode.looks_like_a_shift())
        {
            all_steps.extend(episode.steps.iter().map(|step| step.abs()));
        }
        println!(
            "\nper-tick step sizes across shifts (|max_health| change): {}",
            step_histogram(&all_steps)
        );
        if let Some(unit) = infer_unit(&all_steps) {
            println!(
                "→ every step is a multiple of {} HP — that is one strength point on this patch",
                unit
            );
        } else if !all_steps.is_empty() {
            println!("→ no single divisor fits every step; HP per strength is not clean from this capture");
        }

        // Question 2: did ability_active track the toggle?
        let rising: BTreeMap<String, u32> = merge_active(&self.episodes, 1);
        let falling: BTreeMap<String, u32> = merge_active(&self.episodes, -1);
        println!("\nability_active while max_health rose:   {}", format_counts(&rising));
        println!("ability_active while max_health fell:   {}", format_counts(&falling));
        if rising.is_empty() && falling.is_empty() {
            println!("→ ability_active is useless here; the max_health delta is the only shift signal");
        } else if rising.keys().eq(falling.keys()) {
            println!("→ ability_active does not separate the two directions; use the delta");
        } else {
            println!("→ ability_active DOES separate the directions — prefer it, it has no latency");
        }

        println!("\nabilities seen: {}", format_counts(&self.ability_names));
    }
}

fn arrow(direction: i8) -> &'static str {
    if direction > 0 {
        "STR↑"
    } else {
        "AGI↓"
    }
}

fn active_ability_list(tick: &Tick) -> String {
    let active: Vec<&str> = tick
        .abilities
        .iter()
        .filter(|ability| ability.active)
        .map(|ability| ability.name.as_str())
        .collect();
    if active.is_empty() {
        String::from("active: —")
    } else {
        format!("active: {}", active.join("+"))
    }
}

fn step_histogram(steps: &[i64]) -> String {
    if steps.is_empty() {
        return String::from("—");
    }
    let mut counts: BTreeMap<i64, u32> = BTreeMap::new();
    for step in steps {
        *counts.entry(*step).or_insert(0) += 1;
    }
    counts
        .iter()
        .map(|(step, count)| format!("{}×{}", step, count))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The largest value that divides every observed step — the HP one attribute
/// point is worth, if the steps are clean multiples of it.
fn infer_unit(steps: &[i64]) -> Option<i64> {
    let mut unit = 0i64;
    for step in steps.iter().map(|step| step.abs()).filter(|step| *step > 0) {
        unit = gcd(unit, step);
    }
    (unit > 1).then_some(unit)
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

fn merge_active(episodes: &[Episode], direction: i8) -> BTreeMap<String, u32> {
    let mut merged: BTreeMap<String, u32> = BTreeMap::new();
    for episode in episodes
        .iter()
        .filter(|e| e.direction == direction && e.looks_like_a_shift())
    {
        for (name, count) in &episode.active_counts {
            *merged.entry(name.clone()).or_insert(0) += count;
        }
    }
    merged
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

/// The capture file: every payload verbatim, plus the key presses, so a session
/// can be replayed exactly once.
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

    /// Dota pretty-prints its payloads, so the body arrives across many lines.
    /// Re-serialising the parsed value compacts it back to one line per event,
    /// which is what makes the file readable as JSONL. Nothing is dropped —
    /// this is the whole payload, not the fields the probe happens to read.
    fn record_payload(&self, ts_ms: u128, payload: &Value) {
        self.write(&format!(
            "{{\"probe_ts_ms\":{},\"payload\":{}}}",
            ts_ms,
            serde_json::to_string(payload).unwrap_or_else(|_| String::from("null"))
        ));
    }

    fn record_key(&self, ts_ms: u128, key: &str) {
        self.write(&format!("{{\"probe_ts_ms\":{},\"key\":\"{}\"}}", ts_ms, key));
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

/// Watch the shift keys with a plain listener — `rdev::listen` observes, unlike
/// the app's `rdev::grab`, so this never swallows a key even with the real app
/// running alongside.
fn watch_keys(keys: Vec<char>, shared: Arc<Mutex<Shared>>) {
    std::thread::spawn(move || {
        let mut last_press: BTreeMap<String, u128> = BTreeMap::new();
        let result = rdev::listen(move |event| {
            let rdev::EventType::KeyPress(key) = event.event_type else {
                return;
            };
            let label = format!("{:?}", key);
            let matched = keys.iter().any(|wanted| {
                label == format!("Key{}", wanted.to_ascii_uppercase())
            });
            if !matched {
                return;
            }

            // Held keys autorepeat; only the first press of a burst is a cast.
            let ts_ms = now_ms();
            let recent = last_press
                .get(&label)
                .is_some_and(|previous| ts_ms.saturating_sub(*previous) < 250);
            last_press.insert(label.clone(), ts_ms);
            if recent {
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
            eprintln!("key watch unavailable ({:?}) — running without key timing", error);
        }
    });
}

fn replay(path: &str, verbose: bool, flat_gap_ms: u128) {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("could not open {path}: {error}");
            return;
        }
    };

    let mut analyzer = Analyzer::new(verbose, flat_gap_ms);
    let mut synthetic_ts = 0u128;
    // Read it as a stream of JSON values rather than a value per line: Dota
    // pretty-prints, so an early capture has one payload spread over ~200
    // lines. A stream reader takes those and compact JSONL alike.
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
            // The app's own gsi_events log: bare payloads, no timing. The
            // documented throttle is the best assumption available.
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
    keys: Vec<char>,
    replay: Option<String>,
    output_dir: String,
    verbose: bool,
    flat_gap_ms: u128,
}

impl Args {
    fn parse() -> Self {
        let mut args = Self {
            port: 3100,
            keys: vec!['d', 'f'],
            replay: None,
            output_dir: String::from("logs/morphling_shift"),
            verbose: false,
            flat_gap_ms: DEFAULT_FLAT_GAP_MS,
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
                            .filter_map(|part| part.trim().chars().next())
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
                "--flat-gap-ms" => {
                    index += 1;
                    if let Some(value) = raw.get(index).and_then(|v| v.parse().ok()) {
                        args.flat_gap_ms = value;
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
    println!("Morphling Attribute Shift probe — records GSI and reports what a shift looks like");
    println!();
    println!("Usage:");
    println!("  cargo run --example morphling_shift_probe -- [--port 3100] [--keys d,f]");
    println!("  cargo run --example morphling_shift_probe -- --replay <file.jsonl> [--verbose]");
    println!();
    println!("Options:");
    println!("  --port <N>           Listen port for a second GSI cfg (default: 3100)");
    println!("  --keys <a,b>         Shift keys to timestamp (default: d,f)");
    println!("  --no-keys            Skip the keyboard listener");
    println!("  --replay <PATH>      Analyse a capture instead of listening");
    println!("  --output <DIR>       Capture directory (default: logs/morphling_shift)");
    println!("  --flat-gap-ms <N>    Idle gap that ends an episode (default: 700)");
    println!("  --verbose, -v        Print every tick");
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Some(path) = &args.replay {
        replay(path, args.verbose, args.flat_gap_ms);
        return;
    }

    let shared = Arc::new(Mutex::new(Shared {
        analyzer: Analyzer::new(args.verbose, args.flat_gap_ms),
        recorder: Recorder::create(&args.output_dir),
    }));

    if !args.keys.is_empty() {
        watch_keys(args.keys.clone(), Arc::clone(&shared));
        println!(
            "watching keys: {}",
            args.keys
                .iter()
                .map(|key| key.to_ascii_uppercase().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
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
