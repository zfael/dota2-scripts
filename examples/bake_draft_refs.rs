//! Bake the draft reference pack the app ships.
//!
//! Reads every portrait in `.cache/hero_portraits/` (CDN art fetched by
//! `scripts/fetch-hero-portraits.ps1`, plus any harvested exemplars and
//! negative references collected by the live probes), fingerprints them with
//! the exact descriptor `src/observability/draft_vision.rs` uses at runtime,
//! and writes `assets/draft_reference_pack.bin`.
//!
//! The pack is committed: descriptors for ~130 references are ~400KB, and
//! baking them means the app needs no network, no cache directory, and no PNG
//! decoding at runtime — reading a draft is ten crops x ~130 dot products.
//!
//! Naming convention, matching the live harvester:
//!   - `lina.png`            -> hero `lina`, CDN art
//!   - `rubick__1787776498.png` -> hero `rubick`, harvested exemplar (penalised
//!     at match time for its same-domain advantage)
//!   - `_empty__logo1.png`   -> negative reference `_empty` (empty-slot logo,
//!     menu badges — non-hero content a slot can hold)
//!
//! Rerun whenever the art, the harvested set, or the descriptor changes:
//!
//! ```text
//! cargo run --release --example bake_draft_refs
//! ```

use dota2_scripts::observability::draft_vision::{encode_reference_pack, fingerprint, Reference};
use std::path::PathBuf;

fn main() {
    let src = PathBuf::from(".cache/hero_portraits");
    let out = PathBuf::from("assets/draft_reference_pack.bin");

    let entries = match std::fs::read_dir(&src) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Cannot read {}: {e}", src.display());
            eprintln!("Run scripts/fetch-hero-portraits.ps1 first.");
            std::process::exit(1);
        }
    };

    let mut refs: Vec<Reference> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("png") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        let harvested = stem.contains("__");
        let name = stem.split("__").next().unwrap_or("?").to_string();

        match image::open(&path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                match fingerprint(rgba.as_raw(), rgba.width(), rgba.height()) {
                    Some(fp) => refs.push(Reference {
                        name,
                        fingerprint: fp,
                        harvested,
                    }),
                    None => skipped.push(format!("{stem}: degenerate image")),
                }
            }
            Err(e) => skipped.push(format!("{stem}: {e}")),
        }
    }

    if refs.is_empty() {
        eprintln!("No references produced — refusing to write an empty pack.");
        std::process::exit(1);
    }

    // Deterministic order so rebaking without changes leaves the file
    // untouched in git.
    refs.sort_by(|a, b| a.name.cmp(&b.name));

    let heroes = refs
        .iter()
        .filter(|r| !r.is_negative())
        .map(|r| r.name.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let exemplars = refs.iter().filter(|r| r.harvested && !r.is_negative()).count();
    let negatives = refs.iter().filter(|r| r.is_negative()).count();

    let bytes = encode_reference_pack(&refs);
    if let Some(parent) = out.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("Cannot create {}: {e}", parent.display());
            std::process::exit(1);
        }
    }
    if let Err(e) = std::fs::write(&out, &bytes) {
        eprintln!("Cannot write {}: {e}", out.display());
        std::process::exit(1);
    }

    println!(
        "Baked {} references ({heroes} heroes, {exemplars} harvested exemplars, \
         {negatives} negative) -> {} ({} KB)",
        refs.len(),
        out.display(),
        bytes.len() / 1024
    );
    if !skipped.is_empty() {
        println!("Skipped:");
        for s in &skipped {
            println!("  {s}");
        }
        std::process::exit(1);
    }
}
