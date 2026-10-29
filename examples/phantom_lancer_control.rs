//! Phantom Lancer control — the Ctrl latch and the illusion send, proved
//! offline before either can press a key in a game.
//!
//! The latch is a convenience that sits on the most dangerous key Dota has.
//! Ctrl turns a right-click into an order for every illusion, which is the
//! point. It also turns `Q` into "level up Spirit Lance" and `1` into "make
//! the current selection control group 1". A latch that ever leaves Ctrl down
//! under a spell or a number key costs the fight. This binary is the evidence
//! that it does not.
//!
//! Two halves:
//!
//! - `Hook` and `plan_illusion_send` — the state machines, in the shape they
//!   have in `src/actions/heroes/phantom_lancer.rs` and the Phantom Lancer
//!   branch of `src/input/keyboard.rs`.
//! - `World` — a clock, a serial injector, and a model of what Dota sees. Dota
//!   has **one** Ctrl state: a physical Ctrl-up and an injected Ctrl-up clear
//!   the same flag. The model keeps it that way, which is what makes "our
//!   Ctrl-up cut the player's Ctrl short" visible at all.
//!
//!   | Constant | Value | Where it comes from |
//!   |---|---|---|
//!   | synthetic press hold | 5ms | `PRESS_HOLD_MS` in `src/actions/heroes/phantom_lancer.rs` |
//!   | cost of one injected event | 1ms | assumed; only widens the race windows |
//!   | Ctrl-to-click gap | 15ms | config; worked in game for 181 clicks (`phantom_lancer_input_probe`) |
//!
//!   The state machines use none of them. The gap is config.
//!
//! ## The rules it enforces
//!
//! 1. A Ctrl tap toggles the latch; a hold, or Ctrl with anything else, is the
//!    player's own Ctrl and toggles nothing. Ctrl itself is never blocked.
//! 2. While latched, a right-click reaches Dota with Ctrl down.
//! 3. **No key the player did not chord with their own Ctrl ever reaches Dota
//!    under Ctrl.** Physical keys pressed while an injected Ctrl is down are
//!    deferred behind it; macro keys lift the player's own Ctrl around
//!    themselves.
//! 4. Ctrl is never left down once the player has let go of everything.
//! 5. The player's own Ctrl is never cut short by ours.
//! 6. Every physical click and key reaches Dota exactly once, in order.
//!
//! ```powershell
//! cargo run --example phantom_lancer_control              # scenario suite
//! cargo run --example phantom_lancer_control -- --verbose # every event
//! cargo run --example phantom_lancer_control -- --without defer
//! ```
//!
//! ## Every safeguard is load-bearing
//!
//! `--without <rule>` switches one off. Each must turn the suite red; a rule
//! that fails nothing is either dead or untested.
//!
//! | Rule | What it does | Fails without it |
//! |---|---|---|
//! | `defer` | a physical key waits behind an injected Ctrl | Ctrl+Q and Ctrl+2 pressed mid-wrap |
//! | `lift-player-ctrl` | a macro key lifts the player's own Ctrl around itself | Ctrl+F3 and Ctrl+1 from an illusion send under the player's Ctrl |
//!
//! An earlier version also proved a `hold` replay mode and a panic
//! Doppelganger. The probe showed click mode is enough, and the player dropped
//! the Doppelganger, so both went — along with three safeguards only they
//! needed.

use std::collections::VecDeque;

// ───────────────────────── the state machines ─────────────────────────
// This section mirrors src/actions/heroes/phantom_lancer.rs and the Phantom
// Lancer branch of src/input/keyboard.rs. Keep them in step.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Ctrl,
    Char(char),
    F(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    KeyDown(Key),
    KeyUp(Key),
    ButtonDown(Button),
    ButtonUp(Button),
}

/// One unit of work for the serial injector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Our own input: the latch's Ctrl, a macro key.
    Send(Input),
    /// A physical click the latch swallowed, replayed inside its Ctrl.
    Replay(Input),
    /// A physical event held back because it would otherwise land under an
    /// injected Ctrl, or overtake input still queued.
    Forward(Input),
    Wait(u64),
}

#[derive(Debug, Clone, Copy)]
pub struct LatchConfig {
    pub tap_ms: u64,
    pub gap_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct Binds {
    pub select_hero: Key,
    pub select_others: Key,
    pub step_ms: u64,
}

impl Default for Binds {
    fn default() -> Self {
        Self {
            select_hero: Key::Char('1'),
            select_others: Key::F(3),
            step_ms: 30,
        }
    }
}

/// What the hook does with one physical event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    Pass,
    Swallow,
    Replace(Vec<Step>),
}

const PRESS_HOLD_MS: u64 = 5;

fn tap(key: Key) -> Vec<Step> {
    vec![
        Step::Send(Input::KeyDown(key)),
        Step::Wait(PRESS_HOLD_MS),
        Step::Send(Input::KeyUp(key)),
    ]
}

#[derive(Debug, Default, Clone)]
pub struct Hook {
    ctrl_down_at: Option<u64>,
    ctrl_chorded: bool,
    pub latched: bool,
    wrapped_right: bool,
}

impl Hook {
    /// Death and hero changes land here. A click still held keeps its wrapped
    /// flag: its release is still ours to swallow.
    pub fn unlatch(&mut self) {
        self.latched = false;
    }

    /// Decide one physical event. `active` is "Phantom Lancer is the hero and
    /// the latch is enabled".
    pub fn on_physical(
        &mut self,
        now: u64,
        input: Input,
        config: &LatchConfig,
        binds: &Binds,
        active: bool,
    ) -> Plan {
        if input == Input::ButtonUp(Button::Right)
            && std::mem::replace(&mut self.wrapped_right, false)
        {
            return Plan::Swallow;
        }

        if !active {
            self.ctrl_down_at = None;
            self.latched = false;
            return Plan::Pass;
        }

        match input {
            Input::KeyDown(Key::Ctrl) => {
                // Autorepeat sends more presses while it is held.
                if self.ctrl_down_at.is_none() {
                    self.ctrl_down_at = Some(now);
                    self.ctrl_chorded = false;
                }
                Plan::Pass
            }
            Input::KeyUp(Key::Ctrl) => {
                if let Some(down_at) = self.ctrl_down_at.take() {
                    if !self.ctrl_chorded && now.saturating_sub(down_at) <= config.tap_ms {
                        self.latched = !self.latched;
                    }
                }
                Plan::Pass
            }
            Input::KeyDown(_) => {
                if self.ctrl_down_at.is_some() {
                    self.ctrl_chorded = true;
                }
                Plan::Pass
            }
            Input::KeyUp(_) => Plan::Pass,
            Input::ButtonDown(Button::Middle) => {
                if self.ctrl_down_at.is_some() {
                    self.ctrl_chorded = true;
                }
                Plan::Replace(plan_illusion_send(binds))
            }
            Input::ButtonUp(Button::Middle) => Plan::Swallow,
            Input::ButtonDown(button) => {
                // The player's own Ctrl is already doing the job, and our
                // Ctrl-up would cut it short.
                if self.ctrl_down_at.is_some() {
                    self.ctrl_chorded = true;
                    return Plan::Pass;
                }
                if !self.latched || button != Button::Right {
                    return Plan::Pass;
                }
                self.wrapped_right = true;
                Plan::Replace(vec![
                    Step::Send(Input::KeyDown(Key::Ctrl)),
                    Step::Wait(config.gap_ms),
                    Step::Replay(Input::ButtonDown(button)),
                    Step::Wait(PRESS_HOLD_MS),
                    Step::Replay(Input::ButtonUp(button)),
                    Step::Wait(config.gap_ms),
                    Step::Send(Input::KeyUp(Key::Ctrl)),
                ])
            }
            Input::ButtonUp(_) => Plan::Pass,
        }
    }
}

/// Select every other unit → right-click at the cursor → select the hero.
/// Every key is the player's own Dota bind.
pub fn plan_illusion_send(binds: &Binds) -> Vec<Step> {
    let mut steps = tap(binds.select_others);
    steps.push(Step::Wait(binds.step_ms));
    steps.extend([
        Step::Send(Input::ButtonDown(Button::Right)),
        Step::Wait(PRESS_HOLD_MS),
        Step::Send(Input::ButtonUp(Button::Right)),
        Step::Wait(binds.step_ms),
    ]);
    steps.extend(tap(binds.select_hero));
    steps
}

// ─────────────────────────────── the world ───────────────────────────────

/// One injected event costs this much of the injector's time. An assumption:
/// it only decides how wide the race windows are.
const INJECT_COST_MS: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Physical,
    Forwarded,
    Replayed,
    Synthetic,
}

#[derive(Debug, Clone, Copy)]
struct Delivered {
    at: u64,
    input: Input,
    source: Source,
    /// Dota's Ctrl state when this arrived.
    ctrl: bool,
    /// Whether the player was physically holding Ctrl at that moment.
    player_ctrl: bool,
}

#[derive(Debug, Clone, Copy)]
enum Event {
    Physical(Input),
    Death,
}

struct World {
    latch: LatchConfig,
    binds: Binds,
    hook: Hook,
    active: bool,
    now: u64,
    events: Vec<(u64, u64, Event)>,
    seq: u64,
    // The injector.
    pending: VecDeque<Step>,
    next_step_at: Option<u64>,
    /// Ctrl that the injector pressed and still owns.
    our_ctrl: bool,
    /// The player's Ctrl the injector lifted for a macro key, to put back.
    restore_player_ctrl: bool,
    // Dota.
    ctrl_in_dota: bool,
    player_ctrl: bool,
    delivered: Vec<Delivered>,
    physical: Vec<Input>,
    verbose: bool,
    /// A safeguard switched off with `--without`, to show it is load-bearing.
    without: Option<String>,
}

impl World {
    fn new(verbose: bool) -> Self {
        Self {
            latch: LatchConfig {
                tap_ms: 200,
                gap_ms: 15,
            },
            binds: Binds::default(),
            hook: Hook::default(),
            active: true,
            now: 0,
            events: Vec::new(),
            seq: 0,
            pending: VecDeque::new(),
            next_step_at: None,
            our_ctrl: false,
            restore_player_ctrl: false,
            ctrl_in_dota: false,
            player_ctrl: false,
            delivered: Vec::new(),
            physical: Vec::new(),
            verbose,
            without: None,
        }
    }

    fn keeps(&self, rule: &str) -> bool {
        self.without.as_deref() != Some(rule)
    }

    fn at(&mut self, at: u64, event: Event) -> &mut Self {
        self.seq += 1;
        self.events.push((at, self.seq, event));
        self
    }

    fn down(&mut self, at: u64, key: Key) -> &mut Self {
        self.at(at, Event::Physical(Input::KeyDown(key)))
    }

    fn up(&mut self, at: u64, key: Key) -> &mut Self {
        self.at(at, Event::Physical(Input::KeyUp(key)))
    }

    fn press(&mut self, at: u64, key: Key, hold_ms: u64) -> &mut Self {
        self.down(at, key).up(at + hold_ms, key)
    }

    fn click(&mut self, at: u64, button: Button, hold_ms: u64) -> &mut Self {
        self.at(at, Event::Physical(Input::ButtonDown(button)));
        self.at(at + hold_ms, Event::Physical(Input::ButtonUp(button)))
    }

    fn ctrl_tap(&mut self, at: u64) -> &mut Self {
        self.press(at, Key::Ctrl, 80)
    }

    fn busy(&self) -> bool {
        !self.pending.is_empty()
    }

    fn enqueue(&mut self, steps: Vec<Step>) {
        self.pending.extend(steps);
        if self.next_step_at.is_none() {
            self.next_step_at = Some(self.now);
        }
    }

    fn run(&mut self, until: u64) {
        loop {
            let next_event = self
                .events
                .iter()
                .enumerate()
                .min_by_key(|(_, (at, seq, _))| (*at, *seq))
                .map(|(index, (at, _, _))| (index, *at));
            let next = match (next_event.map(|(_, at)| at), self.next_step_at) {
                (Some(e), Some(s)) => e.min(s),
                (Some(e), None) => e,
                (None, Some(s)) => s,
                (None, None) => return,
            };
            if next > until {
                return;
            }
            self.now = self.now.max(next);

            if self.next_step_at == Some(next) && next_event.map(|(_, at)| at) != Some(next) {
                self.step_injector();
                continue;
            }
            if let Some((index, _)) = next_event {
                let (_, _, event) = self.events.remove(index);
                self.handle(event);
            }
        }
    }

    fn handle(&mut self, event: Event) {
        match event {
            Event::Physical(input) => {
                self.physical.push(input);
                match input {
                    Input::KeyDown(Key::Ctrl) => self.player_ctrl = true,
                    Input::KeyUp(Key::Ctrl) => self.player_ctrl = false,
                    _ => {}
                }
                let was_latched = self.hook.latched;
                let plan =
                    self.hook
                        .on_physical(self.now, input, &self.latch, &self.binds, self.active);
                if self.verbose && was_latched != self.hook.latched {
                    println!("  {:>6}ms  latch {}", self.now, self.hook.latched);
                }
                match plan {
                    // Rule 3: nothing physical overtakes queued input or lands
                    // under a Ctrl we still hold.
                    Plan::Pass if (self.busy() || self.our_ctrl) && self.keeps("defer") => {
                        self.enqueue(vec![Step::Forward(input)])
                    }
                    Plan::Pass => self.deliver(input, Source::Physical),
                    Plan::Swallow => {}
                    Plan::Replace(steps) => self.enqueue(steps),
                }
            }
            Event::Death => self.hook.unlatch(),
        }
    }

    fn step_injector(&mut self) {
        let Some(step) = self.pending.pop_front() else {
            self.next_step_at = None;
            return;
        };
        let mut cost = INJECT_COST_MS;
        match step {
            Step::Wait(ms) => cost = ms,
            Step::Send(Input::KeyDown(Key::Ctrl)) => {
                self.our_ctrl = true;
                self.deliver(Input::KeyDown(Key::Ctrl), Source::Synthetic);
            }
            Step::Send(Input::KeyUp(Key::Ctrl)) => {
                self.our_ctrl = false;
                self.deliver(Input::KeyUp(Key::Ctrl), Source::Synthetic);
            }
            Step::Send(Input::KeyDown(key)) => {
                // The player's Ctrl turns a macro key into a level-up or a
                // control-group assignment too.
                if !self.our_ctrl && self.player_ctrl && self.keeps("lift-player-ctrl") {
                    self.deliver(Input::KeyUp(Key::Ctrl), Source::Synthetic);
                    self.restore_player_ctrl = true;
                }
                self.deliver(Input::KeyDown(key), Source::Synthetic);
            }
            Step::Send(Input::KeyUp(key)) => {
                self.deliver(Input::KeyUp(key), Source::Synthetic);
                if std::mem::take(&mut self.restore_player_ctrl) && self.player_ctrl {
                    self.deliver(Input::KeyDown(Key::Ctrl), Source::Synthetic);
                }
            }
            Step::Send(input) => self.deliver(input, Source::Synthetic),
            Step::Forward(input) => self.deliver(input, Source::Forwarded),
            Step::Replay(input) => self.deliver(input, Source::Replayed),
        }
        self.next_step_at = if self.pending.is_empty() {
            None
        } else {
            Some(self.now + cost)
        };
    }

    fn deliver(&mut self, input: Input, source: Source) {
        match input {
            Input::KeyDown(Key::Ctrl) => self.ctrl_in_dota = true,
            Input::KeyUp(Key::Ctrl) => self.ctrl_in_dota = false,
            _ => {}
        }
        if self.verbose {
            println!(
                "  {:>6}ms  -> Dota {:?} ({:?}){}",
                self.now,
                input,
                source,
                if self.ctrl_in_dota { " [ctrl]" } else { "" }
            );
        }
        self.delivered.push(Delivered {
            at: self.now,
            input,
            source,
            ctrl: self.ctrl_in_dota,
            player_ctrl: self.player_ctrl,
        });
    }

    fn synthetic_keys(&self) -> Vec<Key> {
        self.delivered
            .iter()
            .filter(|d| d.source == Source::Synthetic)
            .filter_map(|d| match d.input {
                Input::KeyDown(key) if key != Key::Ctrl => Some(key),
                _ => None,
            })
            .collect()
    }

    fn synthetic_clicks(&self) -> Vec<Button> {
        self.delivered
            .iter()
            .filter(|d| d.source == Source::Synthetic)
            .filter_map(|d| match d.input {
                Input::ButtonDown(button) => Some(button),
                _ => None,
            })
            .collect()
    }

    fn clicks_down(&self, button: Button) -> Vec<Delivered> {
        self.delivered
            .iter()
            .filter(|d| d.input == Input::ButtonDown(button) && d.source != Source::Synthetic)
            .copied()
            .collect()
    }

    /// Rules 3, 4 and 6, checked after every scenario.
    fn invariants(&self) -> Result<(), String> {
        for d in &self.delivered {
            let Input::KeyDown(key) = d.input else {
                continue;
            };
            if key == Key::Ctrl || !d.ctrl {
                continue;
            }
            let the_players_chord =
                d.player_ctrl && matches!(d.source, Source::Physical | Source::Forwarded);
            if !the_players_chord {
                return Err(format!(
                    "Ctrl+{key:?} reached Dota at {}ms ({:?}) — a level-up or a lost control group",
                    d.at, d.source
                ));
            }
        }

        if self.ctrl_in_dota && !self.player_ctrl {
            return Err(String::from("Ctrl was left down after the player let go"));
        }

        for input in &self.physical {
            // The illusion-send trigger is ours; Dota never sees it.
            if matches!(
                input,
                Input::ButtonDown(Button::Middle) | Input::ButtonUp(Button::Middle)
            ) {
                continue;
            }
            let pressed = self.physical.iter().filter(|p| *p == input).count();
            let arrived = self
                .delivered
                .iter()
                .filter(|d| d.input == *input && d.source != Source::Synthetic)
                .count();
            if pressed != arrived {
                return Err(format!(
                    "{input:?} pressed {pressed}x, reached Dota {arrived}x"
                ));
            }
        }

        // Order of the player's own keys must survive deferral.
        let pressed_keys: Vec<Key> = self
            .physical
            .iter()
            .filter_map(|p| match p {
                Input::KeyDown(key) => Some(*key),
                _ => None,
            })
            .collect();
        let arrived_keys: Vec<Key> = self
            .delivered
            .iter()
            .filter(|d| d.source != Source::Synthetic)
            .filter_map(|d| match d.input {
                Input::KeyDown(key) => Some(key),
                _ => None,
            })
            .collect();
        if pressed_keys != arrived_keys {
            return Err(format!(
                "key order changed: pressed {pressed_keys:?}, arrived {arrived_keys:?}"
            ));
        }
        Ok(())
    }
}

// ──────────────────────────── the scenarios ────────────────────────────

struct Suite {
    verbose: bool,
    without: Option<String>,
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
        world.without = self.without.clone();
        let result = body(&mut world).and_then(|()| world.invariants());
        match result {
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
    let args: Vec<String> = std::env::args().collect();
    let verbose = args.iter().any(|a| a == "--verbose");
    let without = args
        .iter()
        .position(|a| a == "--without")
        .and_then(|index| args.get(index + 1).cloned());
    if let Some(rule) = &without {
        println!("safeguard switched off: {rule} — failures below are the point\n");
    }
    let mut suite = Suite {
        verbose,
        without,
        failures: Vec::new(),
        run: 0,
    };

    println!("Phantom Lancer control suite");
    println!(
        "tap ≤200ms, gap 15ms, injected event {}ms, illusion send step {}ms\n",
        INJECT_COST_MS,
        Binds::default().step_ms
    );

    // ── the latch ────────────────────────────────────────────────────────

    suite.check("a clean tap toggles the latch, a second clears it", |w| {
        w.ctrl_tap(0);
        w.run(500);
        expect(w.hook.latched, || String::from("one tap did not latch"))?;
        w.ctrl_tap(600);
        w.run(1_000);
        expect(!w.hook.latched, || {
            String::from("the second tap did not clear it")
        })
    });

    suite.check("a held Ctrl is the player's and toggles nothing", |w| {
        w.press(0, Key::Ctrl, 450);
        w.run(1_000);
        expect(!w.hook.latched, || String::from("a 450ms hold latched"))
    });

    suite.check("Ctrl+1 is a chord, never a toggle, however fast", |w| {
        w.down(0, Key::Ctrl)
            .press(20, Key::Char('1'), 20)
            .up(60, Key::Ctrl);
        w.run(500);
        expect(!w.hook.latched, || String::from("a 60ms Ctrl+1 latched"))
    });

    suite.check("autorepeat while Ctrl is held is one episode", |w| {
        w.down(0, Key::Ctrl)
            .down(30, Key::Ctrl)
            .down(60, Key::Ctrl)
            .up(90, Key::Ctrl);
        w.run(500);
        expect(w.hook.latched, || String::from("repeats broke the tap"))?;
        let downs = w
            .delivered
            .iter()
            .filter(|d| d.input == Input::KeyDown(Key::Ctrl))
            .count();
        expect(downs == 3, || {
            format!("Ctrl itself must pass through; {downs} of 3 did")
        })
    });

    suite.check("a latched right-click reaches Dota under Ctrl", |w| {
        w.ctrl_tap(0);
        w.click(300, Button::Right, 40);
        w.click(900, Button::Right, 300);
        w.run(2_000);
        let clicks = w.clicks_down(Button::Right);
        expect(clicks.len() == 2, || {
            format!("{} clicks arrived", clicks.len())
        })?;
        expect(
            clicks
                .iter()
                .all(|c| c.ctrl && c.source == Source::Replayed),
            || format!("a latched click arrived without Ctrl: {clicks:?}"),
        )
    });

    suite.check("an unlatched right-click passes straight through", |w| {
        w.click(0, Button::Right, 40);
        w.run(500);
        let clicks = w.clicks_down(Button::Right);
        expect(
            clicks.len() == 1 && clicks[0].source == Source::Physical && !clicks[0].ctrl,
            || format!("expected one raw click, got {clicks:?}"),
        )
    });

    suite.check("the player's held Ctrl survives a latched click", |w| {
        w.ctrl_tap(0);
        w.down(400, Key::Ctrl);
        w.click(450, Button::Right, 40);
        w.run(800);
        let click = w.clicks_down(Button::Right);
        expect(
            click.len() == 1 && click[0].source == Source::Physical,
            || format!("manual mode should pass the click untouched: {click:?}"),
        )?;
        expect(w.ctrl_in_dota, || {
            String::from("our Ctrl-up cut the player's Ctrl short")
        })?;
        w.up(900, Key::Ctrl);
        w.run(1_200);
        expect(w.hook.latched, || {
            String::from("a Ctrl+click toggled the latch")
        })
    });

    suite.check("a spell pressed mid-wrap never lands under Ctrl", |w| {
        w.ctrl_tap(0);
        w.click(300, Button::Right, 40);
        w.press(303, Key::Char('q'), 30);
        w.press(306, Key::Char('2'), 30);
        w.run(1_000);
        Ok(())
    });

    suite.check(
        "a spell while the right button is held casts, never levels",
        |w| {
            w.ctrl_tap(0);
            w.click(300, Button::Right, 800);
            w.press(600, Key::Char('q'), 40);
            w.run(2_000);
            let q = w
                .delivered
                .iter()
                .find(|d| d.input == Input::KeyDown(Key::Char('q')))
                .copied();
            expect(q.is_some_and(|q| q.at < 700), || {
                format!("Q was held back too long: {q:?}")
            })
        },
    );

    suite.check(
        "the player's Ctrl pressed during a held click outlives it",
        |w| {
            w.ctrl_tap(0);
            w.click(300, Button::Right, 1_000);
            w.down(600, Key::Ctrl).press(650, Key::Char('1'), 30);
            w.run(1_500);
            expect(w.ctrl_in_dota, || {
                String::from("the button release let go of the player's held Ctrl")
            })?;
            w.up(1_800, Key::Ctrl);
            w.run(2_500);
            expect(w.hook.latched, || {
                String::from("the chord toggled the latch")
            })
        },
    );

    suite.check(
        "tapping the latch off mid-click still lets go of our Ctrl",
        |w| {
            w.ctrl_tap(0);
            w.click(300, Button::Right, 900);
            w.ctrl_tap(600);
            w.run(2_000);
            expect(!w.hook.latched, || String::from("the second tap was lost"))
        },
    );

    suite.check("dying unlatches; the next click is raw", |w| {
        w.ctrl_tap(0);
        w.at(300, Event::Death);
        w.click(600, Button::Right, 40);
        w.run(1_000);
        let click = w.clicks_down(Button::Right);
        expect(click.len() == 1 && !click[0].ctrl, || {
            format!("a click after death went out unified: {click:?}")
        })
    });

    suite.check("dying mid-click still swallows its release", |w| {
        w.ctrl_tap(0);
        w.click(300, Button::Right, 900);
        w.at(500, Event::Death);
        w.run(2_000);
        Ok(())
    });

    suite.check("another hero: taps do nothing, clicks are raw", |w| {
        w.active = false;
        w.ctrl_tap(0);
        w.click(300, Button::Right, 40);
        w.run(1_000);
        expect(!w.hook.latched, || {
            String::from("the latch turned on for another hero")
        })?;
        expect(w.clicks_down(Button::Right).iter().all(|c| !c.ctrl), || {
            String::from("a click went out unified for another hero")
        })
    });

    suite.check("left-clicks are left alone while latched", |w| {
        w.ctrl_tap(0);
        w.click(300, Button::Left, 40);
        w.run(800);
        let click = w.clicks_down(Button::Left);
        expect(click.len() == 1 && !click[0].ctrl, || {
            format!("a left-click went out under Ctrl: {click:?}")
        })
    });

    // ── the illusion send ────────────────────────────────────────────────

    suite.check(
        "illusion send: select others, right-click, select hero",
        |w| {
            w.click(0, Button::Middle, 40);
            w.run(1_000);
            let keys = w.synthetic_keys();
            let want = vec![Key::F(3), Key::Char('1')];
            expect(keys == want, || format!("expected {want:?}, got {keys:?}"))?;
            expect(w.synthetic_clicks() == vec![Button::Right], || {
                format!("expected one right-click, got {:?}", w.synthetic_clicks())
            })?;
            expect(
                !w.delivered
                    .iter()
                    .any(|d| d.input == Input::ButtonDown(Button::Middle)),
                || String::from("the middle click leaked to Dota"),
            )
        },
    );

    suite.check("illusion send mid-wrap never Ctrl+1s", |w| {
        w.ctrl_tap(0);
        w.click(300, Button::Right, 40);
        w.click(303, Button::Middle, 40);
        w.run(2_000);
        Ok(())
    });

    suite.check(
        "illusion send under the player's own Ctrl never Ctrl+1s",
        |w| {
            w.down(0, Key::Ctrl);
            w.click(50, Button::Middle, 40);
            w.up(600, Key::Ctrl);
            w.run(1_000);
            expect(!w.hook.latched, || {
                String::from("Ctrl+middle toggled the latch")
            })
        },
    );

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
