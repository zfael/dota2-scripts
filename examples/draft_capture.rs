//! Draft-screen capture probe.
//!
//! Answers one question before any draft-reading work is built: **does
//! `PrintWindow` return real pixels for Dota's draft screen?**
//!
//! The in-game HUD captures fine (see `logs/minimap_capture/`), but the draft
//! is a different render path, and a GDI capture of a GPU-composited surface
//! can silently come back as a solid black frame. Eyeballing a PNG is a weak
//! check — a mostly-dark draft screen and a failed capture look similar in a
//! thumbnail — so this reports luminance spread, distinct colour count, and
//! alpha, then states a verdict.
//!
//! Defaults to the **full client rect** rather than a fixed region, because the
//! draft slot coordinates are exactly what we do not know yet.
//!
//! Usage:
//!   cargo run --example draft_capture
//!   cargo run --example draft_capture -- --count 12 --interval-ms 5000
//!   cargo run --example draft_capture -- --x 0 --y 0 --width 800 --height 400
//!   cargo run --example draft_capture -- --output logs/draft_probe

use dota2_scripts::observability::minimap_artifacts::{
    build_artifact_metadata, save_capture_artifact, save_metadata_json,
};
use dota2_scripts::observability::minimap_capture_backend::{
    capture_window_region, find_dota2_window_rect, CaptureBackendResult,
};
use std::collections::HashSet;
use std::env;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn main() {
    let args = Args::parse();

    println!("Draft Capture Probe");
    println!();

    // Step 1: locate Dota and learn the client size, so a full-rect capture can
    // default to the actual window instead of a guess.
    print!("Finding Dota 2 window... ");
    let (client_width, client_height) = match find_dota2_window_rect() {
        CaptureBackendResult::Success { window_rect, .. } => {
            println!("found ({}x{})", window_rect.width, window_rect.height);
            (window_rect.width, window_rect.height)
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
    };

    let region = args.resolve_region(client_width, client_height);

    if region.x + region.width > client_width || region.y + region.height > client_height {
        eprintln!(
            "Error: region ({}+{}, {}+{}) does not fit the client area ({}x{}).",
            region.x, region.width, region.y, region.height, client_width, client_height
        );
        std::process::exit(1);
    }

    println!(
        "  Region: x={}, y={}, {}x{}",
        region.x, region.y, region.width, region.height
    );
    println!("  Output: {}", args.output);
    println!("  Frames: {} (every {}ms)", args.count, args.interval_ms);
    println!();

    let mut verdicts: Vec<Verdict> = Vec::with_capacity(args.count as usize);

    for frame in 1..=args.count {
        if frame > 1 {
            std::thread::sleep(Duration::from_millis(args.interval_ms));
        }

        print!("[{}/{}] capturing... ", frame, args.count);

        let start = Instant::now();
        let result = capture_window_region(region.x, region.y, region.width, region.height);
        let duration = start.elapsed();

        let (pixels, width, height) = match result {
            CaptureBackendResult::Success {
                pixels,
                width,
                height,
                ..
            } => (pixels, width, height),
            CaptureBackendResult::WindowNotFound => {
                println!("FAILED — window lost");
                verdicts.push(Verdict::Failed);
                continue;
            }
            CaptureBackendResult::CaptureError(e) => {
                println!("FAILED — {}", e);
                verdicts.push(Verdict::Failed);
                continue;
            }
        };

        let stats = FrameStats::analyse(&pixels);
        let verdict = stats.verdict();

        println!("{}ms, {}x{}", duration.as_millis(), width, height);
        println!(
            "        luma mean={:.1} min={:.0} max={:.0} stddev={:.1} | colours={} | alpha={}",
            stats.mean_luma,
            stats.min_luma,
            stats.max_luma,
            stats.stddev_luma,
            stats.distinct_colours,
            if stats.fully_opaque {
                "255 (opaque)".to_string()
            } else {
                format!("varies (min {})", stats.min_alpha)
            }
        );
        println!("        {} {}", verdict.marker(), verdict.describe());

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let file_stem = format!("draft_{}_{:03}", timestamp, frame);

        match save_capture_artifact(&args.output, &file_stem, &pixels, width, height) {
            Ok(path) => println!("        saved {}", path),
            Err(e) => eprintln!("        error saving PNG: {}", e),
        }

        let metadata = build_artifact_metadata(
            timestamp.to_string(),
            "bound".to_string(),
            region.x,
            region.y,
            region.width,
            region.height,
            width,
            height,
            duration.as_millis() as u64,
            format!("{:?}", verdict).to_lowercase(),
            None,
        );

        if let Err(e) = save_metadata_json(&args.output, &file_stem, &metadata) {
            eprintln!("        error saving metadata: {}", e);
        }

        verdicts.push(verdict);
    }

    println!();
    summarise(&verdicts, &args.output);
}

/// Cheap statistics that separate a real frame from a failed one.
struct FrameStats {
    mean_luma: f64,
    min_luma: f64,
    max_luma: f64,
    stddev_luma: f64,
    distinct_colours: usize,
    fully_opaque: bool,
    min_alpha: u8,
}

impl FrameStats {
    /// Pixels arrive as top-down RGBA (the backend swaps BGRA on the way out).
    ///
    /// Sampled rather than exhaustive: a full-screen capture is millions of
    /// pixels and every statistic here converges long before that. The stride is
    /// deliberately not a power of two, so it does not phase-lock onto UI
    /// gridlines and sample only one kind of pixel.
    fn analyse(pixels: &[u8]) -> Self {
        const STRIDE: usize = 7;

        let mut lumas: Vec<f64> = Vec::new();
        let mut colours: HashSet<u32> = HashSet::new();
        let mut min_alpha = u8::MAX;

        for chunk in pixels.chunks_exact(4).step_by(STRIDE) {
            let (r, g, b, a) = (chunk[0], chunk[1], chunk[2], chunk[3]);
            lumas.push(0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64);
            colours.insert(u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b));
            min_alpha = min_alpha.min(a);
        }

        if lumas.is_empty() {
            return Self {
                mean_luma: 0.0,
                min_luma: 0.0,
                max_luma: 0.0,
                stddev_luma: 0.0,
                distinct_colours: 0,
                fully_opaque: false,
                min_alpha: 0,
            };
        }

        let mean = lumas.iter().sum::<f64>() / lumas.len() as f64;
        let variance = lumas.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / lumas.len() as f64;

        Self {
            mean_luma: mean,
            min_luma: lumas.iter().cloned().fold(f64::INFINITY, f64::min),
            max_luma: lumas.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            stddev_luma: variance.sqrt(),
            distinct_colours: colours.len(),
            fully_opaque: min_alpha == 255,
            min_alpha,
        }
    }

    /// Thresholds are calibrated against a known-good capture: the existing
    /// minimap artifacts sample at mean luma ~52, max ~247, thousands of
    /// colours. A GDI capture that loses the GPU surface returns solid black —
    /// near-zero max luma and a single colour.
    fn verdict(&self) -> Verdict {
        if self.max_luma < 4.0 {
            Verdict::Blank
        } else if self.distinct_colours < 16 || self.stddev_luma < 2.0 {
            Verdict::Uniform
        } else {
            Verdict::Content
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Content,
    Uniform,
    Blank,
    Failed,
}

impl Verdict {
    fn marker(&self) -> &'static str {
        match self {
            Verdict::Content => "OK  ",
            Verdict::Uniform => "WARN",
            Verdict::Blank => "FAIL",
            Verdict::Failed => "FAIL",
        }
    }

    fn describe(&self) -> &'static str {
        match self {
            Verdict::Content => "real content — this screen captures.",
            Verdict::Uniform => "nearly uniform: a solid fill, an overlay, or a loading screen.",
            Verdict::Blank => "black frame — PrintWindow did not get the rendered surface.",
            Verdict::Failed => "capture call failed outright.",
        }
    }
}

fn summarise(verdicts: &[Verdict], output: &str) {
    let content = verdicts.iter().filter(|v| **v == Verdict::Content).count();
    let uniform = verdicts.iter().filter(|v| **v == Verdict::Uniform).count();
    let blank = verdicts.iter().filter(|v| **v == Verdict::Blank).count();
    let failed = verdicts.iter().filter(|v| **v == Verdict::Failed).count();

    println!("Summary: {} frames", verdicts.len());
    println!("  content : {}", content);
    println!("  uniform : {}", uniform);
    println!("  blank   : {}", blank);
    println!("  failed  : {}", failed);
    println!();

    if content > 0 {
        println!("VERDICT: this screen is capturable.");
        println!("Open the PNGs in {} and confirm they show the draft —", output);
        println!("the probe cannot tell a draft from a menu, only real pixels from a");
        println!("failed capture. Once confirmed, slot rectangles can be measured off them.");
    } else if blank > 0 || uniform > 0 {
        println!("VERDICT: no usable frame.");
        println!("Before concluding the approach is dead, check:");
        println!("  - Dota is in Borderless or Windowed, not exclusive fullscreen");
        println!("  - the draft screen was actually on-screen during the run");
        println!("  - this probe ran elevated, matching how the app runs");
    } else {
        println!("VERDICT: every capture call failed — see the errors above.");
    }
}

/// The capture rectangle, client-relative, after defaults are resolved.
struct Region {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// The hero-portrait strip only, derived from the measured draft geometry.
///
/// Measured at 1080p: ten 107px slots on a 124px pitch, the outermost edges
/// sitting 138px + 5*124 either side of centre. Everything scales with client
/// *height*, because Panorama sizes its UI that way and centres horizontally.
///
/// Worth having as its own mode: capturing this instead of the full client area
/// measured 17-20ms versus 59-61ms, and 144KB versus 2.4MB per frame. The
/// PrintWindow call still covers the whole window — that cost is fixed — but
/// everything downstream scales with the region.
fn strip_region(client_width: u32, client_height: u32) -> Region {
    const CENTER_GAP: f32 = 138.0 / 1080.0;
    const SLOT_WIDTH: f32 = 107.0 / 1080.0;
    const SLOT_HEIGHT: f32 = 64.0 / 1080.0;
    const PITCH: f32 = 124.0 / 1080.0;
    const SLOTS_PER_TEAM: f32 = 5.0;

    let h = client_height as f32;
    let centre = client_width as f32 / 2.0;

    // Left edge of the outermost ally slot through to the right edge of the
    // outermost enemy slot — the timer between them comes along for the ride.
    let half_span = CENTER_GAP * h + PITCH * h * (SLOTS_PER_TEAM - 1.0) + SLOT_WIDTH * h;
    let x = (centre - half_span).round().max(0.0);
    let width = (half_span * 2.0).round().min(client_width as f32 - x);

    Region {
        x: x as u32,
        y: 0,
        width: width as u32,
        height: (SLOT_HEIGHT * h).round().min(client_height as f32) as u32,
    }
}

struct Args {
    x: u32,
    y: u32,
    width: Option<u32>,
    height: Option<u32>,
    output: String,
    count: u32,
    /// Capture only the hero-portrait strip instead of the whole client area.
    strip: bool,
    interval_ms: u64,
}

impl Args {
    /// Unspecified width/height mean "to the client edge" — the whole point of
    /// the probe is that the interesting sub-rectangles are not known yet.
    fn resolve_region(&self, client_width: u32, client_height: u32) -> Region {
        if self.strip {
            return strip_region(client_width, client_height);
        }
        Region {
            x: self.x,
            y: self.y,
            width: self
                .width
                .unwrap_or_else(|| client_width.saturating_sub(self.x)),
            height: self
                .height
                .unwrap_or_else(|| client_height.saturating_sub(self.y)),
        }
    }

    fn parse() -> Self {
        let mut args = Self {
            x: 0,
            y: 0,
            width: None,
            height: None,
            output: "logs/draft_capture".to_string(),
            count: 1,
            strip: false,
            interval_ms: 3000,
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
                    args.width = Some(parse_u32(&raw, i, "--width"));
                }
                "--height" => {
                    i += 1;
                    args.height = Some(parse_u32(&raw, i, "--height"));
                }
                "--strip" => args.strip = true,
                "--count" => {
                    i += 1;
                    args.count = parse_u32(&raw, i, "--count").max(1);
                }
                "--interval-ms" => {
                    i += 1;
                    args.interval_ms = parse_u32(&raw, i, "--interval-ms") as u64;
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
    println!("Usage: cargo run --example draft_capture [OPTIONS]");
    println!();
    println!("Captures Dota's client area during the draft and reports whether the");
    println!("frame contains real pixels. Defaults to the full client rect.");
    println!();
    println!("Options:");
    println!("  --x <N>            Region X offset (default: 0)");
    println!("  --y <N>            Region Y offset (default: 0)");
    println!("  --width <N>        Region width (default: to the client edge)");
    println!("  --height <N>       Region height (default: to the client edge)");
    println!("  --count <N>        Number of frames to capture (default: 1)");
    println!("  --interval-ms <N>  Delay between frames (default: 3000)");
    println!("  --strip            Capture only the hero strip (3x faster, 16x smaller)");
    println!("  --output <DIR>     Output directory (default: logs/draft_capture)");
    println!("  --help, -h         Show this help");
}
