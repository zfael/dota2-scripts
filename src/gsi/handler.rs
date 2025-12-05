use crate::models::GsiWebhookEvent;
use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

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
) {
    while let Some(event) = rx.recv().await {
        // Update app state
        {
            let mut state = app_state.lock().unwrap();
            state.update_from_gsi(event.clone());
            state.metrics.current_queue_depth = rx.len();
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
