# Invoker Active Combo Selection Design

## Problem

Invoker's current profile UX has a mismatch between what the UI suggests and what
the runtime actually does.

Today:

- clicking a configured profile in the React UI only selects it for editing
- there is no stored runtime notion of an "active Invoker combo profile"
- pressing a profile's own hotkey runs that exact profile immediately
- the generic standalone/combo trigger falls back to the **first enabled combo
  profile** in config order

That makes profile selection feel unreliable. A user can click a combo profile
and reasonably expect it to become the one "ready to use," but gameplay does not
change.

The user wants:

- exactly **one active Invoker combo profile at a time**
- clicking a combo profile in the UI to make it active
- an in-game way to switch active combo profiles without alt-tabbing
- cycling to consider only **enabled combo profiles**
- per-profile combo hotkeys to remain useful
- lightweight confirmation when the active combo changes

## Goals

- Add a real runtime "active Invoker combo profile" concept
- Make UI combo selection set the active combo profile
- Add a global hotkey to cycle enabled Invoker combo profiles during gameplay
- Make the generic combo trigger use the active combo instead of "first enabled"
- Keep per-profile combo hotkeys, but have them also update active combo state
- Show the current active combo clearly in the app UI
- Provide lightweight non-overlay feedback when the active combo changes

## Non-Goals

- No in-game overlay in this design
- No prep-profile cycling
- No general profile-state system for every hero
- No branching combo logic or runtime combo DSL changes
- No attempt to auto-detect "best" combo profile from current game state

## Recommended Approach

Introduce a small, explicit **active combo profile state** for Invoker and wire
all relevant selection paths through it.

The runtime should treat the active combo as a separate concern from the config
profile list itself:

- config defines which profiles exist, whether they are enabled, and their
  individual hotkeys
- runtime tracks which enabled **combo** profile is currently active

That active state should change from three places:

1. clicking a combo profile in the UI
2. pressing a new global "cycle active Invoker combo" hotkey
3. pressing a per-profile combo hotkey, which should both set active combo and
   run that combo immediately

The generic combo trigger should then use the active combo profile instead of
the first enabled combo profile.

## Alternatives Considered

### 1. Keep today's direct-hotkey model and only improve the UI copy

This would be the smallest change, but it would not solve the actual usability
problem. The user's expectation is reasonable: clicking a combo profile should
make it the one ready for play.

### 2. Replace per-profile hotkeys with one shared combo trigger only

This would simplify the keyboard model, but the user explicitly wants per-profile
combo hotkeys to remain available. Removing them would reduce flexibility.

### 3. Add an in-game overlay immediately

An overlay would improve visibility, but it is the highest-risk and highest-cost
option. The repo has no current overlay/always-on-top infrastructure, and this
design should avoid taking on that extra complexity before the basic active
selection model is fixed.

## Execution Model

### Active Combo Semantics

Only **combo** profiles participate in active Invoker combo selection.

Rules:

- there is at most one active Invoker combo profile at a time
- only enabled combo profiles are eligible
- prep profiles are never considered active combo candidates
- clicking an **enabled** combo profile in the UI makes it active
- clicking a disabled combo profile may still open it for editing, but must not
  activate it
- disabling or deleting the active combo profile forces a repair to the first
  remaining enabled combo profile

If no enabled combo profiles exist, Invoker should have **no active combo**
rather than silently choosing an invalid profile.

For v1, the active combo should be treated as **runtime state**, not persisted
into the profile config itself. On startup or after settings reload, runtime may
seed active combo from the first enabled combo profile.

### Hotkey Behavior

The keyboard model should become:

- **Per-profile combo hotkey**
  - if the targeted profile is enabled and `mode = combo`, set it active
  - then immediately execute it
- **Per-profile prep hotkey**
  - execute that prep profile directly
  - do not change active combo state
- **Global cycle hotkey**
  - when the current hero is Invoker, advance to the next enabled combo profile
  - wrap around at the end
- **Generic combo trigger**
  - run the current active Invoker combo profile
  - no longer use "first enabled combo profile" as the normal selection rule

This preserves the user's requested model:

- one active combo profile at a time
- direct per-profile combo keys still work
- UI click and cycle hotkey both change the active combo

### Hero Gating

The cycle hotkey should be active only when the selected/current hero is
Invoker.

If the selected hero is not Invoker:

- cycling should do nothing
- no audio confirmation should play
- a low-noise log line is acceptable, but not required

This keeps the feature scoped and avoids creating surprising global behavior for
other heroes.

## UI Model

The UI should clearly separate:

1. **editing selection** - which profile is open in the editor
2. **runtime active combo** - which combo profile gameplay will use

For combo profiles, clicking the card should do both:

- open it in the editor
- mark it active

Prep profiles should remain editable, but never show as the active combo.

Recommended UI changes:

- add a visible `Active` badge/marker to the current combo profile
- add a small summary label above or near the list:
  - `Active combo: QE Burst`
- keep the existing selected-editor styling, but ensure the active marker is
  distinct from "currently open in the editor"

This prevents the current ambiguity where a highlighted card looks "selected for
play" when it is actually only selected for editing.

## Lightweight Feedback Model

The feedback should stay intentionally lightweight and non-overlay.

### Always-on Feedback

1. **In-app status**
   - the app shows the current active Invoker combo profile
2. **Activity event**
   - when active combo changes, append an entry such as:
   - `Invoker active combo changed to QE Burst`

### Optional Feedback

3. **Short audio confirmation**
   - a small beep on successful cycle/select
   - recommended as a toggleable setting, off by default unless the user
     explicitly wants it on

### Excluded from v1

- desktop toast notifications
- simulated in-game text or chat output
- any persistent on-top overlay window

The intended feel is:

`cycle key -> short confirmation -> active combo changes -> activity log records it -> UI shows the new active combo`

## Overlay Findings

The repository already has a good way to expose app/runtime state to the Tauri
frontend:

- `AppState`
- `app_state_update` emission

That is enough for in-app status and lightweight feedback.

The repository does **not** currently include:

- transparent overlay window infrastructure
- always-on-top gameplay status window handling
- any Invoker-specific runtime profile status surface outside the main app UI

Because of that, an overlay should be treated as a separate later project, not
part of this design.

This document does **not** claim an overlay would be ban-safe. Without a much
more deliberate review of implementation strategy and game compatibility, the
overlay path is not recommended.

## Components and Boundaries

### `src/state/app_state.rs`

Add runtime-owned Invoker combo selection state here or in an equally central
runtime state surface.

Responsibilities:

- hold the current active Invoker combo profile id
- repair invalid active selection when needed
- expose enough state for the UI/event layer to display it

### `src/input/keyboard.rs`

Extend keyboard planning to support:

- the Invoker cycle hotkey
- per-profile combo hotkeys that also update active combo state
- preserving prep-profile direct execution behavior

### `src/actions/heroes/invoker.rs`

Replace the "first enabled combo profile" fallback with active-profile lookup.

Responsibilities:

- execute the explicitly requested profile id when one is provided
- resolve generic combo trigger to the current active combo profile
- fall back safely when the active id is missing or invalid

### `src-ui/src/components/heroes/configs/InvokerConfig.tsx`

Update the UI so combo-profile clicks mark the combo active, not just selected
for editing.

### Tauri/UI event bridge

Use existing app-state emission patterns to surface current active combo state
to the desktop UI.

## Error Handling

Behavior should remain explicit and observable.

### Invalid active combo state

If the stored active combo profile:

- no longer exists
- is disabled
- is no longer a combo profile

then runtime should repair it to the first enabled combo profile and emit a
lightweight activity event.

### No eligible combo profiles

If no enabled combo profiles exist:

- active combo becomes `None`
- cycle hotkey becomes a no-op
- generic combo trigger becomes a no-op with a clear log/activity message

### Non-Invoker hero

If the selected hero is not Invoker:

- cycle hotkey does nothing
- Invoker-specific active combo state remains unchanged

## Testing

Add coverage for:

- UI click on a combo profile marks it active
- prep-profile click does not change active combo
- per-profile combo hotkey sets active combo and executes that profile
- per-profile prep hotkey executes without changing active combo
- cycle hotkey skips disabled combo profiles
- cycle hotkey wraps correctly
- generic combo trigger uses active combo instead of first enabled combo
- deleting/disabling the active combo repairs state correctly
- no enabled combo profiles results in a clear no-op path
- non-Invoker hero ignores the cycle hotkey

## Documentation Impact

If implemented, update:

- `docs/heroes/invoker.md`
- `docs/reference/configuration.md`

The docs should explicitly explain:

- clicking a combo profile in the UI now makes it active
- only one combo profile is active at a time
- cycling considers enabled combo profiles only
- per-profile combo hotkeys still execute directly and also update active combo

## Suggested Implementation Order

1. Add runtime active-combo state and fallback repair logic
2. Change generic trigger resolution from "first enabled" to "active combo"
3. Add cycle-hotkey handling
4. Update per-profile combo hotkeys to set active combo before execution
5. Update UI markers and click behavior
6. Add lightweight feedback and docs
