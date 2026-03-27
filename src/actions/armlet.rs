use crate::config::settings::EffectiveArmletConfig;
use crate::config::Settings;
use crate::input::simulation::{modifier_down, modifier_up, press_key, ModifierKey};
use crate::models::GsiWebhookEvent;
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

lazy_static! {
    static ref ARMLET_LAST_TOGGLE: Mutex<Option<Instant>> = Mutex::new(None);
    static ref ARMLET_CRITICAL_HP: Mutex<Option<u32>> = Mutex::new(None);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArmletTriggerStep {
    QuickCast(char),
    ModifierDown(ModifierKey),
    ModifierUp(ModifierKey),
}

fn parse_cast_modifier(raw: &str) -> Option<ModifierKey> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "alt" => Some(ModifierKey::Alt),
        "ctrl" | "control" => Some(ModifierKey::Control),
        "shift" => Some(ModifierKey::Shift),
        _ => None,
    }
}

fn resolve_cast_modifier(config: &EffectiveArmletConfig) -> ModifierKey {
    parse_cast_modifier(&config.cast_modifier).unwrap_or_else(|| {
        warn!(
            "Unknown armlet cast modifier {:?}; defaulting to Alt",
            config.cast_modifier
        );
        ModifierKey::Alt
    })
}

fn plan_dual_trigger_sequence(
    slot_key: char,
    cast_modifier: ModifierKey,
) -> [ArmletTriggerStep; 4] {
    [
        ArmletTriggerStep::QuickCast(slot_key),
        ArmletTriggerStep::ModifierDown(cast_modifier),
        ArmletTriggerStep::QuickCast(slot_key),
        ArmletTriggerStep::ModifierUp(cast_modifier),
    ]
}

fn execute_trigger_step(step: ArmletTriggerStep) {
    match step {
        ArmletTriggerStep::QuickCast(key) => press_key(key),
        ArmletTriggerStep::ModifierDown(modifier) => modifier_down(modifier),
        ArmletTriggerStep::ModifierUp(modifier) => modifier_up(modifier),
    }
}

fn execute_dual_trigger(slot_key: char, cast_modifier: ModifierKey) {
    for step in plan_dual_trigger_sequence(slot_key, cast_modifier) {
        execute_trigger_step(step);
    }
}

fn next_critical_retry_health(health: u32, threshold: u32) -> Option<u32> {
    if health < threshold / 2 {
        Some(health)
    } else {
        None
    }
}

fn cooldown_ready(last_toggle: Option<Instant>, cooldown_ms: u64) -> bool {
    match last_toggle {
        Some(last_time) => last_time.elapsed() >= Duration::from_millis(cooldown_ms),
        None => true,
    }
}

fn cooldown_remaining_ms(last_toggle: Option<Instant>, cooldown_ms: u64) -> u64 {
    match last_toggle {
        Some(last_time) => cooldown_ms.saturating_sub(last_time.elapsed().as_millis() as u64),
        None => 0,
    }
}

fn should_force_critical_retry(
    health: u32,
    threshold: u32,
    last_critical: Option<u32>,
    last_toggle: Option<Instant>,
    cooldown_ms: u64,
) -> bool {
    match last_critical {
        Some(last_critical) => {
            health < threshold / 2
                && health <= last_critical
                && cooldown_ready(last_toggle, cooldown_ms)
        }
        None => false,
    }
}

fn find_armlet_slot_key(event: &GsiWebhookEvent, settings: &Settings) -> Option<char> {
    let armlet_slot = event
        .items
        .all_slots()
        .iter()
        .find(|(_, item)| item.name == "item_armlet")
        .map(|(slot, _)| *slot)?;

    settings.get_key_for_slot(armlet_slot)
}

pub fn maybe_toggle(event: &GsiWebhookEvent, settings: &Settings) {
    if !event.hero.is_alive() {
        return;
    }

    let resolved = settings.resolve_armlet_config(&event.hero.name);
    if !resolved.enabled {
        return;
    }

    let Some(slot_key) = find_armlet_slot_key(event, settings) else {
        return;
    };

    let health = event.hero.health;
    let threshold = resolved.toggle_threshold;
    let trigger_point = threshold + resolved.predictive_offset;
    let cooldown_ms = resolved.toggle_cooldown_ms;
    let cast_modifier = resolve_cast_modifier(&resolved);

    let last_critical = *ARMLET_CRITICAL_HP.lock().unwrap();
    let last_toggle_snapshot = *ARMLET_LAST_TOGGLE.lock().unwrap();

    if should_force_critical_retry(
        health,
        threshold,
        last_critical,
        last_toggle_snapshot,
        cooldown_ms,
    ) {
        warn!(
            "Critical HP detected! HP: {} (likely armlet stuck on). Forcing emergency toggle.",
            health
        );

        execute_dual_trigger(slot_key, cast_modifier);

        let mut critical_hp = ARMLET_CRITICAL_HP.lock().unwrap();
        *critical_hp = None;
        drop(critical_hp);

        let mut last_toggle = ARMLET_LAST_TOGGLE.lock().unwrap();
        *last_toggle = Some(Instant::now());
        return;
    }

    if health < trigger_point {
        if event.hero.is_stunned() {
            debug!("Hero stunned, skipping armlet toggle (HP: {})", health);
            return;
        }

        let mut last_toggle = ARMLET_LAST_TOGGLE.lock().unwrap();
        let can_toggle = cooldown_ready(*last_toggle, cooldown_ms);

        if !can_toggle {
            let remaining = cooldown_remaining_ms(*last_toggle, cooldown_ms);
            debug!("Armlet toggle on cooldown ({}ms remaining)", remaining);
            return;
        }

        info!(
            "Triggering armlet toggle (HP: {} < trigger: {}, base: {}, cooldown: {}ms)",
            health, trigger_point, threshold, cooldown_ms
        );

        execute_dual_trigger(slot_key, cast_modifier);
        *last_toggle = Some(Instant::now());
        drop(last_toggle);

        let mut critical_hp = ARMLET_CRITICAL_HP.lock().unwrap();
        *critical_hp = next_critical_retry_health(health, threshold);
    } else if let Ok(mut critical_hp) = ARMLET_CRITICAL_HP.try_lock() {
        if critical_hp.is_some() {
            debug!("HP recovered to safe levels, resetting critical HP tracker");
            *critical_hp = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cooldown_ready, next_critical_retry_health, parse_cast_modifier, plan_dual_trigger_sequence,
        resolve_cast_modifier, should_force_critical_retry, ArmletTriggerStep,
    };
    use crate::config::{
        settings::{ArmletAutomationConfig, HeroArmletOverrideConfig},
        Settings,
    };
    use crate::input::simulation::ModifierKey;
    use std::time::{Duration, Instant};

    #[test]
    fn dual_trigger_plan_uses_quick_cast_then_modified_cast() {
        assert_eq!(
            plan_dual_trigger_sequence('x', ModifierKey::Alt),
            [
                ArmletTriggerStep::QuickCast('x'),
                ArmletTriggerStep::ModifierDown(ModifierKey::Alt),
                ArmletTriggerStep::QuickCast('x'),
                ArmletTriggerStep::ModifierUp(ModifierKey::Alt),
            ]
        );
    }

    #[test]
    fn parse_cast_modifier_supports_common_aliases() {
        assert_eq!(parse_cast_modifier("Alt"), Some(ModifierKey::Alt));
        assert_eq!(parse_cast_modifier("ctrl"), Some(ModifierKey::Control));
        assert_eq!(parse_cast_modifier("Control"), Some(ModifierKey::Control));
        assert_eq!(parse_cast_modifier("Shift"), Some(ModifierKey::Shift));
    }

    #[test]
    fn invalid_modifier_falls_back_to_alt() {
        let config = crate::config::settings::EffectiveArmletConfig {
            enabled: true,
            cast_modifier: "weird".to_string(),
            toggle_threshold: 320,
            predictive_offset: 30,
            toggle_cooldown_ms: 250,
        };

        assert_eq!(resolve_cast_modifier(&config), ModifierKey::Alt);
    }

    #[test]
    fn critical_retry_health_only_arms_for_very_low_hp() {
        assert_eq!(next_critical_retry_health(100, 320), Some(100));
        assert_eq!(next_critical_retry_health(220, 320), None);
    }

    #[test]
    fn critical_retry_waits_for_cooldown_before_forcing_another_toggle() {
        let just_now = Some(Instant::now());

        assert!(!should_force_critical_retry(
            1,
            120,
            Some(1),
            just_now,
            300,
        ));
    }

    #[test]
    fn critical_retry_can_fire_after_cooldown_when_hp_is_still_critical() {
        let cooled_down = Some(Instant::now() - Duration::from_millis(400));

        assert!(should_force_critical_retry(
            1,
            120,
            Some(1),
            cooled_down,
            300,
        ));
    }

    #[test]
    fn cooldown_ready_is_false_until_window_has_elapsed() {
        assert!(!cooldown_ready(
            Some(Instant::now() - Duration::from_millis(50)),
            300
        ));
        assert!(cooldown_ready(
            Some(Instant::now() - Duration::from_millis(350)),
            300
        ));
    }

    #[test]
    fn resolve_armlet_config_uses_shared_defaults_for_unknown_hero() {
        let mut settings = Settings::default();
        settings.armlet = ArmletAutomationConfig {
            enabled: true,
            cast_modifier: "Shift".to_string(),
            toggle_threshold: 350,
            predictive_offset: 40,
            toggle_cooldown_ms: 280,
        };

        let resolved = settings.resolve_armlet_config("npc_dota_hero_kunkka");

        assert_eq!(resolved.enabled, true);
        assert_eq!(resolved.cast_modifier, "Shift");
        assert_eq!(resolved.toggle_threshold, 350);
        assert_eq!(resolved.predictive_offset, 40);
        assert_eq!(resolved.toggle_cooldown_ms, 280);
    }

    #[test]
    fn resolve_armlet_config_uses_huskar_legacy_fields_when_nested_override_is_empty() {
        let mut settings = Settings::default();
        settings.armlet.toggle_threshold = 320;
        settings.armlet.predictive_offset = 30;
        settings.armlet.toggle_cooldown_ms = 250;
        settings.heroes.huskar.armlet_toggle_threshold = 120;
        settings.heroes.huskar.armlet_predictive_offset = 150;
        settings.heroes.huskar.armlet_toggle_cooldown_ms = 300;

        let resolved = settings.resolve_armlet_config("npc_dota_hero_huskar");

        assert_eq!(resolved.toggle_threshold, 120);
        assert_eq!(resolved.predictive_offset, 150);
        assert_eq!(resolved.toggle_cooldown_ms, 300);
    }

    #[test]
    fn resolve_armlet_config_prefers_nested_override_and_falls_back_to_shared_defaults() {
        let mut settings = Settings::default();
        settings.armlet.toggle_threshold = 320;
        settings.armlet.predictive_offset = 30;
        settings.armlet.toggle_cooldown_ms = 250;
        settings.heroes.huskar.armlet = HeroArmletOverrideConfig {
            enabled: Some(false),
            toggle_threshold: Some(110),
            predictive_offset: None,
            toggle_cooldown_ms: Some(190),
        };
        settings.heroes.huskar.armlet_toggle_threshold = 120;
        settings.heroes.huskar.armlet_predictive_offset = 150;
        settings.heroes.huskar.armlet_toggle_cooldown_ms = 300;

        let resolved = settings.resolve_armlet_config("npc_dota_hero_huskar");

        assert_eq!(resolved.enabled, false);
        assert_eq!(resolved.toggle_threshold, 110);
        assert_eq!(resolved.predictive_offset, 30);
        assert_eq!(resolved.toggle_cooldown_ms, 190);
    }
}
