# Outworld Destroyer Automation

## Purpose

Learn how the Outworld Destroyer script automates `Objurgation`, `Sanity's Eclipse` fight entry, self-Astral panic casting, and an optional aggressive combo layer.  
**Read this when:** configuring OD automation, tuning `Objurgation` danger gates, understanding `R` interception, or extending the combo flow.

## Feature Summary

- **Danger-driven Objurgation** – Auto-casts `Objurgation` when OD is in danger and configured HP / mana gates pass
- **Ultimate interception** – Blocks `R` and runs `BKB -> Objurgation -> Sanity's Eclipse` when enabled
- **Standalone engage combo** – Uses the generic combo trigger for `Blink -> BKB -> combo items -> Objurgation -> R`
- **Self-Astral panic hotkey** – Optional dedicated hotkey that double-taps Astral on yourself
- **Post-ultimate Arcane Orb helper** – Optional Q-spam follow-up after the ultimate
- **Survivability actions** – Auto-use healing, defensive items, and neutral items

## Configuration

All settings live in `config/config.toml` under `[heroes.outworld_destroyer]`:

```toml
[heroes.outworld_destroyer]
standalone_key = "Home"
objurgation_key = "e"
arcane_orb_key = "q"
astral_imprisonment_key = "w"
auto_objurgation_on_danger = true
objurgation_hp_threshold_percent = 55
objurgation_min_mana_percent = 25
objurgation_trigger_cooldown_ms = 1500
ultimate_intercept_enabled = true
auto_bkb_on_ultimate = true
auto_objurgation_on_ultimate = true
post_bkb_delay_ms = 50
post_blink_delay_ms = 100
astral_self_cast_enabled = false
astral_self_cast_key = "F5"
combo_items = ["sheepstick", "bloodthorn"]
combo_item_spam_count = 1
combo_item_delay_ms = 50
post_ultimate_arcane_orb_presses = 0
arcane_orb_press_interval_ms = 30
```

| Option | Type | Default | Description |
|---|---|---|---|
| `standalone_key` | string | `"Home"` | Generic combo-trigger hotkey for the OD engage combo |
| `objurgation_key` | char | `'e'` | Synthetic key used when the script casts `Objurgation` |
| `arcane_orb_key` | char | `'q'` | Synthetic key used for optional post-ultimate Orb presses |
| `astral_imprisonment_key` | char | `'w'` | Synthetic key used for the self-Astral helper |
| `auto_objurgation_on_danger` | bool | `true` | Enables danger-driven passive `Objurgation` |
| `objurgation_hp_threshold_percent` | u32 | `55` | HP% gate for passive `Objurgation` |
| `objurgation_min_mana_percent` | u32 | `25` | Minimum mana% required before passive `Objurgation` can fire |
| `objurgation_trigger_cooldown_ms` | u64 | `1500` | Local anti-spam lockout between passive `Objurgation` casts |
| `ultimate_intercept_enabled` | bool | `true` | Enables the OD `R` interception path in `keyboard.rs` |
| `auto_bkb_on_ultimate` | bool | `true` | If true, OD tries BKB before `Sanity's Eclipse` |
| `auto_objurgation_on_ultimate` | bool | `true` | If true, OD tries `Objurgation` before `Sanity's Eclipse` |
| `post_bkb_delay_ms` | u64 | `50` | Delay after BKB before the combo continues |
| `post_blink_delay_ms` | u64 | `100` | Delay after Blink before the standalone combo continues |
| `astral_self_cast_enabled` | bool | `false` | Enables the dedicated self-Astral panic hotkey |
| `astral_self_cast_key` | string | `"F5"` | Hotkey intercepted for self-Astral |
| `combo_items` | array of strings | `[]` | Ordered item-name substrings to cast during the standalone combo |
| `combo_item_spam_count` | u32 | `1` | Number of presses per configured combo item |
| `combo_item_delay_ms` | u64 | `50` | Delay between combo-item presses |
| `post_ultimate_arcane_orb_presses` | u32 | `0` | Optional number of Orb presses after `Sanity's Eclipse` |
| `arcane_orb_press_interval_ms` | u64 | `30` | Delay between post-ultimate Orb presses |

## Related Files

| File | Purpose |
|---|---|
| `src/actions/heroes/outworld_destroyer.rs` | OD hero logic, request worker, danger gating, standalone combo |
| `src/input/keyboard.rs` | `R` interception and self-Astral hotkey wiring |
| `src/config/settings.rs` | `OutworldDestroyerConfig` and serde defaults |
| `config/config.toml` | Checked-in OD configuration |
| `docs/features/keyboard-interception.md` | Global interception ordering and replay model |

---

## Details

### Danger-driven Objurgation

On every GSI event, the OD script:

1. caches the latest event for later combo/intercept use
2. runs the shared danger detector
3. evaluates `Objurgation` against OD-specific gates
4. enqueues one synthetic keypress on the shared executor if all gates pass

Passive `Objurgation` requires:

- OD is alive
- danger logic currently says the hero is in danger
- OD is not stunned or silenced
- HP% is at or below `objurgation_hp_threshold_percent`
- mana% is at or above `objurgation_min_mana_percent`
- `obsidian_destroyer_objurgation` is learned and castable
- the local anti-spam cooldown has elapsed

The anti-spam lockout is local to the hero script, so bursty GSI updates do not repeatedly burn `Objurgation` in the same panic window.

### Ultimate interception

If OD is the active hero and `ultimate_intercept_enabled = true`, the keyboard hook checks `R` before the generic Soul Ring / Largo path.

When `Sanity's Eclipse` is ready, the hook:

1. blocks the original `R`
2. enqueues one OD request onto the dedicated OD worker
3. optionally uses BKB
4. optionally uses `Objurgation`
5. presses `R`
6. optionally presses Arcane Orb a configured number of times after the ultimate

If `Sanity's Eclipse` is not ready, the hook does not swallow `R`.

### Standalone engage combo

OD participates in the generic standalone combo flow, so the configured `standalone_key` uses the same `HotkeyEvent::ComboTrigger` path as Tiny and Legion Commander.

The current OD standalone order is:

1. Blink if available
2. optional BKB
3. ordered `combo_items`
4. optional `Objurgation`
5. `Sanity's Eclipse`
6. optional Arcane Orb follow-up presses

`combo_items` is substring-matched against inventory item names, so values like `"sheepstick"` or `"bloodthorn"` work without needing the full internal item name.

### Self-Astral panic hotkey

If `astral_self_cast_enabled = true`, the keyboard hook watches `astral_self_cast_key`.

When pressed, the OD worker:

1. verifies `Astral Imprisonment` is ready
2. blocks the original panic hotkey
3. double-taps the configured Astral key

This path is intentionally opt-in because self-cast semantics are timing-sensitive and player preference varies.

### Survivability actions

OD still composes the shared `SurvivabilityActions` stack:

- healing items
- danger-triggered defensive items
- danger-triggered neutral items

Those systems remain configured through the shared `[common]`, `[danger_detection]`, and `[neutral_items]` sections.

### Limitations

- **No automatic enemy targeting** – The script does not choose Astral, Hex, Bloodthorn, or Orb targets for you
- **Arcane Orb remains opt-in** – Only the post-ultimate follow-up helper is automated; there is no always-on Orb interception
- **Ability-key assumptions** – Defaults assume `Q/W/D/R` style bindings for OD, but the config exists to retune them
- **GSI scope is local hero only** – The script cannot reason about enemy mana pools or target priority from current telemetry

## Maintenance Checklist

- New OD config option added? → Update the table above
- Changed combo ordering? → Update the sequence sections
- Added more intercepted keys? → Update this doc and `docs/features/keyboard-interception.md`
- Changed GSI assumptions? → Update `docs/reference/gsi-schema-and-usage.md`
