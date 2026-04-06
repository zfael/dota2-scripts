# Huskar Automation

## Purpose

Learn how the Huskar script plugs into the shared armlet module and automates Berserker Blood cleansing for survival optimization.

**Read this when:** configuring Huskar automation, tuning shared or Huskar-specific armlet thresholds, or understanding debuff-cleanse logic.

## Feature Summary

- **Shared armlet toggle automation** - Uses the repo-wide armlet module with Huskar-specific override values
- **Roshan Burning Spears gate** - Can suppress Burning Spears near the Armlet Roshan trigger band and restore it after HP recovers
- **Berserker Blood debuff cleanse** - Activates Berserker Blood to cleanse debuffs after a configurable delay
- **GSI-based detection** - Auto-enables when `npc_dota_hero_huskar` is active
- **Survivability actions** - Auto-uses healing, defensive, and neutral items
- **No standalone trigger** - Combo key is still not implemented

## Configuration

Huskar armlet behavior uses shared defaults from `[armlet]` plus optional Huskar-specific overrides in `[heroes.huskar.armlet]`:

```toml
[armlet]
cast_modifier = "Alt"
toggle_threshold = 320
predictive_offset = 30
toggle_cooldown_ms = 250

[heroes.huskar]
berserker_blood_key = "e"
berserker_blood_delay_ms = 300
standalone_key = "Home"

[heroes.huskar.armlet]
toggle_threshold = 120
predictive_offset = 150
toggle_cooldown_ms = 300

[heroes.huskar.roshan_spears]
enabled = false
burning_spear_key = "w"
disable_buffer_hp = 60
reenable_buffer_hp = 100
```

The checked-in `config/config.toml` values below override the Rust defaults from `src/config/settings.rs`.

| Option | Type | `config.toml` | Rust default | Description |
|---|---|---|---|---|
| `[armlet].cast_modifier` | string | `"Alt"` | `"Alt"` | Shared cast-side modifier paired with the quick-cast slot key |
| `[armlet].toggle_threshold` | u32 | `320` | `320` | Shared base HP threshold before hero overrides |
| `[armlet].predictive_offset` | u32 | `30` | `30` | Shared predictive HP buffer |
| `[armlet].toggle_cooldown_ms` | u64 | `250` | `250` | Shared cooldown between toggle attempts |
| `[heroes.huskar.armlet].toggle_threshold` | u32 | `120` | inherits `[armlet]` | Huskar override for base threshold |
| `[heroes.huskar.armlet].predictive_offset` | u32 | `150` | inherits `[armlet]` | Huskar override for predictive buffer |
| `[heroes.huskar.armlet].toggle_cooldown_ms` | u64 | `300` | inherits `[armlet]` | Huskar override for cooldown |
| `[heroes.huskar.roshan_spears].enabled` | bool | `false` | `false` | Master switch for Huskar's Roshan-only Burning Spears gate |
| `[heroes.huskar.roshan_spears].burning_spear_key` | char | `'w'` | `'w'` | Burning Spears key paired with `Alt` to toggle autocast |
| `[heroes.huskar.roshan_spears].disable_buffer_hp` | u32 | `60` | `60` | Extra HP above the effective Armlet trigger where Spears is turned off |
| `[heroes.huskar.roshan_spears].reenable_buffer_hp` | u32 | `100` | `100` | Extra HP above the effective Armlet trigger where Spears may turn back on |
| `berserker_blood_key` | char | `'e'` | `'e'` | Key to press for Berserker Blood |
| `berserker_blood_delay_ms` | u64 | `300` | `300` | Delay before activating cleanse |
| `standalone_key` | string | `"Home"` | `"Home"` | Reserved for future standalone combo |

Legacy flat Huskar keys (`armlet_toggle_threshold`, `armlet_predictive_offset`, `armlet_toggle_cooldown_ms`) are still read when the nested `[heroes.huskar.armlet]` block is absent, so older local configs keep their Huskar tuning.

## Related Files

| File | Purpose |
|---|---|
| `src/actions/heroes/huskar.rs` | Huskar Berserker Blood cleanse plus shared armlet-survivability wiring |
| `src/actions/armlet.rs` | Shared armlet planning, config resolution, cooldown tracking, and dual-trigger execution |
| `src/actions/common.rs` | Shared survivability helpers that enqueue armlet checks |
| `src/config/settings.rs` | Shared `[armlet]` config plus Huskar-specific override fields |
| `config/config.toml` | User configuration |

---

## Details

### Armlet Toggle Automation

Armlet of Mordiggian is a toggle item that drains HP continuously but provides large offensive stats. Huskar benefits from low HP because of his passive, so armlet toggling remains a key survival mechanic.

Huskar no longer owns a separate armlet implementation. On each GSI event, `src/actions/dispatcher.rs` runs the shared Armlet check inline before Huskar-specific survivability or cleanse logic, and `src/actions/armlet.rs` handles:

1. resolving shared `[armlet]` defaults
2. applying Huskar overrides from `[heroes.huskar.armlet]`
3. checking HP, stun state, cooldown, and equipped Armlet
4. emitting the dual-trigger toggle sequence

Because the dispatcher now runs Armlet first and `src/input/simulation.rs` gives Armlet chords their own fast lane, Huskar Armlet toggles can jump ahead of queued normal synthetic inputs after the current atomic input command finishes.

#### Dual-trigger sequence

The shared armlet module toggles by pressing:

1. the quick-cast slot key from `[keybindings]`
2. the same slot key again while holding `[armlet].cast_modifier`

With the checked-in config and Armlet in `slot1`, the sequence is:

- `x`
- `Alt + x`

#### Predictive Offset

The effective trigger line is:

`toggle_threshold + predictive_offset`

With the checked-in Huskar override:

- shared `toggle_threshold = 320`
- Huskar override `toggle_threshold = 120`
- shared `predictive_offset = 30`
- Huskar override `predictive_offset = 150`
- effective trigger = `270 HP`

When Huskar's HP drops below 270, the shared armlet module becomes eligible to toggle.

#### Roshan Burning Spears gate

When shared Armlet Roshan mode is armed and `[heroes.huskar.roshan_spears].enabled = true`, Huskar also manages Burning Spears autocast in a small band above the effective Armlet trigger:

- disable line = `effective_armlet_trigger + disable_buffer_hp`
- re-enable line = `effective_armlet_trigger + reenable_buffer_hp`

With the checked-in values:

- effective Armlet trigger = `270 HP`
- Burning Spears disable line = `330 HP`
- Burning Spears re-enable line = `370 HP`

That means Huskar can stop self-damage from Burning Spears before drifting into the Roshan Armlet save window, then restore Spears only after HP has clearly recovered.

#### Cooldown and critical retry

The shared `toggle_cooldown_ms` prevents rapid retriggers. Huskar overrides it to `300ms` in the checked-in config.

If a toggle fires at extremely low HP, the shared armlet module also arms a critical retry marker. If later GSI updates show HP still critically low or even lower, the module forces one more dual-trigger attempt to recover from a likely failed or missed toggle.

### Berserker Blood Debuff Cleanse

Berserker Blood (E) is still Huskar-specific and remains in `src/actions/heroes/huskar.rs`.

#### Trigger Conditions

All conditions must be met:

1. hero is alive
2. hero currently has a debuff
3. the Berserker Blood ability is present in `ability0`-`ability3`
4. the ability is castable, leveled, and off cooldown
5. the configured delay has elapsed since the first debuff detection

#### Delay Timer Logic

When a debuff is first detected, the script starts a timer. If the debuff persists for the configured delay, Berserker Blood is activated. This lets Huskar wait briefly for stacked debuffs instead of cleansing the very first one immediately.

State tracking:

- `BERSERKER_BLOOD_DEBUFF_DETECTED` stores the first debuff timestamp
- when debuffs disappear, the tracker resets
- once the delay elapses, the ability is activated and the tracker resets

### Survivability Actions

Huskar still uses the shared `SurvivabilityActions` system for:

- healing items
- defensive items
- neutral items
- danger detection updates

See `docs/features/survivability.md` for the shared pipeline details.

### Standalone Trigger

The `standalone_key` config option still exists, but the Huskar script currently logs:

```text
Huskar standalone trigger not implemented
```

### Usage

1. Equip Armlet of Mordiggian in-game.
2. Level Berserker Blood.
3. Tune `[armlet]` and `[heroes.huskar.armlet]` to your preference.
4. Run the app and let GSI detect Huskar.
5. Confirm armlet toggles when HP drops below the effective threshold.
6. If Roshan mode is armed and the Roshan Spears gate is enabled, confirm Burning Spears turns off near the disable line and returns after HP recovery.
7. Confirm Berserker Blood cleanses debuffs after the configured delay.

### Logging

With `level = "info"`, you'll see messages like:

```text
Triggering armlet toggle (HP: 250 < trigger: 270, base: 120, cooldown: 300ms)
Entering Roshan Burning Spears danger band at 330HP (disable line: 330)
Disabling Burning Spears due to Roshan threshold protection
Re-enabling Burning Spears after HP recovery
Debuff detected, starting 300ms timer for Berserker Blood
Activating Berserker Blood to cleanse debuffs (300ms delay elapsed)
```

With `level = "debug"`:

```text
Armlet toggle on cooldown (125ms remaining)
Berserker Blood not ready: can_cast=true, level=4, cooldown=5.2
Waiting for more debuffs... (150ms elapsed)
```
