# Invoker Automation

## Purpose

Use this when you need to configure or extend Invoker's named profile system.
The current implementation is **profile-first**: each enabled profile has its
own hotkey, ordered steps, and per-step delays.

## Feature Summary

- **Named profiles** - Combo and prep profiles live in one ordered list.
- **Per-profile hotkeys** - Each profile binds its own trigger key.
- **Ordered steps** - Every step is either a spell or an item with its own
  `delay_after_ms`.
- **Per-step completion mode** - Steps can use fixed delay or wait for a spell
  to enter cooldown before continuing.
- **Dynamic invoke planning** - The runtime reuses active spells when possible
  and updates slot tracking after each invoke.
- **Prep mode** - Prep profiles invoke spells without casting them.
- **Shared survivability** - Healing, defensive items, and neutral items still
  run through the common survivability pipeline.

## Configuration

All settings live in `config/config.toml` under `[heroes.invoker]`.

```toml
[heroes.invoker]
quas_key = "q"
wex_key = "w"
exort_key = "e"
invoke_key = "r"
spell_slot_primary_key = "d"
spell_slot_secondary_key = "f"

[[heroes.invoker.profiles]]
id = "qw-pickoff"
name = "QW Pickoff"
enabled = true
hotkey = "Home"
mode = "combo"
build_tag = "qw"

[[heroes.invoker.profiles.steps]]
kind = "item"
target = "item_spirit_vessel"
delay_after_ms = 50
completion_mode = "fixed_delay"
completion_timeout_ms = 3000

[[heroes.invoker.profiles.steps]]
kind = "item"
target = "item_rod_of_atos"
delay_after_ms = 50
completion_mode = "fixed_delay"
completion_timeout_ms = 3000

[[heroes.invoker.profiles.steps]]
kind = "spell"
target = "invoker_tornado"
delay_after_ms = 700
completion_mode = "fixed_delay"
completion_timeout_ms = 3000

[[heroes.invoker.profiles.steps]]
kind = "spell"
target = "invoker_emp"
delay_after_ms = 100
completion_mode = "fixed_delay"
completion_timeout_ms = 3000
```

### Config fields

| Option | Type | Default | Description |
|---|---|---|---|
| `quas_key` | char | `'q'` | Quas orb keybind |
| `wex_key` | char | `'w'` | Wex orb keybind |
| `exort_key` | char | `'e'` | Exort orb keybind |
| `invoke_key` | char | `'r'` | Invoke keybind |
| `spell_slot_primary_key` | char | `'d'` | Primary invoked spell slot |
| `spell_slot_secondary_key` | char | `'f'` | Secondary invoked spell slot |
| `profiles` | array | seeded preset pack | Ordered named Invoker profiles |
| `profiles[].id` | string | preset-specific | Stable runtime identifier |
| `profiles[].name` | string | preset-specific | Operator-facing profile name |
| `profiles[].enabled` | bool | `true` | Whether the hotkey is active |
| `profiles[].hotkey` | string | preset-specific | Trigger key for this profile |
| `profiles[].mode` | string | preset-specific | `combo` casts steps, `prep` only invokes spell steps |
| `profiles[].build_tag` | string | preset-specific | Freeform build hint shown in UI (`general`, `qw`, `qe`) |
| `profiles[].steps[].kind` | string | n/a | `spell` or `item` |
| `profiles[].steps[].target` | string | n/a | Stable spell/item target id such as `invoker_emp` or `item_rod_of_atos` |
| `profiles[].steps[].delay_after_ms` | u64 | preset-specific | Delay after this step executes |
| `profiles[].steps[].completion_mode` | string | `fixed_delay` | `fixed_delay` or `wait_for_cooldown` for spell steps |
| `profiles[].steps[].completion_timeout_ms` | u64 | `3000` | Timeout used when waiting for cooldown confirmation |
| `profiles[].steps[].notes` | string | `""` | Optional operator note shown in the UI |
| `armlet.*` | object | inherits `[armlet]` | Optional Invoker-specific armlet overrides |

## Preset Pack

The checked-in defaults seed four starter profiles:

1. **QW Pickoff** - Spirit Vessel -> Rod of Atos -> Tornado -> EMP
2. **QE Burst** - Sun Strike -> Chaos Meteor -> Deafening Blast, with Sun Strike waiting for manual cooldown confirmation
3. **Ghost Walk Panic** - single-step emergency Ghost Walk cast
4. **Meteor + Blast Prep** - prepares Meteor and Blast without casting

The React UI exposes these as preset cards and lets operators duplicate them
into custom profiles instead of editing raw config strings.

## Runtime Behavior

### Combo profiles

Combo profiles execute their steps in order:

1. Item steps try to find the matching inventory slot and press it.
2. Spell steps check whether the spell is already active.
3. Missing spells are invoked via the configured orb recipe.
4. After an invoke, slot tracking is updated before planning the next spell.
5. The step either waits on `delay_after_ms` or, for manual spell steps, waits
   for cooldown confirmation first and then applies `delay_after_ms`.

### Prep profiles

Prep profiles still plan every spell step in order, but they **do not cast**
those spells after invoke. This is what fixes the old raw `prep_profile`
special-casing: the runtime now trusts the declared profile order directly.

Item steps are ignored in prep mode and logged as skipped.

### Manual cooldown-wait steps

Spell steps can choose how completion is detected:

- `fixed_delay` keeps the existing execute-then-delay behavior
- `wait_for_cooldown` is for manual-targeted spells such as Sun Strike

When `wait_for_cooldown` is selected, the runner presses the active spell key
once and then waits for the spell to enter cooldown before advancing. If the
spell is already on cooldown when the step begins, the runner logs that the
step is already consumed and skips it. If cooldown never starts before
`completion_timeout_ms`, the remaining profile is aborted.

### Hotkeys

Invoker no longer uses dedicated `panic_key` / `prep_key` config fields.
Instead, the keyboard layer scans `profiles[]` and emits
`HotkeyEvent::InvokerProfile(<id>)` for the selected hero's enabled profile
hotkeys.

### Request queue

Invoker still uses a dedicated FIFO worker queue. The queued request is now
just a named profile id:

- `RunProfile("qw-pickoff")`
- `RunProfile("ghost-walk-panic")`
- `RunProfile("meteor-blast-prep")`

## UI Model

`src-ui` now treats Invoker as a profile builder instead of a pile of raw text
fields:

- preset library cards
- configured profile list
- per-profile hotkey / mode / build-tag controls
- ordered step editor with local chip visuals
- per-step delays, completion controls, and notes
- add / duplicate / delete / reorder actions

The visual chips in the editor are local app-owned assets generated in code;
they do not rely on scraped external art.

## Logging

At `info` level, Invoker now logs:

```text
🔮 Invoker profile: QW Pickoff (combo)
🔮 Planned steps: [...]
🔮 Active slots before step: [...]
🔮 Active slots after invoke: [...]
🔮 Casting invoker_tornado from f
🔮 Waiting for invoker_sun_strike cooldown to start
🔮 Invoker profile complete: QW Pickoff
```

These logs are the first place to look when a spell order feels wrong in live
play.

## Limitations

- **No auto-aim** - spells still cast at the current cursor position
- **No Q/W/E/R interception** - this does not hook normal Invoker orb play
- **No cooldown-aware item planner** - missing or unusable items are skipped at
  execution time
- **No branching DSL** - profiles are strictly ordered step lists

## Related Files

| File | Purpose |
|---|---|
| `src/actions/heroes/invoker.rs` | Named profile runner, invoke planner, queue worker |
| `src/input/keyboard.rs` | Invoker profile hotkey mapping |
| `src/config/settings.rs` | `InvokerConfig` defaults and serde model |
| `src-ui/src/components/heroes/configs/InvokerConfig.tsx` | Profile builder shell |
| `src-ui/src/components/heroes/configs/invoker/` | Preset catalog, profile list, and editor UI |
| `docs/features/keyboard-interception.md` | Global hotkey interception model |

