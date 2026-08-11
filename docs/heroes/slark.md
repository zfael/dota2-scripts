# Slark Automation

## Purpose

Directional **Pounce** (W) on a single keypress, plus GSI-driven **Dark Pact** debuff cleansing.
**Read this when:** changing the Slark Pounce key, the facing technique, the turn timing, the readiness gate, or the Dark Pact cleanse.

## Feature Summary

- The Pounce key (default **W**) is intercepted while Slark is the active hero.
- Combo: `ALT down → right-click (face cursor) → ALT up → wait turn_delay_ms → press W`.
- Pounce leaps along Slark's facing **at cast time**, so facing decides where the leap and the leash land. Turning toward the cursor first aims it.
- **Gated on GSI**: the key is only swallowed when Pounce is levelled and castable. On cooldown it passes straight through, so Slark never takes the facing right-click for nothing.
- Dark Pact, Essence Shift, and Shadow Dance are **not** intercepted.

### Auto Dark Pact

- GSI-driven, not keyboard-driven: runs from `handle_gsi_event`, no key is intercepted.
- Casts Dark Pact when `hero.has_debuff` is true, after a short settle window.
- **GSI exposes a single `has_debuff` bool and never names the modifier**, so this cannot tell a Doom from a Drow slow. It cleanses on anything.
- Held (not dropped) while stunned, hexed, silenced, or while Dark Pact is on cooldown, so the cleanse fires the instant it becomes castable.

## Configuration

`config/config.toml` under `[heroes.slark]`:

| Field | Type | Default | Purpose |
|---|---|---|---|
| `enabled` | bool | `true` | Master toggle for the intercept. |
| `pounce_key` | char | `"w"` | Pounce ability key; also the key intercepted. |
| `turn_delay_ms` | u64 | `200` | Delay after the facing right-click before the Pounce cast. Tuned in-game; Slark needs noticeably more settle time than the 60ms the other facing combos use. |
| `require_ability_ready` | bool | `true` | Pass the key through when Pounce is unlevelled or on cooldown. |
| `auto_dark_pact_on_debuff` | bool | `true` | Cast Dark Pact when GSI reports a debuff. |
| `dark_pact_key` | char | `"q"` | Dark Pact ability key, pressed by the cleanse. |
| `dark_pact_delay_ms` | u64 | `300` | Settle window after the first debuff before casting. |

```toml
[heroes.slark]
enabled = true
pounce_key = "w"
turn_delay_ms = 200
require_ability_ready = true
auto_dark_pact_on_debuff = true
dark_pact_key = "q"
dark_pact_delay_ms = 300
```

All seven fields are exposed in the React UI under **Heroes → Slark**.

## Related Files

| File | Purpose |
|---|---|
| `src/actions/heroes/slark.rs` | Hero script, dedicated worker, `SlarkState::execute_directional_pounce`, readiness gate, `plan_dark_pact` cleanse. |
| `src/input/keyboard.rs` | Pounce-key interception branch + `SlarkKeyboardSnapshot`. |
| `src/config/settings.rs` | `SlarkConfig` + defaults. |
| `config/config.toml` | `[heroes.slark]` block. |
| `src-ui/src/components/heroes/configs/SlarkConfig.tsx` | UI config panel. |
| `tests/fixtures/slark_event.json` | GSI fixture backing the readiness tests. |

## Activation

`slark_enabled` is derived in `KeyboardSnapshot::from_runtime` from
`selected_hero == Some(HeroType::Slark)`, which is set by GSI hero detection
(`AppState::update_from_gsi`) or the manual-override selection. No dedicated
`Arc<Mutex<bool>>` flag is used — same model as Snapfire and Magnus.

## Details

### Input sequence

1. The keyboard hook (`src/input/keyboard.rs`) reads the cached `KeyboardSnapshot`.
2. If `slark_enabled && slark.enabled` and the pressed key equals the parsed
   `pounce_key`, the hook checks `SlarkState::can_intercept_pounce()`
   (skipped when `require_ability_ready = false`).
3. On a pass, the hook **blocks** the original key and calls
   `SlarkState::execute_directional_pounce(pounce_char, turn_delay_ms)`.
   On a fail it falls through and the key reaches Dota unchanged.
4. The dedicated Slark worker runs:
   `alt_down() → mouse_click() → alt_up() → sleep(turn_delay_ms) → press_key(pounce_char)`,
   using the `src/input/simulation.rs` helpers.

`SIMULATING_KEYS` guards the synthetic right-click and W press so they are not
re-intercepted by the global hook.

ALT is held only across the facing right-click, then released before the ability
press. Pounce takes no target, and ALT over an ability key pings it to allies
rather than casting it — the same reason Magnus releases early, and the opposite
of Snapfire's cookie, which needs the modifier for the self-cast.

### Readiness gate

`SlarkState::can_intercept_pounce()` reads `SLARK_LAST_EVENT`, refreshed on
every `handle_gsi_event`, and requires `slark_pounce` to have
`level > 0 && can_cast`. It returns `false` when no GSI event has arrived yet,
so before the first payload W behaves normally.

### Dark Pact cleanse

`SlarkScript::dark_pact_cleanse` runs on every `handle_gsi_event`, before the
shared survivability checks. The gating lives in `plan_dark_pact(event, enabled)`,
which returns one of three decisions:

| Decision | When | Effect on the settle window |
|---|---|---|
| `Idle` | toggle off, dead, or `has_debuff = false` | dropped |
| `Hold` | stunned / hexed / silenced, or `slark_dark_pact` unlevelled or on cooldown | kept running |
| `Arm` | debuffed and castable | started, or spent once `dark_pact_delay_ms` has elapsed |

`Hold` is the interesting case. Dark Pact cannot be cast through a stun or a
silence, but whatever else is on Slark is still worth shedding the moment the
lock lifts — so the window keeps running rather than restarting. The same is
true on cooldown: a debuff that lands during the cooldown fires the cleanse
immediately when Dark Pact comes back, because the elapsed time is already past
the settle window.

The timer itself (`SLARK_DEBUFF_DETECTED`) is taken with `try_lock`. A contended
tick is skipped rather than blocking the GSI handler; the next payload is 0.1s
away.

Same shape as Huskar's Berserker's Blood cleanse, which reads the same
`has_debuff` flag.

### Standalone trigger

`handle_standalone_trigger()` runs the same combo, so the generic standalone
combo trigger (`AppState.trigger_key`) also fires it, for parity with the other
heroes. Note this path does **not** consult the readiness gate.

## Limitations

- **The facing right-click is a move order.** Slark walks toward the cursor for
  the whole `turn_delay_ms` before Pounce fires. At the 200ms default that is a
  visible step — the cost of giving Slark enough time to finish turning. Lower
  it only if the leap still lands on target.
- **Cursor over the minimap issues a cross-map move order.** Pressing W with the
  mouse parked on the minimap sends Slark walking. The hook cannot see cursor
  position; this is shared with Shadow Fiend, Snapfire, and Magnus.
- **Cursor over a unit produces an attack order, not a move order.** Facing
  still resolves toward the target, so the leap direction is right.
- **W bypasses Soul Ring while Slark is active** — the intercept returns before
  the Soul Ring replay path, matching the Shadow Fiend, Magnus, and OD branches.
- **Added latency.** The combo inserts ~100ms plus `turn_delay_ms` in front of
  the cast. On an escape Pounce that delay is real; drop the leading settle time
  in `slark.rs` if it costs you kills.
- **The cleanse cannot tell debuffs apart.** `hero.has_debuff` is one bool for
  every modifier in the game, so Dark Pact gets spent on a passing slow just as
  readily as on a real threat. There is no GSI field that would fix this — if it
  costs you too much farm or too many cooldowns, turn the toggle off.
- **Dark Pact is a basic dispel and it pulses after ~1.75s.** It never removes
  stuns, and the automation reacting instantly does not make the dispel land any
  sooner than the ability itself allows.
- **`dark_pact_key` must match your in-game binding.** The app does not read
  Dota's keybindings; it presses what you configure.
- **Shadow Dance invisibility is not tracked.** `src/actions/invisibility.rs`
  only infers invisibility from Shadow Blade and Silver Edge cooldown edges, so
  item automation does not hold off during Slark's own ultimate.

## Logging

Look for `🐟 Slark` log lines (pounce press, skipped intercepts with the
reason, worker start/exit, queue fallback). The Dark Pact cast logs at `info`;
the settle-window transitions log at `debug`.

---

## Maintenance Checklist

When editing this hero's code, update this doc:

- [ ] New config option added? → Update Configuration table
- [ ] Changed combo sequence? → Update the input sequence
- [ ] Modified the readiness gate? → Update Readiness gate
- [ ] Changed the Dark Pact decisions? → Update the Dark Pact cleanse table
- [ ] New logging statements? → Update Logging
