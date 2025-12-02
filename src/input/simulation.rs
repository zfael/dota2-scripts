use enigo::{Enigo, Key, Keyboard, Mouse, Button, Direction, Settings};
use lazy_static::lazy_static;
use std::sync::Mutex;
use tracing::warn;

lazy_static! {
    static ref ENIGO: Mutex<Enigo> = {
        let enigo = Enigo::new(&Settings::default())
            .expect("Failed to initialize Enigo");
        Mutex::new(enigo)
    };
}

/// Press a single key
pub fn press_key(key_char: char) {
    let mut enigo = ENIGO.lock().unwrap();
    if let Err(e) = enigo.key(Key::Unicode(key_char), enigo::Direction::Click) {
        warn!("Failed to press key '{}': {}", key_char, e);
    }
}

/// Press a key down (hold)
pub fn key_down(key_char: char) {
    let mut enigo = ENIGO.lock().unwrap();
    if let Err(e) = enigo.key(Key::Unicode(key_char), enigo::Direction::Press) {
        warn!("Failed to press down key '{}': {}", key_char, e);
    }
}

/// Release a key
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
