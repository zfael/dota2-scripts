# Mirana Automation

## Purpose

Directional **Leap** (E) on a single keypress.
**Read this when:** changing the Mirana Leap key, the facing technique, the turn timing, or the readiness gate.

## Feature Summary

- The Leap key (default **E**) is intercepted while Mirana is the active hero.
- Combo: `ALT down → right-click (face cursor) → ALT up → wait turn_delay_ms → press E`.
- Leap jumps along Mirana's facing **at cast time**, so facing decides where she lands. Turning toward the cursor first aims it.
- **Gated on GSI**: the key is only swallowed when `mirana_leap` is levelled and castable. On cooldown it passes straight through, so Mirana never takes the facing right-click for nothing.
- Sacred Arrow, Starstorm, and Moonlight Shadow are **not** intercepted.

Same shape as [Slark's directional Pounce](slark.md), which is the reference
implementation — Pounce and Leap are the same kind of ability (no target, leaps
along facing), so the two combos are identical apart from names and defaults.

It is deliberately **not** shaped like Snapfire's cookie: that one holds ALT
across the ability press because Firesnap Cookie is a self-cast. Leap takes no
target, and ALT held over an ability key pings it to allies instead of casting
it, so ALT is released before the press.

## Configuration

`config/config.toml` under `[heroes.mirana]`:

| Field | Type | Default | Purpose |
|---|---|---|---|
| `enabled` | bool | `true` | Master toggle for the intercept. |
| `leap_key` | char | `"e"` | Leap ability key; also the key intercepted. |
| `turn_delay_ms` | u64 | `200` | Delay after the facing right-click before the Leap cast. |
| `require_ability_ready` | bool | `true` | Pass the key through when Leap is unlevelled or on cooldown. |

```toml
[heroes.mirana]
enabled = true
leap_key = "e"
turn_delay_ms = 200
require_ability_ready = true
```

`turn_delay_ms` starts at Slark's `200`, not the `60` the Magnus and Snapfire
combos use. 60ms is enough for a facing that only has to be roughly right; a leap
that fires before the turn finishes lands somewhere else entirely.

All four fields are exposed in the React UI under **Heroes → Mirana**.

## Related Files

| File | Purpose |
|---|---|
| `src/actions/heroes/mirana.rs` | Hero script, dedicated worker, `MiranaState::execute_directional_leap`, readiness gate. |
| `src/input/keyboard.rs` | Leap-key interception branch + `MiranaKeyboardSnapshot`. |
| `src/config/settings.rs` | `MiranaConfig` + defaults. |
| `config/config.toml` | `[heroes.mirana]` block. |
| `src-ui/src/components/heroes/configs/MiranaConfig.tsx` | UI config panel. |
| `tests/fixtures/mirana_event.json` | GSI fixture backing the readiness tests. |

## Activation

`mirana_enabled` is derived in `KeyboardSnapshot::from_runtime` from
`selected_hero == Some(HeroType::Mirana)`, which is set by GSI hero detection
(`AppState::update_from_gsi`) or the manual-override selection. No dedicated
`Arc<Mutex<bool>>` flag is used — same model as Snapfire, Magnus, and Slark.

## Details

### Input sequence

1. The keyboard hook (`src/input/keyboard.rs`) reads the cached `KeyboardSnapshot`.
2. If `mirana_enabled && mirana.enabled` and the pressed key equals the parsed
   `leap_key`, the hook checks `MiranaState::can_intercept_leap()`
   (skipped when `require_ability_ready = false`).
3. On a pass, the hook **blocks** the original key and calls
   `MiranaState::execute_directional_leap(leap_char, turn_delay_ms)`.
   On a fail it falls through and the key reaches Dota unchanged.
4. The dedicated Mirana worker runs:
   `alt_down() → mouse_click() → alt_up() → sleep(turn_delay_ms) → press_key(leap_char)`,
   using the `src/input/simulation.rs` helpers.

`SIMULATING_KEYS` guards the synthetic right-click and E press so they are not
re-intercepted by the global hook.

### Readiness gate

`MiranaState::can_intercept_leap()` reads `MIRANA_LAST_EVENT`, refreshed on every
`handle_gsi_event`, and requires `mirana_leap` to have `level > 0 && can_cast`.
It returns `false` when no GSI event has arrived yet, so before the first payload
E behaves normally.

`ability_is_ready` scans **every slot and matches by name**, never by the index
the key suggests. GSI slot order is ability order, and shard-, scepter- and
innate-granted abilities are inserted ahead of the ultimate, so a key-derived
index reads the wrong ability. Slark's shard fallback shipped broken for exactly
that reason — see [slark.md](slark.md#shard-fallback-cast-at-the-cursor).

### Standalone trigger

`handle_standalone_trigger()` runs the same combo, so the generic standalone
combo trigger (`AppState.trigger_key`) also fires it, for parity with the other
heroes. Note this path does **not** consult the readiness gate.

## Limitations

- **The readiness gate has not been validated against a live charge-based
  ability.** Leap has charges, and how GSI reports `can_cast` with a charge
  banked but the refresh timer running is not covered by any captured payload.
  If `can_cast` reads `false` while a charge is available, the intercept silently
  never fires and E just passes through. Symptom: `🌙 Mirana leap intercept
  skipped: Leap not ready` in the log while Leap is visibly castable. Workaround:
  set `require_ability_ready = false`.
- **`tests/fixtures/mirana_event.json` is handcrafted, not captured.** Its slot
  ordering is a plausible guess. That is safe for the tests it backs — they
  assert name-matching, which is independent of slot order — but replace it with
  a real payload when one is available.
- **The facing right-click is a move order.** Mirana walks toward the cursor for
  the whole `turn_delay_ms` before Leap fires. At the 200ms default that is a
  visible step.
- **Cursor over the minimap issues a cross-map move order.** Pressing E with the
  mouse parked on the minimap sends Mirana walking. The hook cannot see cursor
  position; this is shared with Shadow Fiend, Snapfire, Magnus, and Slark.
- **Cursor over a unit produces an attack order, not a move order.** Facing still
  resolves toward the target, so the leap direction is right.
- **E bypasses Soul Ring while Mirana is active** — the intercept returns before
  the Soul Ring replay path, matching the Shadow Fiend, Magnus, OD, and Slark
  branches.
- **Added latency.** The combo inserts ~100ms plus `turn_delay_ms` in front of
  the cast. On an escape Leap that delay is real; drop the leading settle time in
  `mirana.rs` if it costs you deaths.
- **`leap_key` must match your in-game binding.** The app does not read Dota's
  keybindings; it presses what you configure.
- **No Moonlight Shadow or Sacred Arrow automation.** Moonlight Shadow is a team
  ultimate whose value depends on where four other players are, which GSI does
  not expose; and the hook cannot see targeting mode, so it cannot tell an armed
  arrow from an idle one.

## Logging

Look for `🌙 Mirana` log lines (leap press, skipped intercepts with the reason,
worker start/exit, queue fallback).

---

## Maintenance Checklist

When editing this hero's code, update this doc:

- [ ] New config option added? → Update Configuration table
- [ ] Changed combo sequence? → Update the input sequence
- [ ] Modified the readiness gate? → Update Readiness gate
- [ ] Captured a real GSI payload? → Drop the fixture caveat under Limitations
- [ ] New logging statements? → Update Logging
