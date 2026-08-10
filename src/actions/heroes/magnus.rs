use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::HeroScript;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

const REVERSE_POLARITY_ABILITY_NAME: &str = "magnataur_reverse_polarity";

/// Settle time before the facing right-click, matching the other facing combos.
const PRE_TURN_SETTLE_MS: u64 = 50;

/// Gap between the facing right-click and releasing ALT.
const ALT_RELEASE_DELAY_MS: u64 = 50;

lazy_static! {
    static ref MAGNUS_LAST_EVENT: Arc<Mutex<Option<GsiWebhookEvent>>> = Arc::new(Mutex::new(None));
}

/// Work item for the dedicated Magnus worker thread.
#[derive(Debug, PartialEq, Eq)]
enum MagnusRequest {
    DirectionalUltimate {
        ultimate_key: char,
        turn_delay_ms: u64,
    },
}

fn build_directional_ultimate_request(ultimate_key: char, turn_delay_ms: u64) -> MagnusRequest {
    MagnusRequest::DirectionalUltimate {
        ultimate_key,
        turn_delay_ms,
    }
}

static MAGNUS_REQUEST_QUEUE: LazyLock<mpsc::Sender<MagnusRequest>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<MagnusRequest>();

    thread::spawn(move || {
        info!("🦏 Magnus request worker started");

        while let Ok(request) = rx.recv() {
            run_magnus_request(request);
        }

        info!("🦏 Magnus request worker exited");
    });

    tx
});

fn run_magnus_request(request: MagnusRequest) {
    match request {
        MagnusRequest::DirectionalUltimate {
            ultimate_key,
            turn_delay_ms,
        } => run_directional_ultimate_request(ultimate_key, turn_delay_ms),
    }
}

/// Directional Reverse Polarity.
///
/// Reverse Polarity drags enemies to the arc in front of Magnus, so the facing
/// at cast time decides where they land. Turning toward the cursor first lines
/// the pull up with the Skewer that follows.
///
/// ALT is held only across the facing right-click — Reverse Polarity takes no
/// target, so the cast itself does not need the modifier.
fn run_directional_ultimate_request(ultimate_key: char, turn_delay_ms: u64) {
    thread::sleep(Duration::from_millis(PRE_TURN_SETTLE_MS));

    crate::input::simulation::alt_down();
    crate::input::simulation::mouse_click();

    thread::sleep(Duration::from_millis(ALT_RELEASE_DELAY_MS));
    crate::input::simulation::alt_up();

    thread::sleep(Duration::from_millis(turn_delay_ms));
    crate::input::simulation::press_key(ultimate_key);
}

fn spawn_magnus_fallback(request: MagnusRequest) {
    thread::spawn(move || {
        run_magnus_request(request);
    });
}

fn enqueue_magnus_request(request: MagnusRequest) {
    if let Err(err) = MAGNUS_REQUEST_QUEUE.send(request) {
        warn!("🦏 Magnus request queue unavailable; using fallback thread");
        spawn_magnus_fallback(err.0);
    }
}

fn ability_is_ready(event: &GsiWebhookEvent, ability_name: &str) -> bool {
    (0..=5).any(|index| {
        event
            .abilities
            .get_by_index(index)
            .is_some_and(|ability| {
                ability.name == ability_name && ability.level > 0 && ability.can_cast
            })
    })
}

pub struct MagnusState;

impl MagnusState {
    /// Whether the keyboard hook should swallow the ultimate key.
    ///
    /// Returns `false` when Reverse Polarity is unlevelled, on cooldown, or no
    /// GSI event has arrived yet. The hook then lets the key through untouched,
    /// so a cooldown press never issues the facing right-click and never walks
    /// Magnus toward the cursor for nothing.
    pub fn can_intercept_ultimate() -> bool {
        let event = MAGNUS_LAST_EVENT.lock().unwrap().clone();

        let Some(event) = event else {
            info!("🦏 Magnus ultimate intercept skipped: no GSI event available");
            return false;
        };

        if !ability_is_ready(&event, REVERSE_POLARITY_ABILITY_NAME) {
            info!("🦏 Magnus ultimate intercept skipped: Reverse Polarity not ready");
            return false;
        }

        true
    }

    /// Run the directional ultimate: ALT down → right-click to face cursor →
    /// ALT up → wait `turn_delay_ms` → cast Reverse Polarity.
    pub fn execute_directional_ultimate(ultimate_key: char, turn_delay_ms: u64) {
        enqueue_magnus_request(build_directional_ultimate_request(
            ultimate_key,
            turn_delay_ms,
        ));
    }
}

/// Magnus script.
///
/// Directional Reverse Polarity flow:
/// 1. keyboard.rs intercepts the ultimate key (default R) when Magnus is the
///    active hero and Reverse Polarity is castable.
/// 2. Calls `MagnusState::execute_directional_ultimate()`.
/// 3. The dedicated worker holds ALT, right-clicks to face the cursor, releases
///    ALT, waits, then casts Reverse Polarity.
pub struct MagnusScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl MagnusScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }
}

impl HeroScript for MagnusScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        {
            let mut last_event = MAGNUS_LAST_EVENT.lock().unwrap();
            *last_event = Some(event.clone());
        }

        let settings = self.settings.lock().unwrap();

        // Shared survivability only — the directional ultimate is keyboard-driven.
        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    fn handle_standalone_trigger(&self) {
        let settings = self.settings.lock().unwrap();
        let ultimate_key = settings.heroes.magnus.ultimate_key;
        let turn_delay_ms = settings.heroes.magnus.turn_delay_ms;
        drop(settings);
        info!("🦏 Magnus standalone directional ultimate triggered");
        MagnusState::execute_directional_ultimate(ultimate_key, turn_delay_ms);
    }

    fn hero_name(&self) -> &'static str {
        Hero::Magnataur.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn magnus_fixture() -> GsiWebhookEvent {
        serde_json::from_str(include_str!("../../../tests/fixtures/magnus_event.json"))
            .expect("Magnus fixture should deserialize")
    }

    #[test]
    fn build_directional_ultimate_request_preserves_key_and_delay() {
        let request = build_directional_ultimate_request('r', 60);
        assert_eq!(
            request,
            MagnusRequest::DirectionalUltimate {
                ultimate_key: 'r',
                turn_delay_ms: 60,
            }
        );
    }

    #[test]
    fn finds_reverse_polarity_when_levelled_and_castable() {
        let event = magnus_fixture();
        assert!(ability_is_ready(&event, REVERSE_POLARITY_ABILITY_NAME));
    }

    #[test]
    fn ability_on_cooldown_is_not_ready() {
        let mut event = magnus_fixture();
        event.abilities.ability3.can_cast = false;

        assert!(!ability_is_ready(&event, REVERSE_POLARITY_ABILITY_NAME));
    }

    #[test]
    fn unlevelled_ability_is_not_ready() {
        let mut event = magnus_fixture();
        event.abilities.ability3.level = 0;

        assert!(!ability_is_ready(&event, REVERSE_POLARITY_ABILITY_NAME));
    }

    #[test]
    fn unknown_ability_name_is_not_ready() {
        let event = magnus_fixture();
        assert!(!ability_is_ready(&event, "magnataur_not_a_real_ability"));
    }
}
