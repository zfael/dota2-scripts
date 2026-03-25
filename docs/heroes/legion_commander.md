# Legion Commander Automation

## Purpose

Learn how the Legion Commander script executes a standalone Duel combo with Press The Attack, items, and abilities.  
**Read this when:** configuring Legion Commander automation, understanding Soul Ring integration, debugging combo flow.

## Feature Summary

- **Standalone combo trigger** – Press configured key to execute full Duel combo sequence
- **Conditional Soul Ring** – First Press The Attack uses Soul Ring if available and conditions met
- **Item orchestration** – Blade Mail, Mjollnir, BKB, Blink, Orchid/Bloodthorn sequencing
- **GSI-based detection** – Requires stored GSI event for item slot lookups
- **Survivability actions** – Auto-use healing/defensive items
- **Linkens removal** – Orchid/Bloodthorn spam (10 presses) to break Linken's Sphere

## Configuration

All settings in `config/config.toml` under `[heroes.legion_commander]`:

```toml
[heroes.legion_commander]
# Standalone combo key to execute full Duel sequence
standalone_key = "Home"
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `standalone_key` | string | `"Home"` | Key to trigger standalone combo sequence |

**Soul Ring configuration** (see `docs/features/soul-ring.md`):

Legion Commander's combo automatically uses Soul Ring before the first Press The Attack (W) if:
- Soul Ring is in inventory and off cooldown
- Hero mana is below `min_mana_percent` (current config: 100%)
- Hero health is above `min_health_percent` (default: 20%)

## Related Files

| File | Purpose |
|------|---------|
| `src/actions/heroes/legion_commander.rs` | Legion Commander script and combo execution |
| `src/actions/soul_ring.rs` | `press_ability_with_soul_ring()` helper function |
| `src/config/settings.rs` | `LegionCommanderConfig` struct |
| `config/config.toml` | User configuration |
| `docs/features/soul-ring.md` | Soul Ring automation details |

---

## Details

### ⚔️ Standalone Combo Sequence

Press the standalone key (default: `Home`) to execute the full Legion Commander Duel combo.

**Requirements:**
- Stored GSI event available (for item slot detection)
- If no GSI event, logs message and does nothing

**Combo sequence:**

1. **Press The Attack (W)** – with Soul Ring
   - First press: `press_ability_with_soul_ring('w', &settings)`
     - Checks Soul Ring conditions (see `docs/features/soul-ring.md`)
     - If conditions met: presses Soul Ring → 30ms delay → presses W
     - If not met: just presses W
   - Then presses W again (double-tap for self-cast)
   - 220ms delay

2. **Blade Mail** (if present) – double-tap
   - Checks inventory via `find_item_slot(event, &settings, Item::BladeMail)`
   - Double-tap: press key → 30ms → press key again
   - 50ms delay

3. **Mjollnir** (if present) – double-tap
   - Checks inventory via `find_item_slot()`
   - Double-tap: press key → 30ms → press key again
   - 50ms delay

4. **BKB (Black King Bar)** (if present) – double-tap
   - Checks inventory via `find_item_slot(event, &settings, Item::BlackKingBar)`
   - Double-tap: press key → 30ms → press key again
   - 50ms delay

5. **Blink Dagger** (if present) – single press
   - Checks inventory via `find_item_slot(event, &settings, Item::Blink)`
   - Single press at cursor position
   - 100ms delay

6. **Orchid Malevolence or Bloodthorn** (if present) – spam 10x
   - Checks for either Orchid or Bloodthorn via `find_item_slot()`
   - Spams key 10 times (30ms between presses) to break Linken's Sphere
   - 50ms delay

7. **Duel (R)** – spam 6x
   - Presses R 6 times (50ms between presses)

8. **Overwhelming Odds (Q)** – spam 6x
   - Presses Q 6 times (50ms between presses)

**Total combo duration:** ~1.5 seconds

### 💍 Soul Ring Integration

The first Press The Attack (W) press uses the `press_ability_with_soul_ring()` helper, which:

1. Checks if Soul Ring is available in inventory (via latest GSI event)
2. Verifies Soul Ring is off cooldown and hero is alive
3. Checks mana threshold: `hero.mana_percent < min_mana_percent`
4. Checks health threshold: `hero.health_percent > min_health_percent`
5. If all conditions pass:
   - Presses Soul Ring item key
   - Waits 30ms
   - Presses W
6. If conditions fail, just presses W

**Configuration** (in `[soul_ring]` section):
```toml
[soul_ring]
enabled = true
min_mana_percent = 100      # Only use if mana < 100%
min_health_percent = 20     # Don't use if health <= 20%
delay_before_ability_ms = 30
```

See `docs/features/soul-ring.md` for full Soul Ring documentation.

### 🛡️ Item Ordering

Items are used in a specific order to maximize effectiveness:

1. **Press The Attack (W) first** – Heal and attack speed buff before engaging
2. **Blade Mail** – Damage reflection active
3. **Mjollnir** – Active attack speed and chain lightning
4. **BKB** – Magic immunity before Blink (prevents interruption)
5. **Blink** – Gap closer
6. **Orchid/Bloodthorn** – Silence and damage amplification (with Linken's break)
7. **Duel (R)** – Lock target
8. **Overwhelming Odds (Q)** – Nuke after Duel starts

All **toggle/self-cast items** (Blade Mail, Mjollnir, BKB, Press The Attack) use **double-tap** to ensure self-cast, then engage with Blink.

### 🔗 Linken's Sphere Removal

**Orchid/Bloodthorn is spammed 10 times** (30ms between presses) to:
- Break Linken's Sphere on the first cast
- Apply silence/damage amp on subsequent casts
- Ensure the debuff lands even if first cast is blocked

This is crucial against heroes with Linken's Sphere, as a single Orchid cast would be blocked and waste the combo.

### 📦 GSI Event Requirement

The combo needs the latest GSI event to:
- Look up item slots via `find_item_slot(event, &settings, item)`
- Check Soul Ring availability

**Storage:** The script stores the latest GSI event in `last_event` (Arc<Mutex>):

```rust
pub struct LegionCommanderScript {
    settings: Arc<Mutex<Settings>>,
    last_event: Arc<Mutex<Option<GsiWebhookEvent>>>,
}
```

Every `handle_gsi_event()` call updates this storage. When the standalone trigger is pressed, it reads from this storage.

**If no GSI event yet:**
```
No GSI event available, cannot determine item slots
```

### 🛡️ Survivability Actions

Legion Commander uses the common `SurvivabilityActions` system:
- **Healing items** – Magic Wand, Faerie Fire, Satanic, etc.
- **Defensive items** – BKB, Lotus Orb, Blade Mail when in danger
- **Neutral items** – Witchbane, Safety Bubble
- **Danger detection** – Monitors HP changes and enemy abilities

These run passively on every GSI event, independent of the standalone combo.

### 🔄 Execution Flow

```
┌─────────────────────────────────────────────────────────────┐
│               User presses standalone key (Home)             │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│     Check: Is there a stored GSI event?                      │
└─────────────────────────────────────────────────────────────┘
                            │
              ┌─────────────┴─────────────┐
              │                           │
          Yes (event exists)         No (no event)
              │                           │
              ▼                           ▼
┌───────────────────────────┐   ┌───────────────────────────┐
│  Press The Attack (W)     │   │ Log warning, do nothing   │
│  - Soul Ring helper       │   └───────────────────────────┘
│  - Double-tap W           │
│  Wait 220ms               │
└───────────────────────────┘
              │
              ▼
┌───────────────────────────┐
│  Blade Mail (if present)  │
│  - Double-tap             │
│  Wait 50ms                │
└───────────────────────────┘
              │
              ▼
┌───────────────────────────┐
│  Mjollnir (if present)    │
│  - Double-tap             │
│  Wait 50ms                │
└───────────────────────────┘
              │
              ▼
┌───────────────────────────┐
│  BKB (if present)         │
│  - Double-tap             │
│  Wait 50ms                │
└───────────────────────────┘
              │
              ▼
┌───────────────────────────┐
│  Blink (if present)       │
│  - Single press           │
│  Wait 100ms               │
└───────────────────────────┘
              │
              ▼
┌───────────────────────────┐
│  Orchid/Bloodthorn        │
│  (if present)             │
│  - Spam 10x (30ms each)   │
│  Wait 50ms                │
└───────────────────────────┘
              │
              ▼
┌───────────────────────────┐
│  Duel (R)                 │
│  - Spam 6x (50ms each)    │
└───────────────────────────┘
              │
              ▼
┌───────────────────────────┐
│  Overwhelming Odds (Q)    │
│  - Spam 6x (50ms each)    │
└───────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Combo complete                            │
└─────────────────────────────────────────────────────────────┘
```

### Usage

1. **Pick Legion Commander** in-game (auto-detected via GSI as `npc_dota_hero_legion_commander`)
2. **Equip core items** (Blink, Blade Mail recommended)
3. **Optional items** – BKB, Mjollnir, Orchid/Bloodthorn for full combo
4. **Position cursor** on enemy target
5. **Press standalone key** (default: Home)
6. **Combo executes** automatically

### Tuning

- **Soul Ring usage**: Adjust `min_mana_percent` in `[soul_ring]` config (higher = more frequent Soul Ring use)
- **Combo key**: Change `standalone_key` in `[heroes.legion_commander]`

### Limitations

- **Fixed ability keys**: Assumes Q/W/R keybindings (does not read in-game keybindings)
- **No cooldown checks**: Combo does not verify if abilities/items are off cooldown before use
- **Manual targeting**: You must position cursor on target before pressing combo key
- **No Blink range check**: Combo attempts Blink regardless of distance to cursor
