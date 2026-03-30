#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod config;
mod gsi;
mod input;
mod models;
mod state;
mod ui;
mod update;

use crate::actions::executor::ActionExecutor;
use crate::actions::ActionDispatcher;
use crate::config::Settings;
use crate::gsi::start_gsi_server;
use crate::input::keyboard::{start_keyboard_listener, KeyboardSnapshot};
use crate::state::{AppState, UpdateCheckState};
use crate::ui::Dota2ScriptApp;
use crate::update::{check_for_update, UpdateCheckResult};
use std::sync::{Arc, Mutex, RwLock};
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    // Load settings first to get log level
    let settings = Arc::new(Mutex::new(Settings::load()));
    
    // Initialize logging with config level or environment variable
    let log_level = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| settings.lock().unwrap().logging.level.clone());
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .init();

    info!("Starting Dota 2 Script Automation...");
    info!("Server port: {}", settings.lock().unwrap().server.port);

    // Initialize shared state
    let app_state = AppState::new();
    
    // Build the initial keyboard snapshot before starting the listener
    let initial_snapshot = {
        let settings_guard = settings.lock().unwrap();
        let state_guard = app_state.lock().unwrap();
        Arc::new(RwLock::new(KeyboardSnapshot::from_runtime(&settings_guard, &state_guard)))
    };

    // Initialize action dispatcher
    let action_executor = ActionExecutor::new();
    let dispatcher = Arc::new(ActionDispatcher::new(settings.clone(), action_executor));

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
    tokio::spawn(async move {
        start_gsi_server(port, app_state_clone, dispatcher_clone, settings_clone).await;
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

            tokio::task::spawn_blocking(move || {
                match check_for_update(include_prereleases) {
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
                }
            });
        }
    }

    // Start hotkey event handler in background
    let app_state_clone2 = app_state.clone();
    let dispatcher_clone2 = dispatcher.clone();
    std::thread::spawn(move || {
        while let Ok(event) = hotkey_rx.recv() {
            match event {
                input::keyboard::HotkeyEvent::ComboTrigger => {
                    let state = app_state_clone2.lock().unwrap();
                    if state.standalone_enabled {
                        if let Some(hero_type) = state.selected_hero {
                            let hero_name = match hero_type {
                                state::HeroType::Huskar => models::Hero::Huskar.to_game_name(),
                                state::HeroType::Largo => models::Hero::Largo.to_game_name(),
                                state::HeroType::LegionCommander => models::Hero::LegionCommander.to_game_name(),
                                state::HeroType::OutworldDestroyer => models::Hero::ObsidianDestroyer.to_game_name(),
                                state::HeroType::ShadowFiend => models::Hero::Nevermore.to_game_name(),
                                state::HeroType::Tiny => models::Hero::Tiny.to_game_name(),
                            };
                            info!("Triggering standalone combo for {}", hero_name);
                            drop(state); // Release lock before calling dispatcher
                            dispatcher_clone2.dispatch_standalone_trigger(hero_name);
                        } else {
                            info!("No hero selected for standalone combo");
                        }
                    } else {
                        info!("Standalone scripts disabled");
                    }
                }
                input::keyboard::HotkeyEvent::LargoQ => {
                    let state = app_state_clone2.lock().unwrap();
                    if state.standalone_enabled && state.selected_hero == Some(state::HeroType::Largo) {
                        drop(state);
                        if let Some(script) = dispatcher_clone2.hero_scripts.get(models::Hero::Largo.to_game_name()) {
                            if let Some(largo_script) = script.as_any().downcast_ref::<crate::actions::heroes::LargoScript>() {
                                largo_script.select_song_manually(crate::actions::heroes::largo::Song::Bullbelly);
                            }
                        }
                    }
                }
                input::keyboard::HotkeyEvent::LargoW => {
                    let state = app_state_clone2.lock().unwrap();
                    if state.standalone_enabled && state.selected_hero == Some(state::HeroType::Largo) {
                        drop(state);
                        if let Some(script) = dispatcher_clone2.hero_scripts.get(models::Hero::Largo.to_game_name()) {
                            if let Some(largo_script) = script.as_any().downcast_ref::<crate::actions::heroes::LargoScript>() {
                                largo_script.select_song_manually(crate::actions::heroes::largo::Song::Hotfeet);
                            }
                        }
                    }
                }
                input::keyboard::HotkeyEvent::LargoE => {
                    let state = app_state_clone2.lock().unwrap();
                    if state.standalone_enabled && state.selected_hero == Some(state::HeroType::Largo) {
                        drop(state);
                        if let Some(script) = dispatcher_clone2.hero_scripts.get(models::Hero::Largo.to_game_name()) {
                            if let Some(largo_script) = script.as_any().downcast_ref::<crate::actions::heroes::LargoScript>() {
                                largo_script.select_song_manually(crate::actions::heroes::largo::Song::IslandElixir);
                            }
                        }
                    }
                }
                input::keyboard::HotkeyEvent::LargoR => {
                    // R key pressed - immediately stop the beat loop to prevent stale key presses
                    // GSI will confirm the state change shortly after
                    let state = app_state_clone2.lock().unwrap();
                    if state.standalone_enabled && state.selected_hero == Some(state::HeroType::Largo) {
                        drop(state);
                        if let Some(script) = dispatcher_clone2.hero_scripts.get(models::Hero::Largo.to_game_name()) {
                            if let Some(largo_script) = script.as_any().downcast_ref::<crate::actions::heroes::LargoScript>() {
                                largo_script.deactivate_ultimate();
                            }
                        }
                    }
                }
            }
        }
    });

    // Start UI on main thread
    info!("Starting GUI...");
    
    // Load window icon
    let icon = load_icon();
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 700.0])
            .with_title("Dota 2 Script Automation")
            .with_icon(icon),
        ..Default::default()
    };

    let app = Dota2ScriptApp::new(app_state, settings, initial_snapshot);

    if let Err(e) = eframe::run_native(
        "Dota 2 Script Automation",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    ) {
        eprintln!("Failed to start GUI: {}", e);
    }
}

fn load_icon() -> egui::IconData {
    // Use PNG for better quality - ICO parsing can pick wrong resolution
    let icon_bytes = include_bytes!("../assets/icon.png");
    
    match image::load_from_memory(icon_bytes) {
        Ok(icon) => {
            let image = icon.into_rgba8();
            let (width, height) = image.dimensions();
            println!("Loaded icon: {}x{} pixels", width, height);
            egui::IconData {
                rgba: image.into_raw(),
                width,
                height,
            }
        }
        Err(e) => {
            eprintln!("Failed to load icon: {}", e);
            // Fallback: create a simple colored icon
            let size = 32u32;
            let rgba: Vec<u8> = (0..size * size)
                .flat_map(|_| vec![200u8, 50, 50, 255]) // Red-ish color
                .collect();
            
            egui::IconData {
                rgba,
                width: size,
                height: size,
            }
        }
    }
}
