# Invoker Manual Step Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add per-step manual completion rules for Invoker profiles so steps like Sun Strike can wait for cooldown confirmation before the combo continues.

**Architecture:** Extend the existing `InvokerProfileStep` model with explicit completion metadata, then thread that metadata through the existing Invoker planner and worker queue. Keep the fast path unchanged for normal steps and only poll the latest Invoker GSI snapshot when a spell step opts into cooldown waiting.

**Tech Stack:** Rust (serde, tracing, existing Invoker worker queue), React + TypeScript, Vitest, checked-in TOML config, existing Invoker docs

---

## File Map

**Modify**

- `src/config/settings.rs:405-454,1137-1237,1726-1742` — add step completion enum/fields, seed defaults, extend config tests
- `config/config.toml:245-325` — expose the new step completion fields in checked-in Invoker presets
- `src/actions/heroes/invoker.rs:67-170,231-301,433-635` — carry completion metadata into planned steps and add cooldown-wait execution helpers/tests
- `src-ui/src/types/config.ts:188-206` — mirror the new completion-mode fields in the frontend config model
- `src-ui/src/stores/mockData.ts` — keep mock config aligned with the checked-in Invoker defaults
- `src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx:50-115` — seed preset metadata and blank-step defaults with completion fields
- `src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx:137-257` — expose per-step completion controls in the editor
- `src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx:17-25,63-114` — make manual-wait steps visible in profile summaries
- `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx:11-36` — cover manual completion controls and persistence
- `docs/heroes/invoker.md:98-170` — document manual cooldown-wait steps and the QE Burst default
- `docs/reference/configuration.md` — document `completion_mode` and `completion_timeout_ms`

**Test**

- `cargo test invoker_qe_burst_defaults_to_manual_sun_strike_wait --lib`
- `cargo test manual_wait --lib`
- `npm --prefix src-ui test -- InvokerConfig.test.tsx`
- `cargo test`
- `npm --prefix src-ui test`
- `cargo build --release`

---

### Task 1: Extend the Invoker step config model

**Files:**
- Modify: `src/config/settings.rs:405-454,1137-1237,1726-1742`
- Modify: `config/config.toml:245-325`
- Modify: `src-ui/src/types/config.ts:188-206`
- Modify: `src-ui/src/stores/mockData.ts`
- Modify: `src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx:50-115`

- [ ] **Step 1: Write the failing backend config test**

Add this test near the existing Invoker config tests in `src/config/settings.rs`:

```rust
#[test]
fn invoker_qe_burst_defaults_to_manual_sun_strike_wait() {
    let settings = Settings::default();
    let qe = settings
        .heroes
        .invoker
        .profiles
        .iter()
        .find(|profile| profile.id == "qe-burst")
        .expect("QE Burst profile should exist");

    let sun_strike = qe
        .steps
        .first()
        .expect("QE Burst should seed Sun Strike as the first step");

    assert_eq!(
        sun_strike.completion_mode,
        InvokerProfileStepCompletionMode::WaitForCooldown
    );
    assert_eq!(sun_strike.completion_timeout_ms, 3000);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```powershell
cargo test invoker_qe_burst_defaults_to_manual_sun_strike_wait --lib
```

Expected: FAIL because `InvokerProfileStep` does not yet expose `completion_mode` or `completion_timeout_ms`.

- [ ] **Step 3: Add the new completion fields across backend and frontend config**

Update `src/config/settings.rs` so Invoker steps carry completion metadata:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvokerProfileStepCompletionMode {
    FixedDelay,
    WaitForCooldown,
}

fn default_invoker_profile_step_completion_mode() -> InvokerProfileStepCompletionMode {
    InvokerProfileStepCompletionMode::FixedDelay
}

fn default_invoker_profile_step_completion_timeout_ms() -> u64 {
    3000
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
    #[serde(default)]
    pub notes: String,
}
```

Seed QE Burst with a manual Sun Strike wait in both `src/config/settings.rs` and `config/config.toml`:

```rust
InvokerProfileStep {
    kind: InvokerProfileStepKind::Spell,
    target: "invoker_sun_strike".to_string(),
    delay_after_ms: 150,
    completion_mode: InvokerProfileStepCompletionMode::WaitForCooldown,
    completion_timeout_ms: 3000,
    notes: String::new(),
}
```

```toml
[[heroes.invoker.profiles.steps]]
kind = "spell"
target = "invoker_sun_strike"
delay_after_ms = 150
completion_mode = "wait_for_cooldown"
completion_timeout_ms = 3000
notes = ""
```

Mirror the same shape in `src-ui/src/types/config.ts`, `src-ui/src/stores/mockData.ts`, and `src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx`:

```ts
export type InvokerProfileStepCompletionMode =
  | "fixed_delay"
  | "wait_for_cooldown";

export interface InvokerProfileStep {
  kind: InvokerProfileStepKind;
  target: string;
  delay_after_ms: number;
  completion_mode: InvokerProfileStepCompletionMode;
  completion_timeout_ms: number;
  notes: string;
}
```

```ts
export function createInvokerStep(kind: InvokerProfileStepKind): InvokerProfileStep {
  const fallback = (kind === "spell" ? INVOKER_SPELLS[0] : INVOKER_ITEMS[0])!;
  return {
    kind,
    target: fallback.id,
    delay_after_ms: kind === "spell" ? 100 : 50,
    completion_mode: "fixed_delay",
    completion_timeout_ms: 3000,
    notes: "",
  };
}
```

- [ ] **Step 4: Run the targeted config test again**

Run:

```powershell
cargo test invoker_qe_burst_defaults_to_manual_sun_strike_wait --lib
```

Expected: PASS.

- [ ] **Step 5: Commit the config model change**

Run:

```powershell
git add src/config/settings.rs config/config.toml src-ui/src/types/config.ts src-ui/src/stores/mockData.ts src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx
git commit -m "feat: add invoker step completion config"
```

---

### Task 2: Add cooldown-aware execution to the Invoker runner

**Files:**
- Modify: `src/actions/heroes/invoker.rs:67-170,231-301,433-635`

- [ ] **Step 1: Write the failing Invoker runtime tests**

Add these tests near the existing Invoker tests in `src/actions/heroes/invoker.rs`:

```rust
#[test]
fn manual_wait_planner_copies_completion_metadata() {
    let event = invoker_qe_fixture();
    let settings = Settings::default();
    let state = InvokerObservedState::from_event(&event);
    let profile = find_profile(&settings.heroes.invoker, "qe-burst")
        .expect("QE profile should exist");

    let plan = build_profile_execution_plan(profile, &state, &settings.heroes.invoker)
        .expect("QE profile should build");

    let first_spell = plan
        .iter()
        .find_map(|step| match step {
            PlannedInvokerAction::Spell {
                completion_mode,
                completion_timeout_ms,
                ..
            } => Some((completion_mode.clone(), *completion_timeout_ms)),
            PlannedInvokerAction::Item { .. } => None,
        })
        .expect("QE profile should include a spell step");

    assert_eq!(
        first_spell.0,
        InvokerProfileStepCompletionMode::WaitForCooldown
    );
    assert_eq!(first_spell.1, 3000);
}

#[test]
fn manual_wait_detects_already_on_cooldown() {
    let mut event = invoker_qe_fixture();
    event.abilities.ability4.name = "invoker_sun_strike".to_string();
    event.abilities.ability4.cooldown = 12;
    event.abilities.ability4.can_cast = false;

    assert!(spell_is_on_cooldown(&event, "invoker_sun_strike"));
}

#[test]
fn manual_wait_completes_after_gsi_update() {
    let mut event = invoker_qe_fixture();
    event.abilities.ability4.name = "invoker_sun_strike".to_string();
    event.abilities.ability4.cooldown = 0;
    *INVOKER_LAST_EVENT.lock().unwrap() = Some(event.clone());

    let updater = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(30));
        let mut cooling = event;
        cooling.abilities.ability4.cooldown = 25;
        cooling.abilities.ability4.can_cast = false;
        *INVOKER_LAST_EVENT.lock().unwrap() = Some(cooling);
    });

    let outcome = wait_for_spell_cooldown_start("invoker_sun_strike", 300, 5);
    updater.join().expect("updater should finish");

    assert_eq!(outcome, CooldownWaitOutcome::Started);
}

#[test]
fn manual_wait_times_out_without_cooldown_start() {
    let mut event = invoker_qe_fixture();
    event.abilities.ability4.name = "invoker_sun_strike".to_string();
    event.abilities.ability4.cooldown = 0;
    *INVOKER_LAST_EVENT.lock().unwrap() = Some(event);

    assert_eq!(
        wait_for_spell_cooldown_start("invoker_sun_strike", 20, 5),
        CooldownWaitOutcome::TimedOut
    );
}

#[test]
fn manual_wait_stops_when_hero_becomes_unavailable() {
    let event = invoker_qe_fixture();
    *INVOKER_LAST_EVENT.lock().unwrap() = Some(event);

    let updater = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut disabled = invoker_qe_fixture();
        disabled.hero.alive = false;
        *INVOKER_LAST_EVENT.lock().unwrap() = Some(disabled);
    });

    let outcome = wait_for_spell_cooldown_start("invoker_sun_strike", 200, 5);
    updater.join().expect("updater should finish");

    assert_eq!(outcome, CooldownWaitOutcome::HeroUnavailable);
}
```

- [ ] **Step 2: Run the targeted runtime tests to verify they fail**

Run:

```powershell
cargo test manual_wait --lib
```

Expected: FAIL because the plan action and cooldown-wait helpers do not exist yet.

- [ ] **Step 3: Thread completion metadata through the planner and runner**

Update `PlannedInvokerAction::Spell` and add local cooldown-wait helpers in `src/actions/heroes/invoker.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
enum PlannedInvokerAction {
    Item {
        target: String,
        delay_after_ms: u64,
    },
    Spell {
        target: String,
        prepare_keys: Vec<char>,
        cast_key: char,
        delay_after_ms: u64,
        completion_mode: InvokerProfileStepCompletionMode,
        completion_timeout_ms: u64,
        should_cast: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CooldownWaitOutcome {
    Started,
    TimedOut,
    HeroUnavailable,
    SpellNotObserved,
}

fn spell_is_on_cooldown(event: &GsiWebhookEvent, spell_name: &str) -> bool {
    [4u8, 5u8]
        .into_iter()
        .filter_map(|index| event.abilities.get_by_index(index))
        .any(|ability| ability.name == spell_name && ability.cooldown > 0)
}

fn wait_for_spell_cooldown_start(
    spell_name: &str,
    timeout_ms: u64,
    poll_interval_ms: u64,
) -> CooldownWaitOutcome {
    let started = std::time::Instant::now();

    loop {
        let Some(event) = INVOKER_LAST_EVENT.lock().unwrap().clone() else {
            return CooldownWaitOutcome::SpellNotObserved;
        };

        if !event.hero.alive || event.hero.stunned || event.hero.hexed || event.hero.silenced {
            return CooldownWaitOutcome::HeroUnavailable;
        }

        if spell_is_on_cooldown(&event, spell_name) {
            return CooldownWaitOutcome::Started;
        }

        if started.elapsed() >= Duration::from_millis(timeout_ms) {
            return CooldownWaitOutcome::TimedOut;
        }

        thread::sleep(Duration::from_millis(poll_interval_ms));
    }
}
```

When building the plan, preserve the new step fields:

```rust
actions.push(PlannedInvokerAction::Spell {
    target: step.target.clone(),
    prepare_keys: cast_plan.prepare_keys,
    cast_key: cast_plan.cast_key,
    delay_after_ms: step.delay_after_ms,
    completion_mode: step.completion_mode.clone(),
    completion_timeout_ms: step.completion_timeout_ms,
    should_cast: profile.mode == InvokerProfileMode::Combo,
});
```

In `run_profile`, gate manual waits only for `wait_for_cooldown` steps:

```rust
if should_cast {
    if completion_mode == InvokerProfileStepCompletionMode::WaitForCooldown
        && spell_is_on_cooldown(
            &INVOKER_LAST_EVENT.lock().unwrap().clone().unwrap_or_else(|| event.clone()),
            &target,
        )
    {
        info!("🔮 Manual step {} already on cooldown, skipping", target);
        continue;
    }

    info!("🔮 Casting {} from {}", target, cast_key);
    crate::input::simulation::press_key(cast_key);

    if completion_mode == InvokerProfileStepCompletionMode::WaitForCooldown {
        info!("🔮 Waiting for {} cooldown to start", target);
        match wait_for_spell_cooldown_start(&target, completion_timeout_ms, 25) {
            CooldownWaitOutcome::Started => {
                info!("🔮 {} entered cooldown; continuing profile", target);
            }
            CooldownWaitOutcome::TimedOut => {
                info!(
                    "🔮 Manual step {} timed out after {}ms; aborting profile",
                    target,
                    completion_timeout_ms
                );
                break;
            }
            CooldownWaitOutcome::HeroUnavailable => {
                info!("🔮 Hero unavailable while waiting for {}; aborting profile", target);
                break;
            }
            CooldownWaitOutcome::SpellNotObserved => {
                info!("🔮 Could not observe {} while waiting for cooldown; aborting profile", target);
                break;
            }
        }
    }
} else {
    info!("🔮 Prepared {} without casting", target);
}

thread::sleep(Duration::from_millis(delay_after_ms));
```

- [ ] **Step 4: Run the targeted runtime tests again**

Run:

```powershell
cargo test manual_wait --lib
```

Expected: PASS.

- [ ] **Step 5: Commit the Invoker runtime change**

Run:

```powershell
git add src/actions/heroes/invoker.rs
git commit -m "feat: wait for invoker manual spell cooldowns"
```

---

### Task 3: Expose completion controls in the Invoker editor

**Files:**
- Modify: `src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx:50-115`
- Modify: `src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx:137-257`
- Modify: `src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx:17-25,63-114`
- Modify: `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx:11-36`

- [ ] **Step 1: Write the failing frontend tests**

Extend `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx` with:

```tsx
it("renders QE Burst with manual cooldown wait controls", () => {
  render(<InvokerConfig />);

  fireEvent.click(screen.getByRole("button", { name: /qe burst/i }));

  expect(screen.getByText("Completion Mode")).toBeInTheDocument();
  expect(screen.getByDisplayValue("3000")).toBeInTheDocument();
});

it("persists completion mode edits into the config store", () => {
  render(<InvokerConfig />);

  fireEvent.click(screen.getByRole("button", { name: /qe burst/i }));
  fireEvent.change(screen.getAllByRole("combobox")[2], {
    target: { value: "fixed_delay" },
  });

  const qeProfile = useConfigStore
    .getState()
    .config.heroes.invoker.profiles.find((profile) => profile.id === "qe-burst");

  expect(qeProfile?.steps[0].completion_mode).toBe("fixed_delay");
});
```

- [ ] **Step 2: Run the targeted frontend test to verify it fails**

Run:

```powershell
npm --prefix src-ui test -- InvokerConfig.test.tsx
```

Expected: FAIL because the editor does not yet render completion controls or persist the new fields.

- [ ] **Step 3: Add completion controls and summary labels**

Update `src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx` to keep the new fields when editing steps and show completion controls for spell steps:

```tsx
const COMPLETION_MODE_OPTIONS = [
  { value: "fixed_delay", label: "Fixed Delay" },
  { value: "wait_for_cooldown", label: "Wait for Cooldown" },
];
```

```tsx
setStep(index, {
  kind: nextKind,
  target: nextTarget,
  delay_after_ms: step.delay_after_ms,
  completion_mode:
    nextKind === "item" ? "fixed_delay" : step.completion_mode,
  completion_timeout_ms: step.completion_timeout_ms,
  notes: step.notes,
});
```

```tsx
{step.kind === "spell" ? (
  <>
    <Dropdown
      label="Completion Mode"
      value={step.completion_mode}
      options={COMPLETION_MODE_OPTIONS}
      onChange={(completion_mode) =>
        setStep(index, {
          ...step,
          completion_mode:
            completion_mode as InvokerProfileStepCompletionMode,
        })
      }
    />
    {step.completion_mode === "wait_for_cooldown" ? (
      <NumberInput
        label="Completion Timeout"
        value={step.completion_timeout_ms}
        onChange={(completion_timeout_ms) =>
          setStep(index, { ...step, completion_timeout_ms })
        }
        suffix="ms"
      />
    ) : null}
  </>
) : (
  <div className="rounded-md border border-border bg-input px-3 py-2 text-xs text-subtle md:col-span-2">
    Items always use fixed delay completion.
  </div>
)}
```

Make manual steps visible in `src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx`:

```tsx
function stepSummary(profile: InvokerProfile) {
  if (!profile.steps.length) {
    return "No steps yet";
  }

  return profile.steps
    .map((step) => {
      const label = getInvokerStepLabel(step.target);
      return step.completion_mode === "wait_for_cooldown"
        ? `${label} [manual]`
        : label;
    })
    .join(" → ");
}
```

- [ ] **Step 4: Run the targeted frontend test again**

Run:

```powershell
npm --prefix src-ui test -- InvokerConfig.test.tsx
```

Expected: PASS.

- [ ] **Step 5: Commit the UI update**

Run:

```powershell
git add src-ui/src/components/heroes/configs/invoker/invokerCatalog.tsx src-ui/src/components/heroes/configs/invoker/InvokerProfileEditor.tsx src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx src-ui/src/components/heroes/configs/InvokerConfig.test.tsx
git commit -m "feat(ui): add invoker manual completion controls"
```

---

### Task 4: Document the feature and run repo verification

**Files:**
- Modify: `docs/heroes/invoker.md:98-170`
- Modify: `docs/reference/configuration.md`

- [ ] **Step 1: Update the Invoker hero doc**

Add a manual-wait subsection to `docs/heroes/invoker.md` under runtime behavior:

```md
### Manual cooldown-wait steps

Spell steps can now choose how completion is detected:

- `fixed_delay` keeps the current execute-then-delay behavior
- `wait_for_cooldown` is for manual-targeted spells such as Sun Strike

When `wait_for_cooldown` is selected, the runner presses the active spell key once and then waits for the spell to enter cooldown before it advances. If the spell is already on cooldown when the step starts, the runner skips that step. If cooldown never starts before `completion_timeout_ms`, the remaining profile is aborted.

The checked-in QE Burst preset uses `wait_for_cooldown` on Sun Strike with a `3000` ms timeout.
```

- [ ] **Step 2: Update the configuration reference**

Extend the Invoker step table in `docs/reference/configuration.md`:

```md
| `profiles[].steps[].completion_mode` | string | `fixed_delay` | `fixed_delay` or `wait_for_cooldown` for spell steps |
| `profiles[].steps[].completion_timeout_ms` | u64 | `3000` | Timeout used when waiting for cooldown confirmation |
```

- [ ] **Step 3: Run targeted and full verification**

Run:

```powershell
cargo test manual_wait --lib
npm --prefix src-ui test -- InvokerConfig.test.tsx
cargo test
npm --prefix src-ui test
cargo build --release
```

Expected: all commands PASS.

- [ ] **Step 4: Commit the docs and verification sweep**

Run:

```powershell
git add docs/heroes/invoker.md docs/reference/configuration.md
git commit -m "docs: document invoker manual step completion"
```

- [ ] **Step 5: Request review**

Run:

```powershell
git status --short
```

Expected: empty output before requesting review or moving to merge/push work.
