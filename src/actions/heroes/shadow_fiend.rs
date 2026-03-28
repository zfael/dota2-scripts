use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::HeroScript;
use crate::config::Settings;
use crate::input::simulation::press_key;
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

lazy_static! {
    /// Shared state for Shadow Fiend to allow keyboard.rs to access last GSI event
    pub static ref SF_LAST_EVENT: Arc<Mutex<Option<GsiWebhookEvent>>> = Arc::new(Mutex::new(None));
}

/// Shadow Fiend raze execution helper

#[derive(Debug, PartialEq, Eq)]
enum ShadowFiendRequest {
    Raze { raze_key: char, raze_delay_ms: u64 },
    Ultimate { auto_d_on_ultimate: bool },
    Standalone { auto_bkb_on_ultimate: bool, auto_d_on_ultimate: bool },
}

/// Build a Raze request payload for the worker
fn build_raze_request(raze_key: char, raze_delay_ms: u64) -> ShadowFiendRequest {
    ShadowFiendRequest::Raze {
        raze_key,
        raze_delay_ms,
    }
}

/// Build an Ultimate request payload for the worker
fn build_ultimate_request(auto_d_on_ultimate: bool) -> ShadowFiendRequest {
    ShadowFiendRequest::Ultimate { auto_d_on_ultimate }
}

/// Build a Standalone request payload by copying relevant runtime flags from Settings
fn build_standalone_request(settings: &Settings) -> ShadowFiendRequest {
    let sf = &settings.heroes.shadow_fiend;
    ShadowFiendRequest::Standalone {
        auto_bkb_on_ultimate: sf.auto_bkb_on_ultimate,
        auto_d_on_ultimate: sf.auto_d_on_ultimate,
    }
}

/// Map an inventory slot string (e.g. "slot0") to the common keybinding character
fn slot_to_common_key(slot: &str) -> Option<char> {
    match slot {
        "slot0" => Some('z'),
        "slot1" => Some('x'),
        "slot2" => Some('c'),
        "slot3" => Some('v'),
        "slot4" => Some('b'),
        "slot5" => Some('n'),
        _ => None,
    }
}

static SHADOW_FIEND_REQUEST_QUEUE: LazyLock<mpsc::Sender<ShadowFiendRequest>> =
    LazyLock::new(|| {
        let (tx, rx) = mpsc::channel::<ShadowFiendRequest>();

        thread::spawn(move || {
            info!("👻 Shadow Fiend request worker started");

            while let Ok(request) = rx.recv() {
                run_shadow_fiend_request(request);
            }

            info!("👻 Shadow Fiend request worker exited");
        });

        tx
    });

fn run_shadow_fiend_request(request: ShadowFiendRequest) {
    match request {
        request @ ShadowFiendRequest::Raze { .. } => run_raze_request(request),
        request @ ShadowFiendRequest::Ultimate { .. } => run_ultimate_request(request),
        request @ ShadowFiendRequest::Standalone { .. } => run_standalone_request(request),
    }
}

fn spawn_shadow_fiend_fallback(request: ShadowFiendRequest) {
    thread::spawn(move || {
        run_shadow_fiend_request(request);
    });
}

fn enqueue_shadow_fiend_request(request: ShadowFiendRequest) {
    if let Err(err) = SHADOW_FIEND_REQUEST_QUEUE.send(request) {
        warn!("👻 Shadow Fiend request queue unavailable; using fallback thread");
        spawn_shadow_fiend_fallback(err.0);
    }
}

fn run_raze_request(request: ShadowFiendRequest) {
    let ShadowFiendRequest::Raze {
        raze_key,
        raze_delay_ms,
    } = request
    else {
        return;
    };

    thread::sleep(Duration::from_millis(50));

    crate::input::simulation::alt_down();
    crate::input::simulation::mouse_click();

    thread::sleep(Duration::from_millis(50));
    crate::input::simulation::alt_up();

    thread::sleep(Duration::from_millis(raze_delay_ms));
    crate::input::simulation::press_key(raze_key);
}

fn run_ultimate_request(request: ShadowFiendRequest) {
    let ShadowFiendRequest::Ultimate { auto_d_on_ultimate } = request else {
        return;
    };

    let event_guard = SF_LAST_EVENT.lock().unwrap();

    if let Some(event) = event_guard.as_ref() {
        let bkb_slot = event
            .items
            .all_slots()
            .iter()
            .find(|(_, item)| item.name.contains("black_king_bar") && item.can_cast == Some(true))
            .map(|(slot, _)| *slot);

        if let Some(slot) = bkb_slot {
            let key = slot_to_common_key(slot);

            if let Some(bkb_key) = key {
                info!("👻 SF Ultimate: Using BKB ({}) before Requiem", bkb_key);
                press_key(bkb_key);
                thread::sleep(Duration::from_millis(30));
                press_key(bkb_key);
                thread::sleep(Duration::from_millis(50));
            }
        } else {
            info!("👻 SF Ultimate: BKB not found or on cooldown");
        }
    } else {
        info!("👻 SF Ultimate: No GSI event available, skipping BKB");
    }

    drop(event_guard);

    if auto_d_on_ultimate {
        info!("👻 SF Ultimate: Using D ability");
        press_key('d');
        thread::sleep(Duration::from_millis(50));
    }

    info!("👻 SF Ultimate: Casting Requiem of Souls (R)");
    press_key('r');
}

fn run_standalone_request(request: ShadowFiendRequest) {
    let ShadowFiendRequest::Standalone {
        auto_bkb_on_ultimate,
        auto_d_on_ultimate,
    } = request
    else {
        return;
    };

    let event_guard = SF_LAST_EVENT.lock().unwrap();

    if let Some(event) = event_guard.as_ref() {
        let ult_ready = event.abilities.ability5.can_cast;
        if !ult_ready {
            info!("👻 SF Standalone: Ultimate on cooldown, skipping combo");
            return;
        }

        let blink_slot = event
            .items
            .all_slots()
            .iter()
            .find(|(_, item)| item.name.contains("blink") && item.can_cast == Some(true))
            .map(|(slot, _)| *slot);

        if blink_slot.is_none() {
            info!("👻 SF Standalone: Blink not found or on cooldown, skipping combo");
            return;
        }

        let blink_key = blink_slot.and_then(slot_to_common_key);
        let bkb_key = if auto_bkb_on_ultimate {
            event
                .items
                .all_slots()
                .iter()
                .find(|(_, item)| {
                    item.name.contains("black_king_bar") && item.can_cast == Some(true)
                })
                .and_then(|(slot, _)| slot_to_common_key(*slot))
        } else {
            None
        };

        drop(event_guard);

        if let Some(key) = blink_key {
            info!("👻 SF Standalone: Using Blink ({})", key);
            press_key(key);
            thread::sleep(Duration::from_millis(50));
        }

        if let Some(key) = bkb_key {
            info!("👻 SF Standalone: Using BKB ({})", key);
            press_key(key);
            thread::sleep(Duration::from_millis(30));
            press_key(key);
            thread::sleep(Duration::from_millis(50));
        }

        if auto_d_on_ultimate {
            info!("👻 SF Standalone: Using D ability");
            press_key('d');
            thread::sleep(Duration::from_millis(50));
        }

        info!("👻 SF Standalone: Casting Requiem of Souls (R)");
        press_key('r');
    } else {
        info!("👻 SF Standalone: No GSI event available, cannot check Blink");
    }
}

pub struct ShadowFiendState;

impl ShadowFiendState {
    /// Execute a raze with ALT hold for direction facing.
    pub fn execute_raze(raze_key: char, raze_delay_ms: u64) {
        enqueue_shadow_fiend_request(build_raze_request(raze_key, raze_delay_ms));
    }

    /// Execute ultimate with optional D after the caller has decided to run the auto-BKB path.
    /// Sequence: BKB (if available) → D (if enabled) → R
    pub fn execute_ultimate_combo(auto_d_on_ultimate: bool) {
        enqueue_shadow_fiend_request(build_ultimate_request(auto_d_on_ultimate));
    }

    /// Execute standalone combo: Blink + Ultimate (with BKB/D if configured)
    /// Only executes if Blink AND Ultimate are available (not on cooldown)
    pub fn execute_standalone_combo(settings: &Settings) {
        enqueue_shadow_fiend_request(build_standalone_request(settings));
    }
}

/// Shadow Fiend script
///
/// Raze interception flow:
/// 1. keyboard.rs intercepts Q/W/E when SF is enabled (via app_state.sf_enabled)
/// 2. Calls ShadowFiendState::execute_raze()
/// 3. execute_raze enqueues a request for the dedicated worker, which:
///    - Holds ALT (for cl_dota_alt_unit_movetodirection)
///    - Right-clicks to face direction
///    - Releases ALT, waits for direction to register
///    - Presses the raze key
///
/// Auto-BKB on Ultimate flow:
/// 1. keyboard.rs intercepts R when SF is enabled and auto_bkb_on_ultimate is enabled
/// 2. Calls ShadowFiendState::execute_ultimate_combo()
/// 3. execute_ultimate_combo enqueues a request for the dedicated worker, which:
///    - Checks for BKB in inventory (from SF_LAST_EVENT)
///    - If BKB available and can_cast: double-tap BKB key
///    - If auto_d_on_ultimate enabled: press D
///    - Press R for Requiem of Souls
pub struct ShadowFiendScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl ShadowFiendScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }
}

impl HeroScript for ShadowFiendScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        let settings = self.settings.lock().unwrap();

        // Store last event for ultimate combo (BKB lookup)
        {
            let mut last_event = SF_LAST_EVENT.lock().unwrap();
            *last_event = Some(event.clone());
        }

        // Use common survivability actions (danger detection, healing, defensive items)
        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    fn handle_standalone_trigger(&self) {
        info!("👻 Shadow Fiend standalone combo triggered");
        let settings = self.settings.lock().unwrap();
        ShadowFiendState::execute_standalone_combo(&settings);
    }

    fn hero_name(&self) -> &'static str {
        Hero::Nevermore.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;

    #[test]
    fn build_raze_request_preserves_key_and_delay() {
        let request = build_raze_request('q', 120);
        assert_eq!(
            request,
            ShadowFiendRequest::Raze {
                raze_key: 'q',
                raze_delay_ms: 120,
            }
        );
    }

    #[test]
    fn build_standalone_request_copies_runtime_flags_from_settings() {
        let mut settings = Settings::default();
        // Set non-default values to ensure build_standalone_request copies them
        settings.heroes.shadow_fiend.auto_bkb_on_ultimate = true;
        settings.heroes.shadow_fiend.auto_d_on_ultimate = true;
        let request = build_standalone_request(&settings);
        assert_eq!(
            request,
            ShadowFiendRequest::Standalone {
                auto_bkb_on_ultimate: true,
                auto_d_on_ultimate: true,
            }
        );
    }

    #[test]
    fn slot_to_common_key_maps_inventory_slots_consistently() {
        assert_eq!(slot_to_common_key("slot0"), Some('z'));
        assert_eq!(slot_to_common_key("slot5"), Some('n'));
        assert_eq!(slot_to_common_key("neutral0"), None);
    }

    #[test]
    fn slot_to_common_key_returns_none_for_unknown_slots() {
        assert_eq!(slot_to_common_key("stash0"), None);
    }

    #[test]
    fn build_ultimate_request_sets_auto_d_flag() {
        let request = build_ultimate_request(true);
        assert_eq!(
            request,
            ShadowFiendRequest::Ultimate { auto_d_on_ultimate: true },
        );
    }

    #[test]
    fn build_standalone_request_does_not_borrow_settings_after_build() {
        let request = {
            let settings = Settings::default();
            build_standalone_request(&settings)
        };

        assert!(matches!(request, ShadowFiendRequest::Standalone { .. }));
    }
}
