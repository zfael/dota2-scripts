use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::HeroScript;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

/// Work item for the dedicated Snapfire worker thread.
#[derive(Debug, PartialEq, Eq)]
enum SnapfireRequest {
    CookieLeap { cookie_key: char, turn_delay_ms: u64 },
}

fn build_cookie_leap_request(cookie_key: char, turn_delay_ms: u64) -> SnapfireRequest {
    SnapfireRequest::CookieLeap {
        cookie_key,
        turn_delay_ms,
    }
}

static SNAPFIRE_REQUEST_QUEUE: LazyLock<mpsc::Sender<SnapfireRequest>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<SnapfireRequest>();

    thread::spawn(move || {
        info!("🍪 Snapfire request worker started");

        while let Ok(request) = rx.recv() {
            run_snapfire_request(request);
        }

        info!("🍪 Snapfire request worker exited");
    });

    tx
});

fn run_snapfire_request(request: SnapfireRequest) {
    match request {
        SnapfireRequest::CookieLeap {
            cookie_key,
            turn_delay_ms,
        } => run_cookie_leap_request(cookie_key, turn_delay_ms),
    }
}

/// Directional Firesnap Cookie leap.
///
/// ALT is held across both the facing right-click and the cookie press so the
/// same modifier turns Snapfire toward the cursor and self-casts the leap.
fn run_cookie_leap_request(cookie_key: char, turn_delay_ms: u64) {
    thread::sleep(Duration::from_millis(50));

    crate::input::simulation::alt_down();
    crate::input::simulation::mouse_click();

    thread::sleep(Duration::from_millis(turn_delay_ms));
    crate::input::simulation::press_key(cookie_key);

    crate::input::simulation::alt_up();
}

fn spawn_snapfire_fallback(request: SnapfireRequest) {
    thread::spawn(move || {
        run_snapfire_request(request);
    });
}

fn enqueue_snapfire_request(request: SnapfireRequest) {
    if let Err(err) = SNAPFIRE_REQUEST_QUEUE.send(request) {
        warn!("🍪 Snapfire request queue unavailable; using fallback thread");
        spawn_snapfire_fallback(err.0);
    }
}

pub struct SnapfireState;

impl SnapfireState {
    /// Run the directional cookie combo: ALT down → right-click to face cursor →
    /// wait `turn_delay_ms` → self-cast cookie → ALT up.
    pub fn execute_cookie_leap(cookie_key: char, turn_delay_ms: u64) {
        enqueue_snapfire_request(build_cookie_leap_request(cookie_key, turn_delay_ms));
    }
}

/// Snapfire script.
///
/// Directional cookie flow:
/// 1. keyboard.rs intercepts the trigger key (default Space) when Snapfire is
///    the active hero.
/// 2. Calls `SnapfireState::execute_cookie_leap()`.
/// 3. The dedicated worker holds ALT, right-clicks to face the cursor, waits,
///    self-casts Firesnap Cookie, and releases ALT.
pub struct SnapfireScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl SnapfireScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }
}

impl HeroScript for SnapfireScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        let settings = self.settings.lock().unwrap();

        // Shared survivability only — the directional cookie is keyboard-driven.
        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    fn handle_standalone_trigger(&self) {
        let settings = self.settings.lock().unwrap();
        let cookie_key = settings.heroes.snapfire.cookie_key;
        let turn_delay_ms = settings.heroes.snapfire.turn_delay_ms;
        drop(settings);
        info!("🍪 Snapfire standalone cookie leap triggered");
        SnapfireState::execute_cookie_leap(cookie_key, turn_delay_ms);
    }

    fn hero_name(&self) -> &'static str {
        Hero::Snapfire.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_cookie_leap_request_preserves_key_and_delay() {
        let request = build_cookie_leap_request('w', 60);
        assert_eq!(
            request,
            SnapfireRequest::CookieLeap {
                cookie_key: 'w',
                turn_delay_ms: 60,
            }
        );
    }
}
