use crate::actions::common::{find_item_slot, SurvivabilityActions};
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::meepo_macro::{
    evaluate_farm_pulse, suspend_for_manual_combo, toggle_meepo_macro, MeepoFarmPulseDecision,
    MeepoMacroMode, MeepoMacroSuspendReason,
};
use crate::actions::heroes::traits::HeroScript;
use crate::actions::heroes::meepo_state::{latest_meepo_observed_state, refresh_meepo_observed_state};
use crate::config::settings::MeepoConfig;
use crate::config::Settings;
use crate::input::simulation::{mouse_click, press_key};
use crate::models::{GsiWebhookEvent, Hero, Item};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, warn};

const DIG_ABILITY_NAME: &str = "meepo_petrify";
const MEGAMEEPO_ABILITY_NAME: &str = "meepo_megameepo";

fn ability_is_ready(event: &GsiWebhookEvent, ability_name: &str) -> bool {
    (0..=5).any(|index| {
        event.abilities.get_by_index(index).is_some_and(|ability| {
            ability.name == ability_name && ability.level > 0 && ability.can_cast
        })
    })
}

fn should_cast_dig(
    event: &GsiWebhookEvent,
    config: &MeepoConfig,
    last_cast: Option<Instant>,
    in_danger: bool,
) -> bool {
    if !config.auto_dig_on_danger || !in_danger {
        return false;
    }

    if !event.hero.alive || event.hero.stunned || event.hero.silenced {
        return false;
    }

    if !event.hero.aghanims_shard {
        return false;
    }

    if event.hero.health_percent > config.dig_hp_threshold_percent {
        return false;
    }

    if !ability_is_ready(event, DIG_ABILITY_NAME) {
        return false;
    }

    if let Some(last_cast) = last_cast {
        if last_cast.elapsed() < Duration::from_millis(config.defensive_trigger_cooldown_ms) {
            return false;
        }
    }

    true
}

fn should_cast_megameepo(
    event: &GsiWebhookEvent,
    config: &MeepoConfig,
    last_cast: Option<Instant>,
    in_danger: bool,
) -> bool {
    if !config.auto_megameepo_on_danger || !in_danger {
        return false;
    }

    if !event.hero.alive || event.hero.stunned || event.hero.silenced {
        return false;
    }

    if !event.hero.aghanims_scepter {
        return false;
    }

    if event.hero.health_percent > config.megameepo_hp_threshold_percent {
        return false;
    }

    if !ability_is_ready(event, MEGAMEEPO_ABILITY_NAME) {
        return false;
    }

    if let Some(last_cast) = last_cast {
        if last_cast.elapsed() < Duration::from_millis(config.defensive_trigger_cooldown_ms) {
            return false;
        }
    }

    true
}

fn find_combo_item_slot_key(
    event: &GsiWebhookEvent,
    settings: &Settings,
    partial_item_name: &str,
) -> Option<char> {
    event
        .items
        .all_slots()
        .into_iter()
        .find_map(|(slot, item)| {
            (item.name.contains(partial_item_name) && item.can_cast == Some(true))
                .then(|| settings.get_key_for_slot(slot))
                .flatten()
        })
}

fn press_key_repeatedly(key: char, count: u32, interval_ms: u64) {
    for index in 0..count {
        press_key(key);
        if index + 1 < count {
            thread::sleep(Duration::from_millis(interval_ms));
        }
    }
}

pub struct MeepoScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
    latest_event: Mutex<Option<GsiWebhookEvent>>,
    last_defensive_trigger: Mutex<Option<Instant>>,
}

impl MeepoScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self {
            settings,
            executor,
            latest_event: Mutex::new(None),
            last_defensive_trigger: Mutex::new(None),
        }
    }

    fn execute_combo(&self, event: &GsiWebhookEvent, settings: &Settings) {
        let meepo = &settings.heroes.meepo;

        info!("Executing Meepo combo sequence...");

        if let Some(key) = find_item_slot(event, settings, Item::Blink) {
            info!("Using Blink ({})", key);
            press_key(key);
            thread::sleep(Duration::from_millis(meepo.post_blink_delay_ms));
        } else {
            info!("Meepo combo continuing without Blink");
        }

        for item_name in &meepo.combo_items {
            if let Some(key) = find_combo_item_slot_key(event, settings, item_name) {
                info!("Using combo item '{}' ({})", item_name, key);
                for index in 0..meepo.combo_item_spam_count {
                    press_key(key);
                    if index + 1 < meepo.combo_item_spam_count {
                        thread::sleep(Duration::from_millis(meepo.combo_item_delay_ms));
                    }
                }
            } else {
                warn!("Combo item '{}' not found or not castable", item_name);
            }
        }

        info!("Casting Earthbind ({})", meepo.earthbind_key);
        press_key_repeatedly(
            meepo.earthbind_key,
            meepo.earthbind_press_count,
            meepo.earthbind_press_interval_ms,
        );

        info!("Casting Poof ({})", meepo.poof_key);
        press_key_repeatedly(
            meepo.poof_key,
            meepo.poof_press_count,
            meepo.poof_press_interval_ms,
        );
    }

    fn maybe_trigger_defensive_cast(
        &self,
        event: &GsiWebhookEvent,
        config: &MeepoConfig,
        in_danger: bool,
    ) {
        let action = {
            let mut last_trigger = self.last_defensive_trigger.lock().unwrap();

            if should_cast_dig(event, config, *last_trigger, in_danger) {
                *last_trigger = Some(Instant::now());
                Some(("meepo-dig-danger", config.dig_key, "Dig"))
            } else if should_cast_megameepo(event, config, *last_trigger, in_danger) {
                *last_trigger = Some(Instant::now());
                Some(("meepo-megameepo-danger", config.megameepo_key, "MegaMeepo"))
            } else {
                None
            }
        };

        if let Some((label, key, ability_name)) = action {
            self.executor.enqueue(label, move || {
                info!("Auto-casting Meepo {} ({})", ability_name, key);
                press_key(key);
            });
        }
    }

    fn maybe_run_farm_assist(&self, config: &MeepoConfig) {
        let Some(observed) = latest_meepo_observed_state() else {
            return;
        };

        if evaluate_farm_pulse(&observed, &config.farm_assist, Instant::now())
            != MeepoFarmPulseDecision::Run
        {
            return;
        }

        let poof_key = config.poof_key;
        let pulse_count = config.farm_assist.poof_press_count;
        let interval_ms = config.farm_assist.poof_press_interval_ms;
        let right_click_after_poof = config.farm_assist.right_click_after_poof;

        self.executor.enqueue("meepo-farm-pulse", move || {
            info!("Executing Meepo farm-assist pulse");
            press_key_repeatedly(poof_key, pulse_count, interval_ms);
            if right_click_after_poof {
                mouse_click();
            }
        });
    }

    pub fn toggle_farm_assist(&self) -> MeepoMacroMode {
        let config = { self.settings.lock().unwrap().heroes.meepo.farm_assist.clone() };
        let snapshot_available = latest_meepo_observed_state().is_some();
        let mode = toggle_meepo_macro(config.enabled, snapshot_available);

        match mode {
            MeepoMacroMode::Armed => info!("Meepo farm assist armed"),
            MeepoMacroMode::Inactive => info!("Meepo farm assist disabled"),
            MeepoMacroMode::Suspended(MeepoMacroSuspendReason::Disabled) => {
                warn!("Meepo farm assist is disabled in config")
            }
            MeepoMacroMode::Suspended(MeepoMacroSuspendReason::UnableToCast) => {
                warn!("Meepo farm assist needs a fresh Meepo GSI snapshot before arming")
            }
            MeepoMacroMode::Suspended(reason) => {
                info!("Meepo farm assist suspended: {:?}", reason)
            }
        }

        mode
    }
}

impl HeroScript for MeepoScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        {
            let mut latest_event = self.latest_event.lock().unwrap();
            *latest_event = Some(event.clone());
        }

        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        let (in_danger, meepo_config) = {
            let settings = self.settings.lock().unwrap();
            let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
            refresh_meepo_observed_state(event, &settings, in_danger);
            (
                in_danger,
                settings.heroes.meepo.clone(),
            )
        };

        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);

        self.maybe_trigger_defensive_cast(event, &meepo_config, in_danger);
        self.maybe_run_farm_assist(&meepo_config);
    }

    fn handle_standalone_trigger(&self) {
        let latest_event = { self.latest_event.lock().unwrap().clone() };
        let Some(event) = latest_event else {
            warn!("No GSI event received yet - Meepo combo needs item data");
            return;
        };

        let settings = { self.settings.lock().unwrap().clone() };
        suspend_for_manual_combo(settings.heroes.meepo.farm_assist.suspend_after_manual_combo_ms);
        self.execute_combo(&event, &settings);
    }

    fn hero_name(&self) -> &'static str {
        Hero::Meepo.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ability_is_ready, should_cast_dig, should_cast_megameepo, DIG_ABILITY_NAME,
        MEGAMEEPO_ABILITY_NAME,
    };
    use crate::actions::heroes::meepo_macro::{
        clear_meepo_macro_state, latest_meepo_macro_status, meepo_macro_test_lock,
        MeepoMacroMode, MeepoMacroSuspendReason,
    };
    use crate::actions::heroes::meepo_state::{clear_meepo_observed_state, refresh_meepo_observed_state};
    use crate::actions::executor::ActionExecutor;
    use crate::config::Settings;
    use crate::models::GsiWebhookEvent;
    use std::sync::{Arc, Mutex};

    fn meepo_fixture() -> GsiWebhookEvent {
        serde_json::from_str(include_str!("../../../tests/fixtures/meepo_event.json"))
            .expect("Meepo fixture should deserialize")
    }

    #[test]
    fn meepo_ability_is_ready_finds_petrify() {
        let event = meepo_fixture();
        assert!(ability_is_ready(&event, DIG_ABILITY_NAME));
    }

    #[test]
    fn meepo_ability_is_ready_finds_megameepo() {
        let event = meepo_fixture();
        assert!(ability_is_ready(&event, MEGAMEEPO_ABILITY_NAME));
    }

    #[test]
    fn meepo_should_cast_dig_when_conditions_match() {
        let mut event = meepo_fixture();
        let config = &Settings::default().heroes.meepo;
        event.hero.health_percent = config.dig_hp_threshold_percent;

        assert!(should_cast_dig(&event, config, None, true));
    }

    #[test]
    fn meepo_should_not_cast_dig_without_danger() {
        let event = meepo_fixture();
        let config = &Settings::default().heroes.meepo;

        assert!(!should_cast_dig(&event, config, None, false));
    }

    #[test]
    fn meepo_should_cast_megameepo_when_conditions_match() {
        let event = meepo_fixture();
        let config = &Settings::default().heroes.meepo;

        assert!(should_cast_megameepo(&event, config, None, true));
    }

    #[test]
    fn meepo_should_not_cast_megameepo_without_danger() {
        let event = meepo_fixture();
        let config = &Settings::default().heroes.meepo;

        assert!(!should_cast_megameepo(&event, config, None, false));
    }

    #[test]
    fn toggle_farm_assist_without_snapshot_suspends() {
        let _guard = meepo_macro_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        clear_meepo_macro_state();
        clear_meepo_observed_state();
        let script = super::MeepoScript::new(
            Arc::new(Mutex::new(Settings::default())),
            ActionExecutor::new(),
        );

        assert_eq!(
            script.toggle_farm_assist(),
            MeepoMacroMode::Suspended(MeepoMacroSuspendReason::UnableToCast)
        );
    }

    #[test]
    fn toggle_farm_assist_arms_with_snapshot() {
        let _guard = meepo_macro_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        clear_meepo_macro_state();
        clear_meepo_observed_state();
        let settings = Arc::new(Mutex::new(Settings::default()));
        let script = super::MeepoScript::new(settings.clone(), ActionExecutor::new());
        let event = meepo_fixture();
        let settings_guard = settings.lock().unwrap().clone();
        refresh_meepo_observed_state(&event, &settings_guard, false);

        assert_eq!(script.toggle_farm_assist(), MeepoMacroMode::Armed);
        assert_eq!(latest_meepo_macro_status().mode, MeepoMacroMode::Armed);
    }
}
