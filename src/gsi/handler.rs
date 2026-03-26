use crate::config::Settings;
use crate::models::GsiWebhookEvent;
use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use chrono::Local;
use lazy_static::lazy_static;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

lazy_static! {
    /// Track if hero was alive in the previous GSI event (to detect death transitions)
    static ref WAS_ALIVE: Mutex<bool> = Mutex::new(true);
}

pub type GsiEventSender = mpsc::Sender<GsiWebhookEvent>;

#[derive(Clone)]
pub struct GsiServerState {
    pub tx: GsiEventSender,
    pub app_state: Arc<Mutex<AppState>>,
}

pub async fn gsi_webhook_handler(
    State(server_state): State<GsiServerState>,
    Json(event): Json<GsiWebhookEvent>,
) -> StatusCode {
    debug!("Received GSI event for hero: {}", event.hero.name);

    match server_state.tx.try_send(event) {
        Ok(_) => StatusCode::OK,
        Err(mpsc::error::TrySendError::Full(_)) => {
            if let Ok(mut state) = server_state.app_state.lock() {
                state.metrics.events_dropped += 1;
            }
            warn!("GSI event queue full, dropping event");
            StatusCode::SERVICE_UNAVAILABLE
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            warn!("GSI event channel closed");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub async fn process_gsi_events(
    mut rx: mpsc::Receiver<GsiWebhookEvent>,
    app_state: Arc<Mutex<AppState>>,
    dispatcher: Arc<crate::actions::ActionDispatcher>,
    settings: Arc<Mutex<Settings>>,
) {
    // Generate session filename once at startup
    let session_file: Option<PathBuf> = {
        let settings = settings.lock().unwrap();
        if settings.gsi_logging.enabled {
            let output_dir = PathBuf::from(&settings.gsi_logging.output_dir);
            if let Err(e) = fs::create_dir_all(&output_dir) {
                warn!("Failed to create GSI log directory: {}", e);
                None
            } else {
                let filename = output_dir.join(format!(
                    "gsi_events_{}.jsonl",
                    Local::now().format("%Y-%m-%d_%H-%M-%S")
                ));
                info!("GSI event logging enabled, writing to: {:?}", filename);
                Some(filename)
            }
        } else {
            None
        }
    };

    while let Some(event) = rx.recv().await {
        // Log event to file if enabled
        if let Some(ref filename) = session_file {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(filename) {
                if let Ok(json) = serde_json::to_string(&event) {
                    let _ = writeln!(file, "{}", json);
                }
            }
        }

        // Update app state
        {
            let mut state = app_state.lock().unwrap();
            state.update_from_gsi(event.clone());
            state.metrics.current_queue_depth = rx.len();
        }

        // Detect hero death (transition from alive to dead)
        {
            let is_alive = event.hero.is_alive();
            if let Ok(mut was_alive) = WAS_ALIVE.try_lock() {
                if *was_alive && !is_alive {
                    info!("💀 Hero died! (HP: {})", event.hero.health);
                } else if !*was_alive && is_alive {
                    info!("🔄 Hero respawned! (HP: {})", event.hero.health);
                }
                *was_alive = is_alive;
            }
        }

        // Check if GSI automation is enabled
        let gsi_enabled = {
            let state = app_state.lock().unwrap();
            state.gsi_enabled
        };

        if gsi_enabled {
            dispatcher.dispatch_gsi_event(&event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{gsi_webhook_handler, GsiServerState};
    use crate::models::GsiWebhookEvent;
    use crate::state::AppState;
    use axum::{extract::State, http::StatusCode, Json};
    use std::fs;
    use tokio::sync::mpsc;

    fn load_fixture_event(path: &str) -> GsiWebhookEvent {
        let json_data = fs::read_to_string(path).expect("Failed to read GSI fixture");
        serde_json::from_str(&json_data).expect("Failed to deserialize GSI fixture")
    }

    #[tokio::test]
    async fn webhook_handler_tracks_dropped_events_when_queue_is_full() {
        let event = load_fixture_event("tests/fixtures/huskar_event.json");
        let app_state = AppState::new();
        let (tx, _rx) = mpsc::channel(1);

        tx.try_send(event.clone())
            .expect("Channel should accept first event");

        let status = gsi_webhook_handler(
            State(GsiServerState {
                tx,
                app_state: app_state.clone(),
            }),
            Json(event),
        )
        .await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(app_state.lock().unwrap().metrics.events_dropped, 1);
    }
}
