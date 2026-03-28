# Shadow Fiend (Nevermore) Automation

## Purpose

Learn how the Shadow Fiend script remaps razes to face cursor direction and auto-BKB on ultimate.  
**Read this when:** implementing or debugging SF automation, tuning raze delay, understanding direction-facing mechanics.

## Feature Summary

- **Automatic direction facing** – Intercepts Q/W/E, enqueues one request onto a dedicated Shadow Fiend worker, then faces cursor and razes
- **Auto-BKB on ultimate** – Intercepts R, enqueues the combo onto the same worker, and uses BKB before Requiem of Souls when enabled
- **Standalone combo implementation** – Blink + BKB + D + Ultimate exists in code, and the current standalone-key conflict remains unchanged in this slice
- **GSI-based hero detection** – Automatically enables when `npc_dota_hero_nevermore` detected
- **Survivability actions** – Auto-use healing/defensive items

## Configuration

All settings in `config/config.toml` under `[heroes.shadow_fiend]`:

```toml
[heroes.shadow_fiend]
# Enable raze interception (ALT + right-click before Q/W/E)
raze_intercept_enabled = true
# Delay between right-click and raze key press (ms)
raze_delay_ms = 10
# Automatically use BKB before ultimate (Requiem of Souls) when pressing R
# Sequence: BKB (double-tap) → D (if enabled) → R
auto_bkb_on_ultimate = true
# Automatically press D (Aghanim's ability) before ultimate
auto_d_on_ultimate = true
# Checked-in config value; current runtime does not honor this field directly
standalone_key = "Home"
```

**Important runtime note:** `config/config.toml` still exposes `standalone_key`, but `Settings::get_standalone_key("shadow_fiend")` currently hardcodes `"q"` instead of reading that value. With `raze_intercept_enabled = true`, the keyboard hook also intercepts `Q` earlier for razes, so the standalone combo path is effectively in conflict with the raze path unless code changes or interception is disabled. That conflict is unchanged and out of scope for this worker-offload slice.

### Tuning `raze_delay_ms`

- **Too low (< 50ms)**: Hero may not have finished turning before raze fires
- **Too high (> 200ms)**: Noticeable delay, feels sluggish
- **Recommended**: 80-120ms works well for most situations

## Related Files

| File | Purpose |
|------|---------|
| `src/actions/heroes/shadow_fiend.rs` | SF script and raze execution logic |
| `src/input/keyboard.rs` | Key interception and SF check |
| `src/input/simulation.rs` | ALT key hold, mouse click, key simulation |
| `src/config/settings.rs` | `ShadowFiendConfig` struct |

---

## Details

### Automatic Direction Facing

Shadow Fiend's Shadowraze abilities (Q/W/E) raze in the direction the hero is facing, not the cursor direction. This can be awkward because you need to right-click to face a direction first, then press the raze key.

The automation solves this by intercepting Q/W/E keypresses and automatically:
1. **Holding ALT** to enable `cl_dota_alt_unit_movetodirection` (move-to-direction on right-click)
2. **Right-clicking** to face the cursor direction
3. **Pressing the raze key** after a configurable delay

This allows you to raze toward your cursor naturally, similar to how most skillshots work.

When you press Q, W, or E with Shadow Fiend selected:

1. The keypress is **intercepted** (blocked from reaching the game)
2. The intercept enqueues one raze request onto Shadow Fiend's dedicated worker
3. That worker uses `src/input/simulation.rs` to hold ALT (enables move-to-direction mode)
4. The worker simulates a right-click to face the cursor
5. The worker releases ALT
6. After a short delay, the worker presses the raze key through `src/input/simulation.rs`

This happens in ~150-200ms total, making it feel nearly instant.

### Auto-BKB on Ultimate

When enabled, pressing R to cast Requiem of Souls will automatically:

1. **Intercept R** and enqueue one ultimate request onto Shadow Fiend's dedicated worker
2. **Check for BKB** in your inventory (if `auto_bkb_on_ultimate` is enabled)
3. **Use BKB** (double-tap for self-cast) if available and off cooldown
4. **Press D** (Aghanim's ability) if `auto_d_on_ultimate` is enabled
5. **Cast Requiem of Souls** (R)

`src/input/simulation.rs` still owns the actual synthetic key emission for the worker's BKB/D/R sequence. This ensures you're protected by magic immunity during the channel without needing to manually activate BKB first.

**Sequence:** BKB (double-tap) → D (optional) → R

**Behavior when BKB is not present or on cooldown:** The script will skip BKB activation and proceed with D (if enabled) and R. If `auto_bkb_on_ultimate` is disabled entirely, pressing R will just press R normally with no interception.

### Standalone Combo (Blink + Ultimate)

The script has a standalone combo implementation that will:

1. **Check if Blink is available** (not on cooldown)
2. If Blink is on cooldown → **Skip the combo entirely** (no action taken)
3. If Blink is available:
   - **Blink** to cursor position
   - **BKB** (double-tap, if `auto_bkb_on_ultimate` enabled and available)
   - **D ability** (if `auto_d_on_ultimate` enabled)
   - **R** (Requiem of Souls)

This allows a full initiation combo in one trigger path, but the current runtime wiring is inconsistent: the checked-in config says `Home`, the helper returns `q`, and the `Q` raze intercept runs first when enabled.

This slice does not change that standalone-key conflict; it only moves intercepted Shadow Fiend work off per-intercept raw threads and onto one dedicated worker.

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
│  ShadowFiendState::execute_raze('q') enqueues worker request │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Dedicated SF worker drains the request                     │
│  1. Wait 50ms (settle time)                                 │
│  2. Use simulation.rs to press and hold ALT                 │
│  3. Use simulation.rs to right-click at cursor position     │
│  4. Wait 50ms, use simulation.rs to release ALT             │
│  5. Wait raze_delay_ms (default 100ms)                      │
│  6. Use simulation.rs to press Q key                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Raze fires in cursor direction                             │
└─────────────────────────────────────────────────────────────┘
```

### Usage

1. **Enable in config**: Ensure `raze_intercept_enabled = true`
2. **Set up Dota 2**: Add `cl_dota_alt_unit_movetodirection 1` to autoexec.cfg
3. **Start the app**: Run dota2-scripts
4. **Pick Shadow Fiend**: Hero is auto-detected via GSI
5. **Play normally**: Press Q/W/E to raze toward your cursor

### Combining with Soul Ring

If you have Soul Ring and Soul Ring automation enabled, note the current ordering caveat: Shadow Fiend's `Q/W/E` raze interception runs earlier than the generic Soul Ring replay branch, so Soul Ring does **not** currently prefire before razes while raze interception is active.

### Troubleshooting

#### Razes not firing toward cursor

1. Check that `cl_dota_alt_unit_movetodirection 1` is set in Dota 2
2. Verify the latest GSI payload shows Shadow Fiend in the UI and that `Active Hero` is Shadow Fiend when using the `HeroType` flow
3. Try increasing `raze_delay_ms` to 150-200ms

#### Razes feel delayed

- Lower `raze_delay_ms` (try 50-80ms)
- Note: Too low may cause missed direction changes

#### Raze interception not working at all

1. Run the app as Administrator (required for key interception on Windows)
2. Check that `raze_intercept_enabled = true` in config
3. Verify GSI is working (check event count in app UI)

### Limitations

- **Requires key interception**: The app must intercept keyboard input, which requires running as Administrator on Windows
- **Standalone trigger drift**: `heroes.shadow_fiend.standalone_key` is present in config, but the current runtime hardcodes `"q"` and that conflicts with raze interception
- **Turn rate dependent**: Very fast successive razes may not work if hero hasn't finished turning
