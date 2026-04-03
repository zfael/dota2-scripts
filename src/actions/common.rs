use crate::actions::activity::{push_activity, ActivityCategory};
use crate::actions::executor::ActionExecutor;
use crate::actions::item_automation::{
    hero_is_excluded, lookup_item_automation, try_acquire_global_lockout, CastMode,
    ItemAutomationSpec, SupportStatus, TriggerFamily,
};
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Item};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, info};

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

const SELF_CAST_DELAY_MS: u64 = 50;
const ITEM_AUTOMATION_LOCKOUT_MS: u64 = 120;

#[cfg(test)]
lazy_static::lazy_static! {
    static ref LOW_MANA_CHECK_CALLS: AtomicUsize = AtomicUsize::new(0);
}

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

fn plan_automation_key_sequence(
    cast_mode: CastMode,
    item_key: char,
    self_cast_key: char,
) -> Vec<PlannedKeyPress> {
    match cast_mode {
        CastMode::SelfCast => vec![
            PlannedKeyPress::new(item_key, SELF_CAST_DELAY_MS),
            PlannedKeyPress::new(self_cast_key, 0),
        ],
        CastMode::NoTarget | CastMode::CursorTargeted => vec![PlannedKeyPress::new(item_key, 0)],
    }
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

fn item_name_matches_lookup(item_name: &str, lookup_name: &str) -> bool {
    if item_name.contains(lookup_name) {
        return true;
    }

    lookup_name == "item_blink"
        && matches!(
            item_name,
            "item_arcane_blink" | "item_overwhelming_blink" | "item_swift_blink"
        )
}

/// Find item slot key by item name string from GSI event (for backward compatibility)
pub fn find_item_slot_by_name(
    event: &GsiWebhookEvent,
    settings: &Settings,
    item_name: &str,
) -> Option<char> {
    let items = &event.items;

    // Check all inventory slots
    if item_name_matches_lookup(&items.slot0.name, item_name) {
        return settings.get_key_for_slot("slot0");
    }
    if item_name_matches_lookup(&items.slot1.name, item_name) {
        return settings.get_key_for_slot("slot1");
    }
    if item_name_matches_lookup(&items.slot2.name, item_name) {
        return settings.get_key_for_slot("slot2");
    }
    if item_name_matches_lookup(&items.slot3.name, item_name) {
        return settings.get_key_for_slot("slot3");
    }
    if item_name_matches_lookup(&items.slot4.name, item_name) {
        return settings.get_key_for_slot("slot4");
    }
    if item_name_matches_lookup(&items.slot5.name, item_name) {
        return settings.get_key_for_slot("slot5");
    }
    if item_name_matches_lookup(&items.neutral0.name, item_name) {
        return settings.get_key_for_slot("neutral0");
    }

    None
}

/// Snapshot-aware helpers for danger-aware gating used by survivability paths
#[cfg_attr(not(test), allow(dead_code))]
fn healing_threshold_for_event(event: &GsiWebhookEvent, settings: &Settings, in_danger: bool) -> u32 {
    let lane_phase_duration_seconds = settings.common.lane_phase_duration_seconds;

    if lane_phase_duration_seconds > 0
        && event.map.clock_time >= 0
        && (event.map.clock_time as u64) < lane_phase_duration_seconds
    {
        return settings.common.lane_phase_healing_threshold;
    }

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

fn current_time_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn acquire_item_trigger_lockout(lockout_key: &str, now_ms: u64, lockout_ms: u64) -> bool {
    try_acquire_global_lockout(lockout_key, now_ms, lockout_ms)
}

fn eligible_danger_neutral_spec<'a>(
    event: &GsiWebhookEvent,
    settings: &'a Settings,
    in_danger: bool,
) -> Option<&'static ItemAutomationSpec> {
    if !should_consider_neutral_item(event, settings, in_danger) {
        return None;
    }

    let neutral_name = &event.items.neutral0.name;
    let spec = lookup_item_automation(neutral_name)?;

    if spec.trigger_family != TriggerFamily::Danger {
        return None;
    }
    if spec.support != SupportStatus::Supported {
        return None;
    }
    if !spec.is_neutral {
        return None;
    }

    Some(spec)
}

fn hero_uses_mana(event: &GsiWebhookEvent) -> bool {
    event.hero.max_mana > 0
}

fn eligible_low_mana_item(
    event: &GsiWebhookEvent,
    settings: &Settings,
) -> Option<(&'static ItemAutomationSpec, char)> {
    if !settings.mana_automation.enabled {
        return None;
    }
    if !event.hero.is_alive() {
        return None;
    }
    if !hero_uses_mana(event) {
        return None;
    }
    if hero_is_excluded(&event.hero.name, &settings.mana_automation.excluded_heroes) {
        return None;
    }
    if event.hero.mana_percent >= settings.mana_automation.mana_threshold_percent {
        return None;
    }

    for (slot, item) in event.items.all_slots() {
        if item.name == "empty" || item.can_cast != Some(true) {
            continue;
        }
        if !settings.mana_automation.allowed_items.contains(&item.name) {
            continue;
        }

        let spec = lookup_item_automation(&item.name)?;
        if spec.trigger_family != TriggerFamily::LowMana {
            continue;
        }
        if spec.support != SupportStatus::Supported {
            continue;
        }

        let key = settings.get_key_for_slot(slot)?;
        return Some((spec, key));
    }

    None
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

    /// Execute default GSI strategy (danger detection + survivability)
    pub fn execute_default_strategy(&self, event: &GsiWebhookEvent) {
        // PRIORITY 1: Update danger detection state
        let in_danger = {
            let settings = self.settings.lock().unwrap();
            crate::actions::danger_detector::update(event, &settings.danger_detection)
        };

        // PRIORITY 2: Always check survivability first
        self.check_and_use_healing_items_with_danger(event, in_danger);

        // PRIORITY 3: Use defensive items if in danger
        self.use_defensive_items_if_danger_with_snapshot(event, in_danger);

        // PRIORITY 4: Use neutral items if in danger
        self.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    #[allow(dead_code)]
    /// Check if hero needs healing and use appropriate items
    pub fn check_and_use_healing_items(&self, event: &GsiWebhookEvent) {
        let in_danger = crate::actions::danger_detector::is_in_danger();
        self.check_and_use_healing_items_with_danger(event, in_danger);
    }

    pub(crate) fn check_and_use_healing_items_with_danger(
        &self,
        event: &GsiWebhookEvent,
        in_danger: bool,
    ) {
        if !event.hero.is_alive() {
            return;
        }

        let settings = self.settings.lock().unwrap();
        let threshold = healing_threshold_for_event(event, &settings, in_danger);

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
            push_activity(
                ActivityCategory::Action,
                format!("Healing item used: {}", item_name.replace("item_", "")),
            );
            crate::input::press_key(key);
        }
    }

    #[allow(dead_code)]
    /// Use defensive items when in danger
    pub fn use_defensive_items_if_danger(&self, event: &GsiWebhookEvent) {
        let in_danger = crate::actions::danger_detector::is_in_danger();
        self.use_defensive_items_if_danger_with_snapshot(event, in_danger);
    }

    pub(crate) fn use_defensive_items_if_danger_with_snapshot(
        &self,
        event: &GsiWebhookEvent,
        in_danger: bool,
    ) {
        // Check danger state and gather config - release lock before item usage
        let (_enabled, satanic_threshold, defensive_items_config) = {
            let settings = self.settings.lock().unwrap();
            let current_config = &settings.danger_detection;

            if !should_consider_defensive_items(event, &settings, in_danger) {
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
                                push_activity(
                                    ActivityCategory::Action,
                                    format!("Defensive item activated: {}", item.name.replace("item_", "")),
                                );
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

    #[allow(dead_code)]
    /// Use neutral items when in danger
    pub fn use_neutral_item_if_danger(&self, event: &GsiWebhookEvent) {
        let in_danger = crate::actions::danger_detector::is_in_danger();
        self.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    pub(crate) fn use_neutral_item_if_danger_with_snapshot(
        &self,
        event: &GsiWebhookEvent,
        in_danger: bool,
    ) {
        if !event.hero.is_alive() {
            return;
        }

        let settings = self.settings.lock().unwrap();
        let Some(spec) = eligible_danger_neutral_spec(event, &settings, in_danger) else {
            return;
        };

        let neutral_item = &event.items.neutral0;

        // Get keybindings
        let neutral_key = settings.keybindings.neutral0;
        let self_cast_key = settings.neutral_items.self_cast_key;
        let lockout_key = format!("danger:{}", neutral_item.name);
        let now_ms = current_time_millis();

        if !acquire_item_trigger_lockout(&lockout_key, now_ms, ITEM_AUTOMATION_LOCKOUT_MS) {
            debug!("Skipping duplicate danger trigger for {}", neutral_item.name);
            return;
        }

        info!(
            "⚡ Using danger automation item: {} (HP: {}%)",
            neutral_item.name, event.hero.health_percent
        );
        push_activity(
            ActivityCategory::Action,
            format!(
                "Danger automation used: {}",
                neutral_item.name.replace("item_", "")
            ),
        );

        // Release lock before input simulation
        drop(settings);

        let sequence = plan_automation_key_sequence(spec.cast_mode, neutral_key, self_cast_key);
        self.executor.enqueue("common-danger-neutral", move || {
            execute_key_sequence(sequence);
        });
    }

    pub fn check_and_use_mana_items(&self, event: &GsiWebhookEvent) {
        #[cfg(test)]
        {
            LOW_MANA_CHECK_CALLS.fetch_add(1, Ordering::SeqCst);
        }

        let settings = self.settings.lock().unwrap();
        let Some((spec, item_key)) = eligible_low_mana_item(event, &settings) else {
            return;
        };

        let self_cast_key = settings.neutral_items.self_cast_key;
        let item_name = spec.item_name.to_string();
        let lockout_key = format!("mana:{}", item_name);
        let now_ms = current_time_millis();

        if !acquire_item_trigger_lockout(&lockout_key, now_ms, ITEM_AUTOMATION_LOCKOUT_MS) {
            return;
        }

        let sequence = plan_automation_key_sequence(spec.cast_mode, item_key, self_cast_key);
        drop(settings);

        info!("💧 Using low-mana automation item: {}", item_name);
        push_activity(
            ActivityCategory::Action,
            format!("Mana automation used: {}", item_name.replace("item_", "")),
        );

        self.executor.enqueue("common-low-mana-item", move || {
            execute_key_sequence(sequence);
        });
    }
}

#[cfg(test)]
pub fn reset_low_mana_check_call_count_for_tests() {
    LOW_MANA_CHECK_CALLS.store(0, Ordering::SeqCst);
}

#[cfg(test)]
pub fn low_mana_check_call_count_for_tests() -> usize {
    LOW_MANA_CHECK_CALLS.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::{
        find_item_slot, plan_automation_key_sequence, plan_defensive_item_key_sequence,
        plan_item_key_sequence, PlannedKeyPress, SELF_CAST_DELAY_MS,
    };
    use crate::actions::item_automation::CastMode;
    use crate::config::Settings;
    use crate::models::gsi_event::{Abilities, Ability, GsiWebhookEvent, Hero, Item as GsiItem, Items, Map};
    use crate::models::Item;

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

    fn empty_hero() -> Hero {
        Hero {
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
        }
    }

    fn empty_items() -> Items {
        Items {
            neutral0: GsiItem::default(),
            slot0: GsiItem::default(),
            slot1: GsiItem::default(),
            slot2: GsiItem::default(),
            slot3: GsiItem::default(),
            slot4: GsiItem::default(),
            slot5: GsiItem::default(),
            slot6: GsiItem::default(),
            slot7: GsiItem::default(),
            slot8: GsiItem::default(),
            stash0: GsiItem::default(),
            stash1: GsiItem::default(),
            stash2: GsiItem::default(),
            stash3: GsiItem::default(),
            stash4: GsiItem::default(),
            stash5: GsiItem::default(),
            teleport0: GsiItem::default(),
        }
    }

    fn base_event(items: Items) -> GsiWebhookEvent {
        GsiWebhookEvent {
            hero: empty_hero(),
            abilities: Abilities {
                ability0: empty_ability(),
                ability1: empty_ability(),
                ability2: empty_ability(),
                ability3: empty_ability(),
                ability4: empty_ability(),
                ability5: empty_ability(),
            },
            items,
            map: Map { clock_time: 0 },
            player: None,
        }
    }

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
    fn automation_plan_for_self_cast_waits_before_tail() {
        assert_eq!(
            plan_automation_key_sequence(CastMode::SelfCast, 'n', 'a'),
            vec![
                PlannedKeyPress::new('n', SELF_CAST_DELAY_MS),
                PlannedKeyPress::new('a', 0),
            ]
        );
    }

    #[test]
    fn automation_plan_for_no_target_is_single_press() {
        assert_eq!(
            plan_automation_key_sequence(CastMode::NoTarget, 'n', 'a'),
            vec![PlannedKeyPress::new('n', 0)]
        );
    }

    #[test]
    fn automation_plan_for_cursor_targeted_is_single_press() {
        assert_eq!(
            plan_automation_key_sequence(CastMode::CursorTargeted, 'n', 'a'),
            vec![PlannedKeyPress::new('n', 0)]
        );
    }

    #[test]
    fn blink_lookup_accepts_arcane_blink_variant() {
        let settings = Settings::default();
        let mut items = empty_items();
        items.slot0 = GsiItem {
            name: "item_arcane_blink".to_string(),
            ..Default::default()
        };

        assert_eq!(
            find_item_slot(&base_event(items), &settings, Item::Blink),
            settings.get_key_for_slot("slot0")
        );
    }
}

#[cfg(test)]
mod snapshot_tests {
    use std::sync::{Arc, Mutex};

    use crate::actions::executor::ActionExecutor;
    use crate::actions::item_automation::reset_global_lockouts_for_tests;
    use crate::config::Settings;
    use crate::models::gsi_event::{Abilities, Ability, GsiWebhookEvent, Hero, Item, Items, Map};

    use super::{
        acquire_item_trigger_lockout, eligible_danger_neutral_spec, eligible_low_mana_item,
        healing_threshold_for_event, should_consider_defensive_items, should_consider_neutral_item,
        SurvivabilityActions,
    };

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

    fn hero_with_health(health: u32, health_percent: u32) -> Hero {
        Hero {
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
            health,
            health_percent,
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
        }
    }

    fn empty_abilities() -> Abilities {
        Abilities {
            ability0: empty_ability(),
            ability1: empty_ability(),
            ability2: empty_ability(),
            ability3: empty_ability(),
            ability4: empty_ability(),
            ability5: empty_ability(),
        }
    }

    fn empty_items() -> Items {
        Items {
            neutral0: Item::default(),
            slot0: Item::default(),
            slot1: Item::default(),
            slot2: Item::default(),
            slot3: Item::default(),
            slot4: Item::default(),
            slot5: Item::default(),
            slot6: Item::default(),
            slot7: Item::default(),
            slot8: Item::default(),
            stash0: Item::default(),
            stash1: Item::default(),
            stash2: Item::default(),
            stash3: Item::default(),
            stash4: Item::default(),
            stash5: Item::default(),
            teleport0: Item::default(),
        }
    }

    fn base_event(hero: Hero, items: Items) -> GsiWebhookEvent {
        GsiWebhookEvent {
            hero,
            abilities: empty_abilities(),
            items,
            map: Map { clock_time: 0 },
            player: None,
        }
    }

    fn test_actions(settings: Settings) -> SurvivabilityActions {
        SurvivabilityActions::new(Arc::new(Mutex::new(settings)), ActionExecutor::new())
    }

    #[test]
    fn healing_threshold_uses_passed_danger_flag_after_lane_phase() {
        let settings = Settings::default();
        let mut event = base_event(hero_with_health(100, 100), empty_items());
        event.map.clock_time = 900;

        assert_eq!(
            healing_threshold_for_event(&event, &settings, true),
            settings.danger_detection.healing_threshold_in_danger
        );
        assert_eq!(
            healing_threshold_for_event(&event, &settings, false),
            settings.common.survivability_hp_threshold
        );
    }

    #[test]
    fn lane_phase_healing_threshold_overrides_danger_before_cutoff() {
        let settings = Settings::default();
        let mut event = base_event(hero_with_health(100, 100), empty_items());
        event.map.clock_time = 479;

        assert_eq!(healing_threshold_for_event(&event, &settings, true), 12);
    }

    #[test]
    fn lane_phase_healing_threshold_expires_at_cutoff() {
        let settings = Settings::default();
        let mut event = base_event(hero_with_health(100, 100), empty_items());
        event.map.clock_time = 480;

        assert_eq!(
            healing_threshold_for_event(&event, &settings, true),
            settings.danger_detection.healing_threshold_in_danger
        );
    }

    #[test]
    fn lane_phase_healing_threshold_falls_back_to_danger_after_cutoff() {
        let settings = Settings::default();
        let mut event = base_event(hero_with_health(100, 100), empty_items());
        event.map.clock_time = 900;

        assert_eq!(
            healing_threshold_for_event(&event, &settings, true),
            settings.danger_detection.healing_threshold_in_danger
        );
    }

    #[test]
    fn lane_phase_healing_threshold_falls_back_to_normal_after_cutoff() {
        let settings = Settings::default();
        let mut event = base_event(hero_with_health(100, 100), empty_items());
        event.map.clock_time = 900;

        assert_eq!(
            healing_threshold_for_event(&event, &settings, false),
            settings.common.survivability_hp_threshold
        );
    }

    #[test]
    fn lane_phase_healing_threshold_is_disabled_when_duration_is_zero() {
        let mut settings = Settings::default();
        settings.common.lane_phase_duration_seconds = 0;
        let mut event = base_event(hero_with_health(100, 100), empty_items());
        event.map.clock_time = 120;

        assert_eq!(
            healing_threshold_for_event(&event, &settings, true),
            settings.danger_detection.healing_threshold_in_danger
        );
    }

    #[test]
    fn lane_phase_healing_threshold_ignores_negative_clock_time() {
        let settings = Settings::default();
        let mut event = base_event(hero_with_health(100, 100), empty_items());
        event.map.clock_time = -30;

        assert_eq!(
            healing_threshold_for_event(&event, &settings, true),
            settings.danger_detection.healing_threshold_in_danger
        );
    }

    #[test]
    fn defensive_items_gate_uses_passed_danger_flag() {
        let settings = Settings::default();
        let mut items = empty_items();
        items.slot0 = Item {
            name: "item_black_king_bar".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };
        let event = base_event(hero_with_health(100, 100), items);

        assert!(!should_consider_defensive_items(&event, &settings, false));
        assert!(should_consider_defensive_items(&event, &settings, true));
    }

    #[test]
    fn neutral_item_gate_requires_passed_danger_flag() {
        let mut settings = Settings::default();
        settings.neutral_items.enabled = true;
        settings.neutral_items.allowed_items = vec!["item_neutral_test".to_string()];
        let mut items = empty_items();
        items.neutral0 = Item {
            name: "item_neutral_test".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };
        let event = base_event(hero_with_health(20, 20), items);

        assert!(!should_consider_neutral_item(&event, &settings, false));
        assert!(should_consider_neutral_item(&event, &settings, true));
    }

    #[test]
    fn danger_neutral_gate_accepts_supported_no_target_item() {
        let mut settings = Settings::default();
        settings.neutral_items.enabled = true;
        settings.neutral_items.allowed_items = vec!["item_jidi_pollen_bag".to_string()];
        let mut items = empty_items();
        items.neutral0 = Item {
            name: "item_jidi_pollen_bag".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };
        let event = base_event(hero_with_health(20, 20), items);

        assert!(eligible_danger_neutral_spec(&event, &settings, true).is_some());
    }

    #[test]
    fn danger_neutral_gate_rejects_known_unsupported_item_even_if_configured() {
        let mut settings = Settings::default();
        settings.neutral_items.enabled = true;
        settings.neutral_items.allowed_items = vec!["item_psychic_headband".to_string()];
        let mut items = empty_items();
        items.neutral0 = Item {
            name: "item_psychic_headband".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };
        let event = base_event(hero_with_health(20, 20), items);

        assert!(eligible_danger_neutral_spec(&event, &settings, true).is_none());
    }

    #[test]
    fn danger_neutral_gate_respects_global_lockout() {
        reset_global_lockouts_for_tests();

        assert!(acquire_item_trigger_lockout(
            "danger:item_jidi_pollen_bag",
            1_000,
            120
        ));
        assert!(!acquire_item_trigger_lockout(
            "danger:item_jidi_pollen_bag",
            1_050,
            120
        ));
        assert!(acquire_item_trigger_lockout(
            "danger:item_jidi_pollen_bag",
            1_200,
            120
        ));
    }

    #[test]
    fn low_mana_gate_accepts_arcane_boots_for_supported_mana_user() {
        let mut settings = Settings::default();
        settings.mana_automation.enabled = true;
        settings.mana_automation.allowed_items = vec![
            "item_arcane_boots".to_string(),
            "item_mana_draught".to_string(),
        ];
        let mut items = empty_items();
        items.slot0 = Item {
            name: "item_arcane_boots".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };

        let mut hero = hero_with_health(100, 100);
        hero.name = "npc_dota_hero_zuus".to_string();
        hero.mana = 100;
        hero.max_mana = 500;
        hero.mana_percent = 20;

        let event = base_event(hero, items);
        let (spec, slot_key) = eligible_low_mana_item(&event, &settings).unwrap();

        assert_eq!(spec.item_name, "item_arcane_boots");
        assert_eq!(slot_key, settings.keybindings.slot0);
    }

    #[test]
    fn low_mana_gate_excludes_huskar() {
        let mut settings = Settings::default();
        settings.mana_automation.enabled = true;
        settings.mana_automation.allowed_items = vec!["item_arcane_boots".to_string()];
        settings.mana_automation.excluded_heroes = vec!["npc_dota_hero_huskar".to_string()];
        let mut items = empty_items();
        items.slot0 = Item {
            name: "item_arcane_boots".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };

        let mut hero = hero_with_health(100, 100);
        hero.name = "npc_dota_hero_huskar".to_string();
        hero.mana = 50;
        hero.max_mana = 200;
        hero.mana_percent = 20;

        let event = base_event(hero, items);
        assert!(eligible_low_mana_item(&event, &settings).is_none());
    }

    #[test]
    fn low_mana_gate_finds_mana_draught_in_neutral_slot() {
        let mut settings = Settings::default();
        settings.mana_automation.enabled = true;
        settings.mana_automation.allowed_items = vec!["item_mana_draught".to_string()];
        let mut items = empty_items();
        items.neutral0 = Item {
            name: "item_mana_draught".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };

        let mut hero = hero_with_health(100, 100);
        hero.name = "npc_dota_hero_lina".to_string();
        hero.mana = 80;
        hero.max_mana = 400;
        hero.mana_percent = 20;

        let event = base_event(hero, items);
        let (spec, slot_key) = eligible_low_mana_item(&event, &settings).unwrap();

        assert_eq!(spec.item_name, "item_mana_draught");
        assert_eq!(slot_key, settings.keybindings.neutral0);
    }

    #[test]
    fn check_and_use_healing_items_with_danger_uses_passed_flag_without_tracker_setup() {
        let actions = test_actions(Settings::default());
        let event = base_event(hero_with_health(40, 40), empty_items());

        actions.check_and_use_healing_items_with_danger(&event, true);
    }

    #[test]
    fn use_defensive_items_if_danger_with_snapshot_returns_early_when_flag_is_false() {
        let actions = test_actions(Settings::default());
        let mut items = empty_items();
        items.slot0 = Item {
            name: "item_black_king_bar".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };
        let event = base_event(hero_with_health(20, 20), items);

        actions.use_defensive_items_if_danger_with_snapshot(&event, false);
    }

    #[test]
    fn use_neutral_item_if_danger_with_snapshot_returns_early_when_flag_is_false() {
        let mut settings = Settings::default();
        settings.neutral_items.enabled = true;
        settings.neutral_items.allowed_items = vec!["item_neutral_test".to_string()];
        let actions = test_actions(settings);
        let mut items = empty_items();
        items.neutral0 = Item {
            name: "item_neutral_test".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };
        let event = base_event(hero_with_health(20, 20), items);

        actions.use_neutral_item_if_danger_with_snapshot(&event, false);
    }
}
