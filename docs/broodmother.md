# Broodmother Automation

## Features

### 🕷️ Spider Micro Macro (Middle Mouse)
Press Middle Mouse to execute a quick spider attack command:
1. Select spiderlings (default: F2)
2. Right-click at current mouse position (attack)
3. Reselect hero (default: F1)

This allows rapid spider micro without taking your hand off the mouse.

### 🎯 Auto-Items on Space + Right-Click
Hold Space and right-click on an enemy to automatically:
1. Use offensive items (Orchid, Bloodthorn, Nullifier, Abyssal Blade)
2. Pop ultimate (R) if enabled
3. Use Spawn Spiderlings (Q) if HP below threshold
4. Attack the target

Ideal for burst combos with instant item usage.

### 🌀 Auto-Manta on Silence
Automatically uses Manta Style when silenced:
- Triggers immediately with 30-100ms random jitter
- Independent of danger detection (global setting)
- Configure via `auto_manta_on_silence` in `[danger_detection]`

### 💊 Survivability
Automatically uses healing and defensive items when HP drops below thresholds:
- **Healing items**: Magic Wand, Faerie Fire, Satanic, etc.
- **Defensive items**: BKB, Lotus Orb when danger detected
- **Neutral items**: Witchbane, Safety Bubble, etc.

Uses shared survivability thresholds from `[survivability]` config section.

### ⚠️ Danger Detection
Detects enemy abilities and triggers defensive item usage:
- Monitors for targeted stuns, silences, and damage abilities
- Triggers BKB, Lotus Orb, or other defensive items

## Configuration

Add to `config/config.toml`:

```toml
[heroes.broodmother]
# Spider micro: Middle Mouse triggers select spiders → right-click → reselect hero
spider_micro_enabled = true
spider_control_group_key = "F2"  # Key to select spiderlings
reselect_hero_key = "F1"         # Key to reselect hero after command

# Auto-items on Space + Right-click
auto_items_enabled = true
auto_items_modifier = "Space"    # Modifier key to hold
auto_items = ["orchid", "bloodthorn", "nullifier", "abyssal_blade"]
auto_ult_enabled = true          # Use R (ultimate) during combo
auto_q_enabled = true            # Use Q when HP below threshold
auto_q_hp_threshold = 80         # HP % threshold for auto-Q

[danger_detection]
# Auto-use Manta Style when silenced (applies to all heroes)
auto_manta_on_silence = true
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `spider_micro_enabled` | bool | `true` | Enable/disable spider micro macro |
| `spider_control_group_key` | string | `"F2"` | Control group key for spiderlings |
| `reselect_hero_key` | string | `"F1"` | Key to reselect hero after issuing command |
| `auto_items_enabled` | bool | `true` | Enable Space+Right-click auto-items |
| `auto_items_modifier` | string | `"Space"` | Modifier key for auto-items |
| `auto_items` | list | `[...]` | Items to use on combo |
| `auto_ult_enabled` | bool | `true` | Use ultimate during combo |
| `auto_q_enabled` | bool | `true` | Use Q ability during combo |
| `auto_q_hp_threshold` | u8 | `80` | HP % threshold for auto-Q |

## Usage

1. Set up control group 2 (F2) for your spiderlings in-game
2. Press Middle Mouse to send spiders to right-click at cursor location
3. Hold Space + Right-click on enemy to execute full burst combo
4. Manta Style automatically dispels silence
5. Survivability features activate automatically when playing Broodmother
