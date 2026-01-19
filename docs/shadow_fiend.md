# Shadow Fiend (Nevermore) Automation

This document describes the Shadow Fiend hero automation features, including the Shadowraze direction-facing helper and automatic BKB on ultimate.

## Overview

Shadow Fiend's Shadowraze abilities (Q/W/E) raze in the direction the hero is facing, not the cursor direction. This can be awkward because you need to right-click to face a direction first, then press the raze key.

The automation solves this by intercepting Q/W/E keypresses and automatically:
1. **Holding ALT** to enable `cl_dota_alt_unit_movetodirection` (move-to-direction on right-click)
2. **Right-clicking** to face the cursor direction
3. **Pressing the raze key** after a configurable delay

This allows you to raze toward your cursor naturally, similar to how most skillshots work.

## Features

### Automatic Direction Facing

When you press Q, W, or E with Shadow Fiend selected:

1. The keypress is **intercepted** (blocked from reaching the game)
2. ALT is held down (enables move-to-direction mode)
3. A right-click is simulated to face the cursor
4. ALT is released
5. After a short delay, the raze key is pressed

This happens in ~150-200ms total, making it feel nearly instant.

### Auto-BKB on Ultimate

When enabled, pressing R to cast Requiem of Souls will automatically:

1. **Check for BKB** in your inventory (if `auto_bkb_on_ultimate` is enabled)
2. **Use BKB** (double-tap for self-cast) if available and off cooldown
3. **Press D** (Aghanim's ability) if `auto_d_on_ultimate` is enabled
4. **Cast Requiem of Souls** (R)

This ensures you're protected by magic immunity during the channel without needing to manually activate BKB first.

**Sequence:** BKB (double-tap) → D (optional) → R

**Behavior when BKB is not present or on cooldown:** The script will skip BKB activation and proceed with D (if enabled) and R. If `auto_bkb_on_ultimate` is disabled entirely, pressing R will just press R normally with no interception.

### Standalone Combo (Blink + Ultimate)

When you press the standalone key (default: Home), the script will:

1. **Check if Blink is available** (not on cooldown)
2. If Blink is on cooldown → **Skip the combo entirely** (no action taken)
3. If Blink is available:
   - **Blink** to cursor position
   - **BKB** (double-tap, if `auto_bkb_on_ultimate` enabled and available)
   - **D ability** (if `auto_d_on_ultimate` enabled)
   - **R** (Requiem of Souls)

This allows you to execute the full initiation combo with a single key press, but only when Blink is ready.

**Sequence:** Blink → BKB (optional) → D (optional) → R

### Dota 2 Console Variable

The automation relies on this Dota 2 console variable:
```
cl_dota_alt_unit_movetodirection 1
```

When this is set to `1`, holding ALT and right-clicking makes your hero face that direction without moving. The script holds ALT during the right-click to trigger this behavior.

**Note:** You should have this set in your `autoexec.cfg`:
```
cl_dota_alt_unit_movetodirection 1
```

### GSI-Based Hero Detection

Shadow Fiend is automatically detected via GSI when you pick the hero:
- GSI reports `hero.name = "npc_dota_hero_nevermore"`
- The app automatically selects Shadow Fiend and enables raze interception
- No manual hero selection needed

### Survivability Actions

While playing SF, the script also provides common survivability features:
- **Danger detection** - Monitors for rapid HP loss
- **Auto healing items** - Uses Faerie Fire, Magic Wand, etc. when in danger
- **Defensive items** - Uses BKB, Satanic, Blade Mail, etc. when configured

## Configuration

### Config File (`config/config.toml`)

```toml
[heroes.shadow_fiend]
# Enable raze interception (ALT + right-click before Q/W/E)
raze_intercept_enabled = true
# Delay between right-click and raze key press (ms)
raze_delay_ms = 100
# Automatically use BKB before ultimate (Requiem of Souls) when pressing R
# Sequence: BKB (double-tap) → D (if enabled) → R
auto_bkb_on_ultimate = false
# Automatically press D (Aghanim's ability) before ultimate
auto_d_on_ultimate = false
# Standalone combo key: Blink + Ultimate (only executes if Blink is off cooldown)
standalone_key = "Home"
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `raze_intercept_enabled` | bool | `true` | Master toggle for raze interception |
| `raze_delay_ms` | u64 | `100` | Delay in milliseconds between facing and razing |
| `auto_bkb_on_ultimate` | bool | `false` | Auto-use BKB before Requiem of Souls |
| `auto_d_on_ultimate` | bool | `false` | Auto-press D (Aghanim's ability) before ultimate |
| `standalone_key` | string | `"Home"` | Key to trigger Blink + Ultimate combo |

### Tuning `raze_delay_ms`

- **Too low (< 50ms)**: Hero may not have finished turning before raze fires
- **Too high (> 200ms)**: Noticeable delay, feels sluggish
- **Recommended**: 80-120ms works well for most situations

## How It Works

### Execution Flow

```
┌─────────────────────────────────────────────────────────────┐
│                     User presses Q                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  keyboard.rs intercepts keypress                            │
│  Checks: sf_enabled && raze_intercept_enabled               │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  ShadowFiendState::execute_raze('q') spawns thread          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Wait 50ms (settle time)                                 │
│  2. Press and hold ALT                                      │
│  3. Right-click at cursor position                          │
│  4. Wait 50ms, release ALT                                  │
│  5. Wait raze_delay_ms (default 100ms)                      │
│  6. Press Q key                                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Raze fires in cursor direction                             │
└─────────────────────────────────────────────────────────────┘
```

### Key Files

| File | Purpose |
|------|---------|
| `src/actions/heroes/shadow_fiend.rs` | SF script and raze execution logic |
| `src/input/keyboard.rs` | Key interception and SF check |
| `src/input/simulation.rs` | ALT key hold, mouse click, key simulation |
| `src/config/settings.rs` | `ShadowFiendConfig` struct |
| `src/state/app_state.rs` | `sf_enabled` flag management |

## Usage

1. **Enable in config**: Ensure `raze_intercept_enabled = true`
2. **Set up Dota 2**: Add `cl_dota_alt_unit_movetodirection 1` to autoexec.cfg
3. **Start the app**: Run dota2-scripts
4. **Pick Shadow Fiend**: Hero is auto-detected via GSI
5. **Play normally**: Press Q/W/E to raze toward your cursor

## Combining with Soul Ring

If you have Soul Ring and the Soul Ring automation enabled, it will trigger Soul Ring before the raze automatically. The Q/W/E keys are in the default `ability_keys` list for Soul Ring interception.

## Troubleshooting

### Razes not firing toward cursor

1. Check that `cl_dota_alt_unit_movetodirection 1` is set in Dota 2
2. Verify Shadow Fiend is detected (check "Active Hero" in app UI)
3. Try increasing `raze_delay_ms` to 150-200ms

### Razes feel delayed

- Lower `raze_delay_ms` (try 50-80ms)
- Note: Too low may cause missed direction changes

### Raze interception not working at all

1. Run the app as Administrator (required for key interception on Windows)
2. Check that `raze_intercept_enabled = true` in config
3. Verify GSI is working (check event count in app UI)

## Limitations

- **Requires key interception**: The app must intercept keyboard input, which requires running as Administrator on Windows
- **Fixed Q/W/E keys**: Currently hardcoded to Q, W, E - doesn't support remapped ability keys
- **Turn rate dependent**: Very fast successive razes may not work if hero hasn't finished turning
