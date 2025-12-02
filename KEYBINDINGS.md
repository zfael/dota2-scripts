# Custom Keybindings Per Hero

## Overview
Each hero can have a custom keybinding for triggering standalone combos. This allows you to use different keys based on your in-game setup.

## Default Key
All heroes default to `Home` key for standalone combos.

## Configuration

Edit `config/config.toml` to customize keybindings:

```toml
[heroes.huskar]
standalone_key = "Home"  # Change to your preferred key

[heroes.tiny]
standalone_key = "Home"  # Change to your preferred key
```

## Supported Keys

### Function Keys
- `F1` through `F12`

### Special Keys
- `Home`, `End`, `Insert`, `Delete`
- `PageUp`, `PageDown`

### Letter Keys
- Any single letter from `a` to `z` (case-insensitive)

## Example Use Case: Shadow Fiend

For Shadow Fiend, you might remap ability keys in-game:
- Normal keys: Q, W, E (Shadow Raze)
- Remapped to: L, K, J

Then set the standalone combo to intercept Q/W/E:
```toml
[heroes.shadow_fiend]
standalone_key = "q"  # or "w" or "e"
```

This allows the script to:
1. Detect Q/W/E keypress
2. Emit right-click to point SF in correct direction
3. Press the actual ability key (L/K/J)

NOTE: enable cl_dota_alt_unit_movetodirection true

## Runtime Behavior

- The active keybinding is displayed in the UI
- When you switch heroes, the keybinding automatically updates
- The keyboard listener dynamically listens for the current hero's key
- No restart required when changing heroes in the UI

## Notes

- Keybindings are case-insensitive for letter keys
- The UI shows the currently active key for the selected hero
- Changes to `config.toml` require application restart
