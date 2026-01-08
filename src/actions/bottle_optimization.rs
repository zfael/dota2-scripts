//! Bottle Optimization Module
//!
//! Automatically moves stat items (Iron Branch) to stash and back before bottle usage
//! to put them on cooldown, reducing max HP/mana for better % healing from bottle.
//!
//! This only triggers when:
//! - Bottle is in inventory and has charges
//! - Game time is before the configured threshold (default: 10 min)
//! - There are target items (branches) in inventory
//! - There are empty stash slots available
//! - Not already in progress (prevents re-triggering mid-sequence)

use crate::config::{Settings, ScreenPosition};
use crate::input::simulation::{
    drag_mouse_with_jitter, get_mouse_position, move_mouse_to, SIMULATING_KEYS,
};
use crate::models::gsi_event::{Items, Hero, Map};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Item slot with its screen position
#[derive(Debug, Clone)]
pub struct ItemSlot {
    pub slot_index: usize,
    pub item_name: String,
    pub screen_pos: ScreenPosition,
}

/// Shared state for bottle optimization
#[derive(Debug)]
pub struct BottleOptimizationState {
    /// Whether bottle is in inventory
    pub bottle_available: bool,
    /// Bottle slot index (0-5)
    pub bottle_slot_index: Option<usize>,
    /// Bottle slot key character
    pub bottle_slot_key: Option<char>,
    /// Whether bottle can be cast (has charges, not on cooldown)
    pub bottle_can_cast: bool,
    /// Bottle charges (0-3)
    pub bottle_charges: u32,
    /// Current game time in seconds
    pub game_time: i32,
    /// Whether hero is alive
    pub hero_alive: bool,
    /// Stat items in inventory that should be swapped
    pub stat_items: Vec<ItemSlot>,
    /// Empty stash slots available for swapping
    pub empty_stash_slots: Vec<ItemSlot>,
    /// Last time bottle optimization was triggered
    pub last_triggered: Option<Instant>,
    /// Whether an optimization sequence is currently in progress
    pub in_progress: AtomicBool,
}

impl Default for BottleOptimizationState {
    fn default() -> Self {
        Self {
            bottle_available: false,
            bottle_slot_index: None,
            bottle_slot_key: None,
            bottle_can_cast: false,
            bottle_charges: 0,
            game_time: 0,
            hero_alive: false,
            stat_items: Vec::new(),
            empty_stash_slots: Vec::new(),
            last_triggered: None,
            in_progress: AtomicBool::new(false),
        }
    }
}

impl BottleOptimizationState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if bottle optimization should trigger
    pub fn should_trigger(&self, settings: &Settings) -> bool {
        // Master toggle
        if !settings.bottle_optimization.enabled {
            return false;
        }

        // Must not be in progress
        if self.in_progress.load(Ordering::SeqCst) {
            debug!("🍾 Bottle opt: already in progress, skipping");
            return false;
        }

        // Hero must be alive
        if !self.hero_alive {
            return false;
        }

        // Bottle must be available with charges
        if !self.bottle_available || !self.bottle_can_cast || self.bottle_charges == 0 {
            return false;
        }

        // Game time must be before threshold
        if self.game_time >= settings.bottle_optimization.max_game_time_seconds {
            debug!(
                "🍾 Bottle opt: game time {}s >= threshold {}s, skipping",
                self.game_time, settings.bottle_optimization.max_game_time_seconds
            );
            return false;
        }

        // Must have stat items to swap
        if self.stat_items.is_empty() {
            debug!("🍾 Bottle opt: no stat items to swap");
            return false;
        }

        // Must have empty stash slots
        if self.empty_stash_slots.is_empty() {
            debug!("🍾 Bottle opt: no empty stash slots");
            return false;
        }

        // Check cooldown lockout
        if let Some(last) = self.last_triggered {
            let elapsed = last.elapsed();
            let cooldown = Duration::from_millis(settings.bottle_optimization.trigger_cooldown_ms);
            if elapsed < cooldown {
                debug!(
                    "🍾 Bottle opt: cooldown ({:?} < {:?}), skipping",
                    elapsed, cooldown
                );
                return false;
            }
        }

        true
    }

    /// Check if a key press should be intercepted for bottle optimization
    pub fn should_intercept_key(&self, key_char: char, _settings: &Settings) -> bool {
        if let Some(bottle_key) = self.bottle_slot_key {
            // Only intercept the bottle slot key
            key_char.to_ascii_lowercase() == bottle_key.to_ascii_lowercase()
        } else {
            false
        }
    }

    /// Mark as triggered
    pub fn mark_triggered(&mut self) {
        self.last_triggered = Some(Instant::now());
    }
}

/// Global state for bottle optimization
pub static BOTTLE_OPT_STATE: std::sync::LazyLock<Arc<Mutex<BottleOptimizationState>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(BottleOptimizationState::new())));

/// Update bottle optimization state from GSI event
pub fn update_from_gsi(items: &Items, hero: &Hero, map: &Map, settings: &Settings) {
    let mut state = BOTTLE_OPT_STATE.lock().unwrap();
    
    // Update hero state
    state.hero_alive = hero.alive;
    state.game_time = map.clock_time;
    
    // Clear previous item tracking
    state.bottle_available = false;
    state.bottle_slot_index = None;
    state.bottle_slot_key = None;
    state.bottle_can_cast = false;
    state.bottle_charges = 0;
    state.stat_items.clear();
    state.empty_stash_slots.clear();
    
    // Scan inventory slots for bottle and target items
    let inventory_slots = [
        ("slot0", &items.slot0, 0),
        ("slot1", &items.slot1, 1),
        ("slot2", &items.slot2, 2),
        ("slot3", &items.slot3, 3),
        ("slot4", &items.slot4, 4),
        ("slot5", &items.slot5, 5),
    ];
    
    for (slot_name, item, index) in &inventory_slots {
        // Check for bottle
        if item.name == "item_bottle" {
            state.bottle_available = true;
            state.bottle_slot_index = Some(*index);
            state.bottle_slot_key = settings.get_key_for_slot(slot_name);
            state.bottle_can_cast = item.can_cast.unwrap_or(false);
            state.bottle_charges = item.charges.unwrap_or(0);
            debug!(
                "🍾 Found bottle in {} (key={:?}, can_cast={}, charges={})",
                slot_name, state.bottle_slot_key, state.bottle_can_cast, state.bottle_charges
            );
        }
        
        // Check for target items (Iron Branch, etc.)
        if settings.bottle_optimization.target_items.contains(&item.name) {
            if let Some(pos) = settings.screen_positions.inventory_positions.get_slot(*index) {
                state.stat_items.push(ItemSlot {
                    slot_index: *index,
                    item_name: item.name.clone(),
                    screen_pos: pos.clone(),
                });
                debug!("🍾 Found target item '{}' in slot{}", item.name, index);
            }
        }
    }
    
    // Scan backpack for empty slots (slot6-8 are the 3 backpack slots below inventory)
    let backpack_slots = [
        (&items.slot6, 0),  // backpack slot 0
        (&items.slot7, 1),  // backpack slot 1
        (&items.slot8, 2),  // backpack slot 2
    ];
    
    for (item, index) in &backpack_slots {
        if item.name == "empty" {
            if let Some(pos) = settings.screen_positions.stash_positions.get_slot(*index) {
                state.empty_stash_slots.push(ItemSlot {
                    slot_index: *index,
                    item_name: "empty".to_string(),
                    screen_pos: pos.clone(),
                });
                debug!("🍾 Found empty backpack slot: backpack{}", index);
            }
        }
    }
}

/// Execute the bottle optimization sequence
/// Moves stat items to stash and back (in batches), then uses bottle
pub fn execute_bottle_optimization(bottle_key: char, settings: Settings) {
    use crate::input::keyboard::{char_to_key, simulate_key};
    
    let mut state = BOTTLE_OPT_STATE.lock().unwrap();
    
    // Set in_progress flag
    state.in_progress.store(true, Ordering::SeqCst);
    state.mark_triggered();
    
    // Gather info we need before dropping lock
    let stat_items = state.stat_items.clone();
    let empty_stash_slots = state.empty_stash_slots.clone();
    let jitter = settings.bottle_optimization.mouse_jitter_px;
    let delay = settings.bottle_optimization.delay_between_drags_ms;
    let restore_mouse = settings.bottle_optimization.restore_mouse_position;
    
    drop(state); // Release lock before doing mouse operations
    
    // Save original mouse position
    let original_mouse_pos = if restore_mouse {
        Some(get_mouse_position())
    } else {
        None
    };
    
    info!(
        "🍾 Starting bottle optimization: {} stat items, {} empty stash slots",
        stat_items.len(),
        empty_stash_slots.len()
    );
    
    // Calculate batch size (min of stat items and empty stash slots)
    let batch_size = std::cmp::min(stat_items.len(), empty_stash_slots.len());
    
    if batch_size == 0 {
        warn!("🍾 No items to swap or no stash space");
        let state = BOTTLE_OPT_STATE.lock().unwrap();
        state.in_progress.store(false, Ordering::SeqCst);
        return;
    }
    
    // Process items in batches
    let mut items_remaining: Vec<ItemSlot> = stat_items;
    
    while !items_remaining.is_empty() {
        // Calculate current batch
        let current_batch_size = std::cmp::min(items_remaining.len(), empty_stash_slots.len());
        let batch: Vec<ItemSlot> = items_remaining.drain(..current_batch_size).collect();
        
        info!("🍾 Processing batch of {} items", batch.len());
        
        // Step 1: Drag items from inventory to stash
        for (i, item) in batch.iter().enumerate() {
            let stash_slot = &empty_stash_slots[i];
            debug!(
                "🍾 Dragging {} from slot{} ({},{}) to stash{} ({},{})",
                item.item_name, item.slot_index, item.screen_pos.x, item.screen_pos.y,
                stash_slot.slot_index, stash_slot.screen_pos.x, stash_slot.screen_pos.y
            );
            
            drag_mouse_with_jitter(
                item.screen_pos.x,
                item.screen_pos.y,
                stash_slot.screen_pos.x,
                stash_slot.screen_pos.y,
                jitter,
                delay,
            );
            
            // Small delay between items
            std::thread::sleep(Duration::from_millis(delay / 2));
        }
        
        // Small delay before dragging back
        std::thread::sleep(Duration::from_millis(delay));
        
        // Step 2: Drag items back from stash to inventory (puts them on cooldown)
        for (i, item) in batch.iter().enumerate() {
            let stash_slot = &empty_stash_slots[i];
            debug!(
                "🍾 Dragging back from stash{} ({},{}) to slot{} ({},{})",
                stash_slot.slot_index, stash_slot.screen_pos.x, stash_slot.screen_pos.y,
                item.slot_index, item.screen_pos.x, item.screen_pos.y
            );
            
            drag_mouse_with_jitter(
                stash_slot.screen_pos.x,
                stash_slot.screen_pos.y,
                item.screen_pos.x,
                item.screen_pos.y,
                jitter,
                delay,
            );
            
            // Small delay between items
            std::thread::sleep(Duration::from_millis(delay / 2));
        }
        
        // Delay before next batch or final bottle use
        std::thread::sleep(Duration::from_millis(delay));
    }
    
    // Step 3: Use bottle
    info!("🍾 Using bottle (key: {})", bottle_key);
    SIMULATING_KEYS.store(true, Ordering::SeqCst);
    if let Some(rdev_key) = char_to_key(bottle_key) {
        simulate_key(rdev_key);
    }
    std::thread::sleep(Duration::from_millis(10));
    SIMULATING_KEYS.store(false, Ordering::SeqCst);
    
    // Restore mouse position if configured
    if let Some((x, y)) = original_mouse_pos {
        std::thread::sleep(Duration::from_millis(delay));
        move_mouse_to(x, y);
        debug!("🍾 Restored mouse position to ({}, {})", x, y);
    }
    
    // Clear in_progress flag
    let state = BOTTLE_OPT_STATE.lock().unwrap();
    state.in_progress.store(false, Ordering::SeqCst);
    
    info!("🍾 Bottle optimization complete!");
}
