# Snapfire Directional Cookie Leap — Design

**Date:** 2026-06-06
**Status:** Approved design, pending implementation plan
**Hero:** Snapfire (`npc_dota_hero_snapfire`)

---

## Problem

Snapfire's **Firesnap Cookie** (W) makes her leap a fixed distance in the
direction she is facing when self-cast. Manually doing this mid-fight requires
turning the hero toward the target direction and then self-casting in one quick
motion, which is hard to execute reliably.

We want a single keypress that:

1. Turns Snapfire toward the mouse cursor.
2. Self-casts Firesnap Cookie so she leaps in that direction.

The W key must remain **untouched** so the player can still cast Cookie on
allies manually.

## Goals

- Add a Snapfire hero script driven by **keyboard interception**.
- Trigger key is **Space** (configurable), not W.
- The combo faces the cursor and self-casts Cookie via the ALT self-cast
  modifier.
- Behavior is gated so it only runs when Snapfire is the active hero.
- All keys and timing are config-driven.

## Non-Goals

- No GSI cooldown/ability-ready gating. The combo always fires when Snapfire is
  active (explicit user choice — keep it a pure keyboard combo).
- No changes to W's normal targeted behavior.
- No new GSI fields consumed.

---

## Approach

Mirror the existing **Shadow Fiend raze** interception pattern
(`src/actions/heroes/shadow_fiend.rs` + the SF branch in
`src/input/keyboard.rs`), which already proves out the
"block key → ALT + right-click to face → press ability" technique on a
dedicated worker.

### Input sequence

When Snapfire is active and the trigger key is pressed:

1. Keyboard hook **blocks** the original trigger key (returns `None` from the
   `rdev::grab` callback).
2. Hook enqueues one request onto a dedicated Snapfire worker.
3. Worker executes, using `src/input/simulation.rs` helpers:
   - `alt_down()` — engage the self-cast / face modifier
   - `mouse_click()` — right-click to face Snapfire toward the cursor
   - sleep `turn_delay_ms` — let the hero rotate to face the cursor
   - `press_key(cookie_key)` — self-cast Firesnap Cookie (ALT still held → leap)
   - `alt_up()` — release the modifier

ALT is held across both the right-click and the W press, so the same modifier
both sets facing and self-casts. This removes the need for a W double-tap.

`SIMULATING_KEYS` (managed by `src/input/simulation.rs`) prevents the synthetic
right-click and W press from being re-intercepted by the global hook.

### Activation gating

Follow the Shadow Fiend / Outworld Destroyer model:

- Add `snapfire_enabled: Arc<Mutex<bool>>` to `AppState`.
- Set it `true` when `selected_hero == Some(HeroType::Snapfire)` (UI manual
  override) or when GSI detects `npc_dota_hero_snapfire`, matching how
  `sf_enabled` / `od_enabled` are toggled in
  `AppState::set_selected_hero(...)`.
- Mirror it into the cached `KeyboardSnapshot` (`snapfire_enabled` plus a
  `SnapfireKeyboardSnapshot` holding the parsed trigger key, cookie key, and
  `turn_delay_ms`), refreshed per frame in `Dota2ScriptApp::update(...)`.

The keyboard callback only runs the Snapfire branch when
`snapshot.snapfire_enabled && snapshot.snapfire.enabled`. This keeps the Space
intercept from conflicting with Broodmother's Space + right-click handling,
since the branches are gated by different active heroes.

### Decision-tree placement

Add the Snapfire branch in `src/input/keyboard.rs` alongside the other
hero-specific intercepts (Shadow Fiend / Outworld Destroyer region), before the
generic ability-key / standalone-combo paths. Because the trigger defaults to
**Space**, place the check so it does not interfere with the existing
`MODIFIER_KEY_HELD` Space tracking used by Broodmother — read the snapshot,
confirm `snapfire_enabled`, then block and enqueue.

---

## Components

| File | Change |
|---|---|
| `src/actions/heroes/snapfire.rs` | **New.** `SnapfireScript` implementing `HeroScript`; dedicated request worker; `execute_cookie_leap(trigger config)` enqueue helper running the ALT + right-click + W sequence. `handle_gsi_event` runs only shared survivability; `handle_standalone_trigger` may map to the same combo or be a no-op. |
| `src/actions/heroes/mod.rs` | `pub mod snapfire;` + `pub use snapfire::SnapfireScript;` |
| `src/actions/dispatcher.rs` | Register `SnapfireScript` under `npc_dota_hero_snapfire` in `new()`. |
| `src/config/settings.rs` | New `SnapfireConfig`; add to `HeroesConfig`; `#[serde(default)]` helpers; `impl Default`. |
| `config/config.toml` | New `[heroes.snapfire]` block. |
| `src/state/app_state.rs` | Add `HeroType::Snapfire`; extend `from_hero_name`, `to_display_name`; add `snapfire_enabled` flag and toggle it in `set_selected_hero`. |
| `src/input/keyboard.rs` | `SnapfireKeyboardSnapshot` struct; `snapfire_enabled` + `snapfire` fields on `KeyboardSnapshot`; populate in `from_runtime`; new interception branch. |
| `src/ui/app.rs` | Snapfire manual-override button + keybinding hint line. |

### Docs

| File | Change |
|---|---|
| `docs/heroes/snapfire.md` | **New**, from `docs/heroes/hero-template.md`. |
| `docs/reference/file-index.md` | Add `src/actions/heroes/snapfire.rs` row. |
| `docs/reference/configuration.md` | Add `[heroes.snapfire]` section. |
| `docs/features/keyboard-interception.md` | Note the Snapfire Space intercept in the decision tree + Largo/Broodmother-style notes. |
| `AGENTS.md` | Add Snapfire to supported heroes, Hero Docs table, and Code Map. |

---

## Configuration

`[heroes.snapfire]` in `config/config.toml`:

```toml
[heroes.snapfire]
enabled = true
trigger_key = "Space"   # Key that triggers the directional cookie combo
cookie_key = "w"        # Firesnap Cookie ability key (self-cast via ALT)
turn_delay_ms = 60      # Wait after right-click for the hero to face the cursor
```

`SnapfireConfig` fields:

| Field | Type | Default | Purpose |
|---|---|---|---|
| `enabled` | `bool` | `true` | Master toggle for the Snapfire combo intercept. |
| `trigger_key` | `String` | `"Space"` | Key intercepted to start the combo. |
| `cookie_key` | `String` | `"w"` | Ability key self-cast for Firesnap Cookie. |
| `turn_delay_ms` | `u64` | `60` | Delay between the facing right-click and the self-cast press. |

**UI decision:** `enabled` and the Snapfire manual-override selection are
operator-facing and exposed in `src/ui/app.rs`. The key/timing fields stay
config-file only for now, consistent with how Shadow Fiend's
`raze_delay_ms` is handled.

---

## Data Flow

```
Space pressed (Snapfire active)
  └─ rdev::grab callback (src/input/keyboard.rs)
       ├─ read KeyboardSnapshot once
       ├─ snapfire_enabled && snapfire.enabled && key == trigger_key?
       │     ├─ yes → block key (return None) + enqueue SnapfireRequest::CookieLeap
       │     └─ no  → fall through to existing branches
       └─ Snapfire worker (src/actions/heroes/snapfire.rs)
            alt_down → mouse_click → sleep(turn_delay_ms) → press(cookie_key) → alt_up
```

GSI events for `npc_dota_hero_snapfire` still route through the dispatcher to
`SnapfireScript::handle_gsi_event`, which only applies shared survivability
(common actions). The directional cookie is purely keyboard-driven.

---

## Error Handling / Edge Cases

- **Space modifier conflict:** Space is tracked as `MODIFIER_KEY_HELD` and used
  by Broodmother's Space + right-click. Gating the Snapfire branch on
  `snapfire_enabled` ensures only one hero's Space behavior is active at a time.
- **Self-reinterception:** `SIMULATING_KEYS` guards the synthetic right-click
  and W press, same as every other simulated combo.
- **Worker queue closed:** follow the existing dedicated-worker fallback pattern
  (spawn a short-lived thread with identical logic) so a dropped queue does not
  silently swallow the combo.
- **Turn rate:** if `turn_delay_ms` is too low the leap may fire before Snapfire
  finishes rotating; the value is configurable so the player can tune it.

---

## Testing

- `cargo test` — existing suite must stay green; add a `KeyboardSnapshot` unit
  test asserting Snapfire fields populate from `Settings` + `AppState`
  (mirroring the existing `sf_enabled` snapshot tests in `keyboard.rs`).
- `cargo build --release`.
- Manual in-game verification: select Snapfire, press the trigger key, confirm
  she faces the cursor and leaps that direction, and that pressing W normally
  still targets allies.

---

## Verification Checklist

- [ ] Trigger key blocks and runs the combo only when Snapfire is active.
- [ ] W remains usable for manual ally cookies.
- [ ] Leap direction follows the mouse cursor.
- [ ] Config fields parse and drive behavior.
- [ ] Docs + `AGENTS.md` updated per the maintenance contract.
