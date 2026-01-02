//! Power Treads automation module
//!
//! Automatically toggles Power Treads to optimal stat before ability/item usage:
//! - Switch to INT before abilities (mana efficiency)
//! - Switch to AGI before healing items (HP efficiency - smaller pool = larger % heal)
//! - Switch back to original stat after use
//!
//! Tracks treads state internally and updates when user manually toggles.
//! Initializes current stat based on hero's primary attribute.

use crate::config::Settings;
use crate::models::gsi_event::{Hero, Items};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Power Treads stat cycle: STR → INT → AGI → STR
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stat {
    Str,
    Int,
    Agi,
}

impl Stat {
    /// Get the next stat in the cycle (STR → INT → AGI → STR)
    pub fn next(self) -> Stat {
        match self {
            Stat::Str => Stat::Int,
            Stat::Int => Stat::Agi,
            Stat::Agi => Stat::Str,
        }
    }

    /// Calculate how many presses needed to go from one stat to another
    pub fn presses_to_reach(from: Stat, to: Stat) -> u8 {
        if from == to {
            return 0;
        }
        match (from, to) {
            // STR -> INT = 1, STR -> AGI = 2
            (Stat::Str, Stat::Int) => 1,
            (Stat::Str, Stat::Agi) => 2,
            // INT -> AGI = 1, INT -> STR = 2
            (Stat::Int, Stat::Agi) => 1,
            (Stat::Int, Stat::Str) => 2,
            // AGI -> STR = 1, AGI -> INT = 2
            (Stat::Agi, Stat::Str) => 1,
            (Stat::Agi, Stat::Int) => 2,
            _ => 0, // Same stat (handled above)
        }
    }

    /// Get primary stat from hero name
    pub fn from_hero_name(hero_name: &str) -> Stat {
        // STR heroes
        const STR_HEROES: &[&str] = &[
            "npc_dota_hero_axe",
            "npc_dota_hero_earthshaker",
            "npc_dota_hero_pudge",
            "npc_dota_hero_sand_king",
            "npc_dota_hero_sven",
            "npc_dota_hero_tiny",
            "npc_dota_hero_kunkka",
            "npc_dota_hero_beastmaster",
            "npc_dota_hero_dragon_knight",
            "npc_dota_hero_clockwerk",
            "npc_dota_hero_omniknight",
            "npc_dota_hero_huskar",
            "npc_dota_hero_alchemist",
            "npc_dota_hero_brewmaster",
            "npc_dota_hero_treant",
            "npc_dota_hero_wisp",
            "npc_dota_hero_centaur",
            "npc_dota_hero_shredder",
            "npc_dota_hero_bristleback",
            "npc_dota_hero_tusk",
            "npc_dota_hero_elder_titan",
            "npc_dota_hero_legion_commander",
            "npc_dota_hero_earth_spirit",
            "npc_dota_hero_phoenix",
            "npc_dota_hero_abaddon",
            "npc_dota_hero_underlord",
            "npc_dota_hero_spirit_breaker",
            "npc_dota_hero_doom_bringer",
            "npc_dota_hero_lycan",
            "npc_dota_hero_chaos_knight",
            "npc_dota_hero_undying",
            "npc_dota_hero_magnataur",
            "npc_dota_hero_life_stealer",
            "npc_dota_hero_night_stalker",
            "npc_dota_hero_skeleton_king",
            "npc_dota_hero_mars",
            "npc_dota_hero_snapfire",
            "npc_dota_hero_dawnbreaker",
            "npc_dota_hero_primal_beast",
            "npc_dota_hero_marci",
            "npc_dota_hero_muerta",
            "npc_dota_hero_ringmaster",
        ];

        // AGI heroes
        const AGI_HEROES: &[&str] = &[
            "npc_dota_hero_antimage",
            "npc_dota_hero_bloodseeker",
            "npc_dota_hero_drow_ranger",
            "npc_dota_hero_juggernaut",
            "npc_dota_hero_mirana",
            "npc_dota_hero_morphling",
            "npc_dota_hero_phantom_lancer",
            "npc_dota_hero_vengefulspirit",
            "npc_dota_hero_riki",
            "npc_dota_hero_sniper",
            "npc_dota_hero_templar_assassin",
            "npc_dota_hero_luna",
            "npc_dota_hero_bounty_hunter",
            "npc_dota_hero_ursa",
            "npc_dota_hero_gyrocopter",
            "npc_dota_hero_lone_druid",
            "npc_dota_hero_naga_siren",
            "npc_dota_hero_troll_warlord",
            "npc_dota_hero_ember_spirit",
            "npc_dota_hero_monkey_king",
            "npc_dota_hero_pangolier",
            "npc_dota_hero_dark_willow",
            "npc_dota_hero_grimstroke",
            "npc_dota_hero_hoodwink",
            "npc_dota_hero_void_spirit",
            "npc_dota_hero_spectre",
            "npc_dota_hero_razor",
            "npc_dota_hero_viper",
            "npc_dota_hero_clinkz",
            "npc_dota_hero_broodmother",
            "npc_dota_hero_weaver",
            "npc_dota_hero_nyx_assassin",
            "npc_dota_hero_slark",
            "npc_dota_hero_medusa",
            "npc_dota_hero_terrorblade",
            "npc_dota_hero_arc_warden",
            "npc_dota_hero_phantom_assassin",
            "npc_dota_hero_faceless_void",
        ];

        if STR_HEROES.contains(&hero_name) {
            Stat::Str
        } else if AGI_HEROES.contains(&hero_name) {
            Stat::Agi
        } else {
            // Default to INT for all other heroes (INT heroes and unknown)
            Stat::Int
        }
    }

    /// Parse stat from string (for config)
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Stat> {
        match s.to_lowercase().as_str() {
            "str" | "strength" => Some(Stat::Str),
            "int" | "intelligence" => Some(Stat::Int),
            "agi" | "agility" => Some(Stat::Agi),
            _ => None,
        }
    }
}

/// Healing items - switch to AGI for smaller HP pool = larger % heal
pub const HEALING_ITEMS: &[&str] = &[
    "item_flask",           // Healing Salve
    "item_tango",           // Tango
    "item_faerie_fire",     // Faerie Fire
    "item_magic_wand",      // Magic Wand (heals HP and mana, default to AGI)
    "item_magic_stick",     // Magic Stick (heals HP and mana, default to AGI)
    "item_cheese",          // Cheese (heals HP and mana, default to AGI)
    "item_bottle",          // Bottle (heals HP and mana, default to AGI)
    "item_mekansm",         // Mekansm
    "item_guardian_greaves",// Guardian Greaves
    "item_holy_locket",     // Holy Locket
    "item_urn_of_shadows",  // Urn of Shadows
    "item_spirit_vessel",   // Spirit Vessel
];

/// Mana items - switch to INT for larger mana pool = smaller % cost
pub const MANA_ITEMS: &[&str] = &[
    "item_clarity",         // Clarity
    "item_enchanted_mango", // Enchanted Mango
    "item_arcane_boots",    // Arcane Boots
];

/// Shared state for Power Treads automation, updated by GSI events
#[derive(Debug)]
pub struct PowerTreadsState {
    /// Whether Power Treads are currently in inventory
    pub available: bool,
    /// The key to press to toggle Power Treads (based on its slot)
    pub slot_key: Option<char>,
    /// Current stat the treads are set to (tracked internally)
    pub current_stat: Stat,
    /// Primary stat based on hero (used for initialization)
    pub primary_stat: Stat,
    /// Whether the hero is alive
    pub hero_alive: bool,
    /// Last time Power Treads were toggled (for cooldown lockout)
    pub last_triggered: Option<Instant>,
    /// Whether state has been initialized from hero
    pub initialized: bool,
}

impl Default for PowerTreadsState {
    fn default() -> Self {
        Self {
            available: false,
            slot_key: None,
            current_stat: Stat::Agi, // Default, will be updated from hero
            primary_stat: Stat::Agi,
            hero_alive: false,
            last_triggered: None,
            initialized: false,
        }
    }
}

impl PowerTreadsState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if Power Treads toggle should be performed
    pub fn should_toggle(&self, target_stat: Stat, settings: &Settings) -> bool {
        // Master toggle must be enabled
        if !settings.power_treads.enabled {
            return false;
        }

        // Power Treads must be available
        if !self.available || self.slot_key.is_none() {
            return false;
        }

        // Hero must be alive
        if !self.hero_alive {
            return false;
        }

        // Already on target stat - no-op
        if self.current_stat == target_stat {
            debug!(
                "👟 Power Treads: already on {:?}, skipping toggle",
                target_stat
            );
            return false;
        }

        // Check cooldown lockout
        if let Some(last) = self.last_triggered {
            let elapsed = last.elapsed();
            let cooldown = Duration::from_millis(settings.power_treads.toggle_cooldown_ms);
            if elapsed < cooldown {
                debug!(
                    "👟 Power Treads: cooldown lockout ({:?} < {:?}), skipping",
                    elapsed, cooldown
                );
                return false;
            }
        }

        true
    }

    /// Mark Power Treads as toggled (updates cooldown and current stat)
    pub fn mark_toggled(&mut self, new_stat: Stat) {
        self.last_triggered = Some(Instant::now());
        self.current_stat = new_stat;
        info!(
            "👟 Power Treads toggled to {:?}",
            new_stat
        );
    }

    /// Advance current stat by one step (for manual toggle tracking)
    pub fn advance_stat(&mut self) {
        let old_stat = self.current_stat;
        self.current_stat = self.current_stat.next();
        debug!(
            "👟 Power Treads manual toggle detected: {:?} -> {:?}",
            old_stat, self.current_stat
        );
    }

    /// Check if a key is an ability key that should trigger INT toggle
    pub fn is_ability_key(&self, key_char: char, settings: &Settings) -> bool {
        if !settings.power_treads.toggle_for_abilities {
            return false;
        }
        let key_str = key_char.to_ascii_lowercase().to_string();
        settings
            .power_treads
            .ability_keys
            .iter()
            .any(|k| k.to_lowercase() == key_str)
    }

    /// Check if a key is the Power Treads slot key
    pub fn is_treads_key(&self, key_char: char) -> bool {
        if let Some(treads_key) = self.slot_key {
            key_char.to_ascii_lowercase() == treads_key.to_ascii_lowercase()
        } else {
            false
        }
    }

    /// Get target stat for an item (AGI for healing, INT for mana, None for neither)
    pub fn get_target_stat_for_item(item_name: &str) -> Option<Stat> {
        if HEALING_ITEMS.contains(&item_name) {
            Some(Stat::Agi)
        } else if MANA_ITEMS.contains(&item_name) {
            Some(Stat::Int)
        } else {
            None
        }
    }

    /// Check if a key corresponds to a healing/mana item and return target stat
    pub fn get_target_stat_for_key(&self, key_char: char, items: &Items, settings: &Settings) -> Option<Stat> {
        if !settings.power_treads.toggle_for_items {
            return None;
        }

        // Map key to slot
        let slot_name = self.key_to_slot_name(key_char, settings)?;
        
        // Get item in that slot
        let item = match slot_name {
            "slot0" => &items.slot0,
            "slot1" => &items.slot1,
            "slot2" => &items.slot2,
            "slot3" => &items.slot3,
            "slot4" => &items.slot4,
            "slot5" => &items.slot5,
            _ => return None,
        };

        // Don't trigger on Power Treads key (would cause issues)
        if item.name == "item_power_treads" {
            return None;
        }

        Self::get_target_stat_for_item(&item.name)
    }

    /// Map a key character to slot name
    fn key_to_slot_name(&self, key_char: char, settings: &Settings) -> Option<&'static str> {
        let key_lower = key_char.to_ascii_lowercase();
        if key_lower == settings.keybindings.slot0.to_ascii_lowercase() {
            Some("slot0")
        } else if key_lower == settings.keybindings.slot1.to_ascii_lowercase() {
            Some("slot1")
        } else if key_lower == settings.keybindings.slot2.to_ascii_lowercase() {
            Some("slot2")
        } else if key_lower == settings.keybindings.slot3.to_ascii_lowercase() {
            Some("slot3")
        } else if key_lower == settings.keybindings.slot4.to_ascii_lowercase() {
            Some("slot4")
        } else if key_lower == settings.keybindings.slot5.to_ascii_lowercase() {
            Some("slot5")
        } else {
            None
        }
    }
}

/// Global Power Treads state, shared between keyboard listener and GSI handler
pub static POWER_TREADS_STATE: LazyLock<Arc<Mutex<PowerTreadsState>>> =
    LazyLock::new(|| Arc::new(Mutex::new(PowerTreadsState::new())));

/// Cached copy of items for use in keyboard handler
pub static CACHED_ITEMS: LazyLock<Arc<Mutex<Option<Items>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

/// Update Power Treads state from GSI event
pub fn update_from_gsi(items: &Items, hero: &Hero, settings: &Settings) {
    let mut state = POWER_TREADS_STATE.lock().unwrap();

    // Update hero status
    state.hero_alive = hero.alive;

    // Initialize primary stat from hero on first detection
    if !state.initialized {
        state.primary_stat = Stat::from_hero_name(&hero.name);
        state.current_stat = state.primary_stat;
        state.initialized = true;
        info!(
            "👟 Power Treads: initialized for {} with primary stat {:?}",
            hero.name, state.primary_stat
        );
    }

    // Search for Power Treads in inventory
    let mut found = false;
    for (slot_name, item) in items.all_slots() {
        if item.name == "item_power_treads" {
            found = true;
            let new_slot_key = settings.get_key_for_slot(slot_name);
            
            // Only log if state changed
            if !state.available || state.slot_key != new_slot_key {
                info!(
                    "👟 Power Treads found in {}: key={:?}, current_stat={:?}",
                    slot_name, new_slot_key, state.current_stat
                );
            }
            
            state.available = true;
            state.slot_key = new_slot_key;
            break;
        }
    }

    // If Power Treads not found, mark as unavailable
    if !found && state.available {
        info!("👟 Power Treads no longer in inventory, disabling automation");
        state.available = false;
        state.slot_key = None;
    }

    // Cache items for keyboard handler
    let mut cached = CACHED_ITEMS.lock().unwrap();
    *cached = Some(items.clone());
}

/// Reset Power Treads state (e.g., on game start)
#[allow(dead_code)]
pub fn reset_state() {
    let mut state = POWER_TREADS_STATE.lock().unwrap();
    *state = PowerTreadsState::default();
    info!("👟 Power Treads: state reset");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_cycle() {
        assert_eq!(Stat::Str.next(), Stat::Int);
        assert_eq!(Stat::Int.next(), Stat::Agi);
        assert_eq!(Stat::Agi.next(), Stat::Str);
    }

    #[test]
    fn test_presses_to_reach() {
        // Same stat = 0 presses
        assert_eq!(Stat::presses_to_reach(Stat::Str, Stat::Str), 0);
        assert_eq!(Stat::presses_to_reach(Stat::Int, Stat::Int), 0);
        assert_eq!(Stat::presses_to_reach(Stat::Agi, Stat::Agi), 0);

        // From STR
        assert_eq!(Stat::presses_to_reach(Stat::Str, Stat::Int), 1);
        assert_eq!(Stat::presses_to_reach(Stat::Str, Stat::Agi), 2);

        // From INT
        assert_eq!(Stat::presses_to_reach(Stat::Int, Stat::Agi), 1);
        assert_eq!(Stat::presses_to_reach(Stat::Int, Stat::Str), 2);

        // From AGI
        assert_eq!(Stat::presses_to_reach(Stat::Agi, Stat::Str), 1);
        assert_eq!(Stat::presses_to_reach(Stat::Agi, Stat::Int), 2);
    }

    #[test]
    fn test_hero_primary_stat() {
        assert_eq!(Stat::from_hero_name("npc_dota_hero_axe"), Stat::Str);
        assert_eq!(Stat::from_hero_name("npc_dota_hero_huskar"), Stat::Str);
        assert_eq!(Stat::from_hero_name("npc_dota_hero_morphling"), Stat::Agi);
        assert_eq!(Stat::from_hero_name("npc_dota_hero_antimage"), Stat::Agi);
        assert_eq!(Stat::from_hero_name("npc_dota_hero_invoker"), Stat::Int);
        assert_eq!(Stat::from_hero_name("npc_dota_hero_lina"), Stat::Int);
    }

    #[test]
    fn test_item_stat_target() {
        // Healing items -> AGI
        assert_eq!(PowerTreadsState::get_target_stat_for_item("item_flask"), Some(Stat::Agi));
        assert_eq!(PowerTreadsState::get_target_stat_for_item("item_bottle"), Some(Stat::Agi));
        assert_eq!(PowerTreadsState::get_target_stat_for_item("item_magic_wand"), Some(Stat::Agi));

        // Mana items -> INT
        assert_eq!(PowerTreadsState::get_target_stat_for_item("item_clarity"), Some(Stat::Int));
        assert_eq!(PowerTreadsState::get_target_stat_for_item("item_enchanted_mango"), Some(Stat::Int));
        assert_eq!(PowerTreadsState::get_target_stat_for_item("item_arcane_boots"), Some(Stat::Int));

        // Other items -> None
        assert_eq!(PowerTreadsState::get_target_stat_for_item("item_blink"), None);
        assert_eq!(PowerTreadsState::get_target_stat_for_item("item_power_treads"), None);
    }

    #[test]
    fn test_default_state() {
        let state = PowerTreadsState::default();
        assert!(!state.available);
        assert!(state.slot_key.is_none());
        assert!(!state.hero_alive);
        assert!(!state.initialized);
    }
}
