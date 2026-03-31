use crate::ipc_types::GameStateDto;
use crate::TauriAppState;

/// Returns current game state from the latest GSI event
#[tauri::command]
pub fn get_game_state(state: tauri::State<'_, TauriAppState>) -> Result<GameStateDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    let dto = if let Some(ref event) = app.last_event {
        let rune_timer = app
            .rune_alerts
            .as_ref()
            .and_then(|ra| ra.seconds_until_next_rune);

        GameStateDto {
            hero_name: app
                .selected_hero
                .map(|h| h.to_display_name().to_string()),
            hero_level: event.hero.level,
            hp_percent: event.hero.health_percent,
            mana_percent: event.hero.mana_percent,
            in_danger: false, // TODO: wire to danger detector
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
    };

    Ok(dto)
}
