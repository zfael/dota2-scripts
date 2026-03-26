use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use chrono::Local;
use lazy_static::lazy_static;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
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

fn refresh_keyboard_runtime_state(event: &GsiWebhookEvent, settings: &Settings) {
    crate::actions::soul_ring::update_from_gsi(&event.items, &event.hero, settings);
    crate::actions::auto_items::update_gsi_state(event);
    crate::actions::heroes::broodmother::BROODMOTHER_ACTIVE.store(
        event.hero.name == Hero::Broodmother.to_game_name(),
        Ordering::SeqCst,
    );

    if event.hero.name == Hero::Nevermore.to_game_name() {
        let mut last_event =
            crate::actions::heroes::shadow_fiend::SF_LAST_EVENT.lock().unwrap();
        *last_event = Some(event.clone());
    }
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

        // Keep keyboard-supporting runtime state fresh even when the main
        // GSI automation toggle is disabled.
        {
            let settings = settings.lock().unwrap();
            refresh_keyboard_runtime_state(&event, &settings);
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
    use super::{gsi_webhook_handler, process_gsi_events, GsiServerState};
    use crate::actions::auto_items::LATEST_GSI_EVENT;
    use crate::actions::executor::ActionExecutor;
    use crate::actions::heroes::broodmother::BROODMOTHER_ACTIVE;
    use crate::actions::heroes::shadow_fiend::SF_LAST_EVENT;
    use crate::actions::soul_ring::{SoulRingState, SOUL_RING_STATE};
    use crate::actions::ActionDispatcher;
    use crate::config::Settings;
    use crate::models::GsiWebhookEvent;
    use crate::state::AppState;
    use axum::{extract::State, http::StatusCode, Json};
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use tokio::sync::mpsc;

    fn shared_test_lock() -> &'static Mutex<()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK.get_or_init(|| Mutex::new(()))
    }

    fn load_fixture_event(path: &str) -> GsiWebhookEvent {
        let json_data = fs::read_to_string(path).expect("Failed to read GSI fixture");
        serde_json::from_str(&json_data).expect("Failed to deserialize GSI fixture")
    }

    fn reset_keyboard_runtime_state() {
        *LATEST_GSI_EVENT.lock().unwrap() = None;
        *SF_LAST_EVENT.lock().unwrap() = None;
        *SOUL_RING_STATE.lock().unwrap() = SoulRingState::new();
        BROODMOTHER_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
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

    #[tokio::test]
    async fn process_gsi_events_refreshes_keyboard_state_when_gsi_automation_is_disabled() {
        let _guard = shared_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset_keyboard_runtime_state();

        let mut event = load_fixture_event("tests/fixtures/huskar_event.json");
        event.hero.name = crate::models::Hero::Broodmother.to_game_name().to_string();
        event.hero.alive = true;
        event.hero.mana_percent = 10;
        event.hero.health_percent = 80;
        event.items.slot0 = crate::models::gsi_event::Item {
            name: "item_soul_ring".to_string(),
            can_cast: Some(true),
            cooldown: Some(0),
            ..Default::default()
        };
        event.items.slot1 = crate::models::gsi_event::Item {
            name: "item_orchid".to_string(),
            can_cast: Some(true),
            cooldown: Some(0),
            ..Default::default()
        };

        let app_state = AppState::new();
        app_state.lock().unwrap().gsi_enabled = false;
        let settings = std::sync::Arc::new(std::sync::Mutex::new(Settings::default()));
        let dispatcher = std::sync::Arc::new(ActionDispatcher::new(
            settings.clone(),
            ActionExecutor::new(),
        ));
        let (tx, rx) = mpsc::channel(1);

        tx.send(event.clone()).await.expect("test event should send");
        drop(tx);

        process_gsi_events(rx, app_state.clone(), dispatcher, settings).await;

        assert_eq!(
            app_state.lock().unwrap().last_event.as_ref().unwrap().hero.name,
            crate::models::Hero::Broodmother.to_game_name()
        );
        assert!(BROODMOTHER_ACTIVE.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(
            LATEST_GSI_EVENT
                .lock()
                .unwrap()
                .as_ref()
                .map(|event| event.hero.name.as_str()),
            Some(crate::models::Hero::Broodmother.to_game_name())
        );

        let soul_ring_state = SOUL_RING_STATE.lock().unwrap();
        assert!(soul_ring_state.available);
        assert_eq!(soul_ring_state.slot_key, Some('z'));
        assert!(soul_ring_state.can_cast);
        assert!(soul_ring_state.hero_alive);
        assert_eq!(soul_ring_state.hero_mana_percent, 10);
    }

    #[tokio::test]
    async fn process_gsi_events_refreshes_sf_last_event_when_gsi_automation_is_disabled() {
        let _guard = shared_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset_keyboard_runtime_state();

        let mut event = load_fixture_event("tests/fixtures/huskar_event.json");
        event.hero.name = crate::models::Hero::Nevermore.to_game_name().to_string();

        let app_state = AppState::new();
        app_state.lock().unwrap().gsi_enabled = false;
        let settings = std::sync::Arc::new(std::sync::Mutex::new(Settings::default()));
        let dispatcher = std::sync::Arc::new(ActionDispatcher::new(
            settings.clone(),
            ActionExecutor::new(),
        ));
        let (tx, rx) = mpsc::channel(1);

        tx.send(event.clone()).await.expect("test event should send");
        drop(tx);

        process_gsi_events(rx, app_state, dispatcher, settings).await;

        assert_eq!(
            SF_LAST_EVENT
                .lock()
                .unwrap()
                .as_ref()
                .map(|event| event.hero.name.as_str()),
            Some(crate::models::Hero::Nevermore.to_game_name())
        );
    }
}
