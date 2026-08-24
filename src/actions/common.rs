use crate::actions::activity::{push_activity, ActivityCategory};
use crate::actions::executor::ActionExecutor;
use crate::actions::invisibility;
use crate::actions::item_automation::{
    clear_movement_snapshot, hero_is_excluded, lookup_item_automation, read_movement_snapshot,
    try_acquire_global_lockout, write_movement_snapshot, CastMode, ItemAutomationSpec,
    MovementSnapshot, SupportStatus, TriggerFamily,
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
const MOVEMENT_IDLE_TOLERANCE_UNITS: f64 = 5.0;
const MOVEMENT_IDLE_RESET_SAMPLES: u8 = 2;

#[cfg(test)]
lazy_static::lazy_static! {
    static ref LOW_MANA_CHECK_CALLS: AtomicUsize = AtomicUsize::new(0);
    static ref MOVEMENT_CHECK_CALLS: AtomicUsize = AtomicUsize::new(0);
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

/// Defensive items that target a unit, so the key has to be double-tapped to
/// land on ourselves instead of waiting for a cursor click.
fn defensive_item_needs_self_cast(item_name: &str) -> bool {
    matches!(item_name, "item_glimmer_cape" | "item_mjollnir")
}

fn plan_item_key_sequence(item_name: &str, key: char) -> Vec<PlannedKeyPress> {
    if defensive_item_needs_self_cast(item_name) {
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
fn healing_threshold_for_event(
    event: &GsiWebhookEvent,
    settings: &Settings,
    in_danger: bool,
) -> u32 {
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

/// Decide which healing items to fire, in press order, as `(slot, item_name)`.
///
/// Pure so the selection can be asserted in tests: the caller owns the key
/// presses, which is why the old inline version had no coverage beyond "does
/// not panic".
fn plan_healing_items<'a>(
    event: &'a GsiWebhookEvent,
    settings: &Settings,
    in_danger: bool,
) -> Vec<(&'a str, String)> {
    if !event.hero.is_alive() {
        return Vec::new();
    }

    let threshold = healing_threshold_for_event(event, settings, in_danger);
    if event.hero.health_percent >= threshold {
        return Vec::new();
    }

    debug!(
        "HP below threshold: {}% < {}% (in_danger: {})",
        event.hero.health_percent, threshold, in_danger
    );

    // Priority order - high value first when in danger, low value first otherwise
    let healing_items = if in_danger {
        [
            "item_cheese",
            "item_greater_faerie_fire",
            "item_enchanted_mango",
            "item_magic_wand",
            "item_magic_stick",
            "item_faerie_fire",
        ]
    } else {
        [
            "item_cheese",
            "item_magic_stick",
            "item_faerie_fire",
            "item_magic_wand",
            "item_enchanted_mango",
            "item_greater_faerie_fire",
        ]
    };

    let max_items = if in_danger && settings.danger_detection.enabled {
        settings.danger_detection.max_healing_items_per_danger
    } else {
        1 // Normal mode: only one item
    };

    let mut plan = Vec::new();

    for item_name in healing_items {
        if plan.len() as u32 >= max_items {
            break;
        }

        for (slot, item) in event.items.all_slots() {
            if item.name == item_name && item.can_cast == Some(true) {
                plan.push((slot, item.name.clone()));
                break; // Move to next item type
            }
        }
    }

    plan
}

#[cfg_attr(not(test), allow(dead_code))]
fn should_consider_defensive_items(
    event: &GsiWebhookEvent,
    settings: &Settings,
    in_danger: bool,
) -> bool {
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
fn should_consider_neutral_item(
    event: &GsiWebhookEvent,
    settings: &Settings,
    in_danger: bool,
) -> bool {
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

fn movement_distance_units(
    previous_xpos: i32,
    previous_ypos: i32,
    current_xpos: i32,
    current_ypos: i32,
) -> f64 {
    let dx = (current_xpos - previous_xpos) as f64;
    let dy = (current_ypos - previous_ypos) as f64;
    (dx * dx + dy * dy).sqrt()
}

fn new_movement_snapshot(event: &GsiWebhookEvent) -> MovementSnapshot {
    MovementSnapshot {
        hero_name: event.hero.name.clone(),
        alive: event.hero.alive,
        anchor_xpos: event.hero.xpos,
        anchor_ypos: event.hero.ypos,
        last_xpos: event.hero.xpos,
        last_ypos: event.hero.ypos,
        idle_samples: 0,
    }
}

fn advance_movement_snapshot(
    previous: Option<MovementSnapshot>,
    event: &GsiWebhookEvent,
) -> MovementSnapshot {
    let Some(previous) = previous else {
        return new_movement_snapshot(event);
    };

    if !previous.alive || previous.hero_name != event.hero.name {
        return new_movement_snapshot(event);
    }

    let step_distance = movement_distance_units(
        previous.last_xpos,
        previous.last_ypos,
        event.hero.xpos,
        event.hero.ypos,
    );

    if step_distance <= MOVEMENT_IDLE_TOLERANCE_UNITS {
        let idle_samples = previous.idle_samples.saturating_add(1);
        if idle_samples >= MOVEMENT_IDLE_RESET_SAMPLES {
            return new_movement_snapshot(event);
        }

        return MovementSnapshot {
            hero_name: previous.hero_name,
            alive: event.hero.alive,
            anchor_xpos: previous.anchor_xpos,
            anchor_ypos: previous.anchor_ypos,
            last_xpos: event.hero.xpos,
            last_ypos: event.hero.ypos,
            idle_samples,
        };
    }

    MovementSnapshot {
        hero_name: previous.hero_name,
        alive: event.hero.alive,
        anchor_xpos: previous.anchor_xpos,
        anchor_ypos: previous.anchor_ypos,
        last_xpos: event.hero.xpos,
        last_ypos: event.hero.ypos,
        idle_samples: 0,
    }
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

fn eligible_movement_item(
    event: &GsiWebhookEvent,
    settings: &Settings,
) -> Option<(&'static ItemAutomationSpec, char)> {
    if !settings.phase_boots_automation.enabled {
        return None;
    }
    if !event.hero.is_alive() {
        return None;
    }
    if hero_is_excluded(
        &event.hero.name,
        &settings.phase_boots_automation.excluded_heroes,
    ) {
        return None;
    }
    if invisibility::suppresses_automation(settings) {
        return None;
    }

    let previous = read_movement_snapshot()?;
    let movement = advance_movement_snapshot(Some(previous), event);
    if !movement.alive || movement.hero_name != event.hero.name {
        return None;
    }
    if movement_distance_units(
        movement.anchor_xpos,
        movement.anchor_ypos,
        movement.last_xpos,
        movement.last_ypos,
    ) < settings.phase_boots_automation.minimum_distance_units as f64
    {
        return None;
    }

    for (slot, item) in event.items.all_slots() {
        if item.name == "empty" || item.can_cast != Some(true) {
            continue;
        }

        let Some(spec) = lookup_item_automation(&item.name) else {
            continue;
        };
        if spec.trigger_family != TriggerFamily::Movement {
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
        let plan = {
            let settings = self.settings.lock().unwrap();
            if invisibility::suppresses_automation(&settings) {
                return;
            }
            plan_healing_items(event, &settings, in_danger)
        }; // Lock released before any key press

        for (slot, item_name) in plan {
            self.use_item(slot, &item_name);
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
            if invisibility::suppresses_automation(&settings) {
                return;
            }

            debug!("In danger - checking defensive items");

            // Gather config before releasing lock
            let defensive_items = vec![
                ("item_black_king_bar", current_config.auto_bkb),
                ("item_satanic", current_config.auto_satanic),
                ("item_blade_mail", current_config.auto_blade_mail),
                ("item_mjollnir", current_config.auto_mjollnir),
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
                // max_health is 0 in menu/draft payloads; dividing by it panics
                // the whole GSI processing task.
                if event.hero.max_health == 0 {
                    continue;
                }
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
                                    format!(
                                        "Defensive item activated: {}",
                                        item.name.replace("item_", "")
                                    ),
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

        if let Some(self_cast_index) = ready_items
            .iter()
            .position(|(item_name, _)| defensive_item_needs_self_cast(item_name))
        {
            for (_item_name, key) in &ready_items[..self_cast_index] {
                crate::input::press_key(*key);
            }

            let sequence = plan_defensive_item_key_sequence(&ready_items[self_cast_index..]);
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
        if invisibility::suppresses_automation(&settings) {
            return;
        }
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
            debug!(
                "Skipping duplicate danger trigger for {}",
                neutral_item.name
            );
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
        if invisibility::suppresses_automation(&settings) {
            return;
        }
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

    pub fn check_and_use_movement_items(&self, event: &GsiWebhookEvent) {
        #[cfg(test)]
        {
            MOVEMENT_CHECK_CALLS.fetch_add(1, Ordering::SeqCst);
        }

        let settings = self.settings.lock().unwrap();

        if !event.hero.is_alive() {
            clear_movement_snapshot();
            invisibility::clear_snapshot();
            return;
        }

        let current_snapshot = advance_movement_snapshot(read_movement_snapshot(), event);
        let decision = if !settings.phase_boots_automation.enabled
            || hero_is_excluded(
                &event.hero.name,
                &settings.phase_boots_automation.excluded_heroes,
            )
            || movement_distance_units(
                current_snapshot.anchor_xpos,
                current_snapshot.anchor_ypos,
                current_snapshot.last_xpos,
                current_snapshot.last_ypos,
            ) < settings.phase_boots_automation.minimum_distance_units as f64
        {
            None
        } else {
            eligible_movement_item(event, &settings)
        };

        write_movement_snapshot(current_snapshot.clone());

        let Some((spec, item_key)) = decision else {
            return;
        };

        let lockout_key = format!("movement:{}", spec.item_name);
        let now_ms = current_time_millis();
        if !acquire_item_trigger_lockout(&lockout_key, now_ms, ITEM_AUTOMATION_LOCKOUT_MS) {
            return;
        }

        let sequence = plan_automation_key_sequence(
            spec.cast_mode,
            item_key,
            settings.neutral_items.self_cast_key,
        );
        write_movement_snapshot(new_movement_snapshot(event));
        drop(settings);

        info!("🏃 Using movement automation item: {}", spec.item_name);
        push_activity(
            ActivityCategory::Action,
            format!(
                "Movement automation used: {}",
                spec.item_name.replace("item_", "")
            ),
        );

        self.executor.enqueue("common-movement-item", move || {
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
pub fn reset_movement_check_call_count_for_tests() {
    MOVEMENT_CHECK_CALLS.store(0, Ordering::SeqCst);
}

#[cfg(test)]
pub fn movement_check_call_count_for_tests() -> usize {
    MOVEMENT_CHECK_CALLS.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::{
        find_item_slot, plan_automation_key_sequence, plan_defensive_item_key_sequence,
        plan_item_key_sequence, PlannedKeyPress, SELF_CAST_DELAY_MS,
    };
    use crate::actions::item_automation::CastMode;
    use crate::config::Settings;
    use crate::models::gsi_event::{
        Abilities, Ability, GsiWebhookEvent, Hero, Item as GsiItem, Items, Map,
    };
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
    fn mjollnir_plan_double_taps_for_self_cast() {
        assert_eq!(
            plan_item_key_sequence("item_mjollnir", '2'),
            vec![
                PlannedKeyPress::new('2', SELF_CAST_DELAY_MS),
                PlannedKeyPress::new('2', 0),
            ]
        );
    }

    #[test]
    fn defensive_item_plan_double_taps_every_self_cast_item() {
        let items = vec![
            ("item_mjollnir".to_string(), '2'),
            ("item_glimmer_cape".to_string(), '4'),
            ("item_ghost".to_string(), '5'),
        ];

        assert_eq!(
            plan_defensive_item_key_sequence(&items),
            vec![
                PlannedKeyPress::new('2', SELF_CAST_DELAY_MS),
                PlannedKeyPress::new('2', 0),
                PlannedKeyPress::new('4', SELF_CAST_DELAY_MS),
                PlannedKeyPress::new('4', 0),
                PlannedKeyPress::new('5', 0),
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
    use crate::actions::invisibility;
    use crate::actions::item_automation::{
        read_movement_snapshot, replace_movement_snapshot_for_tests,
        reset_global_lockouts_for_tests, MovementSnapshot,
    };
    use crate::config::Settings;
    use crate::models::gsi_event::{Abilities, Ability, GsiWebhookEvent, Hero, Item, Items, Map};

    use super::{
        acquire_item_trigger_lockout, eligible_danger_neutral_spec, eligible_low_mana_item,
        eligible_movement_item, healing_threshold_for_event, plan_healing_items,
        should_consider_defensive_items, should_consider_neutral_item, SurvivabilityActions,
    };

    /// Guards the movement snapshot, the global lockouts and the invisibility
    /// tracker. It *is* the invisibility lock rather than a second one beside
    /// it — these tests swap the invisibility snapshot, so a separate lock would
    /// serialise nothing.
    fn shared_state_test_lock() -> &'static Mutex<()> {
        invisibility::snapshot_test_lock()
    }

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
        let _guard = shared_state_test_lock().lock().unwrap();
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
    fn movement_gate_requires_previous_sample_before_phase_boots_can_trigger() {
        let _guard = shared_state_test_lock().lock().unwrap();
        reset_global_lockouts_for_tests();
        replace_movement_snapshot_for_tests(None);

        let mut settings = Settings::default();
        settings.phase_boots_automation.enabled = true;
        settings.phase_boots_automation.minimum_distance_units = 100;

        let mut items = empty_items();
        items.slot0 = Item {
            name: "item_phase_boots".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };

        let mut hero = hero_with_health(100, 100);
        hero.name = "npc_dota_hero_axe".to_string();
        hero.xpos = 1000;
        hero.ypos = 1000;

        let event = base_event(hero, items);
        assert!(eligible_movement_item(&event, &settings).is_none());
    }

    #[test]
    fn movement_gate_rejects_sub_threshold_motion_and_accepts_real_travel() {
        let _guard = shared_state_test_lock().lock().unwrap();
        reset_global_lockouts_for_tests();

        let mut settings = Settings::default();
        settings.phase_boots_automation.enabled = true;
        settings.phase_boots_automation.minimum_distance_units = 100;

        let mut items = empty_items();
        items.slot0 = Item {
            name: "item_phase_boots".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };

        replace_movement_snapshot_for_tests(Some(MovementSnapshot {
            hero_name: "npc_dota_hero_axe".to_string(),
            alive: true,
            anchor_xpos: 1000,
            anchor_ypos: 1000,
            last_xpos: 1000,
            last_ypos: 1000,
            idle_samples: 0,
        }));

        let mut jitter_hero = hero_with_health(100, 100);
        jitter_hero.name = "npc_dota_hero_axe".to_string();
        jitter_hero.xpos = 1030;
        jitter_hero.ypos = 1040;
        let jitter_event = base_event(jitter_hero, items.clone());
        assert!(eligible_movement_item(&jitter_event, &settings).is_none());

        let mut travel_hero = hero_with_health(100, 100);
        travel_hero.name = "npc_dota_hero_axe".to_string();
        travel_hero.xpos = 1120;
        travel_hero.ypos = 1000;
        let travel_event = base_event(travel_hero, items);
        let (spec, slot_key) = eligible_movement_item(&travel_event, &settings).unwrap();

        assert_eq!(spec.item_name, "item_phase_boots");
        assert_eq!(slot_key, settings.keybindings.slot0);
    }

    /// A hero walking far enough to arm the trigger, with Phase Boots ready.
    fn walking_phase_boots_event() -> GsiWebhookEvent {
        let mut items = empty_items();
        items.slot0 = Item {
            name: "item_phase_boots".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };

        replace_movement_snapshot_for_tests(Some(MovementSnapshot {
            hero_name: "npc_dota_hero_axe".to_string(),
            alive: true,
            anchor_xpos: 1000,
            anchor_ypos: 1000,
            last_xpos: 1000,
            last_ypos: 1000,
            idle_samples: 0,
        }));

        let mut hero = hero_with_health(100, 100);
        hero.name = "npc_dota_hero_axe".to_string();
        hero.xpos = 1120;
        hero.ypos = 1000;
        base_event(hero, items)
    }

    #[test]
    fn movement_gate_holds_phase_boots_while_invisible() {
        let _guard = shared_state_test_lock().lock().unwrap();
        reset_global_lockouts_for_tests();

        let mut settings = Settings::default();
        settings.phase_boots_automation.enabled = true;
        settings.phase_boots_automation.minimum_distance_units = 100;

        let event = walking_phase_boots_event();
        invisibility::replace_snapshot_for_tests(Some(invisibility::active_snapshot_for_tests(
            "npc_dota_hero_axe",
        )));
        let while_invisible = eligible_movement_item(&event, &settings);

        // The same event fires the moment the window closes: suppressed, then
        // resumed, with nothing queued in between.
        invisibility::replace_snapshot_for_tests(None);
        let after_invisible = eligible_movement_item(&event, &settings);

        assert!(while_invisible.is_none());
        assert!(after_invisible.is_some());
    }

    #[test]
    fn movement_gate_ignores_invisibility_when_suppression_is_disabled() {
        let _guard = shared_state_test_lock().lock().unwrap();
        reset_global_lockouts_for_tests();

        let mut settings = Settings::default();
        settings.phase_boots_automation.enabled = true;
        settings.phase_boots_automation.minimum_distance_units = 100;
        settings.invisibility.suppress_automation = false;

        let event = walking_phase_boots_event();
        invisibility::replace_snapshot_for_tests(Some(invisibility::active_snapshot_for_tests(
            "npc_dota_hero_axe",
        )));

        let eligible = eligible_movement_item(&event, &settings);
        invisibility::replace_snapshot_for_tests(None);

        assert_eq!(
            eligible.map(|(spec, _)| spec.item_name),
            Some("item_phase_boots")
        );
    }

    #[test]
    fn movement_gate_can_cross_threshold_across_multiple_walking_samples() {
        let _guard = shared_state_test_lock().lock().unwrap();
        reset_global_lockouts_for_tests();
        replace_movement_snapshot_for_tests(None);

        let mut settings = Settings::default();
        settings.phase_boots_automation.enabled = true;
        settings.phase_boots_automation.minimum_distance_units = 90;

        let mut items = empty_items();
        items.slot0 = Item {
            name: "item_phase_boots".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };

        let actions = test_actions(settings.clone());

        let mut start_hero = hero_with_health(100, 100);
        start_hero.name = "npc_dota_hero_axe".to_string();
        start_hero.xpos = 1000;
        start_hero.ypos = 1000;
        actions.check_and_use_movement_items(&base_event(start_hero, items.clone()));

        let mut mid_hero = hero_with_health(100, 100);
        mid_hero.name = "npc_dota_hero_axe".to_string();
        mid_hero.xpos = 1045;
        mid_hero.ypos = 1000;
        actions.check_and_use_movement_items(&base_event(mid_hero, items.clone()));

        let mut end_hero = hero_with_health(100, 100);
        end_hero.name = "npc_dota_hero_axe".to_string();
        end_hero.xpos = 1090;
        end_hero.ypos = 1000;
        let end_event = base_event(end_hero, items);

        let (spec, slot_key) = eligible_movement_item(&end_event, &settings).unwrap();

        assert_eq!(spec.item_name, "item_phase_boots");
        assert_eq!(slot_key, settings.keybindings.slot0);
    }

    #[test]
    fn movement_gate_resets_after_idle_before_counting_a_new_walk() {
        let _guard = shared_state_test_lock().lock().unwrap();
        reset_global_lockouts_for_tests();
        replace_movement_snapshot_for_tests(None);

        let mut settings = Settings::default();
        settings.phase_boots_automation.enabled = true;
        settings.phase_boots_automation.minimum_distance_units = 90;

        let mut items = empty_items();
        items.slot0 = Item {
            name: "item_phase_boots".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };

        let actions = test_actions(settings.clone());

        let mut start_hero = hero_with_health(100, 100);
        start_hero.name = "npc_dota_hero_axe".to_string();
        start_hero.xpos = 1000;
        start_hero.ypos = 1000;
        actions.check_and_use_movement_items(&base_event(start_hero, items.clone()));

        let mut short_walk_hero = hero_with_health(100, 100);
        short_walk_hero.name = "npc_dota_hero_axe".to_string();
        short_walk_hero.xpos = 1045;
        short_walk_hero.ypos = 1000;
        actions.check_and_use_movement_items(&base_event(short_walk_hero.clone(), items.clone()));
        actions.check_and_use_movement_items(&base_event(short_walk_hero.clone(), items.clone()));
        actions.check_and_use_movement_items(&base_event(short_walk_hero, items.clone()));

        assert_eq!(
            read_movement_snapshot(),
            Some(MovementSnapshot {
                hero_name: "npc_dota_hero_axe".to_string(),
                alive: true,
                anchor_xpos: 1045,
                anchor_ypos: 1000,
                last_xpos: 1045,
                last_ypos: 1000,
                idle_samples: 0,
            })
        );

        let mut second_walk_hero = hero_with_health(100, 100);
        second_walk_hero.name = "npc_dota_hero_axe".to_string();
        second_walk_hero.xpos = 1090;
        second_walk_hero.ypos = 1000;
        let second_walk_event = base_event(second_walk_hero, items);

        assert!(eligible_movement_item(&second_walk_event, &settings).is_none());
    }

    #[test]
    fn movement_gate_respects_phase_boots_excluded_heroes() {
        let _guard = shared_state_test_lock().lock().unwrap();
        let mut settings = Settings::default();
        settings.phase_boots_automation.enabled = true;
        settings.phase_boots_automation.excluded_heroes = vec!["npc_dota_hero_huskar".to_string()];

        let mut items = empty_items();
        items.slot0 = Item {
            name: "item_phase_boots".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };

        replace_movement_snapshot_for_tests(Some(MovementSnapshot {
            hero_name: "npc_dota_hero_huskar".to_string(),
            alive: true,
            anchor_xpos: 1000,
            anchor_ypos: 1000,
            last_xpos: 1000,
            last_ypos: 1000,
            idle_samples: 0,
        }));

        let mut hero = hero_with_health(100, 100);
        hero.name = "npc_dota_hero_huskar".to_string();
        hero.xpos = 1200;
        hero.ypos = 1000;

        let event = base_event(hero, items);
        assert!(eligible_movement_item(&event, &settings).is_none());
    }

    #[test]
    fn movement_gate_skips_non_automation_items_before_phase_boots() {
        let _guard = shared_state_test_lock().lock().unwrap();
        let mut settings = Settings::default();
        settings.phase_boots_automation.enabled = true;
        settings.phase_boots_automation.minimum_distance_units = 100;

        let mut items = empty_items();
        items.slot0 = Item {
            name: "item_bracer".to_string(),
            can_cast: Some(false),
            ..Default::default()
        };
        items.slot1 = Item {
            name: "item_phase_boots".to_string(),
            can_cast: Some(true),
            ..Default::default()
        };

        replace_movement_snapshot_for_tests(Some(MovementSnapshot {
            hero_name: "npc_dota_hero_axe".to_string(),
            alive: true,
            anchor_xpos: 1000,
            anchor_ypos: 1000,
            last_xpos: 1000,
            last_ypos: 1000,
            idle_samples: 0,
        }));

        let mut hero = hero_with_health(100, 100);
        hero.name = "npc_dota_hero_axe".to_string();
        hero.xpos = 1200;
        hero.ypos = 1000;

        let event = base_event(hero, items);
        let (spec, slot_key) = eligible_movement_item(&event, &settings).unwrap();

        assert_eq!(spec.item_name, "item_phase_boots");
        assert_eq!(slot_key, settings.keybindings.slot1);
    }

    #[test]
    fn movement_check_records_first_sample_for_future_events() {
        let _guard = shared_state_test_lock().lock().unwrap();
        replace_movement_snapshot_for_tests(None);

        let actions = test_actions(Settings::default());
        let mut hero = hero_with_health(100, 100);
        hero.name = "npc_dota_hero_axe".to_string();
        hero.xpos = 1000;
        hero.ypos = 1000;

        actions.check_and_use_movement_items(&base_event(hero, empty_items()));

        assert_eq!(
            read_movement_snapshot(),
            Some(MovementSnapshot {
                hero_name: "npc_dota_hero_axe".to_string(),
                alive: true,
                anchor_xpos: 1000,
                anchor_ypos: 1000,
                last_xpos: 1000,
                last_ypos: 1000,
                idle_samples: 0,
            })
        );
    }

    #[test]
    fn movement_check_clears_snapshot_when_hero_is_dead() {
        let _guard = shared_state_test_lock().lock().unwrap();
        replace_movement_snapshot_for_tests(Some(MovementSnapshot {
            hero_name: "npc_dota_hero_axe".to_string(),
            alive: true,
            anchor_xpos: 1000,
            anchor_ypos: 1000,
            last_xpos: 1000,
            last_ypos: 1000,
            idle_samples: 0,
        }));

        let actions = test_actions(Settings::default());
        let mut hero = hero_with_health(100, 100);
        hero.name = "npc_dota_hero_axe".to_string();
        hero.alive = false;
        hero.xpos = 1200;
        hero.ypos = 1200;

        actions.check_and_use_movement_items(&base_event(hero, empty_items()));
        assert_eq!(read_movement_snapshot(), None);
    }

    #[test]
    fn check_and_use_healing_items_with_danger_uses_passed_flag_without_tracker_setup() {
        let actions = test_actions(Settings::default());
        let event = base_event(hero_with_health(40, 40), empty_items());

        actions.check_and_use_healing_items_with_danger(&event, true);
    }

    fn castable(name: &str) -> Item {
        Item {
            name: name.to_string(),
            can_cast: Some(true),
            ..Default::default()
        }
    }

    /// `base_event` starts at `clock_time = 0`, which is inside the lane-phase
    /// window. Tests about the normal/danger thresholds have to step past it.
    fn post_lane_event(hero: Hero, items: Items) -> GsiWebhookEvent {
        let mut event = base_event(hero, items);
        event.map.clock_time = 600;
        event
    }

    /// The reported symptom: a hero at 1% HP holding healing items. Every
    /// threshold - lane phase, normal, danger - is far above 1%, so the planner
    /// must fire regardless of when in the game this happens.
    #[test]
    fn healing_plan_fires_at_one_percent_hp_in_every_phase() {
        let settings = Settings::default();
        let mut items = empty_items();
        items.slot2 = castable("item_magic_stick");

        for clock_time in [30, 479, 900] {
            for in_danger in [false, true] {
                let mut event = base_event(hero_with_health(1, 1), items.clone());
                event.map.clock_time = clock_time;

                assert_eq!(
                    plan_healing_items(&event, &settings, in_danger),
                    vec![("slot2", "item_magic_stick".to_string())],
                    "clock_time={clock_time} in_danger={in_danger}"
                );
            }
        }
    }

    #[test]
    fn healing_plan_finds_magic_stick_not_just_wand() {
        let settings = Settings::default();
        let mut items = empty_items();
        items.slot0 = castable("item_magic_stick");
        let event = post_lane_event(hero_with_health(20, 20), items);

        assert_eq!(
            plan_healing_items(&event, &settings, false),
            vec![("slot0", "item_magic_stick".to_string())]
        );
    }

    #[test]
    fn healing_plan_takes_cheese_first_and_stops_at_one_item_outside_danger() {
        let settings = Settings::default();
        let mut items = empty_items();
        items.slot0 = castable("item_faerie_fire");
        items.slot1 = castable("item_cheese");
        let event = post_lane_event(hero_with_health(20, 20), items);

        assert_eq!(
            plan_healing_items(&event, &settings, false),
            vec![("slot1", "item_cheese".to_string())]
        );
    }

    #[test]
    fn healing_plan_stacks_up_to_the_danger_limit_in_value_order() {
        let settings = Settings::default();
        let mut items = empty_items();
        items.slot0 = castable("item_faerie_fire");
        items.slot1 = castable("item_cheese");
        items.slot2 = castable("item_enchanted_mango");
        items.slot3 = castable("item_greater_faerie_fire");
        let event = post_lane_event(hero_with_health(20, 20), items);

        assert_eq!(settings.danger_detection.max_healing_items_per_danger, 3);
        assert_eq!(
            plan_healing_items(&event, &settings, true),
            vec![
                ("slot1", "item_cheese".to_string()),
                ("slot3", "item_greater_faerie_fire".to_string()),
                ("slot2", "item_enchanted_mango".to_string()),
            ]
        );
    }

    #[test]
    fn healing_plan_skips_items_that_cannot_be_cast() {
        let settings = Settings::default();
        let mut items = empty_items();
        items.slot0 = Item {
            name: "item_cheese".to_string(),
            can_cast: Some(false),
            ..Default::default()
        };
        // Dota omits can_cast entirely on some payloads; that is not "ready".
        items.slot1 = Item {
            name: "item_faerie_fire".to_string(),
            can_cast: None,
            ..Default::default()
        };
        items.slot2 = castable("item_magic_wand");
        let event = post_lane_event(hero_with_health(20, 20), items);

        assert_eq!(
            plan_healing_items(&event, &settings, false),
            vec![("slot2", "item_magic_wand".to_string())]
        );
    }

    #[test]
    fn healing_plan_reads_the_neutral_slot() {
        let settings = Settings::default();
        let mut items = empty_items();
        items.neutral0 = castable("item_faerie_fire");
        let event = post_lane_event(hero_with_health(20, 20), items);

        assert_eq!(
            plan_healing_items(&event, &settings, false),
            vec![("neutral0", "item_faerie_fire".to_string())]
        );
    }

    #[test]
    fn healing_plan_ignores_backpack_and_stash() {
        let settings = Settings::default();
        let mut items = empty_items();
        items.slot6 = castable("item_cheese");
        items.stash0 = castable("item_faerie_fire");
        let event = post_lane_event(hero_with_health(20, 20), items);

        assert!(plan_healing_items(&event, &settings, false).is_empty());
    }

    #[test]
    fn healing_plan_is_empty_while_dead() {
        let settings = Settings::default();
        let mut items = empty_items();
        items.slot0 = castable("item_cheese");
        let mut hero = hero_with_health(1, 1);
        hero.alive = false;
        let event = base_event(hero, items);

        assert!(plan_healing_items(&event, &settings, false).is_empty());
    }

    /// The lane-phase override is what suppresses healing early, and it beats
    /// the danger threshold while it is active.
    #[test]
    fn healing_plan_is_suppressed_in_lane_phase_above_the_lane_threshold() {
        let settings = Settings::default();
        let mut items = empty_items();
        items.slot0 = castable("item_magic_wand");
        let mut event = base_event(hero_with_health(25, 25), items);
        event.map.clock_time = 300;

        assert!(plan_healing_items(&event, &settings, false).is_empty());
        assert!(plan_healing_items(&event, &settings, true).is_empty());

        event.map.clock_time = 600;
        assert_eq!(
            plan_healing_items(&event, &settings, false),
            vec![("slot0", "item_magic_wand".to_string())]
        );
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
