# Invoker Profile Builder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Invoker's raw-string combo config with a preset-first named profile system that supports custom combo/prep profiles, per-profile hotkeys, ordered spell/item steps, inline validation, and a real UI builder.

**Architecture:** Move Invoker from the current `primary_profile` / `prep_profile` branching model to a single greenfield profile-list config. On the backend, add a generic profile runner and route named Invoker hotkeys through the existing keyboard → dispatcher → hero-script pipeline. On the frontend, replace the placeholder Invoker config with a profile list/editor backed by bundled app-owned Invoker metadata and visuals.

**Tech Stack:** Rust (serde, tracing, existing keyboard/dispatcher pipeline), React + TypeScript + Zustand, existing common UI components (`Card`, `KeyInput`, `NumberInput`, `Toggle`), Vitest, cargo test

---

## File Map

**Modify**

- `src/config/settings.rs` — replace the current Invoker raw profile fields with named profile structs, defaults, and serde support
- `config/config.toml` — seed default Invoker profiles in TOML
- `src/actions/heroes/invoker.rs` — replace hardcoded `primary_profile`/`prep_profile` branches with a generic named profile runner
- `src/input/keyboard.rs` — map Invoker profile hotkeys from config into hotkey events
- `src/actions/dispatcher.rs` — dispatch named Invoker profiles
- `src/main.rs` — handle named Invoker hotkey events in the headless entrypoint
- `src-tauri/src/lib.rs` — handle named Invoker hotkey events in the Tauri entrypoint
- `src-ui/src/types/config.ts` — mirror the new Invoker profile model
- `src-ui/src/stores/mockData.ts` — seed the new default Invoker profiles
- `src-ui/src/components/heroes/configs/InvokerConfig.tsx` — replace the placeholder UI with a profile builder shell
- `docs/heroes/invoker.md` — document the new profile model and UI
- `docs/reference/configuration.md` — document the new config format
- `docs/reference/file-index.md` — record new source files if created
- `docs/features/keyboard-interception.md` — update the Invoker hotkey model

**Create**

- `src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx` — app-owned Invoker spell/item metadata plus local SVG/icon presentation
- `src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx` — profile list and preset library UI
- `src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx` — profile editor and step editor UI
- `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx` — frontend tests for preset rendering, editing, reordering, and validation

**Test**

- `cargo test invoker --lib`
- `cargo test keyboard --lib`
- `npm --prefix src-ui test -- InvokerConfig.test.tsx`
- `cargo test`
- `npm --prefix src-ui test`
- `cargo build --release`

---

### Task 1: Replace Invoker's raw-string config with named profiles

**Files:**
- Modify: `src/config/settings.rs`
- Modify: `config/config.toml`
- Modify: `src-ui/src/types/config.ts`
- Modify: `src-ui/src/stores/mockData.ts`

- [ ] **Step 1: Write the failing backend config test**

Add this test near the existing config tests in `src/config/settings.rs`:

```rust
#[test]
fn invoker_defaults_seed_named_profiles() {
    let settings = Settings::default();
    let invoker = settings.heroes.invoker;

    let names: Vec<_> = invoker
        .profiles
        .iter()
        .map(|profile| (profile.name.as_str(), profile.mode.as_str(), profile.enabled))
        .collect();

    assert_eq!(
        names,
        vec![
            ("QW Pickoff", "combo", true),
            ("QE Burst", "combo", false),
            ("Ghost Walk Panic", "combo", true),
            ("Meteor + Blast Prep", "prep", true),
        ]
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```powershell
cargo test invoker_defaults_seed_named_profiles --lib
```

Expected: FAIL with missing `profiles` field / missing `mode.as_str()` support.

- [ ] **Step 3: Replace the Invoker config model in Rust**

In `src/config/settings.rs`, replace the current raw-string profile fields with structured profile types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvokerProfileMode {
    Combo,
    Prep,
}

impl InvokerProfileMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Combo => "combo",
            Self::Prep => "prep",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvokerProfileStepKind {
    Spell,
    Item,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvokerProfileStep {
    pub kind: InvokerProfileStepKind,
    pub target: String,
    #[serde(default)]
    pub delay_after_ms: u64,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvokerProfile {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub hotkey: String,
    pub mode: InvokerProfileMode,
    #[serde(default)]
    pub build_tag: String,
    #[serde(default)]
    pub steps: Vec<InvokerProfileStep>,
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
    #[serde(default = "default_invoker_profiles")]
    pub profiles: Vec<InvokerProfile>,
    #[serde(default)]
    pub armlet: HeroArmletOverrideConfig,
}
```

Seed the default profiles with `default_invoker_profiles()`:

```rust
fn default_invoker_profiles() -> Vec<InvokerProfile> {
    vec![
        InvokerProfile {
            id: "qw-pickoff".to_string(),
            name: "QW Pickoff".to_string(),
            enabled: true,
            hotkey: "Home".to_string(),
            mode: InvokerProfileMode::Combo,
            build_tag: "qw".to_string(),
            steps: vec![
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Item,
                    target: "item_spirit_vessel".to_string(),
                    delay_after_ms: 50,
                    notes: "".to_string(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Item,
                    target: "item_rod_of_atos".to_string(),
                    delay_after_ms: 50,
                    notes: "".to_string(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_tornado".to_string(),
                    delay_after_ms: 700,
                    notes: "".to_string(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_emp".to_string(),
                    delay_after_ms: 100,
                    notes: "".to_string(),
                },
            ],
        },
        InvokerProfile {
            id: "qe-burst".to_string(),
            name: "QE Burst".to_string(),
            enabled: false,
            hotkey: "PageDown".to_string(),
            mode: InvokerProfileMode::Combo,
            build_tag: "qe".to_string(),
            steps: vec![
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_sun_strike".to_string(),
                    delay_after_ms: 150,
                    notes: "".to_string(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_chaos_meteor".to_string(),
                    delay_after_ms: 450,
                    notes: "".to_string(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_deafening_blast".to_string(),
                    delay_after_ms: 100,
                    notes: "".to_string(),
                },
            ],
        },
        InvokerProfile {
            id: "ghost-walk-panic".to_string(),
            name: "Ghost Walk Panic".to_string(),
            enabled: true,
            hotkey: "End".to_string(),
            mode: InvokerProfileMode::Combo,
            build_tag: "general".to_string(),
            steps: vec![InvokerProfileStep {
                kind: InvokerProfileStepKind::Spell,
                target: "invoker_ghost_walk".to_string(),
                delay_after_ms: 100,
                notes: "".to_string(),
            }],
        },
        InvokerProfile {
            id: "meteor-blast-prep".to_string(),
            name: "Meteor + Blast Prep".to_string(),
            enabled: true,
            hotkey: "PageUp".to_string(),
            mode: InvokerProfileMode::Prep,
            build_tag: "qe".to_string(),
            steps: vec![
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_chaos_meteor".to_string(),
                    delay_after_ms: 0,
                    notes: "".to_string(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_deafening_blast".to_string(),
                    delay_after_ms: 0,
                    notes: "".to_string(),
                },
            ],
        },
    ]
}
```

- [ ] **Step 4: Replace the checked-in TOML and frontend mirror types**

Update `config/config.toml` to use array-of-tables:

```toml
[heroes.invoker]
quas_key = "q"
wex_key = "w"
exort_key = "e"
invoke_key = "r"
spell_slot_primary_key = "d"
spell_slot_secondary_key = "f"

[[heroes.invoker.profiles]]
id = "qw-pickoff"
name = "QW Pickoff"
enabled = true
hotkey = "Home"
mode = "combo"
build_tag = "qw"

[[heroes.invoker.profiles.steps]]
kind = "item"
target = "item_spirit_vessel"
delay_after_ms = 50

[[heroes.invoker.profiles.steps]]
kind = "item"
target = "item_rod_of_atos"
delay_after_ms = 50

[[heroes.invoker.profiles.steps]]
kind = "spell"
target = "invoker_tornado"
delay_after_ms = 700

[[heroes.invoker.profiles.steps]]
kind = "spell"
target = "invoker_emp"
delay_after_ms = 100
```

Update `src-ui/src/types/config.ts` and `src-ui/src/stores/mockData.ts`:

```ts
export type InvokerProfileMode = "combo" | "prep";
export type InvokerProfileStepKind = "spell" | "item";

export interface InvokerProfileStep {
  kind: InvokerProfileStepKind;
  target: string;
  delay_after_ms: number;
  notes: string;
}

export interface InvokerProfile {
  id: string;
  name: string;
  enabled: boolean;
  hotkey: string;
  mode: InvokerProfileMode;
  build_tag: string;
  steps: InvokerProfileStep[];
}

export interface InvokerConfig {
  quas_key: string;
  wex_key: string;
  exort_key: string;
  invoke_key: string;
  spell_slot_primary_key: string;
  spell_slot_secondary_key: string;
  profiles: InvokerProfile[];
  armlet: HeroArmletOverride;
}
```

- [ ] **Step 5: Run the targeted backend test to verify it passes**

Run:

```powershell
cargo test invoker_defaults_seed_named_profiles --lib
```

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/config/settings.rs config/config.toml src-ui/src/types/config.ts src-ui/src/stores/mockData.ts
git commit -m "feat: model invoker as named profiles"
```

---

### Task 2: Route named Invoker profiles through keyboard, dispatcher, and runtime

**Files:**
- Modify: `src/input/keyboard.rs`
- Modify: `src/actions/dispatcher.rs`
- Modify: `src/actions/heroes/invoker.rs`
- Modify: `src/main.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing runtime and hotkey tests**

Add these tests to `src/actions/heroes/invoker.rs` and `src/input/keyboard.rs`:

```rust
#[test]
fn invoker_profile_runner_keeps_declared_tornado_then_emp_order() {
    let settings = sample_settings();
    let state = sample_state_with_spells([None, None]);
    let profile = settings.heroes.invoker.profiles
        .iter()
        .find(|profile| profile.id == "qw-pickoff")
        .unwrap();

    let plan = build_profile_execution_plan(profile, &state, &settings.heroes.invoker)
        .expect("plan");

    let spell_targets: Vec<_> = plan
        .iter()
        .filter_map(|step| match step {
            PlannedInvokerAction::Spell { target, .. } => Some(target.as_str()),
            _ => None,
        })
        .collect();

    assert_eq!(spell_targets, vec!["invoker_tornado", "invoker_emp"]);
}
```

```rust
#[test]
fn plan_global_hotkey_event_maps_invoker_profile_hotkeys() {
    let snapshot = KeyboardSnapshot {
        selected_hero: Some(HeroType::Invoker),
        invoker_profiles: vec![
            InvokerHotkeyProfileSnapshot {
                id: "qw-pickoff".to_string(),
                hotkey: Some(Key::Home),
                enabled: true,
            },
        ],
        ..sample_keyboard_snapshot()
    };

    assert_eq!(
        plan_global_hotkey_event(Key::Home, &snapshot),
        Some(HotkeyEvent::InvokerProfile("qw-pickoff".to_string()))
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```powershell
cargo test invoker_profile_runner_keeps_declared_tornado_then_emp_order --lib
cargo test plan_global_hotkey_event_maps_invoker_profile_hotkeys --lib
```

Expected: FAIL with missing `build_profile_execution_plan`, missing `InvokerProfile` hotkey support, and/or missing `HotkeyEvent::InvokerProfile`.

- [ ] **Step 3: Introduce named-profile hotkey dispatch**

Update `src/input/keyboard.rs` so Invoker hotkeys are derived from profile data:

```rust
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum HotkeyEvent {
    ComboTrigger,
    MeepoFarmToggle,
    ArmletRoshanToggle,
    LargoQ,
    LargoW,
    LargoE,
    LargoR,
    InvokerProfile(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvokerHotkeyProfileSnapshot {
    pub id: String,
    pub hotkey: Option<Key>,
    pub enabled: bool,
}
```

In `plan_global_hotkey_event`:

```rust
if snapshot.selected_hero == Some(HeroType::Invoker) {
    if let Some(profile) = snapshot
        .invoker_profiles
        .iter()
        .find(|profile| profile.enabled && profile.hotkey == Some(key))
    {
        return Some(HotkeyEvent::InvokerProfile(profile.id.clone()));
    }
}
```

Update the snapshot-building code so it translates `settings.heroes.invoker.profiles` into `InvokerHotkeyProfileSnapshot` values.

- [ ] **Step 4: Replace hardcoded Invoker request types with named profile execution**

In `src/actions/heroes/invoker.rs`, replace the current request enum and branchy runner with profile-based execution:

```rust
#[derive(Debug, Clone)]
enum InvokerRequest {
    RunProfile(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlannedInvokerAction {
    Item { target: String, delay_after_ms: u64 },
    Spell {
        target: String,
        prepare_keys: Vec<char>,
        cast_key: char,
        delay_after_ms: u64,
        should_cast: bool,
    },
}
```

Add a generic planner:

```rust
fn build_profile_execution_plan(
    profile: &InvokerProfile,
    state: &InvokerObservedState,
    config: &crate::config::settings::InvokerConfig,
) -> Option<Vec<PlannedInvokerAction>> {
    let mut actions = Vec::new();
    let mut current_active_spells = state.active_spells.clone();

    for step in &profile.steps {
        match step.kind {
            InvokerProfileStepKind::Item => {
                actions.push(PlannedInvokerAction::Item {
                    target: step.target.clone(),
                    delay_after_ms: step.delay_after_ms,
                });
            }
            InvokerProfileStepKind::Spell => {
                let current_state = InvokerObservedState {
                    active_spells: current_active_spells.clone(),
                    ..state.clone()
                };
                let cast_plan = plan_single_spell(&step.target, &current_state, config)?;
                if !cast_plan.prepare_keys.is_empty() {
                    current_active_spells[0] = current_active_spells[1].clone();
                    current_active_spells[1] = Some(step.target.clone());
                }
                actions.push(PlannedInvokerAction::Spell {
                    target: step.target.clone(),
                    prepare_keys: cast_plan.prepare_keys,
                    cast_key: cast_plan.cast_key,
                    delay_after_ms: step.delay_after_ms,
                    should_cast: profile.mode == InvokerProfileMode::Combo,
                });
            }
        }
    }

    Some(actions)
}
```

Dispatch by profile ID from `dispatcher.rs`, `main.rs`, and `src-tauri/src/lib.rs`:

```rust
pub fn dispatch_invoker_profile(&self, profile_id: &str) {
    if let Some(hero) = self.hero_scripts.get(crate::models::Hero::Invoker.to_game_name()) {
        if let Some(invoker) = hero.as_any().downcast_ref::<crate::actions::heroes::InvokerScript>() {
            invoker.handle_profile_trigger(profile_id);
        }
    }
}
```

- [ ] **Step 5: Add step-by-step Invoker logs**

In `run_invoker_request`, emit logs that make slot/state visible:

```rust
info!("🔮 Invoker profile: {} ({:?})", profile.name, profile.mode);
info!("🔮 Planned steps: {:?}", profile.steps);
info!("🔮 Active slots before cast: {:?}", current_active_spells);
info!("🔮 Casting {} from {}", target, cast_key);
info!("🔮 Active slots after invoke: {:?}", current_active_spells);
```

- [ ] **Step 6: Run the targeted tests to verify they pass**

Run:

```powershell
cargo test invoker_profile_runner_keeps_declared_tornado_then_emp_order --lib
cargo test plan_global_hotkey_event_maps_invoker_profile_hotkeys --lib
```

Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/input/keyboard.rs src/actions/dispatcher.rs src/actions/heroes/invoker.rs src/main.rs src-tauri/src/lib.rs
git commit -m "feat: route invoker named profiles through runtime"
```

---

### Task 3: Build the preset-first Invoker profile editor UI

**Files:**
- Create: `src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx`
- Create: `src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx`
- Create: `src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx`
- Create: `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx`
- Modify: `src-ui/src/components/heroes/configs/InvokerConfig.tsx`

- [ ] **Step 1: Write the failing frontend tests**

Create `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import InvokerConfig from "./InvokerConfig";
import { useConfigStore } from "../../../stores/configStore";

describe("InvokerConfig", () => {
  beforeEach(() => {
    useConfigStore.setState((state) => ({
      ...state,
      config: {
        ...state.config,
        heroes: {
          ...state.config.heroes,
          invoker: {
            ...state.config.heroes.invoker,
            profiles: [
              {
                id: "qw-pickoff",
                name: "QW Pickoff",
                enabled: true,
                hotkey: "Home",
                mode: "combo",
                build_tag: "qw",
                steps: [
                  { kind: "spell", target: "invoker_tornado", delay_after_ms: 700, notes: "" },
                  { kind: "spell", target: "invoker_emp", delay_after_ms: 100, notes: "" },
                ],
              },
            ],
          },
        },
      },
    }));
  });

  it("renders invoker profiles as editable cards", () => {
    render(<InvokerConfig />);
    expect(screen.getByText("QW Pickoff")).toBeInTheDocument();
    expect(screen.getByText("invoker_tornado")).toBeInTheDocument();
    expect(screen.getByText("invoker_emp")).toBeInTheDocument();
  });

  it("lets the user duplicate a preset profile", async () => {
    const user = userEvent.setup();
    render(<InvokerConfig />);

    await user.click(screen.getByRole("button", { name: /duplicate q w pickoff/i }));

    expect(screen.getAllByText(/QW Pickoff/i).length).toBeGreaterThan(1);
  });
});
```

- [ ] **Step 2: Run the frontend test to verify it fails**

Run:

```powershell
npm --prefix src-ui test -- InvokerConfig.test.tsx
```

Expected: FAIL because the current Invoker UI still renders raw fields and placeholder combo-item text.

- [ ] **Step 3: Add bundled app-owned Invoker metadata and visuals**

Create `src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx` with local metadata and lightweight SVG visuals:

```tsx
import type { ReactNode } from "react";

export interface InvokerCatalogEntry {
  id: string;
  label: string;
  kind: "spell" | "item";
  icon: ReactNode;
}

const chip = (label: string, bg: string) => (
  <span className={`inline-flex h-6 min-w-6 items-center justify-center rounded-md px-1.5 text-[10px] font-semibold text-white ${bg}`}>
    {label}
  </span>
);

export const INVOKER_SPELLS: InvokerCatalogEntry[] = [
  { id: "invoker_tornado", label: "Tornado", kind: "spell", icon: chip("TO", "bg-sky-600") },
  { id: "invoker_emp", label: "EMP", kind: "spell", icon: chip("EM", "bg-violet-600") },
  { id: "invoker_chaos_meteor", label: "Meteor", kind: "spell", icon: chip("ME", "bg-orange-600") },
  { id: "invoker_deafening_blast", label: "Blast", kind: "spell", icon: chip("DB", "bg-amber-700") },
  { id: "invoker_ghost_walk", label: "Ghost Walk", kind: "spell", icon: chip("GW", "bg-cyan-700") },
];
```

- [ ] **Step 4: Build the profile list and editor components**

Create `InvokerProfileList.tsx`:

```tsx
import type { InvokerProfile } from "../../../../types/config";

interface InvokerProfileListProps {
  profiles: InvokerProfile[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onDuplicate: (id: string) => void;
  onDelete: (id: string) => void;
}

export function InvokerProfileList({ profiles, selectedId, onSelect, onDuplicate, onDelete }: InvokerProfileListProps) {
  return (
    <div className="space-y-2">
      {profiles.map((profile) => (
        <button
          key={profile.id}
          type="button"
          onClick={() => onSelect(profile.id)}
          className={`w-full rounded-lg border p-3 text-left ${selectedId === profile.id ? "border-accent bg-elevated" : "border-border bg-surface"}`}
        >
          <div className="flex items-center justify-between gap-2">
            <div>
              <div className="text-sm font-medium text-content">{profile.name}</div>
              <div className="text-xs text-subtle">{profile.mode} · {profile.hotkey}</div>
            </div>
            <div className="flex gap-2">
              <span className="text-xs text-subtle">{profile.steps.map((step) => step.target).join(" → ")}</span>
              <button type="button" aria-label={`duplicate ${profile.name}`} onClick={(e) => { e.stopPropagation(); onDuplicate(profile.id); }}>Duplicate</button>
              <button type="button" aria-label={`delete ${profile.name}`} onClick={(e) => { e.stopPropagation(); onDelete(profile.id); }}>Delete</button>
            </div>
          </div>
        </button>
      ))}
    </div>
  );
}
```

Create `InvokerProfileEditor.tsx`:

```tsx
import { KeyInput } from "../../../common/KeyInput";
import { NumberInput } from "../../../common/NumberInput";
import type { InvokerProfile } from "../../../../types/config";
import { INVOKER_SPELLS } from "./invokerCatalog";

interface InvokerProfileEditorProps {
  profile: InvokerProfile;
  onChange: (next: InvokerProfile) => void;
}

export function InvokerProfileEditor({ profile, onChange }: InvokerProfileEditorProps) {
  return (
    <div className="space-y-4">
      <input
        value={profile.name}
        onChange={(e) => onChange({ ...profile, name: e.target.value })}
        className="w-full rounded bg-elevated px-3 py-2 text-sm"
      />
      <KeyInput
        label="Hotkey"
        value={profile.hotkey}
        onChange={(hotkey) => onChange({ ...profile, hotkey })}
      />
      <div className="space-y-2">
        {profile.steps.map((step, index) => (
          <div key={`${profile.id}-${index}`} className="rounded border border-border p-3">
            <div className="flex items-center gap-2">
              {INVOKER_SPELLS.find((entry) => entry.id === step.target)?.icon ?? <span className="text-xs">{step.kind}</span>}
              <span className="text-sm text-content">{step.target}</span>
            </div>
            <NumberInput
              label="Delay After"
              value={step.delay_after_ms}
              onChange={(delay_after_ms) => {
                const steps = [...profile.steps];
                steps[index] = { ...step, delay_after_ms };
                onChange({ ...profile, steps });
              }}
              suffix="ms"
            />
          </div>
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Replace the placeholder Invoker config page**

Update `src-ui/src/components/heroes/configs/InvokerConfig.tsx`:

```tsx
import { useMemo, useState } from "react";
import { Card } from "../../common/Card";
import { useConfigStore } from "../../../stores/configStore";
import { InvokerProfileList } from "./invoker/InvokerProfileList";
import { InvokerProfileEditor } from "./invoker/InvokerProfileEditor";

export default function InvokerConfig() {
  const config = useConfigStore((s) => s.config.heroes.invoker);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const [selectedId, setSelectedId] = useState<string | null>(config.profiles[0]?.id ?? null);

  const selected = useMemo(
    () => config.profiles.find((profile) => profile.id === selectedId) ?? config.profiles[0] ?? null,
    [config.profiles, selectedId],
  );

  const setProfiles = (profiles: typeof config.profiles) => update("invoker", { profiles });

  return (
    <div className="grid gap-4 lg:grid-cols-[minmax(280px,360px)_1fr]">
      <Card title="Preset Library & Profiles">
        <InvokerProfileList
          profiles={config.profiles}
          selectedId={selected?.id ?? null}
          onSelect={setSelectedId}
          onDuplicate={(id) => {
            const source = config.profiles.find((profile) => profile.id === id);
            if (!source) return;
            const copy = { ...source, id: `${source.id}-copy`, name: `${source.name} Copy` };
            setProfiles([...config.profiles, copy]);
          }}
          onDelete={(id) => setProfiles(config.profiles.filter((profile) => profile.id !== id))}
        />
      </Card>

      <Card title="Profile Editor">
        {selected ? (
          <InvokerProfileEditor
            profile={selected}
            onChange={(next) =>
              setProfiles(config.profiles.map((profile) => (profile.id === next.id ? next : profile)))
            }
          />
        ) : (
          <p className="text-sm text-subtle">Select a profile to edit.</p>
        )}
      </Card>
    </div>
  );
}
```

- [ ] **Step 6: Run the frontend test to verify it passes**

Run:

```powershell
npm --prefix src-ui test -- InvokerConfig.test.tsx
```

Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx src-ui/src/components/heroes/configs/InvokerConfig.tsx src-ui/src/components/heroes/configs/InvokerConfig.test.tsx
git commit -m "feat(ui): add invoker profile builder"
```

---

### Task 4: Document the new Invoker model and verify the full repo

**Files:**
- Modify: `docs/heroes/invoker.md`
- Modify: `docs/reference/configuration.md`
- Modify: `docs/reference/file-index.md`
- Modify: `docs/features/keyboard-interception.md`

- [ ] **Step 1: Update the hero documentation**

In `docs/heroes/invoker.md`, replace the old profile explanation with the new model:

```md
## Profile Model

Invoker now uses a unified list of named profiles.

Each profile has:

- mode (`combo` or `prep`)
- hotkey
- enabled toggle
- ordered steps
- per-step delays

The shipped preset pack includes QW Pickoff, QE Burst, Ghost Walk Panic, and Meteor + Blast Prep.
```

Add a TOML example:

```toml
[[heroes.invoker.profiles]]
id = "qw-pickoff"
name = "QW Pickoff"
enabled = true
hotkey = "Home"
mode = "combo"
build_tag = "qw"
```

- [ ] **Step 2: Update the configuration reference and file index**

In `docs/reference/configuration.md`, replace the old Invoker table rows:

```md
| `profiles` | array | seeded preset list | Ordered named Invoker profiles. Each profile has `id`, `name`, `enabled`, `hotkey`, `mode`, `build_tag`, and `steps[]`. |
| `profiles[].steps[].target` | string | n/a | Stable spell/item target ID such as `invoker_tornado` or `item_spirit_vessel`. |
| `profiles[].steps[].delay_after_ms` | u64 | varies by preset | Delay applied after the step executes. |
```

If new UI source files were added, append them to `docs/reference/file-index.md`:

```md
| `src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx` | Invoker spell/item metadata plus local visuals |
| `src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx` | Invoker profile library/list UI |
| `src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx` | Invoker profile editor UI |
```

Update `docs/features/keyboard-interception.md`:

```md
- Invoker no longer reserves a single panic/prep hotkey pair. Instead, the keyboard layer maps any enabled Invoker profile hotkey to `HotkeyEvent::InvokerProfile(<id>)`.
```

- [ ] **Step 3: Run full verification**

Run:

```powershell
cargo test
npm --prefix src-ui test
cargo build --release
```

Expected:

- `cargo test` → PASS
- `npm --prefix src-ui test` → PASS
- `cargo build --release` → PASS

- [ ] **Step 4: Commit**

```bash
git add docs/heroes/invoker.md docs/reference/configuration.md docs/reference/file-index.md docs/features/keyboard-interception.md
git commit -m "docs: document invoker profile builder"
```

---

## Plan Self-Review

### Spec coverage

- **Unified named profiles** — covered by Task 1 config model and Task 2 runtime routing
- **Preset-first UI with custom builder** — covered by Task 3
- **Explicit order / Tornado→EMP trust** — covered by Task 2 runtime tests and logs, plus Task 3 profile summary UI
- **Bundled local assets** — covered by Task 3 `invokerCatalog.tsx`
- **Greenfield/no migration** — reflected in Task 1 replacement config model
- **Docs/testing/observability** — covered by Tasks 2 and 4

No gaps found.

### Placeholder scan

- No `TBD`, `TODO`, or “similar to Task N” placeholders
- Each task includes concrete files, code, commands, and expected outcomes

### Type consistency

- Rust types consistently use `InvokerProfile`, `InvokerProfileStep`, `InvokerProfileMode`, `InvokerProfileStepKind`
- Frontend types mirror the same names and shape
- Hotkey flow consistently uses `HotkeyEvent::InvokerProfile(String)`

No inconsistencies found.

