//! Standalone minimap analysis utility.
//!
//! Loads PNG captures from a directory, optionally builds a baseline mask,
//! then runs hero detection on each frame and prints results.
//!
//! Usage:
//!   cargo run --example minimap_analyze -- --dir logs/minimap_capture
//!   cargo run --example minimap_analyze -- --dir logs/minimap_capture --baseline-frames 5
//!   cargo run --example minimap_analyze -- --dir logs/minimap_capture --min-cluster 10 --max-cluster 150
//!   cargo run --example minimap_analyze -- --dir logs/minimap_capture --team dire --window 5

use dota2_scripts::observability::lane_heat::{
    classify_zone_activity, ActivityLevel, LaneHeatTracker, TeamSide,
};
use dota2_scripts::observability::minimap_analysis::{
    build_color_masks, detect_heroes, ColorThresholds, TeamColor,
};
use dota2_scripts::observability::minimap_baseline::BaselineMask;
use std::env;
use std::time::Instant;

fn main() {
    let args = Args::parse();

    let team_side = TeamSide::from_team_name(&args.team);
    let mut tracker = LaneHeatTracker::new(args.window);

    println!("Minimap Analysis Utility");
    println!("  Directory: {}", args.dir);
    println!("  Baseline frames: {}", args.baseline_frames);
    println!(
        "  Cluster size: {}-{}",
        args.min_cluster, args.max_cluster
    );
    println!("  Team: {} (ally={}, enemy={})", args.team, team_side.ally_color, team_side.enemy_color);
    println!("  Rolling window: {} frames", args.window);
    println!();

    // Collect PNG files sorted by name
    let mut png_files: Vec<_> = std::fs::read_dir(&args.dir)
        .unwrap_or_else(|e| {
            eprintln!("Error reading directory '{}': {}", args.dir, e);
            std::process::exit(1);
        })
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("png") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    png_files.sort();

    if png_files.is_empty() {
        eprintln!("No PNG files found in '{}'", args.dir);
        std::process::exit(1);
    }
    println!("Found {} PNG files", png_files.len());

    let thresholds = ColorThresholds {
        min_cluster_size: args.min_cluster,
        max_cluster_size: args.max_cluster,
        ..ColorThresholds::default()
    };

    // Load all frames
    let mut frames: Vec<(String, Vec<u8>, u32, u32)> = Vec::new();
    for path in &png_files {
        let img = image::open(path).unwrap_or_else(|e| {
            eprintln!("Error loading {}: {}", path.display(), e);
            std::process::exit(1);
        });
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        frames.push((name, rgba.into_raw(), w, h));
    }

    // Build baseline from first N frames
    let baseline_count = (args.baseline_frames as usize).min(frames.len());
    let mut baseline: Option<BaselineMask> = None;
    if baseline_count > 0 && !frames.is_empty() {
        let (_, _, w, h) = &frames[0];
        let mut bl = BaselineMask::new(*w, *h, 0.8);
        println!("\nBuilding baseline from {} frames...", baseline_count);
        for (name, pixels, w, h) in frames.iter().take(baseline_count) {
            let (red, green) = build_color_masks(pixels, *w, *h, &thresholds);
            bl.accumulate_frame(&red, &green);
            println!("  Accumulated: {}", name);
        }
        bl.build();
        println!("Baseline built.\n");
        baseline = Some(bl);
    }

    // Analyze each frame
    println!("{:-<70}", "");
    for (name, pixels, w, h) in &frames {
        let start = Instant::now();
        let heroes = detect_heroes(pixels, *w, *h, baseline.as_ref(), &thresholds);
        let elapsed = start.elapsed();

        let red_count = heroes.iter().filter(|h| h.team_color == TeamColor::Red).count();
        let green_count = heroes.iter().filter(|h| h.team_color == TeamColor::Green).count();

        println!(
            "{}: {} heroes detected ({} red, {} green) [{:.1}ms]",
            name, heroes.len(), red_count, green_count,
            elapsed.as_secs_f64() * 1000.0
        );

        for hero in &heroes {
            println!(
                "  {} at ({},{}) → {} [{}px]",
                hero.team_color, hero.x, hero.y, hero.zone, hero.cluster_size
            );
        }

        // Zone activity classification
        let snapshots = classify_zone_activity(&heroes, &team_side);
        if !snapshots.is_empty() {
            println!("  Zone Activity:");
            for snap in &snapshots {
                let detail = match snap.activity {
                    ActivityLevel::Fight => format!(
                        "{} ({} ally, {} enemy)",
                        snap.activity, snap.ally_count, snap.enemy_count
                    ),
                    _ => {
                        if snap.ally_count > 0 && snap.enemy_count > 0 {
                            format!("{} ({} ally, {} enemy)", snap.activity, snap.ally_count, snap.enemy_count)
                        } else if snap.ally_count > 0 {
                            format!("{} ({} ally)", snap.activity, snap.ally_count)
                        } else {
                            format!("{} ({} enemy)", snap.activity, snap.enemy_count)
                        }
                    }
                };
                println!("    {}: {}", snap.zone, detail);
            }
        }

        // Rolling window tracker
        tracker.push_frame(snapshots);
        let summary = tracker.summary();
        if !summary.is_empty() {
            println!("  Rolling Summary (last {} frames):", args.window);
            for s in &summary {
                let mut line = format!(
                    "    {}: avg {:.1} ally, {:.1} enemy | peak: {}",
                    s.zone, s.avg_ally_count, s.avg_enemy_count, s.peak_activity
                );
                if s.frames_with_fight > 0 {
                    line.push_str(&format!(
                        " | fight in {}/{} frames",
                        s.frames_with_fight, tracker.frame_count()
                    ));
                }
                println!("{}", line);
            }
        }

        let events = tracker.events();
        if !events.is_empty() {
            println!("  ⚠ Events:");
            for event in &events {
                println!("    {}", event);
            }
        }

        println!();
    }
    println!("{:-<70}", "");
}

struct Args {
    dir: String,
    baseline_frames: u32,
    min_cluster: usize,
    max_cluster: usize,
    team: String,
    window: usize,
}

impl Args {
    fn parse() -> Self {
        let mut args = Self {
            dir: "logs/minimap_capture".to_string(),
            baseline_frames: 5,
            min_cluster: 20,
            max_cluster: 200,
            team: "dire".to_string(),
            window: 5,
        };
        let raw: Vec<String> = env::args().collect();
        let mut i = 1;
        while i < raw.len() {
            match raw[i].as_str() {
                "--dir" => {
                    i += 1;
                    if i >= raw.len() {
                        eprintln!("Error: --dir requires a value");
                        std::process::exit(1);
                    }
                    args.dir = raw[i].clone();
                }
                "--baseline-frames" => {
                    i += 1;
                    args.baseline_frames = parse_u32(&raw, i, "--baseline-frames");
                }
                "--min-cluster" => {
                    i += 1;
                    args.min_cluster = parse_usize(&raw, i, "--min-cluster");
                }
                "--max-cluster" => {
                    i += 1;
                    args.max_cluster = parse_usize(&raw, i, "--max-cluster");
                }
                "--team" => {
                    i += 1;
                    if i >= raw.len() {
                        eprintln!("Error: --team requires a value");
                        std::process::exit(1);
                    }
                    let val = raw[i].to_lowercase();
                    if val != "dire" && val != "radiant" {
                        eprintln!("Error: --team must be 'dire' or 'radiant'");
                        std::process::exit(1);
                    }
                    args.team = val;
                }
                "--window" => {
                    i += 1;
                    args.window = parse_usize(&raw, i, "--window");
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

fn parse_usize(raw: &[String], i: usize, flag: &str) -> usize {
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
    println!("Usage: cargo run --example minimap_analyze [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --dir <PATH>            PNG directory (default: logs/minimap_capture)");
    println!("  --baseline-frames <N>   Frames for baseline (default: 5)");
    println!("  --min-cluster <N>       Min cluster size (default: 20)");
    println!("  --max-cluster <N>       Max cluster size (default: 200)");
    println!("  --team <dire|radiant>   Player's team (default: dire)");
    println!("  --window <N>            Rolling window size (default: 5)");
    println!("  --help, -h              Show this help");
}
