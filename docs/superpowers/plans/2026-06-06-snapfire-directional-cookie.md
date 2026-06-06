# Snapfire Directional Cookie Leap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Snapfire hero that intercepts a trigger key (default **Space**) and, while ALT is held, right-clicks to face the cursor then self-casts Firesnap Cookie (W) so Snapfire leaps toward the cursor — leaving W free for manual ally cookies.

**Architecture:** Pure keyboard-interception combo modeled on the existing Shadow Fiend raze path. A new `SnapfireScript` (`HeroScript`) owns a dedicated request worker that runs `alt_down → mouse_click → wait → press(cookie) → alt_up`. The keyboard callback gates the intercept on Snapfire being the active hero (via `selected_hero`, set by GSI detection or the Tauri manual-override command). No GSI cooldown gating.

**Tech Stack:** Rust 2021, `rdev` (global hook), `enigo`-backed `src/input/simulation.rs`, `serde`/TOML config, `tracing`.

**Spec:** `docs/superpowers/specs/2026-06-06-snapfire-directional-cookie-design.md`

---

## File Structure

| File | Responsibility | Create/Modify |
|---|---|---|
| `src/models/heroes.rs` | `Hero::Snapfire` + `npc_dota_hero_snapfire` mapping (already present) | No change |
| `src/state/app_state.rs` | `HeroType::Snapfire` enum case + `from_hero_name`/`to_display_name` | Modify |
| `src/input/keyboard.rs` | `"space"` key parse; `SnapfireKeyboardSnapshot`; snapshot fields; decision-tree branch | Modify |
| `src/config/settings.rs` | `SnapfireConfig`, defaults, `HeroesConfig.snapfire` | Modify |
| `config/config.toml` | `[heroes.snapfire]` block | Modify |
| `src/actions/heroes/snapfire.rs` | `SnapfireScript`, worker, `SnapfireState::execute_cookie_leap` | Create |
| `src/actions/heroes/mod.rs` | module + re-export | Modify |
| `src/actions/dispatcher.rs` | register `SnapfireScript` | Modify |
| `docs/heroes/snapfire.md` | hero doc | Create |
| `docs/reference/file-index.md`, `docs/reference/configuration.md`, `docs/features/keyboard-interception.md`, `AGENTS.md` | doc maintenance contract | Modify |

**Activation note:** Following the Meepo pattern (`keyboard.rs:761`), `snapfire_enabled` is computed in `KeyboardSnapshot::from_runtime` directly from `state.selected_hero == Some(HeroType::Snapfire)`. `selected_hero` is already set both by GSI detection (`app_state.rs:143`) and the Tauri manual-override command (`src-tauri/src/commands/state.rs:117`), so **no new `Arc<Mutex<bool>>` flag and no Tauri command change are required.**

---

## Task 1: Add `HeroType::Snapfire`

**Files:**
- Modify: `src/state/app_state.rs` (enum at `:10`, `from_hero_name` at `:42`, `to_display_name` at `:58`)
- Test: `src/state/app_state.rs` (inline `#[cfg(test)]` module)

- [ ] **Step 1: Write the failing test**

Add this test to the existing `#[cfg(test)] mod tests` block in `src/state/app_state.rs`:

```rust
    #[test]
    fn snapfire_hero_type_maps_name_and_display() {
        assert_eq!(
            HeroType::from_hero_name(crate::models::Hero::Snapfire.to_game_name()),
            Some(HeroType::Snapfire)
        );
        assert_eq!(HeroType::Snapfire.to_display_name(), "Snapfire");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dota2_scripts snapfire_hero_type_maps_name_and_display`
Expected: FAIL — `no variant named Snapfire found for enum HeroType`.

- [ ] **Step 3: Add the enum case and mappings**

In `src/state/app_state.rs`, add `Snapfire,` to the `HeroType` enum (after `OutworldDestroyer,`):

```rust
pub enum HeroType {
    Huskar,
    Invoker,
    Largo,
    LegionCommander,
    Meepo,
    OutworldDestroyer,
    ShadowFiend,
    Snapfire,
    Tiny,
}
```

In `from_hero_name`, add this arm before `_ => None,`:

```rust
            name if name == Hero::Snapfire.to_game_name() => Some(HeroType::Snapfire),
```

In `to_display_name`, add this arm before the closing brace of the match:

```rust
            HeroType::Snapfire => "Snapfire",
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p dota2_scripts snapfire_hero_type_maps_name_and_display`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/state/app_state.rs
git commit -m "feat: add Snapfire hero type"
```

---

## Task 2: Parse the `"space"` trigger key

**Files:**
- Modify: `src/input/keyboard.rs` (`parse_key` at `:47`)
- Test: `src/input/keyboard.rs` (inline `#[cfg(test)]` module)

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `src/input/keyboard.rs`:

```rust
    #[test]
    fn parse_key_string_parses_space() {
        assert_eq!(parse_key_string("Space"), Some(Key::Space));
        assert_eq!(parse_key_string("space"), Some(Key::Space));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dota2_scripts parse_key_string_parses_space`
Expected: FAIL — `assertion failed: ... left: None, right: Some(Space)`.

- [ ] **Step 3: Add the `"space"` arm**

In `src/input/keyboard.rs`, in `fn parse_key`, add this arm right after the `"home" => Some(Key::Home),` line:

```rust
        "space" => Some(Key::Space),
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p dota2_scripts parse_key_string_parses_space`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/input/keyboard.rs
git commit -m "feat: parse Space key string in keyboard listener"
```

---

## Task 3: Add `SnapfireConfig`

**Files:**
- Modify: `src/config/settings.rs` (config structs, default helpers near `:948`, `HeroesConfig` at `:507`, `impl Default for HeroesConfig` at `:1959`)
- Test: `src/config/settings.rs` (inline `#[cfg(test)]` module)

- [ ] **Step 1: Write the failing test**

Add a test module entry (or extend the existing test module) in `src/config/settings.rs`:

```rust
    #[test]
    fn snapfire_config_defaults_are_directional_cookie() {
        let cfg = SnapfireConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.trigger_key, "Space");
        assert_eq!(cfg.cookie_key, 'w');
        assert_eq!(cfg.turn_delay_ms, 60);
    }
```

If `src/config/settings.rs` has no `#[cfg(test)] mod tests`, add one at the end of the file:

```rust
#[cfg(test)]
mod snapfire_config_tests {
    use super::*;

    #[test]
    fn snapfire_config_defaults_are_directional_cookie() {
        let cfg = SnapfireConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.trigger_key, "Space");
        assert_eq!(cfg.cookie_key, 'w');
        assert_eq!(cfg.turn_delay_ms, 60);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dota2_scripts snapfire_config_defaults_are_directional_cookie`
Expected: FAIL — `cannot find type SnapfireConfig`.

- [ ] **Step 3: Add the struct, defaults, and `HeroesConfig` field**

In `src/config/settings.rs`, add the struct near the other hero configs (e.g. after `OutworldDestroyerConfig`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapfireConfig {
    /// Master toggle for the directional cookie intercept.
    #[serde(default = "default_snapfire_enabled")]
    pub enabled: bool,
    /// Key intercepted to start the combo (default Space).
    #[serde(default = "default_snapfire_trigger_key")]
    pub trigger_key: String,
    /// Firesnap Cookie ability key, self-cast via ALT.
    #[serde(default = "default_snapfire_cookie_key")]
    pub cookie_key: char,
    /// Delay between the facing right-click and the self-cast press (ms).
    #[serde(default = "default_snapfire_turn_delay_ms")]
    pub turn_delay_ms: u64,
}
```

Add the default helpers near the other `default_*` fns (around `:948`):

```rust
fn default_snapfire_enabled() -> bool {
    true
}
fn default_snapfire_trigger_key() -> String {
    "Space".to_string()
}
fn default_snapfire_cookie_key() -> char {
    'w'
}
fn default_snapfire_turn_delay_ms() -> u64 {
    60
}
```

Add the `impl Default` near the other hero-config `Default` impls (e.g. after `impl Default for OutworldDestroyerConfig`):

```rust
impl Default for SnapfireConfig {
    fn default() -> Self {
        Self {
            enabled: default_snapfire_enabled(),
            trigger_key: default_snapfire_trigger_key(),
            cookie_key: default_snapfire_cookie_key(),
            turn_delay_ms: default_snapfire_turn_delay_ms(),
        }
    }
}
```

Add the field to `HeroesConfig` (after `pub meepo: MeepoConfig,` at `:525`):

```rust
    #[serde(default)]
    pub snapfire: SnapfireConfig,
```

Add to `impl Default for HeroesConfig` (after `meepo: MeepoConfig::default(),` at `:1970`):

```rust
            snapfire: SnapfireConfig::default(),
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p dota2_scripts snapfire_config_defaults_are_directional_cookie`
Expected: PASS.

- [ ] **Step 5: Add the `[heroes.snapfire]` block to `config/config.toml`**

Append this block in the heroes section of `config/config.toml` (e.g. after the `[heroes.meepo]` block):

```toml
[heroes.snapfire]
# Press the trigger key to face the cursor and self-cast Firesnap Cookie (leap toward cursor).
# W stays free for manually cookie-ing allies.
enabled = true
# Key intercepted to start the combo.
trigger_key = "Space"
# Firesnap Cookie ability key (self-cast via ALT).
cookie_key = "w"
# Delay after the facing right-click before the self-cast leap (ms). Increase if the leap fires before the hero finishes turning.
turn_delay_ms = 60
```

- [ ] **Step 6: Commit**

```bash
git add src/config/settings.rs config/config.toml
git commit -m "feat: add Snapfire config surface"
```

---

## Task 4: Create the Snapfire hero script

**Files:**
- Create: `src/actions/heroes/snapfire.rs`
- Modify: `src/actions/heroes/mod.rs`
- Modify: `src/actions/dispatcher.rs` (import at `:2-5`, registration near `:145`)
- Test: `src/actions/heroes/snapfire.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing test (in the new file)**

Create `src/actions/heroes/snapfire.rs` with the full implementation below. It includes the test:

```rust
use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::HeroScript;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

/// Work item for the dedicated Snapfire worker thread.
#[derive(Debug, PartialEq, Eq)]
enum SnapfireRequest {
    CookieLeap { cookie_key: char, turn_delay_ms: u64 },
}

fn build_cookie_leap_request(cookie_key: char, turn_delay_ms: u64) -> SnapfireRequest {
    SnapfireRequest::CookieLeap {
        cookie_key,
        turn_delay_ms,
    }
}

static SNAPFIRE_REQUEST_QUEUE: LazyLock<mpsc::Sender<SnapfireRequest>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<SnapfireRequest>();

    thread::spawn(move || {
        info!("🍪 Snapfire request worker started");

        while let Ok(request) = rx.recv() {
            run_snapfire_request(request);
        }

        info!("🍪 Snapfire request worker exited");
    });

    tx
});

fn run_snapfire_request(request: SnapfireRequest) {
    match request {
        SnapfireRequest::CookieLeap {
            cookie_key,
            turn_delay_ms,
        } => run_cookie_leap_request(cookie_key, turn_delay_ms),
    }
}

/// Directional Firesnap Cookie leap.
///
/// ALT is held across both the facing right-click and the cookie press so the
/// same modifier turns Snapfire toward the cursor and self-casts the leap.
fn run_cookie_leap_request(cookie_key: char, turn_delay_ms: u64) {
    thread::sleep(Duration::from_millis(50));

    crate::input::simulation::alt_down();
    crate::input::simulation::mouse_click();

    thread::sleep(Duration::from_millis(turn_delay_ms));
    crate::input::simulation::press_key(cookie_key);

    crate::input::simulation::alt_up();
}

fn spawn_snapfire_fallback(request: SnapfireRequest) {
    thread::spawn(move || {
        run_snapfire_request(request);
    });
}

fn enqueue_snapfire_request(request: SnapfireRequest) {
    if let Err(err) = SNAPFIRE_REQUEST_QUEUE.send(request) {
        warn!("🍪 Snapfire request queue unavailable; using fallback thread");
        spawn_snapfire_fallback(err.0);
    }
}

pub struct SnapfireState;

impl SnapfireState {
    /// Run the directional cookie combo: ALT down → right-click to face cursor →
    /// wait `turn_delay_ms` → self-cast cookie → ALT up.
    pub fn execute_cookie_leap(cookie_key: char, turn_delay_ms: u64) {
        enqueue_snapfire_request(build_cookie_leap_request(cookie_key, turn_delay_ms));
    }
}

/// Snapfire script.
///
/// Directional cookie flow:
/// 1. keyboard.rs intercepts the trigger key (default Space) when Snapfire is
///    the active hero.
/// 2. Calls `SnapfireState::execute_cookie_leap()`.
/// 3. The dedicated worker holds ALT, right-clicks to face the cursor, waits,
///    self-casts Firesnap Cookie, and releases ALT.
pub struct SnapfireScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl SnapfireScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }
}

impl HeroScript for SnapfireScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        let settings = self.settings.lock().unwrap();

        // Shared survivability only — the directional cookie is keyboard-driven.
        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    fn handle_standalone_trigger(&self) {
        let settings = self.settings.lock().unwrap();
        let cookie_key = settings.heroes.snapfire.cookie_key;
        let turn_delay_ms = settings.heroes.snapfire.turn_delay_ms;
        drop(settings);
        info!("🍪 Snapfire standalone cookie leap triggered");
        SnapfireState::execute_cookie_leap(cookie_key, turn_delay_ms);
    }

    fn hero_name(&self) -> &'static str {
        Hero::Snapfire.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_cookie_leap_request_preserves_key_and_delay() {
        let request = build_cookie_leap_request('w', 60);
        assert_eq!(
            request,
            SnapfireRequest::CookieLeap {
                cookie_key: 'w',
                turn_delay_ms: 60,
            }
        );
    }
}
```

- [ ] **Step 2: Register the module**

In `src/actions/heroes/mod.rs`, add `pub mod snapfire;` (keep alphabetical-ish, after `shadow_fiend`) and the re-export `pub use snapfire::SnapfireScript;`:

```rust
pub mod shadow_fiend;
pub mod snapfire;
pub mod tiny;
```

```rust
pub use shadow_fiend::ShadowFiendScript;
pub use snapfire::SnapfireScript;
pub use tiny::TinyScript;
```

- [ ] **Step 3: Run test to verify it fails (then passes after registration)**

Run: `cargo test -p dota2_scripts build_cookie_leap_request_preserves_key_and_delay`
Expected: PASS (the module now compiles and the test passes). If compilation fails first, fix the reported import/path mismatch, then rerun.

- [ ] **Step 4: Register the dispatcher**

In `src/actions/dispatcher.rs`, add `SnapfireScript` to the import list at `:2-5`:

```rust
use crate::actions::heroes::{
    BroodmotherScript, HeroScript, HuskarScript, InvokerScript, LargoScript,
    LegionCommanderScript, MeepoScript, OutworldDestroyerScript, ShadowFiendScript,
    SnapfireScript, TinyScript,
};
```

In `ActionDispatcher::new`, add the registration after the Meepo registration (`:145-146`):

```rust
        let snapfire = Arc::new(SnapfireScript::new(settings.clone(), executor.clone()));
        hero_scripts.insert(snapfire.hero_name().to_string(), snapfire);
```

- [ ] **Step 5: Run the full crate test build to verify wiring compiles**

Run: `cargo test -p dota2_scripts --no-run`
Expected: Compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add src/actions/heroes/snapfire.rs src/actions/heroes/mod.rs src/actions/dispatcher.rs
git commit -m "feat: add Snapfire hero script and dispatch registration"
```

---

## Task 5: Wire the keyboard interception

**Files:**
- Modify: `src/input/keyboard.rs` (imports `:13-17`; snapshot structs `:562`; `KeyboardSnapshot` `:609`; `from_runtime` `:748`; `Default` `:823`; `broodmother_test_snapshot` `:973`; decision tree near `:432`)
- Test: `src/input/keyboard.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `src/input/keyboard.rs`:

```rust
    #[test]
    fn keyboard_snapshot_populates_snapfire_fields_for_snapfire() {
        let mut state = AppState::default();
        state.selected_hero = Some(HeroType::Snapfire);
        let snapshot = KeyboardSnapshot::from_runtime(&Settings::default(), &state);

        assert!(snapshot.snapfire_enabled);
        assert!(snapshot.snapfire.enabled);
        assert_eq!(snapshot.snapfire.trigger_key, Some(Key::Space));
        assert_eq!(snapshot.snapfire.cookie_key, 'w');

        let other = KeyboardSnapshot::from_runtime(&Settings::default(), &AppState::default());
        assert!(!other.snapfire_enabled);
    }
```

(`HeroType` is already imported in that test module — see the existing Meepo test at `:1094`.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dota2_scripts keyboard_snapshot_populates_snapfire_fields_for_snapfire`
Expected: FAIL — `no field snapfire_enabled on type KeyboardSnapshot`.

- [ ] **Step 3: Add the snapshot struct**

In `src/input/keyboard.rs`, add after `ShadowFiendKeyboardSnapshot` (`:567`):

```rust
/// Snapshot of the Snapfire keyboard-relevant config.
#[derive(Debug, Clone)]
pub struct SnapfireKeyboardSnapshot {
    pub enabled: bool,
    /// Pre-parsed trigger key (default Space).
    pub trigger_key: Option<Key>,
    pub cookie_key: char,
    pub turn_delay_ms: u64,
}
```

- [ ] **Step 4: Add fields to `KeyboardSnapshot`**

In `struct KeyboardSnapshot` (`:609`), add after `pub broodmother: BroodmotherKeyboardSnapshot,` (`:625`):

```rust
    /// Whether Snapfire is the active hero (drives the cookie intercept).
    pub snapfire_enabled: bool,
    pub snapfire: SnapfireKeyboardSnapshot,
```

- [ ] **Step 5: Populate in `from_runtime`**

In `from_runtime` (`:748`), add a binding near the other `let sf = ...` lines (`:754-756`):

```rust
        let sp = &settings.heroes.snapfire;
```

Then add these fields to the returned `Self { ... }` after the `broodmother: BroodmotherKeyboardSnapshot { ... },` block (`:805`):

```rust
            snapfire_enabled: state.selected_hero == Some(crate::state::HeroType::Snapfire),
            snapfire: SnapfireKeyboardSnapshot {
                enabled: sp.enabled,
                trigger_key: parse_key_string(&sp.trigger_key),
                cookie_key: sp.cookie_key,
                turn_delay_ms: sp.turn_delay_ms,
            },
```

- [ ] **Step 6: Populate in `Default`**

In `impl Default for KeyboardSnapshot` (`:823`), add after the `broodmother: BroodmotherKeyboardSnapshot { ... },` block (`:854`):

```rust
            snapfire_enabled: false,
            snapfire: SnapfireKeyboardSnapshot {
                enabled: false,
                trigger_key: None,
                cookie_key: 'w',
                turn_delay_ms: 0,
            },
```

- [ ] **Step 7: Populate in `broodmother_test_snapshot`**

In the test helper `broodmother_test_snapshot` (`:973`), add after the `broodmother: BroodmotherKeyboardSnapshot { ... },` block (`:1003`), before `soul_ring: ...`:

```rust
            snapfire_enabled: false,
            snapfire: SnapfireKeyboardSnapshot {
                enabled: false,
                trigger_key: None,
                cookie_key: 'w',
                turn_delay_ms: 0,
            },
```

- [ ] **Step 8: Run test to verify it passes**

Run: `cargo test -p dota2_scripts keyboard_snapshot_populates_snapfire_fields_for_snapfire`
Expected: PASS.

- [ ] **Step 9: Add the import and the decision-tree branch**

In `src/input/keyboard.rs`, add the import after `use crate::actions::heroes::shadow_fiend::ShadowFiendState;` (`:16`):

```rust
use crate::actions::heroes::snapfire::SnapfireState;
```

Add the intercept branch immediately after the Shadow Fiend `R` ultimate block (after `:432`, before the `if snapshot.od_enabled {` block at `:434`):

```rust
                // Handle Snapfire directional Firesnap Cookie combo.
                // Trigger (default Space): ALT + right-click to face cursor, then
                // self-cast Cookie so Snapfire leaps toward the cursor. W stays
                // free for manual ally cookies.
                if snapshot.snapfire_enabled && snapshot.snapfire.enabled {
                    if let Some(trigger) = snapshot.snapfire.trigger_key {
                        if key == trigger {
                            info!("🍪 Snapfire trigger pressed - directional cookie leap");
                            SnapfireState::execute_cookie_leap(
                                snapshot.snapfire.cookie_key,
                                snapshot.snapfire.turn_delay_ms,
                            );
                            // Block original key (combo handles the cookie cast).
                            return None;
                        }
                    }
                }
```

- [ ] **Step 10: Verify the whole crate compiles and tests pass**

Run: `cargo test -p dota2_scripts`
Expected: All tests pass, no compile errors.

- [ ] **Step 11: Commit**

```bash
git add src/input/keyboard.rs
git commit -m "feat: intercept Snapfire trigger key for directional cookie leap"
```

---

## Task 6: Documentation

**Files:**
- Create: `docs/heroes/snapfire.md`
- Modify: `docs/reference/file-index.md`, `docs/reference/configuration.md`, `docs/features/keyboard-interception.md`, `AGENTS.md`

- [ ] **Step 1: Create `docs/heroes/snapfire.md`**

Use `docs/heroes/hero-template.md` as the base and fill it with the real behavior. Required content:

```markdown
# Snapfire Automation

## Purpose

Directional **Firesnap Cookie** (W) leap on a single keypress.
**Read this when:** changing the Snapfire trigger key, the facing technique, or the leap timing.

## Feature Summary

- Trigger key (default **Space**) is intercepted while Snapfire is the active hero.
- Combo: `ALT down → right-click (face cursor) → wait turn_delay_ms → press W (self-cast) → ALT up`.
- ALT is held across the right-click and the W press, so the same modifier faces the hero and self-casts the cookie, making her leap toward the cursor.
- **W is not intercepted** — manual ally cookies still work normally.
- No GSI cooldown gating; the combo always fires when Snapfire is active.

## Configuration

`config/config.toml` under `[heroes.snapfire]`:

| Field | Type | Default | Purpose |
|---|---|---|---|
| `enabled` | bool | `true` | Master toggle for the intercept. |
| `trigger_key` | string | `"Space"` | Key intercepted to start the combo. |
| `cookie_key` | char | `"w"` | Firesnap Cookie ability key, self-cast via ALT. |
| `turn_delay_ms` | u64 | `60` | Delay after the facing right-click before the self-cast leap. |

## Related Files

| File | Purpose |
|---|---|
| `src/actions/heroes/snapfire.rs` | Hero script, worker, `SnapfireState::execute_cookie_leap`. |
| `src/input/keyboard.rs` | Trigger-key interception branch + `SnapfireKeyboardSnapshot`. |
| `src/config/settings.rs` | `SnapfireConfig` + defaults. |
| `config/config.toml` | `[heroes.snapfire]` block. |

## Activation

`snapfire_enabled` is derived from `selected_hero == Some(HeroType::Snapfire)`, which is set by GSI hero detection or the manual-override selection.

## Limitations

- Space is also tracked as the auto-items modifier and used by Broodmother's Space + right-click. The intercept is gated on Snapfire being active, so only one hero's Space behavior is live at a time.
- If `turn_delay_ms` is too low the leap may fire before Snapfire finishes turning — increase it to taste.

## Logging

Look for `🍪 Snapfire` log lines (trigger press, worker start/exit, queue fallback).
```

- [ ] **Step 2: Update `docs/reference/file-index.md`**

Add a row for `src/actions/heroes/snapfire.rs` → "Snapfire directional Firesnap Cookie automation" → links `docs/heroes/snapfire.md`, mirroring the existing hero rows.

- [ ] **Step 3: Update `docs/reference/configuration.md`**

Add a `[heroes.snapfire]` section documenting `enabled`, `trigger_key`, `cookie_key`, `turn_delay_ms` with their defaults (matching Task 3).

- [ ] **Step 4: Update `docs/features/keyboard-interception.md`**

In the decision-tree section, add a Snapfire step after the Shadow Fiend ultimate intercept: "Snapfire directional cookie — if `snapfire_enabled` and `[heroes.snapfire].enabled`, block the trigger key (default Space) and enqueue the ALT + right-click + W self-cast onto the Snapfire worker." Add a short Snapfire entry to the "Largo and Broodmother notes" area describing the worker.

- [ ] **Step 5: Update `AGENTS.md`**

- Add Snapfire to the "Supported heroes" line.
- Add a Hero Docs table row: `Snapfire | npc_dota_hero_snapfire | docs/heroes/snapfire.md | src/actions/heroes/snapfire.rs`.
- Add `actions/heroes/snapfire.rs` to the Code Map `src/actions/` table.
- Add a "Read Before Editing" row pointing `src/actions/heroes/snapfire.rs` at `docs/heroes/snapfire.md` and `docs/features/keyboard-interception.md`.

- [ ] **Step 6: Commit**

```bash
git add docs/heroes/snapfire.md docs/reference/file-index.md docs/reference/configuration.md docs/features/keyboard-interception.md AGENTS.md
git commit -m "docs: document Snapfire directional cookie automation"
```

---

## Task 7: Full verification

- [ ] **Step 1: Run the full test suite**

Run: `cargo test`
Expected: All tests pass (workspace-wide, including `src-tauri`).

- [ ] **Step 2: Release build**

Run: `cargo build --release`
Expected: Builds with no errors.

- [ ] **Step 3: Manual smoke check (optional, requires Dota 2)**

```powershell
$env:RUST_LOG="debug"; cargo run --release
```

Then in-game as Snapfire: press the trigger key and confirm Snapfire faces the cursor and leaps that direction; confirm pressing W normally still targets allies. Watch for `🍪 Snapfire` debug lines.

- [ ] **Step 4: Final commit (if any docs/cleanup remain)**

```bash
git add -A
git commit -m "chore: finalize Snapfire directional cookie feature"
```

---

## Self-Review Notes

- **Spec coverage:** trigger=Space (Tasks 2,3,5), ALT+right-click+W self-cast (Task 4), W untouched (no W intercept added), hero gating (Task 1 + Task 5 `snapfire_enabled`), config-driven keys/timing (Task 3), docs (Task 6). All spec sections map to a task.
- **Type consistency:** `SnapfireState::execute_cookie_leap(cookie_key: char, turn_delay_ms: u64)` is defined in Task 4 and called identically in Task 5. `SnapfireKeyboardSnapshot` fields (`enabled`, `trigger_key`, `cookie_key`, `turn_delay_ms`) are defined and consumed consistently across Steps in Task 5. `SnapfireConfig` field names match `from_runtime` reads.
- **Activation simplification:** Uses `selected_hero` directly (Meepo pattern) rather than a new `Arc<Mutex<bool>>`, so no Tauri command change is required; GSI detection already sets `selected_hero`.
- **Gotcha captured:** `parse_key` had no `"space"` arm (Task 2 adds it) — without it the default `trigger_key = "Space"` would silently never match.
