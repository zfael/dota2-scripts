use crate::actions::heroes::traits::HeroScript;
use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::config::{settings::HuskarRoshanSpearsConfig, Settings};
use crate::models::{gsi_event::Ability, GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info};

lazy_static! {
    static ref BERSERKER_BLOOD_DEBUFF_DETECTED: Mutex<Option<Instant>> = Mutex::new(None);
    static ref ROSHAN_SPEARS_STATE: Mutex<HuskarRoshanSpearsState> =
        Mutex::new(HuskarRoshanSpearsState::default());
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct HuskarRoshanSpearsState {
    disabled_by_app: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HuskarRoshanSpearsAction {
    None,
    Disable,
    Reenable,
    ClearOwnership,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RoshanSpearsThresholds {
    effective_trigger: u32,
    disable_line: u32,
    reenable_line: u32,
}

fn roshan_spears_thresholds(
    effective_trigger: u32,
    config: &HuskarRoshanSpearsConfig,
) -> RoshanSpearsThresholds {
    RoshanSpearsThresholds {
        effective_trigger,
        disable_line: effective_trigger.saturating_add(config.disable_buffer_hp),
        reenable_line: effective_trigger.saturating_add(config.reenable_buffer_hp),
    }
}

fn should_log_roshan_spears_idle(health: u32, thresholds: RoshanSpearsThresholds) -> bool {
    health <= thresholds.reenable_line.saturating_add(80)
}

fn should_log_roshan_spears_noop(
    health: u32,
    thresholds: RoshanSpearsThresholds,
    owned_by_app: bool,
    clear_reason: Option<&'static str>,
) -> bool {
    owned_by_app || (clear_reason.is_none() && should_log_roshan_spears_idle(health, thresholds))
}

fn roshan_spears_clear_reason(
    config_enabled: bool,
    roshan_mode_armed: bool,
    burning_spear_present: bool,
) -> Option<&'static str> {
    if !config_enabled {
        Some("feature disabled")
    } else if !roshan_mode_armed {
        Some("roshan mode disarmed")
    } else if !burning_spear_present {
        Some("Burning Spears missing or unparseable")
    } else {
        None
    }
}

fn roshan_spears_action_label(action: HuskarRoshanSpearsAction) -> &'static str {
    match action {
        HuskarRoshanSpearsAction::None => "none",
        HuskarRoshanSpearsAction::Disable => "disable",
        HuskarRoshanSpearsAction::Reenable => "reenable",
        HuskarRoshanSpearsAction::ClearOwnership => "clear_ownership",
    }
}

fn evaluate_roshan_spears_gate(
    health: u32,
    effective_trigger: u32,
    roshan_mode_armed: bool,
    burning_spear_present: bool,
    config: &HuskarRoshanSpearsConfig,
    state: &mut HuskarRoshanSpearsState,
) -> HuskarRoshanSpearsAction {
    if !config.enabled || !roshan_mode_armed || !burning_spear_present {
        let had_ownership = state.disabled_by_app;
        state.disabled_by_app = false;
        return if had_ownership {
            HuskarRoshanSpearsAction::ClearOwnership
        } else {
            HuskarRoshanSpearsAction::None
        };
    }

    let thresholds = roshan_spears_thresholds(effective_trigger, config);

    if state.disabled_by_app {
        if health >= thresholds.reenable_line {
            state.disabled_by_app = false;
            HuskarRoshanSpearsAction::Reenable
        } else {
            HuskarRoshanSpearsAction::None
        }
    } else if health <= thresholds.disable_line {
        state.disabled_by_app = true;
        HuskarRoshanSpearsAction::Disable
    } else {
        HuskarRoshanSpearsAction::None
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn evaluate_resolved_roshan_spears_gate(
    settings: &Settings,
    health: u32,
    roshan_mode_armed: bool,
    burning_spear_present: bool,
    state: &mut HuskarRoshanSpearsState,
) -> HuskarRoshanSpearsAction {
    let resolved = settings.resolve_armlet_config(Hero::Huskar.to_game_name());
    let effective_trigger = resolved
        .toggle_threshold
        .saturating_add(resolved.predictive_offset);

    evaluate_roshan_spears_gate(
        health,
        effective_trigger,
        roshan_mode_armed,
        burning_spear_present,
        &settings.heroes.huskar.roshan_spears,
        state,
    )
}

fn clear_roshan_spears_ownership(reason: &str) {
    if let Ok(mut state) = ROSHAN_SPEARS_STATE.lock() {
        if state.disabled_by_app {
            state.disabled_by_app = false;
            info!(
                "Clearing Roshan Burning Spears ownership state without toggling (reason: {})",
                reason
            );
        }
    }
}

fn log_roshan_spears_gate_event(
    action: &str,
    reason: &str,
    health: u32,
    thresholds: RoshanSpearsThresholds,
    roshan_mode_armed: bool,
    owned_by_app_before: bool,
    owned_by_app_after: bool,
    config_enabled: bool,
    burning_spear_present: bool,
) {
    info!(
        "{} (hp={}, trigger={}, disable_line={}, reenable_line={}, roshan_mode_armed={}, owned_by_app_before={}, owned_by_app_after={}, config_enabled={}, burning_spears_present={}, reason={})",
        action,
        health,
        thresholds.effective_trigger,
        thresholds.disable_line,
        thresholds.reenable_line,
        roshan_mode_armed,
        owned_by_app_before,
        owned_by_app_after,
        config_enabled,
        burning_spear_present,
        reason
    );
}

fn find_burning_spear_ability<'a>(event: &'a GsiWebhookEvent) -> Option<&'a Ability> {
    [
        &event.abilities.ability0,
        &event.abilities.ability1,
        &event.abilities.ability2,
        &event.abilities.ability3,
        &event.abilities.ability4,
        &event.abilities.ability5,
    ]
    .iter()
    .find(|ability| ability.name == "huskar_burning_spear" && ability.level > 0 && !ability.passive)
    .copied()
}

fn emit_burning_spear_toggle(key: char) {
    crate::input::simulation::modifier_down(crate::input::simulation::ModifierKey::Alt);
    crate::input::simulation::press_key(key);
    crate::input::simulation::modifier_up(crate::input::simulation::ModifierKey::Alt);
}

pub struct HuskarScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl HuskarScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }

    fn berserker_blood_cleanse(&self, event: &GsiWebhookEvent) {
        if !event.hero.is_alive() {
            return;
        }

        // Check if hero has debuff
        if !event.hero.has_debuff {
            // Reset debuff tracker when no debuff
            if let Ok(mut debuff_time) = BERSERKER_BLOOD_DEBUFF_DETECTED.try_lock() {
                if debuff_time.is_some() {
                    debug!("No debuffs detected, resetting berserker blood tracker");
                    *debuff_time = None;
                }
            }
            return;
        }

        // Find berserker blood ability
        let berserker_ability = [
            &event.abilities.ability0,
            &event.abilities.ability1,
            &event.abilities.ability2,
            &event.abilities.ability3,
        ]
        .iter()
        .find(|ability| ability.name == "huskar_berserkers_blood")
        .copied();

        let Some(ability) = berserker_ability else {
            return;
        };

        // Check if ability can be cast (not on cooldown and has levels)
        if !ability.can_cast || ability.level == 0 || ability.cooldown > 0 {
            debug!("Berserker Blood not ready: can_cast={}, level={}, cooldown={}",
                ability.can_cast, ability.level, ability.cooldown);
            return;
        }

        let settings = self.settings.lock().unwrap();
        let delay_ms = settings.heroes.huskar.berserker_blood_delay_ms;
        let key = settings.heroes.huskar.berserker_blood_key;
        drop(settings);

        if let Ok(mut debuff_time) = BERSERKER_BLOOD_DEBUFF_DETECTED.try_lock() {
            match *debuff_time {
                Some(first_debuff_time) => {
                    // Debuff already detected, check if delay has passed
                    if first_debuff_time.elapsed() >= Duration::from_millis(delay_ms) {
                        info!("Activating Berserker Blood to cleanse debuffs ({}ms delay elapsed)", delay_ms);
                        crate::input::press_key(key);

                        // Reset tracker after activation
                        *debuff_time = None;
                    } else {
                        debug!("Waiting for more debuffs... ({}ms elapsed)",
                            first_debuff_time.elapsed().as_millis());
                    }
                }
                None => {
                    // First debuff detected, start tracking
                    info!("Debuff detected, starting {}ms timer for Berserker Blood", delay_ms);
                    *debuff_time = Some(Instant::now());
                }
            }
        }
    }

    fn manage_roshan_burning_spears(&self, event: &GsiWebhookEvent) {
        if !event.hero.is_alive() {
            clear_roshan_spears_ownership("hero died");
            return;
        }

        let settings = self.settings.lock().unwrap();
        let config = settings.heroes.huskar.roshan_spears.clone();
        let resolved = settings.resolve_armlet_config(&event.hero.name);
        let effective_trigger = resolved
            .toggle_threshold
            .saturating_add(resolved.predictive_offset);
        let thresholds = roshan_spears_thresholds(effective_trigger, &config);
        let roshan_mode_armed = crate::actions::armlet::is_roshan_mode_armed();
        let burning_spear_present = find_burning_spear_ability(event).is_some();
        let clear_reason =
            roshan_spears_clear_reason(config.enabled, roshan_mode_armed, burning_spear_present);
        drop(settings);

        let mut state = ROSHAN_SPEARS_STATE.lock().unwrap();
        let owned_by_app_before = state.disabled_by_app;
        let action = evaluate_roshan_spears_gate(
            event.hero.health,
            effective_trigger,
            roshan_mode_armed,
            burning_spear_present,
            &config,
            &mut state,
        );

        if roshan_mode_armed {
            info!(
                "Roshan Burning Spears summary: hp={}, trigger={}, disable_line={}, reenable_line={}, roshan_armed={}, config_enabled={}, burning_spears_present={}, owned_by_app={}, action={}",
                event.hero.health,
                thresholds.effective_trigger,
                thresholds.disable_line,
                thresholds.reenable_line,
                roshan_mode_armed,
                config.enabled,
                burning_spear_present,
                state.disabled_by_app,
                roshan_spears_action_label(action)
            );
        }

        match action {
            HuskarRoshanSpearsAction::Disable => {
                log_roshan_spears_gate_event(
                    "Disabling Burning Spears in Roshan mode",
                    "entered disable band",
                    event.hero.health,
                    thresholds,
                    roshan_mode_armed,
                    owned_by_app_before,
                    state.disabled_by_app,
                    config.enabled,
                    burning_spear_present,
                );
                info!(
                    "Emitting Roshan Burning Spears Alt+W toggle (action={}, key={}, hp={}, owned_by_app_after={})",
                    roshan_spears_action_label(HuskarRoshanSpearsAction::Disable),
                    config.burning_spear_key,
                    event.hero.health,
                    state.disabled_by_app
                );
                emit_burning_spear_toggle(config.burning_spear_key);
            }
            HuskarRoshanSpearsAction::Reenable => {
                log_roshan_spears_gate_event(
                    "Re-enabling Burning Spears in Roshan mode",
                    "recovered above re-enable line",
                    event.hero.health,
                    thresholds,
                    roshan_mode_armed,
                    owned_by_app_before,
                    state.disabled_by_app,
                    config.enabled,
                    burning_spear_present,
                );
                info!(
                    "Emitting Roshan Burning Spears Alt+W toggle (action={}, key={}, hp={}, owned_by_app_after={})",
                    roshan_spears_action_label(HuskarRoshanSpearsAction::Reenable),
                    config.burning_spear_key,
                    event.hero.health,
                    state.disabled_by_app
                );
                emit_burning_spear_toggle(config.burning_spear_key);
            }
            HuskarRoshanSpearsAction::ClearOwnership => {
                log_roshan_spears_gate_event(
                    "Clearing Roshan Burning Spears ownership state without toggling",
                    clear_reason.unwrap_or("gate preconditions changed"),
                    event.hero.health,
                    thresholds,
                    roshan_mode_armed,
                    owned_by_app_before,
                    state.disabled_by_app,
                    config.enabled,
                    burning_spear_present,
                );
            }
            HuskarRoshanSpearsAction::None => {
                if should_log_roshan_spears_noop(
                    event.hero.health,
                    thresholds,
                    owned_by_app_before,
                    clear_reason,
                ) {
                    let reason = if let Some(reason) = clear_reason {
                        reason
                    } else if state.disabled_by_app {
                        "still waiting to cross re-enable line"
                    } else {
                        "hp remains above disable line"
                    };

                    log_roshan_spears_gate_event(
                        "Roshan Burning Spears gate no-op",
                        reason,
                        event.hero.health,
                        thresholds,
                        roshan_mode_armed,
                        owned_by_app_before,
                        state.disabled_by_app,
                        config.enabled,
                        burning_spear_present,
                    );
                }
            }
        }
    }
}

impl HeroScript for HuskarScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // PRIORITY 1: Update danger detection state
        let settings = self.settings.lock().unwrap();
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);

        // PRIORITY 2: Create survivability actions for healing and defensive items
        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());

        // Check healing items (danger-aware)
        survivability.check_and_use_healing_items_with_danger(event, in_danger);

        // Use defensive items if in danger
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);

        // Use neutral items if in danger
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);

        // PRIORITY 3: Huskar-specific Roshan Burning Spears gate
        self.manage_roshan_burning_spears(event);

        // PRIORITY 4: Huskar-specific berserker blood cleanse
        self.berserker_blood_cleanse(event);
    }

    fn handle_standalone_trigger(&self) {
        info!("Huskar standalone trigger not implemented");
    }

    fn hero_name(&self) -> &'static str {
        Hero::Huskar.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::HuskarRoshanSpearsConfig;

    #[test]
    fn roshan_spears_thresholds_match_configured_buffers() {
        let thresholds = roshan_spears_thresholds(
            270,
            &HuskarRoshanSpearsConfig {
                enabled: true,
                burning_spear_key: 'w',
                disable_buffer_hp: 60,
                reenable_buffer_hp: 100,
            },
        );

        assert_eq!(thresholds.effective_trigger, 270);
        assert_eq!(thresholds.disable_line, 330);
        assert_eq!(thresholds.reenable_line, 370);
    }

    #[test]
    fn roshan_spears_idle_logging_window_turns_on_near_reenable_line() {
        let thresholds = RoshanSpearsThresholds {
            effective_trigger: 270,
            disable_line: 330,
            reenable_line: 370,
        };

        assert!(should_log_roshan_spears_idle(390, thresholds));
        assert!(!should_log_roshan_spears_idle(520, thresholds));
    }

    #[test]
    fn roshan_spears_idle_logging_window_stops_after_idle_buffer() {
        let thresholds = RoshanSpearsThresholds {
            effective_trigger: 270,
            disable_line: 330,
            reenable_line: 370,
        };

        assert!(should_log_roshan_spears_idle(450, thresholds));
        assert!(!should_log_roshan_spears_idle(451, thresholds));
    }

    #[test]
    fn roshan_spears_noop_logging_stays_quiet_when_roshan_mode_is_disarmed() {
        let thresholds = RoshanSpearsThresholds {
            effective_trigger: 270,
            disable_line: 330,
            reenable_line: 370,
        };

        assert!(!should_log_roshan_spears_noop(
            380,
            thresholds,
            false,
            Some("roshan mode disarmed"),
        ));
    }

    #[test]
    fn roshan_spears_noop_logging_stays_on_while_owned_by_app() {
        let thresholds = RoshanSpearsThresholds {
            effective_trigger: 270,
            disable_line: 330,
            reenable_line: 370,
        };

        assert!(should_log_roshan_spears_noop(
            520,
            thresholds,
            true,
            None,
        ));
    }

    #[test]
    fn roshan_spears_action_labels_match_summary_logs() {
        assert_eq!(roshan_spears_action_label(HuskarRoshanSpearsAction::None), "none");
        assert_eq!(roshan_spears_action_label(HuskarRoshanSpearsAction::Disable), "disable");
        assert_eq!(
            roshan_spears_action_label(HuskarRoshanSpearsAction::Reenable),
            "reenable"
        );
        assert_eq!(
            roshan_spears_action_label(HuskarRoshanSpearsAction::ClearOwnership),
            "clear_ownership"
        );
    }

    #[test]
    fn roshan_spears_gate_disables_once_when_hp_enters_disable_band() {
        let config = HuskarRoshanSpearsConfig {
            enabled: true,
            burning_spear_key: 'w',
            disable_buffer_hp: 60,
            reenable_buffer_hp: 100,
        };
        let mut state = HuskarRoshanSpearsState::default();

        let action = evaluate_roshan_spears_gate(320, 270, true, true, &config, &mut state);

        assert_eq!(action, HuskarRoshanSpearsAction::Disable);
        assert!(state.disabled_by_app);
    }

    #[test]
    fn roshan_spears_gate_does_not_repeat_disable_while_owned_by_app() {
        let config = HuskarRoshanSpearsConfig {
            enabled: true,
            burning_spear_key: 'w',
            disable_buffer_hp: 60,
            reenable_buffer_hp: 100,
        };
        let mut state = HuskarRoshanSpearsState {
            disabled_by_app: true,
        };

        let action = evaluate_roshan_spears_gate(300, 270, true, true, &config, &mut state);

        assert_eq!(action, HuskarRoshanSpearsAction::None);
        assert!(state.disabled_by_app);
    }

    #[test]
    fn roshan_spears_gate_reenables_only_after_hp_crosses_reenable_line() {
        let config = HuskarRoshanSpearsConfig {
            enabled: true,
            burning_spear_key: 'w',
            disable_buffer_hp: 60,
            reenable_buffer_hp: 100,
        };
        let mut state = HuskarRoshanSpearsState {
            disabled_by_app: true,
        };

        let action = evaluate_roshan_spears_gate(380, 270, true, true, &config, &mut state);

        assert_eq!(action, HuskarRoshanSpearsAction::Reenable);
        assert!(!state.disabled_by_app);
    }

    #[test]
    fn roshan_spears_gate_clears_ownership_when_roshan_mode_is_not_armed() {
        let config = HuskarRoshanSpearsConfig {
            enabled: true,
            burning_spear_key: 'w',
            disable_buffer_hp: 60,
            reenable_buffer_hp: 100,
        };
        let mut state = HuskarRoshanSpearsState {
            disabled_by_app: true,
        };

        let action = evaluate_roshan_spears_gate(300, 270, false, true, &config, &mut state);

        assert_eq!(action, HuskarRoshanSpearsAction::ClearOwnership);
        assert!(!state.disabled_by_app);
    }

    #[test]
    fn resolved_roshan_spears_gate_uses_huskar_armlet_override_thresholds() {
        let mut settings = Settings::default();
        settings.armlet.toggle_threshold = 320;
        settings.armlet.predictive_offset = 30;
        settings.heroes.huskar.armlet.toggle_threshold = Some(120);
        settings.heroes.huskar.armlet.predictive_offset = Some(150);
        settings.heroes.huskar.roshan_spears = HuskarRoshanSpearsConfig {
            enabled: true,
            burning_spear_key: 'w',
            disable_buffer_hp: 60,
            reenable_buffer_hp: 100,
        };
        let mut state = HuskarRoshanSpearsState::default();

        let action = evaluate_resolved_roshan_spears_gate(&settings, 330, true, true, &mut state);

        assert_eq!(action, HuskarRoshanSpearsAction::Disable);
        assert!(state.disabled_by_app);
    }
}
