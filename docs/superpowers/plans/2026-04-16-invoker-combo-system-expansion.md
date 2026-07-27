# Invoker Combo System Expansion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand Invoker's combo system with richer preset defaults, true manual-targeted spell continuation, Cataclysm-style cast behaviors, extra supported combo items, and a configurable combo-cycle hotkey.

**Architecture:** Keep the existing linear Invoker profile runner, but extend each step with an explicit `cast_behavior` so activation semantics and completion semantics stay separate. Add the new behavior to the Rust config model first, mirror it into the React editor/catalog, and then teach the Invoker runtime to execute modifier/double-tap/manual-ready flows while preserving the existing active-combo and optional-item behavior.

**Tech Stack:** Rust (`serde`, existing Invoker runner, keyboard hook, synthetic input worker), TOML config, Tauri-backed React/TypeScript UI, Zustand config store, Vitest, Markdown docs

---

## File Map

**Modify**

- `src/config/settings.rs` — add `InvokerProfileStepCastBehavior`, `cycle_combo_profiles_hotkey`, expanded default profiles, and config tests
- `config/config.toml` — mirror the new Invoker defaults into the checked-in config template
- `src-ui/src/types/config.ts` — mirror the new Invoker schema in the frontend type model
- `src-ui/src/stores/mockData.ts` — keep mock config aligned with the new Invoker defaults
- `src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx` — add new spells/items and expanded preset library data
- `src/actions/heroes/invoker.rs` — carry `cast_behavior` through planning and execute normal/manual/Alt/double-tap casts correctly
- `src/input/keyboard.rs` — read the cycle hotkey from config instead of hardcoding `Delete`
- `src-ui/src/components/heroes/configs/InvokerConfig.tsx` — expose the configurable cycle key in the Invoker core-keys card
- `src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx` — expose `cast_behavior` controls for spell steps
- `src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx` — surface new cast-behavior labels in preset/configured profile summaries
- `docs/heroes/invoker.md` — document new preset pack, cast behaviors, optional item skipping, and configurable cycle key
- `docs/reference/configuration.md` — document the new Invoker fields and their behavior

**Test**

- `src/config/settings.rs` — new Invoker default/config tests
- `src/actions/heroes/invoker.rs` — new cast-behavior planner/runtime tests
- `src/input/keyboard.rs` — new configurable cycle-hotkey tests
- `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx` — new UI tests for cycle key + cast behavior editing
- `cargo test invoker_defaults_expose_cycle_hotkey_and_cast_behavior_defaults --lib`
- `cargo test invoker_defaults_seed_combo_system_profiles --lib`
- `cargo test cast_sequence_for_alt_double_tap_holds_alt_for_both_presses --lib`
- `cargo test manual_wait_cast_behavior_produces_no_auto_cast_sequence --lib`
- `cargo test keyboard_snapshot_uses_configured_invoker_cycle_hotkey --lib`
- `npm --prefix src-ui test -- InvokerConfig.test.tsx`
- `cargo test`
- `npm --prefix src-ui test`
- `cargo build --release`

---

### Task 1: Extend the Invoker config schema and seed the new preset data

**Files:**
- Modify: `src/config/settings.rs`
- Modify: `config/config.toml`
- Modify: `src-ui/src/types/config.ts`
- Modify: `src-ui/src/stores/mockData.ts`
- Modify: `src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx`
- Test: `src/config/settings.rs`

- [ ] **Step 1: Write the failing Rust config tests**

Add these tests to `src/config/settings.rs` near the existing Invoker default tests:

```rust
    #[test]
    fn invoker_defaults_expose_cycle_hotkey_and_cast_behavior_defaults() {
        let settings = Settings::default();
        let invoker = &settings.heroes.invoker;

        assert_eq!(invoker.cycle_combo_profiles_hotkey, "Delete");

        let qe = invoker
            .profiles
            .iter()
            .find(|profile| profile.id == "qe-burst")
            .expect("QE Burst profile should exist");

        let sun_strike = qe.steps.first().expect("QE Burst should have a first step");
        assert_eq!(
            sun_strike.cast_behavior,
            InvokerProfileStepCastBehavior::ManualWaitCooldown
        );
    }

    #[test]
    fn invoker_defaults_seed_combo_system_profiles() {
        let settings = Settings::default();

        let seeded: Vec<_> = settings
            .heroes
            .invoker
            .profiles
            .iter()
            .map(|profile| (profile.id.as_str(), profile.hotkey.as_str(), profile.enabled))
            .collect();

        assert_eq!(
            seeded,
            vec![
                ("qw-pickoff", "Home", true),
                ("qe-burst", "PageDown", false),
                ("ghost-walk-panic", "End", true),
                ("meteor-blast-prep", "PageUp", true),
                ("lane-pressure", "F5", false),
                ("meta-catch", "F6", false),
                ("shotgun-burst", "F7", false),
                ("ice-floe-lockdown", "F8", false),
                ("refresher-sequence", "F9", false),
            ]
        );
    }
```

- [ ] **Step 2: Run the targeted config tests to confirm they fail**

Run:

```powershell
cargo test invoker_defaults_expose_cycle_hotkey_and_cast_behavior_defaults --lib
```

Expected: FAIL because `cycle_combo_profiles_hotkey` and `InvokerProfileStepCastBehavior` do not exist yet.

- [ ] **Step 3: Add the Rust/TypeScript schema and seeded data**

In `src/config/settings.rs`, add the new enum, default helpers, and fields:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvokerProfileStepCastBehavior {
    Normal,
    ManualWaitCooldown,
    AltCast,
    DoubleTap,
    AltDoubleTap,
}

fn default_invoker_profile_step_cast_behavior() -> InvokerProfileStepCastBehavior {
    InvokerProfileStepCastBehavior::Normal
}

fn default_invoker_cycle_combo_profiles_hotkey() -> String {
    "Delete".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvokerProfileStep {
    pub kind: InvokerProfileStepKind,
    pub target: String,
    #[serde(default)]
    pub delay_after_ms: u64,
    #[serde(default = "default_invoker_profile_step_completion_mode")]
    pub completion_mode: InvokerProfileStepCompletionMode,
    #[serde(default = "default_invoker_profile_step_completion_timeout_ms")]
    pub completion_timeout_ms: u64,
    #[serde(default = "default_invoker_profile_step_cast_behavior")]
    pub cast_behavior: InvokerProfileStepCastBehavior,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokerConfig {
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
    #[serde(default = "default_invoker_cycle_combo_profiles_hotkey")]
    pub cycle_combo_profiles_hotkey: String,
    #[serde(default = "default_invoker_profiles")]
    pub profiles: Vec<InvokerProfile>,
    #[serde(default)]
    pub armlet: HeroArmletOverrideConfig,
}
```

In `src-ui/src/types/config.ts`, mirror the shape:

```ts
export type InvokerProfileStepCastBehavior =
  | "normal"
  | "manual_wait_cooldown"
  | "alt_cast"
  | "double_tap"
  | "alt_double_tap";

export interface InvokerProfileStep {
  kind: InvokerProfileStepKind;
  target: string;
  delay_after_ms: number;
  completion_mode: InvokerProfileStepCompletionMode;
  completion_timeout_ms: number;
  cast_behavior: InvokerProfileStepCastBehavior;
  notes: string;
}

export interface InvokerConfig {
  quas_key: string;
  wex_key: string;
  exort_key: string;
  invoke_key: string;
  spell_slot_primary_key: string;
  spell_slot_secondary_key: string;
  cycle_combo_profiles_hotkey: string;
  profiles: InvokerProfile[];
  armlet: HeroArmletOverride;
}
```

Seed these exact new profiles in **both** `default_invoker_profiles()` and `INVOKER_PRESET_PROFILES`, and mirror them into `config/config.toml` and `src-ui/src/stores/mockData.ts`:

```text
lane-pressure
  hotkey: F5
  enabled: false
  mode: combo
  build_tag: qe
  steps:
    - invoker_forge_spirit / normal / fixed_delay / 150ms

meta-catch
  hotkey: F6
  enabled: false
  mode: combo
  build_tag: qw
  steps:
    - invoker_tornado / normal / fixed_delay / 700ms
    - invoker_emp / normal / fixed_delay / 100ms
    - invoker_cold_snap / normal / fixed_delay / 100ms

shotgun-burst
  hotkey: F7
  enabled: false
  mode: combo
  build_tag: qe
  steps:
    - item_rod_of_atos / normal / fixed_delay / 50ms
    - invoker_sun_strike / manual_wait_cooldown / wait_for_cooldown / 150ms / timeout 3000ms
    - invoker_chaos_meteor / normal / fixed_delay / 450ms
    - invoker_deafening_blast / normal / fixed_delay / 100ms

ice-floe-lockdown
  hotkey: F8
  enabled: false
  mode: combo
  build_tag: qe
  steps:
    - invoker_ice_wall / normal / fixed_delay / 2500ms
    - invoker_chaos_meteor / normal / fixed_delay / 450ms

refresher-sequence
  hotkey: F9
  enabled: false
  mode: combo
  build_tag: general
  steps:
    - invoker_tornado / normal / fixed_delay / 700ms
    - invoker_emp / normal / fixed_delay / 100ms
    - invoker_chaos_meteor / normal / fixed_delay / 350ms
    - invoker_deafening_blast / normal / fixed_delay / 100ms
    - item_refresher / normal / fixed_delay / 100ms
    - invoker_sun_strike / alt_double_tap / fixed_delay / 100ms
    - invoker_chaos_meteor / normal / fixed_delay / 350ms
    - invoker_deafening_blast / normal / fixed_delay / 100ms
```

Also make these catalog additions in `src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx`:

```ts
{ id: "item_cyclone", label: "Eul's", kind: "item", icon: chip("EU", "bg-sky-700") },
{ id: "item_refresher", label: "Refresher", kind: "item", icon: chip("RF", "bg-emerald-800") },
```

Do the same `cast_behavior` field addition on every existing step in:

- `default_invoker_profiles()`
- `config/config.toml`
- `src-ui/src/stores/mockData.ts`
- `INVOKER_PRESET_PROFILES`

Use `cast_behavior = "normal"` everywhere except:

- QE Burst Sun Strike -> `manual_wait_cooldown`
- Refresher Sequence Sun Strike -> `alt_double_tap`

- [ ] **Step 4: Run the targeted config tests again**

Run:

```powershell
cargo test invoker_defaults_seed_combo_system_profiles --lib
```

Expected: PASS.

- [ ] **Step 5: Commit the schema/defaults slice**

Run:

```powershell
git add src/config/settings.rs config/config.toml src-ui/src/types/config.ts src-ui/src/stores/mockData.ts src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx
git commit -m "feat: expand invoker combo config defaults"
```

---

### Task 2: Teach the Invoker runner how to execute cast behaviors

**Files:**
- Modify: `src/actions/heroes/invoker.rs`
- Test: `src/actions/heroes/invoker.rs`

- [ ] **Step 1: Write the failing cast-behavior tests**

Add these tests to `src/actions/heroes/invoker.rs` near the existing manual-wait tests:

```rust
    #[test]
    fn cast_sequence_for_alt_double_tap_holds_alt_for_both_presses() {
        assert_eq!(
            cast_sequence_for_behavior('d', &InvokerProfileStepCastBehavior::AltDoubleTap),
            vec![
                CastSequenceAction::AltDown,
                CastSequenceAction::Press('d'),
                CastSequenceAction::SleepMs(50),
                CastSequenceAction::Press('d'),
                CastSequenceAction::AltUp,
            ]
        );
    }

    #[test]
    fn manual_wait_cast_behavior_produces_no_auto_cast_sequence() {
        assert!(
            cast_sequence_for_behavior('d', &InvokerProfileStepCastBehavior::ManualWaitCooldown)
                .is_empty()
        );
    }

    #[test]
    fn build_profile_execution_plan_carries_cast_behavior_for_qe_burst() {
        let event = invoker_qe_fixture();
        let settings = Settings::default();
        let state = InvokerObservedState::from_event(&event);
        let profile = find_profile(&settings.heroes.invoker, "qe-burst").unwrap();

        let plan = build_profile_execution_plan(profile, &state, &settings.heroes.invoker)
            .expect("QE Burst plan should build");

        let first_spell = plan
            .iter()
            .find_map(|action| match action {
                PlannedInvokerAction::Spell { cast_behavior, .. } => Some(cast_behavior.clone()),
                PlannedInvokerAction::Item { .. } => None,
            })
            .expect("QE Burst should contain a spell step");

        assert_eq!(first_spell, InvokerProfileStepCastBehavior::ManualWaitCooldown);
    }
```

- [ ] **Step 2: Run the targeted runtime tests to confirm they fail**

Run:

```powershell
cargo test cast_sequence_for_alt_double_tap_holds_alt_for_both_presses --lib
```

Expected: FAIL because `cast_sequence_for_behavior`, `CastSequenceAction`, and `cast_behavior` on planned spell actions do not exist yet.

- [ ] **Step 3: Add cast-behavior planning and execution**

In `src/actions/heroes/invoker.rs`, import the new enum and add pure cast-sequence helpers:

```rust
use crate::config::settings::{
    InvokerProfile,
    InvokerProfileMode,
    InvokerProfileStepCastBehavior,
    InvokerProfileStepCompletionMode,
    InvokerProfileStepKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum CastSequenceAction {
    Press(char),
    AltDown,
    AltUp,
    SleepMs(u64),
}

fn cast_sequence_for_behavior(
    cast_key: char,
    cast_behavior: &InvokerProfileStepCastBehavior,
) -> Vec<CastSequenceAction> {
    match cast_behavior {
        InvokerProfileStepCastBehavior::Normal => vec![CastSequenceAction::Press(cast_key)],
        InvokerProfileStepCastBehavior::ManualWaitCooldown => Vec::new(),
        InvokerProfileStepCastBehavior::AltCast => vec![
            CastSequenceAction::AltDown,
            CastSequenceAction::Press(cast_key),
            CastSequenceAction::AltUp,
        ],
        InvokerProfileStepCastBehavior::DoubleTap => vec![
            CastSequenceAction::Press(cast_key),
            CastSequenceAction::SleepMs(50),
            CastSequenceAction::Press(cast_key),
        ],
        InvokerProfileStepCastBehavior::AltDoubleTap => vec![
            CastSequenceAction::AltDown,
            CastSequenceAction::Press(cast_key),
            CastSequenceAction::SleepMs(50),
            CastSequenceAction::Press(cast_key),
            CastSequenceAction::AltUp,
        ],
    }
}

fn execute_cast_sequence(sequence: &[CastSequenceAction]) {
    for action in sequence {
        match action {
            CastSequenceAction::Press(key) => crate::input::simulation::press_key(*key),
            CastSequenceAction::AltDown => crate::input::simulation::alt_down(),
            CastSequenceAction::AltUp => crate::input::simulation::alt_up(),
            CastSequenceAction::SleepMs(delay_ms) => {
                thread::sleep(Duration::from_millis(*delay_ms));
            }
        }
    }
}
```

Carry `cast_behavior` through `PreparedSpellStep` and `PlannedInvokerAction::Spell`:

```rust
struct PreparedSpellStep {
    target: String,
    prepare_keys: Vec<char>,
    prepared_slots_after_prepare: Option<[Option<String>; 2]>,
    cast_key: char,
    delay_after_ms: u64,
    completion_mode: InvokerProfileStepCompletionMode,
    completion_timeout_ms: u64,
    cast_behavior: InvokerProfileStepCastBehavior,
}

enum PlannedInvokerAction {
    Spell {
        target: String,
        prepare_keys: Vec<char>,
        prepared_slots_after_prepare: Option<[Option<String>; 2]>,
        cast_key: char,
        delay_after_ms: u64,
        completion_mode: InvokerProfileStepCompletionMode,
        completion_timeout_ms: u64,
        cast_behavior: InvokerProfileStepCastBehavior,
        should_cast: bool,
    },
    // existing Item variant unchanged
}
```

Apply the behavior in runtime:

```rust
let effective_completion_mode = if cast_behavior
    == InvokerProfileStepCastBehavior::ManualWaitCooldown
{
    InvokerProfileStepCompletionMode::WaitForCooldown
} else {
    completion_mode.clone()
};

let cast_sequence = if should_cast {
    cast_sequence_for_behavior(cast_key, &cast_behavior)
} else {
    Vec::new()
};

if should_cast && !cast_sequence.is_empty() {
    info!("🔮 Casting {} with {:?}", target, cast_behavior);
    execute_cast_sequence(&cast_sequence);
} else if should_cast {
    info!("🔮 Prepared {} for manual cast", target);
} else {
    info!("🔮 Prepared {} without casting", target);
}

if effective_completion_mode == InvokerProfileStepCompletionMode::WaitForCooldown {
    match wait_for_spell_cooldown_start(&target, completion_timeout_ms, 25) {
        // keep the existing Started / TimedOut / HeroUnavailable / SpellNotObserved handling
    }
}
```

- [ ] **Step 4: Run the targeted runtime tests**

Run:

```powershell
cargo test manual_wait_cast_behavior_produces_no_auto_cast_sequence --lib
```

Expected: PASS.

- [ ] **Step 5: Commit the runtime cast-behavior slice**

Run:

```powershell
git add src/actions/heroes/invoker.rs
git commit -m "feat: add invoker cast behavior execution"
```

---

### Task 3: Make the Invoker combo-cycle hotkey configurable in the backend

**Files:**
- Modify: `src/input/keyboard.rs`
- Test: `src/input/keyboard.rs`

- [ ] **Step 1: Write the failing keyboard tests**

Add these tests to `src/input/keyboard.rs` near the current Invoker cycle tests:

```rust
    #[test]
    fn keyboard_snapshot_uses_configured_invoker_cycle_hotkey() {
        let mut settings = Settings::default();
        settings.heroes.invoker.cycle_combo_profiles_hotkey = "F10".to_string();

        let state = AppState {
            selected_hero: Some(HeroType::Invoker),
            gsi_enabled: true,
            standalone_enabled: true,
            last_event: None,
            last_gsi_activity_at: None,
            metrics: QueueMetrics::default(),
            trigger_key: Arc::new(Mutex::new("Home".to_string())),
            sf_enabled: Arc::new(Mutex::new(false)),
            od_enabled: Arc::new(Mutex::new(false)),
            update_state: Arc::new(Mutex::new(UpdateCheckState::Idle)),
            invoker_active_combo_profile_id: None,
            rune_alerts: None,
            minimap_capture: None,
        };

        let snapshot = KeyboardSnapshot::from_runtime(&settings, &state);
        assert_eq!(snapshot.invoker_cycle_hotkey, Some(Key::F10));
    }

    #[test]
    fn plan_global_hotkey_event_uses_configured_invoker_cycle_hotkey() {
        let mut snapshot = KeyboardSnapshot::default();
        snapshot.selected_hero = Some(HeroType::Invoker);
        snapshot.invoker_cycle_hotkey = Some(Key::F10);
        snapshot.invoker_profiles = vec![InvokerHotkeyProfileSnapshot {
            id: "qw-pickoff".to_string(),
            hotkey: Some(Key::Home),
            mode: InvokerProfileMode::Combo,
            enabled: true,
        }];

        assert_eq!(
            plan_global_hotkey_event(Key::F10, &snapshot),
            Some(HotkeyEvent::InvokerCycleComboProfile)
        );
    }
```

- [ ] **Step 2: Run the keyboard tests to confirm they fail**

Run:

```powershell
cargo test keyboard_snapshot_uses_configured_invoker_cycle_hotkey --lib
```

Expected: FAIL because `KeyboardSnapshot::from_runtime(...)` still hardcodes `Delete`.

- [ ] **Step 3: Replace the hardcoded cycle key with the config field**

Update `KeyboardSnapshot::from_runtime(...)` in `src/input/keyboard.rs`:

```rust
invoker_cycle_hotkey: parse_key_string(&settings.heroes.invoker.cycle_combo_profiles_hotkey),
```

Keep `KeyboardSnapshot::default()` with `invoker_cycle_hotkey: None`; that default is only for tests and empty snapshots.

Do **not** change the event shape or `main.rs` handling in this task. The only behavioral change here is where the key value comes from.

- [ ] **Step 4: Run the targeted keyboard tests**

Run:

```powershell
cargo test plan_global_hotkey_event_uses_configured_invoker_cycle_hotkey --lib
```

Expected: PASS.

- [ ] **Step 5: Commit the configurable-cycle backend slice**

Run:

```powershell
git add src/input/keyboard.rs
git commit -m "feat: make invoker cycle hotkey configurable"
```

---

### Task 4: Expose cast behavior and cycle-key controls in the React Invoker UI

**Files:**
- Modify: `src-ui/src/components/heroes/configs/InvokerConfig.tsx`
- Modify: `src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx`
- Modify: `src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx`
- Test: `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx`

- [ ] **Step 1: Write the failing UI tests**

Add these tests to `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx`:

```tsx
  it("renders and persists the cycle active combo hotkey", () => {
    render(<InvokerConfig />);

    expect(screen.getByDisplayValue("Delete")).toBeInTheDocument();

    fireEvent.change(screen.getByDisplayValue("Delete"), {
      target: { value: "F10" },
    });

    expect(
      useConfigStore.getState().config.heroes.invoker.cycle_combo_profiles_hotkey,
    ).toBe("F10");
  });

  it("persists cast behavior edits into the config store", () => {
    render(<InvokerConfig />);

    fireEvent.click(screen.getByText(/PageDown/).closest("button")!);
    fireEvent.change(screen.getAllByDisplayValue("Manual Wait Cooldown")[0], {
      target: { value: "alt_double_tap" },
    });

    const qeProfile = useConfigStore
      .getState()
      .config.heroes.invoker.profiles.find((profile) => profile.id === "qe-burst");

    expect(qeProfile?.steps[0].cast_behavior).toBe("alt_double_tap");
  });

  it("shows the new preset library entries", () => {
    render(<InvokerConfig />);

    expect(screen.getByText("Refresher Sequence")).toBeInTheDocument();
    expect(screen.getByText("Lane Pressure")).toBeInTheDocument();
  });
```

- [ ] **Step 2: Run the UI tests to confirm they fail**

Run:

```powershell
npm --prefix src-ui test -- InvokerConfig.test.tsx
```

Expected: FAIL because the cycle hotkey field, cast-behavior dropdown, and new preset labels do not exist yet.

- [ ] **Step 3: Add the UI controls and labels**

In `src-ui/src/components/heroes/configs/InvokerConfig.tsx`, add the cycle key to the core keys card:

```tsx
<KeyInput
  label="Cycle Active Combo"
  value={config.cycle_combo_profiles_hotkey}
  onChange={(cycle_combo_profiles_hotkey) =>
    set({ cycle_combo_profiles_hotkey })
  }
/>
```

In `src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx`, add cast-behavior options and bind them to spell steps:

```tsx
import type {
  InvokerProfile,
  InvokerProfileStep,
  InvokerProfileStepCastBehavior,
  InvokerProfileStepCompletionMode,
  InvokerProfileStepKind,
} from "../../../../types/config";

const CAST_BEHAVIOR_OPTIONS = [
  { value: "normal", label: "Normal" },
  { value: "manual_wait_cooldown", label: "Manual Wait Cooldown" },
  { value: "alt_cast", label: "Alt Cast" },
  { value: "double_tap", label: "Double Tap" },
  { value: "alt_double_tap", label: "Alt Double Tap" },
];

<Dropdown
  label="Cast Behavior"
  value={step.cast_behavior}
  options={CAST_BEHAVIOR_OPTIONS}
  onChange={(cast_behavior) =>
    setStep(index, {
      ...step,
      cast_behavior: cast_behavior as InvokerProfileStepCastBehavior,
      completion_mode:
        cast_behavior === "manual_wait_cooldown"
          ? "wait_for_cooldown"
          : step.completion_mode,
    })
  }
/>
```

Also update `createInvokerStep(...)` so new spell and item steps default to:

```ts
cast_behavior: "normal",
```

In `src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx`, replace the current `[manual]` label shortcut with a suffix helper:

```tsx
function stepBehaviorSuffix(step: InvokerProfile["steps"][number]) {
  switch (step.cast_behavior) {
    case "manual_wait_cooldown":
      return " [manual]";
    case "alt_cast":
      return " [Alt]";
    case "double_tap":
      return " [x2]";
    case "alt_double_tap":
      return " [Alt x2]";
    default:
      return "";
  }
}
```

Use that helper in both:

- `InvokerProfileList.tsx` step summaries
- `InvokerProfileEditor.tsx` execution preview

- [ ] **Step 4: Run the UI tests again**

Run:

```powershell
npm --prefix src-ui test -- InvokerConfig.test.tsx
```

Expected: PASS.

- [ ] **Step 5: Commit the Invoker UI slice**

Run:

```powershell
git add src-ui/src/components/heroes/configs/InvokerConfig.tsx src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx src-ui/src/components/heroes/configs/InvokerConfig.test.tsx
git commit -m "feat(ui): expose invoker combo behavior controls"
```

---

### Task 5: Document the new Invoker combo behavior and verify the repo

**Files:**
- Modify: `docs/heroes/invoker.md`
- Modify: `docs/reference/configuration.md`

- [ ] **Step 1: Update the hero-facing Invoker doc**

Add an `Invoker combo system` section to `docs/heroes/invoker.md` that explicitly covers:

```md
- `cast_behavior` values:
  - `normal`
  - `manual_wait_cooldown`
  - `alt_cast`
  - `double_tap`
  - `alt_double_tap`
- `manual_wait_cooldown` prepares the spell and waits for the player's real cast
- optional combo items are skipped when not found
- the cycle hotkey is now configurable through `heroes.invoker.cycle_combo_profiles_hotkey`
- the lane-pressure preset is summon-only and does not add follow-up spells or Forge Spirit control
```

- [ ] **Step 2: Update the configuration reference**

Add these new fields to `docs/reference/configuration.md` under the Invoker section:

```md
- `heroes.invoker.cycle_combo_profiles_hotkey` — runtime key used to rotate the active combo through enabled combo profiles
- `heroes.invoker.profiles[].steps[].cast_behavior` — spell activation behavior (`normal`, `manual_wait_cooldown`, `alt_cast`, `double_tap`, `alt_double_tap`)
```

Also add one sentence clarifying that:

```md
Configured item steps continue automatically when the item is missing; they do not abort the rest of the profile.
```

- [ ] **Step 3: Run focused and full verification**

Run:

```powershell
cargo test invoker_defaults_seed_combo_system_profiles --lib
cargo test cast_sequence_for_alt_double_tap_holds_alt_for_both_presses --lib
cargo test keyboard_snapshot_uses_configured_invoker_cycle_hotkey --lib
npm --prefix src-ui test -- InvokerConfig.test.tsx
cargo test
npm --prefix src-ui test
cargo build --release
```

Expected:

- targeted Rust/UI tests PASS
- full `cargo test` PASS
- full `npm --prefix src-ui test` PASS
- `cargo build --release` succeeds

- [ ] **Step 4: Commit the docs + verification slice**

Run:

```powershell
git add docs/heroes/invoker.md docs/reference/configuration.md
git commit -m "docs: explain invoker combo system expansion"
```

- [ ] **Step 5: Clean handoff check**

Before marking the work complete, run:

```powershell
git --no-pager status --short
```

Expected: only the intended task files are modified or the worktree is clean after the final commit.

---

## Self-Review Checklist

- Spec coverage:
  - preset expansion -> Task 1 + Task 4
  - true manual Sun Strike continuation -> Task 2
  - Cataclysm `Alt + D/F` repeat casting -> Task 2
  - Refresher/Eul's support -> Task 1
  - configurable cycle key -> Task 1 + Task 3 + Task 4
  - optional item skip documentation -> Task 5
  - lane-pressure micro explicitly scoped out -> Task 5 docs note
- Placeholder scan:
  - no `TODO` / `TBD`
  - every task has concrete files, code snippets, commands, and commit messages
- Naming consistency:
  - `InvokerProfileStepCastBehavior`
  - `cycle_combo_profiles_hotkey`
  - `manual_wait_cooldown`
  - `alt_double_tap`

