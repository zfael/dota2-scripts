# Phase Boots Movement Automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add shared Phase Boots automation that triggers on real walking, ignores stationary farming jitter, and re-triggers on long travel when the item becomes ready again.

**Architecture:** Extend the shared item-automation registry with a `Movement` trigger family plus a minimal cached movement snapshot, then evaluate Phase Boots in a new dispatcher-owned `SurvivabilityActions::check_and_use_movement_items(...)` pre-hook. Wire the feature through `Settings`, add checked-in TOML defaults, and update the survivability/config/GSI docs so `hero.xpos` and `hero.ypos` are documented as active runtime inputs.

**Tech Stack:** Rust 2021, serde/TOML config, tracing, shared action executor, Cargo test/build, Markdown docs

---

## File Structure

- `src/actions/item_automation.rs`
  - Add the `Movement` trigger family
  - Register `item_phase_boots` as a supported shared automation item
  - Store the last movement snapshot used by shared movement gating
  - Expose small snapshot helpers for runtime code and unit tests
- `src/actions/common.rs`
  - Add the movement distance gate and Phase Boots eligibility helper
  - Add `check_and_use_movement_items(...)`
  - Add unit tests for first-sample, dead-zone, exclusion, and retrigger behavior
  - Add a test-only call counter, matching the existing low-mana pre-hook pattern
- `src/actions/dispatcher.rs`
  - Run the new movement pre-hook after low-mana automation and before hero routing
  - Add a regression test proving the new shared pre-hook runs even for custom hero scripts
- `src/config/settings.rs`
  - Add `PhaseBootsAutomationConfig`
  - Add default functions, `Default` impls, `Settings` field wiring, and a defaults regression test
- `config/config.toml`
  - Add checked-in `[phase_boots_automation]` defaults
- `docs/features/survivability.md`
  - Document the new movement automation section and config touchpoint
- `docs/reference/configuration.md`
  - Add a `[phase_boots_automation]` table
- `docs/reference/gsi-schema-and-usage.md`
  - Mark `hero.xpos` and `hero.ypos` as runtime-consumed by shared movement automation

## Task 1: Add config and item-registry scaffolding

**Files:**
- Modify: `src/actions/item_automation.rs`
- Modify: `src/config/settings.rs`
- Test: `src/actions/item_automation.rs`
- Test: `src/config/settings.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests before changing the implementation:

```rust
// src/actions/item_automation.rs
#[test]
fn lookup_returns_phase_boots_as_supported_movement_item() {
    let boots = lookup_item_automation("item_phase_boots").unwrap();

    assert_eq!(boots.trigger_family, TriggerFamily::Movement);
    assert_eq!(boots.cast_mode, CastMode::NoTarget);
    assert_eq!(boots.support, SupportStatus::Supported);
    assert!(!boots.is_neutral);
}
```

```rust
// src/config/settings.rs
#[test]
fn phase_boots_automation_defaults_are_exposed_through_settings() {
    let settings = Settings::default();

    assert!(settings.phase_boots_automation.enabled);
    assert_eq!(settings.phase_boots_automation.minimum_distance_units, 100);
    assert!(settings.phase_boots_automation.excluded_heroes.is_empty());
}
```

- [ ] **Step 2: Run the targeted tests to verify they fail**

Run:

```powershell
cargo test lookup_returns_phase_boots_as_supported_movement_item --lib
cargo test phase_boots_automation_defaults_are_exposed_through_settings --lib
```

Expected:

- the item-automation test fails because `TriggerFamily::Movement` and the Phase Boots registry entry do not exist yet
- the settings test fails because `phase_boots_automation` does not exist yet

- [ ] **Step 3: Write the minimal implementation**

Add the new trigger family and config wiring:

```rust
// src/actions/item_automation.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerFamily {
    Danger,
    LowMana,
    Movement,
}

pub const ITEM_AUTOMATION_SPECS: &[ItemAutomationSpec] = &[
    // existing specs...
    ItemAutomationSpec {
        item_name: "item_phase_boots",
        trigger_family: TriggerFamily::Movement,
        cast_mode: CastMode::NoTarget,
        support: SupportStatus::Supported,
        is_neutral: false,
    },
];
```

```rust
// src/config/settings.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseBootsAutomationConfig {
    #[serde(default = "default_phase_boots_automation_enabled")]
    pub enabled: bool,
    #[serde(default = "default_phase_boots_minimum_distance_units")]
    pub minimum_distance_units: u32,
    #[serde(default = "default_phase_boots_excluded_heroes")]
    pub excluded_heroes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // existing fields...
    #[serde(default)]
    pub phase_boots_automation: PhaseBootsAutomationConfig,
}

fn default_phase_boots_automation_enabled() -> bool {
    true
}

fn default_phase_boots_minimum_distance_units() -> u32 {
    100
}

fn default_phase_boots_excluded_heroes() -> Vec<String> {
    vec![]
}

impl Default for PhaseBootsAutomationConfig {
    fn default() -> Self {
        Self {
            enabled: default_phase_boots_automation_enabled(),
            minimum_distance_units: default_phase_boots_minimum_distance_units(),
            excluded_heroes: default_phase_boots_excluded_heroes(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // existing defaults...
            phase_boots_automation: PhaseBootsAutomationConfig::default(),
        }
    }
}
```

- [ ] **Step 4: Run the targeted tests to verify they pass**

Run:

```powershell
cargo test lookup_returns_phase_boots_as_supported_movement_item --lib
cargo test phase_boots_automation_defaults_are_exposed_through_settings --lib
```

Expected:

- both tests pass

- [ ] **Step 5: Commit**

Run:

```powershell
git add src/actions/item_automation.rs src/config/settings.rs
git commit -m "feat: add phase boots automation config and registry"
```

## Task 2: Add movement snapshot storage and eligibility gating

**Files:**
- Modify: `src/actions/item_automation.rs`
- Modify: `src/actions/common.rs`
- Test: `src/actions/common.rs`

- [ ] **Step 1: Write the failing movement-gate tests**

Add these tests to `src/actions/common.rs` near the existing low-mana tests:

```rust
#[test]
fn movement_gate_requires_previous_sample_before_phase_boots_can_trigger() {
    reset_global_lockouts_for_tests();
    replace_movement_snapshot_for_tests(None);

    let mut settings = Settings::default();
    settings.phase_boots_automation.enabled = true;
    settings.phase_boots_automation.minimum_distance_units = 100;

    let mut items = empty_items();
    items.slot0 = GsiItem {
        name: "item_phase_boots".to_string(),
        can_cast: Some(true),
        ..Default::default()
    };

    let mut event = base_event(items);
    event.hero.name = "npc_dota_hero_axe".to_string();
    event.hero.xpos = 1000;
    event.hero.ypos = 1000;

    assert!(eligible_movement_item(&event, &settings).is_none());
}

#[test]
fn movement_gate_rejects_sub_threshold_motion_and_accepts_real_travel() {
    reset_global_lockouts_for_tests();

    let mut settings = Settings::default();
    settings.phase_boots_automation.enabled = true;
    settings.phase_boots_automation.minimum_distance_units = 100;

    let mut items = empty_items();
    items.slot0 = GsiItem {
        name: "item_phase_boots".to_string(),
        can_cast: Some(true),
        ..Default::default()
    };

    replace_movement_snapshot_for_tests(Some(MovementSnapshot {
        hero_name: "npc_dota_hero_axe".to_string(),
        alive: true,
        xpos: 1000,
        ypos: 1000,
    }));

    let mut jitter_event = base_event(items.clone());
    jitter_event.hero.name = "npc_dota_hero_axe".to_string();
    jitter_event.hero.xpos = 1030;
    jitter_event.hero.ypos = 1040;
    assert!(eligible_movement_item(&jitter_event, &settings).is_none());

    let mut travel_event = base_event(items);
    travel_event.hero.name = "npc_dota_hero_axe".to_string();
    travel_event.hero.xpos = 1120;
    travel_event.hero.ypos = 1000;

    let (spec, slot_key) = eligible_movement_item(&travel_event, &settings).unwrap();
    assert_eq!(spec.item_name, "item_phase_boots");
    assert_eq!(slot_key, settings.keybindings.slot0);
}

#[test]
fn movement_gate_respects_phase_boots_excluded_heroes() {
    let mut settings = Settings::default();
    settings.phase_boots_automation.enabled = true;
    settings.phase_boots_automation.excluded_heroes =
        vec!["npc_dota_hero_huskar".to_string()];

    let mut items = empty_items();
    items.slot0 = GsiItem {
        name: "item_phase_boots".to_string(),
        can_cast: Some(true),
        ..Default::default()
    };

    replace_movement_snapshot_for_tests(Some(MovementSnapshot {
        hero_name: "npc_dota_hero_huskar".to_string(),
        alive: true,
        xpos: 1000,
        ypos: 1000,
    }));

    let mut event = base_event(items);
    event.hero.name = "npc_dota_hero_huskar".to_string();
    event.hero.xpos = 1200;
    event.hero.ypos = 1000;

    assert!(eligible_movement_item(&event, &settings).is_none());
}
```

- [ ] **Step 2: Run the targeted tests to verify they fail**

Run:

```powershell
cargo test movement_gate_requires_previous_sample_before_phase_boots_can_trigger --lib
cargo test movement_gate_rejects_sub_threshold_motion_and_accepts_real_travel --lib
cargo test movement_gate_respects_phase_boots_excluded_heroes --lib
```

Expected:

- the tests fail because the movement snapshot helpers and `eligible_movement_item(...)` do not exist yet

- [ ] **Step 3: Write the minimal implementation**

Add shared movement snapshot storage in `src/actions/item_automation.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovementSnapshot {
    pub hero_name: String,
    pub alive: bool,
    pub xpos: i32,
    pub ypos: i32,
}

lazy_static! {
    static ref LAST_MOVEMENT_SNAPSHOT: Mutex<Option<MovementSnapshot>> = Mutex::new(None);
}

pub fn read_movement_snapshot() -> Option<MovementSnapshot> {
    LAST_MOVEMENT_SNAPSHOT.lock().unwrap().clone()
}

pub fn write_movement_snapshot(snapshot: MovementSnapshot) {
    *LAST_MOVEMENT_SNAPSHOT.lock().unwrap() = Some(snapshot);
}

pub fn clear_movement_snapshot() {
    *LAST_MOVEMENT_SNAPSHOT.lock().unwrap() = None;
}

#[cfg(test)]
pub fn replace_movement_snapshot_for_tests(snapshot: Option<MovementSnapshot>) {
    *LAST_MOVEMENT_SNAPSHOT.lock().unwrap() = snapshot;
}
```

Add the movement gate in `src/actions/common.rs`:

```rust
fn movement_distance_units(previous: &MovementSnapshot, event: &GsiWebhookEvent) -> f64 {
    let dx = (event.hero.xpos - previous.xpos) as f64;
    let dy = (event.hero.ypos - previous.ypos) as f64;
    (dx * dx + dy * dy).sqrt()
}

fn eligible_movement_item(
    event: &GsiWebhookEvent,
    settings: &Settings,
) -> Option<(&'static ItemAutomationSpec, char)> {
    if !settings.phase_boots_automation.enabled {
        return None;
    }
    if !event.hero.is_alive() {
        return None;
    }
    if hero_is_excluded(
        &event.hero.name,
        &settings.phase_boots_automation.excluded_heroes,
    ) {
        return None;
    }

    let previous = read_movement_snapshot()?;
    if !previous.alive || previous.hero_name != event.hero.name {
        return None;
    }
    if movement_distance_units(&previous, event)
        < settings.phase_boots_automation.minimum_distance_units as f64
    {
        return None;
    }

    for (slot, item) in event.items.all_slots() {
        if item.name != "item_phase_boots" || item.can_cast != Some(true) {
            continue;
        }

        let spec = lookup_item_automation(&item.name)?;
        if spec.trigger_family != TriggerFamily::Movement {
            continue;
        }
        if spec.support != SupportStatus::Supported {
            continue;
        }

        let key = settings.get_key_for_slot(slot)?;
        return Some((spec, key));
    }

    None
}

pub fn check_and_use_movement_items(&self, event: &GsiWebhookEvent) {
    let settings = self.settings.lock().unwrap();
    let snapshot = MovementSnapshot {
        hero_name: event.hero.name.clone(),
        alive: event.hero.alive,
        xpos: event.hero.xpos,
        ypos: event.hero.ypos,
    };

    let decision = eligible_movement_item(event, &settings);
    if !event.hero.is_alive() {
        clear_movement_snapshot();
    } else {
        write_movement_snapshot(snapshot);
    }

    let Some((spec, item_key)) = decision else {
        return;
    };

    let lockout_key = format!("movement:{}", spec.item_name);
    let now_ms = current_time_millis();
    if !acquire_item_trigger_lockout(&lockout_key, now_ms, ITEM_AUTOMATION_LOCKOUT_MS) {
        return;
    }

    let sequence =
        plan_automation_key_sequence(spec.cast_mode, item_key, settings.neutral_items.self_cast_key);
    drop(settings);

    self.executor.enqueue("common-movement-item", move || {
        execute_key_sequence(sequence);
    });
}
```

- [ ] **Step 4: Run the targeted tests to verify they pass**

Run:

```powershell
cargo test movement_gate_requires_previous_sample_before_phase_boots_can_trigger --lib
cargo test movement_gate_rejects_sub_threshold_motion_and_accepts_real_travel --lib
cargo test movement_gate_respects_phase_boots_excluded_heroes --lib
```

Expected:

- all three tests pass

- [ ] **Step 5: Commit**

Run:

```powershell
git add src/actions/item_automation.rs src/actions/common.rs
git commit -m "feat: add shared phase boots movement gate"
```

## Task 3: Wire the dispatcher pre-hook and lock in regression coverage

**Files:**
- Modify: `src/actions/common.rs`
- Modify: `src/actions/dispatcher.rs`
- Test: `src/actions/dispatcher.rs`

- [ ] **Step 1: Write the failing dispatcher regression test**

Add this test beside the existing low-mana pre-hook regression in `src/actions/dispatcher.rs`:

```rust
#[test]
fn dispatch_gsi_event_runs_movement_pre_hook_for_custom_hero_scripts() {
    reset_movement_check_call_count_for_tests();

    let settings = Arc::new(Mutex::new(Settings::default()));
    let executor = ActionExecutor::new();
    let mut hero_scripts: HashMap<String, Arc<dyn HeroScript>> = HashMap::new();
    hero_scripts.insert(
        "npc_dota_hero_test".to_string(),
        Arc::new(NoopHeroScript {
            hero_name: "npc_dota_hero_test",
        }),
    );

    let dispatcher = ActionDispatcher {
        hero_scripts,
        executor: executor.clone(),
        survivability: SurvivabilityActions::new(settings, executor),
    };

    let empty_ability = Ability {
        ability_active: false,
        can_cast: false,
        cooldown: 0,
        level: 0,
        name: String::new(),
        passive: false,
        ultimate: false,
    };

    let event = GsiWebhookEvent {
        hero: Hero {
            name: "npc_dota_hero_test".to_string(),
            alive: true,
            health: 100,
            health_percent: 100,
            mana: 50,
            mana_percent: 20,
            max_health: 100,
            max_mana: 200,
            aghanims_scepter: false,
            aghanims_shard: false,
            attributes_level: 0,
            is_break: false,
            buyback_cooldown: 0,
            buyback_cost: 0,
            disarmed: false,
            facet: 0,
            has_debuff: false,
            hexed: false,
            id: 0,
            level: 1,
            magicimmune: false,
            muted: false,
            respawn_seconds: 0,
            silenced: false,
            smoked: false,
            stunned: false,
            talent_1: false,
            talent_2: false,
            talent_3: false,
            talent_4: false,
            talent_5: false,
            talent_6: false,
            talent_7: false,
            talent_8: false,
            xp: 0,
            xpos: 0,
            ypos: 0,
        },
        abilities: Abilities {
            ability0: empty_ability.clone(),
            ability1: empty_ability.clone(),
            ability2: empty_ability.clone(),
            ability3: empty_ability.clone(),
            ability4: empty_ability.clone(),
            ability5: empty_ability,
        },
        items: Items {
            neutral0: Item::default(),
            slot0: Item::default(),
            slot1: Item::default(),
            slot2: Item::default(),
            slot3: Item::default(),
            slot4: Item::default(),
            slot5: Item::default(),
            slot6: Item::default(),
            slot7: Item::default(),
            slot8: Item::default(),
            stash0: Item::default(),
            stash1: Item::default(),
            stash2: Item::default(),
            stash3: Item::default(),
            stash4: Item::default(),
            stash5: Item::default(),
            teleport0: Item::default(),
        },
        map: Map { clock_time: 0 },
        player: None,
    };

    dispatcher.dispatch_gsi_event(&event);
    assert_eq!(movement_check_call_count_for_tests(), 1);
}
```

- [ ] **Step 2: Run the targeted test to verify it fails**

Run:

```powershell
cargo test dispatch_gsi_event_runs_movement_pre_hook_for_custom_hero_scripts --lib
```

Expected:

- the test fails because the movement pre-hook is not called yet and the movement counter helpers do not exist

- [ ] **Step 3: Write the minimal implementation**

Mirror the existing low-mana counter pattern in `src/actions/common.rs`:

```rust
#[cfg(test)]
lazy_static::lazy_static! {
    static ref MOVEMENT_CHECK_CALLS: AtomicUsize = AtomicUsize::new(0);
}

pub fn check_and_use_movement_items(&self, event: &GsiWebhookEvent) {
    #[cfg(test)]
    {
        MOVEMENT_CHECK_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    // existing movement implementation...
}

#[cfg(test)]
pub fn reset_movement_check_call_count_for_tests() {
    MOVEMENT_CHECK_CALLS.store(0, Ordering::SeqCst);
}

#[cfg(test)]
pub fn movement_check_call_count_for_tests() -> usize {
    MOVEMENT_CHECK_CALLS.load(Ordering::SeqCst)
}
```

Then wire the dispatcher:

```rust
// src/actions/dispatcher.rs
self.survivability.check_and_use_mana_items(event);
self.survivability.check_and_use_movement_items(event);
```

And import the new helpers in dispatcher tests:

```rust
use crate::actions::common::{
    low_mana_check_call_count_for_tests, movement_check_call_count_for_tests,
    reset_low_mana_check_call_count_for_tests, reset_movement_check_call_count_for_tests,
    SurvivabilityActions,
};
```

- [ ] **Step 4: Run the targeted regression tests**

Run:

```powershell
cargo test dispatch_gsi_event_runs_low_mana_pre_hook_for_custom_hero_scripts --lib
cargo test dispatch_gsi_event_runs_movement_pre_hook_for_custom_hero_scripts --lib
```

Expected:

- both dispatcher pre-hook tests pass

- [ ] **Step 5: Commit**

Run:

```powershell
git add src/actions/common.rs src/actions/dispatcher.rs
git commit -m "feat: run phase boots automation before hero routing"
```

## Task 4: Add checked-in defaults and update the docs

**Files:**
- Modify: `config/config.toml`
- Modify: `docs/features/survivability.md`
- Modify: `docs/reference/configuration.md`
- Modify: `docs/reference/gsi-schema-and-usage.md`

- [ ] **Step 1: Add the checked-in config section**

Insert this block after `[mana_automation]` in `config/config.toml`:

```toml
[phase_boots_automation]
enabled = true
minimum_distance_units = 100
excluded_heroes = []
```

- [ ] **Step 2: Update the survivability feature doc**

Add a new section in `docs/features/survivability.md` immediately after low-mana automation:

```md
## Movement automation

Shared movement automation is dispatcher-owned and runs before hero routing.

Current movement-supported items:

- `item_phase_boots`

The feature is controlled by `[phase_boots_automation]`.
It only fires when `hero.xpos` / `hero.ypos` move by at least
`phase_boots_automation.minimum_distance_units` between GSI events.
```

Also extend the config touchpoints table:

```md
| `[phase_boots_automation]` | `enabled`, `minimum_distance_units`, `excluded_heroes` |
```

- [ ] **Step 3: Update the configuration and GSI reference docs**

Add this table to `docs/reference/configuration.md` after `[mana_automation]`:

```md
## `[phase_boots_automation]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `enabled` | `true` | `true` | Master switch for shared walking-triggered Phase Boots automation. |
| `minimum_distance_units` | `100` | `100` | Minimum between-sample travel distance required before the runtime treats the hero as truly walking. |
| `excluded_heroes` | `[]` | `[]` | Exact internal hero names skipped by Phase Boots movement automation. |
```

Update `docs/reference/gsi-schema-and-usage.md` in two places:

```md
| `hero.xpos` | `src/actions/common.rs` | Shared Phase Boots movement automation pathing checks |
| `hero.ypos` | `src/actions/common.rs` | Shared Phase Boots movement automation pathing checks |
```

and replace the current prose about positions being unused with:

```md
Fields such as `hero.magicimmune`, `hero.break`, talents, and buyback data are modeled but not currently consumed by runtime logic. `hero.xpos` and `hero.ypos` are now consumed by shared Phase Boots movement automation.
```

- [ ] **Step 4: Run a quick Rust regression pass**

Run:

```powershell
cargo test --lib
```

Expected:

- library tests pass with the new config and docs-aligned behavior in place

- [ ] **Step 5: Commit**

Run:

```powershell
git add config/config.toml docs/features/survivability.md docs/reference/configuration.md docs/reference/gsi-schema-and-usage.md
git commit -m "docs: document phase boots movement automation"
```

## Task 5: Run full verification

**Files:**
- Modify: none
- Test: repo-wide verification commands only

- [ ] **Step 1: Run the Rust test suite**

Run:

```powershell
cargo test
```

Expected:

- the full Rust test suite passes

- [ ] **Step 2: Run the React UI tests**

Run:

```powershell
npm --prefix src-ui test
```

Expected:

- the existing `src-ui` test suite passes without new failures

- [ ] **Step 3: Run the release build**

Run:

```powershell
cargo build --release
```

Expected:

- the release build succeeds

- [ ] **Step 4: Check the worktree state**

Run:

```powershell
git --no-pager status --short
```

Expected:

- no uncommitted changes remain in the dedicated implementation worktree
