# Invoker Automation

## Purpose

Learn how the Invoker script automates combo profiles, panic Ghost Walk, and spell preparation sequences using dynamic invoke planning and profile-driven execution.  
**Read this when:** configuring Invoker automation, tuning combo profiles, understanding invoke planning, or extending the profile system.

## Feature Summary

- **Profile-driven combos** – Pre-configured spell sequences for QW pickoff (`qw_pickoff`) and QE burst (`qe_burst`)
- **Dynamic invoke planning** – Automatically invokes needed spells, reuses already-invoked spells when present
- **Panic Ghost Walk** – Dedicated hotkey that invokes and casts Ghost Walk for emergency escape
- **Spell preparation** – Hotkey-triggered prep sequences to set up two spells without casting them
- **Configurable timing delays** – Profile-specific delays between spells (Tornado→EMP, Sun Strike→Meteor→Blast)
- **Combo item support** – Optional item usage before spell execution; configured items are pressed in order with 50ms delays
- **Survivability actions** – Auto-use healing, defensive items, and neutral items

## Configuration

All settings live in `config/config.toml` under `[heroes.invoker]`:

```toml
[heroes.invoker]
# Standalone combo trigger
standalone_key = "Home"
# Panic Ghost Walk trigger
panic_key = "End"
# Prep combo trigger
prep_key = "PageUp"
# Quas orb ability key
quas_key = "q"
# Wex orb ability key
wex_key = "w"
# Exort orb ability key
exort_key = "e"
# Invoke ability key
invoke_key = "r"
# Primary invoked spell slot
spell_slot_primary_key = "d"
# Secondary invoked spell slot
spell_slot_secondary_key = "f"
# Primary combo profile name
primary_profile = "qw_pickoff"
# Prep combo profile name
prep_profile = "tornado_emp"
# Combo items to use during sequences
combo_items = ["item_spirit_vessel", "item_rod_of_atos"]
# Tornado → EMP timing delay
tornado_emp_delay_ms = 700
# Sun Strike combo delay
sun_strike_delay_ms = 150
# Meteor → Blast combo delay
meteor_blast_delay_ms = 450

[heroes.invoker.armlet]
# Optional Invoker-specific armlet overrides (inherit from [armlet] by default)
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `standalone_key` | string | `"Home"` | Key to trigger standalone combo execution |
| `panic_key` | string | `"End"` | Panic Ghost Walk trigger |
| `prep_key` | string | `"PageUp"` | Prep combo trigger |
| `quas_key` | char | `'q'` | Quas orb ability keybind |
| `wex_key` | char | `'w'` | Wex orb ability keybind |
| `exort_key` | char | `'e'` | Exort orb ability keybind |
| `invoke_key` | char | `'r'` | Invoke ability keybind |
| `spell_slot_primary_key` | char | `'d'` | Primary invoked spell slot keybind |
| `spell_slot_secondary_key` | char | `'f'` | Secondary invoked spell slot keybind |
| `primary_profile` | string | `"qw_pickoff"` | Combo profile name: `"qw_pickoff"` or `"qe_burst"` |
| `prep_profile` | string | `"tornado_emp"` | Prep combo profile: `"tornado_emp"`, `"meteor_blast"`, `"cold_snap_forge_spirit"`, or `"ghost_walk_ice_wall"` |
| `combo_items` | array | `["item_spirit_vessel", "item_rod_of_atos"]` | Items to use before combo spell sequence |
| `tornado_emp_delay_ms` | u64 | `700` | Tornado → EMP timing delay in ms |
| `sun_strike_delay_ms` | u64 | `150` | Sun Strike → Meteor timing delay in ms |
| `meteor_blast_delay_ms` | u64 | `450` | Meteor → Blast timing delay in ms |
| `armlet.*` | object | inherits `[armlet]` | Optional Invoker-specific armlet overrides |

## Related Files

| File | Purpose |
|------|---------|
| `src/actions/heroes/invoker.rs` | Invoker script implementation, invoke planner, combo execution |
| `src/input/keyboard.rs` | Panic and prep hotkey wiring |
| `src/config/settings.rs` | `InvokerConfig` struct and defaults |
| `config/config.toml` | User configuration |
| `docs/features/keyboard-interception.md` | Global interception ordering and hotkey model |

---

## Details

### Combo Profiles

Invoker supports two primary combo profiles:

#### QW Pickoff (`qw_pickoff`)

**Spell sequence:**
1. Tornado
2. EMP (delayed by `tornado_emp_delay_ms`)

**Use case:** QW build pickoff and teamfight initiation

#### QE Burst (`qe_burst`)

**Spell sequence:**
1. Sun Strike
2. Chaos Meteor (delayed by `sun_strike_delay_ms`)
3. Deafening Blast (delayed by `meteor_blast_delay_ms`)

**Use case:** QE build burst damage combo

### Combo Items

The `combo_items` config setting allows you to use items before the spell sequence executes:

- **Execution order** – Items are pressed in the configured order, before any spells are cast
- **Item lookup** – Uses partial name matching (e.g., `"item_orchid"` matches `"item_orchid"` and `"item_bloodthorn"`)
- **Skips missing items** – If an item is not in inventory or not found, logs a skip message and continues
- **50ms delay** – Each item press is followed by a 50ms delay before the next item or spell

**Example:**
```toml
combo_items = ["item_orchid", "item_spirit_vessel", "item_rod_of_atos"]
```

This configuration will use Orchid/Bloodthorn, Spirit Vessel, and Rod of Atos (in that order) before casting the spell sequence.

### Invoke Planning

The script uses dynamic invoke planning:

1. **Check active spells first** – If a spell is already invoked, cast it directly without re-invoking
2. **Plan orb sequence** – For spells not currently invoked, plan the orb presses needed (e.g., `E-E-W-R` for Meteor)
3. **Reuse secondary slot** – Newly invoked spells always replace the secondary spell slot

This approach minimizes invoke overhead and works correctly when spells are already prepared.

### Panic Ghost Walk

Pressing `panic_key` triggers the panic Ghost Walk sequence:

1. Checks if Ghost Walk is already invoked
2. If not invoked, presses orb sequence (`Q-Q-W-R`)
3. Casts Ghost Walk
4. Skips execution if hero is stunned, silenced, or dead

### Prep Sequences

Pressing `prep_key` invokes two spells without casting them, setting up for manual execution:

**Supported prep profiles:**
- `tornado_emp` – Prepares Tornado and EMP
- `meteor_blast` – Prepares Chaos Meteor and Deafening Blast
- `cold_snap_forge_spirit` – Prepares Cold Snap and Forge Spirit
- `ghost_walk_ice_wall` – Prepares Ghost Walk and Ice Wall

The prep system only invokes spells that aren't already active, skipping redundant invocations.

### Request Queue

Invoker uses a dedicated worker thread with a request queue:

- **FIFO ordering** – Requests execute in the order received
- **Non-blocking enqueue** – Hotkey handlers return immediately after enqueuing
- **Singleton worker** – One worker processes all Invoker requests sequentially

Request types:
- `PrimaryCombo` – Executes the configured `primary_profile` combo
- `PanicGhostWalk` – Invokes and casts Ghost Walk
- `PrepPair` – Invokes the configured `prep_profile` spells without casting

### Survivability Actions

Invoker uses the common `SurvivabilityActions` system:
- **Healing items** – Magic Wand, Faerie Fire, Satanic, etc.
- **Defensive items** – BKB, Lotus Orb, Blade Mail when in danger
- **Neutral items** – Witchbane, Safety Bubble, etc.
- **Danger detection** – Monitors HP changes and enemy abilities

These features share the global `[common]`, `[danger_detection]`, and `[neutral_items]` config sections.

### Usage

1. **Pick Invoker** in-game (auto-detected via GSI)
2. **Configure keybinds** in `config/config.toml` to match in-game settings
3. **Set primary_profile** to `"qw_pickoff"` or `"qe_burst"` based on your build
4. **Run the app** – Hero is auto-selected
5. **Survivability automation** works immediately (healing/defensive items)
6. **Press standalone_key** to execute combo
7. **Press panic_key** for emergency Ghost Walk
8. **Press prep_key** to prepare spells without casting

### Tuning

- Adjust `tornado_emp_delay_ms` to match Tornado travel time (depends on Wex level and distance)
- Tune `sun_strike_delay_ms` and `meteor_blast_delay_ms` for your cast-point and target prediction
- Modify `combo_items` list to include items you want to use before spell execution
- Set `prep_profile` to match your intended follow-up combo

### Logging

With `level = "info"`, you'll see:
```
🔮 Invoker: Primary Combo - qw_pickoff
🔮 Cast invoker_tornado
🔮 Cast invoker_emp
🔮 Primary combo complete
```

With `level = "debug"`, survivability actions may log additional item usage details.

### Limitations

- **No auto-aim** – Spells cast at current cursor position, no prediction or targeting
- **No core spell interception** – Does not hook orb presses or modify core Invoker gameplay
- **No inferred orb upgrades** – Does not read ability levels or adapt recipes dynamically
- **Fixed ability keys** – Assumes configured keybindings match in-game settings
- **No cooldown checks** – Does not verify abilities are off cooldown before attempting cast
- **No Scepter/Shard awareness** – Does not adapt combo logic based on Aghanim's upgrades
- **No item cooldown checks** – Combo items are pressed regardless of cooldown state

---

## Maintenance Checklist

When editing this hero's code, update this doc:

- [ ] New config option added? → Update Configuration table
- [ ] New behavior/feature? → Add section under Details
- [ ] New combo profile? → Document under Combo Profiles
- [ ] Changed invoke planning? → Update Invoke Planning section
- [ ] New request type? → Update Request Queue section
- [ ] Changed state tracking? → Add state diagram if needed
- [ ] New logging statements? → Update Logging section

