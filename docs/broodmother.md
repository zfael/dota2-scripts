# Broodmother Automation

## Features

### 🕷️ Spider Micro Macro (Middle Mouse)
Press Middle Mouse to execute a quick spider attack command:
1. Select spiderlings (default: F3)
2. Right-click at current mouse position (attack)
3. Reselect hero (default: 1)

This allows rapid spider micro without taking your hand off the mouse.

### 🎯 Auto-Items & Abilities on Space + Right-Click
Hold Space and right-click on an enemy to automatically:
1. Use offensive items (Orchid, Bloodthorn, Nullifier, Abyssal Blade, etc.)
2. Use configured abilities with optional HP thresholds
3. Attack the target

**Broodmother Ability Layout (as of 2026):**
- `ability0` (Q): Insatiable Hunger - lifesteal buff
- `ability1` (W): Spin Web - creates web
- `ability2` (E): Incapacitating Bite - passive slow
- `ability3` (D): Spider's Milk - innate passive
- `ability4` (R): Spawn Spiderlings - **Ultimate**

Ideal for burst combos with instant item and ability usage.

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
spider_control_group_key = "F3"  # Key to select spiderlings
reselect_hero_key = "1"          # Key to reselect hero after command

# Auto-items on Space + Right-click
auto_items_enabled = true
auto_items_modifier = "Space"    # Modifier key to hold
auto_items = ["orchid", "bloodthorn", "diffusal", "disperser", "nullifier", "abyssal_blade"]

# Auto-abilities: Cast abilities during Space+Right-click combo
# Each entry: index (ability0-5), key to press, optional hp_threshold (only cast if HP% below)
auto_abilities = [
    { index = 0, key = "q", hp_threshold = 80 },  # Insatiable Hunger when HP < 80%
    { index = 4, key = "r" },                      # Spawn Spiderlings (ultimate) - always
]

# Execution order: false = items first (default), true = abilities first
auto_abilities_first = false

[danger_detection]
# Auto-use Manta Style when silenced (applies to all heroes)
auto_manta_on_silence = true
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `spider_micro_enabled` | bool | `true` | Enable/disable spider micro macro |
| `spider_control_group_key` | string | `"F3"` | Control group key for spiderlings |
| `reselect_hero_key` | string | `"1"` | Key to reselect hero after issuing command |
| `auto_items_enabled` | bool | `true` | Enable Space+Right-click auto-items |
| `auto_items_modifier` | string | `"Space"` | Modifier key for auto-items |
| `auto_items` | list | `[]` | Items to use on combo |
| `auto_abilities` | list | `[]` | Abilities to cast (see format below) |
| `auto_abilities_first` | bool | `false` | Cast abilities before items if true |

### Auto-Abilities Format

Each entry in `auto_abilities` is an object with:
- `index`: Ability slot (0-5, maps to ability0-ability5 in GSI)
- `key`: Key to press ('q', 'w', 'e', 'r', 'd', 'f')
- `hp_threshold` (optional): Only cast when HP% is below this value

Example configurations:
```toml
# Always cast ultimate (R)
{ index = 4, key = "r" }

# Cast Q only when HP below 50%
{ index = 0, key = "q", hp_threshold = 50 }
```

## Usage

1. Set up control group for your spiderlings in-game (e.g., F3 for "Select All Other Units")
2. Press Middle Mouse to send spiders to right-click at cursor location
3. Hold Space + Right-click on enemy to execute full burst combo
4. Manta Style automatically dispels silence
5. Survivability features activate automatically when playing Broodmother
