use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerFamily {
    Danger,
    LowMana,
    Movement,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovementSnapshot {
    pub hero_name: String,
    pub alive: bool,
    pub anchor_xpos: i32,
    pub anchor_ypos: i32,
    pub last_xpos: i32,
    pub last_ypos: i32,
    pub idle_samples: u8,
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
    // Cycled out of the neutral pool in 7.42. Kept `Inactive` so the spec is ready if
    // Valve brings it back.
    ItemAutomationSpec {
        item_name: "item_essence_ring",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::SelfCast,
        support: SupportStatus::Inactive,
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
        item_name: "item_phase_boots",
        trigger_family: TriggerFamily::Movement,
        cast_mode: CastMode::NoTarget,
        support: SupportStatus::Supported,
        is_neutral: false,
    },
    ItemAutomationSpec {
        item_name: "item_psychic_headband",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::CursorTargeted,
        support: SupportStatus::KnownUnsupported,
        is_neutral: true,
    },
    // Ally-targeted heal. Capture 1 of `examples/neutral_cast_probe.rs` saw the double
    // tap fail to cast it under Quick Cast; only Alt (the self-cast modifier) worked.
    // Held until the neutral path learns a modifier self-cast.
    ItemAutomationSpec {
        item_name: "item_polliwog_charm",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::SelfCast,
        support: SupportStatus::KnownUnsupported,
        is_neutral: true,
    },
    // Ground-targeted regen zone. Whether a double tap drops it at the caster's
    // feet is unmeasured; `examples/neutral_cast_probe.rs` step 9 settles it.
    ItemAutomationSpec {
        item_name: "item_seeds_of_serenity",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::SelfCast,
        support: SupportStatus::KnownUnsupported,
        is_neutral: true,
    },
    // Enemy-targeted, so it only works with an enemy under the cursor. Its value
    // is offensive, which is why it stays out of the default list.
    ItemAutomationSpec {
        item_name: "item_brick",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::CursorTargeted,
        support: SupportStatus::Supported,
        is_neutral: true,
    },
    // No target: a basic dispel and a short blink, in place if the hero took
    // player damage in the last 3s.
    ItemAutomationSpec {
        item_name: "item_flicker",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::NoTarget,
        support: SupportStatus::Supported,
        is_neutral: true,
    },
    // Ground-targeted AoE dispel (Witchbane). Whether a double tap lands on the
    // caster is unmeasured; `examples/neutral_cast_probe.rs` steps 6-7 settle it.
    ItemAutomationSpec {
        item_name: "item_heavy_blade",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::SelfCast,
        support: SupportStatus::KnownUnsupported,
        is_neutral: true,
    },
    // Book of the Dead: summons. Supported, but left out of the default list.
    ItemAutomationSpec {
        item_name: "item_demonicon",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::NoTarget,
        support: SupportStatus::Supported,
        is_neutral: true,
    },
    // Cycled out of the neutral pool in 7.42, like Essence Ring.
    ItemAutomationSpec {
        item_name: "item_flayers_bota",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::NoTarget,
        support: SupportStatus::Inactive,
        is_neutral: true,
    },
    // No-target decoy illusion. It costs 4% of *current* HP, so it is cheap at
    // exactly the moment danger fires.
    ItemAutomationSpec {
        item_name: "item_riftshadow_prism",
        trigger_family: TriggerFamily::Danger,
        cast_mode: CastMode::NoTarget,
        support: SupportStatus::Supported,
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
    static ref LAST_MOVEMENT_SNAPSHOT: Mutex<Option<MovementSnapshot>> = Mutex::new(None);
}

pub fn try_acquire_global_lockout(key: &str, now_ms: u64, lockout_ms: u64) -> bool {
    GLOBAL_TRIGGER_LOCKOUTS
        .lock()
        .unwrap()
        .try_acquire(key, now_ms, lockout_ms)
}

pub fn read_movement_snapshot() -> Option<MovementSnapshot> {
    LAST_MOVEMENT_SNAPSHOT.lock().unwrap().clone()
}

pub fn write_movement_snapshot(snapshot: MovementSnapshot) {
    *LAST_MOVEMENT_SNAPSHOT.lock().unwrap() = Some(snapshot);
}

pub fn clear_movement_snapshot() {
    *LAST_MOVEMENT_SNAPSHOT.lock().unwrap() = None;
}

#[cfg(test)]
pub fn reset_global_lockouts_for_tests() {
    *GLOBAL_TRIGGER_LOCKOUTS.lock().unwrap() = TriggerLockoutState::default();
}

#[cfg(test)]
pub fn replace_movement_snapshot_for_tests(snapshot: Option<MovementSnapshot>) {
    *LAST_MOVEMENT_SNAPSHOT.lock().unwrap() = snapshot;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_returns_supported_danger_specs_for_first_pass_neutrals() {
        let spec = lookup_item_automation("item_minotaur_horn").unwrap();
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
    fn lookup_supports_the_7_42_danger_neutrals_with_their_cast_modes() {
        for (name, cast_mode) in [
            ("item_riftshadow_prism", CastMode::NoTarget),
            ("item_flicker", CastMode::NoTarget),
            ("item_demonicon", CastMode::NoTarget),
            ("item_brick", CastMode::CursorTargeted),
        ] {
            let spec = lookup_item_automation(name).unwrap();
            assert_eq!(spec.trigger_family, TriggerFamily::Danger, "{name}");
            assert_eq!(spec.cast_mode, cast_mode, "{name}");
            assert_eq!(spec.support, SupportStatus::Supported, "{name}");
            assert!(spec.is_neutral, "{name}");
        }
    }

    #[test]
    fn lookup_holds_neutrals_a_double_tap_cannot_self_cast() {
        for name in ["item_seeds_of_serenity", "item_heavy_blade", "item_polliwog_charm"] {
            let spec = lookup_item_automation(name).unwrap();
            assert_eq!(spec.support, SupportStatus::KnownUnsupported, "{name}");
        }
    }

    #[test]
    fn lookup_keeps_cycled_out_neutrals_as_inactive() {
        for name in ["item_essence_ring", "item_flayers_bota"] {
            let spec = lookup_item_automation(name).unwrap();
            assert_eq!(spec.support, SupportStatus::Inactive, "{name}");
        }
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
    fn lookup_returns_phase_boots_as_supported_movement_item() {
        let boots = lookup_item_automation("item_phase_boots").unwrap();

        assert_eq!(boots.trigger_family, TriggerFamily::Movement);
        assert_eq!(boots.cast_mode, CastMode::NoTarget);
        assert_eq!(boots.support, SupportStatus::Supported);
        assert!(!boots.is_neutral);
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
