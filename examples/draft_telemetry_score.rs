//! Offline scorer for draft telemetry frames.
//!
//! Loads a saved strip PNG from a telemetry session and prints, per slot,
//! the contrast plus the top matches against the shipped reference pack,
//! ignoring the occupancy gate — for diagnosing slots the live reader
//! refused to read.
//!
//! Usage:
//!   cargo run --release --example draft_telemetry_score -- <strip.png> [--slot N] [--save-crop path.png]
//!
//! The strip is assumed captured at 1920x1080 (the only validated client size).

use dota2_scripts::observability::draft_vision::{
    self, HARVESTED_PENALTY, Reference,
};

fn score_all(refs: &[Reference], fp: &[f32]) -> Vec<(String, f32)> {
    let mut best: std::collections::HashMap<&str, f32> = std::collections::HashMap::new();
    for r in refs {
        let mut s = draft_vision::trimmed_similarity(&r.fingerprint, fp);
        if r.harvested && !r.is_negative() {
            s -= HARVESTED_PENALTY;
        }
        let e = best.entry(r.name.as_str()).or_insert(f32::MIN);
        if s > *e {
            *e = s;
        }
    }
    let mut v: Vec<(String, f32)> = best.into_iter().map(|(n, s)| (n.to_string(), s)).collect();
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    v
}

/// Batch regression: replay the labelled corpus through the *shipping*
/// pipeline (`draft_vision` + the baked pack), not the probe's inlined copy.
///
/// Labels live in `logs/draft_capture/ground_truth.txt` as
/// `<capture_stem> <ALLY|ENEMY> <0-4> <hero>`; ALLY is the left column, so
/// `ALLY i` is slot `i` and `ENEMY i` is slot `5 + i`.
fn run_truth_regression(dir: &std::path::Path) {
    let truth_path = dir.join("ground_truth.txt");
    let text = std::fs::read_to_string(&truth_path).expect("read ground_truth.txt");

    // stem -> [(slot_index, hero)]
    let mut by_capture: std::collections::BTreeMap<String, Vec<(usize, String)>> =
        std::collections::BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() != 4 {
            eprintln!("skipping malformed line: {line}");
            continue;
        }
        let Ok(i) = f[2].parse::<usize>() else { continue };
        let slot = match f[1] {
            "ALLY" => i,
            "ENEMY" => i + 5,
            other => {
                eprintln!("skipping unknown team '{other}'");
                continue;
            }
        };
        by_capture
            .entry(f[0].to_string())
            .or_default()
            .push((slot, f[3].to_string()));
    }

    let refs = draft_vision::builtin_references().expect("pack");
    let (mut correct, mut wrong, mut missed) = (0usize, 0usize, 0usize);
    let mut failures: Vec<String> = Vec::new();

    for (stem, labels) in &by_capture {
        let png = dir.join(format!("{stem}.png"));
        let Ok(img) = image::open(&png) else {
            eprintln!("missing capture {}", png.display());
            continue;
        };
        let frame = img.to_rgba8();
        let (cw, ch) = (frame.width(), frame.height());
        // Full-screen capture: slot coordinates are already frame-relative.
        let outcomes = draft_vision::match_frame(&frame, cw, ch, 0, &refs);

        for (slot, expected) in labels {
            let o = &outcomes[*slot];
            match o.best() {
                Some((hero, score, margin)) if hero == expected => {
                    correct += 1;
                    let _ = (score, margin);
                }
                Some((hero, score, margin)) => {
                    wrong += 1;
                    failures.push(format!(
                        "  WRONG {stem} slot {slot}: got {hero} {score:.3} (margin {margin:.3}), want {expected}"
                    ));
                }
                None => {
                    missed += 1;
                    failures.push(format!(
                        "  MISS  {stem} slot {slot}: no read (contrast {:.1}), want {expected}",
                        o.contrast
                    ));
                }
            }
        }
    }

    for f in &failures {
        println!("{f}");
    }
    let total = correct + wrong + missed;
    println!(
        "\nshipping pipeline over labelled corpus: {correct}/{total} correct ({wrong} wrong, {missed} unread)"
    );
}

/// Replay whole telemetry sessions through the shipping pipeline *with
/// voting*, and score the final lineup against the user's own ✓/✗ verdicts in
/// `feedback.jsonl`.
///
/// Only every Nth frame was saved as a PNG, so this votes over fewer frames
/// than the live reader did — a pessimistic replay.
fn run_session_replay(root: &std::path::Path) {
    let refs = draft_vision::builtin_references().expect("pack");
    let (mut agree, mut disagree, mut unjudged) = (0usize, 0usize, 0usize);
    let mut notes: Vec<String> = Vec::new();

    let mut dirs: Vec<std::path::PathBuf> = std::fs::read_dir(root)
        .expect("read telemetry root")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();

    for dir in dirs {
        let mut frames: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("png"))
            .collect();
        frames.sort();
        if frames.is_empty() {
            continue;
        }

        let mut votes: Vec<draft_vision::SlotVotes> = (0..draft_vision::TOTAL_SLOTS)
            .map(|_| draft_vision::SlotVotes::default())
            .collect();
        for f in &frames {
            let Ok(img) = image::open(f) else { continue };
            let strip = img.to_rgba8();
            // Saved frames are strips; recover the client size they came from.
            let (cw, ch) = (1920u32, 1080u32);
            let (sx, _, sw, sh) = draft_vision::strip_region(cw, ch);
            if (strip.width(), strip.height()) != (sw, sh) {
                continue;
            }
            for (i, o) in draft_vision::match_frame(&strip, cw, ch, sx, &refs)
                .iter()
                .enumerate()
            {
                votes[i].observe(o);
            }
        }

        // The user's verdicts, keyed by slot.
        let fb_path = dir.join("feedback.jsonl");
        let Ok(fb) = std::fs::read_to_string(&fb_path) else {
            continue;
        };
        let name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("?");
        for line in fb.lines() {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let Some(slot) = v["slot_index"].as_u64().map(|n| n as usize) else {
                continue;
            };
            let ok = v["correct"].as_bool().unwrap_or(false);
            let read = if votes[slot].confident() {
                votes[slot].winner().map(|(h, _)| h.to_string())
            } else {
                None
            };

            // What the user said the slot really holds: the read they
            // confirmed, or the correction they typed.
            let expected: Option<String> = if ok {
                v["identified"].as_str().map(|s| s.to_string())
            } else {
                v["actual_hero"].as_str().map(|s| s.to_string())
            };
            let Some(expected) = expected else {
                unjudged += 1;
                continue;
            };

            match read {
                Some(h) if h == expected => agree += 1,
                Some(h) => {
                    disagree += 1;
                    notes.push(format!("  WRONG {name} slot {slot}: got {h}, want {expected}"));
                }
                None => {
                    disagree += 1;
                    notes.push(format!("  UNREAD {name} slot {slot}: want {expected}"));
                }
            }
        }
    }

    for n in &notes {
        println!("{n}");
    }
    println!(
        "\nsession replay vs user verdicts: {agree}/{} correct ({disagree} wrong, {unjudged} unlabelled)",
        agree + disagree
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--sessions") {
        let root = args
            .get(1)
            .map(std::path::PathBuf::from)
            .expect("usage: --sessions <telemetry_root>");
        run_session_replay(&root);
        return;
    }
    if args.first().is_some_and(|a| a == "--truth") {
        let dir = args
            .get(1)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("logs/draft_capture"));
        run_truth_regression(&dir);
        return;
    }
    // --compare a.png b.png: similarity of two crops, plain and mirrored.
    if args.first().is_some_and(|a| a == "--compare") {
        let a = image::open(&args[1]).expect("open a").to_rgba8();
        let b = image::open(&args[2]).expect("open b").to_rgba8();
        let fa = draft_vision::fingerprint(a.as_raw(), a.width(), a.height()).unwrap();
        let fb = draft_vision::fingerprint(b.as_raw(), b.width(), b.height()).unwrap();
        let bf = image::imageops::flip_horizontal(&b);
        let fbf = draft_vision::fingerprint(bf.as_raw(), bf.width(), bf.height()).unwrap();
        println!(
            "plain {:.3}  mirrored {:.3}",
            draft_vision::trimmed_similarity(&fa, &fb),
            draft_vision::trimmed_similarity(&fa, &fbf)
        );
        return;
    }
    let png = args.first().expect("usage: draft_telemetry_score <strip.png> [--slot N] [--save-crop path]");
    let only_slot: Option<usize> = args
        .iter()
        .position(|a| a == "--slot")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok());
    let save_crop: Option<&String> = args
        .iter()
        .position(|a| a == "--save-crop")
        .and_then(|i| args.get(i + 1));

    let (cw, ch) = (1920u32, 1080u32);
    let strip = image::open(png).expect("cannot open strip png").to_rgba8();
    let (sx, _sy, sw, sh) = draft_vision::strip_region(cw, ch);
    assert_eq!(
        (strip.width(), strip.height()),
        (sw, sh),
        "strip size does not match 1080p geometry"
    );

    let refs = draft_vision::builtin_references().expect("pack");
    let slots = draft_vision::resolve_slots(cw, ch);

    for slot in &slots {
        if only_slot.is_some_and(|n| n != slot.index) {
            continue;
        }
        // Same shift match_frame applies: slot coords are client-absolute,
        // the strip starts at sx.
        let x = slot.x - sx;
        let crop = image::imageops::crop_imm(&strip, x, slot.y, slot.w, slot.h).to_image();
        if let (Some(path), Some(n)) = (save_crop, only_slot) {
            if n == slot.index {
                crop.save(path).expect("save crop");
                println!("saved slot {n} crop -> {path}");
            }
        }
        let contrast = draft_vision::luma_std_dev(crop.as_raw(), crop.width(), crop.height());
        let Some(fp) = draft_vision::fingerprint(crop.as_raw(), crop.width(), crop.height())
        else {
            println!("slot {:>2}  contrast {:>5.1}  (no fingerprint)", slot.index, contrast);
            continue;
        };
        let ranked = score_all(&refs, &fp);
        let top: Vec<String> = ranked
            .iter()
            .take(4)
            .map(|(n, s)| format!("{n} {s:.3}"))
            .collect();
        println!(
            "slot {:>2}  contrast {:>5.1}  {}",
            slot.index,
            contrast,
            top.join(" | ")
        );
    }
}
