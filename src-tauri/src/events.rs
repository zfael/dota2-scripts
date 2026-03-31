use crate::ipc_types::GameStateDto;
use crate::TauriAppState;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Starts a background task that polls AppState and emits game_state events at ~5Hz
pub fn start_game_state_emitter(app: AppHandle) {
    let tauri_state = app.state::<TauriAppState>();
    let app_state = tauri_state.app_state.clone();

    tauri::async_runtime::spawn(async move {
        let mut last_events_processed: u64 = 0;

        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;

            let dto = {
                let state = match app_state.lock() {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                // Only emit if there's new data
                if state.metrics.events_processed == last_events_processed {
                    continue;
                }
                last_events_processed = state.metrics.events_processed;

                build_game_state_dto(&state)
            };

            let _ = app.emit("gsi_update", &dto);
        }
    });
}

fn build_game_state_dto(state: &dota2_scripts::state::AppState) -> GameStateDto {
    if let Some(ref event) = state.last_event {
        let rune_timer = state
            .rune_alerts
            .as_ref()
            .and_then(|ra| ra.seconds_until_next_rune);

        GameStateDto {
            hero_name: state
                .selected_hero
                .map(|h| h.to_display_name().to_string()),
            hero_level: event.hero.level,
            hp_percent: event.hero.health_percent,
            mana_percent: event.hero.mana_percent,
            in_danger: false,
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
