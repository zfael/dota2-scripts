# Invoker Semi-Auto Combo Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a combo-only `semi_auto` execution style for Invoker profiles so combo items stay automatic while spell steps are prepared one-by-one onto the configured secondary invoked-spell slot and advance when that spell enters cooldown.

**Architecture:** Keep `mode = combo | prep` unchanged and add a new `execution_style` field to `InvokerProfile`. Preserve the current automatic combo runner, then add a separate semi-auto state machine inside `src/actions/heroes/invoker.rs` that snapshots the triggered profile, prepares one spell at a time, and advances from fresh GSI updates. Mirror the new field through the checked-in TOML template, React types/mock data, the Invoker profile editor/list UI, and the Invoker/config reference docs.

**Tech Stack:** Rust 2021, serde + TOML config, tracing, axum/GSI event flow, React 19, TypeScript, Zustand, Vitest

---

## Execution prerequisite

Implementation should happen in a fresh git worktree, not in the current repository root. The current root already has unrelated local changes, so the executor should isolate this feature branch before touching code.

## File Structure

- `src/config/settings.rs` - add the Rust enum/default for Invoker profile execution style, wire it into `InvokerProfile`, seed defaults, and extend existing Invoker config unit tests.
- `config/config.toml` - add `execution_style = "automatic"` to each checked-in Invoker combo/prep profile so the shipped template matches the Rust schema.
- `src/actions/heroes/invoker.rs` - keep the automatic runner unchanged, add pure semi-auto planning helpers, add the in-flight semi-auto session state machine, and add focused unit tests for planning/session advancement.
- `src-ui/src/types/config.ts` - add the TypeScript execution-style union and field on `InvokerProfile`.
- `src-ui/src/stores/mockData.ts` - seed `execution_style: "automatic"` for every mock Invoker profile.
- `src-ui/src/components/heroes/configs/InvokerConfig.tsx` - default newly created combo profiles to `execution_style: "automatic"`.
- `src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx` - add the combo-only execution-style control.
- `src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx` - show a compact semi-auto indicator in the profile list summary.
- `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx` - add UI regressions for the new field, combo-only editor behavior, and list badge.
- `docs/heroes/invoker.md` - document the new profile field plus the automatic vs semi-auto runtime split.
- `docs/reference/configuration.md` - add the new config row for `profiles[].execution_style` and update the Invoker notes to mention semi-auto behavior.

### Task 1: Add Invoker execution-style schema and template defaults

**Files:**
- Modify: `src/config/settings.rs:389-489`
- Modify: `src/config/settings.rs:2329-2387`
- Modify: `config/config.toml:237-500`
- Test: `src/config/settings.rs`

- [ ] **Step 1: Write the failing backend tests**

Add these tests near the existing Invoker defaults block in `src/config/settings.rs`:

```rust
#[test]
fn invoker_profiles_default_to_automatic_execution_style() {
    let settings = Settings::default();

    let qw = settings
        .heroes
        .invoker
        .profiles
        .iter()
        .find(|profile| profile.id == "qw-pickoff")
        .expect("QW Pickoff should exist");
    let prep = settings
        .heroes
        .invoker
        .profiles
        .iter()
        .find(|profile| profile.id == "meteor-blast-prep")
        .expect("Meteor + Blast Prep should exist");

    assert_eq!(qw.execution_style, InvokerProfileExecutionStyle::Automatic);
    assert_eq!(prep.execution_style, InvokerProfileExecutionStyle::Automatic);
}

#[test]
fn invoker_profile_execution_style_defaults_when_field_is_missing() {
    let profile: InvokerProfile = toml::from_str(
        r#"
id = "semi-auto-check"
name = "Semi Auto Check"
enabled = true
hotkey = "F10"
mode = "combo"
build_tag = "qw"
"#,
    )
    .expect("profile should deserialize");

    assert_eq!(profile.execution_style, InvokerProfileExecutionStyle::Automatic);
}
```

- [ ] **Step 2: Run the targeted test command and verify it fails**

Run:

```powershell
cargo test invoker_profile_execution_style --lib
```

Expected: FAIL with compile errors about `InvokerProfileExecutionStyle` / `execution_style` not existing yet.

- [ ] **Step 3: Implement the Rust enum, profile field, and TOML defaults**

Add the new enum and default function near the existing Invoker enums:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvokerProfileExecutionStyle {
    Automatic,
    SemiAuto,
}

fn default_invoker_profile_execution_style() -> InvokerProfileExecutionStyle {
    InvokerProfileExecutionStyle::Automatic
}
```

Wire it into `InvokerProfile`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvokerProfile {
    pub id: String,
    pub name: String,
    #[serde(default = "default_invoker_profile_enabled")]
    pub enabled: bool,
    pub hotkey: String,
    pub mode: InvokerProfileMode,
    #[serde(default = "default_invoker_profile_execution_style")]
    pub execution_style: InvokerProfileExecutionStyle,
    #[serde(default)]
    pub build_tag: String,
    #[serde(default)]
    pub steps: Vec<InvokerProfileStep>,
}
```

Update every seeded Invoker profile in `Settings::default()` and every profile entry in `config/config.toml` to include:

```toml
execution_style = "automatic"
```

- [ ] **Step 4: Re-run the targeted backend checks**

Run:

```powershell
cargo test invoker_profile_execution_style --lib
cargo test invoker_defaults_seed_expected_profiles --lib
```

Expected: PASS for both tests.

- [ ] **Step 5: Commit the schema/default change**

```powershell
git add src/config/settings.rs config/config.toml
git commit -m "feat: add invoker profile execution style"
```

### Task 2: Add pure semi-auto planning helpers in the Invoker runner

**Files:**
- Modify: `src/actions/heroes/invoker.rs:338-648`
- Modify: `src/actions/heroes/invoker.rs:1081-1465`
- Test: `src/actions/heroes/invoker.rs`

- [ ] **Step 1: Write failing semi-auto planning tests**

Add these tests near the current `build_profile_execution_plan` coverage:

```rust
#[test]
fn build_semi_auto_execution_plan_for_qw_pickoff_keeps_items_then_secondary_slot_spells() {
    let event = invoker_qw_fixture();
    let settings = Settings::default();
    let config = &settings.heroes.invoker;
    let state = InvokerObservedState::from_event(&event);
    let profile = find_profile(config, "qw-pickoff").expect("QW profile should exist");

    let plan = build_semi_auto_execution_plan(profile, &state, config)
        .expect("semi-auto plan should build");

    assert_eq!(plan.monitored_slot_key, config.spell_slot_secondary_key);
    assert_eq!(
        plan.steps
            .iter()
            .map(|step| match step {
                SemiAutoPlanStep::Item { target, .. } => format!("item:{target}"),
                SemiAutoPlanStep::Spell { target, .. } => format!("spell:{target}"),
            })
            .collect::<Vec<_>>(),
        vec![
            "item:item_spirit_vessel",
            "item:item_rod_of_atos",
            "spell:invoker_tornado",
            "spell:invoker_emp",
        ]
    );
}

#[test]
fn build_semi_auto_execution_plan_reuses_loaded_secondary_spell_without_extra_invoke() {
    let event = invoker_qw_fixture();
    let settings = Settings::default();
    let config = &settings.heroes.invoker;
    let mut state = InvokerObservedState::from_event(&event);
    state.active_spells = [None, Some("invoker_tornado".to_string())];
    let profile = find_profile(config, "qw-pickoff").expect("QW profile should exist");

    let plan = build_semi_auto_execution_plan(profile, &state, config)
        .expect("semi-auto plan should build");

    let first_spell = plan
        .steps
        .iter()
        .find_map(|step| match step {
            SemiAutoPlanStep::Spell { prepare_keys, .. } => Some(prepare_keys.clone()),
            SemiAutoPlanStep::Item { .. } => None,
        })
        .expect("plan should contain a spell");

    assert!(first_spell.is_empty(), "already-loaded spell should not invoke again");
}
```

- [ ] **Step 2: Run the targeted Invoker planner tests and verify they fail**

Run:

```powershell
cargo test build_semi_auto_execution_plan --lib
```

Expected: FAIL because the semi-auto plan types and helper do not exist yet.

- [ ] **Step 3: Implement the pure semi-auto plan types and helper**

Add focused private types and a builder alongside the existing automatic planner:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct SemiAutoExecutionPlan {
    monitored_slot_key: char,
    steps: Vec<SemiAutoPlanStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SemiAutoPlanStep {
    Item { target: String, delay_after_ms: u64 },
    Spell { target: String, prepare_keys: Vec<char> },
}

fn build_semi_auto_execution_plan(
    profile: &InvokerProfile,
    state: &InvokerObservedState,
    config: &crate::config::settings::InvokerConfig,
) -> Option<SemiAutoExecutionPlan> {
    let mut current_slots = state.active_spells.clone();
    let mut steps = Vec::new();

    for step in &profile.steps {
        match step.kind {
            InvokerProfileStepKind::Item => steps.push(SemiAutoPlanStep::Item {
                target: step.target.clone(),
                delay_after_ms: step.delay_after_ms,
            }),
            InvokerProfileStepKind::Spell => {
                let already_on_secondary =
                    current_slots[1].as_deref() == Some(step.target.as_str());
                let prepare_keys = if already_on_secondary {
                    Vec::new()
                } else {
                    let keys = orb_recipe(&step.target, config)?.to_vec();
                    current_slots = apply_invoke_to_slot_state(&current_slots, &step.target);
                    keys
                };

                steps.push(SemiAutoPlanStep::Spell {
                    target: step.target.clone(),
                    prepare_keys,
                });
            }
        }
    }

    Some(SemiAutoExecutionPlan {
        monitored_slot_key: config.spell_slot_secondary_key,
        steps,
    })
}
```

- [ ] **Step 4: Re-run the semi-auto planner tests**

Run:

```powershell
cargo test build_semi_auto_execution_plan --lib
cargo test build_profile_execution_plan_for_qw_pickoff_keeps_tornado_then_emp_order --lib
```

Expected: PASS for the new semi-auto tests and PASS for the existing automatic regression.

- [ ] **Step 5: Commit the pure semi-auto planner**

```powershell
git add src/actions/heroes/invoker.rs
git commit -m "feat: add invoker semi-auto planner"
```

### Task 3: Add the in-flight semi-auto session and GSI-driven advancement

**Files:**
- Modify: `src/actions/heroes/invoker.rs:421-760`
- Modify: `src/actions/heroes/invoker.rs:1332-1465`
- Test: `src/actions/heroes/invoker.rs`

- [ ] **Step 1: Write failing session-advancement tests**

Add these tests after the existing cooldown-wait coverage:

```rust
#[test]
fn semi_auto_session_advances_after_watched_spell_enters_cooldown() {
    let settings = Settings::default();
    let config = &settings.heroes.invoker;
    let state = InvokerObservedState::from_event(&invoker_qw_fixture());
    let profile = find_profile(config, "qw-pickoff").expect("QW profile should exist");
    let plan = build_semi_auto_execution_plan(profile, &state, config)
        .expect("semi-auto plan should build");
    let mut session = InvokerSemiAutoSession::from_plan("qw-pickoff", &settings, plan);

    let first = advance_semi_auto_session(&mut session, &invoker_qw_fixture());
    assert_eq!(first.prepared_spell.as_deref(), Some("invoker_tornado"));

    let mut cooling = invoker_qw_fixture();
    cooling.abilities.ability5.name = "invoker_tornado".to_string();
    cooling.abilities.ability5.cooldown = 12;
    cooling.abilities.ability5.can_cast = false;

    let second = advance_semi_auto_session(&mut session, &cooling);
    assert_eq!(second.prepared_spell.as_deref(), Some("invoker_emp"));
}

#[test]
fn replace_semi_auto_session_swaps_in_the_latest_profile() {
    let settings = Settings::default();
    let config = &settings.heroes.invoker;
    let state = InvokerObservedState::from_event(&invoker_qw_fixture());

    let old_plan = build_semi_auto_execution_plan(
        find_profile(config, "qw-pickoff").unwrap(),
        &state,
        config,
    )
    .unwrap();
    let new_plan = build_semi_auto_execution_plan(
        find_profile(config, "ghost-walk-panic").unwrap(),
        &state,
        config,
    )
    .unwrap();

    let old_session = InvokerSemiAutoSession::from_plan("qw-pickoff", &settings, old_plan);
    let new_session = InvokerSemiAutoSession::from_plan("ghost-walk-panic", &settings, new_plan);

    let replaced = replace_semi_auto_session(Some(old_session), new_session);

    assert_eq!(replaced.profile_id, "ghost-walk-panic");
}
```

- [ ] **Step 2: Run the targeted session tests and verify they fail**

Run:

```powershell
cargo test semi_auto_session --lib
```

Expected: FAIL because the session state machine and helper functions do not exist yet.

- [ ] **Step 3: Implement the semi-auto session state machine and hook it into trigger/GSI flow**

Add a compact in-flight session model and advancement helpers:

```rust
use std::collections::VecDeque;

#[derive(Debug, Clone)]
struct InvokerSemiAutoSession {
    profile_id: String,
    settings: Settings,
    monitored_slot_key: char,
    pending_steps: VecDeque<SemiAutoPlanStep>,
    watched_spell: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemiAutoAdvanceResult {
    prepared_spell: Option<String>,
    completed: bool,
}

static INVOKER_ACTIVE_SEMI_AUTO_SESSION: LazyLock<Mutex<Option<InvokerSemiAutoSession>>> =
    LazyLock::new(|| Mutex::new(None));
```

Drive it from the existing flow:

```rust
fn run_invoker_request(request: InvokerRequest) {
    // existing setup ...
    if profile.mode == InvokerProfileMode::Combo
        && profile.execution_style == InvokerProfileExecutionStyle::SemiAuto
    {
        let plan = build_semi_auto_execution_plan(profile, &state, config)
            .expect("semi-auto profile should build");
        let session = InvokerSemiAutoSession::from_plan(&profile.id, &settings, plan);
        *INVOKER_ACTIVE_SEMI_AUTO_SESSION.lock().unwrap() = Some(session);
        advance_active_semi_auto_session(&event);
        return;
    }

    run_profile(&event, &settings, &state, config, profile);
}

fn advance_active_semi_auto_session(event: &GsiWebhookEvent) {
    let mut guard = INVOKER_ACTIVE_SEMI_AUTO_SESSION.lock().unwrap();
    let Some(session) = guard.as_mut() else {
        return;
    };

    if !event.hero.alive || event.hero.stunned || event.hero.hexed || event.hero.silenced {
        *guard = None;
        return;
    }

    if let Some(watched_spell) = session.watched_spell.as_deref() {
        if !spell_is_on_cooldown(event, watched_spell) {
            return;
        }
    }

    // consume item steps, prepare the next spell onto session.monitored_slot_key,
    // and clear the session when the queue is exhausted
}
```

Also call `advance_active_semi_auto_session(event);` from `handle_gsi_event` after `INVOKER_LAST_EVENT` is updated.

- [ ] **Step 4: Re-run the session tests plus one existing manual-wait regression**

Run:

```powershell
cargo test semi_auto_session --lib
cargo test manual_wait_completes_after_gsi_update --lib
```

Expected: PASS for the new semi-auto tests and PASS for the existing manual cooldown regression.

- [ ] **Step 5: Commit the session runner integration**

```powershell
git add src/actions/heroes/invoker.rs
git commit -m "feat: wire invoker semi-auto session runner"
```

### Task 4: Mirror execution style through the React data model and defaults

**Files:**
- Modify: `src-ui/src/types/config.ts:188-229`
- Modify: `src-ui/src/stores/mockData.ts:38-164`
- Modify: `src-ui/src/components/heroes/configs/InvokerConfig.tsx:45-58`
- Test: `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx`

- [ ] **Step 1: Write the failing UI data/default test**

Add this test to `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx`:

```tsx
it("creates new combo profiles with automatic execution style", () => {
  render(<InvokerConfig />);

  fireEvent.click(screen.getByRole("button", { name: /new combo profile/i }));

  const created = useConfigStore
    .getState()
    .config.heroes.invoker.profiles.find((profile) => profile.name === "Custom Combo");

  expect(created?.execution_style).toBe("automatic");
});
```

- [ ] **Step 2: Run the focused UI test and verify it fails**

Run:

```powershell
npm --prefix src-ui test -- src/components/heroes/configs/InvokerConfig.test.tsx
```

Expected: FAIL with a TypeScript or assertion error because `execution_style` is missing from the TS shape and new profile factory.

- [ ] **Step 3: Add the TypeScript field, mock defaults, and combo-profile factory default**

Update the config type:

```ts
export type InvokerProfileExecutionStyle = "automatic" | "semi_auto";

export interface InvokerProfile {
  id: string;
  name: string;
  enabled: boolean;
  hotkey: string;
  mode: InvokerProfileMode;
  execution_style: InvokerProfileExecutionStyle;
  build_tag: string;
  steps: InvokerProfileStep[];
}
```

Add `execution_style: "automatic"` to every Invoker profile in `mockData.ts`, and set the new combo/prep profile factory default in `InvokerConfig.tsx`:

```tsx
return {
  id: nextProfileId(name, profiles),
  name,
  enabled: true,
  hotkey: "",
  mode,
  execution_style: "automatic",
  build_tag: "general",
  steps: [createInvokerStep("spell")],
};
```

- [ ] **Step 4: Re-run the focused UI test**

Run:

```powershell
npm --prefix src-ui test -- src/components/heroes/configs/InvokerConfig.test.tsx
```

Expected: PASS, with the new combo profile defaulting to `automatic`.

- [ ] **Step 5: Commit the UI data/default change**

```powershell
git add src-ui/src/types/config.ts src-ui/src/stores/mockData.ts src-ui/src/components/heroes/configs/InvokerConfig.tsx src-ui/src/components/heroes/configs/InvokerConfig.test.tsx
git commit -m "feat(ui): add invoker execution style defaults"
```

### Task 5: Expose semi-auto controls and badges in the Invoker UI

**Files:**
- Modify: `src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx:26-53`
- Modify: `src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx:115-347`
- Modify: `src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx:18-39`
- Modify: `src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx:78-119`
- Modify: `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx:12-222`
- Test: `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx`

- [ ] **Step 1: Write the failing editor/list UI tests**

Add these tests to `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx`:

```tsx
it("persists combo execution style edits into the config store", () => {
  render(<InvokerConfig />);

  fireEvent.change(screen.getByDisplayValue("Automatic"), {
    target: { value: "semi_auto" },
  });

  const qw = useConfigStore
    .getState()
    .config.heroes.invoker.profiles.find((profile) => profile.id === "qw-pickoff");

  expect(qw?.execution_style).toBe("semi_auto");
});

it("does not show the execution-style control for prep profiles", () => {
  render(<InvokerConfig />);

  fireEvent.click(screen.getByText(/PageUp/).closest("button")!);

  expect(screen.queryByLabelText("Execution Style")).not.toBeInTheDocument();
});

it("shows a semi-auto indicator on combo profile cards", () => {
  useConfigStore.setState((state) => ({
    config: {
      ...state.config,
      heroes: {
        ...state.config.heroes,
        invoker: {
          ...state.config.heroes.invoker,
          profiles: state.config.heroes.invoker.profiles.map((profile) =>
            profile.id === "qw-pickoff"
              ? { ...profile, execution_style: "semi_auto" }
              : profile,
          ),
        },
      },
    },
  }));

  render(<InvokerConfig />);

  expect(screen.getAllByText(/semi-auto/i).length).toBeGreaterThan(0);
});
```

- [ ] **Step 2: Run the focused UI suite and verify it fails**

Run:

```powershell
npm --prefix src-ui test -- src/components/heroes/configs/InvokerConfig.test.tsx
```

Expected: FAIL because the editor does not render an execution-style control or list badge yet.

- [ ] **Step 3: Implement the combo-only editor control and profile-list badge**

Add execution-style options to the editor:

```tsx
const EXECUTION_STYLE_OPTIONS = [
  { value: "automatic", label: "Automatic" },
  { value: "semi_auto", label: "Semi-auto" },
];
```

Render the combo-only control in `InvokerProfileEditor.tsx`:

```tsx
{profile.mode === "combo" ? (
  <Dropdown
    label="Execution Style"
    value={profile.execution_style}
    options={EXECUTION_STYLE_OPTIONS}
    onChange={(execution_style) =>
      onChange({
        ...profile,
        execution_style:
          execution_style as InvokerProfile["execution_style"],
      })
    }
  />
) : (
  <div className="rounded-md border border-border bg-input px-3 py-2 text-xs text-subtle">
    Prep profiles always invoke without auto-casting, so execution style is fixed.
  </div>
)}
```

Add a compact badge in `InvokerProfileList.tsx`:

```tsx
{profile.mode === "combo" && profile.execution_style === "semi_auto" && (
  <span className="rounded-full bg-brand/15 px-2 py-0.5 text-[10px] font-medium text-brand">
    Semi-auto
  </span>
)}
```

- [ ] **Step 4: Re-run the focused UI suite**

Run:

```powershell
npm --prefix src-ui test -- src/components/heroes/configs/InvokerConfig.test.tsx
```

Expected: PASS, including the new persistence and badge assertions.

- [ ] **Step 5: Commit the Invoker semi-auto UI**

```powershell
git add src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx src-ui/src/components/heroes/configs/InvokerConfig.test.tsx
git commit -m "feat(ui): expose invoker semi-auto mode"
```

### Task 6: Update the Invoker docs and configuration reference

**Files:**
- Modify: `docs/heroes/invoker.md:27-220`
- Modify: `docs/reference/configuration.md:396-434`
- Test: `docs/heroes/invoker.md`

- [ ] **Step 1: Update the hero doc and config reference with the new field**

Add the new table row and runtime notes to `docs/heroes/invoker.md`:

```md
| `profiles[].execution_style` | string | `automatic` | Combo-only spell execution style: `automatic` preserves the current auto-cast planner, while `semi_auto` auto-runs item steps and prepares one spell at a time onto `spell_slot_secondary_key`. |
```

Add a dedicated runtime note under combo behavior:

```md
### Semi-auto combo profiles

Semi-auto combo profiles still execute item steps automatically, but spell steps are no longer auto-cast. The runtime prepares one authored spell at a time onto the configured secondary invoked slot, waits for that spell to enter cooldown from the player's real cast, then immediately prepares the next spell onto that same slot.
```

Update the Invoker section in `docs/reference/configuration.md`:

```md
| `profiles[].execution_style` | `automatic` | `automatic` | Combo spell execution style. `automatic` keeps the current pair-aware auto-cast runner. `semi_auto` keeps item steps automatic and prepares one spell at a time onto `spell_slot_secondary_key`, advancing when that spell enters cooldown. |
```

- [ ] **Step 2: Proofread the docs diff**

Run:

```powershell
git --no-pager diff -- docs/heroes/invoker.md docs/reference/configuration.md
```

Expected: the diff mentions `execution_style`, explains automatic vs semi-auto, and does not describe prep as changing behavior.

- [ ] **Step 3: Commit the documentation update**

```powershell
git add docs/heroes/invoker.md docs/reference/configuration.md
git commit -m "docs: document invoker semi-auto combo mode"
```

### Task 7: Run full verification before handoff

**Files:**
- Test: `src/actions/heroes/invoker.rs`
- Test: `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx`
- Test: `docs/heroes/invoker.md`

- [ ] **Step 1: Run the Rust test suite**

Run:

```powershell
cargo test
```

Expected: PASS.

- [ ] **Step 2: Run the React test suite**

Run:

```powershell
npm --prefix src-ui test
```

Expected: PASS.

- [ ] **Step 3: Run the release build**

Run:

```powershell
cargo build --release
```

Expected: PASS.

- [ ] **Step 4: Confirm the working tree only contains the intended feature changes**

Run:

```powershell
git --no-pager status --short
```

Expected: only the Invoker semi-auto feature files are modified in the worktree being used for implementation.

## Self-review checklist

- Spec coverage: Tasks 1-3 cover the backend schema, preserved automatic path, new semi-auto state machine, deviation handling, and indefinite wait model. Tasks 4-5 cover the React field plumbing plus combo-only editor/list UI. Task 6 covers the required hero/config docs. Task 7 covers repo-wide verification.
- Placeholder scan: no `TBD`, `TODO`, "implement later", or "write tests for the above" placeholders remain.
- Type consistency: this plan consistently uses `execution_style` / `InvokerProfileExecutionStyle` on both Rust and TypeScript sides, and `build_semi_auto_execution_plan` / `InvokerSemiAutoSession` for the backend helper/state names.

