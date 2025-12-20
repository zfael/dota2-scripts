# Soul Ring Automation

This document describes the Soul Ring item automation feature, which automatically triggers Soul Ring before ability or item usage to optimize mana efficiency.

## Overview

**Soul Ring** is an item that sacrifices 170 HP to grant 170 temporary mana for 10 seconds. The automation optimizes its usage by:

1. **Automatically detecting** when Soul Ring is in your inventory via GSI
2. **Intercepting** ability and item keypresses
3. **Triggering Soul Ring first**, then forwarding the original keypress
4. **Applying safety checks** to avoid suicide or wasted usage

## How It Works

### Key Interception Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    You press Q                               │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│              Keyboard grab() intercepts key                  │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   Check conditions:                          │
│   • Soul Ring in inventory?                                  │
│   • Soul Ring off cooldown?                                  │
│   • Mana below threshold?                                    │
│   • Health above safety threshold?                           │
│   • Cooldown lockout elapsed?                                │
└─────────────────────────────────────────────────────────────┘
                           │
              ┌────────────┴────────────┐
              │                         │
         All true                  Any false
              │                         │
              ▼                         ▼
┌─────────────────────┐    ┌─────────────────────┐
│ Block original key  │    │ Pass through key    │
│ Press Soul Ring     │    │ (no interception)   │
│ Wait delay (30ms)   │    └─────────────────────┘
│ Simulate original Q │
└─────────────────────┘
```

### GSI State Updates

On every Game State Integration event, the script updates:

| Field | Source | Description |
|-------|--------|-------------|
| `available` | `items.slot0-5` | Whether Soul Ring is in inventory |
| `slot_key` | Keybindings config | Which key to press for Soul Ring |
| `can_cast` | `item.can_cast` | Not on cooldown |
| `hero_mana_percent` | `hero.mana_percent` | Current mana % |
| `hero_health_percent` | `hero.health_percent` | Current health % |
| `hero_alive` | `hero.alive` | Whether hero is alive |

### Auto-Enable/Disable

The automation **automatically enables** when Soul Ring appears in your inventory and **automatically disables** when you sell or drop it. No manual toggle needed.

## Configuration

Located in `config/config.toml` under `[soul_ring]`:

```toml
[soul_ring]
# Master toggle for Soul Ring automation
enabled = true

# Only trigger Soul Ring if mana percent is below this threshold
min_mana_percent = 90

# Safety threshold - don't use Soul Ring if health percent is at or below this
min_health_percent = 20

# Delay in milliseconds between Soul Ring press and ability press
delay_before_ability_ms = 30

# Cooldown lockout in milliseconds to prevent double-fire on double-tap
trigger_cooldown_ms = 500

# Ability keys to intercept for Soul Ring triggering
ability_keys = ["q", "w", "e", "r", "d", "f"]

# Also trigger Soul Ring before item key presses (items that cost mana)
intercept_item_keys = true
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `true` | Master toggle for the feature |
| `min_mana_percent` | u32 | `90` | Only trigger if mana is below this % |
| `min_health_percent` | u32 | `20` | Don't trigger if health is at or below this % |
| `delay_before_ability_ms` | u64 | `30` | Delay between Soul Ring and ability press |
| `trigger_cooldown_ms` | u64 | `500` | Lockout period after triggering (prevents double-fire) |
| `ability_keys` | Vec<String> | `["q","w","e","r","d","f"]` | Ability keys to intercept |
| `intercept_item_keys` | bool | `true` | Also intercept item slot keys |

## Safety Features

### Health Threshold

Soul Ring costs 170 HP. The `min_health_percent` setting (default 20%) prevents the automation from triggering when your health is too low, avoiding accidental suicide.

### Mana Threshold

The `min_mana_percent` setting (default 90%) ensures Soul Ring only triggers when you actually need mana. If you're at 95% mana, triggering Soul Ring would waste most of the temporary mana.

### Cooldown Lockout

The `trigger_cooldown_ms` setting (default 500ms) prevents double-firing when:
- You double-tap an ability for self-cast
- Multiple keypresses happen in quick succession
- GSI updates arrive rapidly

### Infinite Loop Prevention

The automation will **not** trigger Soul Ring when you press Soul Ring's own item slot key (would cause infinite loop).

## Intercepted Keys

### Ability Keys

By default, these keys are intercepted:
- **Q** - First ability
- **W** - Second ability
- **E** - Third ability
- **R** - Ultimate
- **D** - Fourth ability (if available)
- **F** - Fifth ability (if available)

### Item Keys

When `intercept_item_keys = true`, item slot keys are also intercepted:
- Slot keys from `[keybindings]` config (default: Z, X, C, V, B, N)
- Excludes Soul Ring's own slot (to prevent infinite loop)

This is useful for mana-costing items like:
- Shiva's Guard
- Scythe of Vyse (Hex)
- Orchid Malevolence
- Dagon
- etc.

## Integration with Hero Scripts

Soul Ring automation runs **before** hero-specific key handling:

1. Key intercepted
2. Soul Ring triggered (if applicable)
3. Hero-specific logic runs (e.g., Shadow Fiend raze remap)
4. Final key simulated

This ensures Soul Ring works correctly with:
- Shadow Fiend's Q/W/E raze remapping
- Largo's beat timing system
- Any other hero-specific automation

## Logging

With `level = "info"` in logging config, you'll see:
```
💍 Soul Ring triggered! mana=45%, health=80%
💍 Soul Ring found in slot2: can_cast=true, key=Some('c')
💍 Soul Ring no longer in inventory, disabling automation
```

With `level = "debug"`, additional diagnostics:
```
💍 Key 'q': intercept=true, trigger=true, available=true, can_cast=true, mana=45%, health=80%
💍 Pressing Soul Ring key: c
```

## Technical Details

### Key Files

| File | Purpose |
|------|---------|
| `src/actions/soul_ring.rs` | State tracking and trigger logic |
| `src/input/keyboard.rs` | Key interception with `grab()` |
| `src/actions/dispatcher.rs` | GSI event updates to Soul Ring state |
| `src/config/settings.rs` | `SoulRingConfig` struct |

### Dependencies

- **rdev** with `unstable_grab` feature for key interception
- Key interception uses Windows low-level keyboard hooks
- `grab()` blocks keys from reaching other applications when returning `None`

### Thread Safety

Soul Ring state is stored in a global `Arc<Mutex<SoulRingState>>`:
- GSI handler thread updates state
- Keyboard listener thread reads state
- Mutex ensures safe concurrent access
