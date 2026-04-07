use crate::config::settings::{ArmletRoshanConfig, EffectiveArmletConfig};
use crate::config::Settings;
use crate::input::simulation::{armlet_chord, ModifierKey};
use crate::models::GsiWebhookEvent;
use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use tracing::{debug, info, trace, warn};

lazy_static! {
    static ref ARMLET_LAST_TOGGLE: Mutex<Option<Instant>> = Mutex::new(None);
    static ref ARMLET_CRITICAL_HP: Mutex<Option<u32>> = Mutex::new(None);
    static ref ARMLET_ROSHAN_STATE: Mutex<ArmletRoshanState> =
        Mutex::new(ArmletRoshanState::default());
}

static ARMLET_ROSHAN_MODE_ARMED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArmletTriggerStep {
    QuickCast(char),
    ModifierDown(ModifierKey),
    ModifierUp(ModifierKey),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArmletDecision {
    Toggle,
    ToggleRoshan,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct RoshanHitSample {
    at_ms: u64,
    damage: u32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ArmletRoshanState {
    last_health: Option<u32>,
    latest_observed_damage: Option<u32>,
    samples: VecDeque<RoshanHitSample>,
    was_stunned_last_tick: bool,
    awaiting_post_stun_hit: bool,
    stun_recovery_estimate_damage: Option<u32>,
    stun_recovery_started_at_ms: Option<u64>,
}

impl ArmletRoshanState {
    #[cfg_attr(not(test), allow(dead_code))]
    fn armed() -> Self {
        Self::default()
    }

    fn reset_learning(&mut self) {
        self.last_health = None;
        self.latest_observed_damage = None;
        self.samples.clear();
        self.was_stunned_last_tick = false;
        self.awaiting_post_stun_hit = false;
        self.stun_recovery_estimate_damage = None;
        self.stun_recovery_started_at_ms = None;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoshanArmletTrigger {
    EmergencyFallback {
        observed_damage: u32,
        lethal_zone: u32,
    },
    LearnedHit {
        predicted_damage: u32,
        lethal_zone: u32,
        sample_count: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoshanRecoveryAction {
    None,
    ProtectNow,
    AwaitNextHit { predicted_damage: u32 },
    TriggerDeferredHit { observed_damage: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoshanResetReason {
    HeroDied,
    ArmletDisabled,
    ArmletMissing,
    RoshanModeDisarmed,
    ModeToggled,
}

impl RoshanResetReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::HeroDied => "hero died",
            Self::ArmletDisabled => "armlet automation disabled",
            Self::ArmletMissing => "armlet item missing",
            Self::RoshanModeDisarmed => "roshan mode inactive",
            Self::ModeToggled => "roshan mode toggled",
        }
    }
}

fn should_log_roshan_skip_context(
    health: u32,
    trigger_point: u32,
    observed_damage: Option<u32>,
    predicted_damage: Option<u32>,
    emergency_margin_hp: u32,
) -> bool {
    let lethal_zone = observed_damage
        .into_iter()
        .chain(predicted_damage)
        .map(|damage| damage.saturating_add(emergency_margin_hp))
        .max()
        .unwrap_or_default();
    let near_danger_floor = trigger_point.max(lethal_zone);

    health <= near_danger_floor.saturating_add(120)
}

pub fn is_roshan_mode_armed() -> bool {
    ARMLET_ROSHAN_MODE_ARMED.load(Ordering::SeqCst)
}

pub fn set_roshan_mode_armed(armed: bool) -> bool {
    let previous = ARMLET_ROSHAN_MODE_ARMED.swap(armed, Ordering::SeqCst);
    if previous != armed {
        if let Ok(mut state) = ARMLET_ROSHAN_STATE.lock() {
            clear_roshan_learning_state_with_reason(&mut state, RoshanResetReason::ModeToggled);
        }
        info!(
            "Armlet Roshan mode {} (reason: {})",
            if armed { "armed" } else { "disarmed" },
            RoshanResetReason::ModeToggled.as_str()
        );
    }

    armed
}

pub fn toggle_roshan_mode() -> bool {
    set_roshan_mode_armed(!is_roshan_mode_armed())
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

fn prune_stale_roshan_samples(
    state: &mut ArmletRoshanState,
    now_ms: u64,
    config: &ArmletRoshanConfig,
) {
    if let Some(last_sample) = state.samples.back() {
        if now_ms.saturating_sub(last_sample.at_ms) > config.stale_reset_ms {
            state.reset_learning();
            return;
        }
    }

    while let Some(sample) = state.samples.front() {
        if now_ms.saturating_sub(sample.at_ms) > config.learning_window_ms {
            state.samples.pop_front();
        } else {
            break;
        }
    }
}

fn record_roshan_health_sample(
    state: &mut ArmletRoshanState,
    now_ms: u64,
    health: u32,
    config: &ArmletRoshanConfig,
) -> Option<u32> {
    prune_stale_roshan_samples(state, now_ms, config);

    let observed_damage = state
        .last_health
        .and_then(|last_health| last_health.checked_sub(health))
        .filter(|damage| *damage >= config.min_sample_damage);

    state.last_health = Some(health);
    state.latest_observed_damage = observed_damage;

    if let Some(damage) = observed_damage {
        state.samples.push_back(RoshanHitSample {
            at_ms: now_ms,
            damage,
        });
    }

    observed_damage
}

fn evaluate_roshan_trigger(
    health: u32,
    now_ms: u64,
    state: &ArmletRoshanState,
    config: &ArmletRoshanConfig,
) -> Option<RoshanArmletTrigger> {
    if !config.enabled {
        return None;
    }

    let recent_samples: Vec<_> = state
        .samples
        .iter()
        .filter(|sample| now_ms.saturating_sub(sample.at_ms) <= config.learning_window_ms)
        .collect();

    if recent_samples.len() >= config.min_confidence_hits {
        let predicted_damage = recent_samples.iter().map(|sample| sample.damage).max()?;
        let lethal_zone = predicted_damage.saturating_add(config.emergency_margin_hp);
        if health <= lethal_zone {
            return Some(RoshanArmletTrigger::LearnedHit {
                predicted_damage,
                lethal_zone,
                sample_count: recent_samples.len(),
            });
        }
    }

    let observed_damage = state.latest_observed_damage?;
    let lethal_zone = observed_damage.saturating_add(config.emergency_margin_hp);
    if health <= lethal_zone {
        Some(RoshanArmletTrigger::EmergencyFallback {
            observed_damage,
            lethal_zone,
        })
    } else {
        None
    }
}

fn best_roshan_hit_estimate(
    state: &ArmletRoshanState,
    now_ms: u64,
    config: &ArmletRoshanConfig,
) -> Option<u32> {
    let recent_max = state
        .samples
        .iter()
        .filter(|sample| now_ms.saturating_sub(sample.at_ms) <= config.learning_window_ms)
        .map(|sample| sample.damage)
        .max();

    recent_max.or(state.latest_observed_damage)
}

fn clear_roshan_recovery_defer(state: &mut ArmletRoshanState) {
    state.awaiting_post_stun_hit = false;
    state.stun_recovery_estimate_damage = None;
    state.stun_recovery_started_at_ms = None;
}

fn evaluate_roshan_stun_recovery(
    state: &mut ArmletRoshanState,
    health: u32,
    is_stunned: bool,
    just_recovered_from_stun: bool,
    now_ms: u64,
    config: &ArmletRoshanConfig,
) -> RoshanRecoveryAction {
    if is_stunned {
        clear_roshan_recovery_defer(state);
        state.was_stunned_last_tick = true;
        return RoshanRecoveryAction::None;
    }

    if state.awaiting_post_stun_hit {
        if let Some(observed_damage) = state.latest_observed_damage {
            if observed_damage >= config.min_sample_damage {
                clear_roshan_recovery_defer(state);
                state.was_stunned_last_tick = false;
                return RoshanRecoveryAction::TriggerDeferredHit { observed_damage };
            }
        }

        state.was_stunned_last_tick = false;
        return RoshanRecoveryAction::None;
    }

    if !just_recovered_from_stun {
        state.was_stunned_last_tick = false;
        return RoshanRecoveryAction::None;
    }

    let Some(predicted_damage) = best_roshan_hit_estimate(state, now_ms, config) else {
        state.was_stunned_last_tick = false;
        return RoshanRecoveryAction::None;
    };

    state.was_stunned_last_tick = false;
    if health <= predicted_damage.saturating_add(config.emergency_margin_hp) {
        RoshanRecoveryAction::ProtectNow
    } else {
        state.awaiting_post_stun_hit = true;
        state.stun_recovery_estimate_damage = Some(predicted_damage);
        state.stun_recovery_started_at_ms = Some(now_ms);
        RoshanRecoveryAction::AwaitNextHit { predicted_damage }
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
            ArmletDecision::Toggle | ArmletDecision::ToggleRoshan => {
                report.normal_toggles += 1;
                last_toggle_at_ms = Some(sample.at_ms);
                last_critical = next_critical_retry_health(sample.health, config.toggle_threshold);
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

fn clear_roshan_learning_state(state: &mut ArmletRoshanState) {
    state.reset_learning();
}

fn clear_roshan_learning_state_with_reason(
    state: &mut ArmletRoshanState,
    reason: RoshanResetReason,
) {
    if *state != ArmletRoshanState::default() {
        info!(
            "Clearing Armlet Roshan learning state ({})",
            reason.as_str()
        );
    }
    clear_roshan_learning_state(state);
}

fn roshan_prediction_summary(
    state: &ArmletRoshanState,
    now_ms: u64,
    config: &ArmletRoshanConfig,
) -> (Option<u32>, usize) {
    let mut predicted_damage = None;
    let mut recent_sample_count = 0;

    for sample in state
        .samples
        .iter()
        .filter(|sample| now_ms.saturating_sub(sample.at_ms) <= config.learning_window_ms)
    {
        recent_sample_count += 1;
        predicted_damage = Some(predicted_damage.map_or(sample.damage, |current_max: u32| {
            current_max.max(sample.damage)
        }));
    }

    (predicted_damage, recent_sample_count)
}

fn maybe_log_roshan_skip_context(
    evaluation: ArmletEvaluation,
    health: u32,
    threshold: u32,
    predictive_offset: u32,
    is_stunned: bool,
    state: &ArmletRoshanState,
    recovery_action: RoshanRecoveryAction,
    now_ms: u64,
    config: &ArmletRoshanConfig,
) {
    let observed_damage = state.latest_observed_damage;
    let (predicted_damage, recent_sample_count) = roshan_prediction_summary(state, now_ms, config);
    let predicted_context_damage = state.stun_recovery_estimate_damage.or(predicted_damage);

    if !should_log_roshan_skip_context(
        health,
        evaluation.trigger_point,
        observed_damage,
        predicted_context_damage,
        config.emergency_margin_hp,
    ) {
        return;
    }

    debug!(
        "Armlet Roshan no-toggle context: hp={}, threshold={}, offset={}, trigger={}, cooldown={}ms, observed_hit={:?}, predicted_hit={:?}, samples={}, awaiting_post_stun_hit={}, stunned={}",
        health,
        threshold,
        predictive_offset,
        evaluation.trigger_point,
        evaluation.cooldown_remaining_ms,
        observed_damage,
        predicted_context_damage,
        recent_sample_count,
        state.awaiting_post_stun_hit,
        is_stunned
    );

    match evaluation.decision {
        ArmletDecision::SkipCooldown => {
            debug!(
                "Skipping Armlet Roshan protection near danger: cooldown {}ms remaining",
                evaluation.cooldown_remaining_ms,
            );
        }
        ArmletDecision::SkipStunned => {
            debug!("Skipping Armlet Roshan protection near danger: still stunned");
        }
        ArmletDecision::SkipSafe => match recovery_action {
            RoshanRecoveryAction::AwaitNextHit { predicted_damage } => {
                debug!(
                    "Skipping Armlet Roshan protection near danger: waiting for deferred post-stun hit re-sync (predicted hit: {})",
                    predicted_damage
                );
            }
            RoshanRecoveryAction::None if state.awaiting_post_stun_hit => {
                debug!(
                    "Skipping Armlet Roshan protection near danger: deferred post-stun wait still active"
                );
            }
            RoshanRecoveryAction::None => {
                if recent_sample_count < config.min_confidence_hits && observed_damage.is_none() {
                    debug!(
                        "Skipping Armlet Roshan protection near danger: insufficient sample confidence (samples: {}/{}, predicted hit: {:?})",
                        recent_sample_count,
                        config.min_confidence_hits,
                        predicted_damage
                    );
                } else if recent_sample_count >= config.min_confidence_hits {
                    if let Some(predicted_damage) = predicted_damage {
                        debug!(
                            "Skipping Armlet Roshan protection near danger: HP {} still above learned lethal zone {} (predicted hit: {}, samples: {})",
                            health,
                            predicted_damage.saturating_add(config.emergency_margin_hp),
                            predicted_damage,
                            recent_sample_count
                        );
                    }
                } else if let Some(observed_damage) = observed_damage {
                    debug!(
                        "Skipping Armlet Roshan protection near danger: HP {} still above emergency fallback zone {} (observed hit: {})",
                        health,
                        observed_damage.saturating_add(config.emergency_margin_hp),
                        observed_damage
                    );
                } else {
                    debug!(
                        "Skipping Armlet Roshan protection near danger: insufficient sample confidence (samples: {}/{})",
                        recent_sample_count, config.min_confidence_hits
                    );
                }
            }
            _ => {}
        },
        ArmletDecision::Toggle | ArmletDecision::ToggleRoshan | ArmletDecision::CriticalRetry => {}
    }
}

pub fn maybe_toggle(event: &GsiWebhookEvent, settings: &Settings) {
    if !event.hero.is_alive() {
        if let Ok(mut roshan_state) = ARMLET_ROSHAN_STATE.lock() {
            clear_roshan_learning_state_with_reason(&mut roshan_state, RoshanResetReason::HeroDied);
        }
        return;
    }

    let resolved = settings.resolve_armlet_config(&event.hero.name);
    if !resolved.enabled {
        if let Ok(mut roshan_state) = ARMLET_ROSHAN_STATE.lock() {
            clear_roshan_learning_state_with_reason(
                &mut roshan_state,
                RoshanResetReason::ArmletDisabled,
            );
        }
        return;
    }

    let Some(slot_key) = find_armlet_slot_key(event, settings) else {
        if let Ok(mut roshan_state) = ARMLET_ROSHAN_STATE.lock() {
            clear_roshan_learning_state_with_reason(
                &mut roshan_state,
                RoshanResetReason::ArmletMissing,
            );
        }
        return;
    };

    let health = event.hero.health;
    let threshold = resolved.toggle_threshold;
    let cooldown_ms = resolved.toggle_cooldown_ms;
    let cast_modifier = resolve_cast_modifier(&resolved);

    let last_critical = *ARMLET_CRITICAL_HP.lock().unwrap();
    let last_toggle_snapshot = *ARMLET_LAST_TOGGLE.lock().unwrap();
    let elapsed_since_last_toggle_ms = elapsed_since_toggle_ms(last_toggle_snapshot);
    let mut evaluation = evaluate_armlet_decision(
        health,
        threshold,
        resolved.predictive_offset,
        event.hero.is_stunned(),
        last_critical,
        elapsed_since_last_toggle_ms,
        cooldown_ms,
    );

    let roshan_active = resolved.roshan.enabled && is_roshan_mode_armed();

    if roshan_active {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or_default();

        if let Ok(mut roshan_state) = ARMLET_ROSHAN_STATE.lock() {
            let was_stunned_last_tick = roshan_state.was_stunned_last_tick;
            record_roshan_health_sample(&mut roshan_state, now_ms, health, &resolved.roshan);

            if matches!(
                evaluation.decision,
                ArmletDecision::Toggle | ArmletDecision::CriticalRetry
            ) {
                roshan_state.was_stunned_last_tick = event.hero.is_stunned();
                if event.hero.is_stunned() {
                    clear_roshan_recovery_defer(&mut roshan_state);
                }
            } else {
                let roshan_recovery_action = evaluate_roshan_stun_recovery(
                    &mut roshan_state,
                    health,
                    event.hero.is_stunned(),
                    was_stunned_last_tick && !event.hero.is_stunned(),
                    now_ms,
                    &resolved.roshan,
                );
                let (predicted_damage, recent_sample_count) =
                    roshan_prediction_summary(&roshan_state, now_ms, &resolved.roshan);

                if evaluation.decision == ArmletDecision::SkipSafe {
                    match roshan_recovery_action {
                        RoshanRecoveryAction::ProtectNow => {
                            info!(
                                "Triggering immediate Roshan protection after stun recovery (HP: {}, threshold: {}, offset: {}, trigger: {}, cooldown: {}ms, predicted hit: {:?}, samples: {})",
                                health,
                                threshold,
                                resolved.predictive_offset,
                                evaluation.trigger_point,
                                evaluation.cooldown_remaining_ms,
                                predicted_damage,
                                recent_sample_count
                            );
                            evaluation.decision = ArmletDecision::ToggleRoshan;
                        }
                        RoshanRecoveryAction::AwaitNextHit { predicted_damage } => {
                            maybe_log_roshan_skip_context(
                                evaluation,
                                health,
                                threshold,
                                resolved.predictive_offset,
                                event.hero.is_stunned(),
                                &roshan_state,
                                RoshanRecoveryAction::AwaitNextHit { predicted_damage },
                                now_ms,
                                &resolved.roshan,
                            );
                            debug!(
                                "Deferring Roshan toggle after stun recovery; waiting for next hit (HP: {}, threshold: {}, offset: {}, trigger: {}, cooldown: {}ms, predicted damage: {}, samples: {})",
                                health,
                                threshold,
                                resolved.predictive_offset,
                                evaluation.trigger_point,
                                evaluation.cooldown_remaining_ms,
                                predicted_damage,
                                recent_sample_count
                            );
                        }
                        RoshanRecoveryAction::TriggerDeferredHit { observed_damage } => {
                            info!(
                                "Triggering deferred Roshan re-sync toggle after stun recovery hit (HP: {}, threshold: {}, offset: {}, trigger: {}, cooldown: {}ms, observed damage: {}, predicted hit: {:?}, samples: {})",
                                health,
                                threshold,
                                resolved.predictive_offset,
                                evaluation.trigger_point,
                                evaluation.cooldown_remaining_ms,
                                observed_damage,
                                predicted_damage,
                                recent_sample_count
                            );
                            evaluation.decision = ArmletDecision::ToggleRoshan;
                        }
                        RoshanRecoveryAction::None => {
                            if let Some(trigger) = evaluate_roshan_trigger(
                                health,
                                now_ms,
                                &roshan_state,
                                &resolved.roshan,
                            ) {
                                match trigger {
                                    RoshanArmletTrigger::EmergencyFallback {
                                        observed_damage,
                                        lethal_zone,
                                    } => {
                                        info!(
                                            "Triggering armlet Roshan emergency fallback (HP: {} <= lethal zone: {}, threshold: {}, offset: {}, trigger: {}, cooldown: {}ms, observed hit: {}, predicted hit: {:?}, samples: {})",
                                            health,
                                            lethal_zone,
                                            threshold,
                                            resolved.predictive_offset,
                                            evaluation.trigger_point,
                                            evaluation.cooldown_remaining_ms,
                                            observed_damage,
                                            predicted_damage,
                                            recent_sample_count
                                        );
                                    }
                                    RoshanArmletTrigger::LearnedHit {
                                        predicted_damage,
                                        lethal_zone,
                                        sample_count,
                                    } => {
                                        info!(
                                            "Triggering armlet Roshan learned-hit protection (HP: {} <= lethal zone: {}, threshold: {}, offset: {}, trigger: {}, cooldown: {}ms, predicted hit: {}, samples: {})",
                                            health,
                                            lethal_zone,
                                            threshold,
                                            resolved.predictive_offset,
                                            evaluation.trigger_point,
                                            evaluation.cooldown_remaining_ms,
                                            predicted_damage,
                                            sample_count
                                        );
                                    }
                                }

                                evaluation.decision = ArmletDecision::ToggleRoshan;
                            } else {
                                maybe_log_roshan_skip_context(
                                    evaluation,
                                    health,
                                    threshold,
                                    resolved.predictive_offset,
                                    event.hero.is_stunned(),
                                    &roshan_state,
                                    RoshanRecoveryAction::None,
                                    now_ms,
                                    &resolved.roshan,
                                );
                            }
                        }
                    }
                } else {
                    maybe_log_roshan_skip_context(
                        evaluation,
                        health,
                        threshold,
                        resolved.predictive_offset,
                        event.hero.is_stunned(),
                        &roshan_state,
                        roshan_recovery_action,
                        now_ms,
                        &resolved.roshan,
                    );
                    trace!(
                        "Roshan recovery state updated without immediate toggle decision: {:?}",
                        roshan_recovery_action
                    );
                }
            }
        }
    } else if let Ok(mut roshan_state) = ARMLET_ROSHAN_STATE.lock() {
        clear_roshan_learning_state_with_reason(
            &mut roshan_state,
            RoshanResetReason::RoshanModeDisarmed,
        );
    }

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
        ArmletDecision::ToggleRoshan => {
            debug!(
                "Armlet Roshan decision: hero={}, health={}, threshold={}, offset={}, cooldown={}ms, modifier={:?}",
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
                event.hero.name,
                health,
                evaluation.trigger_point
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
        clear_roshan_learning_state, cooldown_ready, cooldown_remaining_ms,
        evaluate_armlet_decision, evaluate_roshan_stun_recovery, evaluate_roshan_trigger,
        next_critical_retry_health, parse_cast_modifier, plan_dual_trigger_sequence,
        record_roshan_health_sample, resolve_cast_modifier, should_force_critical_retry,
        should_log_roshan_skip_context, simulate_armlet_replay, ArmletDecision, ArmletReplaySample,
        ArmletRoshanConfig, ArmletRoshanState, ArmletTriggerStep, RoshanArmletTrigger,
        RoshanRecoveryAction, RoshanResetReason,
    };
    use crate::config::{
        settings::{ArmletAutomationConfig, EffectiveArmletConfig, HeroArmletOverrideConfig},
        Settings,
    };
    use crate::input::simulation::ModifierKey;
    use std::time::{Duration, Instant};

    fn roshan_test_config() -> ArmletRoshanConfig {
        ArmletRoshanConfig {
            enabled: true,
            toggle_key: "Insert".to_string(),
            emergency_margin_hp: 60,
            learning_window_ms: 5_000,
            min_confidence_hits: 2,
            min_sample_damage: 80,
            stale_reset_ms: 6_000,
        }
    }

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
            roshan: ArmletRoshanConfig::default(),
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

        assert!(!should_force_critical_retry(1, 120, Some(1), just_now, 300,));
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
        let remaining =
            cooldown_remaining_ms(Some(Instant::now() - Duration::from_millis(75)), 300);

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
            roshan: ArmletRoshanConfig::default(),
        };
        let aggressive = EffectiveArmletConfig {
            toggle_threshold: 150,
            ..conservative.clone()
        };

        let conservative_report = simulate_armlet_replay(&samples, &conservative);
        let aggressive_report = simulate_armlet_replay(&samples, &aggressive);

        assert_eq!(conservative_report.normal_toggles, 1);
        assert_eq!(aggressive_report.normal_toggles, 2);
        assert_eq!(aggressive_report.events[2].decision, ArmletDecision::Toggle);
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
            roshan: ArmletRoshanConfig::default(),
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
            roshan: ArmletRoshanConfig::default(),
        };

        let report = simulate_armlet_replay(&samples, &config);

        assert_eq!(report.normal_toggles, 1);
        assert_eq!(report.critical_retries, 1);
        assert_eq!(report.events[1].decision, ArmletDecision::CriticalRetry);
    }

    #[test]
    fn roshan_mode_triggers_on_first_large_hit_when_health_enters_emergency_margin() {
        let mut state = ArmletRoshanState::armed();
        let config = ArmletRoshanConfig {
            enabled: true,
            toggle_key: "Insert".to_string(),
            emergency_margin_hp: 60,
            learning_window_ms: 5_000,
            min_confidence_hits: 2,
            min_sample_damage: 80,
            stale_reset_ms: 6_000,
        };

        assert_eq!(
            record_roshan_health_sample(&mut state, 1_000, 320, &config),
            None
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 1_200, 120, &config),
            Some(200)
        );

        assert_eq!(
            evaluate_roshan_trigger(120, 1_200, &state, &config),
            Some(RoshanArmletTrigger::EmergencyFallback {
                observed_damage: 200,
                lethal_zone: 260,
            })
        );
    }

    #[test]
    fn roshan_mode_uses_largest_recent_hit_once_confident() {
        let mut state = ArmletRoshanState::armed();
        let config = ArmletRoshanConfig {
            enabled: true,
            toggle_key: "Insert".to_string(),
            emergency_margin_hp: 60,
            learning_window_ms: 5_000,
            min_confidence_hits: 2,
            min_sample_damage: 80,
            stale_reset_ms: 6_000,
        };

        assert_eq!(
            record_roshan_health_sample(&mut state, 1_000, 500, &config),
            None
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 1_200, 360, &config),
            Some(140)
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 2_200, 210, &config),
            Some(150)
        );

        assert_eq!(
            evaluate_roshan_trigger(190, 2_200, &state, &config),
            Some(RoshanArmletTrigger::LearnedHit {
                predicted_damage: 150,
                lethal_zone: 210,
                sample_count: 2,
            })
        );
    }

    #[test]
    fn clearing_roshan_learning_state_resets_stale_health_and_samples() {
        let mut state = ArmletRoshanState::armed();
        let config = roshan_test_config();

        assert_eq!(
            record_roshan_health_sample(&mut state, 1_000, 500, &config),
            None
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 1_200, 360, &config),
            Some(140)
        );
        assert_eq!(state.last_health, Some(360));
        assert_eq!(state.latest_observed_damage, Some(140));
        assert_eq!(state.samples.len(), 1);

        clear_roshan_learning_state(&mut state);

        assert_eq!(state.last_health, None);
        assert_eq!(state.latest_observed_damage, None);
        assert!(state.samples.is_empty());
    }

    #[test]
    fn roshan_stun_recovery_defers_toggle_until_next_valid_hit_when_one_more_hit_is_survivable() {
        let config = roshan_test_config();
        let mut state = ArmletRoshanState::armed();

        assert_eq!(
            record_roshan_health_sample(&mut state, 1_000, 520, &config),
            None
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 1_200, 380, &config),
            Some(140)
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 2_200, 230, &config),
            Some(150)
        );

        let recovery = evaluate_roshan_stun_recovery(&mut state, 230, false, true, 2_300, &config);

        assert_eq!(
            recovery,
            RoshanRecoveryAction::AwaitNextHit {
                predicted_damage: 150
            }
        );
        assert!(state.awaiting_post_stun_hit);
    }

    #[test]
    fn roshan_stun_recovery_resyncs_on_first_valid_hit_after_deferral() {
        let config = roshan_test_config();
        let mut state = ArmletRoshanState::armed();
        state.awaiting_post_stun_hit = true;
        state.stun_recovery_estimate_damage = Some(150);

        assert_eq!(
            record_roshan_health_sample(&mut state, 3_000, 230, &config),
            None
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 3_250, 80, &config),
            Some(150)
        );

        let recovery = evaluate_roshan_stun_recovery(&mut state, 80, false, false, 3_250, &config);

        assert_eq!(
            recovery,
            RoshanRecoveryAction::TriggerDeferredHit {
                observed_damage: 150
            }
        );
        assert!(!state.awaiting_post_stun_hit);
    }

    #[test]
    fn roshan_stun_recovery_does_not_defer_when_recovery_is_already_lethal() {
        let config = roshan_test_config();
        let mut state = ArmletRoshanState::armed();

        assert_eq!(
            record_roshan_health_sample(&mut state, 1_000, 500, &config),
            None
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 1_200, 360, &config),
            Some(140)
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 2_200, 210, &config),
            Some(150)
        );

        let recovery = evaluate_roshan_stun_recovery(&mut state, 200, false, true, 2_300, &config);

        assert_eq!(recovery, RoshanRecoveryAction::ProtectNow);
        assert!(!state.awaiting_post_stun_hit);
    }

    #[test]
    fn roshan_stun_recovery_ignores_small_damage_while_waiting_for_next_hit() {
        let config = roshan_test_config();
        let mut state = ArmletRoshanState::armed();
        state.awaiting_post_stun_hit = true;
        state.stun_recovery_estimate_damage = Some(150);

        assert_eq!(
            record_roshan_health_sample(&mut state, 4_000, 220, &config),
            None
        );
        assert_eq!(
            record_roshan_health_sample(&mut state, 4_200, 180, &config),
            None
        );

        let recovery = evaluate_roshan_stun_recovery(&mut state, 180, false, false, 4_200, &config);

        assert_eq!(recovery, RoshanRecoveryAction::None);
        assert!(state.awaiting_post_stun_hit);
    }

    #[test]
    fn clearing_roshan_learning_state_resets_stun_recovery_defer_fields() {
        let mut state = ArmletRoshanState::armed();
        state.awaiting_post_stun_hit = true;
        state.stun_recovery_estimate_damage = Some(150);
        state.stun_recovery_started_at_ms = Some(2_300);
        state.was_stunned_last_tick = true;

        clear_roshan_learning_state(&mut state);

        assert_eq!(state.last_health, None);
        assert_eq!(state.latest_observed_damage, None);
        assert!(state.samples.is_empty());
        assert!(!state.awaiting_post_stun_hit);
        assert_eq!(state.stun_recovery_estimate_damage, None);
        assert_eq!(state.stun_recovery_started_at_ms, None);
        assert!(!state.was_stunned_last_tick);
    }

    #[test]
    fn roshan_reset_reason_strings_match_expected_logs() {
        assert_eq!(RoshanResetReason::HeroDied.as_str(), "hero died");
        assert_eq!(
            RoshanResetReason::ArmletDisabled.as_str(),
            "armlet automation disabled"
        );
        assert_eq!(
            RoshanResetReason::ArmletMissing.as_str(),
            "armlet item missing"
        );
        assert_eq!(
            RoshanResetReason::RoshanModeDisarmed.as_str(),
            "roshan mode inactive"
        );
        assert_eq!(
            RoshanResetReason::ModeToggled.as_str(),
            "roshan mode toggled"
        );
    }

    #[test]
    fn roshan_skip_logging_window_stays_quiet_when_health_is_far_above_danger() {
        assert!(!should_log_roshan_skip_context(
            620,
            270,
            None,
            Some(140),
            60,
        ));
    }

    #[test]
    fn roshan_skip_logging_window_turns_on_near_lethal_zone() {
        assert!(should_log_roshan_skip_context(
            340,
            270,
            Some(120),
            None,
            60,
        ));
    }

    #[test]
    fn roshan_skip_logging_window_uses_predicted_damage_when_present() {
        assert!(should_log_roshan_skip_context(315, 270, None, Some(80), 60,));
    }

    #[test]
    fn roshan_skip_logging_window_stays_quiet_until_trigger_band_is_close() {
        assert!(!should_log_roshan_skip_context(
            500,
            270,
            Some(120),
            None,
            60,
        ));
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
                            roshan: ArmletRoshanConfig::default(),
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
            roshan: ArmletRoshanConfig::default(),
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
