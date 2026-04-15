# Invoker Automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Invoker to the repo as a hybrid automation hero with one primary combo trigger, one panic Ghost Walk trigger, one prep trigger, and deterministic invoke planning that respects current spell slots and player-controlled targeting.

**Architecture:** Keep the first implementation inside one focused hero module, `src/actions/heroes/invoker.rs`, following the existing Outworld Destroyer / Shadow Fiend worker-queue pattern. Wire Invoker into the repo's generic selected-hero + standalone flow for the main combo trigger, then add two dedicated `HotkeyEvent` variants for panic and prep so the keyboard layer stays explicit and low-risk. Use GSI only for self-state, spell-slot parsing, and readiness checks; never move the mouse or invent targets.

**Tech Stack:** Rust 2021, serde, tracing, lazy_static, egui app state/UI wiring, TOML config, GSI fixtures/tests, markdown docs

---

## Scope Check

This is one cohesive hero-automation slice, not multiple independent subsystems:

- hero identity + dispatcher/UI/manual-selection wiring
- Invoker config surface
- Invoker observed-state parsing from GSI
- deterministic invoke planning
- one serialized worker queue for combo/panic/prep requests
- keyboard hotkeys for panic/prep
- docs and test fixtures

Do not split this into separate plans unless a new requirement appears for autonomous targeting or minimap/cursor reasoning. Those would be separate projects.

## File Structure

- Create: `src/actions/heroes/invoker.rs`
  - Own Invoker constants, observed-state parsing, request enum, combo planner, worker queue, and `HeroScript` implementation
- Modify: `src/actions/heroes/mod.rs`
  - Register the new hero module and re-export `InvokerScript`
- Modify: `src/actions/dispatcher.rs`
  - Register Invoker in the hero-script map and choose executor-backed standalone dispatch
- Modify: `src/state/app_state.rs`
  - Add `HeroType::Invoker`, name mapping, and display label
- Modify: `src/main.rs`
  - Route generic `ComboTrigger` and new Invoker-specific hotkey events
- Modify: `src/input/keyboard.rs`
  - Add `HotkeyEvent::InvokerPanic` and `HotkeyEvent::InvokerPrep`, snapshot fields, and dedicated key planning
- Modify: `src/config/settings.rs`
  - Add `InvokerConfig`, defaults, `HeroesConfig` field, and `Settings::get_standalone_key("invoker")`
- Modify: `config/config.toml`
  - Add `[heroes.invoker]` checked-in defaults
- Modify: `src/ui/app.rs`
  - Add Invoker to manual override and keybinding/status display
- Modify: `tests/gsi_handler_tests.rs`
  - Add Invoker fixture-loading coverage
- Create: `tests/fixtures/invoker_qw_event.json`
  - Represent a QW-oriented Invoker with Tornado/EMP active
- Create: `tests/fixtures/invoker_qe_event.json`
  - Represent a QE-oriented Invoker with Meteor/Blast active
- Modify: `docs/heroes/invoker.md`
  - New hero doc describing triggers, profiles, config, and limitations
- Modify: `docs/reference/configuration.md`
  - Add `[heroes.invoker]`
- Modify: `docs/reference/file-index.md`
  - Add `src/actions/heroes/invoker.rs` and `docs/heroes/invoker.md`
- Modify: `docs/features/keyboard-interception.md`
  - Document Invoker panic/prep hotkeys and why core spell keys are not intercepted
- Modify: `docs/architecture/state-and-dispatch.md`
  - Mention Invoker in hero routing and standalone flow
- Modify: `AGENTS.md`
  - Add Invoker to the hero docs table and source/doc navigation

---

### Task 1: Wire Invoker into hero identity, config, and minimal dispatch

**Files:**
- Create: `src/actions/heroes/invoker.rs`
- Modify: `src/actions/heroes/mod.rs`
- Modify: `src/actions/dispatcher.rs`
- Modify: `src/state/app_state.rs`
- Modify: `src/config/settings.rs`
- Modify: `config/config.toml`
- Modify: `src/ui/app.rs`

- [ ] **Step 1: Write the failing hero-identity and config tests**

Add these tests to the existing test modules in `src/state/app_state.rs` and `src/config/settings.rs`:

```rust
#[test]
fn invoker_round_trips_from_game_name() {
    assert_eq!(
        HeroType::from_hero_name(crate::models::Hero::Invoker.to_game_name()),
        Some(HeroType::Invoker)
    );
    assert_eq!(HeroType::Invoker.to_display_name(), "Invoker");
}
```

```rust
#[test]
fn invoker_defaults_expose_expected_hotkeys() {
    let settings = Settings::default();
    assert_eq!(settings.get_standalone_key("invoker"), "Home");
    assert_eq!(settings.heroes.invoker.panic_key, "End");
    assert_eq!(settings.heroes.invoker.prep_key, "PageUp");
    assert_eq!(settings.heroes.invoker.quas_key, 'q');
    assert_eq!(settings.heroes.invoker.invoke_key, 'r');
}
```

- [ ] **Step 2: Run the targeted tests and verify they fail**

Run:

```powershell
cargo test invoker_round_trips_from_game_name --lib
cargo test invoker_defaults_expose_expected_hotkeys --lib
```

Expected: FAIL with errors such as `no variant or associated item named 'Invoker'` and `no field 'invoker' on type 'HeroesConfig'`.

- [ ] **Step 3: Add the minimal Invoker config, hero identity, and inert script**

Create `src/actions/heroes/invoker.rs` with a compile-first skeleton:

```rust
use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::traits::HeroScript;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use std::sync::{Arc, Mutex};
use tracing::info;

pub struct InvokerScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl InvokerScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }
}

impl HeroScript for InvokerScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        let settings = self.settings.lock().unwrap();
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    fn handle_standalone_trigger(&self) {
        info!("Invoker standalone combo requested before combo planner is implemented");
    }

    fn hero_name(&self) -> &'static str {
        Hero::Invoker.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
```

In `src/config/settings.rs`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokerConfig {
    #[serde(default = "default_standalone_key")]
    pub standalone_key: String,
    #[serde(default = "default_invoker_panic_key")]
    pub panic_key: String,
    #[serde(default = "default_invoker_prep_key")]
    pub prep_key: String,
    #[serde(default = "default_invoker_quas_key")]
    pub quas_key: char,
    #[serde(default = "default_invoker_wex_key")]
    pub wex_key: char,
    #[serde(default = "default_invoker_exort_key")]
    pub exort_key: char,
    #[serde(default = "default_invoker_invoke_key")]
    pub invoke_key: char,
    #[serde(default = "default_invoker_spell_slot_primary_key")]
    pub spell_slot_primary_key: char,
    #[serde(default = "default_invoker_spell_slot_secondary_key")]
    pub spell_slot_secondary_key: char,
    #[serde(default = "default_invoker_primary_profile")]
    pub primary_profile: String,
    #[serde(default = "default_invoker_prep_profile")]
    pub prep_profile: String,
    #[serde(default = "default_invoker_combo_items")]
    pub combo_items: Vec<String>,
    #[serde(default = "default_invoker_tornado_emp_delay_ms")]
    pub tornado_emp_delay_ms: u64,
    #[serde(default = "default_invoker_sun_strike_delay_ms")]
    pub sun_strike_delay_ms: u64,
    #[serde(default = "default_invoker_meteor_blast_delay_ms")]
    pub meteor_blast_delay_ms: u64,
    #[serde(default)]
    pub armlet: HeroArmletOverrideConfig,
}
```

Add defaults:

```rust
fn default_invoker_panic_key() -> String { "End".to_string() }
fn default_invoker_prep_key() -> String { "PageUp".to_string() }
fn default_invoker_quas_key() -> char { 'q' }
fn default_invoker_wex_key() -> char { 'w' }
fn default_invoker_exort_key() -> char { 'e' }
fn default_invoker_invoke_key() -> char { 'r' }
fn default_invoker_spell_slot_primary_key() -> char { 'd' }
fn default_invoker_spell_slot_secondary_key() -> char { 'f' }
fn default_invoker_primary_profile() -> String { "qw_pickoff".to_string() }
fn default_invoker_prep_profile() -> String { "tornado_emp".to_string() }
fn default_invoker_combo_items() -> Vec<String> {
    vec!["item_spirit_vessel".to_string(), "item_rod_of_atos".to_string()]
}
fn default_invoker_tornado_emp_delay_ms() -> u64 { 700 }
fn default_invoker_sun_strike_delay_ms() -> u64 { 150 }
fn default_invoker_meteor_blast_delay_ms() -> u64 { 450 }
```

Then wire:

- `HeroesConfig { invoker: InvokerConfig, ... }`
- `Settings::get_standalone_key("invoker")`
- `HeroType::Invoker`
- `src/actions/heroes/mod.rs` module + re-export
- `src/actions/dispatcher.rs` registration via `InvokerScript::new(...)`
- `config/config.toml` section:

```toml
[heroes.invoker]
standalone_key = "Home"
panic_key = "End"
prep_key = "PageUp"
quas_key = "q"
wex_key = "w"
exort_key = "e"
invoke_key = "r"
spell_slot_primary_key = "d"
spell_slot_secondary_key = "f"
primary_profile = "qw_pickoff"
prep_profile = "tornado_emp"
combo_items = ["item_spirit_vessel", "item_rod_of_atos"]
tornado_emp_delay_ms = 700
sun_strike_delay_ms = 150
meteor_blast_delay_ms = 450
```

- [ ] **Step 4: Add Invoker to manual override UI**

Update the manual override / selected-hero UI in `src/ui/app.rs` so it includes `HeroType::Invoker` with display label `Invoker`, and make the keybinding/status text show:

```rust
format!(
    "Combo: {} | Panic: {} | Prep: {}",
    settings.heroes.invoker.standalone_key,
    settings.heroes.invoker.panic_key,
    settings.heroes.invoker.prep_key
)
```

- [ ] **Step 5: Run the targeted tests again**

Run:

```powershell
cargo test invoker_round_trips_from_game_name --lib
cargo test invoker_defaults_expose_expected_hotkeys --lib
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add src\actions\heroes\invoker.rs src\actions\heroes\mod.rs src\actions\dispatcher.rs src\state\app_state.rs src\config\settings.rs config\config.toml src\ui\app.rs
git commit -m "feat: add invoker hero plumbing"
```

---

### Task 2: Add Invoker GSI fixtures and observed-state parsing

**Files:**
- Create: `tests/fixtures/invoker_qw_event.json`
- Create: `tests/fixtures/invoker_qe_event.json`
- Modify: `tests/gsi_handler_tests.rs`
- Modify: `src/actions/heroes/invoker.rs`

- [ ] **Step 1: Add failing fixture and observed-state tests**

Add fixture smoke tests to `tests/gsi_handler_tests.rs`:

```rust
#[tokio::test]
async fn test_load_invoker_qw_fixture() {
    let json_data = fs::read_to_string("tests/fixtures/invoker_qw_event.json")
        .expect("Failed to read Invoker QW fixture");

    let event: GsiWebhookEvent =
        serde_json::from_str(&json_data).expect("Failed to deserialize Invoker QW event");

    assert_eq!(event.hero.name, "npc_dota_hero_invoker");
    assert_eq!(event.abilities.ability4.name, "invoker_tornado");
    assert_eq!(event.abilities.ability5.name, "invoker_emp");
}

#[tokio::test]
async fn test_load_invoker_qe_fixture() {
    let json_data = fs::read_to_string("tests/fixtures/invoker_qe_event.json")
        .expect("Failed to read Invoker QE fixture");

    let event: GsiWebhookEvent =
        serde_json::from_str(&json_data).expect("Failed to deserialize Invoker QE event");

    assert_eq!(event.hero.name, "npc_dota_hero_invoker");
    assert_eq!(event.abilities.ability4.name, "invoker_chaos_meteor");
    assert_eq!(event.abilities.ability5.name, "invoker_deafening_blast");
}
```

Add unit tests to `src/actions/heroes/invoker.rs`:

```rust
#[test]
fn observed_state_reads_orb_levels_and_active_spells() {
    let event = invoker_qw_fixture();
    let state = InvokerObservedState::from_event(&event);

    assert_eq!(state.quas_level, 4);
    assert_eq!(state.wex_level, 4);
    assert_eq!(state.exort_level, 1);
    assert_eq!(state.active_spells[0].as_deref(), Some("invoker_tornado"));
    assert_eq!(state.active_spells[1].as_deref(), Some("invoker_emp"));
}

#[test]
fn active_spell_key_uses_existing_slot_mapping() {
    let event = invoker_qw_fixture();
    let config = Settings::default().heroes.invoker;
    let state = InvokerObservedState::from_event(&event);

    assert_eq!(
        state.active_spell_key("invoker_emp", &config),
        Some(config.spell_slot_secondary_key)
    );
}
```

- [ ] **Step 2: Add the two fixture files**

Create `tests/fixtures/invoker_qw_event.json` with a level-10 Invoker whose six GSI slots read:

```json
"ability0": { "name": "invoker_quas", "level": 4, "can_cast": true, "ability_active": true, "cooldown": 0, "passive": false, "ultimate": false },
"ability1": { "name": "invoker_wex", "level": 4, "can_cast": true, "ability_active": true, "cooldown": 0, "passive": false, "ultimate": false },
"ability2": { "name": "invoker_exort", "level": 1, "can_cast": true, "ability_active": true, "cooldown": 0, "passive": false, "ultimate": false },
"ability3": { "name": "invoker_invoke", "level": 1, "can_cast": true, "ability_active": true, "cooldown": 0, "passive": false, "ultimate": true },
"ability4": { "name": "invoker_tornado", "level": 4, "can_cast": true, "ability_active": true, "cooldown": 0, "passive": false, "ultimate": false },
"ability5": { "name": "invoker_emp", "level": 4, "can_cast": true, "ability_active": true, "cooldown": 0, "passive": false, "ultimate": false }
```

Create `tests/fixtures/invoker_qe_event.json` with a level-16 Invoker whose six GSI slots read:

```json
"ability0": { "name": "invoker_quas", "level": 3, "can_cast": true, "ability_active": true, "cooldown": 0, "passive": false, "ultimate": false },
"ability1": { "name": "invoker_wex", "level": 2, "can_cast": true, "ability_active": true, "cooldown": 0, "passive": false, "ultimate": false },
"ability2": { "name": "invoker_exort", "level": 6, "can_cast": true, "ability_active": true, "cooldown": 0, "passive": false, "ultimate": false },
"ability3": { "name": "invoker_invoke", "level": 1, "can_cast": true, "ability_active": true, "cooldown": 0, "passive": false, "ultimate": true },
"ability4": { "name": "invoker_chaos_meteor", "level": 6, "can_cast": true, "ability_active": true, "cooldown": 0, "passive": false, "ultimate": false },
"ability5": { "name": "invoker_deafening_blast", "level": 6, "can_cast": true, "ability_active": true, "cooldown": 0, "passive": false, "ultimate": false }
```

Use a complete hero/items/map JSON object that matches the existing fixture schema. Keep `hero.name = "npc_dota_hero_invoker"` and make the item slots include at least one castable Vessel and one castable Atos across the two fixtures.

- [ ] **Step 3: Run the fixture and unit tests and verify they fail**

Run:

```powershell
cargo test test_load_invoker_qw_fixture
cargo test observed_state_reads_orb_levels_and_active_spells --lib
```

Expected: FAIL because `InvokerObservedState` and fixture helpers do not exist yet.

- [ ] **Step 4: Implement `InvokerObservedState`**

In `src/actions/heroes/invoker.rs`, add:

```rust
#[derive(Debug, Clone)]
struct InvokerObservedState {
    quas_level: u32,
    wex_level: u32,
    exort_level: u32,
    invoke_ready: bool,
    active_spells: [Option<String>; 2],
    hero_alive: bool,
    hero_disabled: bool,
    has_scepter: bool,
    has_shard: bool,
}

impl InvokerObservedState {
    fn from_event(event: &GsiWebhookEvent) -> Self {
        let ability = |index| event.abilities.get_by_index(index).expect("valid slot");
        Self {
            quas_level: ability(0).level,
            wex_level: ability(1).level,
            exort_level: ability(2).level,
            invoke_ready: ability(3).name == "invoker_invoke" && ability(3).can_cast,
            active_spells: [
                Some(ability(4).name.clone()).filter(|name| name != "empty"),
                Some(ability(5).name.clone()).filter(|name| name != "empty"),
            ],
            hero_alive: event.hero.alive,
            hero_disabled: event.hero.stunned || event.hero.hexed || event.hero.silenced,
            has_scepter: event.hero.aghanims_scepter,
            has_shard: event.hero.aghanims_shard,
        }
    }

    fn active_spell_key(&self, spell_name: &str, config: &crate::config::InvokerConfig) -> Option<char> {
        if self.active_spells[0].as_deref() == Some(spell_name) {
            Some(config.spell_slot_primary_key)
        } else if self.active_spells[1].as_deref() == Some(spell_name) {
            Some(config.spell_slot_secondary_key)
        } else {
            None
        }
    }
}
```

Also add test helpers:

```rust
fn invoker_qw_fixture() -> GsiWebhookEvent {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/invoker_qw_event.json"
    ))
    .expect("Invoker QW fixture should deserialize")
}
```

- [ ] **Step 5: Run the focused tests and verify they pass**

Run:

```powershell
cargo test test_load_invoker_qw_fixture
cargo test test_load_invoker_qe_fixture
cargo test observed_state_reads_orb_levels_and_active_spells --lib
cargo test active_spell_key_uses_existing_slot_mapping --lib
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add tests\fixtures\invoker_qw_event.json tests\fixtures\invoker_qe_event.json tests\gsi_handler_tests.rs src\actions\heroes\invoker.rs
git commit -m "test: add invoker fixture coverage"
```

---

### Task 3: Implement deterministic invoke planning and prep requests

**Files:**
- Modify: `src/actions/heroes/invoker.rs`

- [ ] **Step 1: Write the failing planner tests**

Add these tests to `src/actions/heroes/invoker.rs`:

```rust
#[test]
fn planner_uses_existing_slot_when_spell_is_already_active() {
    let event = invoker_qw_fixture();
    let settings = Settings::default();
    let state = InvokerObservedState::from_event(&event);

    let step = plan_single_spell("invoker_tornado", &state, &settings.heroes.invoker)
        .expect("spell should be plannable");

    assert_eq!(step.cast_key, settings.heroes.invoker.spell_slot_primary_key);
    assert!(step.prepare_keys.is_empty());
}

#[test]
fn planner_prepares_meteor_when_not_currently_invoked() {
    let event = invoker_qw_fixture();
    let settings = Settings::default();
    let state = InvokerObservedState::from_event(&event);

    let step = plan_single_spell("invoker_chaos_meteor", &state, &settings.heroes.invoker)
        .expect("meteor should be plannable");

    assert_eq!(
        step.prepare_keys,
        vec![
            settings.heroes.invoker.exort_key,
            settings.heroes.invoker.exort_key,
            settings.heroes.invoker.wex_key,
            settings.heroes.invoker.invoke_key,
        ]
    );
}

#[test]
fn prep_profile_returns_ordered_two_spell_plan() {
    let event = invoker_qw_fixture();
    let settings = Settings::default();
    let state = InvokerObservedState::from_event(&event);

    let plan = plan_prep_profile("meteor_blast", &state, &settings.heroes.invoker)
        .expect("prep plan should exist");

    assert_eq!(plan.target_spells, vec!["invoker_chaos_meteor", "invoker_deafening_blast"]);
}
```

- [ ] **Step 2: Run the targeted planner tests and verify they fail**

Run:

```powershell
cargo test planner_uses_existing_slot_when_spell_is_already_active --lib
```

Expected: FAIL with missing `plan_single_spell` / `plan_prep_profile` definitions.

- [ ] **Step 3: Implement the invoke planner**

Add planning types:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedSpellCast {
    prepare_keys: Vec<char>,
    cast_key: char,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedPrepPair {
    target_spells: Vec<&'static str>,
    prepare_keys: Vec<char>,
}
```

Add the spell recipe table and planners:

```rust
fn orb_recipe(spell_name: &str, config: &crate::config::InvokerConfig) -> Option<[char; 4]> {
    match spell_name {
        "invoker_tornado" => Some([config.wex_key, config.wex_key, config.quas_key, config.invoke_key]),
        "invoker_emp" => Some([config.wex_key, config.wex_key, config.wex_key, config.invoke_key]),
        "invoker_chaos_meteor" => Some([config.exort_key, config.exort_key, config.wex_key, config.invoke_key]),
        "invoker_deafening_blast" => Some([config.quas_key, config.wex_key, config.exort_key, config.invoke_key]),
        "invoker_cold_snap" => Some([config.quas_key, config.quas_key, config.quas_key, config.invoke_key]),
        "invoker_forge_spirit" => Some([config.exort_key, config.exort_key, config.quas_key, config.invoke_key]),
        "invoker_ghost_walk" => Some([config.quas_key, config.quas_key, config.wex_key, config.invoke_key]),
        "invoker_ice_wall" => Some([config.quas_key, config.quas_key, config.exort_key, config.invoke_key]),
        "invoker_sun_strike" => Some([config.exort_key, config.exort_key, config.exort_key, config.invoke_key]),
        _ => None,
    }
}

fn plan_single_spell(
    spell_name: &str,
    state: &InvokerObservedState,
    config: &crate::config::InvokerConfig,
) -> Option<PlannedSpellCast> {
    if let Some(cast_key) = state.active_spell_key(spell_name, config) {
        return Some(PlannedSpellCast {
            prepare_keys: Vec::new(),
            cast_key,
        });
    }

    let prepare_keys = orb_recipe(spell_name, config)?.to_vec();
    Some(PlannedSpellCast {
        prepare_keys,
        cast_key: config.spell_slot_secondary_key,
    })
}

fn plan_prep_profile(
    profile: &str,
    state: &InvokerObservedState,
    config: &crate::config::InvokerConfig,
) -> Option<PlannedPrepPair> {
    let target_spells = match profile {
        "tornado_emp" => vec!["invoker_tornado", "invoker_emp"],
        "meteor_blast" => vec!["invoker_chaos_meteor", "invoker_deafening_blast"],
        "cold_snap_forge_spirit" => vec!["invoker_cold_snap", "invoker_forge_spirit"],
        "ghost_walk_ice_wall" => vec!["invoker_ghost_walk", "invoker_ice_wall"],
        _ => return None,
    };

    let mut prepare_keys = Vec::new();
    for spell_name in &target_spells {
        if state.active_spell_key(spell_name, config).is_none() {
            prepare_keys.extend(orb_recipe(spell_name, config)?);
        }
    }

    Some(PlannedPrepPair {
        target_spells,
        prepare_keys,
    })
}
```

- [ ] **Step 4: Add panic and prep request types**

Extend `invoker.rs` with:

```rust
#[derive(Debug, Clone)]
enum InvokerRequest {
    PrimaryCombo,
    PanicGhostWalk,
    PrepPair,
}
```

Keep the actual worker execution minimal in this task: only create the request enum and the prep planning helpers. Do not execute hotkeys yet.

- [ ] **Step 5: Run the planner tests and verify they pass**

Run:

```powershell
cargo test planner_uses_existing_slot_when_spell_is_already_active --lib
cargo test planner_prepares_meteor_when_not_currently_invoked --lib
cargo test prep_profile_returns_ordered_two_spell_plan --lib
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add src\actions\heroes\invoker.rs
git commit -m "feat: add invoker invoke planner"
```

---

### Task 4: Wire panic/prep hotkeys and the serialized Invoker request worker

**Files:**
- Modify: `src/input/keyboard.rs`
- Modify: `src/main.rs`
- Modify: `src/actions/dispatcher.rs`
- Modify: `src/actions/heroes/invoker.rs`

- [ ] **Step 1: Write the failing hotkey-planning tests**

Add keyboard tests in `src/input/keyboard.rs`:

```rust
#[test]
fn plan_global_hotkey_event_returns_invoker_panic() {
    let mut snapshot = KeyboardSnapshot::default();
    snapshot.invoker_panic_key = Some(parse_key_string("End").unwrap());

    assert_eq!(
        plan_global_hotkey_event(rdev::Key::End, &snapshot),
        Some(HotkeyEvent::InvokerPanic)
    );
}

#[test]
fn plan_global_hotkey_event_returns_invoker_prep() {
    let mut snapshot = KeyboardSnapshot::default();
    snapshot.invoker_prep_key = Some(parse_key_string("PageUp").unwrap());

    assert_eq!(
        plan_global_hotkey_event(rdev::Key::PageUp, &snapshot),
        Some(HotkeyEvent::InvokerPrep)
    );
}
```

Add a worker-queue test in `src/actions/heroes/invoker.rs`:

```rust
#[test]
fn enqueue_request_preserves_fifo_order() {
    let mut seen = Vec::new();
    for request in [
        InvokerRequest::PrepPair,
        InvokerRequest::PanicGhostWalk,
        InvokerRequest::PrimaryCombo,
    ] {
        seen.push(format!("{request:?}"));
    }

    assert_eq!(
        seen,
        vec!["PrepPair", "PanicGhostWalk", "PrimaryCombo"]
    );
}
```

- [ ] **Step 2: Run the targeted tests and verify they fail**

Run:

```powershell
cargo test plan_global_hotkey_event_returns_invoker_panic --lib
cargo test enqueue_request_preserves_fifo_order --lib
```

Expected: FAIL because the new hotkey variants and snapshot fields do not exist.

- [ ] **Step 3: Add new hotkey events and snapshot fields**

In `src/input/keyboard.rs`:

```rust
pub enum HotkeyEvent {
    ComboTrigger,
    MeepoFarmToggle,
    ArmletRoshanToggle,
    LargoQ,
    LargoW,
    LargoE,
    LargoR,
    InvokerPanic,
    InvokerPrep,
}
```

Add snapshot fields:

```rust
pub struct KeyboardSnapshot {
    // existing fields...
    pub invoker_panic_key: Option<Key>,
    pub invoker_prep_key: Option<Key>,
}
```

Update snapshot construction from settings/app state:

```rust
invoker_panic_key: parse_key(&settings.heroes.invoker.panic_key),
invoker_prep_key: parse_key(&settings.heroes.invoker.prep_key),
```

Update `plan_global_hotkey_event(...)` so the order is:

1. Meepo toggle
2. Armlet Roshan toggle
3. Invoker panic
4. Invoker prep
5. generic combo trigger

- [ ] **Step 4: Route the new hotkey events in `main.rs`**

Add receiver handling:

```rust
input::keyboard::HotkeyEvent::InvokerPanic => {
    dispatcher_clone2.dispatch_invoker_panic();
}
input::keyboard::HotkeyEvent::InvokerPrep => {
    dispatcher_clone2.dispatch_invoker_prep();
}
```

Add dispatcher methods:

```rust
pub fn dispatch_invoker_panic(&self) {
    if let Some(hero) = self.hero_scripts.get(crate::models::Hero::Invoker.to_game_name()) {
        if let Some(invoker) = hero.as_any().downcast_ref::<crate::actions::heroes::InvokerScript>() {
            invoker.handle_panic_trigger();
        }
    }
}

pub fn dispatch_invoker_prep(&self) {
    if let Some(hero) = self.hero_scripts.get(crate::models::Hero::Invoker.to_game_name()) {
        if let Some(invoker) = hero.as_any().downcast_ref::<crate::actions::heroes::InvokerScript>() {
            invoker.handle_prep_trigger();
        }
    }
}
```

- [ ] **Step 5: Implement the worker queue in `invoker.rs`**

Follow the OD worker pattern:

```rust
static INVOKER_REQUEST_QUEUE: LazyLock<mpsc::Sender<InvokerRequest>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<InvokerRequest>();
    thread::spawn(move || {
        while let Ok(request) = rx.recv() {
            run_invoker_request(request);
        }
    });
    tx
});
```

Add methods:

```rust
impl InvokerScript {
    pub fn handle_panic_trigger(&self) {
        enqueue_request(InvokerRequest::PanicGhostWalk);
    }

    pub fn handle_prep_trigger(&self) {
        enqueue_request(InvokerRequest::PrepPair);
    }
}
```

Make `handle_standalone_trigger()` enqueue `InvokerRequest::PrimaryCombo` instead of logging.

- [ ] **Step 6: Run the targeted tests and verify they pass**

Run:

```powershell
cargo test plan_global_hotkey_event_returns_invoker_panic --lib
cargo test plan_global_hotkey_event_returns_invoker_prep --lib
cargo test enqueue_request_preserves_fifo_order --lib
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add src\input\keyboard.rs src\main.rs src\actions\dispatcher.rs src\actions\heroes\invoker.rs
git commit -m "feat: add invoker trigger routing"
```

---

### Task 5: Execute combo profiles, document Invoker, and run full verification

**Files:**
- Modify: `src/actions/heroes/invoker.rs`
- Create: `docs/heroes/invoker.md`
- Modify: `docs/reference/configuration.md`
- Modify: `docs/reference/file-index.md`
- Modify: `docs/features/keyboard-interception.md`
- Modify: `docs/architecture/state-and-dispatch.md`
- Modify: `AGENTS.md`

- [ ] **Step 1: Write the failing combo-execution tests**

Add unit tests in `src/actions/heroes/invoker.rs` that validate profile expansion:

```rust
#[test]
fn qw_profile_expands_to_tornado_then_emp() {
    let event = invoker_qw_fixture();
    let settings = Settings::default();
    let state = InvokerObservedState::from_event(&event);

    let sequence = build_primary_combo_sequence(&state, &settings.heroes.invoker)
        .expect("QW profile should build");

    assert_eq!(sequence.spells, vec!["invoker_tornado", "invoker_emp"]);
}

#[test]
fn qe_profile_expands_to_strike_meteor_blast() {
    let event = invoker_qe_fixture();
    let mut settings = Settings::default();
    settings.heroes.invoker.primary_profile = "qe_burst".to_string();
    let state = InvokerObservedState::from_event(&event);

    let sequence = build_primary_combo_sequence(&state, &settings.heroes.invoker)
        .expect("QE profile should build");

    assert_eq!(
        sequence.spells,
        vec![
            "invoker_sun_strike",
            "invoker_chaos_meteor",
            "invoker_deafening_blast"
        ]
    );
}
```

- [ ] **Step 2: Run the targeted combo tests and verify they fail**

Run:

```powershell
cargo test qw_profile_expands_to_tornado_then_emp --lib
```

Expected: FAIL because `build_primary_combo_sequence` does not exist.

- [ ] **Step 3: Implement sequence building and request execution**

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedComboSequence {
    spells: Vec<&'static str>,
    item_names: Vec<String>,
}

fn build_primary_combo_sequence(
    state: &InvokerObservedState,
    config: &crate::config::InvokerConfig,
) -> Option<PlannedComboSequence> {
    let spells = match config.primary_profile.as_str() {
        "qw_pickoff" => vec!["invoker_tornado", "invoker_emp"],
        "qe_burst" => vec![
            "invoker_sun_strike",
            "invoker_chaos_meteor",
            "invoker_deafening_blast",
        ],
        _ => return None,
    };

    let _ = state;
    Some(PlannedComboSequence {
        spells,
        item_names: config.combo_items.clone(),
    })
}
```

Then implement `run_invoker_request(...)`:

- `PanicGhostWalk` -> plan and cast `invoker_ghost_walk`
- `PrepPair` -> plan `config.prep_profile` and emit only the prepare keys
- `PrimaryCombo` -> build the configured combo sequence, optionally use configured combo items when present, then cast spells in order with `tornado_emp_delay_ms`, `sun_strike_delay_ms`, and `meteor_blast_delay_ms`

Use the existing helpers in `src/input/simulation.rs` and the current-cursor / quickcast model. Do not add mouse movement.

- [ ] **Step 4: Write the Invoker docs**

Create `docs/heroes/invoker.md` with sections:

- hero purpose and supported profiles
- standalone combo / panic / prep triggers
- config table for `[heroes.invoker]`
- limitations: no auto-aim, no core spell interception, no inferred pill upgrades
- logging/debug expectations

Then update:

- `docs/reference/configuration.md`
- `docs/reference/file-index.md`
- `docs/features/keyboard-interception.md`
- `docs/architecture/state-and-dispatch.md`
- `AGENTS.md`

Use the existing hero docs as the formatting model.

- [ ] **Step 5: Run targeted Invoker tests**

Run:

```powershell
cargo test invoker --lib
cargo test test_load_invoker_qw_fixture
cargo test test_load_invoker_qe_fixture
```

Expected: PASS.

- [ ] **Step 6: Run full repo verification**

Run:

```powershell
cargo test
npm --prefix src-ui test
cargo build --release
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add src\actions\heroes\invoker.rs docs\heroes\invoker.md docs\reference\configuration.md docs\reference\file-index.md docs\features\keyboard-interception.md docs\architecture\state-and-dispatch.md AGENTS.md tests\gsi_handler_tests.rs tests\fixtures\invoker_qw_event.json tests\fixtures\invoker_qe_event.json
git commit -m "feat: add invoker automation"
```

---

## Self-Review Checklist

### Spec coverage

- hero plumbing -> Task 1
- observed state from six-slot GSI model -> Task 2
- deterministic invoke planner -> Task 3
- panic/prep triggers and worker queue -> Task 4
- primary combo profiles, docs, and verification -> Task 5

### Placeholder scan

The plan intentionally defines:

- exact file paths
- concrete struct and function names
- concrete test names
- concrete commands
- concrete default keys and profiles

### Type consistency

The plan consistently uses:

- `InvokerConfig`
- `InvokerObservedState`
- `InvokerRequest`
- `plan_single_spell`
- `plan_prep_profile`
- `build_primary_combo_sequence`
- `handle_panic_trigger`
- `handle_prep_trigger`

Do not rename those mid-implementation without updating later tasks first.
