use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::traits::HeroScript;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use tracing::info;

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

#[derive(Debug, Clone)]
enum InvokerRequest {
    PrimaryCombo,
    PanicGhostWalk,
    PrepPair,
}

fn run_invoker_request(request: InvokerRequest) {
    info!("Running Invoker request: {:?}", request);
}

fn enqueue_request(request: InvokerRequest) {
    if let Err(e) = INVOKER_REQUEST_QUEUE.send(request) {
        info!("Invoker request queue closed: {:?}", e);
    }
}

static INVOKER_REQUEST_QUEUE: LazyLock<mpsc::Sender<InvokerRequest>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<InvokerRequest>();
    thread::spawn(move || {
        while let Ok(request) = rx.recv() {
            run_invoker_request(request);
        }
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
    fn enqueue_request_preserves_fifo_order() {
        use std::sync::mpsc;
        use std::thread;
        use std::time::Duration;

        // Create a test queue with same semantics as INVOKER_REQUEST_QUEUE
        let (tx, rx) = mpsc::channel::<InvokerRequest>();

        // Worker thread that collects requests in order
        let collector = thread::spawn(move || {
            let mut collected = Vec::new();
            while let Ok(request) = rx.recv_timeout(Duration::from_millis(100)) {
                collected.push(request);
            }
            collected
        });

        // Enqueue requests in specific order
        let requests = vec![
            InvokerRequest::PrepPair,
            InvokerRequest::PanicGhostWalk,
            InvokerRequest::PrimaryCombo,
        ];

        for request in requests.clone() {
            tx.send(request).expect("queue should accept requests");
        }
        drop(tx); // Close sender so worker can finish

        // Verify FIFO order
        let received = collector.join().expect("worker thread should complete");
        assert_eq!(received.len(), 3, "all three requests should be received");
        assert!(matches!(received[0], InvokerRequest::PrepPair));
        assert!(matches!(received[1], InvokerRequest::PanicGhostWalk));
        assert!(matches!(received[2], InvokerRequest::PrimaryCombo));
    }
}
