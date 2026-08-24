# Earth Spirit Automation

## Purpose

Earth Spirit's two signature plays each cost two keys and both open with the
same one. This remaps each play onto its *second* key, so one press does both.
**Read this when:** changing the Earth Spirit keys, either combo's timing, the
roll double-tap, or the readiness gates.

## Feature Summary

- The grip key (default **E**) is remapped to `Stone Remnant → Geomagnetic Grip`.
  That is Earth Spirit's silence: he has no silence button, only a grip that
  silences everything the remnant passes through on its way back to him.
- The roll key (default **W**) is remapped to `Rolling Boulder → Stone Remnant`.
  A roll through a remnant travels **1600 units instead of 800** and moves much
  faster, so the good roll is always the two-key one.
- **The two combos order the same pair of abilities opposite ways**, on purpose.
  The grip resolves on the press, so its remnant has to already be standing. The
  roll has a ~600ms windup, and a remnant dropped into the path during it still
  counts — so the roll goes first and the windup becomes an aiming window.
- Both are intercepted only while Earth Spirit is the active hero.
- **Gated on GSI**: each key is only swallowed when *its own* ability is levelled
  and castable. On cooldown it passes straight through, so a dead press never
  spends a remnant charge.
- Boulder Smash and Magnetize are **not** intercepted.

Same two-press worker shape as [Ember Spirit's remnant chase](ember_spirit.md),
and the same readiness-gate shape as [Mirana's Leap](mirana.md). It is
deliberately *not* a facing combo — nothing here right-clicks, because both
Earth Spirit abilities take a point and neither cares where he is facing.

## Configuration

`config/config.toml` under `[heroes.earth_spirit]`:

| Field | Type | Default | Purpose |
|---|---|---|---|
| `enabled` | bool | `true` | Master toggle for both combos. |
| `remnant_key` | char | `"d"` | Stone Remnant key, pressed first by both combos. |
| `silence_combo_enabled` | bool | `true` | Remap the grip key. |
| `grip_key` | char | `"e"` | Geomagnetic Grip key; also the key intercepted. |
| `silence_remnant_delay_ms` | u64 | `120` | Gap between the remnant press and the grip press. |
| `require_grip_ready` | bool | `true` | Pass the grip key through when Grip is unlevelled or on cooldown. |
| `roll_combo_enabled` | bool | `true` | Remap the roll key. |
| `roll_key` | char | `"w"` | Rolling Boulder key; also the key intercepted. |
| `roll_double_tap` | bool | `true` | Press the roll key twice, for a roll key left off quickcast. |
| `roll_double_tap_delay_ms` | u64 | `60` | Gap between the two roll presses. Ignored when the toggle is off. |
| `roll_to_remnant_delay_ms` | u64 | `300` | Aiming window: gap between the roll firing and the remnant press. Measured from the press that casts the roll, so from the second tap when double-tap is on. |
| `require_roll_ready` | bool | `true` | Pass the roll key through when Rolling Boulder is unlevelled or on cooldown. |

```toml
[heroes.earth_spirit]
enabled = true
remnant_key = "d"
silence_combo_enabled = true
grip_key = "e"
silence_remnant_delay_ms = 120
require_grip_ready = true
roll_combo_enabled = true
roll_key = "w"
roll_double_tap = true
roll_double_tap_delay_ms = 60
roll_to_remnant_delay_ms = 300
require_roll_ready = true
```

Every field is exposed in the React UI under **Heroes → Earth Spirit**.

### Why the two combos are ordered differently

The silence has to place its remnant first: Geomagnetic Grip resolves on the
press, so there is nothing to grip unless the remnant is already standing. Both
halves land on one cursor position, and `silence_remnant_delay_ms` is only the
window the server needs to register the remnant — 120ms, sized for a machine.

The roll is the other way round. Rolling Boulder has a **~600ms windup** before
Earth Spirit actually starts moving, and a remnant dropped into the path during
that windup still counts — this is the documented technique, not a trick:

> "Use Rolling Boulder in the desired direction, and then place the Stone
> Remnant in Earth Spirit's path during the delay, instead of placing the
> Remnant and then casting Rolling Boulder."
> — [Dota 2 Wiki, Earth Spirit guide](https://dota2.fandom.com/wiki/Earth_Spirit/Guide)

Casting the roll first locks in the direction, which frees the windup for
aiming. `roll_to_remnant_delay_ms` is that aiming window, and it is sized for a
**person** — 300ms, enough to flick the cursor to where the boulder will pass.
That is why it is longer than the silence delay rather than shorter: the two
numbers are measuring completely different things.

The ceiling is the windup. `roll_to_remnant_delay_ms + roll_double_tap_delay_ms`
must stay under ~600ms (`ROLLING_BOULDER_WINDUP_MS` in `settings.rs`, asserted
by a unit test). Past it the boulder is already rolling and a remnant placed
behind it does nothing — which looks exactly like the combo firing and having no
effect.

## Related Files

| File | Purpose |
|---|---|
| `src/actions/heroes/earth_spirit.rs` | Hero script, dedicated worker, `EarthSpiritState` combo entry points, readiness gates. |
| `src/input/keyboard.rs` | Grip- and roll-key interception branches + `EarthSpiritKeyboardSnapshot`. |
| `src/config/settings.rs` | `EarthSpiritConfig` + defaults. |
| `config/config.toml` | `[heroes.earth_spirit]` block. |
| `src-ui/src/components/heroes/configs/EarthSpiritConfig.tsx` | UI config panel. |
| `tests/fixtures/earth_spirit_event.json` | GSI fixture backing the readiness tests. |

## Activation

`earth_spirit_enabled` is derived in `KeyboardSnapshot::from_runtime` from
`selected_hero == Some(HeroType::EarthSpirit)`, which is set by GSI hero
detection (`AppState::update_from_gsi`) or the manual-override selection. No
dedicated `Arc<Mutex<bool>>` flag is used — same model as Mirana, Snapfire,
Magnus, and Slark.

## Details

### Ability reference

| Ability | Internal name | Default key |
|---|---|---|
| Boulder Smash | `earth_spirit_boulder_smash` | Q |
| Rolling Boulder | `earth_spirit_rolling_boulder` | W |
| Geomagnetic Grip | `earth_spirit_geomagnetic_grip` | E |
| Stone Remnant | `earth_spirit_stone_caller` | D |
| Magnetize | `earth_spirit_magnetize` | R |
| Enchant Remnant (scepter) | `earth_spirit_petrify` | F |

### Input sequences

**Silence combo**, on the grip key:

1. The keyboard hook reads the cached `KeyboardSnapshot`.
2. If `earth_spirit_enabled && earth_spirit.enabled &&
   earth_spirit.silence_combo_enabled` and the pressed key equals the parsed
   `grip_key`, the hook checks `EarthSpiritState::can_intercept_grip()`
   (skipped when `require_grip_ready = false`).
3. On a pass the hook **blocks** the original key and calls
   `EarthSpiritState::execute_silence_combo(...)`. On a fail it falls through and
   the key reaches Dota unchanged.
4. The dedicated worker runs
   `press_key(remnant) → sleep(silence_remnant_delay_ms) → press_key(grip)`.

**Roll combo**, on the roll key — same gate against
`can_intercept_roll()`, then:

```
press_key(roll) → sleep(roll_double_tap_delay_ms) → press_key(roll)
                → sleep(roll_to_remnant_delay_ms) → press_key(remnant)
```

The second roll press and its sleep are skipped entirely when
`roll_double_tap = false`; nothing stray is sent, and the aiming window is then
measured from the single press.

A configured delay of `0` skips its sleep rather than sleeping for nothing.
`SIMULATING_KEYS` guards every synthetic press so the global hook does not
re-intercept them.

### Readiness gates

`can_intercept_grip()` and `can_intercept_roll()` read
`EARTH_SPIRIT_LAST_EVENT`, refreshed on every `handle_gsi_event`, and require
`earth_spirit_geomagnetic_grip` / `earth_spirit_rolling_boulder` to have
`level > 0 && can_cast`. Both return `false` when no GSI event has arrived yet,
so before the first payload of a game the keys behave normally.

`ability_is_ready` scans **every slot and matches by name**, never by the index
the key suggests. GSI slot order is ability order: Earth Spirit carries Stone
Remnant as its own entry ahead of the ultimate, and a scepter inserts Enchant
Remnant on top of that, so a key-derived index reads the wrong ability. Slark's
shard fallback shipped broken for exactly that reason — see
[slark.md](slark.md#shard-fallback-cast-at-the-cursor).

**Neither gate reads Stone Remnant.** It is charge-based, and GSI's `can_cast`
is unreliable for charge abilities — the same trap documented for Mirana's Leap.
Gating on it would leave both combos dead while remnants are visibly banked.

### Standalone trigger

`handle_standalone_trigger()` runs the silence combo, so the generic standalone
combo trigger (`AppState.trigger_key`, default `Home`) also fires it, for parity
with the other heroes. That path checks `enabled` and `silence_combo_enabled`
but does **not** consult the readiness gate.

## Limitations

- **Quickcast must be on for Stone Remnant and Geomagnetic Grip.** Both are
  point-target: without quickcast the first press only arms the cursor and the
  second cancels the targeting instead of resolving it. The roll key is the
  exception — `roll_double_tap` exists precisely because it is commonly left on
  normal cast.
- **`roll_double_tap` has not been validated against every client setup.** If
  Dota treats the second tap as a cancel rather than a cast, the roll will not
  fire; turn the toggle off and the combo sends a single press. Record which
  setting works here once it has been confirmed in a live game.
- **Aiming stays manual, and the silence aims backwards from where you might
  expect.** Grip pulls the remnant *toward* Earth Spirit, so the silence lands on
  whoever stands between the cursor and him. Aim past the target, not at it.
- **The roll's aiming window is open-loop.** Nothing checks that the cursor
  actually moved before the remnant is placed; the delay just elapses and the
  remnant lands wherever the cursor is by then. Leave the cursor still and the
  remnant drops on the roll's origin, which usually misses the path entirely.
- **`ROLLING_BOULDER_WINDUP_MS` is a documented figure, not a measured one.** If
  the windup turns out to be shorter than 600ms in the live client, the shipped
  300ms window may already run past it; the symptom is a roll that never
  extends. Lower `roll_to_remnant_delay_ms` and update the constant.
- **The readiness gate cannot see remnant charges.** With zero charges banked the
  combo still fires: Dota ignores the remnant press and the grip reaches for
  whatever is already on the map, or nothing. This is the deliberate trade
  against a gate that goes dead while charges are available.
- **`tests/fixtures/earth_spirit_event.json` is handcrafted, not captured.** Its
  slot ordering is a plausible guess. That is safe for the tests it backs — they
  assert name-matching, which is independent of slot order — but replace it with
  a real payload when one is available.
- **E and W bypass Soul Ring while Earth Spirit is active.** The intercepts
  return before the Soul Ring replay path, matching the Shadow Fiend, Magnus,
  Mirana, OD, and Slark branches. Grip costs 75 mana and Rolling Boulder 50, so
  both would otherwise qualify. The synthetic Stone Remnant press is a non-issue:
  it costs 0 mana, and Soul Ring no longer triggers on free abilities.
- **Added latency.** Each combo inserts its remnant delay in front of the cast.
  On an escape roll that delay is real.
- **The keys must match your in-game bindings.** The app does not read Dota's
  keybindings; it presses what you configure.
- **No Boulder Smash or Magnetize automation.** A smashed remnant travels *away*
  from Earth Spirit, so a fixed cursor rule misfires often enough to be worse
  than casting it by hand; and Magnetize's value depends on where enemies are
  standing relative to remnants, which GSI does not expose.

## Logging

Look for `🗿 Earth Spirit` log lines (grip/roll presses, skipped intercepts with
the reason, worker start/exit, queue fallback).

---

## Maintenance Checklist

When editing this hero's code, update this doc:

- [ ] New config option added? → Update Configuration table
- [ ] Changed either combo sequence? → Update the input sequences
- [ ] Modified a readiness gate? → Update Readiness gates
- [ ] Confirmed the double-tap in a live game? → Record the answer under Limitations
- [ ] Measured Rolling Boulder's real windup? → Update `ROLLING_BOULDER_WINDUP_MS` and drop that caveat
- [ ] Captured a real GSI payload? → Drop the fixture caveat under Limitations
- [ ] New logging statements? → Update Logging
