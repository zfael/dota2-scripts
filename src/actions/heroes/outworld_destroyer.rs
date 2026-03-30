use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::HeroScript;
use crate::config::{OutworldDestroyerConfig, Settings};
use crate::input::simulation::press_key;
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, warn};

const ARCANE_ORB_ABILITY_NAME: &str = "obsidian_destroyer_arcane_orb";
const ASTRAL_IMPRISONMENT_ABILITY_NAME: &str = "obsidian_destroyer_astral_imprisonment";
const OBJURGATION_ABILITY_NAME: &str = "obsidian_destroyer_objurgation";
const SANITYS_ECLIPSE_ABILITY_NAME: &str = "obsidian_destroyer_sanity_eclipse";

lazy_static! {
    pub static ref OD_LAST_EVENT: Arc<Mutex<Option<GsiWebhookEvent>>> = Arc::new(Mutex::new(None));
    static ref LAST_OBJURGATION_TRIGGER: Mutex<Option<Instant>> = Mutex::new(None);
}

#[derive(Debug, Clone)]
pub struct OutworldDestroyerComboConfig {
    pub slot_keys: [char; 6],
    pub objurgation_key: char,
    pub arcane_orb_key: char,
    pub astral_imprisonment_key: char,
    pub auto_bkb_on_ultimate: bool,
    pub auto_objurgation_on_ultimate: bool,
    pub post_bkb_delay_ms: u64,
    pub post_blink_delay_ms: u64,
    pub combo_items: Vec<String>,
    pub combo_item_spam_count: u32,
    pub combo_item_delay_ms: u64,
    pub post_ultimate_arcane_orb_presses: u32,
    pub arcane_orb_press_interval_ms: u64,
}

#[derive(Debug, Clone)]
enum OutworldDestroyerRequest {
    Ultimate(OutworldDestroyerComboConfig),
    Standalone(OutworldDestroyerComboConfig),
    SelfAstral { astral_imprisonment_key: char },
}

static OD_REQUEST_QUEUE: LazyLock<mpsc::Sender<OutworldDestroyerRequest>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<OutworldDestroyerRequest>();

    thread::spawn(move || {
        info!("🌌 Outworld Destroyer request worker started");

        while let Ok(request) = rx.recv() {
            run_outworld_destroyer_request(request);
        }

        info!("🌌 Outworld Destroyer request worker exited");
    });

    tx
});

fn ability_is_ready(event: &GsiWebhookEvent, ability_name: &str) -> bool {
    (0..=5).any(|index| {
        event.abilities
            .get_by_index(index)
            .is_some_and(|ability| {
                ability.name == ability_name && ability.level > 0 && ability.can_cast
            })
    })
}

fn find_castable_slot_key_by_name(
    event: &GsiWebhookEvent,
    slot_keys: &[char; 6],
    item_name: &str,
) -> Option<char> {
    let inventory = [
        (&event.items.slot0, slot_keys[0]),
        (&event.items.slot1, slot_keys[1]),
        (&event.items.slot2, slot_keys[2]),
        (&event.items.slot3, slot_keys[3]),
        (&event.items.slot4, slot_keys[4]),
        (&event.items.slot5, slot_keys[5]),
    ];

    inventory.iter().find_map(|(item, key)| {
        (item.name.contains(item_name) && item.can_cast == Some(true)).then_some(*key)
    })
}

fn should_trigger_objurgation(
    event: &GsiWebhookEvent,
    config: &OutworldDestroyerConfig,
    in_danger: bool,
    now: Instant,
    last_trigger: Option<Instant>,
) -> bool {
    if !config.auto_objurgation_on_danger || !in_danger {
        return false;
    }

    if !event.hero.alive || event.hero.stunned || event.hero.silenced {
        return false;
    }

    if event.hero.health_percent > config.objurgation_hp_threshold_percent {
        return false;
    }

    if event.hero.mana_percent < config.objurgation_min_mana_percent {
        return false;
    }

    if !ability_is_ready(event, OBJURGATION_ABILITY_NAME) {
        return false;
    }

    if let Some(last_trigger) = last_trigger {
        if now.duration_since(last_trigger)
            < Duration::from_millis(config.objurgation_trigger_cooldown_ms)
        {
            return false;
        }
    }

    true
}

fn execute_combo_items(event: &GsiWebhookEvent, config: &OutworldDestroyerComboConfig) {
    if config.combo_items.is_empty() {
        return;
    }

    let spam_count = config.combo_item_spam_count.max(1);

    for item_name in &config.combo_items {
        if let Some(key) = find_castable_slot_key_by_name(event, &config.slot_keys, item_name) {
            info!("🌌 OD combo item '{}' on key {}", item_name, key);
            for _ in 0..spam_count {
                press_key(key);
                thread::sleep(Duration::from_millis(config.combo_item_delay_ms));
            }
        }
    }
}

fn maybe_cast_bkb(event: &GsiWebhookEvent, config: &OutworldDestroyerComboConfig) {
    if !config.auto_bkb_on_ultimate {
        return;
    }

    if let Some(key) =
        find_castable_slot_key_by_name(event, &config.slot_keys, "black_king_bar")
    {
        info!("🌌 OD using BKB ({})", key);
        press_key(key);
        thread::sleep(Duration::from_millis(30));
        press_key(key);
        thread::sleep(Duration::from_millis(config.post_bkb_delay_ms));
    }
}

fn maybe_cast_objurgation(event: &GsiWebhookEvent, config: &OutworldDestroyerComboConfig) {
    if !config.auto_objurgation_on_ultimate || !ability_is_ready(event, OBJURGATION_ABILITY_NAME) {
        return;
    }

    info!("🌌 OD using Objurgation ({})", config.objurgation_key);
    press_key(config.objurgation_key);
    thread::sleep(Duration::from_millis(50));
}

fn maybe_cast_post_ultimate_orbs(event: &GsiWebhookEvent, config: &OutworldDestroyerComboConfig) {
    if config.post_ultimate_arcane_orb_presses == 0
        || !ability_is_ready(event, ARCANE_ORB_ABILITY_NAME)
    {
        return;
    }

    thread::sleep(Duration::from_millis(100));
    for _ in 0..config.post_ultimate_arcane_orb_presses {
        press_key(config.arcane_orb_key);
        thread::sleep(Duration::from_millis(config.arcane_orb_press_interval_ms));
    }
}

fn run_outworld_destroyer_request(request: OutworldDestroyerRequest) {
    match request {
        request @ OutworldDestroyerRequest::Ultimate(_) => run_ultimate_request(request),
        request @ OutworldDestroyerRequest::Standalone(_) => run_standalone_request(request),
        request @ OutworldDestroyerRequest::SelfAstral { .. } => run_self_astral_request(request),
    }
}

fn spawn_fallback_request(request: OutworldDestroyerRequest) {
    thread::spawn(move || {
        run_outworld_destroyer_request(request);
    });
}

fn enqueue_request(request: OutworldDestroyerRequest) {
    if let Err(err) = OD_REQUEST_QUEUE.send(request) {
        warn!("🌌 Outworld Destroyer request queue unavailable; using fallback thread");
        spawn_fallback_request(err.0);
    }
}

fn run_ultimate_request(request: OutworldDestroyerRequest) {
    let OutworldDestroyerRequest::Ultimate(config) = request else {
        return;
    };

    let event = OD_LAST_EVENT.lock().unwrap().clone();
    let Some(event) = event else {
        info!("🌌 OD ultimate intercept skipped: no GSI event available");
        return;
    };

    if !ability_is_ready(&event, SANITYS_ECLIPSE_ABILITY_NAME) {
        info!("🌌 OD ultimate intercept skipped: Sanity's Eclipse not ready");
        return;
    }

    maybe_cast_bkb(&event, &config);
    maybe_cast_objurgation(&event, &config);

    info!("🌌 OD casting Sanity's Eclipse (R)");
    press_key('r');
    maybe_cast_post_ultimate_orbs(&event, &config);
}

fn run_standalone_request(request: OutworldDestroyerRequest) {
    let OutworldDestroyerRequest::Standalone(config) = request else {
        return;
    };

    let event = OD_LAST_EVENT.lock().unwrap().clone();
    let Some(event) = event else {
        info!("🌌 OD standalone combo skipped: no GSI event available");
        return;
    };

    if !ability_is_ready(&event, SANITYS_ECLIPSE_ABILITY_NAME) {
        info!("🌌 OD standalone combo skipped: Sanity's Eclipse not ready");
        return;
    }

    if let Some(key) = find_castable_slot_key_by_name(&event, &config.slot_keys, "blink") {
        info!("🌌 OD using Blink ({})", key);
        press_key(key);
        thread::sleep(Duration::from_millis(config.post_blink_delay_ms));
    } else {
        info!("🌌 OD standalone combo continuing without Blink");
    }

    maybe_cast_bkb(&event, &config);
    execute_combo_items(&event, &config);
    maybe_cast_objurgation(&event, &config);

    info!("🌌 OD standalone combo casting Sanity's Eclipse (R)");
    press_key('r');
    maybe_cast_post_ultimate_orbs(&event, &config);
}

fn run_self_astral_request(request: OutworldDestroyerRequest) {
    let OutworldDestroyerRequest::SelfAstral {
        astral_imprisonment_key,
    } = request
    else {
        return;
    };

    let event = OD_LAST_EVENT.lock().unwrap().clone();
    let Some(event) = event else {
        info!("🌌 OD self-Astral skipped: no GSI event available");
        return;
    };

    if !ability_is_ready(&event, ASTRAL_IMPRISONMENT_ABILITY_NAME) {
        info!("🌌 OD self-Astral skipped: Astral Imprisonment not ready");
        return;
    }

    info!(
        "🌌 OD self-Astral via double-tap ({})",
        astral_imprisonment_key
    );
    press_key(astral_imprisonment_key);
    thread::sleep(Duration::from_millis(30));
    press_key(astral_imprisonment_key);
}

fn build_combo_config(settings: &Settings) -> OutworldDestroyerComboConfig {
    let od = &settings.heroes.outworld_destroyer;
    OutworldDestroyerComboConfig {
        slot_keys: [
            settings.keybindings.slot0,
            settings.keybindings.slot1,
            settings.keybindings.slot2,
            settings.keybindings.slot3,
            settings.keybindings.slot4,
            settings.keybindings.slot5,
        ],
        objurgation_key: od.objurgation_key,
        arcane_orb_key: od.arcane_orb_key,
        astral_imprisonment_key: od.astral_imprisonment_key,
        auto_bkb_on_ultimate: od.auto_bkb_on_ultimate,
        auto_objurgation_on_ultimate: od.auto_objurgation_on_ultimate,
        post_bkb_delay_ms: od.post_bkb_delay_ms,
        post_blink_delay_ms: od.post_blink_delay_ms,
        combo_items: od.combo_items.clone(),
        combo_item_spam_count: od.combo_item_spam_count,
        combo_item_delay_ms: od.combo_item_delay_ms,
        post_ultimate_arcane_orb_presses: od.post_ultimate_arcane_orb_presses,
        arcane_orb_press_interval_ms: od.arcane_orb_press_interval_ms,
    }
}

pub struct OutworldDestroyerState;

impl OutworldDestroyerState {
    pub fn can_intercept_ultimate() -> bool {
        OD_LAST_EVENT
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|event| event.hero.alive && ability_is_ready(event, SANITYS_ECLIPSE_ABILITY_NAME))
    }

    pub fn can_self_cast_astral() -> bool {
        OD_LAST_EVENT.lock().unwrap().as_ref().is_some_and(|event| {
            event.hero.alive && ability_is_ready(event, ASTRAL_IMPRISONMENT_ABILITY_NAME)
        })
    }

    pub fn execute_ultimate_combo(config: OutworldDestroyerComboConfig) {
        enqueue_request(OutworldDestroyerRequest::Ultimate(config));
    }

    pub fn execute_standalone_combo(config: OutworldDestroyerComboConfig) {
        enqueue_request(OutworldDestroyerRequest::Standalone(config));
    }

    pub fn execute_self_astral(astral_imprisonment_key: char) {
        enqueue_request(OutworldDestroyerRequest::SelfAstral {
            astral_imprisonment_key,
        });
    }
}

pub struct OutworldDestroyerScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl OutworldDestroyerScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }

    fn maybe_trigger_objurgation(
        &self,
        event: &GsiWebhookEvent,
        config: &OutworldDestroyerConfig,
        in_danger: bool,
    ) {
        let now = Instant::now();
        let mut last_trigger = LAST_OBJURGATION_TRIGGER.lock().unwrap();

        if !should_trigger_objurgation(event, config, in_danger, now, *last_trigger) {
            return;
        }

        *last_trigger = Some(now);
        let key = config.objurgation_key;
        self.executor.enqueue("od-objurgation-danger", move || {
            info!("🌌 OD auto-casting Objurgation on danger ({})", key);
            press_key(key);
        });
    }
}

impl HeroScript for OutworldDestroyerScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        {
            let mut last_event = OD_LAST_EVENT.lock().unwrap();
            *last_event = Some(event.clone());
        }

        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        let settings = self.settings.lock().unwrap();
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        self.maybe_trigger_objurgation(event, &settings.heroes.outworld_destroyer, in_danger);
        drop(settings);

        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    fn handle_standalone_trigger(&self) {
        info!("🌌 Outworld Destroyer standalone combo triggered");
        let settings = self.settings.lock().unwrap();
        OutworldDestroyerState::execute_standalone_combo(build_combo_config(&settings));
    }

    fn hero_name(&self) -> &'static str {
        Hero::ObsidianDestroyer.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub fn build_keyboard_combo_config(settings: &Settings) -> OutworldDestroyerComboConfig {
    build_combo_config(settings)
}

#[cfg(test)]
mod tests {
    use super::{
        ability_is_ready, find_castable_slot_key_by_name, should_trigger_objurgation,
        OBJURGATION_ABILITY_NAME, SANITYS_ECLIPSE_ABILITY_NAME,
    };
    use crate::config::Settings;
    use crate::models::GsiWebhookEvent;
    use std::time::{Duration, Instant};

    fn od_fixture() -> GsiWebhookEvent {
        serde_json::from_str(include_str!("../../../tests/fixtures/outworld_destroyer_event.json"))
            .expect("OD fixture should deserialize")
    }

    #[test]
    fn finds_od_named_abilities() {
        let event = od_fixture();
        assert!(ability_is_ready(&event, SANITYS_ECLIPSE_ABILITY_NAME));
        assert!(ability_is_ready(&event, OBJURGATION_ABILITY_NAME));
    }

    #[test]
    fn maps_castable_item_slots_with_custom_keys() {
        let event = od_fixture();
        assert_eq!(
            find_castable_slot_key_by_name(&event, &['z', 'x', 'c', 'v', 'b', 'n'], "blink"),
            Some('z')
        );
        assert_eq!(
            find_castable_slot_key_by_name(
                &event,
                &['z', 'x', 'c', 'v', 'b', 'n'],
                "black_king_bar"
            ),
            Some('x')
        );
    }

    #[test]
    fn objurgation_danger_plan_honors_thresholds_and_cooldown() {
        let event = od_fixture();
        let config = &Settings::default().heroes.outworld_destroyer;
        let now = Instant::now();

        assert!(should_trigger_objurgation(&event, config, true, now, None));
        assert!(!should_trigger_objurgation(
            &event,
            config,
            true,
            now,
            Some(now - Duration::from_millis(250))
        ));
    }
}
