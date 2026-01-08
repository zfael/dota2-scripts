# Bottle Optimization

Automatically swaps stat items (Iron Branch) to stash and back before using bottle, putting them on cooldown to temporarily reduce max HP/mana for better percentage-based healing from bottle.

## How It Works

1. **Intercepts bottle hotkey** - When you press the bottle slot key, the feature intercepts it
2. **Swaps stat items to stash** - Drags Iron Branches from inventory to empty stash slots via mouse
3. **Swaps items back** - Drags items back to inventory (this puts them on cooldown, reducing stats)
4. **Uses bottle** - Simulates the bottle key press for actual bottle use
5. **Restores mouse position** - Returns cursor to original location

Since the stat items are on cooldown, your max HP/mana is temporarily lower, making the flat HP/mana restore from bottle a higher percentage of your total.

## Requirements

- **Game time**: Only works before configured threshold (default: 10 minutes)
- **Bottle with charges**: Must have bottle in inventory with remaining charges
- **Target items**: Must have configured stat items (Iron Branch by default) in inventory
- **Empty stash slots**: Must have at least one empty stash slot available

## Batch Processing

Items are processed in batches based on available empty stash slots:

| Empty Stash Slots | Items in Inventory | Batches |
|-------------------|-------------------|---------|
| 3 | 2 | 1 batch of 2 |
| 2 | 4 | 2 batches of 2 |
| 1 | 3 | 3 batches of 1 |

## Configuration

```toml
[bottle_optimization]
# Master toggle
enabled = true

# Only trigger before this game time (seconds). 600 = 10 minutes
max_game_time_seconds = 600

# Items to swap (stat items that reduce max HP/mana when on cooldown)
target_items = ["item_branches"]

# Restore mouse position after operations
restore_mouse_position = true

# Delay between mouse operations (ms)
delay_between_drags_ms = 75

# Random pixel offset for humanized mouse movement
mouse_jitter_px = 4

# Cooldown between triggers (ms)
trigger_cooldown_ms = 500
```

## Screen Position Setup

The feature requires accurate screen coordinates for inventory and stash slots. A capture tool is provided:

### Running the Capture Tool

```bash
cargo run --bin capture_coords
```

### Capture Tool Usage

1. **View detected monitors** - Shows all connected displays with positions
2. **Move mouse** - Real-time position display as you move cursor
3. **Left-click** - Capture position at cursor, enter label (0-11 for quick labels)
4. **Right-click** - Save all positions to `config/screen_positions.toml`
5. **Escape** - Exit

### Quick Labels

| Number | Label |
|--------|-------|
| 0-5 | slot0-slot5 (inventory) |
| 6-11 | stash0-stash5 (stash) |

### Screen Positions Config

Positions are stored in `config/config.toml`:

```toml
[screen_positions]
resolution = "1920x1080"

[screen_positions.inventory_positions]
slot0 = { x = 1298, y = 942 }
slot1 = { x = 1347, y = 942 }
# ... etc

[screen_positions.stash_positions]
stash0 = { x = 376, y = 496 }
stash1 = { x = 424, y = 496 }
# ... etc
```

## Multi-Monitor Support

The capture tool and optimization feature support multi-monitor setups:

- Coordinates are absolute across all displays
- Primary monitor origin is (0, 0)
- Secondary monitors may have negative coordinates if positioned left/above primary
- Capture tool shows which monitor the cursor is on

## State Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    Bottle Key Pressed                           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Check Conditions:                                              │
│  - Enabled in config?                                           │
│  - Game time < threshold?                                       │
│  - Bottle has charges?                                          │
│  - Has target items in inventory?                               │
│  - Has empty stash slots?                                       │
│  - Not already in progress?                                     │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
        All conditions met            Conditions not met
              │                               │
              ▼                               ▼
   ┌──────────────────────┐          ┌──────────────────┐
   │ Block original key   │          │ Pass through key │
   │ Start optimization   │          │ (normal bottle)  │
   └──────────────────────┘          └──────────────────┘
              │
              ▼
   ┌──────────────────────┐
   │ Save mouse position  │
   └──────────────────────┘
              │
              ▼
   ┌──────────────────────┐
   │ For each batch:      │
   │ 1. Drag to stash     │◄────┐
   │ 2. Drag back         │     │
   └──────────────────────┘     │
              │                 │
              ▼                 │
   ┌──────────────────────┐     │
   │ More items to swap?  │─Yes─┘
   └──────────────────────┘
              │ No
              ▼
   ┌──────────────────────┐
   │ Press bottle key     │
   └──────────────────────┘
              │
              ▼
   ┌──────────────────────┐
   │ Restore mouse pos    │
   └──────────────────────┘
```

## Logging

The feature uses the 🍾 emoji prefix for log messages:

```
🍾 Found bottle in slot2 (key=Some('c'), can_cast=true, charges=3)
🍾 Found target item 'item_branches' in slot0
🍾 Found empty stash slot: stash0
🍾 Intercepting bottle key 'c' for optimization
🍾 Starting bottle optimization: 2 stat items, 3 empty stash slots
🍾 Processing batch of 2 items
🍾 Using bottle (key: c)
🍾 Bottle optimization complete!
```

## Limitations

1. **Requires open stash** - The stash must be visible on screen (near fountain/secret shop)
2. **UI scale dependent** - Screen positions must match your Dota 2 UI scale
3. **Resolution specific** - Recapture positions if you change resolution
4. **Not for combat** - Designed for fountain healing, not mid-fight use
