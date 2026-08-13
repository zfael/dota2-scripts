#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod audio;
mod config;
mod gsi;
mod input;
mod logging;
mod models;
mod observability;
mod state;

mod update;

use crate::actions::activity::{push_activity, ActivityCategory};
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::invoker::resolve_active_combo_profile_id;
use crate::actions::ActionDispatcher;
use crate::config::settings::InvokerProfileMode;
use crate::config::Settings;
use crate::gsi::start_gsi_server;
use crate::input::keyboard::{start_keyboard_listener, KeyboardSnapshot};
use crate::state::{AppState, UpdateCheckState};

use crate::update::{check_for_update, UpdateCheckResult};
use std::sync::{Arc, Mutex, RwLock};
use tracing::{info, warn};

#[tokio::main]
async fn main() {
    // Load settings first to get log level
    let settings = Arc::new(Mutex::new(Settings::load()));

    // Initialize logging with config level or environment variable.
    // The handle owns the file writer and must outlive everything that logs.
    let (log_level, log_to_file) = {
        let settings = settings.lock().unwrap();
        (
            std::env::var("RUST_LOG").unwrap_or_else(|_| settings.logging.level.clone()),
            settings.logging.file_enabled,
        )
    };
    let _logging = logging::init(&log_level, log_to_file);

    info!("Starting Dota 2 Script Automation...");
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

    // Initialize action dispatcher
    let action_executor = ActionExecutor::new();
    let dispatcher = Arc::new(ActionDispatcher::new(
        settings.clone(),
        action_executor,
        app_state.clone(),
    ));

    // Start keyboard listener with snapshot-based config
    let keyboard_config = input::keyboard::KeyboardListenerConfig {
        snapshot: initial_snapshot.clone(),
    };
    let hotkey_rx = start_keyboard_listener(keyboard_config);

    // Start GSI server in background
    let port = settings.lock().unwrap().server.port;
    let app_state_clone = app_state.clone();
    let dispatcher_clone = dispatcher.clone();
    let settings_clone = settings.clone();
    let gsi_keyboard_snapshot = initial_snapshot.clone();
    tokio::spawn(async move {
        start_gsi_server(
            port,
            app_state_clone,
            dispatcher_clone,
            settings_clone,
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

            tokio::task::spawn_blocking(move || match check_for_update(include_prereleases) {
                UpdateCheckResult::Available(info) => {
                    *update_state.lock().unwrap() = UpdateCheckState::Available {
                        version: info.version,
                        release_notes: info.release_notes,
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

    let minimap_settings = settings.clone();
    let minimap_state = app_state.clone();
    std::thread::spawn(move || {
        crate::observability::minimap_capture::start_minimap_capture_worker(
            minimap_settings,
            minimap_state,
        );
    });

    // Start hotkey event handler in background
    let app_state_clone2 = app_state.clone();
    let dispatcher_clone2 = dispatcher.clone();
    let hotkey_settings = settings.clone();
    std::thread::spawn(move || {
        while let Ok(event) = hotkey_rx.recv() {
            match event {
                input::keyboard::HotkeyEvent::ComboTrigger => {
                    let (standalone_enabled, selected_hero, active_profile_id) = {
                        let state = app_state_clone2.lock().unwrap();
                        (
                            state.standalone_enabled,
                            state.selected_hero,
                            state.invoker_active_combo_profile_id.clone(),
                        )
                    };

                    if !standalone_enabled {
                        info!("Standalone scripts disabled");
                        continue;
                    }

                    match selected_hero {
                        Some(state::HeroType::Invoker) => {
                            let profile_id = {
                                let settings = hotkey_settings.lock().unwrap();
                                resolve_active_combo_profile_id(
                                    &settings.heroes.invoker,
                                    active_profile_id.as_deref(),
                                )
                            };

                            if let Some(profile_id) = profile_id {
                                info!(
                                    "Triggering standalone combo for {} with profile {}",
                                    models::Hero::Invoker.to_game_name(),
                                    profile_id
                                );
                                dispatcher_clone2.dispatch_invoker_profile(&profile_id);
                            } else {
                                info!("Invoker standalone combo skipped: no enabled combo profile");
                            }
                        }
                        Some(hero_type) => {
                            let hero_name = match hero_type {
                                state::HeroType::Huskar => models::Hero::Huskar.to_game_name(),
                                state::HeroType::Largo => models::Hero::Largo.to_game_name(),
                                state::HeroType::LegionCommander => {
                                    models::Hero::LegionCommander.to_game_name()
                                }
                                state::HeroType::Magnus => models::Hero::Magnataur.to_game_name(),
                                state::HeroType::Meepo => models::Hero::Meepo.to_game_name(),
                                state::HeroType::Mirana => models::Hero::Mirana.to_game_name(),
                                state::HeroType::OutworldDestroyer => {
                                    models::Hero::ObsidianDestroyer.to_game_name()
                                }
                                state::HeroType::ShadowFiend => {
                                    models::Hero::Nevermore.to_game_name()
                                }
                                state::HeroType::Slark => models::Hero::Slark.to_game_name(),
                                state::HeroType::Tiny => models::Hero::Tiny.to_game_name(),
                                state::HeroType::Snapfire => models::Hero::Snapfire.to_game_name(),
                                state::HeroType::Invoker => unreachable!(),
                            };
                            info!("Triggering standalone combo for {}", hero_name);
                            dispatcher_clone2.dispatch_standalone_trigger(hero_name);
                        }
                        None => {
                            info!("No hero selected for standalone combo");
                        }
                    }
                }
                input::keyboard::HotkeyEvent::MeepoFarmToggle => {
                    let state = app_state_clone2.lock().unwrap();
                    if state.standalone_enabled
                        && state.selected_hero == Some(state::HeroType::Meepo)
                    {
                        drop(state);
                        if let Some(script) = dispatcher_clone2
                            .hero_scripts
                            .get(models::Hero::Meepo.to_game_name())
                        {
                            if let Some(meepo_script) = script
                                .as_any()
                                .downcast_ref::<crate::actions::heroes::MeepoScript>(
                            ) {
                                meepo_script.toggle_farm_assist();
                            }
                        }
                    }
                }
                input::keyboard::HotkeyEvent::ArmletRoshanToggle => {
                    let armed = crate::actions::armlet::toggle_roshan_mode();
                    info!(
                        "Armlet Roshan mode {} via hotkey",
                        if armed { "armed" } else { "disarmed" }
                    );
                }
                input::keyboard::HotkeyEvent::WaveOverlayToggle => {
                    // The overlay is a Tauri window; this binary is headless and
                    // has no way to show it.
                    info!("Wave overlay toggle ignored - this binary is headless");
                }
                input::keyboard::HotkeyEvent::CaptureHudPortrait => {
                    match observability::hud_anchors::apply_portrait_capture(&hotkey_settings) {
                        Ok((x, y)) => info!(
                            "HUD portrait anchor captured at ({:.4}, {:.4})",
                            x, y
                        ),
                        Err(e) => warn!("HUD portrait capture failed: {}", e.user_message()),
                    }
                }
                input::keyboard::HotkeyEvent::InvokerCycleComboProfile => {
                    let enabled_combo_profile_ids = hotkey_settings
                        .lock()
                        .unwrap()
                        .heroes
                        .invoker
                        .profiles
                        .iter()
                        .filter(|profile| {
                            profile.enabled && profile.mode == InvokerProfileMode::Combo
                        })
                        .map(|profile| profile.id.clone())
                        .collect::<Vec<_>>();

                    let mut state = app_state_clone2.lock().unwrap();
                    if !state.standalone_enabled
                        || state.selected_hero != Some(state::HeroType::Invoker)
                    {
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
                input::keyboard::HotkeyEvent::LargoQ => {
                    let state = app_state_clone2.lock().unwrap();
                    if state.standalone_enabled
                        && state.selected_hero == Some(state::HeroType::Largo)
                    {
                        drop(state);
                        if let Some(script) = dispatcher_clone2
                            .hero_scripts
                            .get(models::Hero::Largo.to_game_name())
                        {
                            if let Some(largo_script) = script
                                .as_any()
                                .downcast_ref::<crate::actions::heroes::LargoScript>(
                            ) {
                                largo_script.select_song_manually(
                                    crate::actions::heroes::largo::Song::Bullbelly,
                                );
                            }
                        }
                    }
                }
                input::keyboard::HotkeyEvent::LargoW => {
                    let state = app_state_clone2.lock().unwrap();
                    if state.standalone_enabled
                        && state.selected_hero == Some(state::HeroType::Largo)
                    {
                        drop(state);
                        if let Some(script) = dispatcher_clone2
                            .hero_scripts
                            .get(models::Hero::Largo.to_game_name())
                        {
                            if let Some(largo_script) = script
                                .as_any()
                                .downcast_ref::<crate::actions::heroes::LargoScript>(
                            ) {
                                largo_script.select_song_manually(
                                    crate::actions::heroes::largo::Song::Hotfeet,
                                );
                            }
                        }
                    }
                }
                input::keyboard::HotkeyEvent::LargoE => {
                    let state = app_state_clone2.lock().unwrap();
                    if state.standalone_enabled
                        && state.selected_hero == Some(state::HeroType::Largo)
                    {
                        drop(state);
                        if let Some(script) = dispatcher_clone2
                            .hero_scripts
                            .get(models::Hero::Largo.to_game_name())
                        {
                            if let Some(largo_script) = script
                                .as_any()
                                .downcast_ref::<crate::actions::heroes::LargoScript>(
                            ) {
                                largo_script.select_song_manually(
                                    crate::actions::heroes::largo::Song::IslandElixir,
                                );
                            }
                        }
                    }
                }
                input::keyboard::HotkeyEvent::LargoR => {
                    // R key pressed - immediately stop the beat loop to prevent stale key presses
                    // GSI will confirm the state change shortly after
                    let state = app_state_clone2.lock().unwrap();
                    if state.standalone_enabled
                        && state.selected_hero == Some(state::HeroType::Largo)
                    {
                        drop(state);
                        if let Some(script) = dispatcher_clone2
                            .hero_scripts
                            .get(models::Hero::Largo.to_game_name())
                        {
                            if let Some(largo_script) = script
                                .as_any()
                                .downcast_ref::<crate::actions::heroes::LargoScript>(
                            ) {
                                largo_script.deactivate_ultimate();
                            }
                        }
                    }
                }
                input::keyboard::HotkeyEvent::InvokerProfile(profile_id) => {
                    let profile = {
                        let settings = hotkey_settings.lock().unwrap();
                        settings
                            .heroes
                            .invoker
                            .profiles
                            .iter()
                            .find(|profile| profile.id == profile_id)
                            .cloned()
                    };

                    if let Some(profile) = profile {
                        if profile.enabled && profile.mode == InvokerProfileMode::Combo {
                            let mut state = app_state_clone2.lock().unwrap();
                            state.invoker_active_combo_profile_id = Some(profile.id.clone());
                            drop(state);

                            info!("Invoker active combo profile set to {}", profile.id);
                            push_activity(
                                ActivityCategory::System,
                                format!("Invoker active combo profile set to {}", profile.id),
                            );
                        }
                    }
                    dispatcher_clone2.dispatch_invoker_profile(&profile_id);
                }
            }
        }
    });

    // Block the main thread so background tasks keep running
    // (The Tauri binary in src-tauri/ provides the GUI)
    info!("Backend running (headless mode). Use the Tauri app for the GUI.");
    loop {
        std::thread::park();
    }
}
