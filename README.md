# Dota 2 Script Automation

A Rust-based automation tool for Dota 2 that provides two main features:

1. **GSI Event-Driven Actions**: Automated responses based on real-time game state (HP management, hero-specific actions)
2. **Standalone Hero Combos**: Key-triggered ability sequences for specific heroes

## Features

### GSI Event Automation
- **Survivability Actions**: Automatically uses healing items when HP drops below threshold
  - item_cheese
  - item_faerie_fire
  - item_magic_wand
  - item_enchanted_mango
  - item_greater_faerie_fire
- **Danger Detection**: Monitors HP changes to detect combat situations
  - Automatically triggers defensive items (Satanic, Blade Mail, BKB, etc.)
  - Uses multiple healing items when in danger
  - Higher HP threshold when danger detected
  - Configurable detection thresholds and item preferences

### Hero-Specific Actions:
  - **Huskar**: Automatic armlet toggling at low HP, Berserker Blood debuff cleansing
  - **Shadow Fiend**: Q/W/E interception with auto right-click for raze accuracy
  - **Tiny**: (GSI actions can be added)

### Standalone Combos
- **Tiny**: Press HOME key to execute full combo sequence (blink + abilities)
- **Shadow Fiend**: Q/W/E keys auto-emit right-click before casting razes
- Easily extensible for other heroes

### GUI Features
- Hero selection (Huskar, Shadow Fiend, Tiny, or None)
- Toggle GSI automation on/off
- Toggle standalone scripts on/off
- Real-time status display (HP, Mana, Hero status, Danger indicator)
- Danger Detection configuration tab with item toggles
- Per-hero keybinding display
- Debug metrics (events processed, queue depth)
- Auto-selection of hero based on GSI events

## Requirements

- Windows OS (tested on Windows 10/11)
- Rust toolchain (1.70+)
- Dota 2 with Game State Integration enabled

## Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/dota2-scripts.git
cd dota2-scripts
```

2. Build the project:
```bash
cargo build --release
```

3. Set up Dota 2 GSI:
   - Create a file in your Dota 2 config folder:
     `steamapps/common/dota 2 beta/game/dota/cfg/gamestate_integration/gamestate_integration_dota2scripts.cfg`
   - Add the following content:
   ```
   "Dota 2 Integration Configuration"
   {
       "uri"               "http://localhost:3000/"
       "timeout"           "5.0"
       "buffer"            "0.1"
       "throttle"          "0.1"
       "heartbeat"         "30.0"
       "data"
       {
           "provider"      "1"
           "map"           "1"
           "player"        "1"
           "hero"          "1"
           "abilities"     "1"
           "items"         "1"
       }
   }
   ```

## Configuration

Edit `config/config.toml` to customize:

```toml
[server]
port = 3000

[keybindings]
slot0 = "z"
slot1 = "x"
slot2 = "c"
slot3 = "v"
slot4 = "b"
slot5 = "n"
neutral0 = "0"
combo_trigger = "Home"

[logging]
level = "info"  # Change to "debug" for verbose logging

[common]
survivability_hp_threshold = 30  # HP percentage

[heroes.huskar]
armlet_toggle_threshold = 320  # Absolute HP value
armlet_predictive_offset = 30
armlet_toggle_cooldown_ms = 250
berserker_blood_key = "e"
berserker_blood_delay_ms = 300
standalone_key = "Home"

[heroes.shadow_fiend]
q_ability_key = "l"  # Actual key for Q raze (remapped from Q in-game)
w_ability_key = "k"  # Actual key for W raze (remapped from W in-game)
e_ability_key = "j"  # Actual key for E raze (remapped from E in-game)
raze_delay_ms = 100  # Delay between right-click and ability cast

[heroes.tiny]
standalone_key = "Home"

[danger_detection]
enabled = true
hp_threshold_percent = 70          # Danger detected below this HP %
rapid_loss_hp = 100                # HP loss to trigger danger
time_window_ms = 500               # Time window for HP loss measurement
clear_delay_seconds = 3            # Delay before clearing danger state
healing_threshold_in_danger = 50   # HP % to use healing when in danger
max_healing_items_per_danger = 3   # Max healing items per danger event
auto_bkb = false                   # Auto Black King Bar (opt-in)
auto_satanic = true                # Auto Satanic
auto_blade_mail = true             # Auto Blade Mail
auto_glimmer_cape = true           # Auto Glimmer Cape
auto_ghost_scepter = true          # Auto Ghost Scepter
auto_shivas_guard = true           # Auto Shiva's Guard
```

### Danger Detection Feature

The Danger Detection system monitors HP changes to identify combat situations and automatically responds:

**How it works:**
- Tracks HP every ~100ms (via GSI events)
- Triggers when HP drops rapidly (100+ HP in 500ms) OR HP below 70% with active loss
- Clears after HP stabilizes for 3 seconds

**When in danger:**
- Uses healing items at 50% HP (vs 30% normally)
- Can use multiple healing items (up to 3) per danger event
- High-value priority: cheese → greater faerie fire → mango → wand → faerie fire
- Auto-activates defensive items (if enabled and off cooldown):
  - **BKB**: Magic immunity (opt-in, default: false)
  - **Satanic**: Active lifesteal
  - **Blade Mail**: Damage reflection
  - **Glimmer Cape**: Magic resistance + invisibility
  - **Ghost Scepter**: Physical immunity
  - **Shiva's Guard**: AoE damage + armor

**Configuration:**
- Use the "Danger Detection" tab in the GUI to adjust thresholds
- Configure which items auto-trigger via checkboxes
- Save configuration to persist settings

**See `.copilot/1_in_danger_implementation.md` for detailed implementation docs**

### Configuring Shadow Fiend
Shadow Fiend uses **key interception** instead of a standalone combo trigger. When enabled:
1. Press Q/W/E in-game
2. Script intercepts the keypress
3. Emits a right-click to point SF in the correct direction
4. After `raze_delay_ms`, presses the actual ability key (L/K/J)

**Why this is useful**: If you remap your abilities in-game (Q→L, W→K, E→J), the script can intercept the normal Q/W/E keys and automatically right-click before casting, ensuring razes point in the correct direction.

**Hero Internal Names**: Find hero names at https://developer.valvesoftware.com/wiki/Dota_2_Workshop_Tools/Scripting/Heroes_internal_names
- Shadow Fiend: `npc_dota_hero_nevermore`
- Huskar: `npc_dota_hero_huskar`
- Tiny: `npc_dota_hero_tiny`
```

## Usage

1. Run the application:
```bash
cargo run --release
```

2. Launch Dota 2 and start a game

3. The GUI will show:
   - Current hero (auto-detected)
   - Real-time HP/Mana
   - Status effects
   - Event metrics

4. Controls:
   - **GSI Automation**: Enabled by default, handles survivability + hero actions
   - **Standalone Script**: Press HOME key to trigger hero combo
   - **Hero Selection**: Auto-selects based on game, or choose manually

## Development

### Project Structure
```
src/
├── actions/         # Action handlers
│   ├── common.rs    # Survivability & armlet logic
│   ├── danger_detector.rs  # Danger detection system
│   ├── dispatcher.rs # Strategy pattern dispatcher
│   └── heroes/      # Hero-specific scripts
│       ├── huskar.rs
│       ├── tiny.rs
│       └── traits.rs
├── config/          # Configuration management
├── gsi/             # GSI server & event handling
├── input/           # Keyboard simulation & listening
├── models/          # GSI event data models
├── state/           # Application state
├── ui/              # GUI implementation (with Danger Detection tab)
└── main.rs          # Entry point
```

### Adding a New Hero

**Reference**: Use https://developer.valvesoftware.com/wiki/Dota_2_Workshop_Tools/Scripting/Heroes_internal_names to find the correct internal hero name (e.g., `npc_dota_hero_nevermore` for Shadow Fiend).

1. Create `src/actions/heroes/your_hero.rs`:
```rust
use crate::actions::heroes::HeroScript;
use crate::models::GsiWebhookEvent;
use std::any::Any;

pub struct YourHeroScript;

impl HeroScript for YourHeroScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // GSI-based automation
    }

    fn handle_standalone_trigger(&self) {
        // Combo sequence
    }

    fn hero_name(&self) -> &'static str {
        "npc_dota_hero_your_hero"  // Use correct internal name
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}
```

2. Register in `src/actions/dispatcher.rs`
3. Add to `HeroType` enum in `src/state/app_state.rs`
4. Add config struct in `src/config/settings.rs`
5. Update UI in `src/ui/app.rs`

### Testing

Run tests:
```bash
cargo test
```

Test fixtures are in `tests/fixtures/` with sample GSI events.

## Logging

Set log level via environment variable:
```bash
# Debug mode
RUST_LOG=debug cargo run

# Info mode (default)
RUST_LOG=info cargo run
```

## License

MIT

## Disclaimer

This tool is for educational purposes. Use responsibly and ensure compliance with Dota 2 and Steam terms of service.

