//! Largo song sync — the beat intercept, proved offline.
//!
//! Amphibian Rhapsody is a rhythm. A song key pressed between beats casts the
//! song off the beat, and with Aghanim's Scepter it also casts it off the
//! *other* song of the pair. So the keyboard hook swallows the press and keeps
//! it as intent, and the beat worker puts it on the grid.
//!
//! That trade is the whole risk: the app now blocks a key the player pressed.
//! Block one it should not have and the hero stops casting. This binary is the
//! evidence the trade is safe, and it runs without Dota.
//!
//! Two halves:
//!
//! - `Schedule` + `plan_key_action` — the state machine, in the shape it has in
//!   `src/actions/heroes/largo.rs` and `src/input/keyboard.rs`.
//! - `World` — a clock that plays presses at chosen offsets and runs the beat
//!   worker against them, wired with these constants:
//!
//!   | Constant | Value | Where it comes from |
//!   |---|---|---|
//!   | beat interval | 995ms | `heroes.largo.beat_interval_ms`, tuned by ear |
//!   | beat correction | +30ms every 5 beats | `heroes.largo.beat_correction_*` |
//!   | synthetic press cost | 10ms | `POST_ACTION_GUARD_DELAY_MS`, `src/input/simulation.rs` |
//!   | GSI tick | ~226ms median | `logs/morphling_shift/probe_2026-09-05_19-34-54.jsonl` |
//!
//!   **The state machine uses none of them.** They describe the world it is
//!   tested against. The interval and the correction are config the player
//!   tunes by ear; the press cost and the tick rate only decide how wide the
//!   ugly windows in the scenario suite are.
//!
//! ## The rules it enforces
//!
//! 1. A song press never reaches Dota while the ultimate is up. It lands on the
//!    next beat instead, on the grid, whatever the offset it arrived at.
//! 2. A song press *outside* the ultimate always reaches Dota. That includes
//!    the window after the ultimate key is pressed and before GSI confirms it,
//!    where passing through is the safe answer.
//! 3. The last press before a beat is the one that lands. Changing your mind
//!    twice costs nothing and never emits twice.
//! 4. Pressing the song already playing cancels a switch queued this beat.
//! 5. With the scepter, both songs go out on the same beat, and a third press
//!    evicts the one pressed longest ago.
//! 6. The ultimate key stops emission on the press, not a GSI tick later, and
//!    is itself never blocked — it is what ends the ultimate.
//! 7. The grid is absolute. Emission latency never accumulates into drift.
//!
//! ## Running it
//!
//! ```powershell
//! cargo run --example largo_song_sync_control              # scenario suite
//! cargo run --example largo_song_sync_control -- --verbose # every event
//! ```


// ───────────────────────── the state machine ─────────────────────────
// This section mirrors src/actions/heroes/largo.rs and the Largo planner in
// src/input/keyboard.rs. Keep them in step.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Song {
    Bullbelly,
    Hotfeet,
    IslandElixir,
}

impl Song {
    fn key(self) -> char {
        match self {
            Song::Bullbelly => 'q',
            Song::Hotfeet => 'w',
            Song::IslandElixir => 'e',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SongRequest {
    Ignored,
    Started(Song),
    Queued(Song),
    Cancelled(Song),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    SelectSong(Song),
    StopUltimate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeatConfig {
    pub interval_ms: u32,
    pub correction_ms: i32,
    pub correction_every_n_beats: u32,
}

impl Default for BeatConfig {
    fn default() -> Self {
        Self {
            interval_ms: 995,
            correction_ms: 30,
            correction_every_n_beats: 5,
        }
    }
}

pub fn beat_offset_ms(config: &BeatConfig, beat_count: u32) -> u64 {
    if beat_count == 0 {
        return 0;
    }

    let mut total_ms = config.interval_ms as i64 * beat_count as i64;
    if config.correction_every_n_beats > 0 {
        let completed_cycles = beat_count / config.correction_every_n_beats;
        total_ms += config.correction_ms as i64 * completed_cycles as i64;
    }

    total_ms.max(0) as u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Schedule {
    pub active: bool,
    pub anchor_ms: Option<u64>,
    pub beat_count: u32,
    pub generation: u64,
    pub current: Option<Song>,
    pub previous: Option<Song>,
    pub pending: Option<Song>,
    pub has_aggs: bool,
}

impl Schedule {
    fn cancel_planned_beats(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    fn reset_song_schedule(&mut self) {
        self.anchor_ms = None;
        self.beat_count = 0;
        self.current = None;
        self.previous = None;
        self.pending = None;
    }

    /// Fold a song press into the schedule. The hook's whole contribution.
    fn queue_song(&mut self, song: Song) -> SongRequest {
        if !self.active {
            return SongRequest::Ignored;
        }

        match self.current {
            None => {
                self.cancel_planned_beats();
                self.reset_song_schedule();
                self.current = Some(song);
                SongRequest::Started(song)
            }
            Some(current) if current == song => {
                self.pending = None;
                SongRequest::Cancelled(song)
            }
            Some(_) => {
                self.pending = Some(song);
                SongRequest::Queued(song)
            }
        }
    }

    fn stop_ultimate(&mut self) -> bool {
        let was_active = self.active;
        if was_active {
            self.active = false;
            self.cancel_planned_beats();
            self.reset_song_schedule();
        }
        was_active
    }

    /// Advance one beat and say which keys go out.
    fn plan_beat(&mut self) -> Vec<Song> {
        self.beat_count += 1;

        if let Some(pending) = self.pending.take() {
            if self.current != Some(pending) {
                self.previous = self.current;
                self.current = Some(pending);
            }
        }

        let Some(current) = self.current else {
            return Vec::new();
        };

        let mut songs = vec![current];
        if self.has_aggs {
            if let Some(previous) = self.previous {
                songs.push(previous);
            }
        }

        songs
    }

    fn next_deadline_ms(&self, config: &BeatConfig, now_ms: u64) -> Option<u64> {
        if !self.active || self.current.is_none() {
            return None;
        }

        // No anchor yet means the first song was just pressed: it goes out now,
        // and that press is what the rest of the grid is measured from.
        let Some(anchor) = self.anchor_ms else {
            return Some(now_ms);
        };

        Some(anchor + beat_offset_ms(config, self.beat_count))
    }
}

/// Mirrors `plan_largo_key_action` in `src/input/keyboard.rs`.
pub fn plan_key_action(
    key: char,
    binds: &Binds,
    largo_enabled: bool,
    ultimate_active: bool,
) -> Option<KeyAction> {
    if !largo_enabled || !ultimate_active {
        return None;
    }
    if key == binds.ultimate {
        return Some(KeyAction::StopUltimate);
    }
    if key == binds.bullbelly {
        return Some(KeyAction::SelectSong(Song::Bullbelly));
    }
    if key == binds.hotfeet {
        return Some(KeyAction::SelectSong(Song::Hotfeet));
    }
    if key == binds.island_elixir {
        return Some(KeyAction::SelectSong(Song::IslandElixir));
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binds {
    pub bullbelly: char,
    pub hotfeet: char,
    pub island_elixir: char,
    pub ultimate: char,
}

impl Default for Binds {
    fn default() -> Self {
        Self {
            bullbelly: 'q',
            hotfeet: 'w',
            island_elixir: 'e',
            ultimate: 'r',
        }
    }
}

// ─────────────────────────────── the world ───────────────────────────────

/// One synthetic key press costs the guard delay in `src/input/simulation.rs`,
/// which the beat worker pays serially. It is why the Aghanim's pair is "on the
/// same beat" rather than "at the same instant".
const SYNTHETIC_PRESS_COST_MS: u64 = 10;

/// Median GSI tick from the Morphling capture. Used only to space the scripted
/// ultimate-state changes realistically.
const GSI_TICK_MS: u64 = 226;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    /// The player pressed a key.
    Press(char),
    /// One key of a planned beat goes out.
    Emit {
        song: Song,
        generation: u64,
    },
    /// GSI reported the ultimate going up or down.
    GsiUltimate(bool),
    /// The scepter appeared or vanished.
    GsiAggs(bool),
}

/// What Dota actually received, and when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Landed {
    at_ms: u64,
    key: char,
    /// False when the press reached Dota untouched rather than via the worker.
    on_beat: bool,
}

struct World {
    config: BeatConfig,
    binds: Binds,
    schedule: Schedule,
    largo_enabled: bool,
    now_ms: u64,
    /// Pending events, kept as `(at_ms, seq, event)`. Small enough that a
    /// linear scan for the earliest beats giving `Event` a meaningless `Ord`.
    queue: Vec<(u64, u64, Event)>,
    seq: u64,
    landed: Vec<Landed>,
    beats_fired: Vec<u64>,
    verbose: bool,
}

impl World {
    fn new(verbose: bool) -> Self {
        Self {
            config: BeatConfig::default(),
            binds: Binds::default(),
            schedule: Schedule::default(),
            largo_enabled: true,
            now_ms: 0,
            queue: Vec::new(),
            seq: 0,
            landed: Vec::new(),
            beats_fired: Vec::new(),
            verbose,
        }
    }

    fn schedule_event(&mut self, at_ms: u64, event: Event) {
        self.seq += 1;
        self.queue.push((at_ms, self.seq, event));
    }

    /// Index of the earliest pending event, ties broken by insertion order.
    fn earliest_event(&self) -> Option<usize> {
        self.queue
            .iter()
            .enumerate()
            .min_by_key(|(_, (at, seq, _))| (*at, *seq))
            .map(|(index, _)| index)
    }

    fn press(&mut self, at_ms: u64, key: char) -> &mut Self {
        self.schedule_event(at_ms, Event::Press(key));
        self
    }

    fn gsi_ultimate(&mut self, at_ms: u64, active: bool) -> &mut Self {
        self.schedule_event(at_ms, Event::GsiUltimate(active));
        self
    }

    fn gsi_aggs(&mut self, at_ms: u64, has_aggs: bool) -> &mut Self {
        self.schedule_event(at_ms, Event::GsiAggs(has_aggs));
        self
    }

    /// Run until `until_ms`, firing beats as they come due.
    fn run(&mut self, until_ms: u64) {
        loop {
            let next_beat = self.schedule.next_deadline_ms(&self.config, self.now_ms);
            let next_index = self.earliest_event();
            let next_event = next_index.map(|index| self.queue[index].0);

            let next = match (next_beat, next_event) {
                (Some(b), Some(e)) => b.min(e),
                (Some(b), None) => b,
                (None, Some(e)) => e,
                (None, None) => return,
            };

            if next > until_ms {
                return;
            }

            self.now_ms = self.now_ms.max(next);

            // A beat that is due now takes priority over a press arriving at
            // the same instant: the worker holds the lock while it plans.
            if next_beat == Some(self.now_ms) {
                self.fire_beat();
                continue;
            }

            let Some(index) = next_index else {
                return;
            };
            let (_, _, event) = self.queue.remove(index);
            self.handle(event);
        }
    }

    fn fire_beat(&mut self) {
        if self.schedule.anchor_ms.is_none() {
            self.schedule.anchor_ms = Some(self.now_ms);
        }

        let songs = self.schedule.plan_beat();
        let generation = self.schedule.generation;
        self.beats_fired.push(self.now_ms);

        if self.verbose {
            println!(
                "  {:>6}ms  beat #{} -> {:?}",
                self.now_ms, self.schedule.beat_count, songs
            );
        }

        // Serial, because the synthetic input worker is serial.
        for (index, song) in songs.into_iter().enumerate() {
            let at = self.now_ms + index as u64 * SYNTHETIC_PRESS_COST_MS;
            self.schedule_event(at, Event::Emit { song, generation });
        }
    }

    fn handle(&mut self, event: Event) {
        match event {
            Event::Press(key) => {
                let action = plan_key_action(
                    key,
                    &self.binds,
                    self.largo_enabled,
                    self.schedule.active,
                );

                match action {
                    Some(KeyAction::SelectSong(song)) => {
                        let outcome = self.schedule.queue_song(song);
                        if self.verbose {
                            println!("  {:>6}ms  '{}' swallowed -> {:?}", self.now_ms, key, outcome);
                        }
                    }
                    Some(KeyAction::StopUltimate) => {
                        self.schedule.stop_ultimate();
                        // Never blocked: this press is what ends the ultimate.
                        self.land(key, false);
                        if self.verbose {
                            println!("  {:>6}ms  '{}' -> stop, passed through", self.now_ms, key);
                        }
                    }
                    None => {
                        self.land(key, false);
                        if self.verbose {
                            println!("  {:>6}ms  '{}' passed through", self.now_ms, key);
                        }
                    }
                }
            }
            Event::Emit { song, generation } => {
                // The worker re-checks both before every key, so a stop that
                // arrived between the two halves of an Aghanim's pair drops the
                // second one.
                if !self.schedule.active || self.schedule.generation != generation {
                    if self.verbose {
                        println!("  {:>6}ms  {:?} cancelled before it landed", self.now_ms, song);
                    }
                    return;
                }
                self.land(song.key(), true);
            }
            Event::GsiUltimate(active) => {
                if self.schedule.active != active {
                    self.schedule.active = active;
                    self.schedule.cancel_planned_beats();
                    self.schedule.reset_song_schedule();
                }
                if self.verbose {
                    println!("  {:>6}ms  GSI: ultimate active={}", self.now_ms, active);
                }
            }
            Event::GsiAggs(has_aggs) => {
                if self.schedule.has_aggs != has_aggs {
                    self.schedule.has_aggs = has_aggs;
                    self.schedule.previous = None;
                }
            }
        }
    }

    fn land(&mut self, key: char, on_beat: bool) {
        self.landed.push(Landed {
            at_ms: self.now_ms,
            key,
            on_beat,
        });
    }

    fn landed_keys(&self) -> Vec<char> {
        self.landed.iter().map(|l| l.key).collect()
    }

    fn raw_keys(&self) -> Vec<char> {
        self.landed
            .iter()
            .filter(|l| !l.on_beat)
            .map(|l| l.key)
            .collect()
    }

    /// Everything that reached Dota at a time that is not a beat boundary.
    fn off_grid(&self) -> Vec<Landed> {
        self.landed
            .iter()
            .filter(|l| l.on_beat)
            .filter(|l| {
                !self
                    .beats_fired
                    .iter()
                    .any(|b| l.at_ms.saturating_sub(*b) < SYNTHETIC_PRESS_COST_MS * 3)
            })
            .copied()
            .collect()
    }
}

// ──────────────────────────── the scenarios ────────────────────────────

struct Suite {
    verbose: bool,
    failures: Vec<String>,
    run: usize,
}

impl Suite {
    fn check(&mut self, name: &str, body: impl FnOnce(&mut World) -> Result<(), String>) {
        self.run += 1;
        if self.verbose {
            println!("\n── {name}");
        }
        let mut world = World::new(self.verbose);
        match body(&mut world) {
            Ok(()) => println!("PASS  {name}"),
            Err(why) => {
                println!("FAIL  {name}\n        {why}");
                self.failures.push(name.to_string());
            }
        }
    }
}

fn expect(condition: bool, why: impl FnOnce() -> String) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(why())
    }
}

fn main() {
    let verbose = std::env::args().any(|a| a == "--verbose");
    let mut suite = Suite {
        verbose,
        failures: Vec::new(),
        run: 0,
    };

    println!("Largo song sync — control suite");
    println!(
        "beat {}ms, correction {:+}ms every {} beats, synthetic press {}ms, GSI tick ~{}ms\n",
        BeatConfig::default().interval_ms,
        BeatConfig::default().correction_ms,
        BeatConfig::default().correction_every_n_beats,
        SYNTHETIC_PRESS_COST_MS,
        GSI_TICK_MS,
    );

    // 1. The whole point: a press between beats lands on the beat.
    suite.check("a switch pressed mid-beat lands on the next beat", |w| {
        w.gsi_ultimate(0, true);
        w.press(10, 'q'); // first song anchors the grid at 10ms
        w.press(400, 'w'); // 390ms into the beat — the off-beat press
        w.run(1_500); // two beats: 10ms and 1005ms

        expect(
            w.raw_keys().is_empty(),
            || format!("a song key reached Dota raw: {:?}", w.raw_keys()),
        )?;
        expect(
            w.landed_keys() == vec!['q', 'w'],
            || format!("expected q then w on the grid, got {:?}", w.landed_keys()),
        )?;
        let switch_at = w.landed[1].at_ms;
        expect(
            switch_at == 10 + 995,
            || format!("the switch landed at {switch_at}ms, not on the 1005ms beat"),
        )
    });

    // 2. Changing your mind twice costs nothing.
    suite.check("only the last press before a beat lands", |w| {
        w.gsi_ultimate(0, true);
        w.press(0, 'q');
        w.press(300, 'w');
        w.press(600, 'e');
        w.press(900, 'w');
        w.run(1_500); // two beats: 0ms and 995ms

        expect(
            w.landed_keys() == vec!['q', 'w'],
            || format!("expected one switch, got {:?}", w.landed_keys()),
        )
    });

    // 3. Pressing back to the song already playing calls the switch off.
    suite.check("pressing the playing song cancels a queued switch", |w| {
        w.gsi_ultimate(0, true);
        w.press(0, 'q');
        w.press(300, 'w');
        w.press(600, 'q');
        w.run(1_500); // two beats: 0ms and 995ms

        expect(
            w.landed_keys() == vec!['q', 'q'],
            || format!("expected to stay on Bullbelly, got {:?}", w.landed_keys()),
        )
    });

    // 4. Aghanim's: both songs on one beat, not one now and one later.
    suite.check("the Aghanim's pair goes out on a single beat", |w| {
        w.gsi_aggs(0, true);
        w.gsi_ultimate(0, true);
        w.press(0, 'q');
        w.press(500, 'w');
        w.run(1_500); // two beats: 0ms and 995ms

        // beat 1: q alone (only one song pressed yet). beat 2: w + q.
        expect(
            w.landed_keys() == vec!['q', 'w', 'q'],
            || format!("expected q, then w+q, got {:?}", w.landed_keys()),
        )?;
        let pair_gap = w.landed[2].at_ms - w.landed[1].at_ms;
        expect(
            pair_gap == SYNTHETIC_PRESS_COST_MS,
            || format!("the pair was {pair_gap}ms apart, not one synthetic press"),
        )?;
        expect(w.raw_keys().is_empty(), || {
            format!("a song key reached Dota raw: {:?}", w.raw_keys())
        })
    });

    // 5. The third song evicts the one pressed longest ago.
    suite.check("a third song evicts the one pressed longest ago", |w| {
        w.gsi_aggs(0, true);
        w.gsi_ultimate(0, true);
        w.press(0, 'q');
        w.press(500, 'w');
        w.press(1_500, 'e');
        w.run(2_500); // three beats: 0ms, 995ms, 1990ms

        // beat 1: q. beat 2: w+q. beat 3: e+w — Bullbelly drops.
        expect(
            w.landed_keys() == vec!['q', 'w', 'q', 'e', 'w'],
            || format!("expected Bullbelly to drop, got {:?}", w.landed_keys()),
        )
    });

    // 6. The ultimate key stops the loop on the press, not a GSI tick later.
    suite.check("the ultimate key stops emission before GSI confirms", |w| {
        w.gsi_aggs(0, true);
        w.gsi_ultimate(0, true);
        w.press(0, 'q');
        w.press(500, 'w');
        w.press(1_400, 'r');
        w.gsi_ultimate(1_400 + GSI_TICK_MS, false); // confirmation, a tick late
        w.run(5_000);

        expect(
            w.raw_keys() == vec!['r'],
            || format!("the ultimate key must reach Dota; raw was {:?}", w.raw_keys()),
        )?;
        let after_stop: Vec<_> = w
            .landed
            .iter()
            .filter(|l| l.on_beat && l.at_ms > 1_400)
            .collect();
        expect(after_stop.is_empty(), || {
            format!("{} song presses landed after the stop", after_stop.len())
        })
    });

    // 7. Outside the ultimate the song keys are ordinary abilities.
    suite.check("song keys outside the ultimate reach Dota untouched", |w| {
        w.press(0, 'q');
        w.press(100, 'w');
        w.press(200, 'e');
        w.run(1_000);

        expect(
            w.raw_keys() == vec!['q', 'w', 'e'],
            || format!("expected all three to pass through, got {:?}", w.raw_keys()),
        )
    });

    // 8. The window between pressing R and GSI confirming it. Passing through
    //    is the safe answer: swallowing a press for an ultimate that never
    //    cast would eat the ability.
    suite.check("presses before GSI confirms the ultimate pass through", |w| {
        w.press(0, 'r'); // cast the ultimate
        w.press(100, 'q'); // eager — GSI has not caught up
        w.gsi_ultimate(GSI_TICK_MS, true);
        w.press(400, 'q'); // now it is ours
        w.run(2_000);

        expect(
            w.raw_keys() == vec!['r', 'q'],
            || format!("expected r and the eager q through, got {:?}", w.raw_keys()),
        )?;
        let on_beat: Vec<_> = w.landed.iter().filter(|l| l.on_beat).collect();
        expect(
            on_beat.len() >= 2 && on_beat[0].at_ms == 400,
            || format!("the second q should have anchored the grid at 400ms, got {on_beat:?}"),
        )
    });

    // 9. Emission latency must not accumulate.
    suite.check("thirty beats stay on the absolute grid", |w| {
        w.gsi_ultimate(0, true);
        w.press(0, 'q');
        w.run(31_000);

        expect(w.beats_fired.len() >= 30, || {
            format!("only {} beats fired", w.beats_fired.len())
        })?;

        let config = BeatConfig::default();
        let anchor = w.beats_fired[0];
        for (index, fired) in w.beats_fired.iter().enumerate().take(30) {
            let want = anchor + beat_offset_ms(&config, index as u32);
            expect(*fired == want, || {
                format!("beat #{} fired at {fired}ms, grid says {want}ms", index + 1)
            })?;
        }

        expect(w.off_grid().is_empty(), || {
            format!("{} presses landed off the grid", w.off_grid().len())
        })
    });

    // 10. The ugly one: a press that arrives while the worker is emitting.
    suite.check("a press inside the emission window is never dropped", |w| {
        w.gsi_aggs(0, true);
        w.gsi_ultimate(0, true);
        w.press(0, 'q');
        w.press(500, 'w');
        // Beat 2 is at 995ms and emits two keys, at 995ms and 1005ms. This
        // press lands between them, while the worker is mid-pair.
        w.press(1_000, 'e');
        w.run(2_500); // three beats: 0ms, 995ms, 1990ms

        expect(
            w.landed_keys() == vec!['q', 'w', 'q', 'e', 'w'],
            || format!("the press in the window was lost or doubled: {:?}", w.landed_keys()),
        )?;
        let applied_at = w.landed[3].at_ms;
        expect(applied_at == 1_990, || {
            format!("Island Elixir landed at {applied_at}ms, not on the next beat")
        })
    });

    // 11. Another hero selected, or standalone control off.
    suite.check("nothing is swallowed when Largo is not the active hero", |w| {
        w.largo_enabled = false;
        w.gsi_ultimate(0, true);
        w.press(0, 'q');
        w.press(500, 'w');
        w.press(900, 'r');
        w.run(2_000);

        expect(
            w.raw_keys() == vec!['q', 'w', 'r'],
            || format!("expected every key through, got {:?}", w.raw_keys()),
        )?;
        expect(w.beats_fired.is_empty(), || {
            format!("{} beats fired with Largo disabled", w.beats_fired.len())
        })
    });

    // 12. Losing the scepter must not leave the pair half-playing.
    suite.check("losing the scepter drops back to one song cleanly", |w| {
        w.gsi_aggs(0, true);
        w.gsi_ultimate(0, true);
        w.press(0, 'q');
        w.press(500, 'w');
        w.gsi_aggs(1_500, false);
        w.run(2_500); // three beats: 0ms, 995ms, 1990ms

        // beat 1: q. beat 2: w+q. beat 3: w alone.
        expect(
            w.landed_keys() == vec!['q', 'w', 'q', 'w'],
            || format!("expected the pair to collapse to Hotfeet, got {:?}", w.landed_keys()),
        )
    });

    println!(
        "\n{} scenarios, {} passed, {} failed",
        suite.run,
        suite.run - suite.failures.len(),
        suite.failures.len()
    );

    if !suite.failures.is_empty() {
        for failure in &suite.failures {
            println!("  failed: {failure}");
        }
        std::process::exit(1);
    }
}
