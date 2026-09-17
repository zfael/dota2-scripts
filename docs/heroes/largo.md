# Largo Automation

## Purpose

Learn how the Largo hero script automates **Amphibian Rhapsody** (ultimate) with precise beat-timing and song management.  
**Read this when:** implementing or debugging Largo automation, tuning beat timings, understanding GSI-based ultimate detection.

## Feature Summary

- **GSI-based ultimate detection** – Monitors `largo_song_*` ability names to auto-enable ultimate mode
- **Song-key interception** – While the ultimate is up, a song press is swallowed and played on the beat instead of reaching Dota off-beat
- **Beat timing system** – Absolute anchor timing with periodic corrections to prevent drift
- **Song queuing** – Switches songs on next beat to preserve rhythm; the last press before a beat wins
- **Aghanim's Scepter support** – Plays two songs on the same beat; a third press evicts the one pressed longest ago
- **Ultimate key handling** – Passes through to Dota, but stops the beat loop on the press rather than a GSI tick later
- **Survivability actions** – Auto-use healing/defensive items when HP is low

## Configuration

All settings in `config/config.toml` under `[heroes.largo]`:

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
standalone_key = "Home"         # Field exists in config, but current runtime does not read a Largo-specific value
```

**Important runtime note:** `heroes.largo.standalone_key` exists in config, but the current runtime does not wire Largo through `Settings::get_standalone_key(...)`. Manual Largo control flows through the shared `AppState.trigger_key` plus the song-key intercept in `src/input/keyboard.rs`.

The ability keys above are **read by the intercept**, not just by the beat worker. Rebinding `w_ability_key` moves which physical key Hotfeet Hustle is captured from; nothing is hardcoded to `Q/W/E/R` any more.

### Tuning Beat Timing

| Problem | Solution |
|---------|----------|
| Clicks happen **too early** | Increase `beat_correction_ms` (positive value) |
| Clicks happen **too late** | Decrease `beat_correction_ms` (negative value) |
| Drift happens too quickly | Decrease `beat_correction_every_n_beats` |
| Drift happens too slowly | Increase `beat_correction_every_n_beats` |
| Disable correction entirely | Set `beat_correction_every_n_beats = 0` |

## Related Files

| File | Purpose |
|------|---------|
| `src/actions/heroes/largo.rs` | Song schedule, beat worker, `ULTIMATE_ACTIVE`, `queue_song`, `stop_ultimate` |
| `src/input/keyboard.rs` | `plan_largo_key_action` — which presses are swallowed, and which pass through |
| `examples/largo_song_sync_control.rs` | Offline proof of the intercept's timing; re-run it after a patch changes the beat |
| `src/config/settings.rs` | LargoConfig struct and defaults |
| `config/config.toml` | User configuration |

---

## Details

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

Songs are selected via the configured ability keys (Dota defaults shown):

| Key | Song | Effect |
|-----|------|--------|
| Q | Bullbelly Blitz | Damage |
| W | Hotfeet Hustle | Movement Speed |
| E | Island Elixir | Healing |

#### The intercept

**While the ultimate is up, a song key never reaches Dota.** The keyboard hook
blocks it and records which song you asked for; the beat worker presses it on the
grid.

That is the whole point. A song cast between beats is cast off the beat, and with
Aghanim's it is also off the *other* song of the pair — the desync the rest of
this page is about. Passing the press through and *also* queuing it, which is
what the script did before, produced both: one off-beat cast from your finger and
one on-beat cast from the worker.

Outside the ultimate the song keys are ordinary abilities and pass through
untouched. That includes the window between pressing the ultimate key and GSI
confirming the cast (~226ms, one tick): during it the hook has no reason to
believe the ultimate is up, so it does not swallow anything. **A song pressed in
that window starts the song in-game but does not start the beat loop.** Press it
again once the ultimate registers and the loop picks it up.

Soul Ring deliberately does not see a swallowed press either — replaying the key
a few hundred milliseconds later is the same desync by another route.

#### Song Queuing

When switching songs mid-ultimate, the new song is **queued** and applied on the
next beat. This maintains perfect rhythm instead of resetting timing:

1. You're playing Bullbelly (Q)
2. At T=400ms, you press W — the press is swallowed, `pending_song = Hotfeet`
3. At T=995ms (next beat), the switch happens and W is pressed
4. Rhythm continues uninterrupted

Two rules fall out of holding the press rather than passing it on:

- **The last press before a beat is the one that lands.** Pressing W then E in
  the same beat queues E; W never plays. Changing your mind is free.
- **Pressing the song already playing cancels a queued switch.** Playing Q,
  pressing W, then pressing Q again keeps you on Q. Previously the W still
  landed on the next beat.

### Aghanim's Scepter Support

With Aghanim's Scepter, you play **two songs at once**. The script detects the
scepter via GSI and presses both keys on the same beat: `current_song` first,
then `previous_song` one synthetic press later (~10ms, the
`POST_ACTION_GUARD_DELAY_MS` the input worker pays per key).

The pair is **the last two distinct songs you pressed, newest first**, so a third
press evicts the one pressed longest ago:

| You press | Playing | Dropped |
|---|---|---|
| Q | Q | — |
| then W | W + Q | — |
| then E | E + W | Q |

Losing or gaining the scepter mid-game clears `previous_song`. Gaining it never
starts a song you did not press, and losing it never leaves a stale one queued
behind the current one.

### Ultimate Key Handling

When you press the ultimate key to end Amphibian Rhapsody:

1. The press **reaches Dota** — it is what actually ends the ultimate
2. The beat loop **stops on the press**, not when GSI confirms
3. All state is cleared (songs, pending, beat count)

Stopping on the press is what prevents song presses during the ~226ms between
your key and GSI's confirmation. The ultimate key is matched before the song keys,
so a config that binds a song to it cannot make the ultimate unstoppable.

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
│          Song keys are now swallowed by the hook             │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ song key swallowed (first song)
                              │ → anchors the beat grid, fires now
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    ACTIVE (Playing)                          │
│         (active = true, current_song = Some(song))           │
│   Scheduled worker waits for beat deadline or wake-up        │
│                                                              │
│   ┌─────────────────────────────────────────────────────┐   │
│   │  song key swallowed → pending_song = new_song       │   │
│   │  same song again    → pending_song = None           │   │
│   │  Next beat → switch current_song, press new key     │   │
│   └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ ultimate key pressed (passes through)
                              │ OR GSI detects normal abilities
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                         INACTIVE                             │
│    (all state reset: songs, beat_count, beat_start_time)     │
└─────────────────────────────────────────────────────────────┘
```

### Scheduled Beat Worker

A dedicated Largo worker uses a timed wait instead of polling every few milliseconds.

1. If ultimate is inactive or no song is selected yet, it blocks until a state change wakes it
2. Once a song is active, it calculates the next beat deadline from the absolute anchor time plus periodic correction offsets
3. It then sleeps until either:
   - the next beat deadline arrives, or
   - a state-change wake-up happens (for example: first song selected, a queued song switch, config snapshot refresh, or ultimate stop)
4. When the beat deadline is due:
   - Process `pending_song` if present (switch songs on the beat)
   - Press `current_song`
   - Press `previous_song` after it when Aghanim's Scepter is active
   - Increment `beat_count`

The first manual song selection still wakes the worker so the first beat fires immediately, and the ultimate key still clears the schedule immediately so no stale song presses happen while waiting for GSI confirmation.

**The state lock is never held across a key press.** `crate::input::press_key`
blocks the caller until the synthetic input worker is done, including
`POST_ACTION_GUARD_DELAY_MS`. The keyboard hook calls `queue_song` on that same
lock, so holding it there would stall the global hook — and therefore every
keystroke in the game — for the length of a beat's emission.

### Re-measuring after a patch

`cargo run --example largo_song_sync_control` replays presses at chosen offsets
against the beat grid and asserts what reaches Dota. It covers the ugly cases: a
press arriving mid-emission, a stop landing between the two halves of an
Aghanim's pair, the GSI confirmation window, and thirty beats of drift. Run it
after any gameplay patch that changes the beat, and after any edit to the
schedule or the hook planner. Add `--verbose` for an event-by-event trace.
