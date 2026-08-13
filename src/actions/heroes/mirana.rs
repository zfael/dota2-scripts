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

const LEAP_ABILITY_NAME: &str = "mirana_leap";

/// Settle time before the facing right-click, matching the other facing combos.
const PRE_TURN_SETTLE_MS: u64 = 50;

/// Gap between the facing right-click and releasing ALT.
const ALT_RELEASE_DELAY_MS: u64 = 50;

lazy_static! {
    static ref MIRANA_LAST_EVENT: Arc<Mutex<Option<GsiWebhookEvent>>> = Arc::new(Mutex::new(None));
}

/// Work item for the dedicated Mirana worker thread.
#[derive(Debug, PartialEq, Eq)]
enum MiranaRequest {
    DirectionalLeap { leap_key: char, turn_delay_ms: u64 },
}

fn build_directional_leap_request(leap_key: char, turn_delay_ms: u64) -> MiranaRequest {
    MiranaRequest::DirectionalLeap {
        leap_key,
        turn_delay_ms,
    }
}

static MIRANA_REQUEST_QUEUE: LazyLock<mpsc::Sender<MiranaRequest>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<MiranaRequest>();

    thread::spawn(move || {
        info!("🌙 Mirana request worker started");

        while let Ok(request) = rx.recv() {
            run_mirana_request(request);
        }

        info!("🌙 Mirana request worker exited");
    });

    tx
});

fn run_mirana_request(request: MiranaRequest) {
    match request {
        MiranaRequest::DirectionalLeap {
            leap_key,
            turn_delay_ms,
        } => run_directional_leap_request(leap_key, turn_delay_ms),
    }
}

/// Directional Leap.
///
/// Leap jumps along Mirana's facing at cast time, so turning toward the cursor
/// first is what decides where she lands. Same shape as Slark's Pounce.
///
/// ALT is released before the ability press — Leap takes no target, and holding
/// ALT over an ability key pings it to allies instead of casting it.
fn run_directional_leap_request(leap_key: char, turn_delay_ms: u64) {
    thread::sleep(Duration::from_millis(PRE_TURN_SETTLE_MS));

    crate::input::simulation::alt_down();
    crate::input::simulation::mouse_click();

    thread::sleep(Duration::from_millis(ALT_RELEASE_DELAY_MS));
    crate::input::simulation::alt_up();

    thread::sleep(Duration::from_millis(turn_delay_ms));
    crate::input::simulation::press_key(leap_key);
}

fn spawn_mirana_fallback(request: MiranaRequest) {
    thread::spawn(move || {
        run_mirana_request(request);
    });
}

fn enqueue_mirana_request(request: MiranaRequest) {
    if let Err(err) = MIRANA_REQUEST_QUEUE.send(request) {
        warn!("🌙 Mirana request queue unavailable; using fallback thread");
        spawn_mirana_fallback(err.0);
    }
}

/// Whether a named ability is levelled and castable right now.
///
/// Scans every slot rather than indexing one, because **GSI slot order is
/// ability order, not key order**: shard-, scepter- and innate-granted
/// abilities are inserted ahead of the ultimate, so the slot a key implies is
/// not the slot the ability lives in. Slark's shard fallback shipped broken for
/// exactly this reason — see `docs/heroes/slark.md`.
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

pub struct MiranaState;

impl MiranaState {
    /// Whether the keyboard hook should swallow the Leap key.
    ///
    /// Returns `false` when Leap is unlevelled, on cooldown, or no GSI event has
    /// arrived yet. The hook then lets the key through untouched, so a cooldown
    /// press never issues the facing right-click and never walks Mirana toward
    /// the cursor.
    pub fn can_intercept_leap() -> bool {
        let event = MIRANA_LAST_EVENT.lock().unwrap().clone();

        let Some(event) = event else {
            info!("🌙 Mirana leap intercept skipped: no GSI event available");
            return false;
        };

        if !ability_is_ready(&event, LEAP_ABILITY_NAME) {
            info!("🌙 Mirana leap intercept skipped: Leap not ready");
            return false;
        }

        true
    }

    /// Run the directional leap: ALT down → right-click to face cursor → ALT up
    /// → wait `turn_delay_ms` → cast Leap.
    pub fn execute_directional_leap(leap_key: char, turn_delay_ms: u64) {
        enqueue_mirana_request(build_directional_leap_request(leap_key, turn_delay_ms));
    }
}

/// Mirana script.
///
/// Directional Leap flow:
/// 1. keyboard.rs intercepts the Leap key (default E) when Mirana is the active
///    hero and Leap is castable.
/// 2. Calls `MiranaState::execute_directional_leap()`.
/// 3. The dedicated worker holds ALT, right-clicks to face the cursor, releases
///    ALT, waits, then casts Leap.
pub struct MiranaScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl MiranaScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }
}

impl HeroScript for MiranaScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        {
            let mut last_event = MIRANA_LAST_EVENT.lock().unwrap();
            *last_event = Some(event.clone());
        }

        let settings = self.settings.lock().unwrap();

        // Shared survivability only — the directional leap is keyboard-driven.
        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    fn handle_standalone_trigger(&self) {
        let settings = self.settings.lock().unwrap();
        let mirana = &settings.heroes.mirana;
        let leap_key = mirana.leap_key;
        let turn_delay_ms = mirana.turn_delay_ms;
        drop(settings);
        info!("🌙 Mirana standalone directional leap triggered");
        MiranaState::execute_directional_leap(leap_key, turn_delay_ms);
    }

    fn hero_name(&self) -> &'static str {
        Hero::Mirana.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mirana_fixture() -> GsiWebhookEvent {
        serde_json::from_str(include_str!("../../../tests/fixtures/mirana_event.json"))
            .expect("Mirana fixture should deserialize")
    }

    #[test]
    fn build_directional_leap_request_preserves_key_and_delay() {
        let request = build_directional_leap_request('e', 200);
        assert_eq!(
            request,
            MiranaRequest::DirectionalLeap {
                leap_key: 'e',
                turn_delay_ms: 200,
            }
        );
    }

    #[test]
    fn finds_leap_when_levelled_and_castable() {
        let event = mirana_fixture();
        assert!(ability_is_ready(&event, LEAP_ABILITY_NAME));
    }

    #[test]
    fn ability_on_cooldown_is_not_ready() {
        let mut event = mirana_fixture();
        assert_eq!(event.abilities.ability2.name, LEAP_ABILITY_NAME);
        event.abilities.ability2.can_cast = false;

        assert!(!ability_is_ready(&event, LEAP_ABILITY_NAME));
    }

    #[test]
    fn unlevelled_ability_is_not_ready() {
        let mut event = mirana_fixture();
        event.abilities.ability2.level = 0;

        assert!(!ability_is_ready(&event, LEAP_ABILITY_NAME));
    }

    #[test]
    fn unknown_ability_name_is_not_ready() {
        let event = mirana_fixture();
        assert!(!ability_is_ready(&event, "mirana_not_a_real_ability"));
    }

    /// Leap is found wherever it sits, not at the index its key suggests.
    ///
    /// `leap_key` defaults to `e`, the third ability slot — but an innate or a
    /// shard entry ahead of it shifts every index. This is the regression that
    /// cost Slark's shard fallback its entire feature.
    #[test]
    fn leap_is_found_by_name_not_by_the_slot_its_key_suggests() {
        let mut event = mirana_fixture();

        // Swap Leap into a slot no key would ever imply.
        let leap = event.abilities.ability2.clone();
        event.abilities.ability2 = event.abilities.ability5.clone();
        event.abilities.ability5 = leap;

        assert!(ability_is_ready(&event, LEAP_ABILITY_NAME));
    }
}
