# Snapfire Automation

## Purpose

Directional **Firesnap Cookie** (W) leap on a single keypress.
**Read this when:** changing the Snapfire trigger key, the facing technique, or the leap timing.

## Feature Summary

- Trigger key (default **Space**) is intercepted while Snapfire is the active hero.
- Combo: `ALT down → right-click (face cursor) → wait turn_delay_ms → press W (self-cast) → ALT up`.
- ALT is held across the right-click and the W press, so the same modifier faces the hero and self-casts the cookie, making her leap toward the cursor.
- **W is not intercepted** — manual ally cookies still work normally.
- No GSI cooldown gating; the combo always fires when Snapfire is active.

## Configuration

`config/config.toml` under `[heroes.snapfire]`:

| Field | Type | Default | Purpose |
|---|---|---|---|
| `enabled` | bool | `true` | Master toggle for the intercept. |
| `trigger_key` | string | `"Space"` | Key intercepted to start the combo. |
| `cookie_key` | char | `"w"` | Firesnap Cookie ability key, self-cast via ALT. |
| `turn_delay_ms` | u64 | `60` | Delay after the facing right-click before the self-cast leap. |

```toml
[heroes.snapfire]
enabled = true
trigger_key = "Space"
cookie_key = "w"
turn_delay_ms = 60
```

## Related Files

| File | Purpose |
|---|---|
| `src/actions/heroes/snapfire.rs` | Hero script, dedicated worker, `SnapfireState::execute_cookie_leap`. |
| `src/input/keyboard.rs` | Trigger-key interception branch + `SnapfireKeyboardSnapshot`. |
| `src/config/settings.rs` | `SnapfireConfig` + defaults. |
| `config/config.toml` | `[heroes.snapfire]` block. |

## Activation

`snapfire_enabled` is derived in `KeyboardSnapshot::from_runtime` from
`selected_hero == Some(HeroType::Snapfire)`, which is set by GSI hero detection
(`AppState::update_from_gsi`) or the manual-override selection. No dedicated
`Arc<Mutex<bool>>` flag is used.

## Details

### Input sequence

1. The keyboard hook (`src/input/keyboard.rs`) reads the cached `KeyboardSnapshot`.
2. If `snapfire_enabled && snapfire.enabled` and the pressed key equals the
   parsed `trigger_key`, the hook **blocks** the original key and calls
   `SnapfireState::execute_cookie_leap(cookie_key, turn_delay_ms)`.
3. The dedicated Snapfire worker runs:
   `alt_down() → mouse_click() → sleep(turn_delay_ms) → press_key(cookie_key) → alt_up()`,
   using the `src/input/simulation.rs` helpers.

`SIMULATING_KEYS` guards the synthetic right-click and W press so they are not
re-intercepted by the global hook.

### Standalone trigger

`handle_standalone_trigger()` runs the same cookie leap, so the generic
standalone combo trigger (`AppState.trigger_key`) also fires the combo for
parity with the other heroes.

## Limitations

- Space is also tracked as the auto-items modifier (`MODIFIER_KEY_HELD`) and
  used by Broodmother's Space + right-click. The intercept is gated on Snapfire
  being active, so only one hero's Space behavior is live at a time.
- If `turn_delay_ms` is too low the leap may fire before Snapfire finishes
  turning — increase it to taste.

## Logging

Look for `🍪 Snapfire` log lines (trigger press, worker start/exit, queue
fallback).
