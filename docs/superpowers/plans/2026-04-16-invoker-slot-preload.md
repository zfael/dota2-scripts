# Invoker Slot Preload Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Correct Invoker slot-order tracking and add pair-aware spell preloading so ordered spell profiles execute with real `D`/`F` invoke behavior.

**Architecture:** Keep the change scoped to `src/actions/heroes/invoker.rs` by replacing the current reversed single-step spell planner with a small pair-aware spell-batch model. Consecutive spell steps are preloaded in profile order, cast oldest-to-newest using their actual slots (`F` then `D` when two spells are loaded), and manual cooldown waits still gate advancement before the next batch is prepared.

**Tech Stack:** Rust (existing Invoker worker queue, serde-backed settings already in place, tracing), existing GSI models, existing Rust unit tests in `src/actions/heroes/invoker.rs`, Markdown docs

---

## File Map

**Modify**

- `src/actions/heroes/invoker.rs` — replace the reversed slot assumptions, add slot-state helpers, introduce spell-batch preparation/execution, and update/add unit tests
- `docs/heroes/invoker.md` — explain that spell order is authored naturally but preloaded pairs execute using actual slot occupancy (`F` then `D` when two spells are loaded)
- `docs/reference/configuration.md` — optionally add one clarification sentence if the Invoker runtime notes need to mention slot-preload behavior (skip if no wording change is needed after implementation)

**Test**

- `cargo test apply_invoke_to_slot_state_places_new_spell_in_primary_and_shifts_old_primary_to_secondary --lib`
- `cargo test build_spell_batch_for_qw_pickoff_casts_secondary_then_primary --lib`
- `cargo test build_profile_execution_plan_for_qw_pickoff_keeps_tornado_then_emp_order --lib`
- `cargo test build_profile_execution_plan_for_qe_burst_preloads_first_pair_then_trailing_primary --lib`
- `cargo test manual_wait --lib`
- `cargo test`
- `cargo build --release`

---

### Task 1: Correct low-level Invoker slot semantics

**Files:**
- Modify: `src/actions/heroes/invoker.rs:63-185,512-735`

- [ ] **Step 1: Write the failing slot-semantics tests**

Add these tests near the existing Invoker planner tests in `src/actions/heroes/invoker.rs`:

```rust
#[test]
fn apply_invoke_to_slot_state_places_new_spell_in_primary_and_shifts_old_primary_to_secondary() {
    let slots = [
        Some("invoker_emp".to_string()),
        Some("invoker_tornado".to_string()),
    ];

    assert_eq!(
        apply_invoke_to_slot_state(&slots, "invoker_sun_strike"),
        [
            Some("invoker_sun_strike".to_string()),
            Some("invoker_emp".to_string()),
        ]
    );
}

#[test]
fn build_spell_batch_for_qw_pickoff_casts_secondary_then_primary() {
    let event = invoker_qw_fixture();
    let settings = Settings::default();
    let config = &settings.heroes.invoker;
    let state = InvokerObservedState::from_event(&event);
    let profile = find_profile(config, "qw-pickoff").expect("QW profile should exist");

    let spell_steps: Vec<_> = profile
        .steps
        .iter()
        .filter(|step| step.kind == InvokerProfileStepKind::Spell)
        .cloned()
        .collect();

    let (batch, next_slots, consumed) =
        build_spell_batch(&spell_steps, &state.active_spells, config)
            .expect("QW spell batch should build");

    assert_eq!(consumed, 2);
    assert_eq!(
        batch
            .iter()
            .map(|step| (step.target.as_str(), step.cast_key))
            .collect::<Vec<_>>(),
        vec![
            ("invoker_tornado", config.spell_slot_secondary_key),
            ("invoker_emp", config.spell_slot_primary_key),
        ]
    );
    assert_eq!(
        next_slots,
        [
            Some("invoker_emp".to_string()),
            Some("invoker_tornado".to_string()),
        ]
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```powershell
cargo test apply_invoke_to_slot_state_places_new_spell_in_primary_and_shifts_old_primary_to_secondary --lib
cargo test build_spell_batch_for_qw_pickoff_casts_secondary_then_primary --lib
```

Expected: FAIL because `apply_invoke_to_slot_state` and `build_spell_batch` do not exist yet, and the current planner still assumes a newly invoked spell casts from the secondary slot.

- [ ] **Step 3: Add the minimal slot-state helpers**

In `src/actions/heroes/invoker.rs`, replace the reversed invoke assumption with an explicit slot helper:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedSpellStep {
    target: String,
    cast_key: char,
    delay_after_ms: u64,
    completion_mode: InvokerProfileStepCompletionMode,
    completion_timeout_ms: u64,
}

fn apply_invoke_to_slot_state(
    slots: &[Option<String>; 2],
    spell_name: &str,
) -> [Option<String>; 2] {
    [Some(spell_name.to_string()), slots[0].clone()]
}

fn spell_cast_key_from_slots(
    slots: &[Option<String>; 2],
    spell_name: &str,
    config: &crate::config::settings::InvokerConfig,
) -> Option<char> {
    if slots[0].as_deref() == Some(spell_name) {
        Some(config.spell_slot_primary_key)
    } else if slots[1].as_deref() == Some(spell_name) {
        Some(config.spell_slot_secondary_key)
    } else {
        None
    }
}
```

Add the first version of `build_spell_batch` that prepares up to two consecutive spell steps in profile order:

```rust
fn build_spell_batch(
    steps: &[crate::config::settings::InvokerProfileStep],
    starting_slots: &[Option<String>; 2],
    config: &crate::config::settings::InvokerConfig,
) -> Option<(Vec<PreparedSpellStep>, [Option<String>; 2], usize)> {
    let mut current_slots = starting_slots.clone();
    let mut prepared = Vec::new();
    let mut consumed = 0usize;

    for step in steps.iter().take(2) {
        let mut next_slots = current_slots.clone();

        if spell_cast_key_from_slots(&next_slots, &step.target, config).is_none() {
            orb_recipe(&step.target, config)?;
            next_slots = apply_invoke_to_slot_state(&next_slots, &step.target);
        }

        let cast_key = spell_cast_key_from_slots(&next_slots, &step.target, config)?;
        prepared.push(PreparedSpellStep {
            target: step.target.clone(),
            cast_key,
            delay_after_ms: step.delay_after_ms,
            completion_mode: step.completion_mode.clone(),
            completion_timeout_ms: step.completion_timeout_ms,
        });
        current_slots = next_slots;
        consumed += 1;
    }

    if prepared.len() == 2 {
        prepared.reverse();
    }

    Some((prepared, current_slots, consumed))
}
```

- [ ] **Step 4: Run the targeted tests again**

Run:

```powershell
cargo test apply_invoke_to_slot_state_places_new_spell_in_primary_and_shifts_old_primary_to_secondary --lib
cargo test build_spell_batch_for_qw_pickoff_casts_secondary_then_primary --lib
```

Expected: PASS.

- [ ] **Step 5: Commit the slot-helper slice**

Run:

```powershell
git add src/actions/heroes/invoker.rs
git commit -m "fix: correct invoker slot rotation semantics"
```

---

### Task 2: Replace the spell runner with pair-aware preloading

**Files:**
- Modify: `src/actions/heroes/invoker.rs:145-401,632-820`

- [ ] **Step 1: Write the failing batch-execution tests**

Add these tests below the new Task 1 tests in `src/actions/heroes/invoker.rs`:

```rust
#[test]
fn build_profile_execution_plan_for_qw_pickoff_keeps_tornado_then_emp_order() {
    let event = invoker_qw_fixture();
    let settings = Settings::default();
    let config = &settings.heroes.invoker;
    let state = InvokerObservedState::from_event(&event);
    let profile = find_profile(config, "qw-pickoff").expect("QW profile should exist");

    let plan =
        build_profile_execution_plan(profile, &state.active_spells, config)
            .expect("QW execution plan should build");

    let planned_spells: Vec<_> = plan
        .iter()
        .filter_map(|action| match action {
            PlannedInvokerAction::Spell {
                target,
                cast_key,
                completion_mode,
                ..
            } => Some((target.as_str(), *cast_key, completion_mode.clone())),
            _ => None,
        })
        .collect();

    assert_eq!(
        planned_spells,
        vec![
            (
                "invoker_tornado",
                config.spell_slot_secondary_key,
                InvokerProfileStepCompletionMode::FixedDelay,
            ),
            (
                "invoker_emp",
                config.spell_slot_primary_key,
                InvokerProfileStepCompletionMode::FixedDelay,
            ),
        ]
    );
}

#[test]
fn build_profile_execution_plan_for_qe_burst_preloads_first_pair_then_trailing_primary() {
    let event = invoker_qe_fixture();
    let settings = Settings::default();
    let config = &settings.heroes.invoker;
    let state = InvokerObservedState::from_event(&event);
    let profile = find_profile(config, "qe-burst").expect("QE profile should exist");

    let plan =
        build_profile_execution_plan(profile, &state.active_spells, config)
            .expect("QE execution plan should build");

    let planned_spells: Vec<_> = plan
        .iter()
        .filter_map(|action| match action {
            PlannedInvokerAction::Spell {
                target,
                cast_key,
                completion_mode,
                ..
            } => Some((target.as_str(), *cast_key, completion_mode.clone())),
            _ => None,
        })
        .collect();

    assert_eq!(
        planned_spells,
        vec![
            (
                "invoker_sun_strike",
                config.spell_slot_secondary_key,
                InvokerProfileStepCompletionMode::WaitForCooldown,
            ),
            (
                "invoker_chaos_meteor",
                config.spell_slot_primary_key,
                InvokerProfileStepCompletionMode::FixedDelay,
            ),
            (
                "invoker_deafening_blast",
                config.spell_slot_primary_key,
                InvokerProfileStepCompletionMode::FixedDelay,
            ),
        ]
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```powershell
cargo test build_profile_execution_plan_for_qw_pickoff_keeps_tornado_then_emp_order --lib
cargo test build_profile_execution_plan_for_qe_burst_preloads_first_pair_then_trailing_primary --lib
```

Expected: FAIL because `build_profile_execution_plan` still follows the reversed invoke model and does not produce the `F, D, D` cast order for real spell pairs.

- [ ] **Step 3: Rebuild profile planning around spell batches**

Keep `run_profile` consuming a flat plan, but change `build_profile_execution_plan` so it prepares spell actions in pair-sized batches instead of calling the old reversed single-step planner:

```rust
fn build_profile_execution_plan(
    profile: &crate::config::settings::InvokerProfile,
    starting_active_spells: &[Option<String>; 2],
    config: &crate::config::settings::InvokerConfig,
) -> Option<Vec<PlannedInvokerAction>> {
    let mut plan = Vec::new();
    let mut current_slots = starting_active_spells.clone();
    let mut index = 0usize;

    while index < profile.steps.len() {
        let step = &profile.steps[index];

        match step.kind {
            InvokerProfileStepKind::Item => {
                plan.push(PlannedInvokerAction::Item {
                    target: step.target.clone(),
                    delay_after_ms: step.delay_after_ms,
                });
                index += 1;
            }
            InvokerProfileStepKind::Spell => {
                let spell_slice: Vec<_> = profile.steps[index..]
                    .iter()
                    .take_while(|candidate| candidate.kind == InvokerProfileStepKind::Spell)
                    .cloned()
                    .collect();

                let (batch, next_slots, consumed) =
                    build_spell_batch(&spell_slice, &current_slots, config)?;

                for prepared in batch {
                    plan.push(PlannedInvokerAction::Spell {
                        target: prepared.target,
                        cast_key: prepared.cast_key,
                        delay_after_ms: prepared.delay_after_ms,
                        completion_mode: prepared.completion_mode,
                        completion_timeout_ms: prepared.completion_timeout_ms,
                    });
                }

                current_slots = next_slots;
                index += consumed;
            }
        }
    }

    Some(plan)
}
```

After this change, update or replace the old reversed-assumption tests:

- remove the assertion that a newly invoked spell should cast from the secondary slot
- replace the old `multi_spell_combo_tracks_slot_state_after_mid_combo_invoke` expectation with the new pair-aware semantics
- if `run_profile` still simulates slot movement anywhere, switch that mutation to `apply_invoke_to_slot_state` so logs and manual-wait skips stay aligned with the planner

- [ ] **Step 4: Run targeted batch and manual-wait verification**

Run:

```powershell
cargo test build_profile_execution_plan_for_qw_pickoff_keeps_tornado_then_emp_order --lib
cargo test build_profile_execution_plan_for_qe_burst_preloads_first_pair_then_trailing_primary --lib
cargo test manual_wait --lib
```

Expected: PASS.

- [ ] **Step 5: Commit the batch-runner slice**

Run:

```powershell
git add src/actions/heroes/invoker.rs
git commit -m "feat: preload invoker spell pairs by slot order"
```

---

### Task 3: Refresh docs and run full verification

**Files:**
- Modify: `docs/heroes/invoker.md`
- Modify: `docs/reference/configuration.md` (only if the runtime wording needs clarification after implementation)

- [ ] **Step 1: Update the Invoker doc**

Add or revise the runtime section in `docs/heroes/invoker.md` with language like:

```md
### Spell preload behavior

Invoker spell profiles are still authored in natural cast order, such as:

`Tornado -> EMP -> Sun Strike`

The runtime now preloads up to two consecutive spell steps in profile order and
casts them using their actual Invoked slots:

- when two spells are loaded, the older prepared spell is usually on `F`
- the newer prepared spell is usually on `D`
- so a preloaded pair executes as `F` first, then `D`

For the example above, the runtime prepares Tornado then EMP, casts Tornado from
`F`, casts EMP from `D`, then invokes and casts Sun Strike from `D`.
```

- [ ] **Step 2: Update configuration wording only if needed**

If `docs/reference/configuration.md` needs a runtime note, add one sentence under
the Invoker table:

```md
Ordered spell steps are authored in natural cast order, but the runtime may
preload them into `F` then `D` because Invoked spell slots rotate by recency.
```

If the existing wording is already clear after the hero-doc update, skip this
file.

- [ ] **Step 3: Run full verification**

Run:

```powershell
cargo test
cargo build --release
```

Expected: PASS.

- [ ] **Step 4: Commit the docs and verification sweep**

Run:

```powershell
git add docs/heroes/invoker.md docs/reference/configuration.md
git commit -m "docs: describe invoker slot preload behavior"
```

If `docs/reference/configuration.md` was intentionally unchanged, omit it from
the `git add` command.

- [ ] **Step 5: Confirm clean status**

Run:

```powershell
git status --short
```

Expected: empty output before requesting review or merge/push actions.
