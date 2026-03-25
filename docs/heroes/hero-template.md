# [Hero Name] Automation

## Purpose

Learn how the [Hero Name] script automates [primary mechanic(s)].  
**Read this when:** configuring [Hero] automation, tuning [key settings], understanding [specific behavior].

## Feature Summary

- **[Feature 1]** – [Brief description]
- **[Feature 2]** – [Brief description]
- **[Feature 3]** – [Brief description]
- **GSI-based detection** – Auto-enables when `npc_dota_hero_[internal_name]` detected
- **Survivability actions** – Auto-use healing/defensive items

## Configuration

All settings in `config/config.toml` under `[heroes.[hero_name]]`:

```toml
[heroes.[hero_name]]
# [Comment describing config option]
option_name = default_value
# [Another option]
another_option = value
# Standalone combo key (if applicable)
standalone_key = "Home"
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `option_name` | type | `default` | Description of what this option does |
| `another_option` | type | `default` | Description of what this option does |
| `standalone_key` | string | `"Home"` | Key to trigger standalone combo (if implemented) |

## Related Files

| File | Purpose |
|------|---------|
| `src/actions/heroes/[hero_name].rs` | [Hero] script implementation |
| `src/config/settings.rs` | `[Hero]Config` struct and defaults |
| `config/config.toml` | User configuration |

---

## Details

### [Icon] [Feature 1 Heading]

[Detailed description of the first feature/mechanic. Include:]

- **How it works** – Step-by-step explanation
- **Trigger conditions** – When/how the feature activates
- **Configuration impact** – How settings affect behavior
- **Code references** – Key functions or state tracking

#### [Subfeature or Technical Detail]

[If the feature has complexity, break it down into subsections.]

Example:
- **State 1**: [Description]
- **State 2**: [Description]

### [Icon] [Feature 2 Heading]

[Detailed description of the second feature.]

#### [Trigger Model]

Describe how the feature is triggered:
- **Passive (GSI-driven)**: Runs on every `handle_gsi_event()` call
- **Standalone trigger**: Activated via `handle_standalone_trigger()` when configured key pressed
- **Key interception**: Intercepts specific keys (e.g., Q/W/E) via `keyboard.rs`

### [Icon] [Feature 3 Heading]

[Detailed description of the third feature.]

If the hero uses Soul Ring integration, reference:
```
See `docs/features/soul-ring.md` for Soul Ring automation details.
```

### 🛡️ Survivability Actions

[Hero] uses the common `SurvivabilityActions` system:
- **Healing items** – Magic Wand, Faerie Fire, Satanic, etc.
- **Defensive items** – BKB, Lotus Orb, Blade Mail when in danger
- **Neutral items** – Witchbane, Safety Bubble, etc.
- **Danger detection** – Monitors HP changes and enemy abilities

These features usually share the global `[common]`, `[danger_detection]`, and `[neutral_items]` config sections. Adjust this sentence if the hero uses a different mix of shared systems.

### 🔄 State Diagram (Optional)

If the hero has complex state transitions (like Largo's beat system), include a visual diagram:

```
┌─────────────────────────────────────────────────────────────┐
│                         STATE 1                              │
│                    (description)                             │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ Trigger condition
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                         STATE 2                              │
│                    (description)                             │
└─────────────────────────────────────────────────────────────┘
```

### 🔒 Thread Safety (If Applicable)

If the hero uses spawned threads or has race condition concerns:

[Describe the threading model, mutexes, guards, etc.]

Example:
```
The [feature] runs in a **separate guarded thread** to prevent blocking.
A `try_lock()` guard ensures only one [action] thread runs at a time.
```

### Usage

1. **Pick [Hero]** in-game (auto-detected via GSI)
2. **Equip [recommended items]**
3. **Level [key abilities]**
4. **Configure thresholds** in `config/config.toml`
5. **Run the app** – Hero is auto-detected
6. **[Feature activates]** automatically / when key pressed

### Tuning

- **[Setting 1]**: How to adjust and expected effect
- **[Setting 2]**: How to adjust and expected effect

Example:
```
Increase `threshold_value` to trigger [feature] more/less frequently.
```

### Logging

With `level = "info"`, you'll see:
```
[Example log message]
```

With `level = "debug"`:
```
[Example debug log message]
```

### Limitations

- **[Limitation 1]** – [Description of what doesn't work or is not implemented]
- **[Limitation 2]** – [Another limitation]
- **[Limitation 3]** – [Another limitation]

Example:
- **Fixed ability keys**: Assumes Q/W/E/R keybindings (does not read in-game keybindings)
- **No cooldown checks**: Does not verify abilities are off cooldown before use

---

## Maintenance Checklist

When editing this hero's code, update this doc:

- [ ] New config option added? → Update Configuration table
- [ ] New behavior/feature? → Add section under Details
- [ ] Changed combo sequence? → Update sequence description and flow diagram
- [ ] Modified trigger model? → Update trigger description
- [ ] Changed state tracking? → Update state diagram
- [ ] New logging statements? → Update Logging section
