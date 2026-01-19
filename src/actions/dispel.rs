//! Auto-dispel module
//!
//! Automatically uses dispel items (Manta Style, Lotus Orb) when silenced.
//! Triggers immediately with random jitter for human-like reaction.

use crate::config::Settings;
use crate::models::GsiWebhookEvent;
use lazy_static::lazy_static;
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tracing::info;

lazy_static! {
    /// Track if we already triggered dispel this silence (avoid spam)
    static ref DISPEL_TRIGGERED: AtomicBool = AtomicBool::new(false);
}

/// Check and use dispel items (Manta/Lotus) if silenced (called every GSI event)
pub fn check_and_dispel_silence(event: &GsiWebhookEvent, settings: &Settings) {
    // Reset trigger flag when not silenced
    if !event.hero.silenced {
        DISPEL_TRIGGERED.store(false, Ordering::SeqCst);
        return;
    }

    // Already triggered this silence
    if DISPEL_TRIGGERED.load(Ordering::SeqCst) {
        return;
    }

    if !event.hero.alive {
        return;
    }

    let manta_enabled = settings.danger_detection.auto_manta_on_silence;
    let lotus_enabled = settings.danger_detection.auto_lotus_on_silence;

    if !manta_enabled && !lotus_enabled {
        return;
    }

    // Find Manta Style or Lotus Orb (prefer Manta as it's instant)
    for (slot, item) in event.items.all_slots() {
        // Check Manta Style first (instant dispel)
        if manta_enabled && item.name == "item_manta" {
            if item.can_cast.unwrap_or(false) && item.cooldown.unwrap_or(0) == 0 {
                DISPEL_TRIGGERED.store(true, Ordering::SeqCst);

                let key = settings.get_key_for_slot(slot);
                if let Some(key) = key {
                    thread::spawn(move || {
                        let jitter = rand::rng().random_range(30..100);
                        thread::sleep(Duration::from_millis(jitter));
                        
                        info!("🌀 Using Manta Style (silenced, jitter {}ms)", jitter);
                        crate::input::simulation::press_key(key);
                    });
                }
                return;
            }
        }
        
        // Check Lotus Orb (self-cast with double-tap)
        if lotus_enabled && item.name == "item_lotus_orb" {
            if item.can_cast.unwrap_or(false) && item.cooldown.unwrap_or(0) == 0 {
                DISPEL_TRIGGERED.store(true, Ordering::SeqCst);

                let key = settings.get_key_for_slot(slot);
                if let Some(key) = key {
                    thread::spawn(move || {
                        let jitter = rand::rng().random_range(30..100);
                        thread::sleep(Duration::from_millis(jitter));
                        
                        info!("🪷 Using Lotus Orb (silenced, jitter {}ms)", jitter);
                        // Double-tap for self-cast
                        crate::input::simulation::press_key(key);
                        thread::sleep(Duration::from_millis(30));
                        crate::input::simulation::press_key(key);
                    });
                }
                return;
            }
        }
    }
}
