use enigo::{Enigo, Key, Keyboard, Mouse, Button, Direction, Settings, Coordinate};
use lazy_static::lazy_static;
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
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

// ============================================================================
// Mouse Simulation Functions (for bottle optimization item dragging)
// ============================================================================

/// Global flag to indicate we're simulating mouse actions
pub static SIMULATING_MOUSE: AtomicBool = AtomicBool::new(false);

/// Get current mouse cursor position
pub fn get_mouse_position() -> (i32, i32) {
    let enigo = ENIGO.lock().unwrap();
    match enigo.location() {
        Ok((x, y)) => (x, y),
        Err(e) => {
            warn!("Failed to get mouse position: {}", e);
            (0, 0)
        }
    }
}

/// Move mouse to absolute screen coordinates
pub fn move_mouse_to(x: i32, y: i32) {
    let mut enigo = ENIGO.lock().unwrap();
    if let Err(e) = enigo.move_mouse(x, y, Coordinate::Abs) {
        warn!("Failed to move mouse to ({}, {}): {}", x, y, e);
    }
}

/// Move mouse to position with jitter for humanization
/// Adds random offset of ±jitter_px pixels
#[allow(dead_code)]
pub fn move_mouse_to_with_jitter(x: i32, y: i32, jitter_px: i32) {
    let mut rng = rand::thread_rng();
    let jitter_x = rng.gen_range(-jitter_px..=jitter_px);
    let jitter_y = rng.gen_range(-jitter_px..=jitter_px);
    move_mouse_to(x + jitter_x, y + jitter_y);
}

/// Press left mouse button down (hold)
pub fn left_mouse_down() {
    let mut enigo = ENIGO.lock().unwrap();
    if let Err(e) = enigo.button(Button::Left, Direction::Press) {
        warn!("Failed to press left mouse button: {}", e);
    }
}

/// Release left mouse button
pub fn left_mouse_up() {
    let mut enigo = ENIGO.lock().unwrap();
    if let Err(e) = enigo.button(Button::Left, Direction::Release) {
        warn!("Failed to release left mouse button: {}", e);
    }
}

/// Perform a left mouse click
#[allow(dead_code)]
pub fn left_mouse_click() {
    let mut enigo = ENIGO.lock().unwrap();
    if let Err(e) = enigo.button(Button::Left, Direction::Click) {
        warn!("Failed to perform left click: {}", e);
    }
}

/// Drag mouse from source to destination with humanized movement
/// Includes jitter and random delays for more natural movement
pub fn drag_mouse_with_jitter(
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    jitter_px: i32,
    base_delay_ms: u64,
) {
    let mut rng = rand::thread_rng();
    
    // Set simulating flag
    SIMULATING_MOUSE.store(true, Ordering::SeqCst);
    
    // Move to source with jitter
    let src_jitter_x = rng.gen_range(-jitter_px..=jitter_px);
    let src_jitter_y = rng.gen_range(-jitter_px..=jitter_px);
    move_mouse_to(from_x + src_jitter_x, from_y + src_jitter_y);
    
    // Random delay before clicking
    let delay1 = base_delay_ms + rng.gen_range(0..30);
    std::thread::sleep(Duration::from_millis(delay1));
    
    // Press and hold left mouse button
    left_mouse_down();
    
    // Random delay while holding
    let delay2 = base_delay_ms + rng.gen_range(0..30);
    std::thread::sleep(Duration::from_millis(delay2));
    
    // Move to destination with jitter
    let dst_jitter_x = rng.gen_range(-jitter_px..=jitter_px);
    let dst_jitter_y = rng.gen_range(-jitter_px..=jitter_px);
    move_mouse_to(to_x + dst_jitter_x, to_y + dst_jitter_y);
    
    // Random delay before releasing
    let delay3 = base_delay_ms + rng.gen_range(0..30);
    std::thread::sleep(Duration::from_millis(delay3));
    
    // Release mouse button
    left_mouse_up();
    
    // Clear simulating flag
    SIMULATING_MOUSE.store(false, Ordering::SeqCst);
}

/// Drag mouse without jitter (for precise operations)
#[allow(dead_code)]
pub fn drag_mouse(from_x: i32, from_y: i32, to_x: i32, to_y: i32, delay_ms: u64) {
    SIMULATING_MOUSE.store(true, Ordering::SeqCst);
    
    move_mouse_to(from_x, from_y);
    std::thread::sleep(Duration::from_millis(delay_ms));
    
    left_mouse_down();
    std::thread::sleep(Duration::from_millis(delay_ms));
    
    move_mouse_to(to_x, to_y);
    std::thread::sleep(Duration::from_millis(delay_ms));
    
    left_mouse_up();
    
    SIMULATING_MOUSE.store(false, Ordering::SeqCst);
}
