use crate::gsi::handler::{gsi_webhook_handler, process_gsi_events};
use crate::models::GsiWebhookEvent;
use crate::state::AppState;
use axum::{routing::post, Router};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::info;

const EVENT_QUEUE_CAPACITY: usize = 10;

pub async fn start_gsi_server(
    port: u16,
    app_state: Arc<Mutex<AppState>>,
    dispatcher: Arc<crate::actions::ActionDispatcher>,
) {
    let (tx, rx) = mpsc::channel::<GsiWebhookEvent>(EVENT_QUEUE_CAPACITY);

    // Spawn event processor
    let app_state_clone = app_state.clone();
    let dispatcher_clone = dispatcher.clone();
    tokio::spawn(async move {
        process_gsi_events(rx, app_state_clone, dispatcher_clone).await;
    });

    // Build router
    let app = Router::new()
        .route("/", post(gsi_webhook_handler))
        .with_state(tx);

    let addr = format!("127.0.0.1:{}", port);
    info!("Starting GSI server on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind GSI server");

    axum::serve(listener, app)
        .await
        .expect("Failed to start GSI server");
}
