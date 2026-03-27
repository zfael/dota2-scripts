use crate::actions::armlet;
use crate::actions::executor::ActionExecutor;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Item};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, info};

const SELF_CAST_DELAY_MS: u64 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlannedKeyPress {
    key: char,
    delay_after_ms: u64,
}

impl PlannedKeyPress {
    const fn new(key: char, delay_after_ms: u64) -> Self {
        Self {
            key,
            delay_after_ms,
        }
    }
}

fn plan_item_key_sequence(item_name: &str, key: char) -> Vec<PlannedKeyPress> {
    if item_name == "item_glimmer_cape" {
        vec![
            PlannedKeyPress::new(key, SELF_CAST_DELAY_MS),
            PlannedKeyPress::new(key, 0),
        ]
    } else {
        vec![PlannedKeyPress::new(key, 0)]
    }
}

fn plan_defensive_item_key_sequence(items: &[(String, char)]) -> Vec<PlannedKeyPress> {
    items
        .iter()
        .flat_map(|(item_name, key)| plan_item_key_sequence(item_name, *key))
        .collect()
}

fn plan_neutral_item_key_sequence(neutral_key: char, self_cast_key: char) -> Vec<PlannedKeyPress> {
    vec![
        PlannedKeyPress::new(neutral_key, SELF_CAST_DELAY_MS),
        PlannedKeyPress::new(self_cast_key, 0),
    ]
}

fn execute_key_sequence(sequence: Vec<PlannedKeyPress>) {
    for press in sequence {
        crate::input::press_key(press.key);
        if press.delay_after_ms > 0 {
            std::thread::sleep(Duration::from_millis(press.delay_after_ms));
        }
    }
}

/// Find the keybinding for a specific item in the hero's inventory
pub fn find_item_slot(event: &GsiWebhookEvent, settings: &Settings, item: Item) -> Option<char> {
    find_item_slot_by_name(event, settings, item.to_game_name())
}

/// Find item slot key by item name string from GSI event (for backward compatibility)
pub fn find_item_slot_by_name(
    event: &GsiWebhookEvent,
    settings: &Settings,
    item_name: &str,
) -> Option<char> {
    let items = &event.items;

    // Check all inventory slots
    if items.slot0.name.contains(item_name) {
        return settings.get_key_for_slot("slot0");
    }
    if items.slot1.name.contains(item_name) {
        return settings.get_key_for_slot("slot1");
    }
    if items.slot2.name.contains(item_name) {
        return settings.get_key_for_slot("slot2");
    }
    if items.slot3.name.contains(item_name) {
        return settings.get_key_for_slot("slot3");
    }
    if items.slot4.name.contains(item_name) {
        return settings.get_key_for_slot("slot4");
    }
    if items.slot5.name.contains(item_name) {
        return settings.get_key_for_slot("slot5");
    }
    if items.neutral0.name.contains(item_name) {
        return settings.get_key_for_slot("neutral0");
    }

    None
}

/// Snapshot-aware helpers for danger-aware gating used by survivability paths
#[cfg_attr(not(test), allow(dead_code))]
fn healing_threshold_for_event(settings: &Settings, in_danger: bool) -> u32 {
    if in_danger && settings.danger_detection.enabled {
        settings.danger_detection.healing_threshold_in_danger
    } else {
        settings.common.survivability_hp_threshold
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn should_consider_defensive_items(event: &GsiWebhookEvent, settings: &Settings, in_danger: bool) -> bool {
    // Mirror the early gates in use_defensive_items_if_danger
    if !settings.danger_detection.enabled {
        return false;
    }
    if !in_danger {
        return false;
    }
    if !event.hero.is_alive() {
        return false;
    }
    true
}

#[cfg_attr(not(test), allow(dead_code))]
fn should_consider_neutral_item(event: &GsiWebhookEvent, settings: &Settings, in_danger: bool) -> bool {
    // Minimal gating used by use_neutral_item_if_danger
    if !settings.neutral_items.enabled || !settings.neutral_items.use_in_danger {
        return false;
    }
    if !in_danger {
        return false;
    }
    if !event.hero.is_alive() {
        return false;
    }
    if event.hero.health_percent >= settings.neutral_items.hp_threshold {
        return false;
    }
    let neutral = &event.items.neutral0;
    if neutral.name == "empty" {
        return false;
    }
    if !settings.neutral_items.allowed_items.contains(&neutral.name) {
        return false;
    }
    if let Some(can_cast) = neutral.can_cast {
        return can_cast;
    }
    false
}

/// Common survivability actions that apply to all heroes
pub struct SurvivabilityActions {
    pub(crate) settings: Arc<Mutex<Settings>>,
    pub(crate) executor: Arc<ActionExecutor>,
}

// Ensure SurvivabilityActions can be shared across threads
unsafe impl Send for SurvivabilityActions {}
unsafe impl Sync for SurvivabilityActions {}

impl SurvivabilityActions {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }

    /// Execute default GSI strategy (survivability + armlet + danger detection)
    pub fn execute_default_strategy(&self, event: &GsiWebhookEvent) {
        // PRIORITY 1: Check for armlet and toggle if needed (non-blocking)
        // Clone event for thread safety
        let event_clone = event.clone();
        let settings_clone = self.settings.clone();
        self.executor.enqueue("default-armlet-toggle", move || {
            let settings = settings_clone.lock().unwrap();

            // Check if hero has armlet in inventory
            let has_armlet = event_clone
                .items
                .all_slots()
                .iter()
                .any(|(_, item)| item.name == "item_armlet");

            if !has_armlet {
                return;
            }

            armlet::maybe_toggle(&event_clone, &settings);
        });

        // PRIORITY 2: Update danger detection state
        {
            let settings = self.settings.lock().unwrap();
            let _in_danger =
                crate::actions::danger_detector::update(event, &settings.danger_detection);
        }

        // PRIORITY 3: Always check survivability first
        self.check_and_use_healing_items(event);

        // PRIORITY 4: Use defensive items if in danger
        self.use_defensive_items_if_danger(event);

        // PRIORITY 5: Use neutral items if in danger
        self.use_neutral_item_if_danger(event);
    }

    /// Check if hero needs healing and use appropriate items
    pub fn check_and_use_healing_items(&self, event: &GsiWebhookEvent) {
        if !event.hero.is_alive() {
            return;
        }

        // Determine threshold based on danger state
        let in_danger = crate::actions::danger_detector::is_in_danger();
        let settings = self.settings.lock().unwrap();
        let threshold = if in_danger && settings.danger_detection.enabled {
            settings.danger_detection.healing_threshold_in_danger
        } else {
            settings.common.survivability_hp_threshold
        };

        // Check if HP is below threshold
        if event.hero.health_percent >= threshold {
            return;
        }

        debug!(
            "HP below threshold: {}% < {}% (in_danger: {})",
            event.hero.health_percent, threshold, in_danger
        );

        // Priority order - high value first when in danger, low value first otherwise
        let healing_items = if in_danger {
            vec![
                ("item_cheese", 2000u32),
                ("item_greater_faerie_fire", 350u32),
                ("item_enchanted_mango", 175u32),
                ("item_magic_wand", 100u32), // Approximate (15 per charge)
                ("item_faerie_fire", 85u32),
            ]
        } else {
            vec![
                ("item_cheese", 2000u32),
                ("item_faerie_fire", 85u32),
                ("item_magic_wand", 100u32),
                ("item_enchanted_mango", 175u32),
                ("item_greater_faerie_fire", 350u32),
            ]
        };

        let max_items = if in_danger && settings.danger_detection.enabled {
            settings.danger_detection.max_healing_items_per_danger
        } else {
            1 // Normal mode: only one item
        };
        drop(settings); // Release lock

        let mut items_used = 0u32;

        // Search for healing items in inventory
        for (item_name, _heal_amount) in healing_items {
            if items_used >= max_items {
                break;
            }

            for (slot, item) in event.items.all_slots() {
                if item.name == item_name {
                    // Check if item can be cast
                    if let Some(can_cast) = item.can_cast {
                        if can_cast {
                            self.use_item(slot, &item.name);
                            items_used += 1;
                            break; // Move to next item type
                        }
                    }
                }
            }
        }
    }

    fn use_item(&self, slot: &str, item_name: &str) {
        let key = {
            let settings = self.settings.lock().unwrap();
            settings.get_key_for_slot(slot)
        };

        if let Some(key) = key {
            info!("Using {} in {} (key: {})", item_name, slot, key);
            crate::input::press_key(key);
        }
    }

    /// Use defensive items when in danger
    pub fn use_defensive_items_if_danger(&self, event: &GsiWebhookEvent) {
        // Check danger state and gather config - release lock before item usage
        let (_enabled, satanic_threshold, defensive_items_config) = {
            let settings = self.settings.lock().unwrap();
            let current_config = &settings.danger_detection;

            if !current_config.enabled {
                return;
            }

            if !crate::actions::danger_detector::is_in_danger() {
                return;
            }

            if !event.hero.is_alive() {
                return;
            }

            debug!("In danger - checking defensive items");

            // Gather config before releasing lock
            let defensive_items = vec![
                ("item_black_king_bar", current_config.auto_bkb),
                ("item_satanic", current_config.auto_satanic),
                ("item_blade_mail", current_config.auto_blade_mail),
                ("item_glimmer_cape", current_config.auto_glimmer_cape),
                ("item_ghost", current_config.auto_ghost_scepter),
                ("item_shivas_guard", current_config.auto_shivas_guard),
            ];

            (true, current_config.satanic_hp_threshold, defensive_items)
        }; // Lock released here

        let mut ready_items = Vec::new();

        // Try to activate all enabled items that are ready
        for (item_name, enabled) in defensive_items_config {
            if !enabled {
                continue;
            }

            // Satanic has its own HP threshold check
            if item_name == "item_satanic" {
                let hp_percent = (event.hero.health * 100) / event.hero.max_health;
                if hp_percent > satanic_threshold {
                    debug!(
                        "Satanic not used: HP {}% > threshold {}%",
                        hp_percent, satanic_threshold
                    );
                    continue;
                }
            }

            for (slot, item) in event.items.all_slots() {
                if item.name == item_name {
                    // Check if item can be cast (not on cooldown)
                    if let Some(can_cast) = item.can_cast {
                        if can_cast {
                            debug!("Activating defensive item: {}", item_name);
                            let key = {
                                let settings = self.settings.lock().unwrap();
                                settings.get_key_for_slot(slot)
                            };

                            if let Some(key) = key {
                                info!("Using {} in {} (key: {})", item.name, slot, key);
                                ready_items.push((item.name.clone(), key));
                            }
                            break; // Move to next item type
                        }
                    }
                }
            }
        }

        if ready_items.is_empty() {
            return;
        }

        if let Some(glimmer_index) = ready_items
            .iter()
            .position(|(item_name, _)| item_name == "item_glimmer_cape")
        {
            for (_item_name, key) in &ready_items[..glimmer_index] {
                crate::input::press_key(*key);
            }

            let sequence = plan_defensive_item_key_sequence(&ready_items[glimmer_index..]);
            self.executor
                .enqueue("common-defensive-self-cast-tail", move || {
                    execute_key_sequence(sequence);
                });
            return;
        }

        for (_item_name, key) in ready_items {
            crate::input::press_key(key);
        }
    }

    /// Use neutral items when in danger
    pub fn use_neutral_item_if_danger(&self, event: &GsiWebhookEvent) {
        if !event.hero.is_alive() {
            return;
        }

        let settings = self.settings.lock().unwrap();

        // Check if neutral item usage is enabled
        if !settings.neutral_items.enabled {
            return;
        }

        // Check if use_in_danger is enabled
        if !settings.neutral_items.use_in_danger {
            return;
        }

        // Check if we're in danger
        let in_danger = crate::actions::danger_detector::is_in_danger();
        if !in_danger {
            return;
        }

        // Check if HP is below threshold
        if event.hero.health_percent >= settings.neutral_items.hp_threshold {
            return;
        }

        let neutral_item = &event.items.neutral0;

        // Check if neutral slot is not empty
        if neutral_item.name == "empty" {
            return;
        }

        // Check if item is in allowed list (exact match)
        if !settings
            .neutral_items
            .allowed_items
            .contains(&neutral_item.name)
        {
            debug!("Neutral item {} not in allowed list", neutral_item.name);
            return;
        }

        // Check if item can be cast (not on cooldown)
        if let Some(can_cast) = neutral_item.can_cast {
            if !can_cast {
                debug!("Neutral item on cooldown: {}", neutral_item.name);
                return;
            }
        } else {
            debug!("Neutral item can_cast is None: {}", neutral_item.name);
            return;
        }

        // Get keybindings
        let neutral_key = settings.keybindings.neutral0;
        let self_cast_key = settings.neutral_items.self_cast_key;

        info!(
            "⚡ Using neutral item in danger: {} (HP: {}%)",
            neutral_item.name, event.hero.health_percent
        );

        // Release lock before input simulation
        drop(settings);

        let sequence = plan_neutral_item_key_sequence(neutral_key, self_cast_key);
        self.executor.enqueue("common-neutral-self-cast", move || {
            execute_key_sequence(sequence);
        });
    }

    /// Check and toggle armlet with default configuration
    #[allow(dead_code)]
    fn check_and_toggle_armlet(&self, event: &GsiWebhookEvent) {
        let settings = self.settings.lock().unwrap();

        // Check if hero has armlet in inventory
        let has_armlet = event
            .items
            .all_slots()
            .iter()
            .any(|(_, item)| item.name == "item_armlet");

        if !has_armlet {
            return;
        }

        armlet::maybe_toggle(event, &settings);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        plan_defensive_item_key_sequence, plan_item_key_sequence, plan_neutral_item_key_sequence,
        PlannedKeyPress, SELF_CAST_DELAY_MS,
    };

    #[test]
    fn glimmer_plan_double_taps_for_self_cast() {
        assert_eq!(
            plan_item_key_sequence("item_glimmer_cape", '4'),
            vec![
                PlannedKeyPress::new('4', SELF_CAST_DELAY_MS),
                PlannedKeyPress::new('4', 0),
            ]
        );
    }

    #[test]
    fn non_self_cast_item_plan_is_single_press() {
        assert_eq!(
            plan_item_key_sequence("item_black_king_bar", '3'),
            vec![PlannedKeyPress::new('3', 0)]
        );
    }

    #[test]
    fn defensive_item_plan_keeps_glimmer_follow_up_before_later_items() {
        let items = vec![
            ("item_glimmer_cape".to_string(), '4'),
            ("item_ghost".to_string(), '5'),
        ];

        assert_eq!(
            plan_defensive_item_key_sequence(&items),
            vec![
                PlannedKeyPress::new('4', SELF_CAST_DELAY_MS),
                PlannedKeyPress::new('4', 0),
                PlannedKeyPress::new('5', 0),
            ]
        );
    }

    #[test]
    fn neutral_item_plan_waits_before_self_cast() {
        assert_eq!(
            plan_neutral_item_key_sequence('n', 'a'),
            vec![
                PlannedKeyPress::new('n', SELF_CAST_DELAY_MS),
                PlannedKeyPress::new('a', 0),
            ]
        );
    }
}

#[cfg(test)]
mod snapshot_tests {
    use crate::config::Settings;
    use crate::models::gsi_event::{GsiWebhookEvent, Hero, Ability, Abilities, Items, Item, Map};
    use super::{healing_threshold_for_event, should_consider_defensive_items, should_consider_neutral_item};

    fn empty_ability() -> Ability {
        Ability {
            ability_active: false,
            can_cast: false,
            cooldown: 0,
            level: 0,
            name: String::new(),
            passive: false,
            ultimate: false,
        }
    }

    #[test]
    fn healing_threshold_uses_passed_danger_flag() {
        let settings = Settings::default();

        assert_eq!(
            healing_threshold_for_event(&settings, true),
            settings.danger_detection.healing_threshold_in_danger
        );
        assert_eq!(
            healing_threshold_for_event(&settings, false),
            settings.common.survivability_hp_threshold
        );
    }

    #[test]
    fn defensive_items_gate_uses_passed_danger_flag() {
        let settings = Settings::default();
        let event = GsiWebhookEvent {
            hero: Hero {
                aghanims_scepter: false,
                aghanims_shard: false,
                alive: true,
                attributes_level: 0,
                is_break: false,
                buyback_cooldown: 0,
                buyback_cost: 0,
                disarmed: false,
                facet: 0,
                has_debuff: false,
                health: 100,
                health_percent: 100,
                hexed: false,
                id: 0,
                level: 1,
                magicimmune: false,
                mana: 0,
                mana_percent: 0,
                max_health: 100,
                max_mana: 0,
                muted: false,
                name: String::new(),
                respawn_seconds: 0,
                silenced: false,
                smoked: false,
                stunned: false,
                talent_1: false,
                talent_2: false,
                talent_3: false,
                talent_4: false,
                talent_5: false,
                talent_6: false,
                talent_7: false,
                talent_8: false,
                xp: 0,
                xpos: 0,
                ypos: 0,
            },
            abilities: Abilities { ability0: empty_ability(), ability1: empty_ability(), ability2: empty_ability(), ability3: empty_ability(), ability4: empty_ability(), ability5: empty_ability() },
            items: Items {
                neutral0: Item::default(),
                slot0: Item { name: "item_black_king_bar".to_string(), can_cast: Some(true), ..Default::default() },
                slot1: Default::default(),
                slot2: Default::default(),
                slot3: Default::default(),
                slot4: Default::default(),
                slot5: Default::default(),
                slot6: Default::default(),
                slot7: Default::default(),
                slot8: Default::default(),
                stash0: Default::default(),
                stash1: Default::default(),
                stash2: Default::default(),
                stash3: Default::default(),
                stash4: Default::default(),
                stash5: Default::default(),
                teleport0: Default::default(),
            },
            map: Map { clock_time: 0 },
        };

        assert!(!should_consider_defensive_items(&event, &settings, false));
        assert!(should_consider_defensive_items(&event, &settings, true));
    }

    #[test]
    fn neutral_item_gate_requires_passed_danger_flag() {
        let mut settings = Settings::default();
        settings.neutral_items.enabled = true;
        settings.neutral_items.allowed_items = vec!["item_neutral_test".to_string()];

        let event = GsiWebhookEvent {
            hero: Hero {
                aghanims_scepter: false,
                aghanims_shard: false,
                alive: true,
                attributes_level: 0,
                is_break: false,
                buyback_cooldown: 0,
                buyback_cost: 0,
                disarmed: false,
                facet: 0,
                has_debuff: false,
                health: 20,
                health_percent: 20,
                hexed: false,
                id: 0,
                level: 1,
                magicimmune: false,
                mana: 0,
                mana_percent: 0,
                max_health: 100,
                max_mana: 0,
                muted: false,
                name: String::new(),
                respawn_seconds: 0,
                silenced: false,
                smoked: false,
                stunned: false,
                talent_1: false,
                talent_2: false,
                talent_3: false,
                talent_4: false,
                talent_5: false,
                talent_6: false,
                talent_7: false,
                talent_8: false,
                xp: 0,
                xpos: 0,
                ypos: 0,
            },
            abilities: Abilities { ability0: empty_ability(), ability1: empty_ability(), ability2: empty_ability(), ability3: empty_ability(), ability4: empty_ability(), ability5: empty_ability() },
            items: Items {
                neutral0: Item { name: "item_neutral_test".to_string(), can_cast: Some(true), ..Default::default() },
                slot0: Default::default(),
                slot1: Default::default(),
                slot2: Default::default(),
                slot3: Default::default(),
                slot4: Default::default(),
                slot5: Default::default(),
                slot6: Default::default(),
                slot7: Default::default(),
                slot8: Default::default(),
                stash0: Default::default(),
                stash1: Default::default(),
                stash2: Default::default(),
                stash3: Default::default(),
                stash4: Default::default(),
                stash5: Default::default(),
                teleport0: Default::default(),
            },
            map: Map { clock_time: 0 },
        };

        assert!(!should_consider_neutral_item(&event, &settings, false));
        assert!(should_consider_neutral_item(&event, &settings, true));
    }
}
