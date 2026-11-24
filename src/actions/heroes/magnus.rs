use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::HeroScript;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use rdev::Key;
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, warn};

const REVERSE_POLARITY_ABILITY_NAME: &str = "magnataur_reverse_polarity";
const SKEWER_ABILITY_NAME: &str = "magnataur_skewer";

/// Settle time before the facing right-click, matching the other facing combos.
const PRE_TURN_SETTLE_MS: u64 = 50;

/// Gap between the facing right-click and releasing ALT.
const ALT_RELEASE_DELAY_MS: u64 = 50;

/// Gap between the two camera-centre taps. Dota treats a second press of the
/// hero-select key as "centre on selection", so the pair has to read as a
/// double-tap. Matches the Broodmother reselect cadence.
const CAMERA_TAP_INTERVAL_MS: u64 = 30;

lazy_static! {
    static ref MAGNUS_LAST_EVENT: Arc<Mutex<Option<GsiWebhookEvent>>> = Arc::new(Mutex::new(None));
    /// Blink as of the last GSI event; read by the keyboard hook on key presses,
    /// so it is kept apart from the full event.
    static ref MAGNUS_BLINK: Mutex<Option<BlinkSnapshot>> = Mutex::new(None);
    /// When the Blink slot's key was last pressed.
    static ref MAGNUS_BLINK_PRESSED_AT: Mutex<Option<Instant>> = Mutex::new(None);
}

/// Post-cast camera recentre: double-tap the hero-select key so the view snaps
/// back to Magnus for the Skewer follow-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraCenter {
    pub key: Key,
    /// Delay between the ultimate cast and the first tap.
    pub delay_ms: u64,
}

/// The saved point, as a fraction of Dota's client area, plus how long the
/// cursor rests on it. Resolved to screen pixels only at use time, against the
/// live window, so a moved or resized Dota window still lands on the same spot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnchorAim {
    pub fraction: (f32, f32),
    pub settle_ms: u64,
}

/// The aim for an anchor option, or `None` when the option is off or no point
/// has been saved yet — in both cases the key must behave as it always did.
pub fn plan_anchor_aim(
    enabled: bool,
    anchor: Option<(f32, f32)>,
    settle_ms: u64,
) -> Option<AnchorAim> {
    if !enabled {
        return None;
    }

    anchor.map(|fraction| AnchorAim {
        fraction,
        settle_ms,
    })
}

/// Blink as the last GSI event showed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlinkSnapshot {
    /// The inventory slot (0–5) holding the Blink.
    pub slot_index: usize,
    /// GSI reports a cooldown on it.
    pub on_cooldown: bool,
}

/// Find a Blink (any of the four) in the main inventory. Backpack, stash and
/// the neutral slot do not count: a Blink there cannot be cast.
pub fn blink_snapshot(event: &GsiWebhookEvent) -> Option<BlinkSnapshot> {
    event
        .items
        .all_slots()
        .into_iter()
        .take(6)
        .enumerate()
        .find(|(_, (_, item))| {
            crate::actions::common::item_name_matches_lookup(&item.name, "item_blink")
        })
        .map(|(slot_index, (_, item))| BlinkSnapshot {
            slot_index,
            on_cooldown: item.cooldown.is_some_and(|seconds| seconds > 0),
        })
}

/// The "only after Blink" gate: Magnus carries a Blink, and it is either on
/// cooldown or its key was pressed within `press_window`.
///
/// The press covers the gap before GSI reports the cooldown, so a Skewer
/// pressed right behind the Blink is not let through as a plain cast.
pub fn blink_gate_open(
    blink: Option<BlinkSnapshot>,
    since_blink_press: Option<Duration>,
    press_window: Duration,
) -> bool {
    let Some(blink) = blink else {
        return false;
    };

    blink.on_cooldown || since_blink_press.is_some_and(|elapsed| elapsed <= press_window)
}

/// Work item for the dedicated Magnus worker thread.
#[derive(Debug, PartialEq)]
enum MagnusRequest {
    DirectionalUltimate {
        ultimate_key: char,
        turn_delay_ms: u64,
        camera: Option<CameraCenter>,
    },
    SkewerToAnchor {
        skewer_key: char,
        aim: AnchorAim,
    },
}

fn build_directional_ultimate_request(
    ultimate_key: char,
    turn_delay_ms: u64,
    camera: Option<CameraCenter>,
) -> MagnusRequest {
    MagnusRequest::DirectionalUltimate {
        ultimate_key,
        turn_delay_ms,
        camera,
    }
}

/// Resolve the camera step from config, or `None` when it is switched off or
/// the configured key does not parse.
pub fn plan_camera_center(
    enabled: bool,
    key: Option<Key>,
    delay_ms: u64,
) -> Option<CameraCenter> {
    if !enabled {
        return None;
    }

    let Some(key) = key else {
        warn!("🦏 Magnus camera centring is enabled but the configured key did not parse");
        return None;
    };

    Some(CameraCenter { key, delay_ms })
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
            camera,
        } => run_directional_ultimate_request(ultimate_key, turn_delay_ms, camera),
        MagnusRequest::SkewerToAnchor { skewer_key, aim } => {
            run_skewer_to_anchor_request(skewer_key, aim)
        }
    }
}

/// Skewer at the saved point.
///
/// Moves the cursor onto the point, presses the Skewer key, and puts the
/// cursor back — one synthetic-input job, with the player's clicks held while
/// the cursor is away. Needs Quick Cast on the Skewer key: the press is the
/// cast. A point past Skewer's range gives a full-length Skewer toward it.
fn run_skewer_to_anchor_request(skewer_key: char, aim: AnchorAim) {
    let Some(point) = crate::observability::hud_anchors::resolve_fraction_point(aim.fraction)
    else {
        warn!("🦏 Magnus Skewer skipped: Dota's window was not found to aim at the saved point");
        return;
    };

    crate::input::simulation::press_key_at_point(skewer_key, point.x, point.y, aim.settle_ms);
}

/// Double-tap the hero-select key to recentre the camera on Magnus.
///
/// Uses the `rdev` replay path rather than the enigo queue because the key is
/// configurable and may be a named key such as `F1`, which the char-based
/// helpers cannot express. Runs after the cast, so it never contends with the
/// ALT hold.
fn run_camera_center(camera: CameraCenter) {
    thread::sleep(Duration::from_millis(camera.delay_ms));

    crate::input::keyboard::simulate_key(camera.key);
    thread::sleep(Duration::from_millis(CAMERA_TAP_INTERVAL_MS));
    crate::input::keyboard::simulate_key(camera.key);
}

/// Directional Reverse Polarity.
///
/// Reverse Polarity drags enemies to the arc in front of Magnus, so the facing
/// at cast time decides where they land. Turning toward the cursor first lines
/// the pull up with the Skewer that follows.
///
/// ALT is held only across the facing right-click — Reverse Polarity takes no
/// target, so the cast itself does not need the modifier.
///
/// The optional camera recentre runs last, after the cast, so it cannot move
/// the view before the facing right-click resolves.
fn run_directional_ultimate_request(
    ultimate_key: char,
    turn_delay_ms: u64,
    camera: Option<CameraCenter>,
) {
    thread::sleep(Duration::from_millis(PRE_TURN_SETTLE_MS));

    crate::input::simulation::alt_down();
    crate::input::simulation::mouse_click();

    thread::sleep(Duration::from_millis(ALT_RELEASE_DELAY_MS));
    crate::input::simulation::alt_up();

    thread::sleep(Duration::from_millis(turn_delay_ms));
    crate::input::simulation::press_key(ultimate_key);

    if let Some(camera) = camera {
        run_camera_center(camera);
    }
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

/// Whether the last GSI event shows `ability_name` levelled and castable.
/// `false` before any event has arrived.
fn last_event_ability_ready(ability_name: &str, label: &str) -> bool {
    let event = MAGNUS_LAST_EVENT.lock().unwrap().clone();

    let Some(event) = event else {
        info!("🦏 Magnus {label} intercept skipped: no GSI event available");
        return false;
    };

    if !ability_is_ready(&event, ability_name) {
        info!("🦏 Magnus {label} intercept skipped: not ready");
        return false;
    }

    true
}

/// Save the cursor's position as the Magnus anchor and persist it.
///
/// The point may be anywhere inside Dota's window; it is meant for the
/// minimap, where a screen point always means the same place on the map.
pub fn apply_anchor_capture(
    settings: &Arc<Mutex<Settings>>,
) -> Result<(f32, f32), crate::observability::hud_anchors::HudAnchorError> {
    let fraction = crate::observability::hud_anchors::capture_cursor_fraction()?;

    let mut settings = settings.lock().unwrap();
    settings.heroes.magnus.anchor_x_fraction = fraction.0;
    settings.heroes.magnus.anchor_y_fraction = fraction.1;
    settings.heroes.magnus.anchor_set = true;

    if let Err(e) = settings.save() {
        // Still good for this session, it just will not survive a restart.
        warn!("🦏 Saved the Magnus anchor but could not persist it: {e}");
    }

    info!(
        "🦏 Magnus anchor saved at ({:.4}, {:.4}) of Dota's client area",
        fraction.0, fraction.1
    );

    Ok(fraction)
}

/// The capture hotkey's handler, shared by both binaries: save the point, then
/// patch the keyboard hook's cached snapshot so the next E uses it at once.
pub fn capture_anchor_from_hotkey(
    settings: &Arc<Mutex<Settings>>,
    keyboard_snapshot: &Arc<std::sync::RwLock<crate::input::keyboard::KeyboardSnapshot>>,
) -> Option<(f32, f32)> {
    match apply_anchor_capture(settings) {
        Ok(fraction) => {
            match keyboard_snapshot.write() {
                Ok(mut cached) => cached.magnus.anchor = Some(fraction),
                Err(e) => warn!("🦏 Magnus anchor saved but the keyboard snapshot is locked: {e}"),
            }
            Some(fraction)
        }
        Err(e) => {
            warn!("🦏 Magnus anchor not saved: {}", anchor_capture_error_message(&e));
            None
        }
    }
}

/// A capture failure as a message for the log or the UI.
pub fn anchor_capture_error_message(
    error: &crate::observability::hud_anchors::HudAnchorError,
) -> &'static str {
    use crate::observability::hud_anchors::HudAnchorError;
    match error {
        // The shared message talks about the hero portrait.
        HudAnchorError::CursorOutsideDota => {
            "Point at a spot inside Dota's window (the minimap), then press the key again."
        }
        other => other.user_message(),
    }
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
        last_event_ability_ready(REVERSE_POLARITY_ABILITY_NAME, "ultimate")
    }

    /// Whether the keyboard hook should swallow the Skewer key. Same rule as
    /// the ultimate: on cooldown the key reaches Dota unchanged.
    pub fn can_intercept_skewer() -> bool {
        last_event_ability_ready(SKEWER_ABILITY_NAME, "Skewer")
    }

    /// Record a press of the Blink slot's key, for the "only after Blink"
    /// gate. `slot_keys` are the six inventory slot bindings; which one is the
    /// Blink comes from the last GSI event. Never blocks the key.
    pub fn note_key_press(key: Key, slot_keys: &[crate::input::binding::KeyBinding; 6]) {
        let Some(blink) = *MAGNUS_BLINK.lock().unwrap() else {
            return;
        };

        if slot_keys[blink.slot_index].matches_event(&rdev::EventType::KeyPress(key)) {
            *MAGNUS_BLINK_PRESSED_AT.lock().unwrap() = Some(Instant::now());
        }
    }

    /// Whether a Skewer press now counts as following a Blink: Magnus has
    /// one, and it is on cooldown or was pressed within `press_window_ms`.
    pub fn skewer_follows_blink(press_window_ms: u64) -> bool {
        let blink = *MAGNUS_BLINK.lock().unwrap();
        let since_press = MAGNUS_BLINK_PRESSED_AT
            .lock()
            .unwrap()
            .map(|pressed_at| pressed_at.elapsed());

        let open = blink_gate_open(blink, since_press, Duration::from_millis(press_window_ms));
        if !open {
            let reason = if blink.is_none() {
                "no Blink in the inventory"
            } else {
                "Blink not used"
            };
            info!("🦏 Magnus Skewer left at the cursor: {reason}");
        }
        open
    }

    /// Run the directional ultimate: ALT down → right-click to face the cursor
    /// → ALT up → wait `turn_delay_ms` → cast Reverse Polarity → optionally
    /// double-tap the hero-select key to recentre.
    pub fn execute_directional_ultimate(
        ultimate_key: char,
        turn_delay_ms: u64,
        camera: Option<CameraCenter>,
    ) {
        enqueue_magnus_request(build_directional_ultimate_request(
            ultimate_key,
            turn_delay_ms,
            camera,
        ));
    }

    /// Skewer at the saved point. Goes through the same worker as the
    /// ultimate, so an E pressed during the Reverse Polarity combo runs after
    /// it rather than in the middle of it.
    pub fn execute_skewer_to_anchor(skewer_key: char, aim: AnchorAim) {
        enqueue_magnus_request(MagnusRequest::SkewerToAnchor { skewer_key, aim });
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
        *MAGNUS_BLINK.lock().unwrap() = blink_snapshot(event);

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
        let magnus = &settings.heroes.magnus;
        let ultimate_key = magnus.ultimate_key;
        let turn_delay_ms = magnus.turn_delay_ms;
        let camera = plan_camera_center(
            magnus.center_camera_on_ultimate,
            crate::input::keyboard::parse_key_string(&magnus.camera_center_key),
            magnus.camera_center_delay_ms,
        );
        drop(settings);
        info!("🦏 Magnus standalone directional ultimate triggered");
        MagnusState::execute_directional_ultimate(ultimate_key, turn_delay_ms, camera);
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
        let request = build_directional_ultimate_request('r', 60, None);
        assert_eq!(
            request,
            MagnusRequest::DirectionalUltimate {
                ultimate_key: 'r',
                turn_delay_ms: 60,
                camera: None,
            }
        );
    }

    #[test]
    fn build_directional_ultimate_request_carries_the_camera_step() {
        let camera = CameraCenter {
            key: Key::Num1,
            delay_ms: 60,
        };
        let request = build_directional_ultimate_request('r', 60, Some(camera));

        assert_eq!(
            request,
            MagnusRequest::DirectionalUltimate {
                ultimate_key: 'r',
                turn_delay_ms: 60,
                camera: Some(camera),
            }
        );
    }

    /// Move the fixture's Blink (slot 0) to slot 2, as `name`.
    fn blink_in_slot(event: &mut GsiWebhookEvent, name: &str, cooldown: u32) {
        event.items.slot0.name = "item_branches".to_string();
        event.items.slot2.name = name.to_string();
        event.items.slot2.cooldown = Some(cooldown);
    }

    const WINDOW: Duration = Duration::from_millis(1000);

    #[test]
    fn finds_a_ready_blink_and_its_slot() {
        let mut event = magnus_fixture();
        blink_in_slot(&mut event, "item_blink", 0);

        assert_eq!(
            blink_snapshot(&event),
            Some(BlinkSnapshot {
                slot_index: 2,
                on_cooldown: false,
            })
        );
    }

    #[test]
    fn finds_an_upgraded_blink_on_cooldown() {
        let mut event = magnus_fixture();
        blink_in_slot(&mut event, "item_overwhelming_blink", 11);

        assert_eq!(
            blink_snapshot(&event),
            Some(BlinkSnapshot {
                slot_index: 2,
                on_cooldown: true,
            })
        );
    }

    #[test]
    fn a_blink_in_the_backpack_does_not_count() {
        let mut event = magnus_fixture();
        for slot in [
            &mut event.items.slot0,
            &mut event.items.slot1,
            &mut event.items.slot2,
            &mut event.items.slot3,
            &mut event.items.slot4,
            &mut event.items.slot5,
            &mut event.items.neutral0,
        ] {
            slot.name = "item_branches".to_string();
        }
        event.items.slot6.name = "item_blink".to_string();

        assert_eq!(blink_snapshot(&event), None);
    }

    #[test]
    fn blink_gate_is_closed_without_a_blink() {
        assert!(!blink_gate_open(None, Some(Duration::ZERO), WINDOW));
    }

    #[test]
    fn blink_gate_opens_on_a_blink_cooldown() {
        let blink = BlinkSnapshot {
            slot_index: 0,
            on_cooldown: true,
        };
        assert!(blink_gate_open(Some(blink), None, WINDOW));
    }

    #[test]
    fn blink_gate_opens_on_a_recent_press_before_gsi_catches_up() {
        let blink = BlinkSnapshot {
            slot_index: 0,
            on_cooldown: false,
        };
        assert!(blink_gate_open(
            Some(blink),
            Some(Duration::from_millis(300)),
            WINDOW
        ));
    }

    #[test]
    fn blink_gate_stays_closed_for_a_ready_blink_pressed_long_ago() {
        let blink = BlinkSnapshot {
            slot_index: 0,
            on_cooldown: false,
        };
        assert!(!blink_gate_open(Some(blink), None, WINDOW));
        assert!(!blink_gate_open(
            Some(blink),
            Some(Duration::from_millis(1500)),
            WINDOW
        ));
    }

    #[test]
    fn anchor_aim_is_skipped_when_the_option_is_off() {
        assert_eq!(plan_anchor_aim(false, Some((0.1, 0.9)), 30), None);
    }

    #[test]
    fn anchor_aim_is_skipped_until_a_point_is_saved() {
        assert_eq!(plan_anchor_aim(true, None, 30), None);
    }

    #[test]
    fn anchor_aim_carries_the_point_and_settle_when_on() {
        assert_eq!(
            plan_anchor_aim(true, Some((0.1, 0.9)), 25),
            Some(AnchorAim {
                fraction: (0.1, 0.9),
                settle_ms: 25,
            })
        );
    }

    #[test]
    fn config_has_no_anchor_until_one_is_captured() {
        let mut config = crate::config::settings::MagnusConfig::default();
        config.anchor_x_fraction = 0.2;
        config.anchor_y_fraction = 0.8;
        assert_eq!(config.anchor_fraction(), None);

        config.anchor_set = true;
        assert_eq!(config.anchor_fraction(), Some((0.2, 0.8)));
    }

    #[test]
    fn anchor_options_default_off() {
        let config = crate::config::settings::MagnusConfig::default();
        assert!(!config.skewer_to_anchor_enabled);
        assert!(!config.skewer_only_after_blink);
        assert_eq!(config.blink_press_window_ms, 1000);
        assert!(!config.anchor_set);
        assert_eq!(config.skewer_key, 'e');
        assert_eq!(config.capture_anchor_key, "PageDown");
    }

    #[test]
    fn finds_skewer_when_levelled_and_castable() {
        let event = magnus_fixture();
        assert!(ability_is_ready(&event, SKEWER_ABILITY_NAME));
    }

    #[test]
    fn camera_center_plan_is_skipped_when_disabled() {
        assert_eq!(plan_camera_center(false, Some(Key::Num1), 60), None);
    }

    #[test]
    fn camera_center_plan_is_skipped_when_the_key_does_not_parse() {
        assert_eq!(plan_camera_center(true, None, 60), None);
    }

    #[test]
    fn camera_center_plan_carries_key_and_delay_when_enabled() {
        assert_eq!(
            plan_camera_center(true, Some(Key::F1), 80),
            Some(CameraCenter {
                key: Key::F1,
                delay_ms: 80,
            })
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
