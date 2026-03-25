# Tiny Automation

## Purpose

Learn how the Tiny script executes a standalone burst combo with Blink, Avalanche, Toss, and Tree Grab.  
**Read this when:** configuring Tiny automation, understanding Soul Ring integration, debugging combo sequence.

## Feature Summary

- **Standalone combo trigger** – Press configured key to execute full combo sequence
- **Conditional Soul Ring** – First Avalanche press uses Soul Ring if available and conditions met
- **GSI-based item detection** – Requires stored GSI event for Blink Dagger slot lookup
- **Survivability actions** – Auto-use healing/defensive items
- **Ability spam** – Presses each ability multiple times to ensure cast

## Configuration

All settings in `config/config.toml` under `[heroes.tiny]`:

```toml
[heroes.tiny]
# Standalone combo key to execute Blink → Avalanche → Toss → Tree Grab
standalone_key = "Home"
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `standalone_key` | string | `"Home"` | Key to trigger standalone combo sequence |

**Soul Ring configuration** (see `docs/features/soul-ring.md`):

Tiny's combo automatically uses Soul Ring before the first Avalanche (W) press if:
- Soul Ring is in inventory and off cooldown
- Hero mana is below `min_mana_percent` (current config: 100%)
- Hero health is above `min_health_percent` (default: 20%)

## Related Files

| File | Purpose |
|------|---------|
| `src/actions/heroes/tiny.rs` | Tiny script and combo execution |
| `src/actions/soul_ring.rs` | `press_ability_with_soul_ring()` helper function |
| `src/config/settings.rs` | `TinyConfig` struct |
| `config/config.toml` | User configuration |
| `docs/features/soul-ring.md` | Soul Ring automation details |

---

## Details

### 🗿 Standalone Combo Sequence

Press the standalone key (default: `Home`) to execute the full Tiny burst combo.

**Requirements:**
- At least one GSI event received (for item slot detection)
- If no GSI event yet, logs warning and does nothing

**Combo sequence:**

1. **Blink Dagger** (if present in inventory)
   - Looks up Blink via `find_item_slot()`
   - Single press at cursor position
   - 100ms delay

2. **Avalanche (W)** – with Soul Ring
   - First press: `press_ability_with_soul_ring('w', &settings)`
     - Checks Soul Ring conditions (see `docs/features/soul-ring.md`)
     - If conditions met: presses Soul Ring key → 30ms delay → presses W
     - If not met: just presses W
   - Then spams W 3 more times (30ms between presses)
   - 50ms delay after

3. **Toss (Q)** – spam to ensure cast
   - Presses Q 4 times (30ms between presses)
   - 1400ms delay (wait for projectile)

4. **Tree Grab (D)** – Aghanim's Shard/Scepter ability
   - Presses D 3 times (30ms between presses)

**Total combo duration:** ~1.8 seconds

### 💍 Soul Ring Integration

The first Avalanche (W) press uses the `press_ability_with_soul_ring()` helper, which:

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

### 🎯 Ability Spam Logic

Each ability is pressed **multiple times** in rapid succession:
- **Avalanche (W)**: 4 presses total (1 with Soul Ring helper + 3 spam)
- **Toss (Q)**: 4 presses
- **Tree Grab (D)**: 3 presses

This ensures the ability casts even if:
- The hero is still turning
- The previous ability animation hasn't finished
- There's slight input lag

The delays between presses (30ms) are short enough to feel instant but long enough to register each press.

### 📦 GSI Event Requirement

The combo needs the latest GSI event to:
- Look up Blink Dagger slot via `find_item_slot(event, &settings, Item::Blink)`
- Check Soul Ring availability and conditions

**Storage:** The script stores the latest GSI event in `LAST_GSI_EVENT` (lazy_static Mutex):

```rust
lazy_static! {
    static ref LAST_GSI_EVENT: Mutex<Option<GsiWebhookEvent>> = Mutex::new(None);
}
```

Every `handle_gsi_event()` call updates this storage. When the standalone trigger is pressed, it reads from this storage.

**If no GSI event yet:**
```
No GSI event received yet - Tiny combo needs item data
```

In practice, this only happens if you press the combo key immediately after launching the app, before picking Tiny in-game.

### 🛡️ Survivability Actions

Tiny uses the common `SurvivabilityActions` system:
- **Healing items** – Magic Wand, Faerie Fire, etc.
- **Defensive items** – BKB, Blade Mail when in danger
- **Neutral items** – Witchbane, Safety Bubble
- **Danger detection** – Monitors HP changes

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
│  Blink (if in inventory)  │   │ Log warning, do nothing   │
│  Wait 100ms               │   └───────────────────────────┘
└───────────────────────────┘
              │
              ▼
┌───────────────────────────┐
│  Avalanche (W)            │
│  - Soul Ring helper       │
│  - Spam W 3x (30ms each)  │
│  Wait 50ms                │
└───────────────────────────┘
              │
              ▼
┌───────────────────────────┐
│  Toss (Q)                 │
│  - Spam Q 4x (30ms each)  │
│  Wait 1400ms              │
└───────────────────────────┘
              │
              ▼
┌───────────────────────────┐
│  Tree Grab (D)            │
│  - Spam D 3x (30ms each)  │
└───────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Combo complete                            │
└─────────────────────────────────────────────────────────────┘
```

### Usage

1. **Pick Tiny** in-game (auto-detected via GSI as `npc_dota_hero_tiny`)
2. **Equip Blink Dagger** (optional but recommended)
3. **Level Avalanche (W) and Toss (Q)** – core combo abilities
4. **Get Aghanim's Shard/Scepter** (optional, for Tree Grab/D)
5. **Position cursor** on enemy or desired location
6. **Press standalone key** (default: Home)
7. **Combo executes** automatically

### Tuning

- **Soul Ring usage**: Adjust `min_mana_percent` in `[soul_ring]` config (higher = more frequent Soul Ring use)
- **Combo key**: Change `standalone_key` in `[heroes.tiny]`

### Limitations

- **Blink detection only**: Combo does not check if Blink is off cooldown before using (unlike Shadow Fiend standalone)
- **Fixed ability keys**: Assumes Q/W/D keybindings (does not read in-game keybindings)
- **No targeting**: Blink and abilities target cursor position or default target
- **Manual aim required**: You must position cursor before pressing combo key
