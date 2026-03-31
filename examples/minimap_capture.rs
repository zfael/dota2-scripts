//! Standalone minimap capture utility.
//!
//! Usage:
//!   cargo run --example minimap_capture
//!   cargo run --example minimap_capture -- --x 10 --y 815 --width 260 --height 260
//!   cargo run --example minimap_capture -- --output captures/test1

use dota2_scripts::observability::minimap_artifacts::{
    build_artifact_metadata, save_capture_artifact, save_metadata_json,
};
use dota2_scripts::observability::minimap_capture_backend::{
    capture_window_region, find_dota2_window_rect, CaptureBackendResult,
};
use std::env;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn main() {
    let args = Args::parse();

    println!("Minimap Capture Utility");
    println!("  Region: x={}, y={}, {}x{}", args.x, args.y, args.width, args.height);
    println!("  Output: {}", args.output);
    println!();

    // Step 1: Find the Dota 2 window
    print!("Finding Dota 2 window... ");
    match find_dota2_window_rect() {
        CaptureBackendResult::Success { window_rect, .. } => {
            println!("Found ({}x{})", window_rect.width, window_rect.height);
        }
        CaptureBackendResult::WindowNotFound => {
            println!("NOT FOUND");
            eprintln!("Error: Dota 2 window not found. Make sure the game is running.");
            std::process::exit(1);
        }
        CaptureBackendResult::CaptureError(e) => {
            println!("ERROR");
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    // Step 2: Capture the minimap region
    print!("Capturing minimap region... ");
    let start = Instant::now();
    let result = capture_window_region(args.x, args.y, args.width, args.height);
    let duration = start.elapsed();

    match result {
        CaptureBackendResult::Success {
            pixels,
            width,
            height,
            ..
        } => {
            println!("OK ({}ms, {}x{} pixels)", duration.as_millis(), width, height);

            // Step 3: Save artifacts
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let file_stem = format!("minimap_{}", timestamp);

            match save_capture_artifact(&args.output, &file_stem, &pixels, width, height) {
                Ok(path) => println!("Saved PNG:  {}", path),
                Err(e) => {
                    eprintln!("Error saving PNG: {}", e);
                    std::process::exit(1);
                }
            }

            let metadata = build_artifact_metadata(
                timestamp.to_string(),
                "bound".to_string(),
                args.x,
                args.y,
                args.width,
                args.height,
                width,
                height,
                duration.as_millis() as u64,
                "success".to_string(),
                None,
            );

            match save_metadata_json(&args.output, &file_stem, &metadata) {
                Ok(()) => println!(
                    "Saved JSON: {}",
                    format!("{}/{}.json", args.output, file_stem)
                ),
                Err(e) => {
                    eprintln!("Error saving metadata: {}", e);
                    std::process::exit(1);
                }
            }
        }
        CaptureBackendResult::WindowNotFound => {
            println!("FAILED");
            eprintln!("Error: Dota 2 window lost during capture.");
            std::process::exit(1);
        }
        CaptureBackendResult::CaptureError(e) => {
            println!("FAILED");
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

struct Args {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    output: String,
}

impl Args {
    fn parse() -> Self {
        let mut args = Self {
            x: 10,
            y: 815,
            width: 260,
            height: 260,
            output: "logs/minimap_capture".to_string(),
        };

        let raw: Vec<String> = env::args().collect();
        let mut i = 1;
        while i < raw.len() {
            match raw[i].as_str() {
                "--x" => {
                    i += 1;
                    args.x = parse_u32(&raw, i, "--x");
                }
                "--y" => {
                    i += 1;
                    args.y = parse_u32(&raw, i, "--y");
                }
                "--width" => {
                    i += 1;
                    args.width = parse_u32(&raw, i, "--width");
                }
                "--height" => {
                    i += 1;
                    args.height = parse_u32(&raw, i, "--height");
                }
                "--output" => {
                    i += 1;
                    if i >= raw.len() {
                        eprintln!("Error: --output requires a value");
                        std::process::exit(1);
                    }
                    args.output = raw[i].clone();
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => {
                    eprintln!("Unknown argument: {}", other);
                    print_usage();
                    std::process::exit(1);
                }
            }
            i += 1;
        }

        args
    }
}

fn parse_u32(raw: &[String], i: usize, flag: &str) -> u32 {
    if i >= raw.len() {
        eprintln!("Error: {} requires a value", flag);
        std::process::exit(1);
    }
    raw[i].parse().unwrap_or_else(|_| {
        eprintln!("Error: {} value '{}' is not a valid number", flag, raw[i]);
        std::process::exit(1);
    })
}

fn print_usage() {
    println!("Usage: cargo run --example minimap_capture [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --x <N>        Minimap X offset (default: 10)");
    println!("  --y <N>        Minimap Y offset (default: 815)");
    println!("  --width <N>    Minimap width (default: 260)");
    println!("  --height <N>   Minimap height (default: 260)");
    println!("  --output <DIR> Output directory (default: logs/minimap_capture)");
    println!("  --help, -h     Show this help");
}
