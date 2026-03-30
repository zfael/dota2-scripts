# Meepo Automation

## Purpose

Learn how the Meepo script executes a standalone burst combo, runs manual farm assist, and auto-casts Dig or MegaMeepo when in danger.  
**Read this when:** configuring Meepo automation, tuning defensive-cast thresholds, understanding combo/farm sequencing, or debugging Dig / MegaMeepo / farm assist not firing.

## Phase-1 / Phase-2 / Phase-3A Feature Summary

- **Standalone combo trigger** – Press configured key to execute Blink → combo items → Earthbind → Poof
- **GSI-driven Dig (Petrify)** – Auto-casts Dig when in danger with Aghanim's Shard below HP threshold
- **GSI-driven MegaMeepo** – Auto-casts MegaMeepo when in danger with Aghanim's Scepter below HP threshold
- **Manual farm assist** – Toggle a cursor-directed Poof farming pulse that suspends on danger or invalid cast conditions
- **Survivability actions** – Auto-use healing/defensive/neutral items on every GSI event
- **GSI-based detection** – Auto-enables when `npc_dota_hero_meepo` detected
- **Observed-state cache** – Maintains a read-only Meepo snapshot for UI/debugging without inventing clone telemetry

## Configuration

Meepo uses checked-in settings from `[heroes.meepo]` plus an optional `[heroes.meepo.armlet]` override block inherited from the shared armlet system:

```toml
[heroes.meepo]
# Key to trigger standalone combo (Blink → items → Earthbind → Poof)
standalone_key = "Home"
# Ability keys
earthbind_key = 'q'
poof_key = 'w'
dig_key = 'd'
megameepo_key = 'f'
# Delay after Blink before casting combo items (ms)
post_blink_delay_ms = 80
# Combo items to use before Earthbind (partial name match, must be castable)
combo_items = ["sheepstick", "disperser"]
# How many times to press each combo item
combo_item_spam_count = 1
# Delay between combo item presses (ms)
combo_item_delay_ms = 40
# How many times to press Earthbind (Q)
earthbind_press_count = 2
# Delay between Earthbind presses (ms)
earthbind_press_interval_ms = 30
# How many times to press Poof (W)
poof_press_count = 3
# Delay between Poof presses (ms)
poof_press_interval_ms = 35
# Auto-cast Dig when in danger (requires Aghanim's Shard)
auto_dig_on_danger = true
# HP% threshold below which auto-Dig fires
dig_hp_threshold_percent = 32
# Auto-cast MegaMeepo when in danger (requires Aghanim's Scepter)
auto_megameepo_on_danger = true
# HP% threshold below which auto-MegaMeepo fires
megameepo_hp_threshold_percent = 45
# Cooldown between defensive-cast triggers (ms)
defensive_trigger_cooldown_ms = 1500

[heroes.meepo.farm_assist]
enabled = true
toggle_key = "End"
pulse_interval_ms = 700
minimum_mana_percent = 35
minimum_health_percent = 45
right_click_after_poof = true
suspend_on_danger = true
suspend_after_manual_combo_ms = 2500
poof_press_count = 1
poof_press_interval_ms = 35

[heroes.meepo.armlet]
# Optional per-hero overrides for shared armlet automation
# enabled = true
# toggle_threshold = 250
# predictive_offset = 20
# toggle_cooldown_ms = 300
```

| Option | Type | Checked-in value | Rust default | Description |
|--------|------|------------------|--------------|-------------|
| `standalone_key` | string | `"Home"` | `"Home"` | Key to trigger standalone combo |
| `earthbind_key` | char | `'q'` | `'q'` | Earthbind ability key |
| `poof_key` | char | `'w'` | `'w'` | Poof ability key |
| `dig_key` | char | `'d'` | `'d'` | Dig (Petrify) ability key |
| `megameepo_key` | char | `'f'` | `'f'` | MegaMeepo ability key |
| `post_blink_delay_ms` | u64 | `80` | `80` | Delay after Blink before combo items (ms) |
| `combo_items` | list | `["sheepstick", "disperser"]` | `["sheepstick", "disperser"]` | Item name fragments used before Earthbind |
| `combo_item_spam_count` | u32 | `1` | `1` | Number of presses per combo item |
| `combo_item_delay_ms` | u64 | `40` | `40` | Delay between combo item presses (ms) |
| `earthbind_press_count` | u32 | `2` | `2` | Number of Earthbind presses |
| `earthbind_press_interval_ms` | u64 | `30` | `30` | Delay between Earthbind presses (ms) |
| `poof_press_count` | u32 | `3` | `3` | Number of Poof presses |
| `poof_press_interval_ms` | u64 | `35` | `35` | Delay between Poof presses (ms) |
| `auto_dig_on_danger` | bool | `true` | `true` | Enable auto-Dig on danger |
| `dig_hp_threshold_percent` | u32 | `32` | `32` | HP% ceiling for auto-Dig |
| `auto_megameepo_on_danger` | bool | `true` | `true` | Enable auto-MegaMeepo on danger |
| `megameepo_hp_threshold_percent` | u32 | `45` | `45` | HP% ceiling for auto-MegaMeepo |
| `defensive_trigger_cooldown_ms` | u64 | `1500` | `1500` | Minimum gap between any two defensive casts (ms) |
| `[heroes.meepo.farm_assist].enabled` | bool | `true` | `true` | Master switch for manual farm assist |
| `[heroes.meepo.farm_assist].toggle_key` | string | `"End"` | `"End"` | Hotkey that arms/disarms farm assist |
| `[heroes.meepo.farm_assist].pulse_interval_ms` | u64 | `700` | `700` | Minimum delay between farm pulses |
| `[heroes.meepo.farm_assist].minimum_mana_percent` | u32 | `35` | `35` | Mana% floor for farm pulses |
| `[heroes.meepo.farm_assist].minimum_health_percent` | u32 | `45` | `45` | HP% floor for farm pulses |
| `[heroes.meepo.farm_assist].right_click_after_poof` | bool | `true` | `true` | Whether a right-click follows Poof |
| `[heroes.meepo.farm_assist].suspend_on_danger` | bool | `true` | `true` | Suspend farm assist when `in_danger` |
| `[heroes.meepo.farm_assist].suspend_after_manual_combo_ms` | u64 | `2500` | `2500` | Cooldown after manual combo before rearming |
| `[heroes.meepo.farm_assist].poof_press_count` | u32 | `1` | `1` | Number of Poof presses per farm pulse |
| `[heroes.meepo.farm_assist].poof_press_interval_ms` | u64 | `35` | `35` | Delay between Poof presses in one pulse |
| `[heroes.meepo.armlet].enabled` | bool | unset | inherits `[armlet]` | Optional per-hero on/off override for shared armlet automation |
| `[heroes.meepo.armlet].toggle_threshold` | u32 | unset | inherits `[armlet]` | Optional Meepo-specific Armlet HP threshold override |
| `[heroes.meepo.armlet].predictive_offset` | u32 | unset | inherits `[armlet]` | Optional Meepo-specific predictive buffer override |
| `[heroes.meepo.armlet].toggle_cooldown_ms` | u64 | unset | inherits `[armlet]` | Optional Meepo-specific Armlet cooldown override |

The checked-in `config/config.toml` values currently match the Rust defaults for `[heroes.meepo]` and `[heroes.meepo.farm_assist]`. The nested `[heroes.meepo.armlet]` block remains optional and is omitted from the checked-in config unless you want Meepo-specific Armlet tuning.

## Related Files

| File | Purpose |
|------|---------|
| `src/actions/heroes/meepo_macro.rs` | Meepo farm-assist macro state, gating, and pulse planning |
| `src/actions/heroes/meepo.rs` | Meepo script, combo execution, defensive trigger logic |
| `src/actions/heroes/meepo_state.rs` | Read-only Meepo observed-state derivation and cache |
| `src/config/settings.rs` | `MeepoConfig` struct and defaults |
| `config/config.toml` | User configuration |
| `src/gsi/handler.rs` | Handler-owned cache refresh and stale-state clearing |
| `docs/features/survivability.md` | Shared healing, dispel, neutral-item behavior |
| `docs/features/danger-detection.md` | How `in_danger` is computed |

---

## Details

### ⚡ Standalone Combo Sequence

Press the standalone key (default: `Home`) to execute the full Meepo burst combo.

The standalone trigger reads the latest cached GSI event at execution time and runs `execute_combo()` synchronously on the calling thread.

**Requirements:**
- At least one GSI event received (for item slot detection)
- If no GSI event yet, logs a warning and does nothing

**Combo sequence:**

1. **Blink Dagger** (if present in inventory and castable)
   - Slot looked up via `find_item_slot()`
   - Single press at cursor position
   - `post_blink_delay_ms` (80ms) delay

2. **Combo items** (Sheepstick, Disperser by default)
   - Each item is looked up by partial name in all inventory slots
   - Item must have `can_cast == true`
   - Pressed `combo_item_spam_count` (1) time per item, `combo_item_delay_ms` (40ms) between presses
   - Missing or non-castable items are logged as warnings and skipped

3. **Earthbind (Q)**
   - Pressed `earthbind_press_count` (2) times
   - `earthbind_press_interval_ms` (30ms) between presses

4. **Poof (W)**
   - Pressed `poof_press_count` (3) times
   - `poof_press_interval_ms` (35ms) between presses

### 🛡️ GSI-Driven Defensive Casts

On every GSI event, Meepo checks whether to auto-cast **Dig** or **MegaMeepo** as a defensive response to danger. Only one of the two fires per evaluation (Dig is checked first).

A shared `defensive_trigger_cooldown_ms` (1500ms) timer prevents both from spam-firing back-to-back.

#### Auto-Dig (Petrify)

Fires when **all** of the following are true:

| Condition | Source |
|-----------|--------|
| `auto_dig_on_danger = true` | config |
| `in_danger == true` | `danger_detector::update()` |
| Hero is alive, not stunned, not silenced | `hero.alive`, `hero.stunned`, `hero.silenced` |
| Hero has Aghanim's Shard | `hero.aghanims_shard` |
| `hero.health_percent <= dig_hp_threshold_percent` (32%) | GSI |
| `meepo_petrify` is learned and `can_cast` | `abilities` scan |
| No defensive cast within `defensive_trigger_cooldown_ms` | internal timer |

→ Presses `dig_key` ('d').

#### Auto-MegaMeepo

Fires when **all** of the following are true (evaluated only if Dig did not fire):

| Condition | Source |
|-----------|--------|
| `auto_megameepo_on_danger = true` | config |
| `in_danger == true` | `danger_detector::update()` |
| Hero is alive, not stunned, not silenced | `hero.alive`, `hero.stunned`, `hero.silenced` |
| Hero has Aghanim's Scepter | `hero.aghanims_scepter` |
| `hero.health_percent <= megameepo_hp_threshold_percent` (45%) | GSI |
| `meepo_megameepo` is learned and `can_cast` | `abilities` scan |
| No defensive cast within `defensive_trigger_cooldown_ms` | internal timer |

→ Presses `megameepo_key` ('f').

#### Ability Readiness Check

Both checks use `ability_is_ready(event, ability_name)`, which scans ability slots 0–5:
- `ability.name == ability_name`
- `ability.level > 0` (ability must be learned)
- `ability.can_cast == true`

### 🛡️ Survivability Actions

Meepo uses the common `SurvivabilityActions` system on every GSI event:
- **Healing items** – Magic Wand, Faerie Fire, Satanic, etc.
- **Defensive items** – BKB, Lotus Orb, Blade Mail when in danger
- **Neutral items** – Witchbane, Safety Bubble, etc.
- **Danger detection** – Monitors HP changes and enemy abilities

These run independently of the standalone combo and defensive-cast logic. Adjust behavior via the global `[common]`, `[danger_detection]`, and `[neutral_items]` config sections.

### 🌾 Manual Farm Assist

Phase 3A adds a manual-armed farm assist. Press the configured toggle key (default: `End`) to arm or disarm it.

When armed, each Meepo GSI event evaluates a conservative farm pulse:

1. Meepo must be alive, not stunned, and not silenced
2. `Poof` must be learned and castable
3. HP% and mana% must stay above the configured farm-assist thresholds
4. If `suspend_on_danger = true`, `in_danger` immediately suspends the mode
5. The pulse interval must have elapsed

If all gates pass, the script:

- presses `poof_key` using the farm-assist Poof count/interval
- optionally right-clicks at the current cursor position

This mode is **cursor-directed**, not route-directed. You still choose where Meepo is looking or moving by controlling the mouse/camera. The helper only reduces repetitive Poof-and-right-click input.

### 🔄 Execution Flow

```
Every GSI event
        │
        ├─→ refresh Meepo observed state cache
        │       └─ clone state remains "unavailable" until GSI exposes per-clone telemetry
        │
        ├─→ SurvivabilityActions (healing, defensive, neutral items)
        │
        ├─→ maybe_trigger_defensive_cast()
        │       │
        │       ├─ should_cast_dig() all conditions met? → press dig_key
        │       └─ else should_cast_megameepo() all conditions met? → press megameepo_key
        │
        └─→ maybe_run_farm_assist()
                └─ if armed + safe + interval elapsed → Poof pulse (+ optional right-click)

User presses standalone key (Home)
        │
        └─→ execute_combo()
                │
                ├─ Blink Dagger (if present) → wait post_blink_delay_ms
                ├─ combo_items (each, if castable)
                ├─ Earthbind (Q) × earthbind_press_count
                └─ Poof (W) × poof_press_count

User presses farm-assist key (End)
        │
        └─→ toggle farm assist
                ├─ Inactive → Armed (if a Meepo snapshot is available)
                └─ Armed/Suspended → Inactive
```

### 👀 Observed State

Phase 2 adds a read-only `MeepoObservedState` snapshot that is refreshed from the latest Meepo GSI event and exposed in the UI.

It currently tracks:
- hero name / level / HP% / mana%
- current `in_danger` flag
- stun / silence state
- `Poof` readiness
- Dig / MegaMeepo readiness
- Aghanim's Shard / Scepter presence
- Blink key and currently castable combo item keys
- clone state as an explicit `Unavailable` enum value

This layer is intentionally conservative. It does **not** guess clone count, clone HP, clone inventory ownership, or which clone should cast an active item.

### 🔒 Thread Safety

`MeepoScript` wraps two pieces of mutable state:
- `latest_event: Mutex<Option<GsiWebhookEvent>>` – updated on every GSI event, read on standalone trigger
- `last_defensive_trigger: Mutex<Option<Instant>>` – guards the defensive-cast cooldown

Phase 2 also adds a process-wide `MeepoObservedState` cache guarded by a `Mutex<Option<...>>`. The GSI handler keeps this cache fresh for Meepo events and clears it when the active hero changes away from Meepo; the Meepo script refreshes it again after danger evaluation so the UI sees the latest `in_danger` result.

Phase 3A adds a second shared cache for farm-assist status (`Inactive`, `Armed`, or suspended with a reason). That cache is updated by the farm-assist toggle path, by hero-change handling, and by GSI-driven safety gates.

All locks are held briefly and released before any `press_key` calls, so there is no lock contention with `ActionExecutor`.

### Usage

1. **Pick Meepo** in-game (auto-detected via GSI as `npc_dota_hero_meepo`)
2. **Equip Blink Dagger** (optional; skipped if absent)
3. **Equip Sheepstick / Disperser** (or override `combo_items`)
4. **Level Earthbind (Q) and Poof (W)**
5. **Get Aghanim's Shard** for auto-Dig; **Aghanim's Scepter** for auto-MegaMeepo
6. **Position cursor** on enemy or desired location
7. **Press standalone key** (default: Home) to burst
8. **Dig / MegaMeepo fire automatically** when danger conditions are met in-game
9. **Press farm-assist key** (default: End) to arm/disarm manual farming pulses

### Tuning

- **Dig threshold**: Lower `dig_hp_threshold_percent` to only Dig when critically low
- **MegaMeepo threshold**: Raise `megameepo_hp_threshold_percent` to cast earlier in a fight
- **Defensive cooldown**: Decrease `defensive_trigger_cooldown_ms` for faster re-triggering
- **Combo timing**: Increase `post_blink_delay_ms` if combo items cast before the hero arrives
- **Poof count**: Increase `poof_press_count` if Poof misses on targets at the edge of range
- **Farm pulse interval**: Raise `farm_assist.pulse_interval_ms` if pulses feel too spammy
- **Farm safety floors**: Raise `farm_assist.minimum_health_percent` / `minimum_mana_percent` for a more conservative helper

### Logging

With `level = "info"`, you'll see:
```
Executing Meepo combo sequence...
Using Blink (x)
Using combo item 'sheepstick' (d)
Casting Earthbind (q)
Casting Poof (w)
Auto-casting Meepo Dig (d)
Auto-casting Meepo MegaMeepo (f)
Meepo farm assist armed
Executing Meepo farm-assist pulse
```

With `level = "warn"`, missing items appear as:
```
Combo item 'sheepstick' not found or not castable
No GSI event received yet - Meepo combo needs item data
Meepo farm assist needs a fresh Meepo GSI snapshot before arming
```

### Limitations

- **No clone-position awareness** – The script operates only on the primary hero's GSI state. It has no visibility into clone positions, clone HP, or multi-hero Poof targeting.
- **Observed clone state is intentionally unavailable** – The UI/debug snapshot shows clone state as unavailable until the GSI model exposes explicit per-clone telemetry.
- **No autonomous route planner** – Farm assist is manual and cursor-directed; it does not choose camps, routes, or targets for you.
- **No objective automation** – No Tormentor, Roshan, or tower-planning helpers yet.
- **No keyboard interception** – Meepo does not intercept or replay any keyboard key. The standalone combo and defensive casts all originate from within the script, never from hooked input.
- **Manual keybind sync** – The script does not discover in-game bindings automatically; if you change Earthbind/Poof/Dig/MegaMeepo keys in Dota, mirror them in `[heroes.meepo]`.
- **No cooldown pre-check on combo** – The combo does not verify that Earthbind or Poof are off cooldown before pressing; it relies on ability spam.
- **Cursor-targeted combo** – Blink and abilities aim at cursor position; manual aim is required before pressing the standalone key.

---

## Maintenance Checklist

When editing this hero's code, update this doc:

- [ ] New config option added? → Update Configuration table
- [ ] New behavior/feature? → Add section under Details
- [ ] Changed combo sequence? → Update sequence description and flow diagram
- [ ] Modified trigger conditions? → Update trigger condition tables
- [ ] Changed state tracking? → Update Thread Safety section
- [ ] New logging statements? → Update Logging section
