use crate::actions::heroes::meepo_state::MeepoObservedState;
use crate::config::settings::MeepoFarmAssistConfig;
use lazy_static::lazy_static;
use std::sync::Mutex;
#[cfg(test)]
use std::sync::OnceLock;
use std::time::{Duration, Instant};

lazy_static! {
    static ref MEEPO_MACRO_STATE: Mutex<MeepoMacroRuntimeState> =
        Mutex::new(MeepoMacroRuntimeState::default());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeepoMacroMode {
    Inactive,
    Armed,
    Suspended(MeepoMacroSuspendReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeepoMacroSuspendReason {
    Danger,
    Disabled,
    HeroChanged,
    ManualCombo,
    UnableToCast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeepoMacroStatusSnapshot {
    pub mode: MeepoMacroMode,
    pub pulses_executed: u64,
}

#[derive(Debug)]
struct MeepoMacroRuntimeState {
    mode: MeepoMacroMode,
    last_pulse_at: Option<Instant>,
    resume_after_manual_combo_at: Option<Instant>,
    pulses_executed: u64,
}

impl Default for MeepoMacroRuntimeState {
    fn default() -> Self {
        Self {
            mode: MeepoMacroMode::Inactive,
            last_pulse_at: None,
            resume_after_manual_combo_at: None,
            pulses_executed: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeepoFarmPulseDecision {
    Skip,
    Run,
}

fn pulse_is_safe(observed: &MeepoObservedState, config: &MeepoFarmAssistConfig) -> bool {
    observed.alive
        && !observed.stunned
        && !observed.silenced
        && observed.poof_ready
        && observed.health_percent >= config.minimum_health_percent
        && observed.mana_percent >= config.minimum_mana_percent
        && (!config.suspend_on_danger || !observed.in_danger)
}

pub fn latest_meepo_macro_status() -> MeepoMacroStatusSnapshot {
    let state = MEEPO_MACRO_STATE.lock().unwrap();
    MeepoMacroStatusSnapshot {
        mode: state.mode,
        pulses_executed: state.pulses_executed,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn clear_meepo_macro_state() {
    *MEEPO_MACRO_STATE.lock().unwrap() = MeepoMacroRuntimeState::default();
}

#[cfg(test)]
pub(crate) fn meepo_macro_test_lock() -> &'static Mutex<()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK.get_or_init(|| Mutex::new(()))
}

pub fn suspend_meepo_macro(reason: MeepoMacroSuspendReason) -> MeepoMacroMode {
    let mut state = MEEPO_MACRO_STATE.lock().unwrap();
    state.mode = MeepoMacroMode::Suspended(reason);
    if reason != MeepoMacroSuspendReason::ManualCombo {
        state.resume_after_manual_combo_at = None;
    }
    state.mode
}

pub fn toggle_meepo_macro(enabled: bool, snapshot_available: bool) -> MeepoMacroMode {
    let mut state = MEEPO_MACRO_STATE.lock().unwrap();
    state.resume_after_manual_combo_at = None;

    if matches!(state.mode, MeepoMacroMode::Inactive) {
        state.mode = if !enabled {
            MeepoMacroMode::Suspended(MeepoMacroSuspendReason::Disabled)
        } else if snapshot_available {
            MeepoMacroMode::Armed
        } else {
            MeepoMacroMode::Suspended(MeepoMacroSuspendReason::UnableToCast)
        };
    } else {
        state.mode = MeepoMacroMode::Inactive;
    }

    state.mode
}

pub fn suspend_for_manual_combo(cooldown_ms: u64) -> MeepoMacroMode {
    let mut state = MEEPO_MACRO_STATE.lock().unwrap();
    if matches!(state.mode, MeepoMacroMode::Inactive) {
        return state.mode;
    }

    state.mode = MeepoMacroMode::Suspended(MeepoMacroSuspendReason::ManualCombo);
    state.resume_after_manual_combo_at = Some(Instant::now() + Duration::from_millis(cooldown_ms));
    state.mode
}

pub fn evaluate_farm_pulse(
    observed: &MeepoObservedState,
    config: &MeepoFarmAssistConfig,
    now: Instant,
) -> MeepoFarmPulseDecision {
    let mut state = MEEPO_MACRO_STATE.lock().unwrap();

    if !config.enabled {
        state.mode = MeepoMacroMode::Suspended(MeepoMacroSuspendReason::Disabled);
        state.resume_after_manual_combo_at = None;
        return MeepoFarmPulseDecision::Skip;
    }

    if matches!(state.mode, MeepoMacroMode::Suspended(MeepoMacroSuspendReason::ManualCombo)) {
        if let Some(resume_at) = state.resume_after_manual_combo_at {
            if now < resume_at {
                return MeepoFarmPulseDecision::Skip;
            }
        }

        state.mode = MeepoMacroMode::Armed;
        state.resume_after_manual_combo_at = None;
    }

    if config.suspend_on_danger && observed.in_danger {
        state.mode = MeepoMacroMode::Suspended(MeepoMacroSuspendReason::Danger);
        return MeepoFarmPulseDecision::Skip;
    }

    if matches!(state.mode, MeepoMacroMode::Suspended(MeepoMacroSuspendReason::Danger))
        && !observed.in_danger
    {
        state.mode = MeepoMacroMode::Armed;
    }

    if !pulse_is_safe(observed, config) {
        if !matches!(state.mode, MeepoMacroMode::Inactive) {
            state.mode = MeepoMacroMode::Suspended(MeepoMacroSuspendReason::UnableToCast);
        }
        return MeepoFarmPulseDecision::Skip;
    }

    if matches!(state.mode, MeepoMacroMode::Suspended(MeepoMacroSuspendReason::UnableToCast)) {
        state.mode = MeepoMacroMode::Armed;
    }

    if !matches!(state.mode, MeepoMacroMode::Armed) {
        return MeepoFarmPulseDecision::Skip;
    }

    if let Some(last_pulse_at) = state.last_pulse_at {
        if now.duration_since(last_pulse_at) < Duration::from_millis(config.pulse_interval_ms) {
            return MeepoFarmPulseDecision::Skip;
        }
    }

    state.last_pulse_at = Some(now);
    state.pulses_executed += 1;
    MeepoFarmPulseDecision::Run
}

#[cfg(test)]
mod tests {
    use super::{
        clear_meepo_macro_state, evaluate_farm_pulse, latest_meepo_macro_status,
        meepo_macro_test_lock, suspend_for_manual_combo, suspend_meepo_macro, toggle_meepo_macro,
        MeepoFarmPulseDecision, MeepoMacroMode, MeepoMacroSuspendReason,
    };
    use crate::actions::heroes::meepo_state::{KnownCloneState, MeepoObservedState};
    use crate::config::Settings;
    use std::time::{Duration, Instant};

    fn observed_state() -> MeepoObservedState {
        MeepoObservedState {
            hero_name: "npc_dota_hero_meepo".to_string(),
            hero_level: 18,
            health_percent: 60,
            mana_percent: 60,
            in_danger: false,
            alive: true,
            stunned: false,
            silenced: false,
            poof_ready: true,
            dig_ready: true,
            megameepo_ready: true,
            has_shard: true,
            has_scepter: true,
            blink_slot_key: Some('z'),
            combo_item_keys: vec![("sheepstick".to_string(), 'x')],
            clone_state: KnownCloneState::Unavailable,
        }
    }

    #[test]
    fn toggle_meepo_macro_arms_with_snapshot_and_toggles_back_off() {
        let _guard = meepo_macro_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        clear_meepo_macro_state();

        assert_eq!(toggle_meepo_macro(true, true), MeepoMacroMode::Armed);
        assert_eq!(toggle_meepo_macro(true, true), MeepoMacroMode::Inactive);
    }

    #[test]
    fn toggle_meepo_macro_without_snapshot_suspends_unable_to_cast() {
        let _guard = meepo_macro_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        clear_meepo_macro_state();

        assert_eq!(
            toggle_meepo_macro(true, false),
            MeepoMacroMode::Suspended(MeepoMacroSuspendReason::UnableToCast)
        );
    }

    #[test]
    fn evaluate_farm_pulse_runs_once_and_respects_interval() {
        let _guard = meepo_macro_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        clear_meepo_macro_state();
        let config = Settings::default().heroes.meepo.farm_assist;
        let observed = observed_state();
        let now = Instant::now();
        toggle_meepo_macro(true, true);

        assert_eq!(
            evaluate_farm_pulse(&observed, &config, now),
            MeepoFarmPulseDecision::Run
        );
        assert_eq!(
            evaluate_farm_pulse(&observed, &config, now + Duration::from_millis(100)),
            MeepoFarmPulseDecision::Skip
        );
        assert_eq!(latest_meepo_macro_status().pulses_executed, 1);
    }

    #[test]
    fn evaluate_farm_pulse_suspends_on_danger() {
        let _guard = meepo_macro_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        clear_meepo_macro_state();
        let config = Settings::default().heroes.meepo.farm_assist;
        let mut observed = observed_state();
        observed.in_danger = true;
        toggle_meepo_macro(true, true);

        assert_eq!(
            evaluate_farm_pulse(&observed, &config, Instant::now()),
            MeepoFarmPulseDecision::Skip
        );
        assert_eq!(
            latest_meepo_macro_status().mode,
            MeepoMacroMode::Suspended(MeepoMacroSuspendReason::Danger)
        );
    }

    #[test]
    fn manual_combo_suspend_rearms_after_timeout() {
        let _guard = meepo_macro_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        clear_meepo_macro_state();
        let config = Settings::default().heroes.meepo.farm_assist;
        let observed = observed_state();
        toggle_meepo_macro(true, true);
        suspend_for_manual_combo(30);

        assert_eq!(
            evaluate_farm_pulse(&observed, &config, Instant::now()),
            MeepoFarmPulseDecision::Skip
        );
        assert_eq!(
            evaluate_farm_pulse(&observed, &config, Instant::now() + Duration::from_millis(50)),
            MeepoFarmPulseDecision::Run
        );
    }

    #[test]
    fn suspend_meepo_macro_marks_hero_changed() {
        let _guard = meepo_macro_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        clear_meepo_macro_state();
        assert_eq!(
            suspend_meepo_macro(MeepoMacroSuspendReason::HeroChanged),
            MeepoMacroMode::Suspended(MeepoMacroSuspendReason::HeroChanged)
        );
    }
}
