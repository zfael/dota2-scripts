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

fn build_game_state_dto(state: &dota2_scripts::state::AppState) -> GameStateDto {
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
            hero_name: state
                .selected_hero
                .map(|h| h.to_display_name().to_string()),
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
