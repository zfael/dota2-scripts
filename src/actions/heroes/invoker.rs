use crate::actions::common::{find_item_slot_by_name, SurvivabilityActions};
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::traits::HeroScript;
use crate::config::settings::{InvokerProfile, InvokerProfileMode, InvokerProfileStepKind};
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
enum PlannedInvokerAction {
    Item {
        target: String,
        delay_after_ms: u64,
    },
    Spell {
        target: String,
        prepare_keys: Vec<char>,
        cast_key: char,
        delay_after_ms: u64,
        should_cast: bool,
    },
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

fn find_profile<'a>(
    config: &'a crate::config::settings::InvokerConfig,
    profile_id: &str,
) -> Option<&'a InvokerProfile> {
    config.profiles.iter().find(|profile| profile.id == profile_id)
}

fn first_enabled_combo_profile_id(
    config: &crate::config::settings::InvokerConfig,
) -> Option<String> {
    config
        .profiles
        .iter()
        .find(|profile| profile.enabled && profile.mode == InvokerProfileMode::Combo)
        .map(|profile| profile.id.clone())
}

fn build_profile_execution_plan(
    profile: &InvokerProfile,
    state: &InvokerObservedState,
    config: &crate::config::settings::InvokerConfig,
) -> Option<Vec<PlannedInvokerAction>> {
    let mut actions = Vec::new();
    let mut current_active_spells = state.active_spells.clone();

    for step in &profile.steps {
        match step.kind {
            InvokerProfileStepKind::Item => actions.push(PlannedInvokerAction::Item {
                target: step.target.clone(),
                delay_after_ms: step.delay_after_ms,
            }),
            InvokerProfileStepKind::Spell => {
                let current_state = InvokerObservedState {
                    active_spells: current_active_spells.clone(),
                    ..state.clone()
                };
                let cast_plan = plan_single_spell(&step.target, &current_state, config)?;

                if !cast_plan.prepare_keys.is_empty() {
                    current_active_spells[0] = current_active_spells[1].clone();
                    current_active_spells[1] = Some(step.target.clone());
                }

                actions.push(PlannedInvokerAction::Spell {
                    target: step.target.clone(),
                    prepare_keys: cast_plan.prepare_keys,
                    cast_key: cast_plan.cast_key,
                    delay_after_ms: step.delay_after_ms,
                    should_cast: profile.mode == InvokerProfileMode::Combo,
                });
            }
        }
    }

    Some(actions)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InvokerRequest {
    RunProfile(String),
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

    let InvokerRequest::RunProfile(profile_id) = request;

    let Some(profile) = find_profile(config, &profile_id) else {
        info!("🔮 Invoker request skipped: profile {} not found", profile_id);
        return;
    };

    run_profile(&event, &settings, &state, config, profile);
}

fn run_profile(
    event: &GsiWebhookEvent,
    settings: &Settings,
    state: &InvokerObservedState,
    config: &crate::config::settings::InvokerConfig,
    profile: &InvokerProfile,
) {
    let Some(plan) = build_profile_execution_plan(profile, state, config) else {
        info!("🔮 Invoker profile {} could not be planned", profile.id);
        return;
    };

    info!("🔮 Invoker profile: {} ({})", profile.name, profile.mode.as_str());
    info!("🔮 Planned steps: {:?}", profile.steps);

    let mut current_active_spells = state.active_spells.clone();

    for action in plan {
        match action {
            PlannedInvokerAction::Item {
                target,
                delay_after_ms,
            } => {
                if profile.mode == InvokerProfileMode::Prep {
                    info!("🔮 Skipping prep item step: {}", target);
                    continue;
                }

                if let Some(key) = find_item_slot_by_name(event, settings, &target) {
                    info!("🔮 Using combo item: {}", target);
                    crate::input::simulation::press_key(key);
                } else {
                    info!("🔮 Combo item {} not found, skipping", target);
                }

                thread::sleep(Duration::from_millis(delay_after_ms));
            }
            PlannedInvokerAction::Spell {
                target,
                prepare_keys,
                cast_key,
                delay_after_ms,
                should_cast,
            } => {
                info!("🔮 Active slots before step: {:?}", current_active_spells);

                for &key in &prepare_keys {
                    crate::input::simulation::press_key(key);
                    thread::sleep(Duration::from_millis(10));
                }

                if !prepare_keys.is_empty() {
                    thread::sleep(Duration::from_millis(50));
                    current_active_spells[0] = current_active_spells[1].clone();
                    current_active_spells[1] = Some(target.clone());
                    info!("🔮 Active slots after invoke: {:?}", current_active_spells);
                }

                if should_cast {
                    info!("🔮 Casting {} from {}", target, cast_key);
                    crate::input::simulation::press_key(cast_key);
                } else {
                    info!("🔮 Prepared {} without casting", target);
                }

                thread::sleep(Duration::from_millis(delay_after_ms));
            }
        }
    }

    info!("🔮 Invoker profile complete: {}", profile.name);
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

    pub fn handle_profile_trigger(&self, profile_id: &str) {
        enqueue_request(InvokerRequest::RunProfile(profile_id.to_string()));
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
        let settings = self.settings.lock().unwrap();
        if let Some(profile_id) = first_enabled_combo_profile_id(&settings.heroes.invoker) {
            enqueue_request(InvokerRequest::RunProfile(profile_id));
        } else {
            info!("🔮 Invoker standalone trigger skipped: no enabled combo profile");
        }
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
        let profile = find_profile(&settings.heroes.invoker, "meteor-blast-prep")
            .expect("prep profile should exist");

        let plan = build_profile_execution_plan(profile, &state, &settings.heroes.invoker)
            .expect("prep plan should exist");

        assert_eq!(
            plan.iter()
                .filter_map(|step| match step {
                    PlannedInvokerAction::Spell { target, .. } => Some(target.as_str()),
                    PlannedInvokerAction::Item { .. } => None,
                })
                .collect::<Vec<_>>(),
            vec!["invoker_chaos_meteor", "invoker_deafening_blast"]
        );
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
        enqueue_request(InvokerRequest::RunProfile("meteor-blast-prep".to_string()));
        enqueue_request(InvokerRequest::RunProfile("ghost-walk-panic".to_string()));
        enqueue_request(InvokerRequest::RunProfile("qw-pickoff".to_string()));

        // Wait for worker to process
        thread::sleep(Duration::from_millis(200));
        clear_test_observer();

        collector.join().expect("collector should complete");

        // Verify the real production worker preserved FIFO order
        let received = observed.lock().unwrap();
        assert_eq!(received.len(), 3, "all three requests should be processed by production worker");
        assert_eq!(received[0], InvokerRequest::RunProfile("meteor-blast-prep".to_string()));
        assert_eq!(received[1], InvokerRequest::RunProfile("ghost-walk-panic".to_string()));
        assert_eq!(received[2], InvokerRequest::RunProfile("qw-pickoff".to_string()));
    }

    fn invoker_qe_fixture() -> GsiWebhookEvent {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/invoker_qe_event.json"
        ))
        .expect("Invoker QE fixture should deserialize")
    }

    #[test]
    fn invoker_profile_runner_keeps_declared_tornado_then_emp_order() {
        let event = invoker_qw_fixture();
        let settings = Settings::default();
        let state = InvokerObservedState::from_event(&event);
        let profile = find_profile(&settings.heroes.invoker, "qw-pickoff")
            .expect("QW profile should exist");

        let plan = build_profile_execution_plan(profile, &state, &settings.heroes.invoker)
            .expect("QW profile should build");

        assert_eq!(
            plan.iter()
                .filter_map(|step| match step {
                    PlannedInvokerAction::Spell { target, .. } => Some(target.as_str()),
                    PlannedInvokerAction::Item { .. } => None,
                })
                .collect::<Vec<_>>(),
            vec!["invoker_tornado", "invoker_emp"]
        );
    }

    #[test]
    fn qe_profile_expands_to_strike_meteor_blast() {
        let event = invoker_qe_fixture();
        let settings = Settings::default();
        let state = InvokerObservedState::from_event(&event);
        let profile = find_profile(&settings.heroes.invoker, "qe-burst")
            .expect("QE profile should exist");

        let sequence = build_profile_execution_plan(profile, &state, &settings.heroes.invoker)
            .expect("QE profile should build");

        assert_eq!(
            sequence
                .iter()
                .filter_map(|step| match step {
                    PlannedInvokerAction::Spell { target, .. } => Some(target.as_str()),
                    PlannedInvokerAction::Item { .. } => None,
                })
                .collect::<Vec<_>>(),
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
        let settings = Settings::default();
        let config = &settings.heroes.invoker;
        let profile = find_profile(config, "qe-burst").expect("QE profile should exist");

        // Initial state: meteor in primary slot, blast in secondary slot
        let mut state = InvokerObservedState::from_event(&event);
        state.active_spells = [
            Some("invoker_chaos_meteor".to_string()),
            Some("invoker_deafening_blast".to_string()),
        ];

        let plan = build_profile_execution_plan(profile, &state, config)
            .expect("QE profile should build");

        let spell_steps: Vec<_> = plan
            .iter()
            .filter_map(|step| match step {
                PlannedInvokerAction::Spell {
                    target,
                    prepare_keys,
                    cast_key,
                    ..
                } => Some((target.as_str(), prepare_keys.clone(), *cast_key)),
                PlannedInvokerAction::Item { .. } => None,
            })
            .collect();

        assert_eq!(spell_steps.len(), 3);
        assert_eq!(spell_steps[0].0, "invoker_sun_strike");
        assert!(!spell_steps[0].1.is_empty(), "sun strike should require invoke");
        assert_eq!(spell_steps[1].0, "invoker_chaos_meteor");
        assert!(
            !spell_steps[1].1.is_empty(),
            "meteor should require re-invoke after being displaced"
        );
        assert_eq!(spell_steps[2].0, "invoker_deafening_blast");
        assert!(
            !spell_steps[2].1.is_empty(),
            "blast should require a fresh invoke after meteor displaces it"
        );
        assert_eq!(
            spell_steps[2].2,
            config.spell_slot_secondary_key,
            "blast should cast from the newly invoked secondary slot"
        );
    }
}
