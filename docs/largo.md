# Largo Automation

This document describes the Largo hero automation features, including the Amphibian Rhapsody ultimate helper.

## Overview

Largo's ultimate ability, **Amphibian Rhapsody**, requires precise timing to maintain Groovin' stacks. The automation assists by:

1. **Automatically detecting** when ultimate is active via GSI (Game State Integration)
2. **Pressing song keys** at precise intervals to maintain rhythm
3. **Queuing song switches** to preserve timing when changing songs mid-ultimate
4. **Supporting Aghanim's Scepter** dual-song mode

## Features

### GSI-Based Ultimate Detection

Instead of relying on manual R key tracking, the script detects ultimate mode by monitoring GSI ability names:

| Normal Mode | Ultimate Mode |
|-------------|---------------|
| `largo_catchy_lick` | `largo_song_fight_song` |
| `largo_frogstomp` | `largo_song_double_time` |
| `largo_croak_of_genius` | `largo_song_good_vibrations` |

When GSI reports abilities with the `largo_song_` prefix, ultimate mode is automatically activated.

### Beat Timing System

The script uses **absolute timing** to prevent cumulative drift:

- An anchor time is set when the first song is selected
- Each beat is calculated as: `anchor_time + (beat_count × interval) + corrections`
- This ensures that even if one beat fires slightly late, subsequent beats remain on schedule

#### Periodic Correction

To fine-tune timing, a correction can be applied every N beats:

```
Total time = (beat_count × beat_interval_ms) + (corrections_applied × beat_correction_ms)
```

Example with default settings (`beat_interval_ms=995`, `beat_correction_ms=30`, `beat_correction_every_n_beats=5`):

| Beat | Base Time | Correction | Actual Time |
|------|-----------|------------|-------------|
| 1 | 995ms | 0 | 995ms |
| 5 | 4975ms | +30ms | 5005ms |
| 10 | 9950ms | +60ms | 10010ms |
| 15 | 14925ms | +90ms | 15015ms |

### Song Selection

Songs are selected via Q/W/E hotkeys:

| Key | Song | Effect |
|-----|------|--------|
| Q | Bullbelly Blitz | Damage |
| W | Hotfeet Hustle | Movement Speed |
| E | Island Elixir | Healing |

#### Song Queuing

When switching songs mid-ultimate, the new song is **queued** and applied on the next beat. This maintains perfect rhythm instead of resetting timing:

1. You're playing Bullbelly (Q)
2. At T=400ms, you press W
3. Song is queued: `pending_song = Hotfeet`
4. At T=995ms (next beat), switch happens and W is pressed
5. Rhythm continues uninterrupted

### Aghanim's Scepter Support

With Aghanim's Scepter, you can play **two songs simultaneously**. The script detects Aghs via GSI and:

- Stores the `previous_song` when switching
- Presses both `current_song` and `previous_song` keys on each beat

### R Key Handling

When you press R to end the ultimate:

1. Beat loop **immediately stops** (prevents stale key presses)
2. All state is cleared (songs, pending, beat count)
3. GSI confirms the state change shortly after

This prevents Q/W/E presses during the window between R press and GSI confirmation.

## Configuration

All settings are in `config/config.toml` under `[heroes.largo]`:

```toml
[heroes.largo]
amphibian_rhapsody_enabled = true
auto_toggle_on_danger = true
mana_threshold_percent = 20     # Auto-disable ultimate below this mana
heal_hp_threshold = 50          # Switch to Island Elixir when HP below this

# Beat timing configuration
beat_interval_ms = 995          # Base interval per beat (990-1000)
beat_correction_ms = 30         # Correction to apply every N beats
beat_correction_every_n_beats = 5  # Apply correction every N beats (0 = disabled)

# Keybindings
q_ability_key = "q"             # Bullbelly Blitz (damage)
w_ability_key = "w"             # Hotfeet Hustle (movement)
e_ability_key = "e"             # Island Elixir (healing)
r_ability_key = "r"             # Amphibian Rhapsody toggle
standalone_key = "Home"         # Manual ultimate activation
```

### Tuning Beat Timing

If beats drift over time:

| Problem | Solution |
|---------|----------|
| Clicks happen **too early** | Increase `beat_correction_ms` (positive value) |
| Clicks happen **too late** | Decrease `beat_correction_ms` (negative value) |
| Drift happens too quickly | Decrease `beat_correction_every_n_beats` |
| Drift happens too slowly | Increase `beat_correction_every_n_beats` |
| Disable correction entirely | Set `beat_correction_every_n_beats = 0` |

## How It Works

### State Machine

```
┌─────────────────────────────────────────────────────────────┐
│                         INACTIVE                             │
│                    (active = false)                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ GSI detects largo_song_* abilities
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    ACTIVE (No Song)                          │
│              (active = true, current_song = None)            │
│                  Waiting for Q/W/E press                     │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ Q/W/E pressed (first song)
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    ACTIVE (Playing)                          │
│         (active = true, current_song = Some(song))           │
│            Beat thread pressing keys every ~995ms            │
│                                                              │
│   ┌─────────────────────────────────────────────────────┐   │
│   │  Q/W/E pressed → pending_song = new_song            │   │
│   │  Next beat → switch current_song, press new key     │   │
│   └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ R pressed OR GSI detects normal abilities
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                         INACTIVE                             │
│    (all state reset: songs, beat_count, beat_start_time)     │
└─────────────────────────────────────────────────────────────┘
```

### Beat Thread

A background thread runs continuously, checking every 5ms:

1. If `active == false` → skip
2. If `current_song == None` → skip
3. Calculate expected beat time with corrections
4. If `now >= expected_beat_time`:
   - Process `pending_song` if present (switch songs)
   - Press `current_song` key
   - Press `previous_song` key (if Aghs)
   - Increment `beat_count`

## Survivability Actions

In addition to ultimate management, the Largo script includes common survivability actions:

- **Healing items**: Auto-use when HP is low
- **Defensive items**: Auto-use when in danger
- **Neutral items**: Auto-use defensive neutral items when in danger

## Files

| File | Description |
|------|-------------|
| `src/actions/heroes/largo.rs` | Main Largo script implementation |
| `src/config/settings.rs` | LargoConfig struct and defaults |
| `config/config.toml` | User configuration |
