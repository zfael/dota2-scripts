use rdev::{listen, Event, EventType, Key};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use tracing::{error, info};

pub enum HotkeyEvent {
    ComboTrigger,
    ShadowFiendQ,
    ShadowFiendW,
    ShadowFiendE,
    LargoQ,
    LargoW,
    LargoE,
    LargoR,
}

pub struct KeyboardListenerConfig {
    pub trigger_key: Arc<Mutex<String>>,
    pub sf_enabled: Arc<Mutex<bool>>,
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
            match ch {
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
                _ => None,
            }
        }
        _ => None,
    }
}

/// Start keyboard listener in a separate thread with dynamic key binding
pub fn start_keyboard_listener(config: KeyboardListenerConfig) -> Receiver<HotkeyEvent> {
    let (event_tx, event_rx) = mpsc::channel::<HotkeyEvent>();

    thread::spawn(move || {
        info!("Starting keyboard listener...");

        let callback = move |event: Event| {
            if let EventType::KeyPress(key) = event.event_type {
                // Check for Shadow Fiend Q/W/E keys (only when SF is selected)
                let sf_enabled = *config.sf_enabled.lock().unwrap();
                if sf_enabled {
                    match key {
                        Key::KeyQ => {
                            info!("Q key pressed - SF raze");
                            let _ = event_tx.send(HotkeyEvent::ShadowFiendQ);
                            return;
                        }
                        Key::KeyW => {
                            info!("W key pressed - SF raze");
                            let _ = event_tx.send(HotkeyEvent::ShadowFiendW);
                            return;
                        }
                        Key::KeyE => {
                            info!("E key pressed - SF raze");
                            let _ = event_tx.send(HotkeyEvent::ShadowFiendE);
                            return;
                        }
                        _ => {}
                    }
                }

                // Check for Largo Q/W/E/R keys (always enabled for Largo)
                match key {
                    Key::KeyQ => {
                        let _ = event_tx.send(HotkeyEvent::LargoQ);
                    }
                    Key::KeyW => {
                        let _ = event_tx.send(HotkeyEvent::LargoW);
                    }
                    Key::KeyE => {
                        let _ = event_tx.send(HotkeyEvent::LargoE);
                    }
                    Key::KeyR => {
                        let _ = event_tx.send(HotkeyEvent::LargoR);
                    }
                    _ => {}
                }
                
                // Check for combo trigger key
                let current_key_str = config.trigger_key.lock().unwrap().clone();
                if let Some(trigger_rdev_key) = parse_key(&current_key_str) {
                    if key == trigger_rdev_key {
                        info!("{} key pressed - triggering combo", current_key_str);
                        let _ = event_tx.send(HotkeyEvent::ComboTrigger);
                    }
                }
            }
        };

        // Start listening
        if let Err(e) = listen(callback) {
            error!("Error in keyboard listener: {:?}", e);
        }
    });

    // Return the event receiver
    event_rx
}

