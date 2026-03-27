use crate::config::settings::EffectiveArmletConfig;
use crate::config::Settings;
use crate::input::simulation::{armlet_chord, ModifierKey};
use crate::models::GsiWebhookEvent;
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::time::Instant;
use tracing::{debug, info, trace, warn};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArmletDecision {
    Toggle,
    CriticalRetry,
    SkipSafe,
    SkipStunned,
    SkipCooldown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ArmletEvaluation {
    decision: ArmletDecision,
    trigger_point: u32,
    cooldown_remaining_ms: u64,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ArmletReplaySample {
    at_ms: u64,
    health: u32,
    stunned: bool,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ArmletReplayEvent {
    at_ms: u64,
    health: u32,
    decision: ArmletDecision,
    trigger_point: u32,
    cooldown_remaining_ms: u64,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ArmletReplayReport {
    normal_toggles: usize,
    critical_retries: usize,
    cooldown_blocks: usize,
    stun_blocks: usize,
    events: Vec<ArmletReplayEvent>,
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

fn execute_dual_trigger(slot_key: char, cast_modifier: ModifierKey) {
    let started = Instant::now();
    let sequence = plan_dual_trigger_sequence(slot_key, cast_modifier);
    debug!(
        "Armlet dual-trigger starting for '{}' with {:?}: {:?}",
        slot_key, cast_modifier, sequence
    );

    armlet_chord(slot_key, cast_modifier);

    debug!(
        "Armlet dual-trigger finished in {}ms via dedicated worker chord",
        started.elapsed().as_millis()
    );
}

fn next_critical_retry_health(health: u32, threshold: u32) -> Option<u32> {
    if health < threshold / 2 {
        Some(health)
    } else {
        None
    }
}

fn elapsed_since_toggle_ms(last_toggle: Option<Instant>) -> Option<u64> {
    last_toggle.map(|last_time| last_time.elapsed().as_millis() as u64)
}

fn cooldown_ready_for_elapsed(elapsed_since_last_toggle_ms: Option<u64>, cooldown_ms: u64) -> bool {
    match elapsed_since_last_toggle_ms {
        Some(elapsed_ms) => elapsed_ms >= cooldown_ms,
        None => true,
    }
}

fn cooldown_remaining_for_elapsed(
    elapsed_since_last_toggle_ms: Option<u64>,
    cooldown_ms: u64,
) -> u64 {
    match elapsed_since_last_toggle_ms {
        Some(elapsed_ms) => cooldown_ms.saturating_sub(elapsed_ms),
        None => 0,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn cooldown_ready(last_toggle: Option<Instant>, cooldown_ms: u64) -> bool {
    cooldown_ready_for_elapsed(elapsed_since_toggle_ms(last_toggle), cooldown_ms)
}

#[cfg_attr(not(test), allow(dead_code))]
fn cooldown_remaining_ms(last_toggle: Option<Instant>, cooldown_ms: u64) -> u64 {
    cooldown_remaining_for_elapsed(elapsed_since_toggle_ms(last_toggle), cooldown_ms)
}

fn should_force_critical_retry_for_elapsed(
    health: u32,
    threshold: u32,
    last_critical: Option<u32>,
    elapsed_since_last_toggle_ms: Option<u64>,
    cooldown_ms: u64,
) -> bool {
    match last_critical {
        Some(last_critical) => {
            health < threshold / 2
                && health <= last_critical
                && cooldown_ready_for_elapsed(elapsed_since_last_toggle_ms, cooldown_ms)
        }
        None => false,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn should_force_critical_retry(
    health: u32,
    threshold: u32,
    last_critical: Option<u32>,
    last_toggle: Option<Instant>,
    cooldown_ms: u64,
) -> bool {
    should_force_critical_retry_for_elapsed(
        health,
        threshold,
        last_critical,
        elapsed_since_toggle_ms(last_toggle),
        cooldown_ms,
    )
}

fn evaluate_armlet_decision(
    health: u32,
    threshold: u32,
    predictive_offset: u32,
    is_stunned: bool,
    last_critical: Option<u32>,
    elapsed_since_last_toggle_ms: Option<u64>,
    cooldown_ms: u64,
) -> ArmletEvaluation {
    let trigger_point = threshold.saturating_add(predictive_offset);
    let cooldown_remaining_ms =
        cooldown_remaining_for_elapsed(elapsed_since_last_toggle_ms, cooldown_ms);

    if should_force_critical_retry_for_elapsed(
        health,
        threshold,
        last_critical,
        elapsed_since_last_toggle_ms,
        cooldown_ms,
    ) {
        return ArmletEvaluation {
            decision: ArmletDecision::CriticalRetry,
            trigger_point,
            cooldown_remaining_ms,
        };
    }

    if health >= trigger_point {
        return ArmletEvaluation {
            decision: ArmletDecision::SkipSafe,
            trigger_point,
            cooldown_remaining_ms,
        };
    }

    if is_stunned {
        return ArmletEvaluation {
            decision: ArmletDecision::SkipStunned,
            trigger_point,
            cooldown_remaining_ms,
        };
    }

    if cooldown_remaining_ms > 0 {
        return ArmletEvaluation {
            decision: ArmletDecision::SkipCooldown,
            trigger_point,
            cooldown_remaining_ms,
        };
    }

    ArmletEvaluation {
        decision: ArmletDecision::Toggle,
        trigger_point,
        cooldown_remaining_ms,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn simulate_armlet_replay(
    samples: &[ArmletReplaySample],
    config: &EffectiveArmletConfig,
) -> ArmletReplayReport {
    let mut report = ArmletReplayReport::default();
    let mut last_critical = None;
    let mut last_toggle_at_ms = None;

    for sample in samples {
        let elapsed_since_last_toggle_ms =
            last_toggle_at_ms.map(|last_toggle_at| sample.at_ms.saturating_sub(last_toggle_at));
        let evaluation = evaluate_armlet_decision(
            sample.health,
            config.toggle_threshold,
            config.predictive_offset,
            sample.stunned,
            last_critical,
            elapsed_since_last_toggle_ms,
            config.toggle_cooldown_ms,
        );

        report.events.push(ArmletReplayEvent {
            at_ms: sample.at_ms,
            health: sample.health,
            decision: evaluation.decision,
            trigger_point: evaluation.trigger_point,
            cooldown_remaining_ms: evaluation.cooldown_remaining_ms,
        });

        match evaluation.decision {
            ArmletDecision::Toggle => {
                report.normal_toggles += 1;
                last_toggle_at_ms = Some(sample.at_ms);
                last_critical =
                    next_critical_retry_health(sample.health, config.toggle_threshold);
            }
            ArmletDecision::CriticalRetry => {
                report.critical_retries += 1;
                last_toggle_at_ms = Some(sample.at_ms);
                last_critical = None;
            }
            ArmletDecision::SkipCooldown => {
                report.cooldown_blocks += 1;
            }
            ArmletDecision::SkipStunned => {
                report.stun_blocks += 1;
            }
            ArmletDecision::SkipSafe => {
                last_critical = None;
            }
        }
    }

    report
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
    let cooldown_ms = resolved.toggle_cooldown_ms;
    let cast_modifier = resolve_cast_modifier(&resolved);

    let last_critical = *ARMLET_CRITICAL_HP.lock().unwrap();
    let last_toggle_snapshot = *ARMLET_LAST_TOGGLE.lock().unwrap();
    let elapsed_since_last_toggle_ms = elapsed_since_toggle_ms(last_toggle_snapshot);
    let evaluation = evaluate_armlet_decision(
        health,
        threshold,
        resolved.predictive_offset,
        event.hero.is_stunned(),
        last_critical,
        elapsed_since_last_toggle_ms,
        cooldown_ms,
    );

    match evaluation.decision {
        ArmletDecision::CriticalRetry => {
            warn!(
                "Critical HP detected! HP: {} (likely armlet stuck on). Forcing emergency toggle.",
                health
            );
            debug!(
                "Armlet emergency retry: hero={}, health={}, trigger={}, cooldown={}ms, modifier={:?}",
                event.hero.name, health, evaluation.trigger_point, cooldown_ms, cast_modifier
            );

            execute_dual_trigger(slot_key, cast_modifier);

            let mut critical_hp = ARMLET_CRITICAL_HP.lock().unwrap();
            *critical_hp = None;
            drop(critical_hp);

            let mut last_toggle = ARMLET_LAST_TOGGLE.lock().unwrap();
            *last_toggle = Some(Instant::now());
        }
        ArmletDecision::Toggle => {
            info!(
                "Triggering armlet toggle (HP: {} < trigger: {}, base: {}, cooldown: {}ms)",
                health, evaluation.trigger_point, threshold, cooldown_ms
            );
            debug!(
                "Armlet decision: hero={}, health={}, threshold={}, offset={}, cooldown={}ms, modifier={:?}",
                event.hero.name,
                health,
                threshold,
                resolved.predictive_offset,
                cooldown_ms,
                cast_modifier
            );

            execute_dual_trigger(slot_key, cast_modifier);
            let mut last_toggle = ARMLET_LAST_TOGGLE.lock().unwrap();
            *last_toggle = Some(Instant::now());

            let mut critical_hp = ARMLET_CRITICAL_HP.lock().unwrap();
            *critical_hp = next_critical_retry_health(health, threshold);
        }
        ArmletDecision::SkipStunned => {
            debug!(
                "Hero stunned, skipping armlet toggle (HP: {}, trigger: {})",
                health, evaluation.trigger_point
            );
        }
        ArmletDecision::SkipCooldown => {
            debug!(
                "Armlet toggle on cooldown ({}ms remaining, HP: {}, trigger: {})",
                evaluation.cooldown_remaining_ms, health, evaluation.trigger_point
            );
        }
        ArmletDecision::SkipSafe => {
            trace!(
                "Armlet safe: hero={}, health={}, trigger={}",
                event.hero.name, health, evaluation.trigger_point
            );

            if let Ok(mut critical_hp) = ARMLET_CRITICAL_HP.try_lock() {
                if critical_hp.is_some() {
                    debug!("HP recovered to safe levels, resetting critical HP tracker");
                    *critical_hp = None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cooldown_ready, cooldown_remaining_ms, evaluate_armlet_decision, next_critical_retry_health,
        parse_cast_modifier, plan_dual_trigger_sequence, resolve_cast_modifier,
        should_force_critical_retry, simulate_armlet_replay, ArmletDecision, ArmletReplaySample,
        ArmletTriggerStep,
    };
    use crate::config::{
        settings::{ArmletAutomationConfig, EffectiveArmletConfig, HeroArmletOverrideConfig},
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
    fn cooldown_remaining_reports_unexpired_window() {
        let remaining = cooldown_remaining_ms(Some(Instant::now() - Duration::from_millis(75)), 300);

        assert!(remaining >= 200);
        assert!(remaining <= 225);
    }

    #[test]
    fn evaluate_armlet_decision_reports_cooldown_blocks_with_remaining_time() {
        let evaluation = evaluate_armlet_decision(100, 120, 0, false, None, Some(150), 300);

        assert_eq!(evaluation.decision, ArmletDecision::SkipCooldown);
        assert_eq!(evaluation.trigger_point, 120);
        assert_eq!(evaluation.cooldown_remaining_ms, 150);
    }

    #[test]
    fn replay_shows_higher_threshold_triggers_earlier_than_lower_threshold() {
        let samples = [
            ArmletReplaySample {
                at_ms: 0,
                health: 260,
                stunned: false,
            },
            ArmletReplaySample {
                at_ms: 200,
                health: 180,
                stunned: false,
            },
            ArmletReplaySample {
                at_ms: 400,
                health: 140,
                stunned: false,
            },
            ArmletReplaySample {
                at_ms: 700,
                health: 60,
                stunned: false,
            },
        ];
        let conservative = EffectiveArmletConfig {
            enabled: true,
            cast_modifier: "Alt".to_string(),
            toggle_threshold: 80,
            predictive_offset: 0,
            toggle_cooldown_ms: 150,
        };
        let aggressive = EffectiveArmletConfig {
            toggle_threshold: 150,
            ..conservative.clone()
        };

        let conservative_report = simulate_armlet_replay(&samples, &conservative);
        let aggressive_report = simulate_armlet_replay(&samples, &aggressive);

        assert_eq!(conservative_report.normal_toggles, 1);
        assert_eq!(aggressive_report.normal_toggles, 2);
        assert_eq!(
            aggressive_report.events[2].decision,
            ArmletDecision::Toggle
        );
        assert_eq!(
            conservative_report.events[2].decision,
            ArmletDecision::SkipSafe
        );
    }

    #[test]
    fn replay_shows_shorter_cooldown_handles_more_burst_windows() {
        let samples = [
            ArmletReplaySample {
                at_ms: 0,
                health: 110,
                stunned: false,
            },
            ArmletReplaySample {
                at_ms: 120,
                health: 95,
                stunned: false,
            },
            ArmletReplaySample {
                at_ms: 220,
                health: 85,
                stunned: false,
            },
            ArmletReplaySample {
                at_ms: 420,
                health: 80,
                stunned: false,
            },
        ];
        let slow = EffectiveArmletConfig {
            enabled: true,
            cast_modifier: "Alt".to_string(),
            toggle_threshold: 120,
            predictive_offset: 0,
            toggle_cooldown_ms: 300,
        };
        let fast = EffectiveArmletConfig {
            toggle_cooldown_ms: 100,
            ..slow.clone()
        };

        let slow_report = simulate_armlet_replay(&samples, &slow);
        let fast_report = simulate_armlet_replay(&samples, &fast);

        assert_eq!(slow_report.normal_toggles, 2);
        assert_eq!(slow_report.cooldown_blocks, 2);
        assert_eq!(fast_report.normal_toggles, 4);
        assert_eq!(fast_report.cooldown_blocks, 0);
    }

    #[test]
    fn replay_surfaces_critical_retry_after_initial_low_hp_toggle() {
        let samples = [
            ArmletReplaySample {
                at_ms: 0,
                health: 40,
                stunned: false,
            },
            ArmletReplaySample {
                at_ms: 350,
                health: 20,
                stunned: false,
            },
        ];
        let config = EffectiveArmletConfig {
            enabled: true,
            cast_modifier: "Alt".to_string(),
            toggle_threshold: 120,
            predictive_offset: 0,
            toggle_cooldown_ms: 300,
        };

        let report = simulate_armlet_replay(&samples, &config);

        assert_eq!(report.normal_toggles, 1);
        assert_eq!(report.critical_retries, 1);
        assert_eq!(report.events[1].decision, ArmletDecision::CriticalRetry);
    }

    #[test]
    #[ignore = "Diagnostic matrix for manual armlet tuning sessions"]
    fn print_armlet_tuning_matrix_for_burst_scenarios() {
        let scenarios = vec![
            (
                "steady burst",
                vec![
                    ArmletReplaySample {
                        at_ms: 0,
                        health: 150,
                        stunned: false,
                    },
                    ArmletReplaySample {
                        at_ms: 120,
                        health: 110,
                        stunned: false,
                    },
                    ArmletReplaySample {
                        at_ms: 240,
                        health: 70,
                        stunned: false,
                    },
                    ArmletReplaySample {
                        at_ms: 420,
                        health: 35,
                        stunned: false,
                    },
                ],
            ),
            (
                "multi source spike",
                vec![
                    ArmletReplaySample {
                        at_ms: 0,
                        health: 220,
                        stunned: false,
                    },
                    ArmletReplaySample {
                        at_ms: 80,
                        health: 130,
                        stunned: false,
                    },
                    ArmletReplaySample {
                        at_ms: 160,
                        health: 65,
                        stunned: false,
                    },
                    ArmletReplaySample {
                        at_ms: 340,
                        health: 25,
                        stunned: false,
                    },
                ],
            ),
        ];
        let thresholds = [50, 80, 120, 150];
        let cooldowns = [100, 150, 250, 300];

        for (name, samples) in scenarios {
            println!("scenario: {}", name);
            for threshold in thresholds {
                for cooldown in cooldowns {
                    let report = simulate_armlet_replay(
                        &samples,
                        &EffectiveArmletConfig {
                            enabled: true,
                            cast_modifier: "Alt".to_string(),
                            toggle_threshold: threshold,
                            predictive_offset: 0,
                            toggle_cooldown_ms: cooldown,
                        },
                    );

                    println!(
                        "  threshold={} cooldown={} => toggles={}, critical_retries={}, cooldown_blocks={}, stun_blocks={}",
                        threshold,
                        cooldown,
                        report.normal_toggles,
                        report.critical_retries,
                        report.cooldown_blocks,
                        report.stun_blocks
                    );
                }
            }
        }
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
