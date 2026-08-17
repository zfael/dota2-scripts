# Ember Spirit Automation

## Purpose

Two independent features: a hotkey **remnant chase**, and a GSI-driven **Flame Guard auto-cast**.
**Read this when:** changing the Ember Spirit combo keys, the delay between the two presses, the trigger path, or the Flame Guard thresholds.

## Feature Summary

### Remnant chase (hotkey)

- Fired by the **global standalone combo trigger key** (`AppState.trigger_key`, default `Home`) while Ember Spirit is the active hero. There is no per-key interception.
- Combo: `press remnant_key → wait activate_delay_ms → press activate_key`.
- Fire Remnant (R) places a remnant at the cursor; Activate Fire Remnant (D) dashes to every remnant on the map. Pairing them turns a two-key chase into one.
- **No readiness gate.** Unlike the facing combos (Magnus, Mirana, Slark), a wasted press costs nothing here: Dota ignores a Fire Remnant press with no charges banked, and the activate that follows is still worth sending because it dashes to remnants already on the map.

### Flame Guard auto-cast (GSI)

- Casts Flame Guard when the shared danger detector is tripped **and** `hero.health_percent` is at or below `flame_guard_hp_threshold_percent`.
- Both conditions are required. Danger alone trips on any rapid HP drop, including at full health, which is too eager for a 20–35s cooldown.
- Gated on Ember being alive and able to act (not stunned, silenced, or hexed) and on `ember_spirit_flame_guard` being levelled and `can_cast`.
- Rate-limited by `flame_guard_trigger_cooldown_ms` so one burst of payloads cannot spam the key.
- **Independent of `enabled`**, which only gates the remnant chase.

Shared survivability (healing, defensive items, neutral items) runs on every GSI
event, as with every other hero.

## Configuration

`config/config.toml` under `[heroes.ember_spirit]`:

| Field | Type | Default | Purpose |
|---|---|---|---|
| `enabled` | bool | `true` | Master toggle for the remnant chase. Does **not** affect Flame Guard. |
| `remnant_key` | char | `"r"` | Fire Remnant ability key, pressed first. |
| `activate_key` | char | `"d"` | Activate Fire Remnant key, pressed second. |
| `activate_delay_ms` | u64 | `150` | Delay between the two presses. |
| `auto_flame_guard_on_danger` | bool | `true` | Master toggle for the Flame Guard auto-cast. |
| `flame_guard_key` | char | `"e"` | Flame Guard ability key pressed by the auto-cast. |
| `flame_guard_hp_threshold_percent` | u32 | `65` | Only auto-cast at or below this health percentage. |
| `flame_guard_trigger_cooldown_ms` | u64 | `2000` | Minimum gap between two auto-casts. |

```toml
[heroes.ember_spirit]
enabled = true
remnant_key = "r"
activate_key = "d"
activate_delay_ms = 150
auto_flame_guard_on_danger = true
flame_guard_key = "e"
flame_guard_hp_threshold_percent = 65
flame_guard_trigger_cooldown_ms = 2000
```

All eight fields are exposed in the React UI under **Heroes → Ember Spirit**.

## Related Files

| File | Purpose |
|---|---|
| `src/actions/heroes/ember_spirit.rs` | Hero script, dedicated worker, `EmberSpiritState::execute_remnant_chase`, `should_trigger_flame_guard`. |
| `src/actions/danger_detector.rs` | Supplies the `in_danger` flag the Flame Guard gate reads. |
| `src/config/settings.rs` | `EmberSpiritConfig` + defaults. |
| `config/config.toml` | `[heroes.ember_spirit]` block. |
| `src-ui/src/components/heroes/configs/EmberSpiritConfig.tsx` | UI config panel. |
| `tests/fixtures/ember_spirit_event.json` | GSI fixture backing the readiness and threshold tests. **Handcrafted**, not captured from a live game — the slot ordering is a plausible guess. Safe for what it backs, since the readiness check matches by name. |

## Activation

`HeroType::EmberSpirit` is set by GSI hero detection (`AppState::update_from_gsi`)
or the manual-override selection. The `ComboTrigger` hero-name match in
`src/main.rs` and `src-tauri/src/lib.rs` routes the trigger to
`dispatch_standalone_trigger("npc_dota_hero_ember_spirit")`. No dedicated
`Arc<Mutex<bool>>` flag and no keyboard-snapshot entry — the hook never
intercepts a key for this hero.

## Details

### Input sequence

1. The keyboard hook matches the pressed key against `KeyboardSnapshot.trigger_key` and sends `HotkeyEvent::ComboTrigger`.
2. The hotkey loop checks `standalone_enabled` and `selected_hero`, then calls `dispatch_standalone_trigger(...)`.
3. `EmberSpiritScript::handle_standalone_trigger()` reads the config, returns early when `enabled = false`, and enqueues the request.
4. The dedicated Ember Spirit worker runs `press_key(remnant_key) → sleep(activate_delay_ms) → press_key(activate_key)` via the `src/input/simulation.rs` helpers.

The enqueue is non-blocking, so the hotkey thread returns immediately and the
sleep happens on the worker. `SIMULATING_KEYS` guards both synthetic presses so
they are not re-intercepted by the global hook.

### Why the delay exists

The remnant has to exist server-side before the activate can pick it up. Press
the activate too early and Ember dashes to the *previous* remnants only, while
the new one is still being placed. `150` is the shipped starting point; tune it
against your latency.

### Flame Guard gate

`should_trigger_flame_guard(...)` runs on every GSI payload inside
`handle_gsi_event`, before the shared survivability calls, and returns `false` on
the first condition that fails:

1. `auto_flame_guard_on_danger` is off, or the danger detector is not tripped
2. Ember is dead, stunned, silenced, or hexed
3. `health_percent` is above `flame_guard_hp_threshold_percent`
4. `ember_spirit_flame_guard` is unlevelled or not `can_cast`
5. the previous auto-cast was less than `flame_guard_trigger_cooldown_ms` ago

On a pass it stamps `LAST_FLAME_GUARD_TRIGGER` and enqueues the keypress on the
shared executor, so the GSI handler thread is never blocked.

There is **no mana floor**. `can_cast` already encodes affordability, and unlike
OD's Objurgation there is nothing to reserve mana for — Flame Guard is the
cheapest thing Ember can do when he is the one being focused.

There is also no "is the shield already up" check, because GSI exposes no
modifiers. It is not needed: Flame Guard's cooldown outlasts its own duration,
so `can_cast == true` already implies the shield is not running. This is the one
assumption that would break if Valve ever made the cooldown shorter than the
duration.

The ability is matched **by name across all six slots**, never by the slot its
key implies. Ember carries Activate Fire Remnant as its own GSI entry, so slot
indices are already shifted relative to keys in a real payload.

## Limitations

- **Quickcast must be on for Fire Remnant.** R is point-target: without
  quickcast the press only arms the cursor, and the no-target activate that
  follows cancels the targeting instead of resolving it. The app does not read
  Dota's keybindings or cast settings, so it cannot detect this — the symptom is
  a dash with no new remnant placed.
- **The remnant lands wherever the cursor is.** The combo does not move the
  mouse, so the chase goes where you were already pointing. Cursor parked on the
  minimap places the remnant across the map.
- **Configured keys must match your in-game bindings.** The app presses what you
  configure, not what Dota is bound to.
- **No readiness gate.** With no charges banked the first press is inert and the
  combo still fires the activate. That is deliberate (see Feature Summary), but
  it does mean the log line does not tell you whether a remnant was actually
  placed.
- **`activate_delay_ms` is latency-sensitive.** A value tuned on a local server
  can be too short on a distant one.
- **The Flame Guard auto-cast cannot tell a gank from a creep pull.** The danger
  detector is a pure HP-rate heuristic with no vision of who is hitting you, so
  taking a hard creep camp at low HP can spend the cooldown. Lower
  `flame_guard_hp_threshold_percent` if that costs you fights.
- **GSI arrives on Dota's push cadence, not continuously.** The auto-cast reacts
  a payload late, so against genuine burst it is a mitigation, not a save.
- **Flame Guard is cast wherever Ember is.** It is a self-buff with no targeting,
  so unlike the remnant chase the cursor is irrelevant.

## Logging

Look for `🔥 Ember Spirit` log lines (combo triggered, skipped when disabled,
Flame Guard auto-cast with the HP percentage that triggered it, worker
start/exit, queue fallback).

---

## Maintenance Checklist

When editing this hero's code, update this doc:

- [ ] New config option added? → Update Configuration table
- [ ] Changed combo sequence? → Update the input sequence
- [ ] Changed the Flame Guard conditions? → Update Flame Guard gate
- [ ] Added a readiness gate to the chase? → Update Feature Summary and Limitations
- [ ] New logging statements? → Update Logging
