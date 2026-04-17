# Invoker Header Profile Display Design

## Problem

The React UI already tracks the active Invoker combo profile and shows it inside
the Invoker config page, but that information is not visible in the always-on
status header.

Today, the header shows:

- GSI connection state
- current hero name
- hero level
- HP and mana bars
- transient combat and rune indicators

That means the operator has to leave the top-level status area and navigate into
the Invoker page to confirm which combo profile is currently active. The user
wants a single glanceable place to verify the active profile while playing
Invoker.

## Goals

- Show the active Invoker combo profile in the top header
- Keep the change small and visually secondary to combat-critical indicators
- Use the existing runtime and config state instead of introducing new state
- Follow the same active-profile validity rules already used by the Invoker
  config page

## Non-Goals

- No profile switching controls in the header
- No new backend fields, events, or persisted settings
- No profile chip for non-Invoker heroes
- No changes to idle header content when no game is active

## Recommended Approach

Compute the Invoker profile label in `App.tsx` and pass it into
`StatusHeader` as a simple optional prop.

This is the best fit because:

- `App.tsx` already composes the header from game and UI store data
- `StatusHeader` stays mostly presentational
- the logic remains easy to test at the composition boundary
- it avoids making a shared layout component subscribe directly to
  Invoker-specific stores

## Alternatives Considered

### 1. Let `StatusHeader` read stores directly

This removes a prop, but it couples a generic layout component to Invoker
configuration and active-combo state. That makes the header harder to reuse and
harder to test in isolation.

### 2. Add a shared selector or hook first

A dedicated helper such as `useInvokerHeaderProfileLabel()` would keep the
lookup logic reusable, but it adds structure that this feature does not yet
need. The current requirement has only one consumer.

## Data Sources

The label should be derived from existing UI state only:

- `game.heroName`
- `invokerActiveComboProfileId`
- `config.heroes.invoker.profiles`

No Rust-side changes are required.

## Resolution Rules

The header label should be resolved with these rules:

1. If there is no active in-game hero, show nothing.
2. If the active hero is not Invoker, show nothing.
3. If the active hero is Invoker and the active combo profile ID resolves to an
   existing profile where:
   - `mode === "combo"`
   - `enabled === true`
   then show `Profile: <profile name>`.
4. If the active hero is Invoker but no valid active combo profile resolves,
   show `Profile: None`.

This intentionally matches the same validity rule already enforced in the
Invoker config page so the header and configuration view cannot disagree about
whether a selected profile is still active.

## UI Placement

The profile should appear in the existing hero identity cluster, beside the hero
name and level pill.

Recommended order:

- hero name
- level pill
- Invoker profile chip

This keeps the profile associated with hero identity rather than with HP, mana,
danger, silence, stun, rune, or respawn indicators, which are more urgent
signals.

## UI Styling

The profile display should be a compact muted chip that visually sits between
the existing level pill and the more dynamic status indicators.

Requirements:

- text format is `Profile: <name>` when resolved
- text format is `Profile: None` when Invoker is active without a valid combo
- long names should truncate rather than forcing the header to reflow
- the chip should not use warning, success, or danger colors that could compete
  with runtime alerts

The chip should feel like informational metadata, not an alert or action target.

## Error Handling and Fallbacks

No explicit error state is needed.

If config data, UI state, or active profile state drift temporarily:

- non-Invoker heroes still show no chip
- Invoker falls back to `Profile: None`

This keeps the header deterministic without inventing a separate loading state.

## Testing

Implementation should add focused React coverage for:

1. `StatusHeader` renders the optional profile chip when provided
2. `StatusHeader` omits the profile chip for non-Invoker and idle cases
3. app-level composition resolves `Profile: <name>` for a valid enabled combo
4. app-level composition resolves `Profile: None` when the active ID is missing,
   disabled, or points to a prep profile
5. existing non-Invoker header behavior remains unchanged

## Scope Check

This is intentionally a single small UI feature:

- one composition lookup in `App.tsx`
- one optional prop on `StatusHeader`
- focused tests around rendering and lookup behavior

It is small enough for one implementation plan and does not require splitting
into additional subprojects.
