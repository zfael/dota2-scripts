use crate::ipc_types::GameStateDto;
use crate::TauriAppState;
use dota2_scripts::actions::danger_detector;
#[cfg(test)]
use dota2_scripts::models::GsiWebhookEvent;
#[cfg(test)]
use dota2_scripts::state::AppState;
#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::time::{Duration, SystemTime};

fn build_game_state_dto(app: &dota2_scripts::state::AppState) -> GameStateDto {
    if app.has_recent_gsi_activity() {
        let event = app
            .last_event
            .as_ref()
            .expect("recent GSI activity should always have a last event");
        let rune_timer = app
            .rune_alerts
            .as_ref()
            .and_then(|ra| ra.seconds_until_next_rune);

        GameStateDto {
            hero_name: app.selected_hero.map(|h| h.to_display_name().to_string()),
            hero_level: event.hero.level,
            hp_percent: event.hero.health_percent,
            mana_percent: event.hero.mana_percent,
            in_danger: danger_detector::is_in_danger(),
            connected: true,
            alive: event.hero.alive,
            stunned: event.hero.stunned,
            silenced: event.hero.silenced,
            respawn_timer: if event.hero.respawn_seconds > 0 {
                Some(event.hero.respawn_seconds)
            } else {
                None
            },
            rune_timer,
            game_time: event.map.clock_time,
        }
    } else {
        GameStateDto {
            hero_name: None,
            hero_level: 0,
            hp_percent: 100,
            mana_percent: 100,
            in_danger: false,
            connected: false,
            alive: true,
            stunned: false,
            silenced: false,
            respawn_timer: None,
            rune_timer: None,
            game_time: 0,
        }
    }
}

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
        app.last_gsi_activity_at = Some(SystemTime::now() - Duration::from_secs(10));

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
