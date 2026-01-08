//! Coordinate Capture Tool for Bottle Optimization
//!
//! This tool helps capture screen coordinates for inventory and stash slots.
//! Run with: `cargo run --bin capture_coords`
//!
//! Usage:
//! - Move mouse to desired position and left-click to capture
//! - Right-click to save all captured positions to config/screen_positions.toml
//! - Press Escape to exit
//!
//! The tool will show:
//! - All connected monitors with their positions
//! - Real-time mouse position as you move
//! - Captured positions with labels

use std::fs;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rdev::{listen, Event, EventType, Button, Key};

#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};

/// Captured screen position with label
#[derive(Debug, Clone)]
struct CapturedPosition {
    label: String,
    x: i32,
    y: i32,
}

/// Monitor information
#[derive(Debug, Clone)]
struct MonitorInfo {
    name: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    is_primary: bool,
}

/// Get all connected monitors using Windows API
#[cfg(target_os = "windows")]
fn get_monitors() -> Vec<MonitorInfo> {
    use std::cell::RefCell;
    
    // Use thread-local storage to collect monitors
    thread_local! {
        static MONITORS: RefCell<Vec<MonitorInfo>> = RefCell::new(Vec::new());
    }
    
    // Clear any previous data
    MONITORS.with(|m| m.borrow_mut().clear());
    
    unsafe extern "system" fn enum_callback(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        _lparam: LPARAM,
    ) -> BOOL {
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        
        if GetMonitorInfoW(hmonitor, &mut info).as_bool() {
            let rect = info.rcMonitor;
            let is_primary = (info.dwFlags & 1) != 0; // MONITORINFOF_PRIMARY = 1
            
            // Access thread-local storage
            MONITORS.with(|monitors| {
                let mut monitors = monitors.borrow_mut();
                let index = monitors.len();
                monitors.push(MonitorInfo {
                    name: format!("Monitor {}", index + 1),
                    x: rect.left,
                    y: rect.top,
                    width: rect.right - rect.left,
                    height: rect.bottom - rect.top,
                    is_primary,
                });
            });
        }
        
        BOOL(1) // Continue enumeration
    }
    
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(enum_callback),
            LPARAM(0),
        );
    }
    
    // Return collected monitors
    MONITORS.with(|m| m.borrow().clone())
}

#[cfg(not(target_os = "windows"))]
fn get_monitors() -> Vec<MonitorInfo> {
    vec![MonitorInfo {
        name: "Primary".to_string(),
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        is_primary: true,
    }]
}

/// Determine which monitor contains the given coordinates
fn find_monitor_for_point(monitors: &[MonitorInfo], x: i32, y: i32) -> Option<&MonitorInfo> {
    monitors.iter().find(|m| {
        x >= m.x && x < m.x + m.width && y >= m.y && y < m.y + m.height
    })
}

/// Save captured positions to TOML file
fn save_positions(positions: &[CapturedPosition]) -> io::Result<()> {
    // Create config directory if it doesn't exist
    fs::create_dir_all("config")?;
    
    let mut content = String::new();
    content.push_str("# Screen positions captured by capture_coords tool\n");
    content.push_str("# These positions are used for bottle optimization item dragging\n");
    content.push_str("# Re-run the capture tool if you change resolution or monitor setup\n\n");
    
    content.push_str("[screen_layout]\n");
    content.push_str("resolution = \"1920x1080\"\n\n");
    
    // Group positions by category
    let mut inventory: Vec<&CapturedPosition> = Vec::new();
    let mut stash: Vec<&CapturedPosition> = Vec::new();
    let mut other: Vec<&CapturedPosition> = Vec::new();
    
    for pos in positions {
        if pos.label.starts_with("slot") {
            inventory.push(pos);
        } else if pos.label.starts_with("stash") {
            stash.push(pos);
        } else {
            other.push(pos);
        }
    }
    
    // Write inventory positions
    if !inventory.is_empty() {
        content.push_str("[inventory_positions]\n");
        for pos in &inventory {
            content.push_str(&format!("{} = {{ x = {}, y = {} }}\n", pos.label, pos.x, pos.y));
        }
        content.push('\n');
    }
    
    // Write stash positions
    if !stash.is_empty() {
        content.push_str("[stash_positions]\n");
        for pos in &stash {
            content.push_str(&format!("{} = {{ x = {}, y = {} }}\n", pos.label, pos.x, pos.y));
        }
        content.push('\n');
    }
    
    // Write other positions
    if !other.is_empty() {
        content.push_str("[other_positions]\n");
        for pos in &other {
            content.push_str(&format!("{} = {{ x = {}, y = {} }}\n", pos.label, pos.x, pos.y));
        }
        content.push('\n');
    }
    
    fs::write("config/screen_positions.toml", content)?;
    Ok(())
}

/// Predefined labels for quick capture
const SLOT_LABELS: &[&str] = &[
    "slot0", "slot1", "slot2", "slot3", "slot4", "slot5",
    "stash0", "stash1", "stash2", "stash3", "stash4", "stash5",
];

fn main() {
    println!("Starting capture_coords tool...");
    println!();
    
    // Flush to ensure output is visible
    io::stdout().flush().unwrap();
    
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║       Dota 2 Screen Coordinate Capture Tool                ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║  Left-click  : Capture position at cursor                  ║");
    println!("║  Right-click : Save all positions to config file           ║");
    println!("║  Escape      : Exit without saving                         ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    io::stdout().flush().unwrap();
    
    // Get and display monitor information
    println!("Detecting monitors...");
    io::stdout().flush().unwrap();
    let monitors = get_monitors();
    println!("📺 Detected Monitors:");
    for monitor in &monitors {
        let primary = if monitor.is_primary { " (PRIMARY)" } else { "" };
        println!(
            "   {} - Position: ({}, {}), Size: {}x{}{}",
            monitor.name, monitor.x, monitor.y, monitor.width, monitor.height, primary
        );
    }
    println!();
    
    println!("📋 Quick labels available (type number or custom name):");
    for (i, label) in SLOT_LABELS.iter().enumerate() {
        print!("   [{}] {:8}", i, label);
        if (i + 1) % 6 == 0 {
            println!();
        }
    }
    println!("\n");
    
    // Shared state
    let positions: Arc<Mutex<Vec<CapturedPosition>>> = Arc::new(Mutex::new(Vec::new()));
    let current_pos: Arc<Mutex<(f64, f64)>> = Arc::new(Mutex::new((0.0, 0.0)));
    let running = Arc::new(AtomicBool::new(true));
    let awaiting_label = Arc::new(AtomicBool::new(false));
    let pending_pos: Arc<Mutex<Option<(i32, i32)>>> = Arc::new(Mutex::new(None));
    
    let positions_clone = positions.clone();
    let current_pos_clone = current_pos.clone();
    let running_clone = running.clone();
    let awaiting_label_clone = awaiting_label.clone();
    let pending_pos_clone = pending_pos.clone();
    let monitors_clone = monitors.clone();
    
    // Start listener in separate thread
    std::thread::spawn(move || {
        let callback = move |event: Event| {
            match event.event_type {
                EventType::MouseMove { x, y } => {
                    let mut pos = current_pos_clone.lock().unwrap();
                    *pos = (x, y);
                    
                    // Only print position updates if not waiting for label input
                    if !awaiting_label_clone.load(Ordering::SeqCst) {
                        let monitor_name = find_monitor_for_point(&monitors_clone, x as i32, y as i32)
                            .map(|m| m.name.as_str())
                            .unwrap_or("Unknown");
                        print!("\r🖱️  Mouse: ({:5}, {:5}) on {}          ", x as i32, y as i32, monitor_name);
                        io::stdout().flush().unwrap();
                    }
                }
                EventType::ButtonPress(Button::Left) => {
                    if !awaiting_label_clone.load(Ordering::SeqCst) {
                        let pos = current_pos_clone.lock().unwrap();
                        let x = pos.0 as i32;
                        let y = pos.1 as i32;
                        drop(pos);
                        
                        // Store pending position and signal for label input
                        *pending_pos_clone.lock().unwrap() = Some((x, y));
                        awaiting_label_clone.store(true, Ordering::SeqCst);
                        
                        println!("\n\n✅ Captured position: ({}, {})", x, y);
                        print!("   Enter label (number 0-11 for quick label, or custom name): ");
                        io::stdout().flush().unwrap();
                    }
                }
                EventType::ButtonPress(Button::Right) => {
                    if !awaiting_label_clone.load(Ordering::SeqCst) {
                        let positions = positions_clone.lock().unwrap();
                        if positions.is_empty() {
                            println!("\n\n⚠️  No positions captured yet!");
                        } else {
                            println!("\n\n💾 Saving {} positions to config/screen_positions.toml...", positions.len());
                            match save_positions(&positions) {
                                Ok(_) => println!("✅ Saved successfully!"),
                                Err(e) => println!("❌ Failed to save: {}", e),
                            }
                        }
                        println!();
                    }
                }
                EventType::KeyPress(Key::Escape) => {
                    println!("\n\n👋 Exiting...");
                    running_clone.store(false, Ordering::SeqCst);
                }
                _ => {}
            }
        };
        
        if let Err(e) = listen(callback) {
            eprintln!("Error listening for events: {:?}", e);
        }
    });
    
    // Main loop for handling label input
    while running.load(Ordering::SeqCst) {
        if awaiting_label.load(Ordering::SeqCst) {
            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                let input = input.trim();
                
                if let Some((x, y)) = pending_pos.lock().unwrap().take() {
                    // Check if input is a number for quick label
                    let label = if let Ok(num) = input.parse::<usize>() {
                        if num < SLOT_LABELS.len() {
                            SLOT_LABELS[num].to_string()
                        } else {
                            input.to_string()
                        }
                    } else if input.is_empty() {
                        format!("pos_{}", positions.lock().unwrap().len())
                    } else {
                        input.to_string()
                    };
                    
                    positions.lock().unwrap().push(CapturedPosition {
                        label: label.clone(),
                        x,
                        y,
                    });
                    
                    println!("   📍 Saved as '{}' at ({}, {})", label, x, y);
                    
                    // Show captured positions count
                    let count = positions.lock().unwrap().len();
                    println!("   Total captured: {} positions\n", count);
                }
                
                awaiting_label.store(false, Ordering::SeqCst);
            }
        }
        
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    
    // Final save prompt
    let positions = positions.lock().unwrap();
    if !positions.is_empty() {
        println!("\n📋 Captured positions:");
        for pos in positions.iter() {
            println!("   {} = ({}, {})", pos.label, pos.x, pos.y);
        }
        
        print!("\nSave before exit? (y/N): ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            if input.trim().to_lowercase() == "y" {
                match save_positions(&positions) {
                    Ok(_) => println!("✅ Saved to config/screen_positions.toml"),
                    Err(e) => println!("❌ Failed to save: {}", e),
                }
            }
        }
    }
    
    // Keep window open on error/exit
    println!("\nPress Enter to close...");
    io::stdout().flush().unwrap();
    let mut _pause = String::new();
    let _ = io::stdin().read_line(&mut _pause);
}
