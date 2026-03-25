# Huskar Automation

## Purpose

Learn how the Huskar script automates armlet toggling and Berserker Blood cleansing for survival optimization.  
**Read this when:** configuring Huskar automation, tuning armlet thresholds and predictive offset, understanding debuff-cleanse logic.

## Feature Summary

- **Armlet toggle automation** – Auto-toggles Armlet of Mordiggian at low HP with predictive offset
- **Berserker Blood debuff cleanse** – Activates Berserker Blood to cleanse debuffs with configurable delay
- **GSI-based detection** – Auto-enables when `npc_dota_hero_huskar` detected
- **Survivability actions** – Auto-use healing/defensive items
- **No standalone trigger** – Combo key not implemented

## Configuration

All settings in `config/config.toml` under `[heroes.huskar]`:

```toml
[heroes.huskar]
# Armlet toggle threshold: activate when HP < this value (raw HP, not percent)
armlet_toggle_threshold = 120
# Predictive offset: additional HP to predict damage and toggle earlier
armlet_predictive_offset = 150
# Cooldown between armlet toggles to prevent spam (ms)
armlet_toggle_cooldown_ms = 300
# Key to press for Berserker Blood (E ability)
berserker_blood_key = "e"
# Delay to wait for additional debuffs before cleansing (ms)
berserker_blood_delay_ms = 300
# Standalone combo key (currently not implemented)
standalone_key = "Home"
```

The checked-in `config/config.toml` values below override the Rust defaults from `src/config/settings.rs`.

| Option | Type | `config.toml` | Rust default | Description |
|--------|------|---------------|--------------|-------------|
| `armlet_toggle_threshold` | u32 | `120` | `320` | Raw HP threshold for armlet toggle |
| `armlet_predictive_offset` | u32 | `150` | `30` | Additional HP buffer to predict incoming damage |
| `armlet_toggle_cooldown_ms` | u64 | `300` | `250` | Cooldown between toggles to prevent spam |
| `berserker_blood_key` | char | `'e'` | `'e'` | Key to press for Berserker Blood |
| `berserker_blood_delay_ms` | u64 | `300` | `300` | Delay before activating cleanse (wait for multiple debuffs) |
| `standalone_key` | string | `"Home"` | `"Home"` | Reserved for future standalone combo |

## Related Files

| File | Purpose |
|------|---------|
| `src/actions/heroes/huskar.rs` | Huskar script implementation |
| `src/actions/common.rs` | `armlet_toggle()` shared function and `ArmletConfig` |
| `src/config/settings.rs` | `HuskarConfig` struct and defaults |
| `config/config.toml` | User configuration |

---

## Details

### ⚔️ Armlet Toggle Automation

Armlet of Mordiggian is a toggle item that drains HP continuously but provides massive damage and attack speed. Huskar benefits from low HP due to his passive, making armlet toggling at critical HP a core mechanic.

The automation runs in a **separate guarded thread** on every GSI event:

1. **Calculate effective threshold**: `armlet_toggle_threshold + armlet_predictive_offset`
2. **Check if hero HP is below threshold**
3. **Toggle armlet** if off cooldown (respects `armlet_toggle_cooldown_ms`)
4. **Prevent race conditions** via `ARMLET_THREAD_GUARD` mutex (only one toggle thread runs at a time)

#### Predictive Offset

The `armlet_predictive_offset` adds a safety buffer to account for incoming damage. For example:

- `armlet_toggle_threshold = 120`
- `armlet_predictive_offset = 150`
- **Effective threshold = 270 HP**

When your HP drops below 270, the script toggles armlet to survive burst damage.

#### Cooldown Guard

The `armlet_toggle_cooldown_ms` prevents rapid toggling that could waste time or cause death. After a toggle, the script waits this duration before allowing another toggle.

### 🩸 Berserker Blood Debuff Cleanse

Berserker Blood (E) is an active ability that can cleanse debuffs by activating the skill. The automation detects debuffs and activates Berserker Blood after a configurable delay.

#### Trigger Conditions

All conditions must be met:

1. **Hero is alive** (`hero.is_alive()`)
2. **Hero has debuff** (`hero.has_debuff == true`)
3. **Berserker Blood ability found** in `ability0-3` with name `"huskar_berserkers_blood"`
4. **Ability is ready**: `can_cast == true`, `level > 0`, `cooldown == 0`
5. **Delay timer elapsed** (`berserker_blood_delay_ms` since first debuff detected)

#### Delay Timer Logic

When a debuff is first detected, the script starts a timer. If the debuff persists for the configured delay (default: 300ms), Berserker Blood is activated. This allows waiting for **multiple debuffs** to stack before cleansing (more efficient).

**State tracking:**
- `BERSERKER_BLOOD_DEBUFF_DETECTED` stores the timestamp of first debuff detection
- When debuff disappears, the tracker is reset
- Once the delay elapses, the ability is activated and tracker is reset

Example flow:
```
T=0ms:    Debuff detected → Start 300ms timer
T=150ms:  Still debuffed, waiting...
T=300ms:  Timer elapsed → Activate Berserker Blood (E key pressed)
T=301ms:  Tracker reset
```

If debuffs are removed before the delay elapses (e.g., by other dispels), the tracker resets without wasting the ability.

### 🛡️ Survivability Actions

Huskar uses the common `SurvivabilityActions` system:

- **Healing items** – Auto-use Magic Wand, Faerie Fire, Satanic, etc. when HP drops
- **Defensive items** – BKB, Lotus Orb, Blade Mail when danger detected
- **Neutral items** – Witchbane, Safety Bubble, etc.
- **Danger detection** – Monitors HP changes and enemy abilities

These features share the global `[common]`, `[danger_detection]`, and `[neutral_items]` config sections.

### 🔒 Thread Safety

**Armlet toggle runs in a spawned thread** to prevent blocking GSI processing. A `try_lock()` guard ensures only one armlet toggle thread runs at a time:

```rust
let Ok(_guard) = ARMLET_THREAD_GUARD.try_lock() else {
    debug!("Armlet toggle already in progress, skipping");
    return;
};
```

If another toggle is already running, subsequent triggers are skipped. This prevents race conditions and excessive toggling.

### ⚠️ Standalone Trigger Not Implemented

The `standalone_key` config option exists but the script currently logs:

```
Huskar standalone trigger not implemented
```

Future enhancements may add a manual combo sequence (e.g., Blink + abilities).

### Usage

1. **Equip Armlet of Mordiggian** in-game
2. **Level Berserker Blood** (E ability)
3. **Configure thresholds** in `config/config.toml` to match your playstyle
4. **Run the app** – Hero is auto-detected via GSI
5. **Armlet toggles automatically** when HP drops below threshold
6. **Berserker Blood cleanses debuffs** after delay

### Logging

With `level = "info"`, you'll see:
```
Debuff detected, starting 300ms timer for Berserker Blood
Activating Berserker Blood to cleanse debuffs (300ms delay elapsed)
```

With `level = "debug"`:
```
Armlet toggle already in progress, skipping
Berserker Blood not ready: can_cast=true, level=4, cooldown=5.2
Waiting for more debuffs... (150ms elapsed)
```
