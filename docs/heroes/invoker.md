# Invoker Automation

## Purpose

Learn the current state of the Invoker script plumbing and configuration.  
**Read this when:** understanding Task 1 plumbing-only implementation, preparing for combo/invoke planner work, or reviewing config keys.

## Feature Summary (Task 1 State)

- **Plumbing-only** – Hero script exists but combo logic not yet implemented
- **Hero selection support** – Detected when `npc_dota_hero_invoker` is active
- **Configuration plumbing** – Full `InvokerConfig` struct and defaults registered
- **Standalone trigger logs** – Pressing the standalone key logs a placeholder message
- **GSI-based detection** – Auto-enables when Invoker is detected via GSI
- **Survivability actions** – Auto-uses healing, defensive, and neutral items (shared system)
- **No combo logic** – Panic/prep triggers, invoke planner, and combo sequences are not yet implemented

## Configuration

All settings in `config/config.toml` under `[heroes.invoker]`:

```toml
[heroes.invoker]
# Standalone combo trigger (placeholder log only in Task 1)
standalone_key = "Home"
# Panic combo key (not yet implemented)
panic_key = "End"
# Prep combo key (not yet implemented)
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
# Primary combo profile name (reserved for future use)
primary_profile = "qw_pickoff"
# Prep combo profile name (reserved for future use)
prep_profile = "tornado_emp"
# Combo items to use during sequences (reserved for future use)
combo_items = ["item_spirit_vessel", "item_rod_of_atos"]
# Tornado → EMP timing delay (reserved for future use)
tornado_emp_delay_ms = 700
# Sun Strike combo delay (reserved for future use)
sun_strike_delay_ms = 150
# Meteor → Blast combo delay (reserved for future use)
meteor_blast_delay_ms = 450

[heroes.invoker.armlet]
# Optional Invoker-specific armlet overrides (inherit from [armlet] by default)
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `standalone_key` | string | `"Home"` | Key to trigger standalone combo (logs placeholder in Task 1) |
| `panic_key` | string | `"End"` | Panic combo trigger (not yet implemented) |
| `prep_key` | string | `"PageUp"` | Prep combo trigger (not yet implemented) |
| `quas_key` | char | `'q'` | Quas orb ability keybind |
| `wex_key` | char | `'w'` | Wex orb ability keybind |
| `exort_key` | char | `'e'` | Exort orb ability keybind |
| `invoke_key` | char | `'r'` | Invoke ability keybind |
| `spell_slot_primary_key` | char | `'d'` | Primary invoked spell slot keybind |
| `spell_slot_secondary_key` | char | `'f'` | Secondary invoked spell slot keybind |
| `primary_profile` | string | `"qw_pickoff"` | Combo profile name (reserved for future use) |
| `prep_profile` | string | `"tornado_emp"` | Prep combo profile name (reserved for future use) |
| `combo_items` | array | `["item_spirit_vessel", "item_rod_of_atos"]` | Items to use during combos (reserved) |
| `tornado_emp_delay_ms` | u64 | `700` | Tornado → EMP timing delay (reserved) |
| `sun_strike_delay_ms` | u64 | `150` | Sun Strike combo delay (reserved) |
| `meteor_blast_delay_ms` | u64 | `450` | Meteor → Blast combo delay (reserved) |
| `armlet.*` | object | inherits `[armlet]` | Optional Invoker-specific armlet overrides |

## Related Files

| File | Purpose |
|------|---------|
| `src/actions/heroes/invoker.rs` | Invoker script implementation (plumbing-only in Task 1) |
| `src/config/settings.rs` | `InvokerConfig` struct and defaults |
| `config/config.toml` | User configuration |

---

## Details

### 🧩 Plumbing-Only Implementation (Task 1)

**Current state:**
- `InvokerScript` implements the `HeroScript` trait
- `handle_gsi_event()` runs shared survivability pipeline (healing, defensive items, neutral items)
- `handle_standalone_trigger()` logs: `"Invoker standalone combo requested before combo planner is implemented"`
- Hero is registered in dispatcher and automatically selected when Invoker is detected via GSI

**Not yet implemented:**
- Combo profiles (QW pickoff, QE burst, Tornado+EMP, etc.)
- Invoke planner module for orb/invoke sequencing
- Panic/prep trigger handlers
- Item combo orchestration
- Keyboard interception for orb presses (if needed)

### 🛡️ Survivability Actions

Invoker uses the common `SurvivabilityActions` system:
- **Healing items** – Magic Wand, Faerie Fire, Satanic, etc.
- **Defensive items** – BKB, Lotus Orb, Blade Mail when in danger
- **Neutral items** – Witchbane, Safety Bubble, etc.
- **Danger detection** – Monitors HP changes and enemy abilities

These features share the global `[common]`, `[danger_detection]`, and `[neutral_items]` config sections.

### 🔑 Trigger Model (Task 1)

- **Passive (GSI-driven)**: Runs `handle_gsi_event()` on every GSI update → survivability pipeline only
- **Standalone trigger**: Pressing `standalone_key` calls `handle_standalone_trigger()` → logs placeholder message
- **Panic/prep triggers**: Config keys exist but no handler logic implemented yet

### Usage

1. **Pick Invoker** in-game (auto-detected via GSI)
2. **Configure keybinds** in `config/config.toml` to match in-game settings
3. **Run the app** – Hero is auto-selected
4. **Survivability automation** works immediately (healing/defensive items)
5. **Standalone combo key** logs a placeholder message (no combo execution yet)

### Tuning

- Adjust `[danger_detection]` thresholds to tune when healing/defensive items trigger
- Modify `combo_items` list to prepare for future combo orchestration
- Set `primary_profile` / `prep_profile` to desired combo profile names (no effect in Task 1)

### Logging

With `level = "info"`, you'll see:
```
Invoker standalone combo requested before combo planner is implemented
```

With `level = "debug"`, survivability actions may log additional item usage details.

### Limitations

- **No combo logic** – Standalone/panic/prep keys do not execute combos yet
- **No invoke planner** – Orb sequencing and invoked spell management not implemented
- **Fixed ability keys** – Assumes Q/W/E/R/D/F keybindings (does not read in-game keybindings)
- **No cooldown checks** – Does not verify abilities are off cooldown before use (will be added in future tasks)
- **No keyboard interception** – Does not hook orb presses yet (may be added if needed for combo flow)

---

## Maintenance Checklist

When editing this hero's code, update this doc:

- [ ] New config option added? → Update Configuration table
- [ ] New behavior/feature? → Add section under Details
- [ ] Combo logic implemented? → Update Feature Summary and Limitations
- [ ] Invoke planner added? → Document invoke planning logic
- [ ] Keyboard interception added? → Update Trigger Model and Limitations
- [ ] Changed state tracking? → Add state diagram if needed
- [ ] New logging statements? → Update Logging section
