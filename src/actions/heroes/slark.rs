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

const POUNCE_ABILITY_NAME: &str = "slark_pounce";

/// Settle time before the facing right-click, matching the other facing combos.
const PRE_TURN_SETTLE_MS: u64 = 50;

/// Gap between the facing right-click and releasing ALT.
const ALT_RELEASE_DELAY_MS: u64 = 50;

lazy_static! {
    static ref SLARK_LAST_EVENT: Arc<Mutex<Option<GsiWebhookEvent>>> = Arc::new(Mutex::new(None));
}

/// Work item for the dedicated Slark worker thread.
#[derive(Debug, PartialEq, Eq)]
enum SlarkRequest {
    DirectionalPounce { pounce_key: char, turn_delay_ms: u64 },
}

fn build_directional_pounce_request(pounce_key: char, turn_delay_ms: u64) -> SlarkRequest {
    SlarkRequest::DirectionalPounce {
        pounce_key,
        turn_delay_ms,
    }
}

static SLARK_REQUEST_QUEUE: LazyLock<mpsc::Sender<SlarkRequest>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<SlarkRequest>();

    thread::spawn(move || {
        info!("🐟 Slark request worker started");

        while let Ok(request) = rx.recv() {
            run_slark_request(request);
        }

        info!("🐟 Slark request worker exited");
    });

    tx
});

fn run_slark_request(request: SlarkRequest) {
    match request {
        SlarkRequest::DirectionalPounce {
            pounce_key,
            turn_delay_ms,
        } => run_directional_pounce_request(pounce_key, turn_delay_ms),
    }
}

/// Directional Pounce.
///
/// Pounce leaps along Slark's facing at cast time, so turning toward the cursor
/// first is what decides where the leash lands. Same shape as the Magnus
/// directional ultimate.
///
/// ALT is released before the ability press — Pounce takes no target, and
/// holding ALT over an ability key pings it to allies instead of casting it.
fn run_directional_pounce_request(pounce_key: char, turn_delay_ms: u64) {
    thread::sleep(Duration::from_millis(PRE_TURN_SETTLE_MS));

    crate::input::simulation::alt_down();
    crate::input::simulation::mouse_click();

    thread::sleep(Duration::from_millis(ALT_RELEASE_DELAY_MS));
    crate::input::simulation::alt_up();

    thread::sleep(Duration::from_millis(turn_delay_ms));
    crate::input::simulation::press_key(pounce_key);
}

fn spawn_slark_fallback(request: SlarkRequest) {
    thread::spawn(move || {
        run_slark_request(request);
    });
}

fn enqueue_slark_request(request: SlarkRequest) {
    if let Err(err) = SLARK_REQUEST_QUEUE.send(request) {
        warn!("🐟 Slark request queue unavailable; using fallback thread");
        spawn_slark_fallback(err.0);
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

pub struct SlarkState;

impl SlarkState {
    /// Whether the keyboard hook should swallow the Pounce key.
    ///
    /// Returns `false` when Pounce is unlevelled, on cooldown, or no GSI event
    /// has arrived yet. The hook then lets the key through untouched, so a
    /// cooldown press never issues the facing right-click and never walks a
    /// squishy carry toward the cursor.
    pub fn can_intercept_pounce() -> bool {
        let event = SLARK_LAST_EVENT.lock().unwrap().clone();

        let Some(event) = event else {
            info!("🐟 Slark pounce intercept skipped: no GSI event available");
            return false;
        };

        if !ability_is_ready(&event, POUNCE_ABILITY_NAME) {
            info!("🐟 Slark pounce intercept skipped: Pounce not ready");
            return false;
        }

        true
    }

    /// Run the directional pounce: ALT down → right-click to face cursor →
    /// ALT up → wait `turn_delay_ms` → cast Pounce.
    pub fn execute_directional_pounce(pounce_key: char, turn_delay_ms: u64) {
        enqueue_slark_request(build_directional_pounce_request(pounce_key, turn_delay_ms));
    }
}

/// Slark script.
///
/// Directional Pounce flow:
/// 1. keyboard.rs intercepts the Pounce key (default W) when Slark is the
///    active hero and Pounce is castable.
/// 2. Calls `SlarkState::execute_directional_pounce()`.
/// 3. The dedicated worker holds ALT, right-clicks to face the cursor, releases
///    ALT, waits, then casts Pounce.
pub struct SlarkScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl SlarkScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }
}

impl HeroScript for SlarkScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        {
            let mut last_event = SLARK_LAST_EVENT.lock().unwrap();
            *last_event = Some(event.clone());
        }

        let settings = self.settings.lock().unwrap();

        // Shared survivability only — the directional pounce is keyboard-driven.
        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    fn handle_standalone_trigger(&self) {
        let settings = self.settings.lock().unwrap();
        let slark = &settings.heroes.slark;
        let pounce_key = slark.pounce_key;
        let turn_delay_ms = slark.turn_delay_ms;
        drop(settings);
        info!("🐟 Slark standalone directional pounce triggered");
        SlarkState::execute_directional_pounce(pounce_key, turn_delay_ms);
    }

    fn hero_name(&self) -> &'static str {
        Hero::Slark.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slark_fixture() -> GsiWebhookEvent {
        serde_json::from_str(include_str!("../../../tests/fixtures/slark_event.json"))
            .expect("Slark fixture should deserialize")
    }

    #[test]
    fn build_directional_pounce_request_preserves_key_and_delay() {
        let request = build_directional_pounce_request('w', 60);
        assert_eq!(
            request,
            SlarkRequest::DirectionalPounce {
                pounce_key: 'w',
                turn_delay_ms: 60,
            }
        );
    }

    #[test]
    fn finds_pounce_when_levelled_and_castable() {
        let event = slark_fixture();
        assert!(ability_is_ready(&event, POUNCE_ABILITY_NAME));
    }

    #[test]
    fn ability_on_cooldown_is_not_ready() {
        let mut event = slark_fixture();
        event.abilities.ability1.can_cast = false;

        assert!(!ability_is_ready(&event, POUNCE_ABILITY_NAME));
    }

    #[test]
    fn unlevelled_ability_is_not_ready() {
        let mut event = slark_fixture();
        event.abilities.ability1.level = 0;

        assert!(!ability_is_ready(&event, POUNCE_ABILITY_NAME));
    }

    #[test]
    fn unknown_ability_name_is_not_ready() {
        let event = slark_fixture();
        assert!(!ability_is_ready(&event, "slark_not_a_real_ability"));
    }
}
