pub mod commands;
pub mod events;
pub mod ipc_types;

use dota2_scripts::actions::executor::{ActionExecutor, ExecutorMetrics};
use dota2_scripts::actions::heroes::{LargoScript, MeepoScript};
use dota2_scripts::actions::activity::{push_activity, ActivityCategory};
use dota2_scripts::actions::ActionDispatcher;
use dota2_scripts::config::settings::InvokerProfileMode;
use dota2_scripts::config::Settings;
use dota2_scripts::gsi::start_gsi_server;
use dota2_scripts::input::keyboard::{
    start_keyboard_listener, HotkeyEvent, KeyboardListenerConfig, KeyboardSnapshot,
};
use dota2_scripts::models::Hero;
use dota2_scripts::state::{AppState, HeroType, UpdateCheckState};
use dota2_scripts::update::{check_for_update, UpdateCheckResult};
use std::sync::{Arc, Mutex, RwLock};
use tracing::info;

/// Shared state managed by Tauri, accessible from all commands
pub struct TauriAppState {
    pub app_state: Arc<Mutex<AppState>>,
    pub settings: Arc<Mutex<Settings>>,
    pub keyboard_snapshot: Arc<RwLock<KeyboardSnapshot>>,
    pub executor_metrics: Arc<ExecutorMetrics>,
}

pub fn run() {
    // Load settings
    let settings = Arc::new(Mutex::new(Settings::load()));

    // Initialize logging with config level or environment variable.
    //
    // The handle owns the file writer and is held for the whole of `run()`:
    // dropping it stops the writer thread and truncates the log mid-session.
    // This matters more here than in the headless binary — the desktop build is
    // a GUI process, so the console half goes nowhere and the file is the only
    // record there is.
    let (log_level, log_to_file) = {
        let settings = settings.lock().unwrap();
        (
            std::env::var("RUST_LOG").unwrap_or_else(|_| settings.logging.level.clone()),
            settings.logging.file_enabled,
        )
    };
    let _logging = dota2_scripts::logging::init(&log_level, log_to_file);

    info!("Starting Dota 2 Script Automation (Tauri)...");
    if let Some(log_dir) = _logging.log_dir() {
        info!("Writing logs to {}", log_dir.display());
    }
    info!("Server port: {}", settings.lock().unwrap().server.port);

    // Initialize shared state
    let app_state = AppState::new();

    // Build the initial keyboard snapshot before starting the listener
    let initial_snapshot = {
        let settings_guard = settings.lock().unwrap();
        let state_guard = app_state.lock().unwrap();
        Arc::new(RwLock::new(KeyboardSnapshot::from_runtime(
            &settings_guard,
            &state_guard,
        )))
    };

    // Initialize action executor and dispatcher
    let action_executor = ActionExecutor::new();
    let executor_metrics = action_executor.metrics();
    let dispatcher = Arc::new(ActionDispatcher::new(settings.clone(), action_executor, app_state.clone()));

    // Start keyboard listener with snapshot-based config
    let keyboard_config = KeyboardListenerConfig {
        snapshot: initial_snapshot.clone(),
    };
    let hotkey_rx = start_keyboard_listener(keyboard_config);

    // Start GSI server in background
    let port = settings.lock().unwrap().server.port;
    let gsi_app_state = app_state.clone();
    let gsi_dispatcher = dispatcher.clone();
    let gsi_settings = settings.clone();
    let gsi_keyboard_snapshot = initial_snapshot.clone();
    tauri::async_runtime::spawn(async move {
        start_gsi_server(
            port,
            gsi_app_state,
            gsi_dispatcher,
            gsi_settings,
            Some(gsi_keyboard_snapshot),
        )
        .await;
    });

    // Start update check in background (if enabled)
    {
        let settings_guard = settings.lock().unwrap();
        let check_on_startup = settings_guard.updates.check_on_startup;
        let include_prereleases = settings_guard.updates.include_prereleases;
        drop(settings_guard);

        if check_on_startup {
            let update_state = app_state.lock().unwrap().update_state.clone();
            *update_state.lock().unwrap() = UpdateCheckState::Checking;

            std::thread::spawn(move || match check_for_update(include_prereleases) {
                UpdateCheckResult::Available(update_info) => {
                    *update_state.lock().unwrap() = UpdateCheckState::Available {
                        version: update_info.version,
                        release_notes: update_info.release_notes,
                    };
                }
                UpdateCheckResult::UpToDate => {
                    *update_state.lock().unwrap() = UpdateCheckState::UpToDate;
                }
                UpdateCheckResult::Error(msg) => {
                    *update_state.lock().unwrap() = UpdateCheckState::Error(msg);
                }
            });
        }
    }

    // Start minimap capture worker in background
    let minimap_settings = settings.clone();
    let minimap_state = app_state.clone();
    std::thread::spawn(move || {
        dota2_scripts::observability::minimap_capture::start_minimap_capture_worker(
            minimap_settings,
            minimap_state,
        );
    });

    // Draft reader: identifies drafted heroes from the screen while GSI
    // reports hero selection. Idles at ~2Hz config polls unless [draft]
    // enabled = true.
    let draft_settings = settings.clone();
    let draft_state = app_state.clone();
    std::thread::spawn(move || {
        dota2_scripts::observability::draft_reader::start_draft_reader_worker(
            draft_settings,
            draft_state,
        );
    });

    // STRATZ dataset: keeps the cached matchup data fresh for draft advice.
    // Idles at 1Hz config polls unless [stratz] enabled = true, and a draft
    // never waits on it — advice is additive to identification.
    let stratz_settings = settings.clone();
    let stratz_state = app_state.clone();
    std::thread::spawn(move || {
        dota2_scripts::stratz::start_stratz_worker(stratz_settings, stratz_state);
    });

    // Start hotkey event handler in background
    let hotkey_app_state = app_state.clone();
    let hotkey_dispatcher = dispatcher.clone();
    let hotkey_settings = settings.clone();
    std::thread::spawn(move || {
        handle_hotkey_events(
            hotkey_rx,
            hotkey_app_state,
            hotkey_dispatcher,
            hotkey_settings,
        );
    });

    // Build and run Tauri application
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(TauriAppState {
            app_state: app_state.clone(),
            settings: settings.clone(),
            keyboard_snapshot: initial_snapshot.clone(),
            executor_metrics,
        })
        .setup(|app| {
            use tauri::Manager;

            let handle = app.handle().clone();
            commands::overlay::register_app_handle(handle.clone());
            commands::hud::register_app_handle(handle.clone());
            events::start_game_state_emitter(handle.clone());

            // Closing the main window must also close the overlay. Tauri exits
            // when its last window closes, and the overlay is transparent,
            // click-through, and hidden from the taskbar — so on its own it
            // would keep the process alive as a ghost with no way to close it
            // except Task Manager.
            if let Some(main_window) = app.get_webview_window("main") {
                main_window.on_window_event(move |event| {
                    if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                        if let Some(overlay) =
                            handle.get_webview_window(commands::overlay::OVERLAY_LABEL)
                        {
                            let _ = overlay.close();
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config,
            commands::config::update_config,
            commands::config::update_hero_config,
            commands::hud::capture_hud_portrait,
            commands::hud::test_hud_portrait,
            commands::state::get_app_state,
            commands::state::set_gsi_enabled,
            commands::state::set_standalone_enabled,
            commands::state::set_armlet_roshan_mode_armed,
            commands::state::set_invoker_active_combo_profile,
            commands::state::select_hero,
            commands::game::get_game_state,
            commands::diagnostics::get_diagnostics,
            commands::updates::get_update_state,
            commands::updates::check_for_updates,
            commands::updates::apply_update,
            commands::updates::dismiss_update,
            commands::meepo::get_meepo_state,
            commands::minimap::get_minimap_status,
            commands::draft::get_draft_status,
            commands::draft::submit_draft_feedback,
            commands::stratz::get_stratz_status,
            commands::stratz::set_stratz_token,
            commands::stratz::clear_stratz_token,
            commands::stratz::get_draft_advice,
            commands::stratz::refresh_stratz_dataset,
            commands::waves::get_wave_lane_paths,
            commands::waves::get_wave_snapshot,
            commands::overlay::show_wave_overlay,
            commands::overlay::hide_wave_overlay,
            commands::overlay::toggle_wave_overlay,
            commands::overlay::get_wave_overlay_status,
            commands::alerts::get_alert_countdowns,
            commands::alerts::test_play_alert,
            commands::alerts::list_voice_packs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn resolve_tauri_invoker_combo_trigger_profile_id(
    app_state: &AppState,
    settings: &Settings,
) -> Option<String> {
    let profile_id = app_state
        .invoker_active_combo_profile_id
        .as_deref()
        .and_then(|active_profile_id| {
            settings.heroes.invoker.profiles.iter().find(|profile| {
                profile.enabled
                    && profile.mode == InvokerProfileMode::Combo
                    && profile.id == active_profile_id
            })
        })
        .map(|profile| profile.id.clone())
        .or_else(|| {
            settings
                .heroes
                .invoker
                .profiles
                .iter()
                .find(|profile| profile.enabled && profile.mode == InvokerProfileMode::Combo)
                .map(|profile| profile.id.clone())
        });
    profile_id
}

fn sync_tauri_invoker_profile_hotkey(
    app_state: &mut AppState,
    settings: &Settings,
    profile_id: &str,
) -> Option<String> {
    let profile = settings
        .heroes
        .invoker
        .profiles
        .iter()
        .find(|profile| profile.id == profile_id)?;

    if profile.enabled && profile.mode == InvokerProfileMode::Combo {
        app_state.invoker_active_combo_profile_id = Some(profile.id.clone());
        Some(format!(
            "Invoker active combo profile set to {}",
            profile.id
        ))
    } else {
        None
    }
}

/// Processes hotkey events from the keyboard listener and dispatches actions.
fn handle_hotkey_events(
    hotkey_rx: std::sync::mpsc::Receiver<HotkeyEvent>,
    app_state: Arc<Mutex<AppState>>,
    dispatcher: Arc<ActionDispatcher>,
    settings: Arc<Mutex<Settings>>,
) {
    while let Ok(event) = hotkey_rx.recv() {
        match event {
            HotkeyEvent::ComboTrigger => {
                let (standalone_enabled, selected_hero) = {
                    let state = app_state.lock().unwrap();
                    (state.standalone_enabled, state.selected_hero)
                };

                if !standalone_enabled {
                    info!("Standalone scripts disabled");
                    continue;
                }

                match selected_hero {
                    Some(HeroType::Invoker) => {
                        let settings_snapshot = settings.lock().unwrap().clone();
                        let profile_id = {
                            let state = app_state.lock().unwrap();
                            resolve_tauri_invoker_combo_trigger_profile_id(&state, &settings_snapshot)
                        };

                        if let Some(profile_id) = profile_id {
                            info!(
                                "Triggering standalone combo for {} with profile {}",
                                Hero::Invoker.to_game_name(),
                                profile_id
                            );
                            dispatcher.dispatch_invoker_profile(&profile_id);
                        } else {
                            info!("Invoker standalone combo skipped: no enabled combo profile");
                        }
                    }
                    Some(hero_type) => {
                        let hero_name = match hero_type {
                            HeroType::EarthSpirit => Hero::EarthSpirit.to_game_name(),
                            HeroType::EmberSpirit => Hero::EmberSpirit.to_game_name(),
                            HeroType::Huskar => Hero::Huskar.to_game_name(),
                            HeroType::Largo => Hero::Largo.to_game_name(),
                            HeroType::LegionCommander => Hero::LegionCommander.to_game_name(),
                            HeroType::Magnus => Hero::Magnataur.to_game_name(),
                            HeroType::Meepo => Hero::Meepo.to_game_name(),
                            HeroType::Mirana => Hero::Mirana.to_game_name(),
                            HeroType::OutworldDestroyer => {
                                Hero::ObsidianDestroyer.to_game_name()
                            }
                            HeroType::ShadowFiend => Hero::Nevermore.to_game_name(),
                            HeroType::Slark => Hero::Slark.to_game_name(),
                            HeroType::Snapfire => Hero::Snapfire.to_game_name(),
                            HeroType::Tiny => Hero::Tiny.to_game_name(),
                            HeroType::Invoker => unreachable!(),
                        };
                        info!("Triggering standalone combo for {}", hero_name);
                        dispatcher.dispatch_standalone_trigger(hero_name);
                    }
                    None => {
                        info!("No hero selected for standalone combo");
                    }
                }
            }
            HotkeyEvent::MeepoFarmToggle => {
                let state = app_state.lock().unwrap();
                if state.standalone_enabled
                    && state.selected_hero == Some(HeroType::Meepo)
                {
                    drop(state);
                    if let Some(script) =
                        dispatcher.hero_scripts.get(Hero::Meepo.to_game_name())
                    {
                        if let Some(meepo_script) =
                            script.as_any().downcast_ref::<MeepoScript>()
                        {
                            meepo_script.toggle_farm_assist();
                        }
                    }
                }
            }
            HotkeyEvent::ArmletRoshanToggle => {
                let armed = dota2_scripts::actions::armlet::toggle_roshan_mode();
                info!(
                    "Armlet Roshan mode {} via hotkey",
                    if armed { "armed" } else { "disarmed" }
                );
            }
            HotkeyEvent::WaveOverlayToggle => {
                commands::overlay::toggle_from_hotkey();
            }
            HotkeyEvent::CaptureHudPortrait => {
                commands::hud::capture_from_hotkey(&settings);
            }
            HotkeyEvent::InvokerCycleComboProfile => {
                let settings = settings.lock().unwrap();
                let enabled_combo_profile_ids: Vec<String> = settings
                    .heroes
                    .invoker
                    .profiles
                    .iter()
                    .filter(|profile| {
                        profile.enabled
                            && profile.mode
                                == dota2_scripts::config::settings::InvokerProfileMode::Combo
                    })
                    .map(|profile| profile.id.clone())
                    .collect();
                drop(settings);

                let mut state = app_state.lock().unwrap();
                if !state.standalone_enabled || state.selected_hero != Some(HeroType::Invoker) {
                    continue;
                }

                if enabled_combo_profile_ids.is_empty() {
                    info!("Invoker cycle hotkey ignored: no enabled combo profiles");
                    continue;
                }

                let current_index =
                    state
                        .invoker_active_combo_profile_id
                        .as_ref()
                        .and_then(|active_id| {
                            enabled_combo_profile_ids
                                .iter()
                                .position(|profile_id| profile_id == active_id)
                        });
                let next_index = current_index
                    .map(|index| (index + 1) % enabled_combo_profile_ids.len())
                    .unwrap_or(0);
                let next_profile_id = enabled_combo_profile_ids[next_index].clone();
                state.invoker_active_combo_profile_id = Some(next_profile_id.clone());
                drop(state);

                info!("Invoker active combo profile set to {}", next_profile_id);
                push_activity(
                    ActivityCategory::System,
                    format!("Invoker active combo profile set to {}", next_profile_id),
                );
            }
            HotkeyEvent::LargoQ => {
                dispatch_largo_song(&app_state, &dispatcher, |largo| {
                    largo.select_song_manually(
                        dota2_scripts::actions::heroes::largo::Song::Bullbelly,
                    );
                });
            }
            HotkeyEvent::LargoW => {
                dispatch_largo_song(&app_state, &dispatcher, |largo| {
                    largo.select_song_manually(
                        dota2_scripts::actions::heroes::largo::Song::Hotfeet,
                    );
                });
            }
            HotkeyEvent::LargoE => {
                dispatch_largo_song(&app_state, &dispatcher, |largo| {
                    largo.select_song_manually(
                        dota2_scripts::actions::heroes::largo::Song::IslandElixir,
                    );
                });
            }
            HotkeyEvent::LargoR => {
                dispatch_largo_song(&app_state, &dispatcher, |largo| {
                    largo.deactivate_ultimate();
                });
            }
            HotkeyEvent::InvokerProfile(profile_id) => {
                let settings_snapshot = settings.lock().unwrap().clone();
                let activity_message = {
                    let mut state = app_state.lock().unwrap();
                    sync_tauri_invoker_profile_hotkey(
                        &mut state,
                        &settings_snapshot,
                        &profile_id,
                    )
                };

                if let Some(activity_message) = activity_message {
                    info!("{}", activity_message);
                    push_activity(ActivityCategory::System, activity_message);
                }
                dispatcher.dispatch_invoker_profile(&profile_id);
            }
        }
    }
}

/// Helper to dispatch Largo-specific song/ultimate actions when conditions are met.
fn dispatch_largo_song(
    app_state: &Arc<Mutex<AppState>>,
    dispatcher: &Arc<ActionDispatcher>,
    action: impl FnOnce(&LargoScript),
) {
    let state = app_state.lock().unwrap();
    if state.standalone_enabled && state.selected_hero == Some(HeroType::Largo) {
        drop(state);
        if let Some(script) = dispatcher.hero_scripts.get(Hero::Largo.to_game_name()) {
            if let Some(largo_script) = script.as_any().downcast_ref::<LargoScript>() {
                action(largo_script);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_tauri_invoker_combo_trigger_profile_id, sync_tauri_invoker_profile_hotkey,
    };
    use dota2_scripts::config::settings::{
        InvokerProfile, InvokerProfileExecutionStyle, InvokerProfileMode, Settings,
    };
    use dota2_scripts::state::AppState;

    fn profile(id: &str, enabled: bool, mode: InvokerProfileMode) -> InvokerProfile {
        InvokerProfile {
            id: id.to_string(),
            name: id.to_string(),
            enabled,
            hotkey: "F1".to_string(),
            mode,
            execution_style: InvokerProfileExecutionStyle::Automatic,
            build_tag: String::new(),
            steps: Vec::new(),
        }
    }

    fn settings_with_profiles(profiles: Vec<InvokerProfile>) -> Settings {
        let mut settings = Settings::default();
        settings.heroes.invoker.profiles = profiles;
        settings
    }

    #[test]
    fn combo_trigger_prefers_existing_enabled_active_combo() {
        let settings = settings_with_profiles(vec![
            profile("combo-a", true, InvokerProfileMode::Combo),
            profile("combo-b", true, InvokerProfileMode::Combo),
        ]);
        let mut app_state = AppState::default();
        app_state.invoker_active_combo_profile_id = Some("combo-b".to_string());

        let selected_profile_id = resolve_tauri_invoker_combo_trigger_profile_id(&app_state, &settings);

        assert_eq!(selected_profile_id.as_deref(), Some("combo-b"));
        assert_eq!(
            app_state.invoker_active_combo_profile_id.as_deref(),
            Some("combo-b")
        );
    }

    #[test]
    fn combo_trigger_resolution_leaves_active_combo_state_unchanged() {
        let settings = settings_with_profiles(vec![
            profile("prep", true, InvokerProfileMode::Prep),
            profile("combo-a", true, InvokerProfileMode::Combo),
            profile("combo-b", true, InvokerProfileMode::Combo),
        ]);
        let mut app_state = AppState::default();
        app_state.invoker_active_combo_profile_id = Some("missing".to_string());

        let selected_profile_id = resolve_tauri_invoker_combo_trigger_profile_id(&app_state, &settings);

        assert_eq!(selected_profile_id.as_deref(), Some("combo-a"));
        assert_eq!(
            app_state.invoker_active_combo_profile_id.as_deref(),
            Some("missing")
        );
    }

    #[test]
    fn combo_profile_hotkey_updates_active_combo_state_and_returns_activity() {
        let settings = settings_with_profiles(vec![
            profile("combo-a", true, InvokerProfileMode::Combo),
            profile("prep", true, InvokerProfileMode::Prep),
        ]);
        let mut app_state = AppState::default();

        let activity =
            sync_tauri_invoker_profile_hotkey(&mut app_state, &settings, "combo-a");

        assert_eq!(
            app_state.invoker_active_combo_profile_id.as_deref(),
            Some("combo-a")
        );
        assert_eq!(
            activity.as_deref(),
            Some("Invoker active combo profile set to combo-a")
        );
    }

    #[test]
    fn prep_profile_hotkey_leaves_active_combo_state_unchanged() {
        let settings = settings_with_profiles(vec![
            profile("combo-a", true, InvokerProfileMode::Combo),
            profile("prep", true, InvokerProfileMode::Prep),
        ]);
        let mut app_state = AppState::default();
        app_state.invoker_active_combo_profile_id = Some("combo-a".to_string());

        let activity = sync_tauri_invoker_profile_hotkey(&mut app_state, &settings, "prep");

        assert_eq!(
            app_state.invoker_active_combo_profile_id.as_deref(),
            Some("combo-a")
        );
        assert_eq!(activity, None);
    }
}
