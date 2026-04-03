use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerFamily {
    Danger,
    LowMana,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastMode {
    SelfCast,
    NoTarget,
    CursorTargeted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportStatus {
    Supported,
    KnownUnsupported,
    Inactive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemAutomationSpec {
    pub item_name: &'static str,
    pub trigger_family: TriggerFamily,
    pub cast_mode: CastMode,
    pub support: SupportStatus,
    pub is_neutral: bool,
}

pub const ITEM_AUTOMATION_SPECS: &[ItemAutomationSpec] = &[
    ItemAutomationSpec {
        item_name: "item_essence_ring",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::SelfCast,
        support: SupportStatus::Supported,
        is_neutral: true,
    },
    ItemAutomationSpec {
        item_name: "item_minotaur_horn",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::SelfCast,
        support: SupportStatus::Supported,
        is_neutral: true,
    },
    ItemAutomationSpec {
        item_name: "item_metamorphic_mandible",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::SelfCast,
        support: SupportStatus::Supported,
        is_neutral: true,
    },
    ItemAutomationSpec {
        item_name: "item_jidi_pollen_bag",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::NoTarget,
        support: SupportStatus::Supported,
        is_neutral: true,
    },
    ItemAutomationSpec {
        item_name: "item_ash_legion_shield",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::NoTarget,
        support: SupportStatus::Supported,
        is_neutral: true,
    },
    ItemAutomationSpec {
        item_name: "item_idol_of_screeauk",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::NoTarget,
        support: SupportStatus::Supported,
        is_neutral: true,
    },
    ItemAutomationSpec {
        item_name: "item_kobold_cup",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::NoTarget,
        support: SupportStatus::Supported,
        is_neutral: true,
    },
    ItemAutomationSpec {
        item_name: "item_crippling_crossbow",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::CursorTargeted,
        support: SupportStatus::Supported,
        is_neutral: true,
    },
    ItemAutomationSpec {
        item_name: "item_arcane_boots",
        trigger_family: TriggerFamily::LowMana,
        cast_mode: CastMode::NoTarget,
        support: SupportStatus::Supported,
        is_neutral: false,
    },
    ItemAutomationSpec {
        item_name: "item_mana_draught",
        trigger_family: TriggerFamily::LowMana,
        cast_mode: CastMode::NoTarget,
        support: SupportStatus::Supported,
        is_neutral: true,
    },
    ItemAutomationSpec {
        item_name: "item_psychic_headband",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::CursorTargeted,
        support: SupportStatus::KnownUnsupported,
        is_neutral: true,
    },
    ItemAutomationSpec {
        item_name: "item_polliwog_charm",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::CursorTargeted,
        support: SupportStatus::KnownUnsupported,
        is_neutral: true,
    },
    ItemAutomationSpec {
        item_name: "item_flayers_bota",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::NoTarget,
        support: SupportStatus::KnownUnsupported,
        is_neutral: true,
    },
    ItemAutomationSpec {
        item_name: "item_riftshadow_prism",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::SelfCast,
        support: SupportStatus::KnownUnsupported,
        is_neutral: true,
    },
    ItemAutomationSpec {
        item_name: "item_pogo_stick",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::NoTarget,
        support: SupportStatus::Inactive,
        is_neutral: true,
    },
];

pub fn lookup_item_automation(item_name: &str) -> Option<&'static ItemAutomationSpec> {
    ITEM_AUTOMATION_SPECS
        .iter()
        .find(|spec| spec.item_name == item_name)
}

pub fn hero_is_excluded(hero_name: &str, excluded_heroes: &[String]) -> bool {
    excluded_heroes.iter().any(|excluded| excluded == hero_name)
}

#[derive(Debug, Default)]
pub struct TriggerLockoutState {
    last_trigger_ms: HashMap<String, u64>,
}

impl TriggerLockoutState {
    pub fn try_acquire(&mut self, key: &str, now_ms: u64, lockout_ms: u64) -> bool {
        if let Some(previous_ms) = self.last_trigger_ms.get(key).copied() {
            if now_ms.saturating_sub(previous_ms) < lockout_ms {
                return false;
            }
        }

        self.last_trigger_ms.insert(key.to_string(), now_ms);
        true
    }
}

lazy_static! {
    static ref GLOBAL_TRIGGER_LOCKOUTS: Mutex<TriggerLockoutState> =
        Mutex::new(TriggerLockoutState::default());
}

pub fn try_acquire_global_lockout(key: &str, now_ms: u64, lockout_ms: u64) -> bool {
    GLOBAL_TRIGGER_LOCKOUTS
        .lock()
        .unwrap()
        .try_acquire(key, now_ms, lockout_ms)
}

#[cfg(test)]
pub fn reset_global_lockouts_for_tests() {
    *GLOBAL_TRIGGER_LOCKOUTS.lock().unwrap() = TriggerLockoutState::default();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_returns_supported_danger_specs_for_first_pass_neutrals() {
        let spec = lookup_item_automation("item_essence_ring").unwrap();
        assert_eq!(spec.trigger_family, TriggerFamily::Danger);
        assert_eq!(spec.cast_mode, CastMode::SelfCast);
        assert_eq!(spec.support, SupportStatus::Supported);
        assert!(spec.is_neutral);

        let spec = lookup_item_automation("item_jidi_pollen_bag").unwrap();
        assert_eq!(spec.trigger_family, TriggerFamily::Danger);
        assert_eq!(spec.cast_mode, CastMode::NoTarget);
        assert_eq!(spec.support, SupportStatus::Supported);
        assert!(spec.is_neutral);
    }

    #[test]
    fn lookup_marks_known_but_unsupported_items_without_dropping_them() {
        let spec = lookup_item_automation("item_psychic_headband").unwrap();
        assert_eq!(spec.trigger_family, TriggerFamily::Danger);
        assert_eq!(spec.cast_mode, CastMode::CursorTargeted);
        assert_eq!(spec.support, SupportStatus::KnownUnsupported);
    }

    #[test]
    fn lookup_returns_low_mana_specs_for_arcane_boots_and_mana_draught() {
        let boots = lookup_item_automation("item_arcane_boots").unwrap();
        assert_eq!(boots.trigger_family, TriggerFamily::LowMana);
        assert_eq!(boots.cast_mode, CastMode::NoTarget);
        assert!(!boots.is_neutral);

        let draught = lookup_item_automation("item_mana_draught").unwrap();
        assert_eq!(draught.trigger_family, TriggerFamily::LowMana);
        assert_eq!(draught.support, SupportStatus::Supported);
    }

    #[test]
    fn hero_exclusion_matches_internal_name_exactly() {
        let excluded = vec!["npc_dota_hero_huskar".to_string()];
        assert!(hero_is_excluded("npc_dota_hero_huskar", &excluded));
        assert!(!hero_is_excluded("npc_dota_hero_zuus", &excluded));
    }

    #[test]
    fn lockout_state_blocks_duplicate_fires_inside_window() {
        let mut state = TriggerLockoutState::default();

        assert!(state.try_acquire("danger:item_jidi_pollen_bag", 1_000, 120));
        assert!(!state.try_acquire("danger:item_jidi_pollen_bag", 1_050, 120));
        assert!(state.try_acquire("danger:item_jidi_pollen_bag", 1_200, 120));
    }
}
