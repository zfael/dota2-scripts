# Invoker Live Profile Refresh Design

## Problem

Editing Invoker combo profiles in the Tauri UI does not reliably take effect
immediately at runtime. The user can add or change profile steps in the editor,
but the automation may continue using the old profile shape until the app is
restarted.

This is especially visible when:

- editing an existing combo profile's steps
- adding a new step to the active combo
- deleting or disabling the current active combo profile

The result is a confusing operator experience: the UI shows one version of the
profile, while the runtime may still execute an older one.

## Goals

- Make Invoker profile edits take effect on the next trigger without restarting
  the app
- Keep the runtime and UI aligned when the active Invoker combo profile is
  edited, disabled, or removed
- Preserve the existing queue-based Invoker execution model
- Limit the fix to the current Invoker/Tauri config flow rather than introducing
  a repo-wide config architecture rewrite

## Non-Goals

- No general live-reload framework for every hero script in this pass
- No rewrite of the global settings storage model
- No redesign of the Invoker profile editor UX
- No changes to the persisted config schema

## Root Cause Summary

The current implementation has two weak points.

### 1. Invoker execution depends on a separate cached settings snapshot

`src/actions/heroes/invoker.rs` stores a global `INVOKER_SETTINGS` snapshot that
is refreshed from `handle_gsi_event()`. The queued request worker later reads
that cached snapshot when executing a profile.

That means Invoker request execution does **not** necessarily plan from the
latest settings at trigger time. A profile edit can persist to the shared
`settings` mutex correctly, yet the worker can still execute from an older
cached copy until another path refreshes it.

### 2. Hero-config edits do not repair active Invoker combo state

`src-tauri/src/commands/config.rs::update_hero_config()` updates the shared
settings and keyboard snapshot, but it does not repair
`app_state.invoker_active_combo_profile_id` against the newly edited profile
list.

That leaves room for drift when the active profile is disabled, deleted, or no
longer a valid combo profile.

## Recommended Approach

Keep the existing Invoker queue worker, but make each queued Invoker profile
request carry the fresh settings snapshot needed to execute that request.

At the same time, repair `app_state.invoker_active_combo_profile_id`
immediately after Invoker hero-config edits.

This is the best fit because:

- it fixes the stale-runtime behavior at the point where it actually matters:
  request execution
- it does not depend on a later GSI tick to refresh cached settings
- it keeps the queue worker model intact
- it uses the existing `AppState::repair_invoker_active_combo()` hook instead of
  inventing new repair logic

## Alternatives Considered

### 1. Keep `INVOKER_SETTINGS`, but refresh it from config commands

This is the smallest patch on paper, but it is fragile:

- it couples Tauri config commands to Invoker internals
- it only fixes paths that happen to go through those commands
- it keeps the stale-cache pattern alive

It would treat the symptom rather than remove the bad dependency.

### 2. Build a general runtime config-broadcast system

A repo-wide live-config propagation layer could eventually help multiple hero
scripts, but it is too much scope for this issue. The current bug is narrow and
well understood.

## Execution Model Changes

### Invoker request payload

Today, the queued request effectively carries only a profile ID and later
depends on global cached state.

The design should change Invoker requests so that a profile execution request is
created from:

- the selected `profile_id`
- a fresh cloned `Settings` snapshot taken at trigger time

The worker can still use `INVOKER_LAST_EVENT` for the current observed game
state, but it should stop depending on a separate stale `INVOKER_SETTINGS`
cache.

### Trigger behavior

Both of these paths should enqueue with fresh settings:

- `InvokerScript::handle_profile_trigger(...)`
- `InvokerScript::handle_standalone_trigger()`

That guarantees the next combo run uses the currently edited profile steps, even
if no new GSI event has arrived between the edit and the trigger.

### GSI path

`handle_gsi_event()` should keep updating `INVOKER_LAST_EVENT`, because the
worker still needs the latest observed spell slots, cooldowns, and hero state.

But `handle_gsi_event()` should no longer be responsible for refreshing the
settings snapshot used by the request worker.

## Tauri Config Repair

After `update_hero_config()` persists new settings, it should detect the Invoker
hero case and repair `app_state.invoker_active_combo_profile_id` using the
already existing `AppState::repair_invoker_active_combo()` behavior.

Repair input should be derived from the newly persisted
`settings.heroes.invoker.profiles` list:

- combo profiles remain eligible only when `enabled == true`
- prep profiles are ignored
- if the current active profile is still valid, keep it
- otherwise fall back to the first enabled combo
- if no enabled combo exists, clear the active ID

This keeps the UI and runtime state aligned immediately after profile edits.

## UI/Event Expectations

No new frontend API is required.

The existing app-state emitter already publishes `app_state_update` whenever the
serialized app state changes. Once `update_hero_config()` repairs the active
Invoker combo ID, the existing emitter loop should propagate that change to the
React UI naturally.

## Error Handling

- If an Invoker request cannot find the referenced profile in the fresh settings
  snapshot, log and skip the request
- If the active combo becomes invalid after editing, repair it immediately
  instead of leaving stale state behind
- If no enabled combo profiles remain, clear the active ID rather than silently
  picking a prep profile

## Testing

Implementation should add focused coverage for:

1. Invoker request construction/execution uses the settings snapshot captured at
   trigger time rather than a stale global cache
2. Editing Invoker profiles in `update_hero_config()` repairs the active combo
   ID when the previous one becomes invalid
3. Valid active Invoker combo IDs survive non-breaking profile edits
4. Disabling or deleting the active combo falls back to the first enabled combo
   or `None` when none remain

## Scope Check

This is still one focused project:

- one runtime fix in Invoker request execution
- one state-repair fix in the Tauri config command path
- targeted tests around those two boundaries

It is small enough for a single implementation plan.
