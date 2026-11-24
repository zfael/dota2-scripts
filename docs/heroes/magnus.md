# Magnus Automation

## Purpose

Directional **Reverse Polarity** (R) on a single keypress, and an optional
**saved point** (anchor) that Skewer aims at — on every Skewer, or only right
after a Blink.
**Read this when:** changing the Magnus ultimate key, the facing technique, the turn timing, the readiness gate, or the saved-point options.

## Feature Summary

- The ultimate key (default **R**) is intercepted while Magnus is the active hero.
- Combo: `ALT down → right-click (face cursor) → ALT up → wait turn_delay_ms → press R → optional camera recentre`.
- Reverse Polarity drags enemies to the arc **in front of Magnus**, so facing at cast time decides where they land. Turning toward the cursor first lines the pull up with the Skewer that follows.
- **Gated on GSI**: the key is only swallowed when Reverse Polarity is levelled and castable. On cooldown it passes straight through, so Magnus never takes the facing right-click for nothing.
- **Saved point (off by default).** Hover a spot — usually the minimap, e.g. your
  base — and press the save key (default **PageDown**). Press it again anywhere to
  move the point.
  - **Skewer to saved point**: the Skewer key (default **E**) moves the cursor to
    the point, presses E, and puts the cursor back. A point past Skewer's range
    gives a full-length Skewer toward it. **Needs Quick Cast on the Skewer key.**
  - **Only after Blink** (off by default, needs the option above): Skewer goes to
    the point only when Magnus carries a Blink and it is on cooldown, or its key
    was pressed in the last `blink_press_window_ms` — the Blink-in, Skewer-out
    play. Any other Skewer casts normally at the cursor.
- With Skewer to saved point off, Skewer and the save key are not intercepted at all.

## Configuration

`config/config.toml` under `[heroes.magnus]`:

| Field | Type | Default | Purpose |
|---|---|---|---|
| `enabled` | bool | `true` | Master toggle for the intercept. |
| `ultimate_key` | char | `"r"` | Reverse Polarity ability key; also the key intercepted. |
| `turn_delay_ms` | u64 | `60` | Delay after the facing right-click before the ultimate cast. |
| `require_ability_ready` | bool | `true` | Pass the key through when the ability it casts (Reverse Polarity, or Skewer for the saved-point option) is unlevelled or on cooldown. |
| `center_camera_on_ultimate` | bool | `true` | Double-tap the hero-select key after the cast to recentre the camera. |
| `camera_center_key` | string | `"1"` | Hero-select key to double-tap. Character (`"1"`) or named key (`"F1"`). |
| `camera_center_delay_ms` | u64 | `60` | Delay between the cast and the first camera tap. |
| `skewer_to_anchor_enabled` | bool | `false` | Skewer key casts at the saved point. Requires Quick Cast on Skewer in Dota. |
| `skewer_key` | char | `"e"` | Skewer ability key; intercepted only while the option above is on and a point is saved. |
| `skewer_only_after_blink` | bool | `false` | Only Skewer to the point when a carried Blink is on cooldown or its key was just pressed; otherwise a normal cast. |
| `blink_press_window_ms` | u64 | `1000` | How long a Blink key press counts as "just blinked", covering the moment before GSI reports the cooldown. |
| `capture_anchor_key` | string | `"PageDown"` | Saves the cursor as the point. Character or named key. Ours (blocked from Dota) only while Skewer to saved point is on. |
| `anchor_settle_ms` | u64 | `30` | How long the cursor rests on the point before the press, and again before it returns. |
| `anchor_set` | bool | `false` | Whether a point has been saved. Set by the save key. |
| `anchor_x_fraction` / `anchor_y_fraction` | f32 | `0.0` | The point as a fraction of Dota's client area. Set by the save key; do not edit by hand. |

```toml
[heroes.magnus]
enabled = true
ultimate_key = "r"
turn_delay_ms = 60
require_ability_ready = true
center_camera_on_ultimate = true
camera_center_key = "1"
camera_center_delay_ms = 60
skewer_to_anchor_enabled = false
skewer_key = "e"
skewer_only_after_blink = false
blink_press_window_ms = 1000
capture_anchor_key = "PageDown"
anchor_settle_ms = 30
anchor_set = false
anchor_x_fraction = 0.0
anchor_y_fraction = 0.0
```

Everything is exposed in the React UI under **Heroes → Magnus**. The three
anchor fields are shown read-only (as a "saved / not saved yet" status): the
only way to set them is the save key, so a typed-in point can never be aimed at.

## Related Files

| File | Purpose |
|---|---|
| `src/actions/heroes/magnus.rs` | Hero script, dedicated worker, `MagnusState::execute_directional_ultimate` / `execute_skewer_to_anchor`, readiness gates, the Blink gate (`blink_snapshot`, `blink_gate_open`, `note_key_press`), `apply_anchor_capture`. |
| `src/input/keyboard.rs` | `plan_magnus_key_action` (save key, ultimate, Skewer) + `MagnusKeyboardSnapshot`. |
| `src/input/simulation.rs` | `press_key_at_point`: the move-press-return job. |
| `src/observability/hud_anchors.rs` | `capture_cursor_fraction` / `resolve_fraction_point`, shared with the HUD portrait anchor. |
| `src-tauri/src/commands/hud.rs` | `capture_magnus_anchor_from_hotkey`: saves and pushes the config to the UI. |
| `src/config/settings.rs` | `MagnusConfig` + defaults. |
| `config/config.toml` | `[heroes.magnus]` block. |
| `src-ui/src/components/heroes/configs/MagnusConfig.tsx` | UI config panel. |
| `tests/fixtures/magnus_event.json` | GSI fixture backing the readiness tests. |

## Activation

`magnus_enabled` is derived in `KeyboardSnapshot::from_runtime` from
`selected_hero == Some(HeroType::Magnus)`, which is set by GSI hero detection
(`AppState::update_from_gsi`) or the manual-override selection. No dedicated
`Arc<Mutex<bool>>` flag is used — same model as Snapfire.

## Details

### Input sequence

1. The keyboard hook (`src/input/keyboard.rs`) reads the cached `KeyboardSnapshot`.
2. If `magnus_enabled && magnus.enabled` and the pressed key equals the parsed
   `ultimate_key`, the hook checks `MagnusState::can_intercept_ultimate()`
   (skipped when `require_ability_ready = false`).
3. On a pass, the hook **blocks** the original key and calls
   `MagnusState::execute_directional_ultimate(ultimate_char, turn_delay_ms)`.
   On a fail it falls through and the key reaches Dota unchanged.
4. The dedicated Magnus worker runs:
   `alt_down() → mouse_click() → alt_up() → sleep(turn_delay_ms) → press_key(ultimate_char)`,
   using the `src/input/simulation.rs` helpers.
5. If camera recentring is on, the worker then sleeps `camera_center_delay_ms`
   and taps the hero-select key twice, 30ms apart.

`SIMULATING_KEYS` guards the synthetic right-click and R press so they are not
re-intercepted by the global hook.

ALT is held only across the facing right-click. Reverse Polarity takes no
target, so unlike Snapfire's cookie the cast itself does not need the modifier.

### Camera recentre

Dota treats a second press of the hero-select key as "centre on selection", so
the worker taps the configured key twice. It runs **after** the cast, never
before: centring the camera moves the world point under the cursor, so doing it
first would change where the ultimate faces.

The tap uses `src/input/keyboard.rs::simulate_key` (the `rdev` replay path)
rather than the char-based `simulation.rs` helpers, because the key is
configurable and may be a named key such as `F1` that a `char` cannot express.
`SIMULATING_KEYS` still guards it. Because it runs after the ALT hold is
released, it never contends with the modifier.

`plan_camera_center(...)` resolves the step to `None` when the toggle is off or
the configured key fails to parse, so a typo degrades to "no camera tap" rather
than pressing something unexpected.

### Readiness gate

`MagnusState::can_intercept_ultimate()` reads `MAGNUS_LAST_EVENT`, refreshed on
every `handle_gsi_event`, and requires `magnataur_reverse_polarity` to have
`level > 0 && can_cast`. It returns `false` when no GSI event has arrived yet,
so before the first payload R behaves normally.

### Saved point (anchor)

**Saving.** `plan_magnus_key_action` maps the save key to
`HotkeyEvent::CaptureMagnusAnchor` while Magnus is active and
`skewer_to_anchor_enabled` is on, and blocks the key. The hotkey thread (both binaries) calls
`magnus::capture_anchor_from_hotkey`, which reads the cursor with
`hud_anchors::capture_cursor_fraction`, writes `anchor_*` to settings, saves the
config, and patches `magnus.anchor` on the cached keyboard snapshot so the next
press uses it immediately. The Tauri app then broadcasts the config so the
Magnus panel shows the new status. A cursor outside Dota's window is refused
(nothing saved, a warning logged).

The point is stored as a **fraction of Dota's client area** — same basis as the
HUD portrait anchor — and resolved to pixels at use time. That is why it should
be on the **minimap**: a minimap pixel is always the same place on the map,
while a pixel in the game view is a different world point every time the camera
moves. Radiant and Dire bases are on opposite corners, so re-save it per game.

**Skewer.** With `skewer_to_anchor_enabled`, a point saved, and (when
`require_ability_ready`) `magnataur_skewer` levelled and castable, the Skewer key
is blocked and the Magnus worker runs one synthetic-input job
(`SyntheticAction::AtPoint`):

`hold the player's clicks → remember cursor → move to point → wait anchor_settle_ms → press E → wait anchor_settle_ms → move cursor back → replay held clicks`

Any of those conditions failing lets E through as a normal cast at the cursor.
If the cursor position cannot be read, nothing is pressed (the cursor could not
be put back). If Dota's window is not found, the cast is skipped.

**Quick Cast is required.** The press *is* the cast only with Quick Cast on the
Skewer key; with normal cast it just arms targeting at the point and the cursor
is gone before anyone clicks. Measured by the user (Nov 2026): quick-cast Skewer
with the cursor on the minimap casts toward that point, and a point past
Skewer's range gives a full-length Skewer rather than walking Magnus toward it.

**Only after Blink.** With `skewer_only_after_blink`, the Skewer conditions
above gain one more: `MagnusState::skewer_follows_blink` must pass. It reads two
things, neither of which blocks anything:

- `MAGNUS_BLINK`, refreshed from every GSI event by `blink_snapshot`: the first
  Blink (`item_blink` or an upgraded one) in inventory slots 0–5, and whether its
  `cooldown` is above 0. Backpack, stash and neutral slot do not count.
- `MAGNUS_BLINK_PRESSED_AT`: the keyboard hook calls `MagnusState::note_key_press`
  on every key press while this option is on, and stamps the time when the key
  matches the binding of the Blink's slot (`[keybindings]`, from the snapshot's
  `slot_keys`). This covers the gap before GSI reports the cooldown.

`blink_gate_open` passes when a Blink is carried **and** (it is on cooldown
**or** its key was pressed within `blink_press_window_ms`). On a fail E reaches
Dota unchanged, a normal Skewer at the cursor, and the log says why (`no Blink in
the inventory` / `Blink not used`). With the option off the gate is never read.

The Skewer goes through the Magnus worker queue, so an E pressed during the
Reverse Polarity combo runs after it, not in the middle of it.

### Standalone trigger

`handle_standalone_trigger()` runs the same combo, so the generic standalone
combo trigger (`AppState.trigger_key`) also fires it, for parity with the other
heroes. Note this path does **not** consult the readiness gate.

## Limitations

- **`camera_center_key` must match your in-game hero-select binding.** The app
  does not read Dota's keybindings; it presses what you configure.

- **The facing right-click is a move order.** Magnus takes a step toward the
  cursor before Reverse Polarity fires. Keep `turn_delay_ms` low.
- **Cursor over the minimap issues a cross-map move order.** Pressing R with the
  mouse parked on the minimap sends Magnus walking. The hook cannot see cursor
  position; this is shared with Shadow Fiend and Snapfire.
- **Cursor over a unit produces an attack order, not a move order.** Facing
  still resolves toward the target, so the pull direction is right.
- **R bypasses Soul Ring while Magnus is active** — the intercept returns before
  the Soul Ring replay path, matching the Shadow Fiend and OD `R` branches.
- **Added latency.** The combo inserts ~50ms plus `turn_delay_ms` in front of
  the cast. On a blink initiation that delay is real; drop the leading settle
  time in `magnus.rs` if it costs you stuns.
- **Skewer to saved point costs ~2 × `anchor_settle_ms` of cursor.** For that
  window your clicks are held (replayed after) and mouse movement is lost — the
  cursor returns to where it was when E was pressed.
- **Only after Blink counts any Blink cooldown.** A Blink disabled by damage
  likely shows a short cooldown in GSI too (not yet measured), which opens the
  gate: an E pressed while Blink is damage-locked goes to the saved point.
  The gate also stays open for Blink's whole cooldown, not just the seconds
  after the Blink.
- **A Blink bound to a mouse button is only seen through GSI.** The press
  record watches key presses; the cooldown still opens the gate a moment later.
- **E bypasses Soul Ring while Skewer-to-point fires**, same as R.

## Logging

Look for `🦏 Magnus` log lines (ultimate and Skewer presses, skipped intercepts
with the reason, anchor saved / not saved, worker start/exit, queue fallback).

---

## Maintenance Checklist

When editing this hero's code, update this doc:

- [ ] New config option added? → Update Configuration table
- [ ] Changed combo sequence? → Update the input sequence
- [ ] Modified the readiness gate? → Update Readiness gate
- [ ] New logging statements? → Update Logging
