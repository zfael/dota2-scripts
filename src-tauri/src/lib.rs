pub mod commands;
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
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config,
            commands::state::get_app_state,
            commands::game::get_game_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
