# Slark Automation

## Purpose

Directional **Pounce** (W) on a single keypress, GSI-driven **Dark Pact** debuff cleansing, and a low-HP **Shadow Dance** escape with a shard fallback.
**Read this when:** changing the Slark Pounce key, the facing technique, the turn timing, the readiness gate, the Dark Pact cleanse, or the low-HP escape.

## Feature Summary

- The Pounce key (default **W**) is intercepted while Slark is the active hero.
- Combo: `ALT down → right-click (face cursor) → ALT up → wait turn_delay_ms → press W`.
- Pounce leaps along Slark's facing **at cast time**, so facing decides where the leap and the leash land. Turning toward the cursor first aims it.
- **Gated on GSI**: the key is only swallowed when Pounce is levelled and castable. On cooldown it passes straight through, so Slark never takes the facing right-click for nothing.
- Dark Pact, Saltwater Shiv, and Shadow Dance are **not** intercepted.

### Auto Dark Pact

- GSI-driven, not keyboard-driven: runs from `handle_gsi_event`, no key is intercepted.
- Casts Dark Pact when `hero.has_debuff` is true, after a short settle window.
- **GSI exposes a single `has_debuff` bool and never names the modifier**, so this cannot tell a Doom from a Drow slow. It cleanses on anything.
- Held (not dropped) while stunned, hexed, silenced, or while Dark Pact is on cooldown, so the cleanse fires the instant it becomes castable.

### Low HP escape

- GSI-driven. Spends **Shadow Dance** when Slark drops to the HP line while the danger detector is active.
- Falls back to the **Aghanim's Shard** ability only when Shadow Dance is on cooldown.
- Depth Shroud cannot be self-cast, so the fallback presses the key and clicks at the cursor's current position. The mouse is never moved.

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
| `auto_shadow_dance_on_low_hp` | bool | `true` | Spend Shadow Dance to survive. |
| `shadow_dance_key` | char | `"r"` | Shadow Dance ability key. |
| `shadow_dance_hp_threshold_percent` | u32 | `35` | HP line for the escape. |
| `shadow_dance_require_danger` | bool | `true` | Also require the danger detector. |
| `shadow_dance_trigger_cooldown_ms` | u64 | `3000` | Minimum gap between attempts. |
| `shard_fallback_enabled` | bool | `true` | Use the shard when the ultimate is down. |
| `shard_key` | char | `"d"` | Key Depth Shroud sits on. Only the key is configurable; the ability is matched by name. |

```toml
[heroes.slark]
enabled = true
pounce_key = "w"
turn_delay_ms = 200
require_ability_ready = true
auto_dark_pact_on_debuff = true
dark_pact_key = "q"
dark_pact_delay_ms = 300
auto_shadow_dance_on_low_hp = true
shadow_dance_key = "r"
shadow_dance_hp_threshold_percent = 35
shadow_dance_require_danger = true
shadow_dance_trigger_cooldown_ms = 3000
shard_fallback_enabled = true
shard_key = "d"
```

All fifteen fields are exposed in the React UI under **Heroes → Slark**. The shard
fallback additionally depends on `[hud]` — see [HUD Anchors](../features/hud-anchors.md).

## Related Files

| File | Purpose |
|---|---|
| `src/actions/heroes/slark.rs` | Hero script, dedicated worker, `SlarkState::execute_directional_pounce`, readiness gate, `plan_dark_pact` cleanse, `plan_low_hp_escape`. |
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

### Low HP escape

`SlarkScript::low_hp_escape` runs on every `handle_gsi_event`, after the Dark Pact
cleanse and before the shared survivability item checks — the ultimate is worth
more than a salve. `plan_low_hp_escape` picks one of three outcomes:

| Result | When |
|---|---|
| `None` | toggle off, dead, stunned/hexed/silenced, above the HP line, danger required but absent, inside the trigger cooldown, or nothing castable |
| `ShadowDance` | `slark_shadow_dance` levelled and castable — **always preferred** |
| `Shard` | ultimate unavailable, `hero.aghanims_shard` set, and `slark_depth_shroud` castable |

Each escape carries its **own** retry timer. A single shared one meant spending
Shadow Dance also locked out the shard for the same window — precisely when the
fallback is wanted, since the ultimate being down is its entire trigger. A
debounced ultimate therefore escalates to the shard rather than stalling.

With `shadow_dance_require_danger` on (the default) this needs the danger detector
*and* the HP line, matching the Outworld Destroyer barrier. `in_danger` already
means "losing HP or lost a burst of it", so the pair reads as "low **and** actually
under fire" — sitting at 30% in the fountain never spends the ultimate. Turn it off
and the HP line alone fires it, including while limping home.

The order is the point: Shadow Dance is the stronger escape, so the shard is only
ever reached once the ultimate has been ruled out.

### Shard fallback: cast at the cursor

Dota will not self-cast Depth Shroud — no double-tap, no ALT modifier — so pressing
the key only *arms* it and a click is what resolves the cast.

Clicking the HUD hero portrait would target Slark himself, and that was the original
approach, driven by a calibrated [HUD anchor](../features/hud-anchors.md). **It does
not work in practice**: with the anchor calibrated correctly the synthetic click on
the portrait does not resolve the targeting, so the cast never happened.

So the fallback presses the key and clicks **wherever the cursor already is**,
without moving the mouse. Mid-fight the cursor is normally pointed somewhere useful,
which is good enough for a defensive shroud, and it removes the calibration
dependency entirely.

The trade: the shroud lands at the cursor, not centred on Slark. If the cursor is
parked on the minimap when it fires, the cast goes there.

`shard_key` is only the key we **press**. Readiness is checked by matching
`slark_depth_shroud` by name across every ability slot, and that distinction is not
academic:

> **GSI slot order is ability order, not key order.** On a shard Slark the payload
> reads `0=slark_dark_pact, 1=slark_pounce, 2=slark_saltwater_shiv,
> 3=slark_depth_shroud, 4=slark_shadow_dance, 5=plus_high_five`. The shard ability
> is inserted *ahead of* the ultimate, so Shadow Dance — the R ability — is at
> index 4, and the `D` ability is at index 3.

An earlier version derived the slot from the key (`d` → index 4) and so read the
ultimate's cooldown instead of Depth Shroud's. That is false exactly when the
fallback is wanted, which is why it never fired. `tests/fixtures/slark_event.json`
is a real captured payload specifically so this layout is pinned by tests.

Readiness matters only to avoid moving the mouse for nothing: if Depth Shroud is on
cooldown there is no point parking the cursor on the portrait and clicking.

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
- **The shard fallback clicks wherever your cursor is.** It does not move the
  mouse, but it does issue a left click, so the shroud lands at the cursor rather
  than on Slark — and a cursor parked on the minimap sends the cast across the map.
  `shard_fallback_enabled` turns just this off and leaves Shadow Dance working.
- **The escape cannot see what is killing you.** It reads HP and the danger
  detector, not incoming damage type or whether Shadow Dance would actually save
  you. It will spend the ultimate on a losing fight.
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
- [ ] Changed the escape ordering or gating? → Update the Low HP escape table
- [ ] New logging statements? → Update Logging
