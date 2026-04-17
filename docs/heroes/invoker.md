# Invoker Automation

## Purpose

Use this when you need to configure or extend Invoker's named profile system.
The current implementation is **profile-first**: each enabled profile has its
own hotkey, ordered steps, and per-step delays.

## Feature Summary

- **Named profiles** - Combo and prep profiles live in one ordered list.
- **Per-profile hotkeys** - Each profile binds its own trigger key.
- **One active combo profile** - Exactly one enabled combo profile is treated as
  the generic active combo at runtime.
- **Ordered steps** - Every step is either a spell or an item with its own
  `delay_after_ms`.
- **Spell activation behavior** - `cast_behavior` only affects spell steps;
  item steps still fire as normal single presses.
- **Per-step completion mode** - Steps can use fixed delay or wait for a spell
  to enter cooldown before continuing.
- **Pair-aware invoke planning** - Consecutive spell steps are preloaded into
  the real `D`/`F` slots before casting.
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
cycle_combo_profiles_hotkey = "Delete"

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
cast_behavior = "normal"
completion_mode = "fixed_delay"
completion_timeout_ms = 3000

[[heroes.invoker.profiles.steps]]
kind = "item"
target = "item_rod_of_atos"
delay_after_ms = 50
cast_behavior = "normal"
completion_mode = "fixed_delay"
completion_timeout_ms = 3000

[[heroes.invoker.profiles.steps]]
kind = "spell"
target = "invoker_tornado"
delay_after_ms = 700
cast_behavior = "normal"
completion_mode = "fixed_delay"
completion_timeout_ms = 3000

[[heroes.invoker.profiles.steps]]
kind = "spell"
target = "invoker_emp"
delay_after_ms = 100
cast_behavior = "normal"
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
| `cycle_combo_profiles_hotkey` | string | `"Delete"` | Rotates the active enabled combo profile without running it |
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
| `profiles[].steps[].cast_behavior` | string | `normal` | `normal`, `manual_wait_cooldown`, `alt_cast`, `double_tap`, or `alt_double_tap` |
| `profiles[].steps[].completion_mode` | string | `fixed_delay` | `fixed_delay` or `wait_for_cooldown` for spell steps |
| `profiles[].steps[].completion_timeout_ms` | u64 | `3000` | Timeout used when waiting for cooldown confirmation |
| `profiles[].steps[].notes` | string | `""` | Optional operator note shown in the UI |
| `armlet.*` | object | inherits `[armlet]` | Optional Invoker-specific armlet overrides |

## Preset Pack

The checked-in defaults seed these starter profiles:

1. **QW Pickoff** - Spirit Vessel -> Rod of Atos -> Tornado -> EMP
2. **QE Burst** - Sun Strike -> Chaos Meteor -> Deafening Blast, with Sun Strike prepared first and then waiting for the player's real cast
3. **Ghost Walk Panic** - single-step emergency Ghost Walk cast
4. **Meteor + Blast Prep** - prepares Meteor and Blast without casting
5. **Lane Pressure** - summon-only Forge Spirit preset; it does not append follow-up spells and does not automate Forge Spirit unit control after the summon
6. **Meta Catch** - Tornado -> EMP -> Cold Snap
7. **Shotgun Burst** - Rod of Atos -> Sun Strike -> Chaos Meteor -> Deafening Blast
8. **Ice Floe Lockdown** - Ice Wall -> Chaos Meteor
9. **Refresher Sequence** - Tornado -> EMP -> Chaos Meteor -> Deafening Blast -> Refresher -> self Sun Strike -> Chaos Meteor -> Deafening Blast

The React UI exposes these as preset cards and lets operators duplicate them
into custom profiles instead of editing raw config strings.

## Runtime Behavior

### Combo profiles

Combo profiles execute their steps in order:

1. Item steps try to find the matching inventory slot and press it.
   Optional combo items are best-effort only: if the configured item is missing,
   that step is logged as skipped and the rest of the profile continues.
   `cast_behavior` does not change item steps; they still execute as single
   presses.
2. Consecutive spell steps are grouped into one- or two-spell batches.
3. A two-spell batch is preloaded in profile order so the older prepared spell
   lands on `F` and the newer prepared spell lands on `D`.
4. Single trailing spell steps still use whichever slot they actually occupy,
   invoking first when needed.
5. The step either waits on `delay_after_ms` or, for manual spell steps, waits
   for cooldown confirmation first and then applies `delay_after_ms`.

### Active combo selection

Invoker now tracks one active combo profile at a time. This is the profile used
by the generic combo trigger; the runtime no longer blindly picks the first
enabled combo in the list unless it needs to repair a missing or invalid active
selection.

- Clicking an **enabled combo** profile in the React UI marks that profile as
  the active combo and opens it in the editor.
- Clicking a **prep** profile still opens it in the editor and keeps it directly
  runnable, but it does **not** replace the active combo.
- Per-profile **combo** hotkeys still run their profile immediately and also
  promote that profile to the active combo.
- Per-profile **prep** hotkeys still run directly without changing the active
  combo.
- The global Invoker cycle hotkey rotates only through enabled `mode = "combo"`
  profiles.

If the active combo changes, the app emits an activity entry so the operator can
see the switch in the recent-event feed, for example
`Invoker active combo changed to QE Burst`.

### Spell preload behavior

Invoker spell profiles are still authored in natural cast order, such as:

`Tornado -> EMP -> Sun Strike`

The runtime now preloads up to two consecutive spell steps in profile order and
casts them using their actual Invoked slots:

- when two spells are loaded, the older prepared spell is usually on `F`
- the newer prepared spell is usually on `D`
- so a preloaded pair executes as `F` first, then `D`

For the example above, the runtime prepares Tornado then EMP, casts Tornado from
`F`, casts EMP from `D`, then invokes and casts Sun Strike from `D`.

### Prep profiles

Prep profiles still plan every spell step in order, but they **do not cast**
those spells after invoke. This is what fixes the old raw `prep_profile`
special-casing: the runtime now trusts the declared profile order directly.

Item steps are ignored in prep mode and logged as skipped.

### Cast behaviors and manual steps

Spell steps can choose how the cast key is pressed:

- `normal` - press the invoked spell key once
- `manual_wait_cooldown` - prepare the spell but do not auto-cast it; the runner waits for the player's real cast to start cooldown before continuing
- `alt_cast` - hold `Alt` while pressing the invoked spell key once
- `double_tap` - press the invoked spell key twice with a short gap
- `alt_double_tap` - hold `Alt` while double-tapping the invoked spell key

Completion still controls when the next step starts:

- `fixed_delay` applies `delay_after_ms` after the cast behavior runs
- `wait_for_cooldown` waits for cooldown start before applying `delay_after_ms`

`manual_wait_cooldown` forces cooldown waiting even if the step is authored with
`completion_mode = "fixed_delay"`.

If a manual-wait spell is already on cooldown when the step begins, the runner
logs that the step is already consumed and skips it. If cooldown never starts
before `completion_timeout_ms`, the remaining profile is aborted.

### Hotkeys

Invoker no longer uses dedicated `panic_key` / `prep_key` config fields.
Instead, the keyboard layer scans `profiles[]` and emits
`HotkeyEvent::InvokerProfile(<id>)` for the selected hero's enabled profile
hotkeys.

The generic combo trigger resolves the current active combo profile first. The
global Invoker cycle hotkey updates that active combo selection without running
the profile, so the next generic combo trigger uses the newly selected combo.
That cycle key is configurable through
`heroes.invoker.cycle_combo_profiles_hotkey`.
Invoker hotkeys are edge-triggered: holding a combo/profile/cycle hotkey only
fires once, and the key must be released before it can trigger again.

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
Invoker active combo profile set to qe-burst
🔮 Invoker profile: QW Pickoff (combo)
🔮 Planned steps: [...]
🔮 Active slots before step: [...]
🔮 Active slots after invoke: [...]
🔮 Casting invoker_tornado from f
🔮 Waiting for invoker_sun_strike cooldown to start
🔮 Invoker profile complete: QW Pickoff
```

The activity feed also records active-combo changes when UI clicks, per-profile
combo hotkeys, or the cycle hotkey switch the current combo. These logs are the
first place to look when a spell order feels wrong in live play.

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

