# Meepo Phase 2 Design

## Problem statement

Meepo phase 1 added a standalone combo plus danger-gated `Dig` / `MegaMeepo`, but the runtime still reasons from a single GSI hero snapshot. That is enough for coarse combat automation, but it is not enough for clone-aware logic, split-map decisions, or any feature that depends on knowing which Meepo body is where.

Phase 2 should improve **observability first** so later Meepo automation can be built on explicit state instead of hidden assumptions.

## Goal

Create a Meepo-specific observability layer that:

- captures all Meepo-relevant signals that the current GSI payload already provides
- cleanly distinguishes **known** state from **unknown / unavailable** clone state
- centralizes Meepo runtime snapshots so future features do not each re-derive partial logic
- adds operator-facing visibility for debugging and tuning

This phase is intentionally **not** about adding macro automation, clone-position targeting, or interception-heavy micro logic.

## Current constraints

- `GsiWebhookEvent` currently models only:
  - `hero`
  - `abilities`
  - `items`
  - `map`
- The schema includes one hero snapshot only.
- No explicit clone list, clone HP table, clone positions, selected-unit identity, or per-clone inventory context exists in the current model.
- Current Meepo logic in `src/actions/heroes/meepo.rs` works from:
  - hero HP / stun / silence / shard / scepter flags
  - ability readiness by name
  - inventory slots and item castability

## Approaches considered

### 1. Observability-first slice (recommended)

Add a Meepo runtime snapshot layer and UI/debug surfaces first, while keeping behavior changes minimal.

Pros:
- safest against false assumptions
- creates reusable structure for later phases
- makes future debugging and review much easier

Cons:
- less immediately flashy than adding new automation
- phase 2 produces infrastructure plus visibility more than raw power

### 2. Behavior-first with inferred clone assumptions

Add new combat or farming behaviors now by inferring clone state from current hero snapshot, item state, and coarse heuristics.

Pros:
- faster visible automation gains

Cons:
- high risk of incorrect behavior
- bakes fragile assumptions into the codebase
- makes later observability cleanup harder

### 3. Full schema expansion first

Aggressively expand `GsiWebhookEvent` toward additional optional Meepo-adjacent fields before building any new runtime layer.

Pros:
- best long-term if the game actually sends richer Meepo data

Cons:
- speculative without fixture or live-log evidence
- easy to over-model fields the runtime never receives
- delays practical progress

## Recommendation

Use **Approach 1: observability-first**, with one narrow schema extension rule:

> Only extend the Rust GSI model for fields that are confirmed by fixture updates or live GSI logs produced by this repo.

That gives phase 2 a concrete output without inventing clone telemetry the app does not yet have.

## Proposed scope

### In scope

- new Meepo runtime snapshot type
- handler-owned refresh path for Meepo-specific derived state
- clear “known vs unknown” modeling for clone-related signals
- UI visibility for the current Meepo snapshot
- targeted tests for state derivation
- docs for the new state model and its limits

### Out of scope

- new standalone combo behaviors
- clone-position-aware casts
- farm/objective automation
- keyboard interception
- speculative GSI schema additions without evidence

## Proposed architecture

### 1. `MeepoObservedState`

Add a new Meepo-specific derived state object, likely in a new file such as:

- `src/actions/heroes/meepo_state.rs`

This type should be a compact, immutable snapshot derived from one GSI event.

Suggested shape:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeepoObservedState {
    pub hero_name: String,
    pub level: u32,
    pub alive: bool,
    pub in_danger: bool,
    pub health_percent: u32,
    pub mana_percent: u32,
    pub has_shard: bool,
    pub has_scepter: bool,
    pub earthbind_ready: bool,
    pub poof_ready: bool,
    pub dig_ready: bool,
    pub megameepo_ready: bool,
    pub blink_key: Option<char>,
    pub combo_item_keys: Vec<(String, char)>,
    pub known_clone_state: KnownCloneState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownCloneState {
    Unavailable,
}
```

The important design choice is that clone state is represented as **unavailable**, not guessed.

### 2. Handler-owned refresh path

Phase 1 already uses `src/gsi/handler.rs` as the canonical place to refresh keyboard/runtime caches before dispatch. Phase 2 should follow that same ownership model.

Add a Meepo refresh path there so the latest event updates:

- Meepo runtime snapshot cache
- optional Meepo debug/UI snapshot

That prevents duplicate derivation work across `meepo.rs`, UI code, and future behaviors.

### 3. Snapshot cache boundary

Use a single shared cache for the latest Meepo-derived snapshot, similar to the repo's other runtime caches. The cache should:

- store `Option<MeepoObservedState>`
- update only when current hero is Meepo
- reset to `None` when another hero is active

This keeps the UI honest and avoids stale cross-hero state.

### 4. UI surface

Extend the existing UI with a read-only Meepo status block when Meepo is selected or currently detected.

Suggested fields:

- current HP% / mana%
- `in_danger`
- Earthbind / Poof / Dig / MegaMeepo readiness
- shard / scepter presence
- blink detected yes/no
- combo items currently detected
- clone state: `Unavailable from current GSI`

This gives operators immediate feedback without promising clone visibility that does not exist.

## Data flow

```text
GSI event
  -> process_gsi_events()
  -> refresh_keyboard_runtime_state(...)
  -> derive MeepoObservedState if hero == Meepo
  -> cache latest MeepoObservedState
  -> update AppState.last_event as usual
  -> dispatcher routes to MeepoScript
  -> MeepoScript may read MeepoObservedState later instead of re-deriving state piecemeal
```

## Behavior changes in phase 2

Phase 2 should keep behavior changes intentionally small:

- `MeepoScript` may switch from ad-hoc readiness checks to reading the derived snapshot
- no new gameplay automation should be added unless it depends only on already-known signals

This keeps the phase focused and reduces churn.

## Testing strategy

### Unit tests

Add deterministic tests for Meepo state derivation:

- shard / scepter detection
- ability readiness detection
- combo item key detection
- cache reset when hero changes
- clone state marked `Unavailable`

### Fixture coverage

Reuse `tests/fixtures/meepo_event.json` for derivation tests, mutating copies in tests where needed.

If live logs later reveal extra Meepo-relevant GSI fields, add a new fixture instead of mutating the existing phase-1 contract silently.

### Verification

Run:

- `cargo test`
- `cargo build --release --target-dir target\\release-verify`

## Risks and mitigations

### Risk: phase 2 becomes a stealth behavior phase

Mitigation:
- keep new automation out of scope
- prioritize snapshot derivation and UI visibility only

### Risk: developers treat unknown clone state as “probably same as prime”

Mitigation:
- encode unknown state explicitly in types
- document it in code and docs

### Risk: over-modeling speculative GSI fields

Mitigation:
- only add new schema fields when backed by fixture or live-log evidence

## Success criteria

Phase 2 is successful when:

1. The repo has a Meepo-specific observed-state layer with tests.
2. The handler refreshes that state from GSI events.
3. The UI can show Meepo readiness/debug information from the derived snapshot.
4. The codebase explicitly encodes that clone-aware state is currently unavailable.
5. No new fragile automation logic is added on top of assumptions.

## Deferred to later phases

- actual clone list modeling, if live GSI evidence supports it
- clone-position-aware item/ability logic
- farming routes
- Tormentor / Roshan planning
- interception-assisted combo execution

## Review notes

This design intentionally chooses infrastructure over new behavior because the current app still lacks explicit clone telemetry. That trade-off is deliberate: later Meepo automation will be safer, easier to test, and easier to explain if phase 2 first creates a trusted observed-state layer.
