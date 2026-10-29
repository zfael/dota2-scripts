# Phantom Lancer Automation

## Purpose

Learn how the Phantom Lancer script keeps Dota's unified orders on without a
held Ctrl, and sends the illusions off with one button.
**Read this when:** configuring the Ctrl latch or the illusion send, debugging
why illusions did or did not follow an order, or changing the Phantom Lancer
branch of the keyboard hook.

## Feature Summary

- **Ctrl latch** – Tap Ctrl and every right-click becomes a unified order, so
  all illusions follow it; tap again to stop
- **Audio cue** – Rising when the latch turns on, falling when it turns off
- **On-screen indicator** – A click-through box over Dota, green "LATCH ON" or
  red "LATCH OFF", left of and below the hero by default, and movable
- **Illusion send** – Middle mouse selects every other unit, right-clicks at
  the cursor and reselects the hero: the illusions go, the hero stays. It
  turns the latch off, so the hero's next right-click is its own
- **GSI-based detection** – Auto-enables when `npc_dota_hero_phantom_lancer` is
  detected; turns the latch off on death
- **Survivability actions** – The shared default strategy, unchanged from
  before Phantom Lancer had a script


## Configuration

All settings in `config/config.toml` under `[heroes.phantom_lancer]`:

```toml
[heroes.phantom_lancer]
ctrl_latch_enabled = true
latch_tap_ms = 200
latch_ctrl_gap_ms = 15
latch_cue_enabled = true
latch_cue_volume = 0.5
illusion_send_enabled = true
illusion_send_trigger = "Mouse3"
select_other_units_key = "F3"
select_hero_key = "1"
illusion_send_step_ms = 30
latch_indicator_enabled = true
latch_indicator_offset_x = -560
latch_indicator_offset_y = 500
latch_indicator_preview = false
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `ctrl_latch_enabled` | bool | `true` | Master toggle for the latch |
| `latch_tap_ms` | u64 | `200` | Longest Ctrl press that counts as a tap |
| `latch_ctrl_gap_ms` | u64 | `15` | Gap between the injected Ctrl and the replayed click, on each side |
| `latch_cue_enabled` | bool | `true` | Play the on/off cue |
| `latch_cue_volume` | f32 | `0.5` | Cue volume, `0.0`–`1.0` |
| `illusion_send_enabled` | bool | `true` | Master toggle for the illusion send |
| `illusion_send_trigger` | key binding | `"Mouse3"` | What fires the send; swallowed, so Dota never sees it |
| `select_other_units_key` | key binding | `"F3"` | Your Dota bind for "Select All Other Units" |
| `select_hero_key` | key binding | `"1"` | Your Dota bind for selecting the hero |
| `illusion_send_step_ms` | u64 | `30` | Gap between the send's steps |
| `latch_indicator_enabled` | bool | `true` | Show the on-screen latch box |
| `latch_indicator_offset_x` | i32 | `-560` | Pixels right of the centre of Dota's window (negative is left) |
| `latch_indicator_offset_y` | i32 | `500` | Pixels below the centre (negative is up) |
| `latch_indicator_preview` | bool | `false` | Keep the box on screen while positioning it |

All fourteen are exposed under **Heroes → Phantom Lancer**. Key bindings accept a
letter, a named key (`"F3"`) or a mouse button (`"Mouse3"`–`"Mouse5"`). If any
of the three send bindings does not resolve, the send is off and a warning is
logged, rather than firing half a sequence.

**Dota side:** "Unified Orders with Ctrl" must be on in Dota's options. The
latch only supplies the Ctrl; that option is what sends the order to every
unit.

## Related Files

| File | Purpose |
|------|---------|
| `src/actions/heroes/phantom_lancer.rs` | Latch state machine, injector, echo matching, hero script |
| `src/input/keyboard.rs` | Calls the latch first on every key and button event |
| `src/config/settings.rs` | `PhantomLancerConfig` struct and defaults |
| `src-ui/src/components/heroes/configs/PhantomLancerConfig.tsx` | Config panel |
| `src/observability/latch_indicator.rs` | Indicator placement and show/hide rule |
| `src-tauri/src/commands/latch_indicator.rs` | Indicator window and the worker that drives it |
| `src-ui/src/pages/LatchIndicator.tsx` | The green/red box itself |
| `examples/phantom_lancer_input_probe.rs` | In-game probe: re-measure after a patch |
| `examples/phantom_lancer_control.rs` | Offline proof that Ctrl never lands under a key |

---

## Details

### 🔱 Ctrl Latch

Dota sends an order to every controlled unit while Ctrl is held. Phantom
Lancer's illusions make that nearly every right-click, so the latch holds Ctrl
for the player — but only around the click.

**Why not just hold Ctrl down.** Ctrl + an ability key levels the ability
instead of casting it, and Ctrl + a number key reassigns that control group. A
held Ctrl would turn every spell in a fight into a skill point.

#### Rules

1. **Toggle.** A clean Ctrl tap — nothing else pressed while it is down,
   released within `latch_tap_ms` — toggles the latch. Holding Ctrl, or Ctrl
   with any other key or click, is the player's own Ctrl and toggles nothing.
   Ctrl itself is never blocked: a bare Ctrl does nothing in Dota, and a tap is
   only known once it is released.
2. **Wrap.** While latched, a right-click is swallowed and replayed as
   Ctrl down → `latch_ctrl_gap_ms` → click → `latch_ctrl_gap_ms` → Ctrl up. Its
   physical release is swallowed too, even if the latch turns off in between.
3. **Defer.** While the injected Ctrl is down or the injector has work queued,
   every physical key and click is held back and replayed behind it, in order.
   A spell pressed in that ~35ms window would otherwise reach Dota as Ctrl+Q.
4. **Manual mode.** With Ctrl physically held, clicks pass untouched: the
   player's own Ctrl already does the job, and our Ctrl-up would cut it short.
5. **Lift.** A key the injector presses for the illusion send releases the
   player's held Ctrl first and puts it back after. Windows tracks left and
   right Ctrl separately, so it lifts whichever one is held.
6. **Off.** Death, another hero, the latch disabled, or an illusion send
   turns it off. The send plays the falling cue like a tap does.

Left-clicks, keys and items are never wrapped.

#### Telling our input from the player's

The app's `SIMULATING_KEYS` guard passes every event while it is set, physical
ones included — exactly the window rule 3 has to catch. So the Phantom Lancer
injector does not use it. It records each event before sending it, and
`is_own_echo` at the very top of the hook recognises those echoes by matching
the front of that queue. An echo not seen within 500ms is dropped, so a lost
one cannot mistake a later physical event for ours.

#### Trigger model

- **Key interception**: the latch and the send run in `src/input/keyboard.rs`
  ahead of every other branch, on key presses, key releases and mouse
  buttons — rule 3 has to see the events any other branch would pass straight
  to Dota
- **Passive (GSI-driven)**: `handle_gsi_event` only turns the latch off on
  death, then runs the shared default strategy

### 🖱️ Illusion Send

One press of `illusion_send_trigger` (default middle mouse) runs:

```
select_other_units_key ▸ step ▸ right-click at the cursor ▸ step ▸ select_hero_key
```

- **The trigger is swallowed**, press and release. With the default that costs
  the middle-mouse camera drag while Phantom Lancer is the hero.
- **It turns the latch off.** The illusions have their order; left on, the
  hero's next right-click would call them all back. Tap Ctrl to latch again.
- **A keyboard trigger fires once per press**: autorepeat is ignored until the
  key is released.
- **Your own Ctrl is lifted around each key** (rule 5 below). Without it,
  holding Ctrl while pressing the trigger would send Ctrl+1 — "assign control
  group 1" — instead of "select the hero". The Ctrl goes back down after each
  key if you are still holding it.
- It queues behind a latched click in flight, so it never runs under the
  latch's Ctrl either.
- Ctrl + trigger is a chord: it never toggles the latch.
- No GSI gate: there may be no illusions to send, and the cost is two
  selection changes.

### 🟩 Latch Indicator

A small click-through window over Dota: green with **LATCH ON** while latched,
red with **LATCH OFF** while not. The cue says the state changed; the box says
what it is now.

- **When it shows.** While Phantom Lancer is the hero, the latch and the
  indicator are both enabled, and Dota is the window in front. Alt-tab away
  and it hides, so it never floats over the desktop. `latch_indicator_preview`
  drops the hero and the window-in-front conditions so it can be positioned
  from the app; it still needs Dota's window to centre on.
- **Where.** The centre of Dota's client area, shifted by
  `latch_indicator_offset_x` / `_y` in physical pixels, and clamped so it
  stays inside the window. Its height is 3% of Dota's (43px at 1440p, 32px at
  1080p), so it reads the same at any resolution.
- **How fast.** A worker thread reads the latch every 30ms and broadcasts
  changes (`pl_latch_update`); it re-reads Dota's window position every
  300ms. The view also asks for the state every 500ms, so a lost event can
  never leave the box wrong for long.
- **Click-through.** Like the wave overlay it ignores the mouse, so a click
  on it reaches the game.
- The window is built the first time it is needed, not at startup, and is
  closed with the main window.
- **Its label, `pl-latch-indicator`, must be in
  `src-tauri/capabilities/default.json`.** A window missing from it gets no
  IPC: the view can neither listen nor ask, and sits on LATCH OFF. A test in
  `commands/latch_indicator.rs` checks both overlay windows are listed.

### 🛡️ Survivability Actions

Phantom Lancer's script calls `SurvivabilityActions::execute_default_strategy`,
the same path every hero without a script takes: healing items, defensive
items, neutral items and danger detection. Shared `[common]`,
`[danger_detection]` and `[neutral_items]` config applies.

### 🔄 State Diagram

```
              clean tap                       clean tap
   ┌─────────┐ ─────────────────────► ┌─────────┐ ──────────► (off)
   │   OFF   │                        │ LATCHED │
   └─────────┘ ◄───────────────────── └─────────┘
              death / other hero /           │ right-click
              latch disabled / send          ▼
                                   Ctrl ▸ gap ▸ click ▸ gap ▸ Ctrl up
                                   (physical input queued behind it)
```

### 🔒 Thread Safety

- The latch state is one `Mutex<CtrlLatch>`, locked briefly from the hook
  thread and from the GSI thread (death only). Never held across a key press.
- One injector thread owns every injected event, in FIFO order.
  `PENDING_JOBS` and `OWN_CTRL` are atomics the hook reads to decide rule 3.
- The cue plays on a spawned thread: the first one opens the audio device.

### Evidence

From the probe captures `logs/phantom_lancer/probe_2026-10-28_20-21-32.jsonl`
and `probe_2026-10-28_20-25-13.jsonl` (click mode, 15ms gap):

| Measured | Value |
|---|---|
| Natural Ctrl taps | 12, all 81–121ms |
| Manual holds | 441–1587ms, every one with a click in it, so a chord |
| Quick Ctrl+key presses | 74–98ms, inside the tap window: the chord rule is what keeps them from toggling |
| Latched right-clicks | 181; the player reports every illusion followed, attacks included, and nothing levelled |
| Illusion send, `a` + left-click | 8 sends (`probe_2026-10-28_20-29-33.jsonl`); the player reports it did not work |
| Illusion send, bare left-click, in the app | Both selections landed, no order: in Dota a left-click only selects. Fixed by ordering with a right-click |

Not measured: the minimum working gap (the probe's `F7` sweep was not run),
the minimum send step, hold-to-move under the latch, and left-click wrapping.

`examples/phantom_lancer_control.rs` proves rules 1–6 offline. Its
`--without defer` run shows Ctrl+Q reaching Dota without rule 3, and
`--without lift-player-ctrl` shows the send producing Ctrl+F3 and Ctrl+1
without rule 5.

**After a gameplay patch**, re-run the probe (steps 1–5 of its capture
protocol) before trusting the latch again.

### Usage

1. Turn on **Unified Orders with Ctrl** in Dota's options
2. Pick Phantom Lancer (auto-detected via GSI)
3. Tap Ctrl — rising cue, green box — and right-click as normal
4. Tap Ctrl again — falling cue, red box — for single-unit orders
5. Middle-click where the illusions should go; you keep control of the hero,
   and the latch turns off (falling cue)

### Tuning

- **`latch_tap_ms`**: lower it if Ctrl presses you meant as holds toggle the
  latch; raise it if deliberate taps are missed. Keep it above your natural
  tap (81–121ms measured).
- **`latch_ctrl_gap_ms`**: raise it if some illusions miss a latched order.
  Every millisecond is added twice to the window in which keys are deferred.
- **`illusion_send_step_ms`**: raise it if the hero moves instead of the
  illusions, or the hero is not reselected. Every millisecond is added twice
  to the window in which keys are deferred.
- **`latch_indicator_offset_x` / `_y`**: turn on Preview, move the box off the
  hero (the default, `-560` / `500`, was placed in game left of and below it), then turn
  Preview off.

### Logging

With `level = "info"`:
```
🔱 Phantom Lancer Ctrl latch ON
🔱 Phantom Lancer Ctrl latch off
🔱 Phantom Lancer Ctrl latch off: hero died
```

With `level = "warn"`, a send binding that does not resolve:
```
🔱 Phantom Lancer illusion send off: a binding did not resolve (trigger …, select others …, select hero …)
```

With `level = "debug"`:
```
🔱 Phantom Lancer: ButtonPress(Right) replaced by 7 steps
🔱 Phantom Lancer: ButtonPress(Middle) replaced by 11 steps
```

### Limitations

- **Deferred keys skip the rest of the hook.** A key held back by rule 3 is
  replayed straight to Dota, so it does not trigger Soul Ring or an app hotkey.
  The window is ~35ms after a latched right-click, and ~80ms after an
  illusion send.
- **Other features' simulated input.** While another feature's enigo input or
  Soul Ring replay is in flight, `SIMULATING_KEYS` passes every event, so a
  right-click in that instant goes out unwrapped.
- **Hold-to-move is not unified.** Holding the right button down sends one
  wrapped click, not a held one.
- **The indicator needs Borderless or Windowed.** Like the wave overlay, no
  window can draw over exclusive fullscreen.
- **The send's order is a plain right-click**: move, or attack whatever is
  under the cursor. There is no attack-move option; an attack key before the
  click did not work in the probe.
- **Middle-mouse camera drag is gone** while Phantom Lancer is the hero and
  the send uses the default trigger.

---

## Maintenance Checklist

When editing this hero's code, update this doc:

- [ ] New config option added? → Update Configuration table
- [ ] New behavior/feature? → Add section under Details
- [ ] Changed the wrap or the defer rule? → Re-run `phantom_lancer_control`, update Rules
- [ ] Modified trigger model? → Update trigger description
- [ ] New logging statements? → Update Logging section
