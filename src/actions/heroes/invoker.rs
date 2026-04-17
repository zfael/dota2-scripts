use crate::actions::common::{find_item_slot_by_name, SurvivabilityActions};
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::traits::HeroScript;
use crate::config::settings::{
    InvokerProfile, InvokerProfileMode, InvokerProfileStepCastBehavior,
    InvokerProfileStepCompletionMode, InvokerProfileStepKind,
};
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use std::time::Duration;
use tracing::info;

static INVOKER_LAST_EVENT: LazyLock<Mutex<Option<GsiWebhookEvent>>> =
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedSpellStep {
    target: String,
    prepare_keys: Vec<char>,
    prepared_slots_after_prepare: Option<[Option<String>; 2]>,
    cast_key: char,
    cast_behavior: InvokerProfileStepCastBehavior,
    delay_after_ms: u64,
    completion_mode: InvokerProfileStepCompletionMode,
    completion_timeout_ms: u64,
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
        prepared_slots_after_prepare: Option<[Option<String>; 2]>,
        cast_key: char,
        cast_behavior: InvokerProfileStepCastBehavior,
        delay_after_ms: u64,
        completion_mode: InvokerProfileStepCompletionMode,
        completion_timeout_ms: u64,
        should_cast: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CastSequenceAction {
    Press(char),
    AltDown,
    AltUp,
    SleepMs(u64),
}

fn cast_sequence_for_behavior(
    cast_key: char,
    cast_behavior: &InvokerProfileStepCastBehavior,
) -> Vec<CastSequenceAction> {
    match cast_behavior {
        InvokerProfileStepCastBehavior::Normal => vec![CastSequenceAction::Press(cast_key)],
        InvokerProfileStepCastBehavior::ManualWaitCooldown => Vec::new(),
        InvokerProfileStepCastBehavior::AltCast => vec![
            CastSequenceAction::AltDown,
            CastSequenceAction::Press(cast_key),
            CastSequenceAction::AltUp,
        ],
        InvokerProfileStepCastBehavior::DoubleTap => vec![
            CastSequenceAction::Press(cast_key),
            CastSequenceAction::SleepMs(50),
            CastSequenceAction::Press(cast_key),
        ],
        InvokerProfileStepCastBehavior::AltDoubleTap => vec![
            CastSequenceAction::AltDown,
            CastSequenceAction::Press(cast_key),
            CastSequenceAction::SleepMs(50),
            CastSequenceAction::Press(cast_key),
            CastSequenceAction::AltUp,
        ],
    }
}

fn execute_cast_sequence(sequence: &[CastSequenceAction]) {
    for action in sequence {
        match action {
            CastSequenceAction::Press(key) => crate::input::simulation::press_key(*key),
            CastSequenceAction::AltDown => crate::input::simulation::alt_down(),
            CastSequenceAction::AltUp => crate::input::simulation::alt_up(),
            CastSequenceAction::SleepMs(duration_ms) => {
                thread::sleep(Duration::from_millis(*duration_ms))
            }
        }
    }
}

fn effective_completion_mode(
    completion_mode: &InvokerProfileStepCompletionMode,
    cast_behavior: &InvokerProfileStepCastBehavior,
) -> InvokerProfileStepCompletionMode {
    match cast_behavior {
        InvokerProfileStepCastBehavior::ManualWaitCooldown => {
            InvokerProfileStepCompletionMode::WaitForCooldown
        }
        _ => completion_mode.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CooldownWaitOutcome {
    Started,
    TimedOut,
    HeroUnavailable,
    SpellNotObserved,
}

fn orb_recipe(
    spell_name: &str,
    config: &crate::config::settings::InvokerConfig,
) -> Option<[char; 4]> {
    match spell_name {
        "invoker_tornado" => Some([
            config.wex_key,
            config.wex_key,
            config.quas_key,
            config.invoke_key,
        ]),
        "invoker_emp" => Some([
            config.wex_key,
            config.wex_key,
            config.wex_key,
            config.invoke_key,
        ]),
        "invoker_chaos_meteor" => Some([
            config.exort_key,
            config.exort_key,
            config.wex_key,
            config.invoke_key,
        ]),
        "invoker_deafening_blast" => Some([
            config.quas_key,
            config.wex_key,
            config.exort_key,
            config.invoke_key,
        ]),
        "invoker_cold_snap" => Some([
            config.quas_key,
            config.quas_key,
            config.quas_key,
            config.invoke_key,
        ]),
        "invoker_forge_spirit" => Some([
            config.exort_key,
            config.exort_key,
            config.quas_key,
            config.invoke_key,
        ]),
        "invoker_ghost_walk" => Some([
            config.quas_key,
            config.quas_key,
            config.wex_key,
            config.invoke_key,
        ]),
        "invoker_ice_wall" => Some([
            config.quas_key,
            config.quas_key,
            config.exort_key,
            config.invoke_key,
        ]),
        "invoker_sun_strike" => Some([
            config.exort_key,
            config.exort_key,
            config.exort_key,
            config.invoke_key,
        ]),
        _ => None,
    }
}

fn apply_invoke_to_slot_state(
    slots: &[Option<String>; 2],
    spell_name: &str,
) -> [Option<String>; 2] {
    [Some(spell_name.to_string()), slots[0].clone()]
}

fn spell_cast_key_from_slots(
    slots: &[Option<String>; 2],
    spell_name: &str,
    config: &crate::config::settings::InvokerConfig,
) -> Option<char> {
    if slots[0].as_deref() == Some(spell_name) {
        Some(config.spell_slot_primary_key)
    } else if slots[1].as_deref() == Some(spell_name) {
        Some(config.spell_slot_secondary_key)
    } else {
        None
    }
}

fn build_spell_batch(
    steps: &[crate::config::settings::InvokerProfileStep],
    starting_slots: &[Option<String>; 2],
    config: &crate::config::settings::InvokerConfig,
) -> Option<(Vec<PreparedSpellStep>, [Option<String>; 2], usize)> {
    let consumed = steps.len().min(2);
    if consumed == 0 {
        return None;
    }

    if consumed == 1 {
        let step = &steps[0];
        let mut current_slots = starting_slots.clone();
        let prepare_keys =
            if spell_cast_key_from_slots(&current_slots, &step.target, config).is_some() {
                Vec::new()
            } else {
                let keys = orb_recipe(&step.target, config)?.to_vec();
                current_slots = apply_invoke_to_slot_state(&current_slots, &step.target);
                keys
            };

        return Some((
            vec![PreparedSpellStep {
                target: step.target.clone(),
                prepare_keys,
                prepared_slots_after_prepare: if current_slots == *starting_slots {
                    None
                } else {
                    Some(current_slots.clone())
                },
                cast_key: spell_cast_key_from_slots(&current_slots, &step.target, config)?,
                cast_behavior: step.cast_behavior.clone(),
                delay_after_ms: step.delay_after_ms,
                completion_mode: step.completion_mode.clone(),
                completion_timeout_ms: step.completion_timeout_ms,
            }],
            current_slots,
            1,
        ));
    }

    let first = &steps[0];
    let second = &steps[1];
    let mut current_slots = starting_slots.clone();
    let desired_slots = [Some(second.target.clone()), Some(first.target.clone())];
    let mut preload_keys = Vec::new();

    if current_slots != desired_slots {
        if current_slots[0].as_deref() == Some(first.target.as_str()) {
            preload_keys.extend(orb_recipe(&second.target, config)?);
            current_slots = apply_invoke_to_slot_state(&current_slots, &second.target);
        } else {
            preload_keys.extend(orb_recipe(&first.target, config)?);
            current_slots = apply_invoke_to_slot_state(&current_slots, &first.target);
            if current_slots != desired_slots {
                preload_keys.extend(orb_recipe(&second.target, config)?);
                current_slots = apply_invoke_to_slot_state(&current_slots, &second.target);
            }
        }
    }

    Some((
        vec![
            PreparedSpellStep {
                target: first.target.clone(),
                prepare_keys: preload_keys,
                prepared_slots_after_prepare: if current_slots == *starting_slots {
                    None
                } else {
                    Some(current_slots.clone())
                },
                cast_key: spell_cast_key_from_slots(&current_slots, &first.target, config)?,
                cast_behavior: first.cast_behavior.clone(),
                delay_after_ms: first.delay_after_ms,
                completion_mode: first.completion_mode.clone(),
                completion_timeout_ms: first.completion_timeout_ms,
            },
            PreparedSpellStep {
                target: second.target.clone(),
                prepare_keys: Vec::new(),
                prepared_slots_after_prepare: None,
                cast_key: spell_cast_key_from_slots(&current_slots, &second.target, config)?,
                cast_behavior: second.cast_behavior.clone(),
                delay_after_ms: second.delay_after_ms,
                completion_mode: second.completion_mode.clone(),
                completion_timeout_ms: second.completion_timeout_ms,
            },
        ],
        current_slots,
        2,
    ))
}

fn find_profile<'a>(
    config: &'a crate::config::settings::InvokerConfig,
    profile_id: &str,
) -> Option<&'a InvokerProfile> {
    config
        .profiles
        .iter()
        .find(|profile| profile.id == profile_id)
}

pub(crate) fn resolve_active_combo_profile_id(
    config: &crate::config::settings::InvokerConfig,
    active_profile_id: Option<&str>,
) -> Option<String> {
    active_profile_id
        .and_then(|profile_id| {
            config.profiles.iter().find(|profile| {
                profile.enabled
                    && profile.mode == InvokerProfileMode::Combo
                    && profile.id == profile_id
            })
        })
        .map(|profile| profile.id.clone())
        .or_else(|| {
            config
                .profiles
                .iter()
                .find(|profile| profile.enabled && profile.mode == InvokerProfileMode::Combo)
                .map(|profile| profile.id.clone())
        })
}

fn build_profile_execution_plan(
    profile: &InvokerProfile,
    state: &InvokerObservedState,
    config: &crate::config::settings::InvokerConfig,
) -> Option<Vec<PlannedInvokerAction>> {
    let mut actions = Vec::new();
    let mut current_active_spells = state.active_spells.clone();
    let mut index = 0usize;

    while index < profile.steps.len() {
        let step = &profile.steps[index];
        match step.kind {
            InvokerProfileStepKind::Item => actions.push(PlannedInvokerAction::Item {
                target: step.target.clone(),
                delay_after_ms: step.delay_after_ms,
            }),
            InvokerProfileStepKind::Spell => {
                let spell_slice: Vec<_> = profile.steps[index..]
                    .iter()
                    .take_while(|candidate| candidate.kind == InvokerProfileStepKind::Spell)
                    .cloned()
                    .collect();
                let (batch, next_slots, consumed) =
                    build_spell_batch(&spell_slice, &current_active_spells, config)?;

                for prepared in batch {
                    actions.push(PlannedInvokerAction::Spell {
                        target: prepared.target,
                        prepare_keys: prepared.prepare_keys,
                        prepared_slots_after_prepare: prepared.prepared_slots_after_prepare,
                        cast_key: prepared.cast_key,
                        cast_behavior: prepared.cast_behavior,
                        delay_after_ms: prepared.delay_after_ms,
                        completion_mode: prepared.completion_mode,
                        completion_timeout_ms: prepared.completion_timeout_ms,
                        should_cast: profile.mode == InvokerProfileMode::Combo,
                    });
                }
                current_active_spells = next_slots;
                index += consumed;
                continue;
            }
        }
        index += 1;
    }

    Some(actions)
}

fn spell_cooldown_in_event(event: &GsiWebhookEvent, spell_name: &str) -> Option<u32> {
    [4u8, 5u8]
        .into_iter()
        .filter_map(|index| event.abilities.get_by_index(index))
        .find(|ability| ability.name == spell_name)
        .map(|ability| ability.cooldown)
}

fn spell_is_on_cooldown(event: &GsiWebhookEvent, spell_name: &str) -> bool {
    matches!(spell_cooldown_in_event(event, spell_name), Some(cooldown) if cooldown > 0)
}

fn wait_for_spell_cooldown_start(
    spell_name: &str,
    timeout_ms: u64,
    poll_interval_ms: u64,
) -> CooldownWaitOutcome {
    let started_at = std::time::Instant::now();

    loop {
        let Some(event) = INVOKER_LAST_EVENT.lock().unwrap().clone() else {
            return CooldownWaitOutcome::SpellNotObserved;
        };

        if !event.hero.alive || event.hero.stunned || event.hero.hexed || event.hero.silenced {
            return CooldownWaitOutcome::HeroUnavailable;
        }

        if spell_is_on_cooldown(&event, spell_name) {
            return CooldownWaitOutcome::Started;
        }

        if started_at.elapsed() >= Duration::from_millis(timeout_ms) {
            return CooldownWaitOutcome::TimedOut;
        }

        thread::sleep(Duration::from_millis(poll_interval_ms));
    }
}

#[derive(Debug, Clone)]
pub(crate) enum InvokerRequest {
    RunProfile {
        profile_id: String,
        settings: Settings,
    },
}

fn build_run_profile_request(profile_id: &str, settings: &Settings) -> InvokerRequest {
    InvokerRequest::RunProfile {
        profile_id: profile_id.to_string(),
        settings: settings.clone(),
    }
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

    let Some(event) = event else {
        info!("🔮 Invoker request skipped: no GSI event available");
        return;
    };

    let InvokerRequest::RunProfile { profile_id, settings } = request;

    let state = InvokerObservedState::from_event(&event);
    let config = &settings.heroes.invoker;

    if !state.hero_alive || state.hero_disabled {
        info!("🔮 Invoker request skipped: hero not available");
        return;
    }

    let Some(profile) = find_profile(config, &profile_id) else {
        info!(
            "🔮 Invoker request skipped: profile {} not found",
            profile_id
        );
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

    info!(
        "🔮 Invoker profile: {} ({})",
        profile.name,
        profile.mode.as_str()
    );
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
                prepared_slots_after_prepare,
                cast_key,
                cast_behavior,
                delay_after_ms,
                completion_mode,
                completion_timeout_ms,
                should_cast,
            } => {
                info!("🔮 Active slots before step: {:?}", current_active_spells);

                for &key in &prepare_keys {
                    crate::input::simulation::press_key(key);
                    thread::sleep(Duration::from_millis(10));
                }

                if !prepare_keys.is_empty() {
                    thread::sleep(Duration::from_millis(50));
                    if let Some(prepared_slots) = prepared_slots_after_prepare.clone() {
                        current_active_spells = prepared_slots;
                    }
                    info!("🔮 Active slots after invoke: {:?}", current_active_spells);
                }

                if should_cast {
                    let effective_completion_mode =
                        effective_completion_mode(&completion_mode, &cast_behavior);
                    let current_event = INVOKER_LAST_EVENT
                        .lock()
                        .unwrap()
                        .clone()
                        .unwrap_or_else(|| event.clone());
                    if effective_completion_mode == InvokerProfileStepCompletionMode::WaitForCooldown
                        && spell_is_on_cooldown(&current_event, &target)
                    {
                        info!("🔮 Manual step {} already on cooldown, skipping", target);
                        continue;
                    }

                    let cast_sequence = cast_sequence_for_behavior(cast_key, &cast_behavior);
                    if cast_sequence.is_empty() {
                        info!(
                            "🔮 Prepared {} without auto-casting; waiting for manual completion",
                            target
                        );
                    } else {
                        info!("🔮 Casting {} from {} via {:?}", target, cast_key, cast_behavior);
                        execute_cast_sequence(&cast_sequence);
                    }

                    if effective_completion_mode == InvokerProfileStepCompletionMode::WaitForCooldown
                    {
                        info!("🔮 Waiting for {} cooldown to start", target);
                        match wait_for_spell_cooldown_start(&target, completion_timeout_ms, 25) {
                            CooldownWaitOutcome::Started => {
                                info!("🔮 {} entered cooldown; continuing profile", target);
                            }
                            CooldownWaitOutcome::TimedOut => {
                                info!(
                                    "🔮 Manual step {} timed out after {}ms; aborting profile",
                                    target, completion_timeout_ms
                                );
                                break;
                            }
                            CooldownWaitOutcome::HeroUnavailable => {
                                info!(
                                    "🔮 Hero unavailable while waiting for {}; aborting profile",
                                    target
                                );
                                break;
                            }
                            CooldownWaitOutcome::SpellNotObserved => {
                                info!(
                                    "🔮 Could not observe {} while waiting for cooldown; aborting profile",
                                    target
                                );
                                break;
                            }
                        }
                    }
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
    app_state: Arc<Mutex<crate::state::AppState>>,
}

impl InvokerScript {
    pub fn new(
        settings: Arc<Mutex<Settings>>,
        executor: Arc<ActionExecutor>,
        app_state: Arc<Mutex<crate::state::AppState>>,
    ) -> Self {
        Self {
            settings,
            executor,
            app_state,
        }
    }

    pub fn handle_profile_trigger(&self, profile_id: &str) {
        let settings = self.settings.lock().unwrap();
        enqueue_request(build_run_profile_request(profile_id, &settings));
    }
}

impl HeroScript for InvokerScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // Store latest event for request worker
        *INVOKER_LAST_EVENT.lock().unwrap() = Some(event.clone());

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
        let active_profile_id = self
            .app_state
            .lock()
            .unwrap()
            .invoker_active_combo_profile_id
            .as_deref()
            .map(|s| s.to_string());
        if let Some(profile_id) = resolve_active_combo_profile_id(
            &settings.heroes.invoker,
            active_profile_id.as_deref(),
        ) {
            enqueue_request(build_run_profile_request(&profile_id, &settings));
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
    use std::sync::{LazyLock, Mutex};

    static MANUAL_WAIT_TEST_GUARD: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
    static QUEUE_TEST_GUARD: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

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
            spell_cast_key_from_slots(&state.active_spells, "invoker_emp", &config),
            Some(config.spell_slot_secondary_key)
        );
    }

    #[test]
    fn apply_invoke_to_slot_state_places_new_spell_in_primary_and_shifts_old_primary_to_secondary()
    {
        let slots = [
            Some("invoker_emp".to_string()),
            Some("invoker_tornado".to_string()),
        ];

        assert_eq!(
            apply_invoke_to_slot_state(&slots, "invoker_sun_strike"),
            [
                Some("invoker_sun_strike".to_string()),
                Some("invoker_emp".to_string()),
            ]
        );
    }

    #[test]
    fn resolve_active_combo_profile_id_prefers_explicit_enabled_combo_profile() {
        let mut config = Settings::default().heroes.invoker;
        let qe_burst = config
            .profiles
            .iter_mut()
            .find(|profile| profile.id == "qe-burst")
            .expect("QE Burst profile should exist");
        qe_burst.enabled = true;

        assert_eq!(
            resolve_active_combo_profile_id(&config, Some("qe-burst")),
            Some("qe-burst".to_string())
        );
    }

    #[test]
    fn resolve_active_combo_profile_id_repairs_invalid_active_profile_with_first_enabled_combo() {
        let mut config = Settings::default().heroes.invoker;
        config
            .profiles
            .iter_mut()
            .find(|profile| profile.id == "qw-pickoff")
            .expect("QW Pickoff profile should exist")
            .enabled = false;

        assert_eq!(
            resolve_active_combo_profile_id(&config, Some("meteor-blast-prep")),
            Some("ghost-walk-panic".to_string())
        );
    }

    #[test]
    fn build_spell_batch_for_qw_pickoff_casts_secondary_then_primary() {
        let event = invoker_qw_fixture();
        let settings = Settings::default();
        let config = &settings.heroes.invoker;
        let state = InvokerObservedState::from_event(&event);
        let profile = find_profile(config, "qw-pickoff").expect("QW profile should exist");

        let spell_steps: Vec<_> = profile
            .steps
            .iter()
            .filter(|step| step.kind == InvokerProfileStepKind::Spell)
            .cloned()
            .collect();

        let (batch, next_slots, consumed) =
            build_spell_batch(&spell_steps, &state.active_spells, config)
                .expect("QW spell batch should build");

        assert_eq!(consumed, 2);
        assert_eq!(
            batch
                .iter()
                .map(|step| (step.target.as_str(), step.cast_key))
                .collect::<Vec<_>>(),
            vec![
                ("invoker_tornado", config.spell_slot_secondary_key),
                ("invoker_emp", config.spell_slot_primary_key),
            ]
        );
        assert_eq!(
            next_slots,
            [
                Some("invoker_emp".to_string()),
                Some("invoker_tornado".to_string()),
            ]
        );
    }

    #[test]
    fn single_spell_batch_uses_existing_slot_when_spell_is_already_active() {
        let event = invoker_qw_fixture();
        let settings = Settings::default();
        let config = &settings.heroes.invoker;
        let state = InvokerObservedState::from_event(&event);
        let step = crate::config::settings::InvokerProfileStep {
            kind: InvokerProfileStepKind::Spell,
            target: "invoker_tornado".to_string(),
            delay_after_ms: 100,
            cast_behavior: crate::config::settings::InvokerProfileStepCastBehavior::Normal,
            completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
            completion_timeout_ms: 3000,
            notes: String::new(),
        };

        let (batch, _, consumed) = build_spell_batch(&[step], &state.active_spells, config)
            .expect("spell should be plannable");

        assert_eq!(consumed, 1);
        assert_eq!(batch[0].cast_key, config.spell_slot_primary_key);
        assert!(batch[0].prepare_keys.is_empty());
    }

    #[test]
    fn single_spell_batch_prepares_meteor_when_not_currently_invoked() {
        let event = invoker_qw_fixture();
        let settings = Settings::default();
        let config = &settings.heroes.invoker;
        let state = InvokerObservedState::from_event(&event);
        let step = crate::config::settings::InvokerProfileStep {
            kind: InvokerProfileStepKind::Spell,
            target: "invoker_chaos_meteor".to_string(),
            delay_after_ms: 100,
            cast_behavior: crate::config::settings::InvokerProfileStepCastBehavior::Normal,
            completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
            completion_timeout_ms: 3000,
            notes: String::new(),
        };

        let (batch, next_slots, consumed) =
            build_spell_batch(&[step], &state.active_spells, config)
                .expect("meteor should be plannable");

        assert_eq!(consumed, 1);
        assert_eq!(
            batch[0].prepare_keys,
            vec![
                config.exort_key,
                config.exort_key,
                config.wex_key,
                config.invoke_key,
            ]
        );
        assert_eq!(batch[0].cast_key, config.spell_slot_primary_key);
        assert_eq!(
            next_slots,
            [
                Some("invoker_chaos_meteor".to_string()),
                Some("invoker_tornado".to_string()),
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
    fn cast_sequence_for_alt_double_tap_wraps_double_press_with_alt() {
        assert_eq!(
            cast_sequence_for_behavior(
                'd',
                &crate::config::settings::InvokerProfileStepCastBehavior::AltDoubleTap,
            ),
            vec![
                CastSequenceAction::AltDown,
                CastSequenceAction::Press('d'),
                CastSequenceAction::SleepMs(50),
                CastSequenceAction::Press('d'),
                CastSequenceAction::AltUp,
            ]
        );
    }

    #[test]
    fn cast_sequence_for_manual_wait_cooldown_is_empty() {
        assert_eq!(
            cast_sequence_for_behavior(
                'd',
                &crate::config::settings::InvokerProfileStepCastBehavior::ManualWaitCooldown,
            ),
            Vec::<CastSequenceAction>::new()
        );
    }

    #[test]
    fn production_queue_preserves_fifo_order() {
        let _guard = QUEUE_TEST_GUARD.lock().unwrap();
        use std::sync::{mpsc, Arc, Mutex};
        use std::thread;
        use std::time::Duration;

        // Set up required GSI event state for worker
        *INVOKER_LAST_EVENT.lock().unwrap() = Some(invoker_qw_fixture());

        // Create observer channel to watch what the real worker processes
        let (observe_tx, observe_rx) = mpsc::channel::<InvokerRequest>();
        
        let observed = Arc::new(Mutex::new(Vec::new()));
        let observed_clone = Arc::clone(&observed);

        set_test_observer(observe_tx);

        // Force initialization of the real production queue
        let _ = &*INVOKER_REQUEST_QUEUE;

        // Enqueue through the actual production path
        let settings = Settings::default();
        enqueue_request(build_run_profile_request("meteor-blast-prep", &settings));
        enqueue_request(build_run_profile_request("ghost-walk-panic", &settings));
        enqueue_request(build_run_profile_request("qw-pickoff", &settings));

        // Collector thread that records processing order - collect exactly 3 requests
        let collector = thread::spawn(move || {
            for _ in 0..3 {
                match observe_rx.recv_timeout(Duration::from_secs(5)) {
                    Ok(request) => {
                        observed_clone.lock().unwrap().push(request);
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        });

        collector.join().expect("collector should complete");
        
        // Clear observer now that we've collected all messages
        clear_test_observer();

        // Verify the real production worker preserved FIFO order
        let received = observed.lock().unwrap();
        assert_eq!(
            received.len(),
            3,
            "all three requests should be processed by production worker"
        );
        match &received[0] {
            InvokerRequest::RunProfile { profile_id, .. } => {
                assert_eq!(profile_id, "meteor-blast-prep");
            }
        }
        match &received[1] {
            InvokerRequest::RunProfile { profile_id, .. } => {
                assert_eq!(profile_id, "ghost-walk-panic");
            }
        }
        match &received[2] {
            InvokerRequest::RunProfile { profile_id, .. } => {
                assert_eq!(profile_id, "qw-pickoff");
            }
        }
    }

    fn invoker_qe_fixture() -> GsiWebhookEvent {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/invoker_qe_event.json"
        ))
        .expect("Invoker QE fixture should deserialize")
    }

    #[test]
    fn build_profile_execution_plan_for_qw_pickoff_keeps_tornado_then_emp_order() {
        let event = invoker_qw_fixture();
        let settings = Settings::default();
        let config = &settings.heroes.invoker;
        let state = InvokerObservedState::from_event(&event);
        let profile = find_profile(config, "qw-pickoff").expect("QW profile should exist");

        let plan = build_profile_execution_plan(profile, &state, config)
            .expect("QW execution plan should build");

        let planned_spells: Vec<_> = plan
            .iter()
            .filter_map(|action| match action {
                PlannedInvokerAction::Spell {
                    target,
                    cast_key,
                    completion_mode,
                    ..
                } => Some((target.as_str(), *cast_key, completion_mode.clone())),
                PlannedInvokerAction::Item { .. } => None,
            })
            .collect();

        assert_eq!(
            planned_spells,
            vec![
                (
                    "invoker_tornado",
                    config.spell_slot_secondary_key,
                    InvokerProfileStepCompletionMode::FixedDelay,
                ),
                (
                    "invoker_emp",
                    config.spell_slot_primary_key,
                    InvokerProfileStepCompletionMode::FixedDelay,
                ),
            ]
        );
    }

    #[test]
    fn build_profile_execution_plan_for_qe_burst_preloads_first_pair_then_trailing_primary() {
        let event = invoker_qe_fixture();
        let settings = Settings::default();
        let config = &settings.heroes.invoker;
        let state = InvokerObservedState::from_event(&event);
        let profile = find_profile(config, "qe-burst").expect("QE profile should exist");

        let plan = build_profile_execution_plan(profile, &state, config)
            .expect("QE execution plan should build");

        let planned_spells: Vec<_> = plan
            .iter()
            .filter_map(|action| match action {
                PlannedInvokerAction::Spell {
                    target,
                    cast_key,
                    completion_mode,
                    ..
                } => Some((target.as_str(), *cast_key, completion_mode.clone())),
                PlannedInvokerAction::Item { .. } => None,
            })
            .collect();

        assert_eq!(
            planned_spells,
            vec![
                (
                    "invoker_sun_strike",
                    config.spell_slot_secondary_key,
                    InvokerProfileStepCompletionMode::WaitForCooldown,
                ),
                (
                    "invoker_chaos_meteor",
                    config.spell_slot_primary_key,
                    InvokerProfileStepCompletionMode::FixedDelay,
                ),
                (
                    "invoker_deafening_blast",
                    config.spell_slot_primary_key,
                    InvokerProfileStepCompletionMode::FixedDelay,
                ),
            ]
        );
    }

    #[test]
    fn build_profile_execution_plan_for_qe_burst_carries_manual_wait_cast_behavior() {
        let event = invoker_qe_fixture();
        let settings = Settings::default();
        let config = &settings.heroes.invoker;
        let state = InvokerObservedState::from_event(&event);
        let profile = find_profile(config, "qe-burst").expect("QE profile should exist");

        let plan = build_profile_execution_plan(profile, &state, config)
            .expect("QE execution plan should build");

        let first_spell = plan
            .iter()
            .find_map(|action| match action {
                PlannedInvokerAction::Spell { cast_behavior, .. } => Some(cast_behavior.clone()),
                PlannedInvokerAction::Item { .. } => None,
            })
            .expect("QE profile should include a spell step");

        assert_eq!(
            first_spell,
            crate::config::settings::InvokerProfileStepCastBehavior::ManualWaitCooldown
        );
    }

    #[test]
    fn invoker_profile_runner_keeps_declared_tornado_then_emp_order() {
        let event = invoker_qw_fixture();
        let settings = Settings::default();
        let state = InvokerObservedState::from_event(&event);
        let profile =
            find_profile(&settings.heroes.invoker, "qw-pickoff").expect("QW profile should exist");

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
        let profile =
            find_profile(&settings.heroes.invoker, "qe-burst").expect("QE profile should exist");

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

        let plan =
            build_profile_execution_plan(profile, &state, config).expect("QE profile should build");

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
        assert!(
            !spell_steps[0].1.is_empty(),
            "sun strike should require invoke"
        );
        assert_eq!(
            spell_steps[0].2, config.spell_slot_secondary_key,
            "sun strike should cast from the older prepared slot"
        );
        assert_eq!(spell_steps[1].0, "invoker_chaos_meteor");
        assert!(
            spell_steps[1].1.is_empty(),
            "meteor should already be loaded after the first pair is prepped"
        );
        assert_eq!(
            spell_steps[1].2, config.spell_slot_primary_key,
            "meteor should cast from the newer prepared primary slot"
        );
        assert_eq!(spell_steps[2].0, "invoker_deafening_blast");
        assert!(
            !spell_steps[2].1.is_empty(),
            "blast should require a fresh invoke after the first pair completes"
        );
        assert_eq!(
            spell_steps[2].2, config.spell_slot_primary_key,
            "blast should cast from the newly invoked primary slot"
        );
    }

    #[test]
    fn manual_wait_planner_copies_completion_metadata() {
        let event = invoker_qe_fixture();
        let settings = Settings::default();
        let state = InvokerObservedState::from_event(&event);
        let profile =
            find_profile(&settings.heroes.invoker, "qe-burst").expect("QE profile should exist");

        let plan = build_profile_execution_plan(profile, &state, &settings.heroes.invoker)
            .expect("QE profile should build");

        let first_spell = plan
            .iter()
            .find_map(|step| match step {
                PlannedInvokerAction::Spell {
                    completion_mode,
                    completion_timeout_ms,
                    ..
                } => Some((completion_mode.clone(), *completion_timeout_ms)),
                PlannedInvokerAction::Item { .. } => None,
            })
            .expect("QE profile should include a spell step");

        assert_eq!(
            first_spell.0,
            InvokerProfileStepCompletionMode::WaitForCooldown
        );
        assert_eq!(first_spell.1, 3000);
    }

    #[test]
    fn manual_wait_detects_already_on_cooldown() {
        let _guard = MANUAL_WAIT_TEST_GUARD.lock().unwrap();
        let mut event = invoker_qe_fixture();
        event.abilities.ability4.name = "invoker_sun_strike".to_string();
        event.abilities.ability4.cooldown = 12;
        event.abilities.ability4.can_cast = false;

        assert!(spell_is_on_cooldown(&event, "invoker_sun_strike"));
    }

    #[test]
    fn manual_wait_completes_after_gsi_update() {
        let _guard = MANUAL_WAIT_TEST_GUARD.lock().unwrap();
        let mut event = invoker_qe_fixture();
        event.abilities.ability4.name = "invoker_sun_strike".to_string();
        event.abilities.ability4.cooldown = 0;
        *INVOKER_LAST_EVENT.lock().unwrap() = Some(event.clone());

        let updater = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(30));
            let mut cooling = event;
            cooling.abilities.ability4.cooldown = 25;
            cooling.abilities.ability4.can_cast = false;
            *INVOKER_LAST_EVENT.lock().unwrap() = Some(cooling);
        });

        let outcome = wait_for_spell_cooldown_start("invoker_sun_strike", 300, 5);
        updater.join().expect("updater should finish");

        assert_eq!(outcome, CooldownWaitOutcome::Started);
    }

    #[test]
    fn manual_wait_times_out_without_cooldown_start() {
        let _guard = MANUAL_WAIT_TEST_GUARD.lock().unwrap();
        let mut event = invoker_qe_fixture();
        event.abilities.ability4.name = "invoker_sun_strike".to_string();
        event.abilities.ability4.cooldown = 0;
        *INVOKER_LAST_EVENT.lock().unwrap() = Some(event);

        assert_eq!(
            wait_for_spell_cooldown_start("invoker_sun_strike", 20, 5),
            CooldownWaitOutcome::TimedOut
        );
    }

    #[test]
    fn manual_wait_stops_when_hero_becomes_unavailable() {
        let _guard = MANUAL_WAIT_TEST_GUARD.lock().unwrap();
        let event = invoker_qe_fixture();
        *INVOKER_LAST_EVENT.lock().unwrap() = Some(event);

        let updater = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(20));
            let mut disabled = invoker_qe_fixture();
            disabled.hero.alive = false;
            *INVOKER_LAST_EVENT.lock().unwrap() = Some(disabled);
        });

        let outcome = wait_for_spell_cooldown_start("invoker_sun_strike", 200, 5);
        updater.join().expect("updater should finish");

        assert_eq!(outcome, CooldownWaitOutcome::HeroUnavailable);
    }

    #[test]
    fn build_run_profile_request_captures_settings_snapshot() {
        let mut settings = Settings::default();
        settings.heroes.invoker.profiles[0].steps.push(
            crate::config::settings::InvokerProfileStep {
                kind: InvokerProfileStepKind::Spell,
                target: "invoker_cold_snap".to_string(),
                delay_after_ms: 999,
                cast_behavior: InvokerProfileStepCastBehavior::Normal,
                completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                completion_timeout_ms: 3000,
                notes: "test extra step".to_string(),
            },
        );

        let request = build_run_profile_request("ghost-walk-panic", &settings);

        let InvokerRequest::RunProfile { profile_id, settings: captured_settings } = request;
        assert_eq!(profile_id, "ghost-walk-panic");
        assert_eq!(
            captured_settings.heroes.invoker.profiles[0]
                .steps
                .last()
                .map(|s| s.delay_after_ms),
            Some(999),
            "captured settings should preserve custom extra step"
        );
    }

    #[test]
    fn handle_standalone_trigger_respects_user_selected_active_combo_profile() {
        let _guard = QUEUE_TEST_GUARD.lock().unwrap();
        use std::sync::mpsc;

        // Set up required GSI event state for worker
        *INVOKER_LAST_EVENT.lock().unwrap() = Some(invoker_qw_fixture());

        let mut settings = Settings::default();
        // Enable qe-burst profile
        settings
            .heroes
            .invoker
            .profiles
            .iter_mut()
            .find(|p| p.id == "qe-burst")
            .unwrap()
            .enabled = true;

        let settings = Arc::new(Mutex::new(settings));
        let executor = ActionExecutor::new();
        let app_state = Arc::new(Mutex::new(crate::state::AppState::default()));

        // Set qe-burst as the active profile in app_state
        app_state.lock().unwrap().invoker_active_combo_profile_id = Some("qe-burst".to_string());

        // Set up observer to capture enqueued request
        let (observe_tx, observe_rx) = mpsc::channel::<InvokerRequest>();
        set_test_observer(observe_tx);

        let script = InvokerScript::new(settings, executor, app_state);
        script.handle_standalone_trigger();

        // Verify that qe-burst was enqueued (the user-selected profile)
        let received = observe_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("should receive request");

        clear_test_observer();

        match received {
            InvokerRequest::RunProfile { profile_id, .. } => {
                assert_eq!(
                    profile_id, "qe-burst",
                    "standalone trigger should use user-selected active combo profile from app_state"
                );
            }
        }
    }
}
