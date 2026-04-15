use crate::actions::common::{find_item_slot_by_name, SurvivabilityActions};
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::traits::HeroScript;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use std::time::Duration;
use tracing::info;

static INVOKER_LAST_EVENT: LazyLock<Mutex<Option<GsiWebhookEvent>>> =
    LazyLock::new(|| Mutex::new(None));
static INVOKER_SETTINGS: LazyLock<Mutex<Option<Settings>>> =
    LazyLock::new(|| Mutex::new(None));

#[derive(Debug, Clone)]
struct InvokerObservedState {
    quas_level: u32,
    wex_level: u32,
    exort_level: u32,
    invoke_ready: bool,
    active_spells: [Option<String>; 2],
    hero_alive: bool,
    hero_disabled: bool,
    has_scepter: bool,
    has_shard: bool,
}

impl InvokerObservedState {
    fn from_event(event: &GsiWebhookEvent) -> Self {
        let ability = |index| event.abilities.get_by_index(index).expect("valid slot");
        Self {
            quas_level: ability(0).level,
            wex_level: ability(1).level,
            exort_level: ability(2).level,
            invoke_ready: ability(3).name == "invoker_invoke" && ability(3).can_cast,
            active_spells: [
                Some(ability(4).name.clone()).filter(|name| name != "empty"),
                Some(ability(5).name.clone()).filter(|name| name != "empty"),
            ],
            hero_alive: event.hero.alive,
            hero_disabled: event.hero.stunned || event.hero.hexed || event.hero.silenced,
            has_scepter: event.hero.aghanims_scepter,
            has_shard: event.hero.aghanims_shard,
        }
    }

    fn active_spell_key(&self, spell_name: &str, config: &crate::config::settings::InvokerConfig) -> Option<char> {
        if self.active_spells[0].as_deref() == Some(spell_name) {
            Some(config.spell_slot_primary_key)
        } else if self.active_spells[1].as_deref() == Some(spell_name) {
            Some(config.spell_slot_secondary_key)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedSpellCast {
    prepare_keys: Vec<char>,
    cast_key: char,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedPrepPair {
    target_spells: Vec<&'static str>,
    prepare_keys: Vec<char>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedComboSequence {
    spells: Vec<&'static str>,
    item_names: Vec<String>,
}

fn orb_recipe(spell_name: &str, config: &crate::config::settings::InvokerConfig) -> Option<[char; 4]> {
    match spell_name {
        "invoker_tornado" => Some([config.wex_key, config.wex_key, config.quas_key, config.invoke_key]),
        "invoker_emp" => Some([config.wex_key, config.wex_key, config.wex_key, config.invoke_key]),
        "invoker_chaos_meteor" => Some([config.exort_key, config.exort_key, config.wex_key, config.invoke_key]),
        "invoker_deafening_blast" => Some([config.quas_key, config.wex_key, config.exort_key, config.invoke_key]),
        "invoker_cold_snap" => Some([config.quas_key, config.quas_key, config.quas_key, config.invoke_key]),
        "invoker_forge_spirit" => Some([config.exort_key, config.exort_key, config.quas_key, config.invoke_key]),
        "invoker_ghost_walk" => Some([config.quas_key, config.quas_key, config.wex_key, config.invoke_key]),
        "invoker_ice_wall" => Some([config.quas_key, config.quas_key, config.exort_key, config.invoke_key]),
        "invoker_sun_strike" => Some([config.exort_key, config.exort_key, config.exort_key, config.invoke_key]),
        _ => None,
    }
}

fn plan_single_spell(
    spell_name: &str,
    state: &InvokerObservedState,
    config: &crate::config::settings::InvokerConfig,
) -> Option<PlannedSpellCast> {
    if let Some(cast_key) = state.active_spell_key(spell_name, config) {
        return Some(PlannedSpellCast {
            prepare_keys: Vec::new(),
            cast_key,
        });
    }

    let prepare_keys = orb_recipe(spell_name, config)?.to_vec();
    Some(PlannedSpellCast {
        prepare_keys,
        cast_key: config.spell_slot_secondary_key,
    })
}

fn plan_prep_profile(
    profile: &str,
    state: &InvokerObservedState,
    config: &crate::config::settings::InvokerConfig,
) -> Option<PlannedPrepPair> {
    let target_spells = match profile {
        "tornado_emp" => vec!["invoker_tornado", "invoker_emp"],
        "meteor_blast" => vec!["invoker_chaos_meteor", "invoker_deafening_blast"],
        "cold_snap_forge_spirit" => vec!["invoker_cold_snap", "invoker_forge_spirit"],
        "ghost_walk_ice_wall" => vec!["invoker_ghost_walk", "invoker_ice_wall"],
        _ => return None,
    };

    let mut prepare_keys = Vec::new();
    for spell_name in &target_spells {
        if state.active_spell_key(spell_name, config).is_none() {
            prepare_keys.extend(orb_recipe(spell_name, config)?);
        }
    }

    Some(PlannedPrepPair {
        target_spells,
        prepare_keys,
    })
}

fn build_primary_combo_sequence(
    state: &InvokerObservedState,
    config: &crate::config::settings::InvokerConfig,
) -> Option<PlannedComboSequence> {
    let spells = match config.primary_profile.as_str() {
        "qw_pickoff" => vec!["invoker_tornado", "invoker_emp"],
        "qe_burst" => vec![
            "invoker_sun_strike",
            "invoker_chaos_meteor",
            "invoker_deafening_blast",
        ],
        _ => return None,
    };

    let _ = state;
    Some(PlannedComboSequence {
        spells,
        item_names: config.combo_items.clone(),
    })
}

#[derive(Debug, Clone)]
enum InvokerRequest {
    PrimaryCombo,
    PanicGhostWalk,
    PrepPair,
}

#[cfg(test)]
static TEST_OBSERVER: LazyLock<Mutex<Option<mpsc::Sender<InvokerRequest>>>> =
    LazyLock::new(|| Mutex::new(None));

#[cfg(test)]
pub(crate) fn set_test_observer(tx: mpsc::Sender<InvokerRequest>) {
    *TEST_OBSERVER.lock().unwrap() = Some(tx);
}

#[cfg(test)]
pub(crate) fn clear_test_observer() {
    *TEST_OBSERVER.lock().unwrap() = None;
}

fn run_invoker_request(request: InvokerRequest) {
    #[cfg(test)]
    {
        if let Some(observer) = TEST_OBSERVER.lock().unwrap().as_ref() {
            observer.send(request.clone()).ok();
        }
    }
    info!("Running Invoker request: {:?}", request);

    let event = INVOKER_LAST_EVENT.lock().unwrap().clone();
    let settings = INVOKER_SETTINGS.lock().unwrap().clone();

    let Some(event) = event else {
        info!("🔮 Invoker request skipped: no GSI event available");
        return;
    };

    let Some(settings) = settings else {
        info!("🔮 Invoker request skipped: no settings available");
        return;
    };

    let state = InvokerObservedState::from_event(&event);
    let config = &settings.heroes.invoker;

    if !state.hero_alive || state.hero_disabled {
        info!("🔮 Invoker request skipped: hero not available");
        return;
    }

    match request {
        InvokerRequest::PanicGhostWalk => run_panic_ghost_walk(&state, config),
        InvokerRequest::PrepPair => run_prep_pair(&state, config),
        InvokerRequest::PrimaryCombo => run_primary_combo(&event, &settings, &state, config),
    }
}

fn run_panic_ghost_walk(
    state: &InvokerObservedState,
    config: &crate::config::settings::InvokerConfig,
) {
    info!("🔮 Invoker: Panic Ghost Walk");

    let Some(plan) = plan_single_spell("invoker_ghost_walk", state, config) else {
        info!("🔮 Ghost Walk not plannable");
        return;
    };

    for &key in &plan.prepare_keys {
        crate::input::simulation::press_key(key);
        thread::sleep(Duration::from_millis(10));
    }

    if !plan.prepare_keys.is_empty() {
        thread::sleep(Duration::from_millis(50));
    }

    crate::input::simulation::press_key(plan.cast_key);
    info!("🔮 Ghost Walk cast complete");
}

fn run_prep_pair(
    state: &InvokerObservedState,
    config: &crate::config::settings::InvokerConfig,
) {
    info!("🔮 Invoker: Prep Pair - {}", config.prep_profile);

    let Some(plan) = plan_prep_profile(&config.prep_profile, state, config) else {
        info!("🔮 Prep profile {} not recognized", config.prep_profile);
        return;
    };

    for &key in &plan.prepare_keys {
        crate::input::simulation::press_key(key);
        thread::sleep(Duration::from_millis(10));
    }

    info!("🔮 Prep pair complete: {:?}", plan.target_spells);
}

fn run_primary_combo(
    event: &GsiWebhookEvent,
    settings: &Settings,
    state: &InvokerObservedState,
    config: &crate::config::settings::InvokerConfig,
) {
    info!("🔮 Invoker: Primary Combo - {}", config.primary_profile);

    let Some(sequence) = build_primary_combo_sequence(state, config) else {
        info!("🔮 Primary profile {} not recognized", config.primary_profile);
        return;
    };

    // Use configured combo items before spell sequence
    for item_name in &sequence.item_names {
        if let Some(key) = find_item_slot_by_name(event, settings, item_name) {
            info!("🔮 Using combo item: {}", item_name);
            crate::input::simulation::press_key(key);
            thread::sleep(Duration::from_millis(50));
        } else {
            info!("🔮 Combo item {} not found, skipping", item_name);
        }
    }

    // Track active spells through the combo to handle mid-combo invokes correctly
    let mut current_active_spells = state.active_spells.clone();

    for spell_name in &sequence.spells {
        // Build a fresh state view with current slot tracking
        let current_state = InvokerObservedState {
            quas_level: state.quas_level,
            wex_level: state.wex_level,
            exort_level: state.exort_level,
            invoke_ready: state.invoke_ready,
            active_spells: current_active_spells.clone(),
            hero_alive: state.hero_alive,
            hero_disabled: state.hero_disabled,
            has_scepter: state.has_scepter,
            has_shard: state.has_shard,
        };

        let Some(plan) = plan_single_spell(spell_name, &current_state, config) else {
            info!("🔮 Spell {} not plannable, skipping", spell_name);
            continue;
        };

        for &key in &plan.prepare_keys {
            crate::input::simulation::press_key(key);
            thread::sleep(Duration::from_millis(10));
        }

        if !plan.prepare_keys.is_empty() {
            thread::sleep(Duration::from_millis(50));

            // Update slot tracking: invoke shifts secondary to primary, new spell to secondary
            current_active_spells[0] = current_active_spells[1].clone();
            current_active_spells[1] = Some(spell_name.to_string());
        }

        crate::input::simulation::press_key(plan.cast_key);
        info!("🔮 Cast {}", spell_name);

        let delay = match (config.primary_profile.as_str(), *spell_name) {
            ("qw_pickoff", "invoker_tornado") => config.tornado_emp_delay_ms,
            ("qe_burst", "invoker_sun_strike") => config.sun_strike_delay_ms,
            ("qe_burst", "invoker_chaos_meteor") => config.meteor_blast_delay_ms,
            _ => 100,
        };

        thread::sleep(Duration::from_millis(delay));
    }

    info!("🔮 Primary combo complete");
}

/// Worker loop that processes requests from the queue in FIFO order.
/// This is the core queue processing logic used by the production INVOKER_REQUEST_QUEUE.
#[cfg(not(test))]
fn process_request_queue(rx: mpsc::Receiver<InvokerRequest>) {
    while let Ok(request) = rx.recv() {
        run_invoker_request(request);
    }
}

/// Test-friendly version that uses recv_timeout to avoid hanging tests.
/// Uses the same recv() → run_invoker_request() pattern as production.
#[cfg(test)]
fn process_request_queue_with_timeout(
    rx: mpsc::Receiver<InvokerRequest>,
    timeout: std::time::Duration,
) {
    while let Ok(request) = rx.recv_timeout(timeout) {
        run_invoker_request(request);
    }
}

fn enqueue_request(request: InvokerRequest) {
    if let Err(e) = INVOKER_REQUEST_QUEUE.send(request) {
        info!("Invoker request queue closed: {:?}", e);
    }
}

static INVOKER_REQUEST_QUEUE: LazyLock<mpsc::Sender<InvokerRequest>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<InvokerRequest>();
    thread::spawn(move || {
        info!("🔮 Invoker request worker started");

        #[cfg(not(test))]
        process_request_queue(rx);
        #[cfg(test)]
        {
            // In test builds, use timeout version to prevent hanging
            use std::time::Duration;
            process_request_queue_with_timeout(rx, Duration::from_secs(60));
        }

        info!("🔮 Invoker request worker exited");
    });
    tx
});

pub struct InvokerScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl InvokerScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }

    pub fn handle_panic_trigger(&self) {
        enqueue_request(InvokerRequest::PanicGhostWalk);
    }

    pub fn handle_prep_trigger(&self) {
        enqueue_request(InvokerRequest::PrepPair);
    }
}

impl HeroScript for InvokerScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // Store latest event and settings for request worker
        *INVOKER_LAST_EVENT.lock().unwrap() = Some(event.clone());
        *INVOKER_SETTINGS.lock().unwrap() = Some(self.settings.lock().unwrap().clone());

        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        let settings = self.settings.lock().unwrap();
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    fn handle_standalone_trigger(&self) {
        enqueue_request(InvokerRequest::PrimaryCombo);
    }

    fn hero_name(&self) -> &'static str {
        Hero::Invoker.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invoker_qw_fixture() -> GsiWebhookEvent {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/invoker_qw_event.json"
        ))
        .expect("Invoker QW fixture should deserialize")
    }

    #[test]
    fn observed_state_reads_orb_levels_and_active_spells() {
        let event = invoker_qw_fixture();
        let state = InvokerObservedState::from_event(&event);

        assert_eq!(state.quas_level, 4);
        assert_eq!(state.wex_level, 4);
        assert_eq!(state.exort_level, 1);
        assert_eq!(state.active_spells[0].as_deref(), Some("invoker_tornado"));
        assert_eq!(state.active_spells[1].as_deref(), Some("invoker_emp"));
    }

    #[test]
    fn active_spell_key_uses_existing_slot_mapping() {
        let event = invoker_qw_fixture();
        let config = Settings::default().heroes.invoker;
        let state = InvokerObservedState::from_event(&event);

        assert_eq!(
            state.active_spell_key("invoker_emp", &config),
            Some(config.spell_slot_secondary_key)
        );
    }

    #[test]
    fn planner_uses_existing_slot_when_spell_is_already_active() {
        let event = invoker_qw_fixture();
        let settings = Settings::default();
        let state = InvokerObservedState::from_event(&event);

        let step = plan_single_spell("invoker_tornado", &state, &settings.heroes.invoker)
            .expect("spell should be plannable");

        assert_eq!(step.cast_key, settings.heroes.invoker.spell_slot_primary_key);
        assert!(step.prepare_keys.is_empty());
    }

    #[test]
    fn planner_prepares_meteor_when_not_currently_invoked() {
        let event = invoker_qw_fixture();
        let settings = Settings::default();
        let state = InvokerObservedState::from_event(&event);

        let step = plan_single_spell("invoker_chaos_meteor", &state, &settings.heroes.invoker)
            .expect("meteor should be plannable");

        assert_eq!(
            step.prepare_keys,
            vec![
                settings.heroes.invoker.exort_key,
                settings.heroes.invoker.exort_key,
                settings.heroes.invoker.wex_key,
                settings.heroes.invoker.invoke_key,
            ]
        );
    }

    #[test]
    fn prep_profile_returns_ordered_two_spell_plan() {
        let event = invoker_qw_fixture();
        let settings = Settings::default();
        let state = InvokerObservedState::from_event(&event);

        let plan = plan_prep_profile("meteor_blast", &state, &settings.heroes.invoker)
            .expect("prep plan should exist");

        assert_eq!(plan.target_spells, vec!["invoker_chaos_meteor", "invoker_deafening_blast"]);
    }

    #[test]
    fn production_queue_preserves_fifo_order() {
        use std::sync::{mpsc, Arc, Mutex};
        use std::thread;
        use std::time::Duration;

        // Create observer channel to watch what the real worker processes
        let (observe_tx, observe_rx) = mpsc::channel::<InvokerRequest>();
        set_test_observer(observe_tx);

        let observed = Arc::new(Mutex::new(Vec::new()));
        let observed_clone = Arc::clone(&observed);

        // Collector thread that records processing order
        let collector = thread::spawn(move || {
            while let Ok(request) = observe_rx.recv_timeout(Duration::from_millis(300)) {
                observed_clone.lock().unwrap().push(request);
            }
        });

        // Force initialization of the real production queue
        let _ = &*INVOKER_REQUEST_QUEUE;

        // Enqueue through the actual production path
        enqueue_request(InvokerRequest::PrepPair);
        enqueue_request(InvokerRequest::PanicGhostWalk);
        enqueue_request(InvokerRequest::PrimaryCombo);

        // Wait for worker to process
        thread::sleep(Duration::from_millis(200));
        clear_test_observer();

        collector.join().expect("collector should complete");

        // Verify the real production worker preserved FIFO order
        let received = observed.lock().unwrap();
        assert_eq!(received.len(), 3, "all three requests should be processed by production worker");
        assert!(matches!(received[0], InvokerRequest::PrepPair));
        assert!(matches!(received[1], InvokerRequest::PanicGhostWalk));
        assert!(matches!(received[2], InvokerRequest::PrimaryCombo));
    }

    fn invoker_qe_fixture() -> GsiWebhookEvent {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/invoker_qe_event.json"
        ))
        .expect("Invoker QE fixture should deserialize")
    }

    #[test]
    fn qw_profile_expands_to_tornado_then_emp() {
        let event = invoker_qw_fixture();
        let settings = Settings::default();
        let state = InvokerObservedState::from_event(&event);

        let sequence = build_primary_combo_sequence(&state, &settings.heroes.invoker)
            .expect("QW profile should build");

        assert_eq!(sequence.spells, vec!["invoker_tornado", "invoker_emp"]);
    }

    #[test]
    fn qe_profile_expands_to_strike_meteor_blast() {
        let event = invoker_qe_fixture();
        let mut settings = Settings::default();
        settings.heroes.invoker.primary_profile = "qe_burst".to_string();
        let state = InvokerObservedState::from_event(&event);

        let sequence = build_primary_combo_sequence(&state, &settings.heroes.invoker)
            .expect("QE profile should build");

        assert_eq!(
            sequence.spells,
            vec![
                "invoker_sun_strike",
                "invoker_chaos_meteor",
                "invoker_deafening_blast"
            ]
        );
    }

    #[test]
    fn multi_spell_combo_tracks_slot_state_after_mid_combo_invoke() {
        let event = invoker_qe_fixture();
        let mut settings = Settings::default();
        settings.heroes.invoker.primary_profile = "qe_burst".to_string();
        let config = &settings.heroes.invoker;

        // Initial state: meteor in primary slot, blast in secondary slot
        let mut state = InvokerObservedState::from_event(&event);
        state.active_spells = [
            Some("invoker_chaos_meteor".to_string()),
            Some("invoker_deafening_blast".to_string()),
        ];

        // QE burst sequence: [sun_strike, meteor, blast]
        // Expected behavior:
        // 1. Sun Strike: not active, must invoke → moves to secondary slot
        //    After invoke: slots become [blast, sun_strike] (old secondary shifts to primary)
        // 2. Meteor: no longer in any slot after Sun Strike invoke → must re-invoke
        // 3. Blast: now in primary slot (shifted during Sun Strike invoke) → press primary key

        // Plan Sun Strike (first spell)
        let sun_strike_plan = plan_single_spell("invoker_sun_strike", &state, config)
            .expect("sun strike should be plannable");
        assert!(!sun_strike_plan.prepare_keys.is_empty(), "sun strike should require invoke");
        assert_eq!(sun_strike_plan.cast_key, config.spell_slot_secondary_key);

        // Simulate the invoke effect: old secondary shifts to primary, Sun Strike to secondary
        let mut current_active_spells = state.active_spells.clone();
        current_active_spells[0] = current_active_spells[1].clone();
        current_active_spells[1] = Some("invoker_sun_strike".to_string());

        // Plan Meteor (second spell) with updated state
        let state_after_sun_strike = InvokerObservedState {
            quas_level: state.quas_level,
            wex_level: state.wex_level,
            exort_level: state.exort_level,
            invoke_ready: state.invoke_ready,
            active_spells: current_active_spells.clone(),
            hero_alive: state.hero_alive,
            hero_disabled: state.hero_disabled,
            has_scepter: state.has_scepter,
            has_shard: state.has_shard,
        };
        let meteor_plan = plan_single_spell("invoker_chaos_meteor", &state_after_sun_strike, config)
            .expect("meteor should be plannable");
        assert!(!meteor_plan.prepare_keys.is_empty(), "meteor should require invoke after being displaced");
        assert_eq!(meteor_plan.cast_key, config.spell_slot_secondary_key);

        // Plan Blast (third spell) with updated state
        // CRITICAL: Blast is now in primary slot (shifted from secondary during Sun Strike invoke)
        let blast_plan = plan_single_spell("invoker_deafening_blast", &state_after_sun_strike, config)
            .expect("blast should be plannable");
        assert!(blast_plan.prepare_keys.is_empty(), "blast should already be active in primary");
        assert_eq!(blast_plan.cast_key, config.spell_slot_primary_key, "blast should use primary slot");

        // This test verifies the fix: without slot tracking, meteor_plan would incorrectly
        // think Meteor is still in the primary slot and press D without invoking,
        // casting Blast instead.
    }
}
