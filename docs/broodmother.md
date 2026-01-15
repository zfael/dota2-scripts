# Broodmother Automation

## Features

### 🕷️ Spider Micro Macro (Middle Mouse)
Press Middle Mouse to execute a quick spider attack-move command:
1. Select spiderlings (default: F2)
2. Issue attack command (default: A)
3. Left-click at current mouse position
4. Reselect hero (default: F1)

This allows rapid spider micro without taking your hand off the mouse.

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
# Spider micro: Mouse5 triggers select spiders → A-click → reselect hero
spider_micro_enabled = true
spider_control_group_key = "F2"  # Key to select spiderlings
reselect_hero_key = "F1"         # Key to reselect hero after command
attack_key = "a"                 # Attack-move command key
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `spider_micro_enabled` | bool | `true` | Enable/disable spider micro macro |
| `spider_control_group_key` | string | `"F2"` | Control group key for spiderlings |
| `reselect_hero_key` | string | `"F1"` | Key to reselect hero after issuing command |
| `attack_key` | char | `'a'` | Attack-move command key |

## Usage

1. Set up control group 2 (F2) for your spiderlings in-game
2. Press Mouse5 to send spiders to attack-move at cursor location
3. Survivability features activate automatically when playing Broodmother
