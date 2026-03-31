pub mod commands;
pub mod events;
pub mod ipc_types;

use dota2_scripts::config::Settings;
use dota2_scripts::state::AppState;
use std::sync::{Arc, Mutex};

/// Shared state managed by Tauri, accessible from all commands
pub struct TauriAppState {
    pub app_state: Arc<Mutex<AppState>>,
    pub settings: Arc<Mutex<Settings>>,
}

pub fn run() {
    let settings = Arc::new(Mutex::new(Settings::load()));
    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(TauriAppState {
            app_state: app_state.clone(),
            settings: settings.clone(),
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
