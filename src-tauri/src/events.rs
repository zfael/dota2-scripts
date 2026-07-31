use crate::ipc_types::{ActivityEntryDto, AppStateDto, GameStateDto};
use crate::TauriAppState;
use dota2_scripts::actions::activity;
use dota2_scripts::actions::armlet;
use dota2_scripts::actions::danger_detector;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

static ACTIVITY_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Starts a background task that polls AppState and emits game_state events at ~5Hz
pub fn start_game_state_emitter(app: AppHandle) {
    let tauri_state = app.state::<TauriAppState>();
    let app_state = tauri_state.app_state.clone();

    tauri::async_runtime::spawn(async move {
        let mut last_emitted_state: Option<GameStateDto> = None;
        let mut last_emitted_app_state: Option<AppStateDto> = None;

        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;

            // Emit game state if changed
            {
                let dto = {
                    let state = match app_state.lock() {
                        Ok(s) => s,
                        Err(_) => {
                            drain_and_emit_activities(&app);
                            continue;
                        }
                    };

                    let dto = build_game_state_dto(&state);
                    if last_emitted_state.as_ref() != Some(&dto) {
                        Some(dto)
                    } else {
                        None
                    }
                };

                if let Some(dto) = dto {
                    last_emitted_state = Some(dto.clone());
                    let _ = app.emit("gsi_update", &dto);
                }
            }

            {
                let dto = {
                    let state = match app_state.lock() {
                        Ok(s) => s,
                        Err(_) => {
                            drain_and_emit_activities(&app);
                            continue;
                        }
                    };

                    let dto = build_app_state_dto(&state);
                    if last_emitted_app_state.as_ref() != Some(&dto) {
                        Some(dto)
                    } else {
                        None
                    }
                };

                if let Some(dto) = dto {
                    last_emitted_app_state = Some(dto.clone());
                    let _ = app.emit("app_state_update", &dto);
                }
            }

            // Drain and emit activity events
            drain_and_emit_activities(&app);
        }
    });
}

fn build_app_state_dto(state: &dota2_scripts::state::AppState) -> AppStateDto {
    AppStateDto {
        selected_hero: state
            .selected_hero
            .map(|hero| hero.to_display_name().to_string()),
        gsi_enabled: state.gsi_enabled,
        standalone_enabled: state.standalone_enabled,
        armlet_roshan_armed: armlet::is_roshan_mode_armed(),
        invoker_active_combo_profile_id: state.invoker_active_combo_profile_id.clone(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn drain_and_emit_activities(app: &AppHandle) {
    let entries = activity::drain_activities();
    for entry in entries {
        let id = ACTIVITY_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let timestamp = entry
            .timestamp
            .duration_since(UNIX_EPOCH)
            .map(|d| {
                let secs = d.as_secs() % 86400;
                let hours = secs / 3600;
                let minutes = (secs % 3600) / 60;
                let seconds = secs % 60;
                let millis = d.subsec_millis();
                format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
            })
            .unwrap_or_else(|_| "00:00:00.000".to_string());

        let dto = ActivityEntryDto {
            id: id.to_string(),
            timestamp,
            category: entry.category.as_str().to_string(),
            message: entry.message,
            details: entry.details,
        };
        let _ = app.emit("activity_event", &dto);
    }
}

/// Single source of truth for the UI's game state, shared by the 5Hz emitter
/// and the `get_game_state` command.
pub(crate) fn build_game_state_dto(state: &dota2_scripts::state::AppState) -> GameStateDto {
    if state.has_recent_gsi_activity() {
        let event = state
            .last_event
            .as_ref()
            .expect("recent GSI activity should always have a last event");
        let rune_timer = state
            .rune_alerts
            .as_ref()
            .and_then(|ra| ra.seconds_until_next_rune);

        GameStateDto {
            // Fall back to the name Dota sent. `selected_hero` only covers the
            // heroes that have automation, so keying the header off it made
            // every other hero render as "Waiting for game...".
            hero_name: state
                .selected_hero
                .map(|h| h.to_display_name().to_string())
                .or_else(|| dota2_scripts::models::display_name_for_game_name(&event.hero.name)),
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

#[cfg(test)]
mod tests {
    use super::build_game_state_dto;
    use dota2_scripts::models::GsiWebhookEvent;
    use dota2_scripts::state::AppState;

    fn state_playing(hero_name: &str) -> dota2_scripts::state::AppState {
        let mut event = GsiWebhookEvent::default();
        event.hero.name = hero_name.to_string();
        event.hero.level = 7;

        let state = AppState::new();
        let mut state = state.lock().unwrap();
        state.update_from_gsi(event);
        state.clone()
    }

    #[test]
    fn a_hero_with_automation_uses_its_configured_display_name() {
        let dto = build_game_state_dto(&state_playing("npc_dota_hero_nevermore"));

        assert_eq!(dto.hero_name.as_deref(), Some("Shadow Fiend"));
        assert!(dto.connected);
    }

    #[test]
    fn a_hero_without_automation_still_reports_as_in_game() {
        // Regression: hero_name came from the nine-hero HeroType enum, so every
        // other hero rendered as "Waiting for game..." despite live GSI.
        let dto = build_game_state_dto(&state_playing("npc_dota_hero_spirit_breaker"));

        assert_eq!(dto.hero_name.as_deref(), Some("Spirit Breaker"));
        assert_eq!(dto.hero_level, 7);
        assert!(dto.connected);
    }

    #[test]
    fn the_draft_placeholder_hero_reports_connected_with_no_hero() {
        let dto = build_game_state_dto(&state_playing(""));

        assert_eq!(dto.hero_name, None);
        assert!(dto.connected);
    }

    #[test]
    fn no_gsi_activity_reports_disconnected() {
        let state = AppState::new();
        let state = state.lock().unwrap().clone();

        let dto = build_game_state_dto(&state);

        assert!(!dto.connected);
        assert_eq!(dto.hero_name, None);
    }
}
