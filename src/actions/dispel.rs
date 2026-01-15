//! Auto-dispel module
//!
//! Automatically uses dispel items (Manta Style) when silenced.
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

/// Check and use Manta if silenced (called every GSI event)
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

    if !settings.danger_detection.auto_manta_on_silence {
        return;
    }

    // Find Manta Style
    for (slot, item) in event.items.all_slots() {
        if item.name == "item_manta" {
            if item.can_cast.unwrap_or(false) && item.cooldown.unwrap_or(0) == 0 {
                // Mark as triggered before spawning thread
                DISPEL_TRIGGERED.store(true, Ordering::SeqCst);

                let key = settings.get_key_for_slot(slot);
                if let Some(key) = key {
                    // Spawn thread with random jitter
                    thread::spawn(move || {
                        // Random jitter 30-100ms
                        let jitter = rand::rng().random_range(30..100);
                        thread::sleep(Duration::from_millis(jitter));
                        
                        info!("🌀 Using Manta Style (silenced, jitter {}ms)", jitter);
                        crate::input::simulation::press_key(key);
                    });
                }
                return;
            }
        }
    }
}
