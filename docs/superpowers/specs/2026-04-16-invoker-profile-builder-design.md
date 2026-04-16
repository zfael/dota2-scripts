# Invoker Profile Builder Design

## Problem

The current Invoker automation config is functional but hard to trust and hard to operate:

- the UI exposes raw string fields for `primary_profile` and `prep_profile`
- the Combo Items section is only placeholder copy, not an actual editor
- the runtime only supports a tiny set of hardcoded profiles
- the current UX does not make cast order, slot usage, or invoke state visible enough for debugging
- the builder has no visual/icon model for Invoker spells, so configuration is harder than it should be

Invoker is unusually stateful and sequence-sensitive, so a profile system that works for simpler heroes is not sufficient here.

## Goals

- Replace the raw-string Invoker config with a structured, easier-to-use profile system
- Keep onboarding simple with presets while allowing advanced users to create custom profiles
- Make spell/item order explicit and visible in the UI
- Support both cast combos and prep-only profiles
- Improve runtime observability so slot/state issues can be diagnosed from logs
- Support bundled local icon assets for Invoker hero/spell presentation in the builder

## Non-Goals

- No backward-compatibility or migration layer for the old Invoker profile fields
- No third-party scraping pipeline for icons
- No visual/browser mockups in this design pass
- No condition/branch mini-language such as "if stunned" or "if scepter" in v1
- No auto-targeting or geometry-aware combo logic in v1

## Recommended Approach

Use a **preset-first unified profile list** for Invoker. Each profile is a named, editable object with:

- profile type (`combo` or `prep`)
- hotkey
- enabled toggle
- ordered steps
- per-step delays

The UI ships with a default preset pack, but presets are implemented using the same profile model as user-created profiles so they can be cloned, edited, disabled, or deleted.

This approach keeps the feature easy to start with while avoiding the ceiling imposed by today's single primary/prep split.

## Alternatives Considered

### 1. Keep the current primary/prep split and improve the UI

This would be the lowest-effort path, but it still leaves Invoker constrained to two main config slots. That is too limiting for a hero where users often want multiple situational sequences.

### 2. Build a fully expressive combo DSL

This would support branching and conditions, but it would be much harder to explain, test, and maintain. It is too much complexity for the first usable version of the new builder.

## Configuration Model

Invoker should move to a greenfield profile model rather than extending `primary_profile` and `prep_profile`.

### Profile Shape

Each profile should contain:

- `id`
- `name`
- `enabled`
- `hotkey`
- `mode`: `combo` or `prep`
- optional `build_tag`: `qw`, `qe`, or `general`
- `steps[]`

Each step should contain:

- `kind`: `spell` or `item`
- `target`: stable internal ID for the spell or item
- `delay_after_ms`
- optional `notes`

### Data Semantics

- A **combo** profile can contain spell and item cast steps
- A **prep** profile prepares spells but does not cast them
- Ordered steps are authoritative
- Per-step delay is explicit, not inferred from a profile name

### Default Preset Pack

The app should ship with starter Invoker profiles such as:

- QW Pickoff
- QE Burst
- Ghost Walk Panic
- Meteor + Blast Prep

These are not special runtime branches; they are normal profiles seeded by default config.

## UI Design

### Overall Structure

The Invoker page should be reorganized around two top-level sections:

1. **Preset Library**
2. **My Profiles**

The user flow should be:

- start from presets if they want something simple
- clone/edit presets if they want small changes
- create new profiles if they want custom sequences

### Profile List

Each profile row should show:

- enabled toggle
- profile name
- mode
- hotkey
- optional build tag
- concise summary, such as `Atos → Tornado → EMP`
- actions: edit, duplicate, delete

### Profile Editor

Selecting a profile opens a profile editor with:

- name field
- hotkey control
- enabled toggle
- mode selector
- build-tag selector
- ordered step list
- step add controls
- step reorder controls
- delay editing per step
- summary / validation panel

### Preset-First Interaction

The UI should make it obvious that presets are only starting points, not locked system behavior. Users should be able to:

- use preset as-is
- clone preset
- edit preset directly
- disable presets they do not want

### Why This UI Shape

The current UI hides too much behind profile names. The redesign should make sequence order obvious enough that a user can answer:

- what will cast
- in what order
- with what delay
- on which hotkey

without reading docs or raw TOML.

## Runtime Design

Invoker should move from hardcoded profile branches to a generic **profile runner**.

### Execution Flow

1. Hotkey pressed
2. Match enabled Invoker profile by key
3. Snapshot current Invoker state
4. Convert profile steps into planned actions
5. Execute steps sequentially
6. Apply each step's configured delay

### Required State Snapshot

The runner should use a snapshot that includes:

- current active invoked spells
- configured orb keys
- configured spell-slot keys
- alive / disabled state
- inventory visibility for item steps

### Spell Step Execution

For a spell step:

- if the spell is already active, cast it directly from the known slot
- otherwise invoke it first, update slot tracking, then cast it
- for prep profiles, do the invoke/prep work only and stop before cast

### Item Step Execution

For an item step:

- resolve item by configured fragment or ID
- if found, cast it
- if not found, log a skip and continue

### Slot Tracking

The runner must track active spell slots after every invoke. This is mandatory because Invoker is stateful, and slot drift is exactly the kind of bug that makes the system feel untrustworthy.

### Cast Order Trust

The profile order is authoritative. If a profile says Tornado then EMP, the runtime should treat that as the intended cast order.

The design should not assume a profile-name shortcut is enough. The sequence must exist as explicit steps that can be reviewed, logged, and tested.

## Debugging and Observability

Invoker needs first-class execution visibility.

### Logs

For each profile execution, logs should include:

- selected profile name and mode
- ordered step list
- slot state before/after invoke
- which slot key was used for cast
- delay applied after each step
- skipped item/spell reasons

### Debug Preview in UI

The UI should expose a non-runtime preview that shows:

- configured primary/secondary slot keys
- planned execution order
- human-readable step summary

This is specifically meant to help diagnose reports such as "it cast EMP before Tornado."

## Validation and Safety

The editor should validate:

- duplicate hotkeys across enabled Invoker profiles
- empty profile names
- enabled profiles with no steps
- invalid step targets
- prep profiles containing cast-only behavior

The UI should surface these as user-facing warnings rather than silently accepting broken config.

## Icon and Content Model

Icons should be a presentation layer on top of stable profile data.

### Stable IDs

Profile steps should store stable internal IDs such as:

- `invoker_tornado`
- `invoker_emp`
- `item_spirit_vessel`

### Metadata Table

The UI should resolve those IDs through a local metadata table containing:

- display name
- type/category
- bundled local icon path
- optional tooltip/help text

### Asset Scope

For Invoker v1, the bundled asset set should include:

- Invoker hero icon or portrait
- Invoker spell icons needed by the builder
- optionally a small set of common combo-item icons if item steps should render with the same visual quality

If an icon is missing, the UI must fall back to text. Runtime behavior must never depend on icon availability.

## Greenfield Decision

This redesign assumes **no backward compatibility requirement** for the old Invoker combo config.

That means:

- no migration layer from `primary_profile` / `prep_profile`
- no hybrid config format
- no requirement to preserve old raw-string behavior

Instead, Invoker should move directly to the new profile-list model with a shipped default preset pack.

## Testing Strategy

### Profile Model Tests

- profile parsing
- validation rules
- hotkey conflict detection
- prep-vs-combo constraints

### Runtime Planning Tests

- ordered step execution
- item skip behavior
- invoke planning for spells already active
- invoke planning for spells not active
- slot tracking after repeated invokes

### Regression Coverage

Tests should explicitly cover the Tornado/EMP order path and verify:

- profile order remains Tornado then EMP
- slot tracking does not cause the wrong spell to fire
- delays are applied after the intended spell

### UI Tests

- create/edit/delete profile
- clone preset
- reorder steps
- validation message rendering
- hotkey conflict display

## Implementation Notes

- This design is intentionally limited to ordered steps plus per-step delay for v1
- No branching, state conditions, or targeting logic is included
- The same profile data model should be usable later for other heroes if desired, but this design is scoped to Invoker first

## Open Choices Resolved During Brainstorming

- Use **guided presets plus custom profiles**
- Use a **unified profile list** rather than primary/prep split
- Support **ordered steps with per-step delays and per-profile hotkeys**
- Use **bundled local assets** instead of Fandom scraping
- Treat Invoker config as **greenfield**, not backward-compatible

## Success Criteria

The redesign is successful if an operator can:

1. create or clone an Invoker profile without editing raw strings
2. see the exact planned order of spells and items in the UI
3. bind multiple Invoker profiles to different keys
4. distinguish combo profiles from prep-only profiles
5. understand slot/cast order issues from logs instead of guesswork

