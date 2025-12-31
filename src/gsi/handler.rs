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

pub async fn gsi_webhook_handler(
    State(tx): State<GsiEventSender>,
    Json(event): Json<GsiWebhookEvent>,
) -> StatusCode {
    debug!("Received GSI event for hero: {}", event.hero.name);

    match tx.try_send(event) {
        Ok(_) => StatusCode::OK,
        Err(mpsc::error::TrySendError::Full(_)) => {
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
            // Dispatch to action handlers asynchronously for time-critical actions
            let dispatcher_clone = dispatcher.clone();
            let event_clone = event.clone();
            tokio::spawn(async move {
                dispatcher_clone.dispatch_gsi_event(&event_clone);
            });
        }
    }
}
