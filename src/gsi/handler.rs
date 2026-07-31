use crate::actions::activity::{push_activity, ActivityCategory};
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use crate::state::AppState;
use axum::{body::Bytes, extract::State, http::StatusCode};
use chrono::Local;
use lazy_static::lazy_static;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Cap on rejected-payload dumps per run, so a permanently incompatible schema
/// cannot fill the disk while we are not looking.
const MAX_REJECTED_PAYLOAD_DUMPS: usize = 5;

lazy_static! {
    /// Track if hero was alive in the previous GSI event (to detect death transitions)
    static ref WAS_ALIVE: Mutex<bool> = Mutex::new(true);
}

static REJECTED_PAYLOAD_DUMPS: AtomicUsize = AtomicUsize::new(0);

pub type GsiEventSender = mpsc::Sender<GsiWebhookEvent>;

#[derive(Clone)]
pub struct GsiServerState {
    pub tx: GsiEventSender,
    pub app_state: Arc<Mutex<AppState>>,
}

fn refresh_keyboard_runtime_state(event: &GsiWebhookEvent, settings: &Settings) {
    // Canonical owner of shared keyboard/runtime cache refresh.
    // Called once per event in process_gsi_events() before the gsi_enabled gate.
    // Dispatcher must NOT duplicate this work.
    crate::actions::soul_ring::update_from_gsi(&event.items, &event.hero, settings);
    crate::actions::auto_items::update_gsi_state(event);
    crate::actions::heroes::broodmother::BROODMOTHER_ACTIVE.store(
        event.hero.name == Hero::Broodmother.to_game_name(),
        Ordering::SeqCst,
    );

    if event.hero.name == Hero::Nevermore.to_game_name() {
        let mut last_event = crate::actions::heroes::shadow_fiend::SF_LAST_EVENT
            .lock()
            .unwrap();
        *last_event = Some(event.clone());
    }

    if event.hero.name == Hero::Meepo.to_game_name() {
        crate::actions::heroes::meepo_state::refresh_meepo_observed_state(
            event,
            settings,
            crate::actions::danger_detector::is_in_danger(),
        );
    } else {
        crate::actions::heroes::meepo_state::clear_meepo_observed_state();
        crate::actions::heroes::meepo_macro::suspend_meepo_macro(
            crate::actions::heroes::meepo_macro::MeepoMacroSuspendReason::HeroChanged,
        );
    }
}

fn refresh_observability_state(
    event: &GsiWebhookEvent,
    app_state: &Arc<Mutex<AppState>>,
    settings: &Settings,
) {
    let snapshot = crate::observability::rune_alerts::process_clock_time(
        event.map.clock_time,
        &settings.rune_alerts,
    );

    // Scheduled objective alerts. Playback happens in Rust rather than the
    // WebView so alerts still sound while the app window is minimised.
    crate::observability::alerts::process_clock_time(event.map.clock_time, &settings.alerts);

    if let Ok(mut state) = app_state.lock() {
        state.rune_alerts = Some(snapshot);
    }
}

/// Write a payload we could not parse to disk so the exact shape can be diffed
/// against `GsiWebhookEvent`. Best-effort: a failure here must not affect the
/// response.
fn dump_rejected_payload(body: &Bytes, error: &serde_json::Error) {
    let dump_index = REJECTED_PAYLOAD_DUMPS.fetch_add(1, Ordering::Relaxed);
    if dump_index >= MAX_REJECTED_PAYLOAD_DUMPS {
        return;
    }

    let dir = PathBuf::from("logs/gsi_rejected");
    if fs::create_dir_all(&dir).is_err() {
        return;
    }

    let path = dir.join(format!(
        "rejected_{}_{}.json",
        Local::now().format("%Y-%m-%d_%H-%M-%S"),
        dump_index
    ));
    let Ok(mut file) = fs::File::create(&path) else {
        return;
    };

    let _ = writeln!(file, "// {}", error);
    let _ = file.write_all(body);
    info!("Wrote rejected GSI payload to {:?}", path);
}

pub async fn gsi_webhook_handler(
    State(server_state): State<GsiServerState>,
    body: Bytes,
) -> StatusCode {
    // Deserialize by hand rather than through the `Json` extractor: an extractor
    // rejection answers 422 before this function runs, so a schema mismatch used
    // to look exactly like "Dota is not sending anything".
    let event: GsiWebhookEvent = match serde_json::from_slice(&body) {
        Ok(event) => event,
        Err(error) => {
            if let Ok(mut state) = server_state.app_state.lock() {
                state.metrics.events_rejected += 1;
            }
            warn!("Could not parse GSI payload: {}", error);
            dump_rejected_payload(&body, &error);
            return StatusCode::BAD_REQUEST;
        }
    };

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
            let first_event = state.last_event.is_none();
            state.update_from_gsi(event.clone());
            state.metrics.current_queue_depth = rx.len();
            if first_event {
                push_activity(ActivityCategory::System, "GSI connected");
            }
        }

        // Keep keyboard-supporting runtime state fresh even when the main
        // GSI automation toggle is disabled.
        {
            let settings = settings.lock().unwrap();
            refresh_keyboard_runtime_state(&event, &settings);
            refresh_observability_state(&event, &app_state, &settings);
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
    use crate::actions::heroes::meepo_macro::{
        clear_meepo_macro_state, latest_meepo_macro_status, meepo_macro_test_lock,
    };
    use crate::actions::heroes::meepo_state::latest_meepo_observed_state;
    use crate::actions::heroes::shadow_fiend::SF_LAST_EVENT;
    use crate::actions::soul_ring::{SoulRingState, SOUL_RING_STATE};
    use crate::actions::ActionDispatcher;
    use crate::config::Settings;
    use crate::models::GsiWebhookEvent;
    use crate::observability::rune_alerts::{
        latest_rune_alert_snapshot, reset_rune_alert_state_for_tests,
    };
    use crate::state::AppState;
    use axum::{body::Bytes, extract::State, http::StatusCode};
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use tokio::sync::mpsc;

    fn encode(event: &GsiWebhookEvent) -> Bytes {
        Bytes::from(serde_json::to_vec(event).expect("event should serialize"))
    }

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
        clear_meepo_macro_state();
        crate::actions::heroes::meepo_state::clear_meepo_observed_state();
        reset_rune_alert_state_for_tests();
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
            encode(&event),
        )
        .await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(app_state.lock().unwrap().metrics.events_dropped, 1);
    }

    #[tokio::test]
    async fn webhook_handler_accepts_a_payload_with_a_short_ability_list() {
        // Heroes with fewer entries in their ability panel than the six the
        // schema models must not take the whole pipeline down.
        let payload = Bytes::from(
            r#"{
                "hero": { "name": "npc_dota_hero_spirit_breaker", "alive": true, "health_percent": 80 },
                "abilities": {
                    "ability0": { "name": "spirit_breaker_charge_of_darkness", "level": 4 },
                    "ability1": { "name": "spirit_breaker_bulldoze", "level": 1 }
                },
                "items": { "slot0": { "name": "item_phase_boots" } },
                "map": { "clock_time": 300 }
            }"#,
        );
        let app_state = AppState::new();
        let (tx, mut rx) = mpsc::channel(1);

        let status = gsi_webhook_handler(
            State(GsiServerState {
                tx,
                app_state: app_state.clone(),
            }),
            payload,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(app_state.lock().unwrap().metrics.events_rejected, 0);

        let event = rx.try_recv().expect("event should be queued");
        assert_eq!(event.hero.name, "npc_dota_hero_spirit_breaker");
        assert_eq!(event.abilities.ability1.name, "spirit_breaker_bulldoze");
        // Slots the payload omitted fall back to the "empty" placeholder the
        // rest of the codebase already expects.
        assert_eq!(event.abilities.ability5.level, 0);
        assert_eq!(event.items.slot1.name, "empty");
    }

    #[tokio::test]
    async fn webhook_handler_accepts_the_unpicked_hero_placeholder() {
        // Dota reports id -1 with no hero name during the draft.
        let payload = Bytes::from(
            r#"{"hero":{"id":-1,"name":""},"abilities":{},"items":{},"map":{"clock_time":-75}}"#,
        );
        let app_state = AppState::new();
        let (tx, mut rx) = mpsc::channel(1);

        let status = gsi_webhook_handler(
            State(GsiServerState {
                tx,
                app_state: app_state.clone(),
            }),
            payload,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let event = rx.try_recv().expect("event should be queued");
        assert_eq!(event.hero.id, -1);
        assert!(!event.has_hero());
    }

    #[tokio::test]
    async fn webhook_handler_counts_payloads_it_cannot_parse() {
        let app_state = AppState::new();
        let (tx, _rx) = mpsc::channel(1);

        let status = gsi_webhook_handler(
            State(GsiServerState {
                tx,
                app_state: app_state.clone(),
            }),
            Bytes::from("{ not json"),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(app_state.lock().unwrap().metrics.events_rejected, 1);
    }

    #[tokio::test]
    async fn process_gsi_events_refreshes_auto_items_cache_once_when_gsi_is_enabled() {
        let _guard = shared_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset_keyboard_runtime_state();
        crate::actions::auto_items::reset_update_counter_for_tests();

        // Use Huskar fixture but mutate event to prevent any real automation:
        // - Set hero to full health (no survivability actions)
        // - Disable all item casting
        let mut event = load_fixture_event("tests/fixtures/huskar_event.json");
        event.hero.health_percent = 100;
        event.hero.health = event.hero.max_health;
        event.hero.silenced = false;
        event.items.slot0.can_cast = Some(false);
        event.items.slot1.can_cast = Some(false);
        event.items.slot2.can_cast = Some(false);
        event.items.slot3.can_cast = Some(false);
        event.items.slot4.can_cast = Some(false);
        event.items.slot5.can_cast = Some(false);

        let app_state = AppState::new();
        app_state.lock().unwrap().gsi_enabled = true;

        let mut settings_value = Settings::default();
        // Disable automation features to prevent gameplay actions
        settings_value.soul_ring.enabled = false;
        settings_value.neutral_items.log_discoveries = false;
        let settings = std::sync::Arc::new(std::sync::Mutex::new(settings_value));
        let dispatcher = std::sync::Arc::new(ActionDispatcher::new(
            settings.clone(),
            ActionExecutor::new(),
            app_state.clone(),
        ));
        let (tx, rx) = mpsc::channel(1);

        tx.send(event.clone())
            .await
            .expect("test event should send");
        drop(tx);

        process_gsi_events(rx, app_state, dispatcher, settings).await;

        // Contract assertion: handler owns shared cache refresh
        // When gsi_enabled = true, handler should refresh caches once per event.
        // Dispatcher should not duplicate that work.
        // Current state: PASSES after deduplication fix (handler is sole owner of cache refresh).
        assert_eq!(
            crate::actions::auto_items::update_counter_for_tests(),
            1,
            "Handler should refresh auto_items cache exactly once per enabled event"
        );
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
            app_state.clone(),
        ));
        let (tx, rx) = mpsc::channel(1);

        tx.send(event.clone())
            .await
            .expect("test event should send");
        drop(tx);

        process_gsi_events(rx, app_state.clone(), dispatcher, settings).await;

        assert_eq!(
            app_state
                .lock()
                .unwrap()
                .last_event
                .as_ref()
                .unwrap()
                .hero
                .name,
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
            app_state.clone(),
        ));
        let (tx, rx) = mpsc::channel(1);

        tx.send(event.clone())
            .await
            .expect("test event should send");
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

    #[tokio::test]
    async fn process_gsi_events_updates_rune_alert_snapshot_from_map_clock_time() {
        let _guard = shared_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset_keyboard_runtime_state();

        let mut event = load_fixture_event("tests/fixtures/huskar_event.json");
        event.map.clock_time = 110;

        let app_state = AppState::new();
        app_state.lock().unwrap().gsi_enabled = false;

        let settings = std::sync::Arc::new(std::sync::Mutex::new(Settings::default()));
        let dispatcher = std::sync::Arc::new(ActionDispatcher::new(
            settings.clone(),
            ActionExecutor::new(),
            app_state.clone(),
        ));
        let (tx, rx) = mpsc::channel(1);

        tx.send(event).await.expect("test event should send");
        drop(tx);

        process_gsi_events(rx, app_state, dispatcher, settings).await;

        let snapshot = latest_rune_alert_snapshot().expect("rune snapshot should exist");
        assert_eq!(snapshot.next_rune_time_seconds, Some(120));
        assert_eq!(snapshot.seconds_until_next_rune, Some(10));
        assert_eq!(snapshot.last_alerted_rune_time_seconds, Some(120));
        assert_eq!(snapshot.last_alert_clock_time_seconds, Some(110));
    }

    #[tokio::test]
    async fn process_gsi_events_refreshes_and_clears_meepo_observed_state() {
        let _guard = shared_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _meepo_guard = meepo_macro_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset_keyboard_runtime_state();

        let meepo_event = load_fixture_event("tests/fixtures/meepo_event.json");
        let huskar_event = load_fixture_event("tests/fixtures/huskar_event.json");

        let app_state = AppState::new();
        app_state.lock().unwrap().gsi_enabled = false;

        let settings = std::sync::Arc::new(std::sync::Mutex::new(Settings::default()));
        let dispatcher = std::sync::Arc::new(ActionDispatcher::new(
            settings.clone(),
            ActionExecutor::new(),
            app_state.clone(),
        ));
        let (tx, rx) = mpsc::channel(2);

        tx.send(meepo_event).await.expect("meepo event should send");
        tx.send(huskar_event).await.expect("huskar event should send");
        drop(tx);

        process_gsi_events(rx, app_state, dispatcher, settings).await;

        assert!(
            latest_meepo_observed_state().is_none(),
            "non-Meepo events should clear the Meepo observed-state cache"
        );
        assert!(matches!(
            latest_meepo_macro_status().mode,
            crate::actions::heroes::meepo_macro::MeepoMacroMode::Suspended(
                crate::actions::heroes::meepo_macro::MeepoMacroSuspendReason::HeroChanged
            )
        ));
    }
}
