# Standalone Combo Offload Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Offload Tiny and Legion Commander standalone combos off the hotkey consumer thread while preserving their existing combo timing and hero-selection behavior.

**Architecture:** Keep `main.rs` as the hotkey gate and keep `ActionDispatcher` as the standalone-routing boundary, but teach the dispatcher to enqueue only Tiny and Legion standalone triggers onto `ActionExecutor` instead of running those long combo sequences inline. Shadow Fiend, Largo, Huskar, and other standalone paths stay on their existing execution model for this slice.

**Tech Stack:** Rust, std::sync::{Arc, Mutex, mpsc}, tracing, cargo test, cargo build

---

## File structure and responsibilities

- Modify: `src\actions\dispatcher.rs`
  - Add the scoped Tiny/Legion standalone-offload decision and the tests that lock down routing/off-thread execution.
- Modify: `docs\architecture\runtime-flow.md`
  - Update the hotkey path and thread table notes so standalone combo execution is no longer described as always inline on the hotkey consumer thread.
- Modify: `docs\architecture\state-and-dispatch.md`
  - Update standalone dispatch flow documentation to explain that Tiny/Legion now enqueue onto `ActionExecutor` while other standalone paths keep their existing handling.
- Modify: `docs\heroes\tiny.md`
  - Document that Tiny's standalone combo still uses the latest cached GSI event at execution time but now runs off the hotkey consumer thread.
- Modify: `docs\heroes\legion_commander.md`
  - Document the same execution-path change for Legion Commander.

## Task 1: Add focused standalone-routing seams and tests

**Files:**
- Modify: `src\actions\dispatcher.rs`
- Test: `src\actions\dispatcher.rs`

- [ ] **Step 1: Add a tiny helper that encodes the scoped offload decision**

Add a small internal helper in `src\actions\dispatcher.rs` so the target-hero policy is explicit and testable:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StandaloneDispatchMode {
    Inline,
    Executor,
}

fn standalone_dispatch_mode(hero_name: &str) -> StandaloneDispatchMode {
    match hero_name {
        "npc_dota_hero_tiny" | "npc_dota_hero_legion_commander" => StandaloneDispatchMode::Executor,
        _ => StandaloneDispatchMode::Inline,
    }
}
```

- [ ] **Step 2: Add deterministic mapping tests**

Add tests in `src\actions\dispatcher.rs` covering:

```rust
#[test]
fn tiny_and_legion_use_executor_standalone_mode() {}

#[test]
fn shadow_fiend_and_huskar_keep_existing_standalone_mode() {}
```

- [ ] **Step 3: Add a runtime-facing off-thread dispatch regression**

Add a focused dispatcher test using a fake `HeroScript` and real `ActionExecutor` that proves Tiny/Legion standalone dispatch returns promptly instead of blocking the caller until the combo body finishes:

```rust
#[test]
fn executor_standalone_dispatch_returns_before_blocking_script_finishes() {
    // fake script blocks on a gate after signaling start
    // dispatch_standalone_trigger("npc_dota_hero_tiny") should return quickly
    // script execution should still happen later on the executor worker
}
```

Use a simple channel/flag gate so the test is deterministic and does not depend on arbitrary long sleeps.

- [ ] **Step 4: Add a regression that unchanged heroes still dispatch inline**

Add one focused test that proves a hero outside the scoped set still runs through the current inline path:

```rust
#[test]
fn inline_standalone_dispatch_keeps_existing_behavior_for_other_heroes() {}
```

- [ ] **Step 5: Run focused dispatcher tests before the implementation**

Run: `cargo test dispatcher -- --nocapture`

Expected: at least the new off-thread dispatch regression fails before the implementation lands, while the file still compiles.

- [ ] **Step 6: Commit the test-first slice**

```bash
git add src/actions/dispatcher.rs
git commit -m "test: add standalone combo dispatch coverage"
```

## Task 2: Implement scoped standalone combo offload

**Files:**
- Modify: `src\actions\dispatcher.rs`
- Test: `src\actions\dispatcher.rs`

- [ ] **Step 1: Keep the existing standalone public API stable**

Do not change the `main.rs` call site if it is not required. Keep:

```rust
dispatcher.dispatch_standalone_trigger(hero_name);
```

The dispatcher should absorb the new async behavior internally.

- [ ] **Step 2: Add the scoped executor branch inside `dispatch_standalone_trigger()`**

Implement the dispatcher logic in `src\actions\dispatcher.rs` so it chooses behavior by `standalone_dispatch_mode(hero_name)`:

```rust
pub fn dispatch_standalone_trigger(&self, hero_name: &str) {
    if let Some(hero_script) = self.hero_scripts.get(hero_name) {
        match standalone_dispatch_mode(hero_name) {
            StandaloneDispatchMode::Inline => hero_script.handle_standalone_trigger(),
            StandaloneDispatchMode::Executor => {
                let hero_name = hero_name.to_string();
                let hero_script = Arc::clone(hero_script);
                self.executor.enqueue("standalone-combo", move || {
                    debug!("Running offloaded standalone trigger for {}", hero_name);
                    hero_script.handle_standalone_trigger();
                });
            }
        }
    }
}
```

- [ ] **Step 3: Keep execution-time event semantics unchanged**

Do not introduce a new hotkey-time snapshot for Tiny or Legion. Their existing `handle_standalone_trigger()` logic should continue reading the latest cached GSI context at execution time:

- Tiny still reads `LAST_GSI_EVENT` when the executor job runs
- Legion Commander still reads `last_event` when the executor job runs

- [ ] **Step 4: Re-run focused dispatcher tests**

Run: `cargo test dispatcher -- --nocapture`

Expected: all standalone dispatch tests pass, including the off-thread regression and the unchanged-inline regression.

- [ ] **Step 5: Review the scoped code diff**

Run:

```bash
git --no-pager diff -- src/actions/dispatcher.rs
```

Check:
- only Tiny and Legion standalone dispatch now enqueue onto `ActionExecutor`
- Shadow Fiend and Largo paths are untouched
- `main.rs` did not need routing logic duplicated into it

- [ ] **Step 6: Commit the implementation**

```bash
git add src/actions/dispatcher.rs
git commit -m "perf: offload standalone combo dispatch"
```

## Task 3: Update docs and verify the full slice

**Files:**
- Modify: `docs\architecture\runtime-flow.md`
- Modify: `docs\architecture\state-and-dispatch.md`
- Modify: `docs\heroes\tiny.md`
- Modify: `docs\heroes\legion_commander.md`
- Modify: `src\actions\dispatcher.rs` only if verification exposes a real bug or flaky test

- [ ] **Step 1: Update runtime-flow documentation**

Adjust `docs\architecture\runtime-flow.md` so it explains:

```md
- the hotkey consumer thread still resolves `standalone_enabled` and the selected hero
- Tiny and Legion standalone triggers now enqueue onto `ActionExecutor`
- Largo manual hotkeys still use their dedicated handling
- Shadow Fiend standalone handling remains on its existing specialized path
```

- [ ] **Step 2: Update state-and-dispatch documentation**

Adjust `docs\architecture\state-and-dispatch.md` so the standalone flow reflects the new scoped async branch:

```md
1. `src/input/keyboard.rs` emits `HotkeyEvent::ComboTrigger`
2. `src/main.rs` resolves `standalone_enabled` and `selected_hero`
3. `ActionDispatcher::dispatch_standalone_trigger(hero_name)` routes the hero
4. Tiny/Legion enqueue onto `ActionExecutor`; other heroes keep their current path
```

- [ ] **Step 3: Update Tiny and Legion hero docs**

Add short execution-path notes to:

- `docs\heroes\tiny.md`
- `docs\heroes\legion_commander.md`

Document that:
- the combo still uses the latest cached GSI event at execution time
- the combo timing/order itself is unchanged
- the long combo no longer blocks the hotkey consumer thread directly

- [ ] **Step 4: Run full automated verification**

Run:

```bash
cargo test
cargo build --release --target-dir target/release-verify
```

Expected:
- `cargo test` passes
- `cargo build --release --target-dir target/release-verify` passes
- only pre-existing warnings remain unless this slice introduces a new one that must be fixed

- [ ] **Step 5: Review the scoped diff**

Run:

```bash
git --no-pager diff -- src/actions/dispatcher.rs docs/architecture/runtime-flow.md docs/architecture/state-and-dispatch.md docs/heroes/tiny.md docs/heroes/legion_commander.md
```

Check:
- the async branch is clearly scoped to Tiny/Legion only
- docs accurately describe the new standalone path
- no unrelated hero behavior drift slipped in

- [ ] **Step 6: Commit docs + verification follow-up**

```bash
git add src/actions/dispatcher.rs docs/architecture/runtime-flow.md docs/architecture/state-and-dispatch.md docs/heroes/tiny.md docs/heroes/legion_commander.md
git commit -m "docs: update standalone combo runtime flow"
```

- [ ] **Step 7: Manual gameplay handoff**

Record the later gameplay checklist:

```text
- Tiny standalone combo still fires in the same order and timing.
- Legion Commander standalone combo still fires in the same order and timing.
- Repeated standalone hotkeys no longer make keyboard-triggered behavior feel stalled.
- Shadow Fiend and Largo standalone behavior remain unchanged.
```
