use rdev::{grab, simulate, Button, Event, EventType, Key};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::actions::heroes::broodmother::BROODMOTHER_ACTIVE;
use crate::actions::heroes::shadow_fiend::ShadowFiendState;
use crate::actions::SOUL_RING_STATE;
use crate::config::Settings;
use crate::input::simulation::SIMULATING_KEYS;

pub enum HotkeyEvent {
    ComboTrigger,
    LargoQ,
    LargoW,
    LargoE,
    LargoR,
    BroodmotherSpiderAttack,
}

pub struct KeyboardListenerConfig {
    pub trigger_key: Arc<Mutex<String>>,
    pub sf_enabled: Arc<Mutex<bool>>,
    pub settings: Arc<Mutex<Settings>>,
}

/// Parse key string to rdev::Key (public version)
pub fn parse_key_string(key_str: &str) -> Option<Key> {
    match key_str.to_lowercase().as_str() {
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "insert" => Some(Key::Insert),
        "delete" => Some(Key::Delete),
        "pageup" => Some(Key::PageUp),
        "pagedown" => Some(Key::PageDown),
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        // Single char keys
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            char_to_key(ch)
        }
        _ => None,
    }
}

/// Parse key string to rdev::Key
fn parse_key(key_str: &str) -> Option<Key> {
    match key_str.to_lowercase().as_str() {
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "insert" => Some(Key::Insert),
        "delete" => Some(Key::Delete),
        "pageup" => Some(Key::PageUp),
        "pagedown" => Some(Key::PageDown),
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        // Single char keys
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            char_to_key(ch)
        }
        _ => None,
    }
}

/// Convert a char to rdev::Key
pub fn char_to_key(ch: char) -> Option<Key> {
    match ch.to_ascii_lowercase() {
        'a' => Some(Key::KeyA),
        'b' => Some(Key::KeyB),
        'c' => Some(Key::KeyC),
        'd' => Some(Key::KeyD),
        'e' => Some(Key::KeyE),
        'f' => Some(Key::KeyF),
        'g' => Some(Key::KeyG),
        'h' => Some(Key::KeyH),
        'i' => Some(Key::KeyI),
        'j' => Some(Key::KeyJ),
        'k' => Some(Key::KeyK),
        'l' => Some(Key::KeyL),
        'm' => Some(Key::KeyM),
        'n' => Some(Key::KeyN),
        'o' => Some(Key::KeyO),
        'p' => Some(Key::KeyP),
        'q' => Some(Key::KeyQ),
        'r' => Some(Key::KeyR),
        's' => Some(Key::KeyS),
        't' => Some(Key::KeyT),
        'u' => Some(Key::KeyU),
        'v' => Some(Key::KeyV),
        'w' => Some(Key::KeyW),
        'x' => Some(Key::KeyX),
        'y' => Some(Key::KeyY),
        'z' => Some(Key::KeyZ),
        '0' => Some(Key::Num0),
        '1' => Some(Key::Num1),
        '2' => Some(Key::Num2),
        '3' => Some(Key::Num3),
        '4' => Some(Key::Num4),
        '5' => Some(Key::Num5),
        '6' => Some(Key::Num6),
        '7' => Some(Key::Num7),
        '8' => Some(Key::Num8),
        '9' => Some(Key::Num9),
        _ => None,
    }
}

/// Convert rdev::Key to char (for keys we care about)
fn key_to_char(key: Key) -> Option<char> {
    match key {
        Key::KeyA => Some('a'),
        Key::KeyB => Some('b'),
        Key::KeyC => Some('c'),
        Key::KeyD => Some('d'),
        Key::KeyE => Some('e'),
        Key::KeyF => Some('f'),
        Key::KeyG => Some('g'),
        Key::KeyH => Some('h'),
        Key::KeyI => Some('i'),
        Key::KeyJ => Some('j'),
        Key::KeyK => Some('k'),
        Key::KeyL => Some('l'),
        Key::KeyM => Some('m'),
        Key::KeyN => Some('n'),
        Key::KeyO => Some('o'),
        Key::KeyP => Some('p'),
        Key::KeyQ => Some('q'),
        Key::KeyR => Some('r'),
        Key::KeyS => Some('s'),
        Key::KeyT => Some('t'),
        Key::KeyU => Some('u'),
        Key::KeyV => Some('v'),
        Key::KeyW => Some('w'),
        Key::KeyX => Some('x'),
        Key::KeyY => Some('y'),
        Key::KeyZ => Some('z'),
        Key::Num0 => Some('0'),
        Key::Num1 => Some('1'),
        Key::Num2 => Some('2'),
        Key::Num3 => Some('3'),
        Key::Num4 => Some('4'),
        Key::Num5 => Some('5'),
        Key::Num6 => Some('6'),
        Key::Num7 => Some('7'),
        Key::Num8 => Some('8'),
        Key::Num9 => Some('9'),
        _ => None,
    }
}

/// Simulate a key press using rdev (must be called from a non-grab thread)
/// Sets SIMULATING_KEYS flag to prevent re-interception
pub fn simulate_key(key: Key) {
    SIMULATING_KEYS.store(true, Ordering::SeqCst);
    
    if let Err(e) = simulate(&EventType::KeyPress(key)) {
        warn!("Failed to simulate key press: {:?}", e);
    }
    thread::sleep(Duration::from_millis(5));
    if let Err(e) = simulate(&EventType::KeyRelease(key)) {
        warn!("Failed to simulate key release: {:?}", e);
    }
    
    thread::sleep(Duration::from_millis(5));
    SIMULATING_KEYS.store(false, Ordering::SeqCst);
}

/// Spawn Soul Ring trigger + ability key simulation in a separate thread
/// This is necessary because grab() callback must return quickly
fn spawn_soul_ring_then_key(original_key: Key, settings: Settings) {
    thread::spawn(move || {
        let mut soul_ring_state = SOUL_RING_STATE.lock().unwrap();
        
        if soul_ring_state.should_trigger(&settings) {
            if let Some(sr_key) = soul_ring_state.slot_key {
                // Mark as triggered to start cooldown lockout
                soul_ring_state.mark_triggered();
                drop(soul_ring_state); // Release lock before sleeping
                
                // Simulate Soul Ring key press
                if let Some(sr_rdev_key) = char_to_key(sr_key) {
                    debug!("💍 Pressing Soul Ring key: {}", sr_key);
                    simulate_key(sr_rdev_key);
                    
                    // Wait configured delay before ability
                    let delay = settings.soul_ring.delay_before_ability_ms;
                    thread::sleep(Duration::from_millis(delay));
                }
            } else {
                drop(soul_ring_state);
            }
        } else {
            drop(soul_ring_state);
        }
        
        // Simulate the original ability/item key
        simulate_key(original_key);
    });
}

/// Start keyboard listener in a separate thread with key interception (grab)
/// This intercepts keys and can block/modify them before they reach the game
pub fn start_keyboard_listener(config: KeyboardListenerConfig) -> Receiver<HotkeyEvent> {
    let (event_tx, event_rx) = mpsc::channel::<HotkeyEvent>();

    thread::spawn(move || {
        info!("Starting keyboard listener with key interception (grab)...");

        let callback = move |event: Event| -> Option<Event> {
            // Pass through all events while we're simulating keys
            // This prevents re-interception of our own simulated keypresses
            if SIMULATING_KEYS.load(Ordering::SeqCst) {
                return Some(event);
            }
            
            // Handle Mouse5 (Button::Unknown(5)) for Broodmother spider micro
            if let EventType::ButtonPress(button) = event.event_type {
                // Mouse5 is typically Button::Unknown(5) on macOS
                let is_mouse5 = matches!(button, Button::Unknown(5) | Button::Unknown(4));
                if is_mouse5 && BROODMOTHER_ACTIVE.load(Ordering::SeqCst) {
                    let settings = config.settings.lock().unwrap().clone();
                    if settings.heroes.broodmother.spider_micro_enabled {
                        info!("🕷️ Mouse5 pressed - Broodmother spider attack-move");
                        let _ = event_tx.send(HotkeyEvent::BroodmotherSpiderAttack);
                        // Spawn in thread to not block the grab callback
                        thread::spawn(move || {
                            crate::actions::heroes::broodmother::BroodmotherScript::execute_spider_attack_move(&settings);
                        });
                        return None; // Block the original mouse button
                    }
                }
            }
            
            if let EventType::KeyPress(key) = event.event_type {
                let settings = config.settings.lock().unwrap().clone();
                let sf_enabled = *config.sf_enabled.lock().unwrap();
                
                // Convert key to char to check if we should intercept
                let key_char = key_to_char(key);
                
                // Check if this is a key we should intercept for Soul Ring
                let should_intercept_for_soul_ring = if let Some(ch) = key_char {
                    let soul_ring_state = SOUL_RING_STATE.lock().unwrap();
                    let should_intercept = soul_ring_state.should_intercept_key(ch, &settings);
                    let should_trigger = soul_ring_state.should_trigger(&settings);
                    debug!(
                        "💍 Key '{}': intercept={}, trigger={}, available={}, can_cast={}, mana={}%, health={}%",
                        ch, should_intercept, should_trigger,
                        soul_ring_state.available, soul_ring_state.can_cast,
                        soul_ring_state.hero_mana_percent, soul_ring_state.hero_health_percent
                    );
                    should_intercept && should_trigger
                } else {
                    false
                };

                // Handle Shadow Fiend Q/W/E keys (when SF is selected AND raze interception is enabled in config)
                let sf_raze_enabled_in_config = settings.heroes.shadow_fiend.raze_intercept_enabled;
                let sf_raze_active = sf_enabled && sf_raze_enabled_in_config;
                if sf_raze_active {
                    match key {
                        Key::KeyQ | Key::KeyW | Key::KeyE => {
                            let raze_key = key_to_char(key).unwrap();
                            info!("{} key pressed - SF raze", raze_key.to_ascii_uppercase());
                            
                            // Delegate to ShadowFiendState for raze execution
                            ShadowFiendState::execute_raze(raze_key, &settings);
                            
                            // Block original key
                            return None;
                        }
                        _ => {}
                    }
                }

                // Handle Largo Q/W/E/R keys and other ability keys with Soul Ring
                match key {
                    Key::KeyQ | Key::KeyW | Key::KeyE | Key::KeyR | Key::KeyD | Key::KeyF => {
                        // Send Largo events for beat timing
                        match key {
                            Key::KeyQ => { let _ = event_tx.send(HotkeyEvent::LargoQ); }
                            Key::KeyW => { let _ = event_tx.send(HotkeyEvent::LargoW); }
                            Key::KeyE => { let _ = event_tx.send(HotkeyEvent::LargoE); }
                            Key::KeyR => { let _ = event_tx.send(HotkeyEvent::LargoR); }
                            _ => {}
                        }
                        
                        // If Soul Ring should trigger, spawn handler and block original
                        if should_intercept_for_soul_ring {
                            spawn_soul_ring_then_key(key, settings.clone());
                            return None; // Block original
                        }
                        
                        // Pass through if not intercepting for Soul Ring
                        return Some(event);
                    }
                    _ => {}
                }
                
                // Check for item slot keys (for Soul Ring triggering)
                if let Some(ch) = key_char {
                    let soul_ring_state = SOUL_RING_STATE.lock().unwrap();
                    if soul_ring_state.is_item_key(ch, &settings) && soul_ring_state.should_trigger(&settings) {
                        drop(soul_ring_state);
                        spawn_soul_ring_then_key(key, settings.clone());
                        return None; // Block original
                    }
                }
                
                // Check for combo trigger key
                let current_key_str = config.trigger_key.lock().unwrap().clone();
                if let Some(trigger_rdev_key) = parse_key(&current_key_str) {
                    if key == trigger_rdev_key {
                        info!("{} key pressed - triggering combo", current_key_str);
                        let _ = event_tx.send(HotkeyEvent::ComboTrigger);
                        // Pass through - combo trigger doesn't need to be blocked
                    }
                }
            }
            
            // Pass through all other events (key releases, mouse events, etc.)
            Some(event)
        };

        // Start grabbing - this blocks forever
        if let Err(e) = grab(callback) {
            error!("Error in keyboard grab listener: {:?}", e);
        }
    });

    // Return the event receiver
    event_rx
}

