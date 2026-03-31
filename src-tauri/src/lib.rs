pub mod commands;
pub mod events;
pub mod ipc_types;

use dota2_scripts::actions::executor::{ActionExecutor, ExecutorMetrics};
use dota2_scripts::actions::heroes::{LargoScript, MeepoScript};
use dota2_scripts::actions::ActionDispatcher;
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
    pub executor_metrics: Arc<ExecutorMetrics>,
}

pub fn run() {
    // Load settings
    let settings = Arc::new(Mutex::new(Settings::load()));

    // Initialize logging with config level or environment variable
    let log_level = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| settings.lock().unwrap().logging.level.clone());
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    info!("Starting Dota 2 Script Automation (Tauri)...");
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
    let dispatcher = Arc::new(ActionDispatcher::new(settings.clone(), action_executor));

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
    tauri::async_runtime::spawn(async move {
        start_gsi_server(port, gsi_app_state, gsi_dispatcher, gsi_settings).await;
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

    // Start hotkey event handler in background
    let hotkey_app_state = app_state.clone();
    let hotkey_dispatcher = dispatcher.clone();
    std::thread::spawn(move || {
        handle_hotkey_events(hotkey_rx, hotkey_app_state, hotkey_dispatcher);
    });

    // Build and run Tauri application
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(TauriAppState {
            app_state: app_state.clone(),
            settings: settings.clone(),
            executor_metrics,
        })
        .setup(|app| {
            let handle = app.handle().clone();
            events::start_game_state_emitter(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config,
            commands::config::update_config,
            commands::config::update_hero_config,
            commands::state::get_app_state,
            commands::state::set_gsi_enabled,
            commands::state::set_standalone_enabled,
            commands::state::select_hero,
            commands::game::get_game_state,
            commands::diagnostics::get_diagnostics,
            commands::updates::get_update_state,
            commands::updates::check_for_updates,
            commands::updates::apply_update,
            commands::updates::dismiss_update,
            commands::meepo::get_meepo_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Processes hotkey events from the keyboard listener and dispatches actions.
fn handle_hotkey_events(
    hotkey_rx: std::sync::mpsc::Receiver<HotkeyEvent>,
    app_state: Arc<Mutex<AppState>>,
    dispatcher: Arc<ActionDispatcher>,
) {
    while let Ok(event) = hotkey_rx.recv() {
        match event {
            HotkeyEvent::ComboTrigger => {
                let state = app_state.lock().unwrap();
                if state.standalone_enabled {
                    if let Some(hero_type) = state.selected_hero {
                        let hero_name = match hero_type {
                            HeroType::Huskar => Hero::Huskar.to_game_name(),
                            HeroType::Largo => Hero::Largo.to_game_name(),
                            HeroType::LegionCommander => Hero::LegionCommander.to_game_name(),
                            HeroType::Meepo => Hero::Meepo.to_game_name(),
                            HeroType::OutworldDestroyer => {
                                Hero::ObsidianDestroyer.to_game_name()
                            }
                            HeroType::ShadowFiend => Hero::Nevermore.to_game_name(),
                            HeroType::Tiny => Hero::Tiny.to_game_name(),
                        };
                        info!("Triggering standalone combo for {}", hero_name);
                        drop(state);
                        dispatcher.dispatch_standalone_trigger(hero_name);
                    } else {
                        info!("No hero selected for standalone combo");
                    }
                } else {
                    info!("Standalone scripts disabled");
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
