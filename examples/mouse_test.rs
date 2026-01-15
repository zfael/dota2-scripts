//! Quick test to see mouse button codes
//! Run with: cargo run --example mouse_test

use rdev::{listen, Event, EventType, Button};

fn main() {
    println!("Press any mouse button to see its code. Press Ctrl+C to exit.");
    
    if let Err(error) = listen(move |event: Event| {
        match event.event_type {
            EventType::ButtonPress(button) => {
                println!("Button pressed: {:?}", button);
            }
            EventType::ButtonRelease(button) => {
                println!("Button released: {:?}", button);
            }
            _ => {}
        }
    }) {
        eprintln!("Error: {:?}", error);
    }
}
