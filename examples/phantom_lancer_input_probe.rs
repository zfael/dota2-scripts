//! Phantom Lancer input probe — what does Dota do with the inputs the Phantom
//! Lancer automation sends?
//!
//! Every Phantom Lancer feature is a guess about how Dota reads *our* input,
//! not about what GSI reports, so the questions below are answered by sending
//! the input and watching the game:
//!
//! 1. **Unified orders.** Dota's "Unified Orders with Ctrl" sends an order to
//!    every controlled unit when Ctrl is down. Does a Ctrl press *injected*
//!    around a right-click do the same, and how small can the gap between the
//!    Ctrl and the click be before Dota misses it?
//! 2. **Replay mode.** The latch swallows a right-click and replays it with
//!    Ctrl around it. `click` mode replays press and release at once; `hold`
//!    mode keeps Ctrl down for as long as the button is physically held. Does
//!    either leak Ctrl into a spell? (Ctrl + ability key *levels the ability*
//!    instead of casting it, and Ctrl + a number key reassigns that control
//!    group.)
//! 3. **Left click.** Does Ctrl on the left-click that completes an A-click
//!    make every illusion attack? What does it do to a left-click on the shop,
//!    the inventory, or a unit?
//! 4. **Tap vs. hold.** How long is a natural Ctrl tap, and how short is a
//!    deliberate hold? That gap is where the latch threshold goes.
//! 5. **Illusion send.** Select all other units → attack → click → select
//!    hero. Does Dota register each selection change in time at the chosen step
//!    gap, and does reselecting the hero move the camera?
//!
//! ## Findings
//!
//! From `logs/phantom_lancer/probe_2026-10-28_20-21-32.jsonl` and
//! `probe_2026-10-28_20-25-13.jsonl`, click mode, 15ms gap:
//!
//! | Question | Measured |
//! |---|---|
//! | Q1 | 181 latched right-clicks; the player reports every illusion followed, attacks included. The gap sweep (`F7`) was not run, so 15ms is known to work, not known to be the minimum |
//! | Q2 | Click mode: no spell levelled, no group reassigned. Hold mode not run — click mode is what ships |
//! | Q3 | Not run; left-click wrapping stays off |
//! | Q4 | 12 clean taps, 81–121ms. Every manual hold (441–1587ms) had a click in it, so it read as a chord and never toggled; quick Ctrl+key presses (74–98ms) did not toggle either. A 200ms threshold has ~80ms of room |
//! | Q5 | With `a` + left-click (`probe_2026-10-28_20-29-33.jsonl`, 8 sends): did not work. With a bare left-click, in the app: both selections landed, no order — a left-click only selects. A right-click is what ships; `--send-click right` (now the default) re-tests it |
//!
//! A Doppelganger panic was probed for and dropped before it was measured: the
//! player judged it not worth having.
//!
//! ## Setup
//!
//! - **Close the app first.** Its hook would replay the probe's keys through
//!   Soul Ring and skew every timing here.
//! - Run it elevated, like the app, or the hook cannot see Dota's input.
//! - If the top row of the keyboard sends media keys, hold `Fn` (or turn on
//!   F-lock): the probe keys are `F6`–`F12`, and it prints a line for each one
//!   it sees.
//!
//! ```powershell
//! cargo run --example phantom_lancer_input_probe -- --select-hero 1 --select-others F3
//! ```
//!
//! ## Probe keys
//!
//! All of them are swallowed — Dota never sees them.
//!
//! | Key | Does |
//! |---|---|
//! | `F6` | Next gap in the sweep (0, 5, 15, 30, 60ms) |
//! | `F7` | One Ctrl-wrapped right-click at the cursor, at the sweep gap |
//! | `F8` | Mark the last action as **worked** |
//! | `F9` | Mark the last action as **failed** |
//! | Middle mouse, `F12` | Illusion send: select others → [attack key] → click → select hero |
//! | `Ctrl` tap | Toggle the latch (rising cue on, falling cue off) |
//!
//! ## Capture protocol
//!
//! Phantom Lancer in a demo game or a lobby with `-wait 0`, enough levels for
//! Doppelganger and Juxtapose. Make illusions (Doppelganger, or hit a creep)
//! before each step that needs them. Mark every action with `F8` / `F9`.
//!
//! | # | Do this | What it measures |
//! |---|---|---|
//! | 1 | Tap Ctrl ten times the way you would in a fight, then hold it ten times as for a manual order | Q4: tap and hold durations |
//! | 2 | Select only the hero (`1`). For each gap (`F6`), press `F7` on open ground three times | Q1: the smallest gap Dota reads |
//! | 3 | Latch on. Right-click ground, an enemy creep, an enemy hero | Q1 end to end; Phantom Rush from every illusion |
//! | 4 | Latch on. **Hold** right-click and drag across the map | Q2: hold-to-move |
//! | 5 | Latch on. Press `Q`, `W`, a number key and an item key right after a right-click | Q2: nothing levels up, no group is reassigned |
//! | 6 | Restart with `--latch-mode hold` and repeat 3–5 | Q2: the other mode |
//! | 7 | Restart with `--wrap-left`. A-click a creep; left-click the shop, an item, a unit | Q3 |
//! | 8 | With illusions out, aim at a creep camp and middle-click, five times. Then `--step-ms 15` and `--step-ms 60`, five times each. If the order does not land, try `--attack a`, then `--send-click right` | Q5, which order click works, and how much room the step gap has |
//! | 9 | Ctrl-C | the summary |
//!
//! Steps that must **not** change anything: a Ctrl+1 or a Ctrl+click while
//! holding Ctrl must never toggle the latch, and nothing in step 5 may level
//! an ability.
//!
//! Every event goes to `logs/phantom_lancer/`, so a session can be summarised
//! again without playing:
//!
//! ```powershell
//! cargo run --example phantom_lancer_input_probe -- --replay logs/phantom_lancer/probe_....jsonl
//! ```

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dota2_scripts::audio::motif::{Motif, Timbre, Tone};
use dota2_scripts::audio::player::play_motif;
use dota2_scripts::input::binding::KeyBinding;
use rdev::{Button, EventType, Key};
use serde::{Deserialize, Serialize};

/// The gaps `F6` cycles through, between the injected Ctrl and the click.
const SWEEP_GAPS_MS: [u64; 5] = [0, 5, 15, 30, 60];

/// How long a synthetic key or button is held between its press and release.
const PRESS_HOLD_MS: u64 = 5;

/// Our own injected input. The hook passes everything through while it is set.
static SIMULATING: AtomicBool = AtomicBool::new(false);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ───────────────────────────── recorded events ─────────────────────────────

/// Everything the summary is built from. Live runs write these to the capture
/// file and replay reads them back, so both produce the same report.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Record {
    /// One physical Ctrl press-to-release. `clean` means nothing else was
    /// pressed or clicked while it was down.
    CtrlEpisode {
        duration_ms: u64,
        clean: bool,
    },
    Latch {
        on: bool,
    },
    /// An input the probe sent, labelled with what it was testing.
    Action {
        label: String,
    },
    /// `F8` / `F9` — the player's verdict on the last action.
    Mark {
        worked: bool,
    },
}

#[derive(Serialize, Deserialize)]
struct Line {
    t: u64,
    record: Record,
}

// ─────────────────────────────── the analysis ───────────────────────────────

struct ActionResult {
    label: String,
    worked: u32,
    failed: u32,
    unmarked: u32,
}

#[derive(Default)]
struct Analyzer {
    ctrl_episodes: Vec<(u64, bool)>,
    latch_toggles: u32,
    /// Every action, in order, so a mark lands on the right one.
    actions: Vec<(String, Option<bool>)>,
}

impl Analyzer {
    fn apply(&mut self, record: &Record) {
        match record {
            Record::CtrlEpisode { duration_ms, clean } => {
                self.ctrl_episodes.push((*duration_ms, *clean));
                println!(
                    "ctrl {} {}ms",
                    if *clean { "clean" } else { "chord" },
                    duration_ms
                );
            }
            Record::Latch { on } => {
                self.latch_toggles += 1;
                println!("latch {}", if *on { "ON" } else { "off" });
            }
            Record::Action { label } => {
                self.actions.push((label.clone(), None));
                println!("sent: {label}");
            }
            Record::Mark { worked } => match self.actions.last_mut() {
                Some((label, mark)) => {
                    *mark = Some(*worked);
                    let verdict = if *worked { "worked" } else { "FAILED" };
                    println!("  marked {verdict}: {label}");
                }
                None => println!("mark with nothing to mark"),
            },
        }
    }

    fn report(&self, tap_ms: u64) {
        println!("\n════════════════════ summary ════════════════════");
        println!("{} latch toggles\n", self.latch_toggles);

        println!("Q4 — Ctrl episodes (clean = nothing else pressed while down)");
        let mut clean: Vec<u64> = self
            .ctrl_episodes
            .iter()
            .filter(|(_, clean)| *clean)
            .map(|(duration, _)| *duration)
            .collect();
        clean.sort_unstable();
        let chords = self.ctrl_episodes.len() - clean.len();
        let (taps, holds): (Vec<u64>, Vec<u64>) = clean.iter().partition(|d| **d <= tap_ms);
        println!("  clean taps  (≤{tap_ms}ms): {}  {:?}", taps.len(), taps);
        println!("  clean holds (>{tap_ms}ms): {}  {:?}", holds.len(), holds);
        println!("  chords (Ctrl + something, never a toggle): {chords}");
        if let (Some(longest_tap), Some(shortest_hold)) = (taps.last(), holds.first()) {
            println!(
                "  gap between the longest tap and the shortest hold: {longest_tap}ms → {shortest_hold}ms"
            );
        }

        println!("\nQ1/Q2/Q3/Q5 — marked actions");
        let mut results: Vec<ActionResult> = Vec::new();
        for (label, mark) in &self.actions {
            let index = match results.iter().position(|r| &r.label == label) {
                Some(index) => index,
                None => {
                    results.push(ActionResult {
                        label: label.clone(),
                        worked: 0,
                        failed: 0,
                        unmarked: 0,
                    });
                    results.len() - 1
                }
            };
            match mark {
                Some(true) => results[index].worked += 1,
                Some(false) => results[index].failed += 1,
                None => results[index].unmarked += 1,
            }
        }
        if results.is_empty() {
            println!("  none");
            return;
        }
        println!(
            "  {:<44} {:>6} {:>6} {:>8}",
            "action", "worked", "failed", "unmarked"
        );
        for result in &results {
            println!(
                "  {:<44} {:>6} {:>6} {:>8}",
                result.label, result.worked, result.failed, result.unmarked
            );
        }
    }
}

// ─────────────────────────────── the capture ───────────────────────────────

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

    fn write(&self, line: &Line) {
        let Ok(text) = serde_json::to_string(line) else {
            return;
        };
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(file, "{text}");
        }
    }
}

struct Shared {
    analyzer: Analyzer,
    recorder: Option<Recorder>,
}

type SharedRef = Arc<Mutex<Shared>>;

fn record(shared: &SharedRef, record: Record) {
    if let Ok(mut shared) = shared.lock() {
        shared.analyzer.apply(&record);
        if let Some(recorder) = &shared.recorder {
            recorder.write(&Line {
                t: now_ms(),
                record,
            });
        }
    }
}

// ───────────────────────────── input injection ─────────────────────────────

#[derive(Debug, Clone, Copy)]
enum Step {
    Send(EventType),
    Wait(u64),
}

fn tap(key: Key) -> Vec<Step> {
    vec![
        Step::Send(EventType::KeyPress(key)),
        Step::Wait(PRESS_HOLD_MS),
        Step::Send(EventType::KeyRelease(key)),
    ]
}

fn click(button: Button) -> Vec<Step> {
    vec![
        Step::Send(EventType::ButtonPress(button)),
        Step::Wait(PRESS_HOLD_MS),
        Step::Send(EventType::ButtonRelease(button)),
    ]
}

fn ctrl_wrapped_click(button: Button, gap_ms: u64) -> Vec<Step> {
    let mut steps = vec![
        Step::Send(EventType::KeyPress(Key::ControlLeft)),
        Step::Wait(gap_ms),
    ];
    steps.extend(click(button));
    steps.extend([
        Step::Wait(gap_ms),
        Step::Send(EventType::KeyRelease(Key::ControlLeft)),
    ]);
    steps
}

/// One serial worker owns every injected event. The hook must return fast,
/// and a FIFO is what keeps a `hold`-mode release behind its press.
fn spawn_injector() -> Sender<Vec<Step>> {
    let (tx, rx) = mpsc::channel::<Vec<Step>>();
    thread::spawn(move || {
        for steps in rx {
            SIMULATING.store(true, Ordering::SeqCst);
            for step in steps {
                match step {
                    Step::Send(event) => {
                        if let Err(error) = rdev::simulate(&event) {
                            eprintln!("could not send {event:?}: {error:?}");
                        }
                    }
                    Step::Wait(ms) => thread::sleep(Duration::from_millis(ms)),
                }
            }
            // Let the hook see the last event before it stops passing ours.
            thread::sleep(Duration::from_millis(PRESS_HOLD_MS));
            SIMULATING.store(false, Ordering::SeqCst);
        }
    });
    tx
}

// ──────────────────────────────── the hook ────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LatchMode {
    /// Swallow the press and the release; replay both inside one Ctrl.
    Click,
    /// Ctrl spans the physical hold: down before the press, up after the release.
    Hold,
}

struct Binds {
    select_hero: Key,
    select_others: Key,
    /// `None` skips the attack key: the click alone gives the order.
    attack: Option<Key>,
    send_click: Button,
}

struct Config {
    tap_ms: u64,
    gap_ms: u64,
    step_ms: u64,
    latch_mode: LatchMode,
    wrap_left: bool,
    volume: f32,
    binds: Binds,
}

#[derive(Default)]
struct HookState {
    ctrl_down_at: Option<u64>,
    ctrl_chorded: bool,
    latched: bool,
    sweep_index: usize,
    swallowed_right: bool,
    swallowed_left: bool,
}

fn cue(on: bool, volume: f32) {
    let (low, high) = (660.0, 990.0);
    let tones = if on {
        vec![Tone::new(low, 60), Tone::new(high, 60)]
    } else {
        vec![Tone::new(high, 60), Tone::new(low, 60)]
    };
    play_motif(&Motif::new(tones, 25, Timbre::Wood), volume);
}

/// Select every other unit, order them at the cursor, give the hero back.
fn illusion_send(config: &Config) -> (Vec<Step>, String) {
    let step = config.step_ms;
    let mut steps = tap(config.binds.select_others);
    steps.push(Step::Wait(step));
    if let Some(attack) = config.binds.attack {
        steps.extend(tap(attack));
        steps.push(Step::Wait(step));
    }
    steps.extend(click(config.binds.send_click));
    steps.push(Step::Wait(step));
    steps.extend(tap(config.binds.select_hero));

    let order = match (config.binds.attack, config.binds.send_click) {
        (Some(_), Button::Left) => "attack + left-click",
        (Some(_), _) => "attack + right-click",
        (None, Button::Left) => "left-click",
        (None, _) => "right-click",
    };
    (steps, format!("illusion send, {order}, step {step}ms"))
}

fn is_ctrl(key: Key) -> bool {
    matches!(key, Key::ControlLeft | Key::ControlRight)
}

fn is_probe_key(key: Key) -> bool {
    matches!(key, Key::F6 | Key::F7 | Key::F8 | Key::F9 | Key::F12)
}

fn swallowed_flag(state: &mut HookState, button: Button) -> Option<&mut bool> {
    match button {
        Button::Right => Some(&mut state.swallowed_right),
        Button::Left => Some(&mut state.swallowed_left),
        _ => None,
    }
}

fn run_hook(config: Config, shared: SharedRef, inject: Sender<Vec<Step>>) {
    let state = Mutex::new(HookState::default());

    let callback = move |event: rdev::Event| -> Option<rdev::Event> {
        if SIMULATING.load(Ordering::SeqCst) {
            return Some(event);
        }
        if matches!(
            event.event_type,
            EventType::MouseMove { .. } | EventType::Wheel { .. }
        ) {
            return Some(event);
        }

        let mut state = state.lock().unwrap();
        let send = |steps: Vec<Step>| {
            let _ = inject.send(steps);
        };

        match event.event_type {
            // Ctrl always reaches Dota: a bare Ctrl does nothing there, and we
            // cannot know it is a tap until it is released.
            EventType::KeyPress(key) if is_ctrl(key) => {
                // Autorepeat sends more presses while it is held.
                if state.ctrl_down_at.is_none() {
                    state.ctrl_down_at = Some(now_ms());
                    state.ctrl_chorded = false;
                }
                Some(event)
            }
            EventType::KeyRelease(key) if is_ctrl(key) => {
                if let Some(down_at) = state.ctrl_down_at.take() {
                    let duration_ms = now_ms().saturating_sub(down_at);
                    let clean = !state.ctrl_chorded;
                    record(&shared, Record::CtrlEpisode { duration_ms, clean });
                    if clean && duration_ms <= config.tap_ms {
                        state.latched = !state.latched;
                        cue(state.latched, config.volume);
                        record(&shared, Record::Latch { on: state.latched });
                    }
                }
                Some(event)
            }
            EventType::KeyPress(key) => {
                if state.ctrl_down_at.is_some() {
                    state.ctrl_chorded = true;
                }
                if !is_probe_key(key) {
                    return Some(event);
                }
                match key {
                    Key::F6 => {
                        state.sweep_index = (state.sweep_index + 1) % SWEEP_GAPS_MS.len();
                        let gap = SWEEP_GAPS_MS[state.sweep_index];
                        println!("sweep gap now {gap}ms");
                    }
                    Key::F7 => {
                        let gap = SWEEP_GAPS_MS[state.sweep_index];
                        send(ctrl_wrapped_click(Button::Right, gap));
                        record(
                            &shared,
                            Record::Action {
                                label: format!("F7 wrapped right-click, gap {gap}ms"),
                            },
                        );
                    }
                    Key::F8 => record(&shared, Record::Mark { worked: true }),
                    Key::F9 => record(&shared, Record::Mark { worked: false }),
                    Key::F12 => {
                        let (steps, label) = illusion_send(&config);
                        send(steps);
                        record(&shared, Record::Action { label });
                    }
                    _ => {}
                }
                None
            }
            EventType::KeyRelease(key) if is_probe_key(key) => None,
            EventType::ButtonPress(Button::Middle) => {
                if state.ctrl_down_at.is_some() {
                    state.ctrl_chorded = true;
                }
                let (steps, label) = illusion_send(&config);
                send(steps);
                record(&shared, Record::Action { label });
                None
            }
            EventType::ButtonRelease(Button::Middle) => None,
            EventType::ButtonPress(button) => {
                // Ctrl physically held is manual mode: the player's own Ctrl
                // already does the job, and our Ctrl-up would cut it short.
                if state.ctrl_down_at.is_some() {
                    state.ctrl_chorded = true;
                    return Some(event);
                }
                let wraps = button == Button::Right || (button == Button::Left && config.wrap_left);
                if !state.latched || !wraps {
                    return Some(event);
                }
                if let Some(flag) = swallowed_flag(&mut state, button) {
                    *flag = true;
                }
                let gap = config.gap_ms;
                let side = if button == Button::Right {
                    "right"
                } else {
                    "left"
                };
                match config.latch_mode {
                    LatchMode::Click => send(ctrl_wrapped_click(button, gap)),
                    LatchMode::Hold => send(vec![
                        Step::Send(EventType::KeyPress(Key::ControlLeft)),
                        Step::Wait(gap),
                        Step::Send(EventType::ButtonPress(button)),
                    ]),
                }
                let mode = match config.latch_mode {
                    LatchMode::Click => "click",
                    LatchMode::Hold => "hold",
                };
                record(
                    &shared,
                    Record::Action {
                        label: format!("latched {side}-click, {mode} mode, gap {gap}ms"),
                    },
                );
                None
            }
            EventType::ButtonRelease(button) => {
                // Keyed on what was swallowed, not on the latch: turning the
                // latch off mid-hold must still release the Ctrl we pressed.
                let swallowed = swallowed_flag(&mut state, button)
                    .map(|flag| std::mem::replace(flag, false))
                    .unwrap_or(false);
                if !swallowed {
                    return Some(event);
                }
                if config.latch_mode == LatchMode::Hold {
                    send(vec![
                        Step::Send(EventType::ButtonRelease(button)),
                        Step::Wait(config.gap_ms),
                        Step::Send(EventType::KeyRelease(Key::ControlLeft)),
                    ]);
                }
                None
            }
            _ => Some(event),
        }
    };

    if let Err(error) = rdev::grab(callback) {
        eprintln!("could not install the hook ({error:?}) — run elevated");
    }
}

// ─────────────────────────────── entry point ───────────────────────────────

fn replay(path: &str, tap_ms: u64) {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("could not open {path}: {error}");
            return;
        }
    };
    let mut analyzer = Analyzer::default();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let Ok(line) = line else { break };
        match serde_json::from_str::<Line>(&line) {
            Ok(line) => analyzer.apply(&line.record),
            Err(error) => eprintln!("line {}: {error}", index + 1),
        }
    }
    analyzer.report(tap_ms);
}

fn parse_key(raw: &str, flag: &str) -> Key {
    match KeyBinding::parse(raw).rdev_key() {
        Some(key) => key,
        None => {
            eprintln!("{flag} {raw}: not a key the probe can press");
            std::process::exit(2);
        }
    }
}

struct Args {
    config: Config,
    replay: Option<String>,
    output_dir: String,
}

fn parse_args() -> Args {
    let mut args = Args {
        config: Config {
            tap_ms: 200,
            gap_ms: 15,
            step_ms: 30,
            latch_mode: LatchMode::Click,
            wrap_left: false,
            volume: 0.5,
            binds: Binds {
                select_hero: Key::Num1,
                select_others: Key::F3,
                attack: None,
                send_click: Button::Right,
            },
        },
        replay: None,
        output_dir: String::from("logs/phantom_lancer"),
    };

    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut index = 0;
    while index < raw.len() {
        let flag = raw[index].as_str();
        let value = raw.get(index + 1).cloned().unwrap_or_default();
        let number = || value.parse::<u64>().ok();
        let mut takes_value = true;
        match flag {
            "--tap-ms" => args.config.tap_ms = number().unwrap_or(args.config.tap_ms),
            "--gap-ms" => args.config.gap_ms = number().unwrap_or(args.config.gap_ms),
            "--step-ms" => args.config.step_ms = number().unwrap_or(args.config.step_ms),
            "--volume" => args.config.volume = value.parse().unwrap_or(args.config.volume),
            "--latch-mode" => {
                args.config.latch_mode = match value.as_str() {
                    "hold" => LatchMode::Hold,
                    _ => LatchMode::Click,
                }
            }
            "--select-hero" => args.config.binds.select_hero = parse_key(&value, flag),
            "--select-others" => args.config.binds.select_others = parse_key(&value, flag),
            "--attack" => {
                args.config.binds.attack = match value.as_str() {
                    "none" => None,
                    raw => Some(parse_key(raw, flag)),
                }
            }
            "--send-click" => {
                args.config.binds.send_click = match value.as_str() {
                    "right" => Button::Right,
                    "left" => Button::Left,
                    _ => Button::Right,
                }
            }
            "--replay" => args.replay = Some(value.clone()),
            "--output" => args.output_dir = value.clone(),
            "--wrap-left" => {
                args.config.wrap_left = true;
                takes_value = false;
            }
            "--help" | "-h" => {
                usage();
                std::process::exit(0);
            }
            other => {
                eprintln!("ignoring unknown argument {other}");
                takes_value = false;
            }
        }
        index += if takes_value { 2 } else { 1 };
    }
    args
}

fn usage() {
    println!("Phantom Lancer input probe — sends the inputs the PL automation sends");
    println!();
    println!("Usage:");
    println!("  cargo run --example phantom_lancer_input_probe -- [options]");
    println!("  cargo run --example phantom_lancer_input_probe -- --replay <file.jsonl>");
    println!();
    println!("Options:");
    println!("  --tap-ms <N>          Longest Ctrl press that toggles the latch (default 200)");
    println!("  --gap-ms <N>          Ctrl-to-click gap for latched clicks (default 15)");
    println!("  --latch-mode <M>      click | hold (default click)");
    println!("  --wrap-left           Also wrap left-clicks while latched");
    println!("  --step-ms <N>         Gap between illusion-send steps (default 30)");
    println!("  --select-hero <KEY>   Your select-hero key (default 1)");
    println!("  --select-others <KEY> Your select-all-other-units key (default F3)");
    println!("  --attack <KEY|none>   Attack key pressed before the send's click (default none)");
    println!("  --send-click <B>      left | right: the send's order click (default right)");
    println!("  --volume <0..1>       Latch cue volume (default 0.5)");
    println!("  --output <DIR>        Capture directory (default logs/phantom_lancer)");
}

fn main() {
    let args = parse_args();
    let tap_ms = args.config.tap_ms;

    if let Some(path) = &args.replay {
        replay(path, tap_ms);
        return;
    }

    let shared: SharedRef = Arc::new(Mutex::new(Shared {
        analyzer: Analyzer::default(),
        recorder: Recorder::create(&args.output_dir),
    }));

    println!(
        "latch: tap Ctrl ≤{}ms, {:?} mode, gap {}ms, left-click wrap {}; illusion send step {}ms",
        args.config.tap_ms,
        args.config.latch_mode,
        args.config.gap_ms,
        if args.config.wrap_left { "on" } else { "off" },
        args.config.step_ms,
    );
    println!("F6 sweep gap · F7 wrapped right-click · F8 worked · F9 failed · middle mouse or F12 illusion send");
    println!("Press F6 once now: if no \"sweep gap\" line appears, hold Fn for the F-keys.");

    let inject = spawn_injector();
    {
        let shared = Arc::clone(&shared);
        let config = args.config;
        thread::spawn(move || run_hook(config, shared, inject));
    }

    // Ctrl-C ends the process, so the summary is printed from the handler.
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    if let Err(error) = ctrlc_handler(stop_tx) {
        eprintln!("no Ctrl-C handler ({error}); the capture file is still written");
    }
    println!("Ctrl-C in this window to stop and print the summary\n");
    let _ = stop_rx.recv();

    if let Ok(shared) = shared.lock() {
        shared.analyzer.report(tap_ms);
    };
}

/// tokio is already a dependency and owns the only Ctrl-C hook in the crate.
fn ctrlc_handler(stop: Sender<()>) -> std::io::Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    thread::spawn(move || {
        runtime.block_on(async {
            let _ = tokio::signal::ctrl_c().await;
        });
        let _ = stop.send(());
    });
    Ok(())
}
