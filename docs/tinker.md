# Tinker Automation

Intelligent combo automation for Tinker that casts available abilities/items in priority order and only triggers Rearm when everything is on cooldown.

## Features

### 🎯 Smart Combo System
- **Priority-Based Casting**: Casts the first available ability/item from a configurable priority list
- **Intelligent Rearm Timing**: Only triggers Rearm when ALL tracked abilities are on cooldown
- **Continuous Loop**: While combo is active, cycles through: Cast → Rearm → Repeat

### 🔄 Rearm Interrupt Detection
- Tracks Laser cooldown before Rearm starts
- After Rearm completes, verifies if Laser's CD was actually reset
- If Laser is still on same CD (interrupt detected) and `rearm_retry_on_interrupt` is enabled, automatically retries Rearm
- Uses mana consumption as indicator - mana is consumed even on interrupted Rearms

### 🛡️ Auto Defense Matrix
- Automatically casts Defense Matrix when in danger (enemy hero nearby)
- Only triggers when HP drops below configured threshold
- Can be disabled independently from combo

## Configuration

In `config/config.toml`:

```toml
[heroes.tinker]
enabled = true
auto_matrix_enabled = true          # Auto-cast Defense Matrix when in danger
auto_matrix_hp_threshold = 50       # HP% threshold for auto matrix
rearm_retry_on_interrupt = true     # Re-attempt Rearm if interrupted

# Priority order for combo (casts first available)
combo_priority = [
    "item_sheepstick",      # Hex - highest priority
    "item_ethereal_blade",  # E-Blade
    "laser",                # Laser
    "item_dagon",           # Dagon (any level)
    "item_shivas_guard",    # Shiva's Guard
    "warp_flare"            # Warp Flare (D)
]

laser_key = "q"
matrix_key = "e"
warp_flare_key = "d"
rearm_key = "r"
standalone_key = "Home"             # Toggle combo on/off
```

### Combo Priority

The `combo_priority` list determines the order abilities/items are cast. The system:
1. Iterates through the list from first to last
2. Finds the first ability/item that is **off cooldown**
3. Casts it
4. If nothing is available, triggers Rearm

#### Supported Items
| Config Name | Item |
|-------------|------|
| `item_sheepstick` | Scythe of Vyse (Hex) |
| `item_ethereal_blade` | Ethereal Blade |
| `item_dagon` | Dagon (any level 1-5) |
| `item_shivas_guard` | Shiva's Guard |

#### Supported Abilities
| Config Name | Ability |
|-------------|---------|
| `laser` | Laser (Q) |
| `warp_flare` | Warp Flare (D) |

## Usage

### Toggle Combo
Press **Home** (default) to toggle the combo on/off.

### How It Works

```
┌─────────────────────────────────────────────────────────────┐
│                    COMBO STATE MACHINE                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  [Home pressed] ──► combo_active = true                     │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              CHECK PRIORITY LIST                     │   │
│  │  For each item in combo_priority:                    │   │
│  │    - Check if off cooldown                           │   │
│  │    - If yes → CAST IT → return                       │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│                    (nothing available)                      │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  TRIGGER REARM                       │   │
│  │  1. Save Laser cooldown (laser_cd_before_rearm)      │   │
│  │  2. Press R                                          │   │
│  │  3. Set awaiting_rearm_verify = true                 │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │               VERIFY REARM SUCCESS                   │   │
│  │  On next GSI tick:                                   │   │
│  │    - Compare current Laser CD vs saved               │   │
│  │    - If CD was reset → Success, continue combo       │   │
│  │    - If CD same → Interrupted, retry if enabled      │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│                           ▼                                 │
│                    (loop continues)                         │
│                                                             │
│  [Home pressed again] ──► combo_active = false              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## State Machine

### TinkerState
| Field | Type | Description |
|-------|------|-------------|
| `combo_active` | `bool` | Whether combo is currently running |
| `rearm_started` | `bool` | Whether Rearm is currently channeling |
| `laser_cd_before_rearm` | `f64` | Laser's cooldown when Rearm started |
| `awaiting_rearm_verify` | `bool` | Waiting to verify Rearm completion |

## Tips

1. **Customize Priority**: Reorder `combo_priority` to match your playstyle. Put your most important item first.

2. **Add/Remove Items**: You can add or remove items from the priority list. Only include items you actually have.

3. **Burst Order**: For maximum burst damage, consider:
   - Hex → E-Blade → Laser → Dagon (E-Blade amp applies to magic damage)

4. **Defense**: The auto matrix feature works independently - even if combo is off, it will protect you.

## Keybindings

| Key | Action |
|-----|--------|
| Home | Toggle combo on/off |

## Logging

Tinker actions are logged with the ⚡ emoji prefix:
- `⚡ Tinker combo ACTIVATED`
- `⚡ Tinker combo DEACTIVATED`
- `⚡ Casting: Laser`
- `⚡ All on CD, triggering Rearm`
- `⚡ Rearm interrupted! Retrying...`
- `⚡ Rearm successful, cooldowns reset`
