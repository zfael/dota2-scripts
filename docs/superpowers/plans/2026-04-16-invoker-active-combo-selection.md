# Invoker Active Combo Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a real active Invoker combo profile so UI clicks, direct combo hotkeys, the generic combo trigger, and a new cycle hotkey all agree on which combo is currently active.

**Architecture:** Keep config as the source of available profiles and their hotkeys, but move the currently active Invoker combo profile into runtime state. Wire that runtime state through `AppState`, keyboard planning, Tauri app-state emission, and the React UI so combo selection, execution, and feedback stay consistent without adding any overlay system.

**Tech Stack:** Rust (`AppState`, keyboard hook, dispatcher, Tauri commands/events), TypeScript/React/Zustand in `src-ui`, Vitest, existing Tauri app-state DTO/event bridge, Markdown docs

---

## File Map

**Modify**

- `src/state/app_state.rs` — add runtime-owned active Invoker combo profile helpers and tests
- `src/input/keyboard.rs` — add cycle-hotkey support, extend Invoker hotkey snapshots, and add keyboard tests
- `src/main.rs` — update hotkey event handling so Invoker combo hotkeys/cycle actions update active combo state, emit activity feedback, and trigger the active combo
- `src/actions/heroes/invoker.rs` — replace `first_enabled_combo_profile_id` fallback with active-combo lookup
- `src-tauri/src/ipc_types.rs` — extend `AppStateDto` with active Invoker combo fields
- `src-tauri/src/events.rs` — emit the new active-combo state to the frontend
- `src-tauri/src/commands/state.rs` — return active-combo state in `get_app_state` and add a command for UI-driven combo selection
- `src-tauri/src/lib.rs` — register the new Tauri command
- `src-ui/src/stores/uiStore.ts` — load/listen to active-combo state and invoke the new backend command
- `src-ui/src/stores/uiStore.test.ts` — verify UI store loads and reacts to active-combo app-state updates
- `src-ui/src/components/heroes/configs/InvokerConfig.tsx` — make combo-card clicks set the runtime active combo and render the active summary
- `src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx` — show the active marker distinctly from editor selection
- `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx` — verify clicking combo cards updates active combo, prep cards do not, and status renders correctly
- `docs/heroes/invoker.md` — document active combo semantics, cycle behavior, and direct combo hotkeys
- `docs/reference/configuration.md` — clarify runtime active-combo behavior vs config-defined profile list

**Test**

- `cargo test app_state_repairs_invalid_invoker_active_combo --lib`
- `cargo test plan_global_hotkey_event_cycles_enabled_invoker_combos --lib`
- `cargo test invoker_standalone_trigger_uses_active_combo_profile --lib`
- `npm --prefix src-ui test -- uiStore.test.ts`
- `npm --prefix src-ui test -- InvokerConfig.test.tsx`
- `cargo test`
- `npm --prefix src-ui test`
- `cargo build --release`

---

### Task 1: Add runtime active-combo state

**Files:**
- Modify: `src/state/app_state.rs:1-172`
- Test: `src/state/app_state.rs:152-220`

- [ ] **Step 1: Write the failing runtime-state tests**

Add these tests to `src/state/app_state.rs`:

```rust
    #[test]
    fn invoker_active_combo_defaults_to_none() {
        let state = AppState::default();
        assert_eq!(state.invoker_active_combo_profile_id, None);
    }

    #[test]
    fn app_state_repairs_invalid_invoker_active_combo() {
        let mut state = AppState::default();
        state.invoker_active_combo_profile_id = Some("deleted-profile".to_string());

        let repaired = state.repair_invoker_active_combo(&[
            InvokerComboProfileState {
                id: "qw-pickoff".to_string(),
                enabled: true,
                mode: "combo".to_string(),
            },
            InvokerComboProfileState {
                id: "meteor-blast-prep".to_string(),
                enabled: true,
                mode: "prep".to_string(),
            },
        ]);

        assert_eq!(repaired.as_deref(), Some("qw-pickoff"));
        assert_eq!(
            state.invoker_active_combo_profile_id.as_deref(),
            Some("qw-pickoff")
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```powershell
cargo test app_state_repairs_invalid_invoker_active_combo --lib
```

Expected: FAIL because `invoker_active_combo_profile_id`, `InvokerComboProfileState`, and `repair_invoker_active_combo(...)` do not exist yet.

- [ ] **Step 3: Add minimal runtime state and helpers**

Update `src/state/app_state.rs` by adding the new helper type and field:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvokerComboProfileState {
    pub id: String,
    pub enabled: bool,
    pub mode: String,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub invoker_active_combo_profile_id: Option<String>,
}
```

Add the new default field value in `impl Default for AppState`:

```rust
            invoker_active_combo_profile_id: None,
```

And add the repair helper in `impl AppState`:

```rust
    pub fn repair_invoker_active_combo(
        &mut self,
        profiles: &[InvokerComboProfileState],
    ) -> Option<String> {
        let current = self.invoker_active_combo_profile_id.as_deref();

        let is_valid = current.and_then(|id| {
            profiles.iter().find(|profile| {
                profile.id == id && profile.enabled && profile.mode == "combo"
            })
        });

        if is_valid.is_some() {
            return self.invoker_active_combo_profile_id.clone();
        }

        self.invoker_active_combo_profile_id = profiles
            .iter()
            .find(|profile| profile.enabled && profile.mode == "combo")
            .map(|profile| profile.id.clone());

        self.invoker_active_combo_profile_id.clone()
    }
}
```

- [ ] **Step 4: Run the targeted runtime-state tests**

Run:

```powershell
cargo test app_state_repairs_invalid_invoker_active_combo --lib
```

Expected: PASS.

- [ ] **Step 5: Commit the runtime-state slice**

Run:

```powershell
git add src/state/app_state.rs
git commit -m "feat: add invoker active combo runtime state"
```

---

### Task 2: Add active-combo keyboard behavior

**Files:**
- Modify: `src/input/keyboard.rs:21-120,548-755,900-1040`
- Modify: `src/main.rs:107-220`
- Test: `src/input/keyboard.rs:1080-1200`

- [ ] **Step 1: Write the failing keyboard tests**

Add these tests to `src/input/keyboard.rs` near the existing Invoker hotkey tests:

```rust
    #[test]
    fn plan_global_hotkey_event_cycles_enabled_invoker_combos() {
        let mut snapshot = KeyboardSnapshot::default();
        snapshot.selected_hero = Some(crate::state::HeroType::Invoker);
        snapshot.invoker_cycle_hotkey = Some(parse_key_string("Delete").unwrap());
        snapshot.invoker_profiles = vec![
            InvokerHotkeyProfileSnapshot {
                id: "qw-pickoff".to_string(),
                hotkey: Some(parse_key_string("Home").unwrap()),
                enabled: true,
                mode: InvokerProfileModeSnapshot::Combo,
            },
            InvokerHotkeyProfileSnapshot {
                id: "meteor-blast-prep".to_string(),
                hotkey: Some(parse_key_string("End").unwrap()),
                enabled: true,
                mode: InvokerProfileModeSnapshot::Prep,
            },
            InvokerHotkeyProfileSnapshot {
                id: "qe-burst".to_string(),
                hotkey: Some(parse_key_string("PageDown").unwrap()),
                enabled: true,
                mode: InvokerProfileModeSnapshot::Combo,
            },
        ];

        assert_eq!(
            plan_global_hotkey_event(Key::Delete, &snapshot),
            Some(HotkeyEvent::InvokerCycleComboProfile)
        );
    }

    #[test]
    fn plan_global_hotkey_event_ignores_invoker_cycle_when_not_invoker() {
        let mut snapshot = KeyboardSnapshot::default();
        snapshot.selected_hero = Some(crate::state::HeroType::Tiny);
        snapshot.invoker_cycle_hotkey = Some(parse_key_string("Delete").unwrap());

        assert_eq!(plan_global_hotkey_event(Key::Delete, &snapshot), None);
    }
```

- [ ] **Step 2: Run the keyboard tests to verify they fail**

Run:

```powershell
cargo test plan_global_hotkey_event_cycles_enabled_invoker_combos --lib
```

Expected: FAIL because `InvokerCycleComboProfile`, `invoker_cycle_hotkey`, and the mode-aware profile snapshot do not exist yet.

- [ ] **Step 3: Implement minimal keyboard snapshot and hotkey planning**

Update `src/input/keyboard.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvokerProfileModeSnapshot {
    Combo,
    Prep,
}

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
    InvokerCycleComboProfile,
}

#[derive(Debug, Clone)]
pub struct InvokerHotkeyProfileSnapshot {
    pub id: String,
    pub hotkey: Option<Key>,
    pub enabled: bool,
    pub mode: InvokerProfileModeSnapshot,
}

#[derive(Debug, Clone)]
pub struct KeyboardSnapshot {
    pub selected_hero: Option<crate::state::HeroType>,
    pub trigger_key: Option<Key>,
    pub meepo_farm_toggle_key: Option<Key>,
    pub armlet_roshan_toggle_key: Option<Key>,
    pub sf_enabled: bool,
    pub od_enabled: bool,
    pub shadow_fiend: ShadowFiendKeyboardSnapshot,
    pub outworld_destroyer: OutworldDestroyerKeyboardSnapshot,
    pub broodmother: BroodmotherKeyboardSnapshot,
    pub soul_ring: SoulRingKeyboardConfig,
    pub invoker_profiles: Vec<InvokerHotkeyProfileSnapshot>,
    pub invoker_cycle_hotkey: Option<Key>,
}
```

Update `KeyboardSnapshot::from_runtime(...)` to populate the new mode field and parse a new Invoker cycle hotkey:

```rust
            invoker_profiles: settings
                .heroes
                .invoker
                .profiles
                .iter()
                .map(|profile| InvokerHotkeyProfileSnapshot {
                    id: profile.id.clone(),
                    hotkey: parse_key(&profile.hotkey),
                    enabled: profile.enabled,
                    mode: match profile.mode {
                        crate::config::settings::InvokerProfileMode::Combo => {
                            InvokerProfileModeSnapshot::Combo
                        }
                        crate::config::settings::InvokerProfileMode::Prep => {
                            InvokerProfileModeSnapshot::Prep
                        }
                    },
                })
                .collect(),
            invoker_cycle_hotkey: parse_key_string("Delete"),
```

Also update `impl Default for KeyboardSnapshot` with:

```rust
            invoker_cycle_hotkey: None,
```

Add the planning branch near the Invoker profile lookup in `plan_global_hotkey_event(...)`:

```rust
    if snapshot.selected_hero == Some(crate::state::HeroType::Invoker)
        && snapshot.invoker_cycle_hotkey == Some(key)
    {
        let has_enabled_combo = snapshot
            .invoker_profiles
            .iter()
            .any(|profile| profile.enabled && profile.mode == InvokerProfileModeSnapshot::Combo);

        if has_enabled_combo {
            return Some(HotkeyEvent::InvokerCycleComboProfile);
        }
    }
```

- [ ] **Step 4: Wire the new hotkey event in `src/main.rs`**

Before the hotkey thread spawn, capture settings too:

```rust
    let settings_for_hotkeys = settings.clone();
```

Then, inside the thread, add a new match arm in the hotkey loop:

```rust
                input::keyboard::HotkeyEvent::InvokerCycleComboProfile => {
                    let mut state = app_state_clone2.lock().unwrap();
                    if state.standalone_enabled
                        && state.selected_hero == Some(state::HeroType::Invoker)
                    {
                        let settings_guard = settings_for_hotkeys.lock().unwrap();
                        let profiles = settings_guard
                            .heroes
                            .invoker
                            .profiles
                            .iter()
                            .filter(|profile| {
                                profile.enabled
                                    && profile.mode
                                        == crate::config::settings::InvokerProfileMode::Combo
                            })
                            .map(|profile| profile.id.clone())
                            .collect::<Vec<_>>();
                        drop(settings_guard);

                        if profiles.is_empty() {
                            info!("No enabled Invoker combo profiles available");
                        } else {
                            let next_index = state
                                .invoker_active_combo_profile_id
                                .as_ref()
                                .and_then(|id| profiles.iter().position(|profile_id| profile_id == id))
                                .map(|index| (index + 1) % profiles.len())
                                .unwrap_or(0);
                            let next_id = profiles[next_index].clone();
                            state.invoker_active_combo_profile_id = Some(next_id.clone());
                            info!("Invoker active combo changed to {}", next_id);
                            crate::actions::activity::push_activity(
                                crate::actions::activity::ActivityCategory::System,
                                format!("Invoker active combo changed to {}", next_id),
                            );
                        }
                    }
                }
```

- [ ] **Step 5: Run the targeted keyboard tests**

Run:

```powershell
cargo test plan_global_hotkey_event_cycles_enabled_invoker_combos --lib
```

Expected: PASS.

- [ ] **Step 6: Commit the keyboard slice**

Run:

```powershell
git add src/input/keyboard.rs src/main.rs
git commit -m "feat: add invoker combo cycle hotkey"
```

---

### Task 3: Make execution resolve through active combo state

**Files:**
- Modify: `src/actions/heroes/invoker.rs:220-380,760-1100`
- Modify: `src/main.rs:107-220`
- Test: `src/actions/heroes/invoker.rs:900-1100`

- [ ] **Step 1: Write the failing execution tests**

Add these tests to `src/actions/heroes/invoker.rs`:

```rust
    #[test]
    fn resolve_active_combo_profile_prefers_explicit_active_combo() {
        let settings = Settings::default();
        let config = &settings.heroes.invoker;

        let active = resolve_active_combo_profile_id(config, Some("qe-burst"));
        assert_eq!(active.as_deref(), Some("qe-burst"));
    }

    #[test]
    fn resolve_active_combo_profile_repairs_missing_active_combo() {
        let settings = Settings::default();
        let config = &settings.heroes.invoker;

        let active = resolve_active_combo_profile_id(config, Some("missing-profile"));
        assert_eq!(active.as_deref(), Some("qw-pickoff"));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```powershell
cargo test resolve_active_combo_profile_prefers_explicit_active_combo --lib
```

Expected: FAIL because `resolve_active_combo_profile_id(...)` does not exist yet.

- [ ] **Step 3: Add minimal active-profile resolution**

In `src/actions/heroes/invoker.rs`, replace `first_enabled_combo_profile_id(...)` with:

```rust
fn resolve_active_combo_profile_id(
    config: &crate::config::settings::InvokerConfig,
    active_profile_id: Option<&str>,
) -> Option<String> {
    let active = active_profile_id.and_then(|id| {
        config.profiles.iter().find(|profile| {
            profile.id == id
                && profile.enabled
                && profile.mode == InvokerProfileMode::Combo
        })
    });

    if let Some(profile) = active {
        return Some(profile.id.clone());
    }

    config
        .profiles
        .iter()
        .find(|profile| profile.enabled && profile.mode == InvokerProfileMode::Combo)
        .map(|profile| profile.id.clone())
}
```

Make the helper `pub(crate)` so `src/main.rs` can use it.

Update the `HotkeyEvent::ComboTrigger` arm in `src/main.rs` so Invoker uses the active combo directly instead of calling `dispatch_standalone_trigger(...)`:

```rust
                input::keyboard::HotkeyEvent::ComboTrigger => {
                    let state = app_state_clone2.lock().unwrap();
                    if state.standalone_enabled {
                        if let Some(hero_type) = state.selected_hero {
                            if hero_type == state::HeroType::Invoker {
                                let active_profile_id =
                                    state.invoker_active_combo_profile_id.clone();
                                drop(state);

                                let settings_guard = settings_for_hotkeys.lock().unwrap();
                                let profile_id =
                                    crate::actions::heroes::invoker::resolve_active_combo_profile_id(
                                        &settings_guard.heroes.invoker,
                                        active_profile_id.as_deref(),
                                    );
                                drop(settings_guard);

                                if let Some(profile_id) = profile_id {
                                    dispatcher_clone2.dispatch_invoker_profile(&profile_id);
                                } else {
                                    info!("Invoker combo trigger skipped: no enabled combo profile");
                                }
                            } else {
                                let hero_name = match hero_type {
                                    state::HeroType::Huskar => models::Hero::Huskar.to_game_name(),
                                    state::HeroType::Largo => models::Hero::Largo.to_game_name(),
                                    state::HeroType::LegionCommander => {
                                        models::Hero::LegionCommander.to_game_name()
                                    }
                                    state::HeroType::Meepo => models::Hero::Meepo.to_game_name(),
                                    state::HeroType::OutworldDestroyer => {
                                        models::Hero::ObsidianDestroyer.to_game_name()
                                    }
                                    state::HeroType::ShadowFiend => {
                                        models::Hero::Nevermore.to_game_name()
                                    }
                                    state::HeroType::Tiny => models::Hero::Tiny.to_game_name(),
                                    state::HeroType::Invoker => unreachable!(),
                                };
                                drop(state);
                                dispatcher_clone2.dispatch_standalone_trigger(hero_name);
                            }
                        }
                    }
                }
```

- [ ] **Step 4: Make direct combo hotkeys set active combo before execution**

In `src/main.rs`, update the existing `HotkeyEvent::InvokerProfile(profile_id)` arm so combo profiles set active combo before running:

```rust
                input::keyboard::HotkeyEvent::InvokerProfile(profile_id) => {
                    let mut state = app_state_clone2.lock().unwrap();
                    let settings_guard = settings_for_hotkeys.lock().unwrap();
                    let profile = settings_guard
                        .heroes
                        .invoker
                        .profiles
                        .iter()
                        .find(|candidate| candidate.id == profile_id)
                        .cloned();
                    drop(settings_guard);

                    if let Some(profile) = profile {
                        if profile.enabled
                            && profile.mode == crate::config::settings::InvokerProfileMode::Combo
                        {
                            state.invoker_active_combo_profile_id = Some(profile.id.clone());
                            info!("Invoker active combo changed to {}", profile.id);
                            crate::actions::activity::push_activity(
                                crate::actions::activity::ActivityCategory::System,
                                format!("Invoker active combo changed to {}", profile.id),
                            );
                        }
                    }
                    drop(state);

                    dispatcher_clone2.dispatch_invoker_profile(&profile_id);
                }
```

- [ ] **Step 5: Run the targeted execution tests**

Run:

```powershell
cargo test resolve_active_combo_profile_prefers_explicit_active_combo --lib
cargo test resolve_active_combo_profile_repairs_missing_active_combo --lib
```

Expected: PASS.

- [ ] **Step 6: Commit the execution slice**

Run:

```powershell
git add src/actions/heroes/invoker.rs src/main.rs
git commit -m "feat: route invoker combo trigger through active profile"
```

---

### Task 4: Expose active combo state to Tauri and the frontend store

**Files:**
- Modify: `src-tauri/src/ipc_types.rs:22-40`
- Modify: `src-tauri/src/events.rs:79-89`
- Modify: `src-tauri/src/commands/state.rs:1-123`
- Modify: `src-tauri/src/lib.rs:30-60`
- Modify: `src-ui/src/stores/uiStore.ts:1-96`
- Modify: `src-ui/src/stores/uiStore.test.ts:1-89`

- [ ] **Step 1: Write the failing UI store test**

In the existing `beforeEach`, add `invokerActiveComboProfileId: null,` to the `useUIStore.setState(...)` reset object. Then add this test to `src-ui/src/stores/uiStore.test.ts`:

```ts
  it("loads and listens to the active Invoker combo profile from app state", async () => {
    invokeMock.mockResolvedValueOnce({
      selectedHero: "Invoker",
      gsiEnabled: true,
      standaloneEnabled: true,
      appVersion: "0.15.0",
      armletRoshanArmed: false,
      invokerActiveComboProfileId: "qe-burst",
    });

    await useUIStore.getState().loadInitialState();
    expect(useUIStore.getState().invokerActiveComboProfileId).toBe("qe-burst");

    const unlisten = await useUIStore.getState().startListening();
    emitEvent("app_state_update", {
      selectedHero: "Invoker",
      gsiEnabled: true,
      standaloneEnabled: true,
      appVersion: "0.15.0",
      armletRoshanArmed: false,
      invokerActiveComboProfileId: "qw-pickoff",
    });

    expect(useUIStore.getState().invokerActiveComboProfileId).toBe("qw-pickoff");
    unlisten();
  });
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```powershell
npm --prefix src-ui test -- uiStore.test.ts
```

Expected: FAIL because the store and DTO do not expose `invokerActiveComboProfileId`.

- [ ] **Step 3: Extend the backend DTO and commands**

Update `src-tauri/src/ipc_types.rs`:

```rust
pub struct AppStateDto {
    pub selected_hero: Option<String>,
    pub gsi_enabled: bool,
    pub standalone_enabled: bool,
    pub armlet_roshan_armed: bool,
    pub app_version: String,
    pub invoker_active_combo_profile_id: Option<String>,
}
```

Update both `src-tauri/src/events.rs` and `src-tauri/src/commands/state.rs` so `AppStateDto` includes:

```rust
        invoker_active_combo_profile_id: state.invoker_active_combo_profile_id.clone(),
```

Add a new Tauri command in `src-tauri/src/commands/state.rs`:

```rust
#[tauri::command]
pub fn set_invoker_active_combo_profile(
    profile_id: Option<String>,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let mut app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    app.invoker_active_combo_profile_id = profile_id;
    if let Some(active_id) = app.invoker_active_combo_profile_id.clone() {
        dota2_scripts::actions::activity::push_activity(
            dota2_scripts::actions::activity::ActivityCategory::System,
            format!("Invoker active combo changed to {}", active_id),
        );
    }
    refresh_keyboard_snapshot(&state, &app)?;
    Ok(())
}
```

Register it in `src-tauri/src/lib.rs`:

```rust
            commands::state::set_invoker_active_combo_profile,
```

- [ ] **Step 4: Extend the frontend UI store**

Update `src-ui/src/stores/uiStore.ts`:

```ts
interface UIStore {
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  gsiEnabled: boolean;
  standaloneEnabled: boolean;
  appVersion: string;
  armletRoshanArmed: boolean;
  invokerActiveComboProfileId: string | null;
  setGsiEnabled: (enabled: boolean) => void;
  setStandaloneEnabled: (enabled: boolean) => void;
  setArmletRoshanArmed: (armed: boolean) => void;
  setInvokerActiveComboProfileId: (profileId: string | null) => void;
  loadInitialState: () => Promise<void>;
  startListening: () => Promise<() => void>;
}
```

Add the setter and load/listen wiring:

```ts
  invokerActiveComboProfileId: null,

  setInvokerActiveComboProfileId: (profileId) => {
    set({ invokerActiveComboProfileId: profileId });
    if (isTauri()) {
      import("@tauri-apps/api/core").then(({ invoke }) => {
        invoke("set_invoker_active_combo_profile", { profileId }).catch(console.error);
      });
    }
  },
```

And in both `loadInitialState()` and `startListening()`:

```ts
        invokerActiveComboProfileId: state.invokerActiveComboProfileId ?? null,
```

- [ ] **Step 5: Run the targeted store test**

Run:

```powershell
npm --prefix src-ui test -- uiStore.test.ts
```

Expected: PASS.

- [ ] **Step 6: Commit the Tauri/store slice**

Run:

```powershell
git add src-tauri/src/ipc_types.rs src-tauri/src/events.rs src-tauri/src/commands/state.rs src-tauri/src/lib.rs src-ui/src/stores/uiStore.ts src-ui/src/stores/uiStore.test.ts
git commit -m "feat(ui): expose invoker active combo app state"
```

---

### Task 5: Update the Invoker UI to mark and set the active combo

**Files:**
- Modify: `src-ui/src/components/heroes/configs/InvokerConfig.tsx:60-212`
- Modify: `src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx:8-125`
- Modify: `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx:1-260`

- [ ] **Step 1: Write the failing UI behavior test**

Add `import { useUIStore } from "../../../stores/uiStore";` near the other imports. In the existing `beforeEach`, add:

```ts
    useUIStore.setState({
      invokerActiveComboProfileId: null,
    });
```

Then add these tests to `src-ui/src/components/heroes/configs/InvokerConfig.test.tsx`:

```ts
  it("marks a clicked combo profile as active", () => {
    render(<InvokerConfig />);

    fireEvent.click(screen.getByRole("button", { name: /QE Burst/i }));

    expect(screen.getByText("Active combo: QE Burst")).toBeInTheDocument();
    expect(useUIStore.getState().invokerActiveComboProfileId).toBe("qe-burst");
  });

  it("does not mark prep profiles as the active combo", () => {
    useUIStore.setState({ invokerActiveComboProfileId: "qw-pickoff" });
    render(<InvokerConfig />);

    fireEvent.click(
      screen.getByRole("button", { name: /Meteor \\+ Blast Prep/i }),
    );

    expect(useUIStore.getState().invokerActiveComboProfileId).toBe("qw-pickoff");
  });

  it("does not activate disabled combo profiles", () => {
    useUIStore.setState({ invokerActiveComboProfileId: "qw-pickoff" });
    useConfigStore.setState({
      config: {
        ...mockConfig,
        heroes: {
          ...mockConfig.heroes,
          invoker: {
            ...mockConfig.heroes.invoker,
            profiles: mockConfig.heroes.invoker.profiles.map((profile) =>
              profile.id === "qe-burst" ? { ...profile, enabled: false } : profile,
            ),
          },
        },
      },
      loaded: true,
    });
    render(<InvokerConfig />);

    fireEvent.click(screen.getByRole("button", { name: /QE Burst/i }));

    expect(useUIStore.getState().invokerActiveComboProfileId).toBe("qw-pickoff");
  });
```

- [ ] **Step 2: Run the UI test to verify it fails**

Run:

```powershell
npm --prefix src-ui test -- InvokerConfig.test.tsx
```

Expected: FAIL because `InvokerConfig` only updates `selectedId`, not runtime active combo state.

- [ ] **Step 3: Update `InvokerConfig.tsx` to drive active combo selection**

Wire the UI store into `InvokerConfig.tsx`:

```tsx
  const activeComboId = useUIStore((state) => state.invokerActiveComboProfileId);
  const setActiveComboId = useUIStore((state) => state.setInvokerActiveComboProfileId);
```

Add a helper:

```tsx
  const selectProfile = (profile: InvokerProfile) => {
    setSelectedId(profile.id);
    if (profile.mode === "combo" && profile.enabled) {
      setActiveComboId(profile.id);
    }
  };
```

Render a summary near the profile card:

```tsx
          <div className="mb-3 rounded-lg border border-border bg-elevated px-3 py-2 text-sm text-content">
            Active combo:{" "}
            <span className="font-semibold">
              {config.profiles.find((profile) => profile.id === activeComboId)?.name ?? "None"}
            </span>
          </div>
```

And change the list callback:

```tsx
            onSelect={(id) => {
              const profile = config.profiles.find((candidate) => candidate.id === id);
              if (profile) {
                selectProfile(profile);
              }
            }}
```

- [ ] **Step 4: Update `InvokerProfileList.tsx` to show the active marker**

Extend props:

```tsx
interface InvokerProfileListProps {
  profiles: InvokerProfile[];
  selectedId: string | null;
  activeComboId: string | null;
  onSelect: (id: string) => void;
  onDuplicate: (id: string) => void;
  onDelete: (id: string) => void;
  onAddPreset: (presetId: string) => void;
}
```

Render the marker:

```tsx
                    <div className="flex items-center gap-2">
                      {profile.mode === "combo" && activeComboId === profile.id && (
                        <span className="rounded bg-success/15 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-success">
                          Active
                        </span>
                      )}
                      <div className="text-xs uppercase tracking-wide text-subtle">
                        {profile.build_tag || "general"}
                      </div>
                    </div>
```

- [ ] **Step 5: Run the targeted UI tests**

Run:

```powershell
npm --prefix src-ui test -- InvokerConfig.test.tsx
```

Expected: PASS.

- [ ] **Step 6: Commit the UI slice**

Run:

```powershell
git add src-ui/src/components/heroes/configs/InvokerConfig.tsx src-ui/src/components/heroes/configs/invoker/InvokerProfileList.tsx src-ui/src/components/heroes/configs/InvokerConfig.test.tsx
git commit -m "feat(ui): mark active invoker combo profile"
```

---

### Task 6: Add docs and full verification

**Files:**
- Modify: `docs/heroes/invoker.md`
- Modify: `docs/reference/configuration.md`

- [ ] **Step 1: Update the Invoker hero doc**

Add a section like this to `docs/heroes/invoker.md`:

```md
### Active combo selection

Invoker now keeps one active combo profile at a time.

- Clicking an enabled combo profile in the UI marks it active and opens it in the editor.
- Prep profiles remain directly runnable, but they do not become the active combo.
- The generic combo trigger uses the active combo profile instead of the first enabled combo in the list.
- Per-profile combo hotkeys still execute directly and also update the active combo.
- The global Invoker cycle hotkey only rotates through enabled combo profiles.
```

Add one sentence in the logging/feedback area:

```md
When the active combo changes, the app records an activity event such as
`Invoker active combo changed to QE Burst`.
```

- [ ] **Step 2: Update the configuration reference**

In `docs/reference/configuration.md`, add this note under the Invoker section:

```md
Invoker profiles are still defined entirely in config, but the currently active
combo profile is runtime state. UI clicks, direct combo hotkeys, and the cycle
hotkey can all change it while the app is running.
```

- [ ] **Step 3: Run full verification**

Run:

```powershell
cargo test
npm --prefix src-ui test
cargo build --release
```

Expected: PASS.

- [ ] **Step 4: Commit docs and verification sweep**

Run:

```powershell
git add docs/heroes/invoker.md docs/reference/configuration.md
git commit -m "docs: explain invoker active combo selection"
```

- [ ] **Step 5: Confirm clean status**

Run:

```powershell
git status --short
```

Expected: empty output before requesting merge/push handling.
