# Soul Ring Automation

## Purpose

Learn how the Soul Ring item automation automatically triggers Soul Ring before ability or item usage to optimize mana efficiency.  
**Read this when:** configuring Soul Ring automation, debugging key interception, understanding safety checks.

## Feature Summary

- **Automatic Soul Ring triggering** – Intercepts ability/item keys and uses Soul Ring first
- **GSI-based detection** – Auto-enables when Soul Ring is in inventory
- **Safety checks** – Health and mana thresholds prevent wasted usage or suicide
- **Cooldown lockout** – Prevents double-fire on double-tap or rapid key presses
- **Mana-cost gating** – Only fires ahead of an ability or item that actually spends mana, priced from a generated table

## Configuration

All settings in `config/config.toml` under `[soul_ring]`:

```toml
[soul_ring]
# Master toggle for Soul Ring automation
enabled = true

# Only trigger Soul Ring if mana percent is below this threshold
min_mana_percent = 100

# Safety threshold - don't use Soul Ring if health percent is at or below this
min_health_percent = 20

# Delay in milliseconds between Soul Ring press and ability press
delay_before_ability_ms = 30

# Cooldown lockout in milliseconds to prevent double-fire on double-tap
trigger_cooldown_ms = 10

# Ability keys to intercept for Soul Ring triggering
ability_keys = ["q", "w", "e", "r", "d", "f"]

# Also trigger Soul Ring before item key presses (items that cost mana)
intercept_item_keys = true
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `true` | Master toggle for the feature |
| `min_mana_percent` | u32 | `100` in `config.toml` (`90` serde fallback) | Only trigger if mana is below this % |
| `min_health_percent` | u32 | `20` | Don't trigger if health is at or below this % |
| `delay_before_ability_ms` | u64 | `30` | Delay between Soul Ring and ability press |
| `trigger_cooldown_ms` | u64 | `10` in `config.toml` (`500` serde fallback) | Lockout period after triggering (prevents double-fire) |
| `ability_keys` | Vec<String> | `["q","w","e","r","d","f"]` | Ability keys to intercept |
| `intercept_item_keys` | bool | `true` | Also intercept item slot keys |

## Related Files

| File | Purpose |
|------|---------|
| `src/actions/soul_ring.rs` | State tracking and trigger logic |
| `src/actions/mana_costs.rs` | **Generated** item/ability mana costs — do not hand-edit |
| `scripts/generate-mana-costs.ps1` | Regenerates the table from odota/dotaconstants |
| `src/input/keyboard.rs` | Key interception with `grab()` |
| `src/actions/dispatcher.rs` | GSI event updates to Soul Ring state |
| `src/config/settings.rs` | `SoulRingConfig` struct |

---

## Details

### How It Works

**Soul Ring** is an item that sacrifices 170 HP to grant 170 temporary mana for 10 seconds. The automation optimizes its usage by:

1. **Automatically detecting** when Soul Ring is in your inventory via GSI
2. **Intercepting** ability and item keypresses
3. **Triggering Soul Ring first**, then forwarding the original keypress
4. **Applying safety checks** to avoid suicide or wasted usage

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
│   • Does the bound ability/item cost mana?  ← mana_costs.rs  │
│   • Hero not silenced / muted / hexed?                       │
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
| `slot_items` | `items.slot0-5`, `neutral0` | Slot key → item name, for cost lookup |
| `ability_slots` | `abilities.ability0-5` | Name, level, and passive flag per slot |
| `ultimate_index` | `ability.ultimate` | Which slot `R` casts |
| `cast_blocked` | `hero.silenced`, `hero.muted`, `hero.hexed` | Press would be dropped by the game |

### Auto-Enable/Disable

The automation **automatically enables** when Soul Ring appears in your inventory and **automatically disables** when you sell or drop it. No manual toggle needed.

### Safety Features

#### Health Threshold

Soul Ring costs 170 HP. The `min_health_percent` setting (default 20%) prevents the automation from triggering when your health is too low, avoiding accidental suicide.

#### Mana Threshold

The checked-in config sets `min_mana_percent = 100`, so Soul Ring can trigger whenever you're below full mana. The Rust fallback is `90` if the key is omitted. If you're already near full mana, triggering Soul Ring would waste most of the temporary mana.

#### Cooldown Lockout

The checked-in config sets `trigger_cooldown_ms = 10`, while the Rust fallback is `500` if the key is omitted. This lockout prevents double-firing when:
- You double-tap an ability for self-cast
- Multiple keypresses happen in quick succession
- GSI updates arrive rapidly

#### Infinite Loop Prevention

The automation will **not** trigger Soul Ring when you press Soul Ring's own item slot key (would cause infinite loop).

### Intercepted Keys

#### Ability Keys

By default these keys are candidates, each mapping to a GSI ability slot:
- **Q** → `ability0`
- **W** → `ability1`
- **E** → `ability2`
- **R** → whichever slot GSI flags `ultimate`
- **D** → `ability3`
- **F** → `ability4`

A candidate is only intercepted if the bound ability is learned, not passive, and costs
mana at its current level. Huskar's `Q`/`W`/`R` and Invoker's orbs never intercept.

#### Item Keys

When `intercept_item_keys = true`, item slot keys are also intercepted:
- Slot keys from `[keybindings]` config (default: Z, X, C, V, B, N)
- Excludes Soul Ring's own slot (to prevent infinite loop)
- **Only items with a non-zero mana cost** (see Mana Cost Table below)

This is useful for mana-costing items like:
- Shiva's Guard (75)
- Scythe of Vyse (250)
- Orchid Malevolence (125)
- Dagon (120)
- Black King Bar (50)

Note that Quelling Blade, Battle Fury, Blink, Hand of Midas, and Satanic all cost **no**
mana and are never intercepted, even though several of them have actives.

### Mana Cost Table

Soul Ring only fires ahead of something that **actually spends mana**. That question is
answered by a generated lookup table, not a hand-maintained skip list.

GSI reports whether an item or ability is *ready* (`can_cast`, `cooldown`) but never what
it *costs*, so the cost comes from `src/actions/mana_costs.rs`:

| Table | Key | Value |
|---|---|---|
| `ITEM_MANA_COST_TABLE` | GSI `item.name` | flat cost, e.g. `("item_shivas_guard", 75)` |
| `ABILITY_MANA_COST_TABLE` | GSI `ability.name` | per-level costs, e.g. `("axe_culling_blade", &[100, 125, 150])` |

Both are loaded into `LazyLock<HashMap<..>>` for O(1) lookup and read through
`item_mana_cost()` / `ability_mana_cost(name, level)`.

#### Why a table and not a skip list

A skip list has to enumerate everything that is *free*, and free is the common case:

| | Items | Hero abilities |
|---|---|---|
| Passive-only | 225 | 2,103 |
| Active, costs mana | **59** | **503** |
| Active, free | 217 | 98 |

Listing what costs mana takes ~59 entries. Listing what does not takes 442, including
every passive item — pressing a Desolator's slot key does nothing, and the old skip list
would happily spend 170 HP on it.

Free actives are not edge cases. Quelling Blade's tree chop, Battle Fury, Blink, all four
Invoker orbs, Ember Spirit's Fire Remnant, and **every one of Huskar's abilities** (he
pays health, not mana) are all free.

#### Regenerating

```bash
pwsh scripts/generate-mana-costs.ps1
```

Source is [odota/dotaconstants](https://github.com/odota/dotaconstants), itself generated
from Valve's game files. Run it after a gameplay patch and review the diff — a mass change
to zero usually means an upstream schema change, not a balance patch.

#### Unknown entries fail safe

A name missing from the table returns `None`, which suppresses Soul Ring rather than
firing it. A post-patch item therefore costs a missed buff, never 170 HP. The distinction
is preserved in `ManaSpend`:

| Variant | Meaning | Triggers? |
|---|---|---|
| `Costs(n)` | known to spend `n` mana | **yes** |
| `Free` | known to cost nothing | no |
| `Nothing` | empty slot, unlearned, or passive | no |
| `Unknown` | not in the table — regenerate | no |

### Ability Keys

Ability keys map to GSI ability slots by Dota's default bindings: `Q`→`ability0`,
`W`→`ability1`, `E`→`ability2`, `D`→`ability3`, `F`→`ability4`. `R` resolves through the
`ultimate` flag rather than a fixed index, because Aghanim's- and talent-granted abilities
shift the tail of the list.

An ability key is skipped when the ability is unlearned (`level == 0`), passive, or has a
zero cost at its current level. `ability.can_cast` is deliberately **not** consulted — it
goes false precisely when mana is short, which is the case Soul Ring exists to fix.

#### Known limitation

The table carries base mana costs. Aghanim's Scepter, Aghanim's Shard, and facets can
change a cost, and those modifiers are not modelled yet. GSI does expose
`hero.aghanims_scepter`, `hero.aghanims_shard`, and `hero.facet` if this needs to be
corrected per-ability later.

### Integration with Hero Scripts

Soul Ring automation does **not** sit in front of every hero-specific path.

Current ordering in `src/input/keyboard.rs` is:

1. calculate Soul Ring eligibility
2. run Shadow Fiend `Q/W/E` raze interception (when enabled)
3. run Shadow Fiend `R` ultimate interception (when enabled)
4. run the generic `Q/W/E/R/D/F` branch where Soul Ring can block and replay the key

That means:

- Largo's `Q/W/E/R` flow can still combine with Soul Ring because it uses the later generic branch
- generic ability and eligible item keys can still prefire Soul Ring
- Shadow Fiend `Q/W/E` raze interception currently wins first, so Soul Ring does **not** prefire there while raze interception is active

### Logging

With `level = "info"` in logging config, you'll see:
```
💍 Soul Ring triggered! mana=45%, health=80%
💍 Soul Ring found in slot2: can_cast=true, key=Some('c')
💍 Soul Ring no longer in inventory, disabling automation
```

With `level = "debug"`, additional diagnostics:
```
💍 Key 'q': spend=Costs(80), intercept=true, trigger=true, available=true, can_cast=true, mana=45%, health=80%
💍 Key 'z': spend=Free, intercept=false, trigger=true, available=true, can_cast=true, mana=45%, health=80%
💍 Pressing Soul Ring key: c
💍 Soul Ring: 'v' is not in mana_costs.rs — rerun scripts/generate-mana-costs.ps1
```

The `spend=` field is the mana-cost verdict for that key. `Free` and `Nothing` mean the
press was correctly left alone; `Unknown` means the table needs regenerating.

### Technical Details

#### Dependencies

- **rdev** with `unstable_grab` feature for key interception
- Key interception uses Windows low-level keyboard hooks
- `grab()` blocks keys from reaching other applications when returning `None`

#### Thread Safety

Soul Ring state is stored in a global `Arc<Mutex<SoulRingState>>`:
- GSI handler thread updates state
- Keyboard listener thread reads state
- Mutex ensures safe concurrent access
