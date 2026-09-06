# Morphling Automation

## Purpose

Learn how the Morphling script shifts attributes for you: strength when a fight
starts, agility again when it is over.
**Read this when:** tuning how much strength a fight buys, deciding whether the
shift back should be automatic, or working out why a shift did or did not fire.

## Feature Summary

- **Strength on danger** – A fight buys a configured amount of max HP, then stops
- **Fresh fights buy more** – A new burst shifts again, even one that arrives before the shift back was due
- **Gated shift back** – Agility is restored only after a quiet period *and* above a health floor
- **Hands off when you are driving** – A shift you start yourself stands the automation down
- **No false danger** – The HP your own shift costs is declared, so it never reads as a gank
- **GSI-based detection** – Auto-enables when `npc_dota_hero_morphling` is detected
- **Survivability actions** – Auto-use healing/defensive items

## Configuration

All settings in `config/config.toml` under `[heroes.morphling]`:

```toml
[heroes.morphling]
enabled = true
strength_key = "f"
agility_key = "d"
target_hp_gain = 330
return_to_agility = true
return_delay_seconds = 5
return_min_health_percent = 70
max_shift_seconds = 4
plateau_ms = 700
press_settle_ms = 500
manual_override_seconds = 5
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `true` | Master toggle for the automatic shift |
| `strength_key` | char | `"f"` | Attribute Shift (Strength Gain) key — must match your in-game binding |
| `agility_key` | char | `"d"` | Attribute Shift (Agility Gain) key |
| `target_hp_gain` | u32 | `330` | Max HP to buy when a fight starts. 22 HP is one strength point on the current patch, so this is ~15 points |
| `return_to_agility` | bool | `true` | Whether to shift back automatically at all |
| `return_delay_seconds` | u64 | `5` | How long danger must stay clear before shifting back |
| `return_min_health_percent` | u32 | `70` | Never hand strength back below this much health |
| `max_shift_seconds` | u64 | `4` | Hard cap on any single shift |
| `plateau_ms` | u64 | `700` | How long `max_health` must stand still before the shift counts as finished |
| `press_settle_ms` | u64 | `500` | How long a press needs before its effect is believable |
| `manual_override_seconds` | u64 | `5` | How long your own shift keeps the automation out of the loop |

## Related Files

| File | Purpose |
|------|---------|
| `src/actions/heroes/morphling.rs` | Morphling script and the shift controller |
| `src/config/settings.rs` | `MorphlingConfig` struct and defaults |
| `config/config.toml` | User configuration |
| `src-ui/src/components/heroes/configs/MorphlingConfig.tsx` | Config panel under **Heroes → Morphling** |
| `examples/morphling_shift_probe.rs` | GSI probe that measured what a shift looks like |
| `examples/morphling_shift_control.rs` | Simulator the decision loop was proved against |

---

## Details

### 🌊 Reading the shift from `max_health`

GSI has no field that says which shift is running.
`abilities.abilityN.ability_active` looks like the answer and is not: a live
capture reports it `true` for every ability on every tick, including
`plus_guild_banner` and `plus_high_five`.

What does work is the side effect. Strength gain raises `hero.max_health`,
agility gain lowers it, and the probe measured that cleanly across 12 shift
episodes:

| Measured | Value |
|---|---|
| GSI tick interval | 226ms median (211–313) |
| Shift rate | ~345 HP/s ≈ 15.6 attribute points/s |
| HP per attribute point | 22 — every observed step was a multiple of it |
| Key press → first visible movement | 95–391ms |
| Same key pressed again | stops the shift (~180ms) |
| Opposite key | reverses it, no stop press needed |
| Exhausted pool | `max_health` simply plateaus |
| Full pool at the captured build | 4378 HP ≈ 199 attribute points |

**None of those numbers are in the code.** They describe the game the logic was
tested against, not the rules it follows. Every threshold is expressed in HP or
seconds, and the "how far will it move before the stop lands" estimate is the
last delta actually observed — so a patch that changes the rate, the HP per
point, or the tick cadence changes nothing.

To re-measure after a patch, see the capture protocol in the header of
`examples/morphling_shift_probe.rs`.

### ⚔️ What a fight buys

**Trigger model:** passive, GSI-driven. Runs on every `handle_gsi_event()`.

When `danger_detector` fires, the script presses `strength_key` and fixes a
target at `max_health + target_hp_gain`. It stops when the *projected* next tick
would cross that target — a tick early, because the stop press lands about a
tick late.

The distinction that matters:

- **The same fight going on** does not keep buying. The target is fixed when the
  fight starts, so one long teamfight cannot ratchet into a full shift.
- **A new fight** — danger cleared, then fired again — always buys another
  slice, even if it arrives before the shift back was due.

If the strength pool runs out first, `max_health` plateaus, and the script logs
it and stops deciding rather than pressing at a ceiling that cannot move.

### 🔙 Shifting back

On by default, and gated twice:

1. Danger has to have been clear for `return_delay_seconds`. This is deliberately
   separate from `[danger_detection].clear_delay_seconds`: a lull in the damage
   is not the end of the fight, and this is the timeout that covers standing in
   one.
2. Health has to be at or above `return_min_health_percent`. Losing max HP costs
   current HP, so shifting back while hurt is how a Morphling dies.

Danger returning mid-return reverses it with a **single** press — the opposite
key cancels a running shift, no stop press needed.

The shift back aims at the `max_health` the whole sequence started from, so
however many fights it takes, "back to normal" means the build you set. The
lookahead is deliberately asymmetric — one tick going up, two coming back — so
both errors land on the side of more strength, and it never shifts *past* your
baseline into agility you never had.

### 🎮 When you take over

There is no keyboard hook. A shift moving while the script believes nothing of
its own is running means you are on the toggles, so it stands down for
`manual_override_seconds` — and keeps standing down for as long as your shift is
still moving, rather than cutting back in mid-shift.

Two consecutive moving ticks are required. One alone is an item, a level, or a
Power Treads swap, which move `max_health` once and stop.

### 🩸 Not reading your own shift as a gank

Handing strength back costs current HP, and `danger_detector` reads HP
disappearing as incoming damage — the same bug Soul Ring had (see
`docs/features/danger-detection.md`). Every `max_health` drop is therefore
declared with `danger_detector::note_self_damage()` *before* the detector sees
the event.

This runs whether or not `enabled` is set: a shift you do by hand produces
exactly the same false reading. Only the health actually lost is claimed, never
the whole `max_health` drop, so real damage landing in the same window still
registers.

### 🛡️ Survivability Actions

Morphling uses the common `SurvivabilityActions` system:

- **Healing items** – Magic Wand, Faerie Fire, Satanic, etc.
- **Defensive items** – BKB, Lotus Orb, Blade Mail when in danger
- **Neutral items** – on the shared danger trigger
- **Danger detection** – the same HP-delta detector that drives the shift

These share the global `[common]`, `[danger_detection]`, and `[neutral_items]`
config sections.

### 🔄 State Diagram

```
┌──────────┐  danger fires        ┌──────────────────────────────┐
│   Idle   │ ───────────────────► │  Gaining                     │
│          │                      │  press strength, watch max_hp│
└──────────┘                      └──────────────┬───────────────┘
     ▲                                           │ target reached (press to stop)
     │                                           │ or pool empty (no press)
     │                                           ▼
     │  back at baseline        ┌──────────────────────────────┐
     │  (press to stop)         │  Holding                     │
     │                          │  keep it while the fight runs│
     │                          └──────────────┬───────────────┘
     │                                         │ danger clear for return_delay_seconds
     │                                         │ AND health >= return_min_health_percent
┌────┴─────────────────────────┐               │
│  Returning                   │ ◄─────────────┘
│  press agility, watch max_hp │
└──────────────────────────────┘
        │ danger returns → single press reverses straight back to Gaining
        ▼
```

A new danger edge re-enters `Gaining` from **any** state.

### 🔒 Thread Safety

The controller is a single process-wide `Mutex<ShiftController>`, advanced once
per GSI event. Key presses are handed to the shared `ActionExecutor` rather than
sent inline, so a press never blocks the GSI event loop.

### Usage

1. **Pick Morphling** in-game (auto-detected via GSI)
2. **Check your bindings** match `strength_key` / `agility_key`
3. **Run the app** – the hero is auto-detected
4. **Fight** – the shift fires on the danger detector

### Tuning

- **Still dying through the shift?** Raise `target_hp_gain`. It is in HP: +660
  is roughly 30 strength, about two seconds of shifting.
- **Losing too much damage?** Lower `target_hp_gain`, or set
  `return_delay_seconds` shorter so agility comes back sooner.
- **Shifting back too early, mid-fight?** Raise `return_delay_seconds`. This is
  the knob for "not taking damage right now" versus "the fight is over".
- **Want the shift back by hand?** `return_to_agility = false`. The script will
  still shift *to* strength on danger.
- **Do not lower `press_settle_ms`** without a fresh capture. A press inside that
  window stops the shift instead of adjusting it.

### Logging

With `level = "info"`:

```
🌊 Morphling strength shift (f): danger — shifting to strength — max_hp 2000
🌊 Morphling strength shift (f): target reached — stopping — max_hp 2308
🌊 Morphling agility shift (d): quiet — shifting back to agility — max_hp 2374
🌊 Morphling agility shift (d): back to baseline — stopping — max_hp 2066
```

With `level = "debug"`:

```
🌊 Morphling: max_health stopped moving — strength pool is empty
🌊 Morphling: shift moving on its own — the player has the toggles
🌊 Morphling: new danger, but already at full strength
💢 Self-inflicted 88HP from Attribute Shift - the next HP drop is discounted by that much
```

### Limitations

- **Keys are pressed, not read** – the script cannot see your in-game bindings,
  so `strength_key` / `agility_key` must be set to match
- **Detection lags by a tick** – roughly 230ms before a shift is visible, which
  is Dota's delivery rate, not something the app can shorten
- **Only Attribute Shift** – Waveform, Adaptive Strike and Replicate are not
  automated
- **Manual shifts are inferred, not observed** – detection takes two moving
  ticks (~0.5s), so a very short manual tap may not register before the
  automation acts

---

## Maintenance Checklist

When editing this hero's code, update this doc:

- [ ] New config option added? → Update Configuration table
- [ ] New behavior/feature? → Add section under Details
- [ ] Changed the stop/target logic? → Update "What a fight buys"
- [ ] Modified trigger model? → Update trigger description
- [ ] Changed state tracking? → Update state diagram
- [ ] New logging statements? → Update Logging section
- [ ] Re-measured after a patch? → Update the measured-values table
