use enigo::{Enigo, Key, Keyboard, Mouse, Button, Direction, Settings};
use lazy_static::lazy_static;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tracing::warn;

lazy_static! {
    static ref ENIGO: Mutex<Enigo> = {
        let enigo = Enigo::new(&Settings::default())
            .expect("Failed to initialize Enigo");
        Mutex::new(enigo)
    };
}

/// Global flag to indicate we're simulating keys - prevents keyboard grab re-interception
pub static SIMULATING_KEYS: AtomicBool = AtomicBool::new(false);

/// Press a single key (sets SIMULATING_KEYS flag to prevent re-interception)
pub fn press_key(key_char: char) {
    // Set flag before simulating
    SIMULATING_KEYS.store(true, Ordering::SeqCst);
    
    let mut enigo = ENIGO.lock().unwrap();
    if let Err(e) = enigo.key(Key::Unicode(key_char), enigo::Direction::Click) {
        warn!("Failed to press key '{}': {}", key_char, e);
    }
    drop(enigo);
    
    // Small delay then clear flag
    std::thread::sleep(std::time::Duration::from_millis(10));
    SIMULATING_KEYS.store(false, Ordering::SeqCst);
}

/// Press a key down (hold)
#[allow(dead_code)]
pub fn key_down(key_char: char) {
    let mut enigo = ENIGO.lock().unwrap();
    if let Err(e) = enigo.key(Key::Unicode(key_char), enigo::Direction::Press) {
        warn!("Failed to press down key '{}': {}", key_char, e);
    }
}

/// Release a key
#[allow(dead_code)]
pub fn key_up(key_char: char) {
    let mut enigo = ENIGO.lock().unwrap();
    if let Err(e) = enigo.key(Key::Unicode(key_char), enigo::Direction::Release) {
        warn!("Failed to release key '{}': {}", key_char, e);
    }
}

/// Perform a right mouse click
pub fn mouse_click() {
    let mut enigo = ENIGO.lock().unwrap();
    if let Err(e) = enigo.button(Button::Right, Direction::Click) {
        warn!("Failed to perform right click: {}", e);
    }
}

/// Hold ALT key down
pub fn alt_down() {
    SIMULATING_KEYS.store(true, Ordering::SeqCst);
    let mut enigo = ENIGO.lock().unwrap();
    if let Err(e) = enigo.key(Key::Alt, enigo::Direction::Press) {
        warn!("Failed to press ALT down: {}", e);
    }
}

/// Release ALT key
pub fn alt_up() {
    let mut enigo = ENIGO.lock().unwrap();
    if let Err(e) = enigo.key(Key::Alt, enigo::Direction::Release) {
        warn!("Failed to release ALT: {}", e);
    }
    drop(enigo);
    std::thread::sleep(std::time::Duration::from_millis(10));
    SIMULATING_KEYS.store(false, Ordering::SeqCst);
}
