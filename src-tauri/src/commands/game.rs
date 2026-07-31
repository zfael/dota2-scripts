use crate::events::build_game_state_dto;
use crate::ipc_types::GameStateDto;
use crate::TauriAppState;
#[cfg(test)]
use dota2_scripts::models::GsiWebhookEvent;
#[cfg(test)]
use dota2_scripts::state::AppState;
#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::time::{Duration, SystemTime};

/// Returns current game state from the latest GSI event
#[tauri::command]
pub fn get_game_state(state: tauri::State<'_, TauriAppState>) -> Result<GameStateDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    Ok(build_game_state_dto(&app))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load_huskar_event() -> GsiWebhookEvent {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("huskar_event.json");
        let json_data = fs::read_to_string(fixture_path).expect("Failed to read fixture");
        serde_json::from_str(&json_data).expect("Failed to deserialize fixture")
    }

    #[test]
    fn game_state_is_disconnected_when_no_recent_gsi_activity() {
        let mut app = AppState::default();
        app.update_from_gsi(load_huskar_event());
        // Past the 35s liveness window, which tracks Dota's 30s GSI heartbeat.
        app.last_gsi_activity_at = Some(SystemTime::now() - Duration::from_secs(60));

        let dto = build_game_state_dto(&app);

        assert!(!dto.connected);
        assert!(dto.hero_name.is_none());
    }

    #[test]
    fn game_state_stays_connected_with_recent_gsi_activity() {
        let mut app = AppState::default();
        app.update_from_gsi(load_huskar_event());

        let dto = build_game_state_dto(&app);

        assert!(dto.connected);
        assert_eq!(dto.hero_name.as_deref(), Some("Huskar"));
    }
}
