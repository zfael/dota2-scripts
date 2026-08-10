# Magnus Automation

## Purpose

Directional **Reverse Polarity** (R) on a single keypress.
**Read this when:** changing the Magnus ultimate key, the facing technique, the turn timing, or the readiness gate.

## Feature Summary

- The ultimate key (default **R**) is intercepted while Magnus is the active hero.
- Combo: `ALT down → right-click (face cursor) → ALT up → wait turn_delay_ms → press R → optional camera recentre`.
- Reverse Polarity drags enemies to the arc **in front of Magnus**, so facing at cast time decides where they land. Turning toward the cursor first lines the pull up with the Skewer that follows.
- **Gated on GSI**: the key is only swallowed when Reverse Polarity is levelled and castable. On cooldown it passes straight through, so Magnus never takes the facing right-click for nothing.
- Skewer is **not** intercepted — you aim and cast it manually after the pull.

## Configuration

`config/config.toml` under `[heroes.magnus]`:

| Field | Type | Default | Purpose |
|---|---|---|---|
| `enabled` | bool | `true` | Master toggle for the intercept. |
| `ultimate_key` | char | `"r"` | Reverse Polarity ability key; also the key intercepted. |
| `turn_delay_ms` | u64 | `60` | Delay after the facing right-click before the ultimate cast. |
| `require_ability_ready` | bool | `true` | Pass the key through when Reverse Polarity is unlevelled or on cooldown. |
| `center_camera_on_ultimate` | bool | `true` | Double-tap the hero-select key after the cast to recentre the camera. |
| `camera_center_key` | string | `"1"` | Hero-select key to double-tap. Character (`"1"`) or named key (`"F1"`). |
| `camera_center_delay_ms` | u64 | `60` | Delay between the cast and the first camera tap. |

```toml
[heroes.magnus]
enabled = true
ultimate_key = "r"
turn_delay_ms = 60
require_ability_ready = true
center_camera_on_ultimate = true
camera_center_key = "1"
camera_center_delay_ms = 60
```

All seven fields are exposed in the React UI under **Heroes → Magnus**.

## Related Files

| File | Purpose |
|---|---|
| `src/actions/heroes/magnus.rs` | Hero script, dedicated worker, `MagnusState::execute_directional_ultimate`, readiness gate. |
| `src/input/keyboard.rs` | Ultimate-key interception branch + `MagnusKeyboardSnapshot`. |
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

## Logging

Look for `🦏 Magnus` log lines (ultimate press, skipped intercepts with the
reason, worker start/exit, queue fallback).

---

## Maintenance Checklist

When editing this hero's code, update this doc:

- [ ] New config option added? → Update Configuration table
- [ ] Changed combo sequence? → Update the input sequence
- [ ] Modified the readiness gate? → Update Readiness gate
- [ ] New logging statements? → Update Logging
