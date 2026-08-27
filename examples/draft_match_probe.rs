//! Throwaway probe: can we identify drafted heroes from a screen capture?
//!
//! **Deliberately self-contained.** Every algorithm here is inlined rather than
//! imported from `src/`, because none of it has earned a place in the library
//! yet. This repo already carries one computer-vision module written before it
//! was proven against real data (`src/observability/minimap_analysis.rs`), which
//! is unreferenced and does not work. The point of this file is to produce the
//! evidence *first*. If the numbers are good, the logic gets promoted into
//! `src/observability/` with unit tests; if they are not, deleting one example
//! costs nothing.
//!
//! What it does:
//!   1. Builds fingerprints for all ~127 heroes from `.cache/hero_portraits/`
//!      (populated by `scripts/fetch-hero-portraits.ps1`).
//!   2. Crops the ten draft slots out of a capture using measured geometry.
//!   3. Matches each occupied slot and reports score, runner-up, and margin.
//!   4. Optionally votes across several frames of the same draft.
//!   5. Writes a montage pairing each crop with whatever it matched, so the
//!      result can be checked by eye and not just believed.
//!
//! # Results on `logs/draft_capture` (8 games, 117 labelled slots, 30 heroes)
//!
//! | Configuration | Correct |
//! |---|---|
//! | as first written | 6/10 on one frame |
//! | + skip the player-colour bar (`--inset-px 6`) | 63/65 steady frames |
//! | + trimmed similarity (`--trim 0.15`) | 65/65 steady frames |
//! | + contrast gate 12 -> 25, voting across frames | 66/66, all frames |
//! | + second capture session (16 new heroes) | 105/106 single-frame |
//!
//! The single miss is a mid-reveal frame; voting across its siblings takes it to
//! 8/8 and flips the wrong read back to correct.
//!
//! Four findings drove that, none of them obvious up front:
//!   - Rows 0-5 of each slot are a player-colour bar, fixed per slot index. Left
//!     in, fingerprints matched by *seat* rather than by hero.
//!   - Occlusions (own-hero badge, FPS overlay) corrupt a minority of cells;
//!     trimming the worst-agreeing cells before scoring recovers the match.
//!   - Frames caught mid-reveal score 11-17 contrast against 32-62 for settled
//!     ones, and every one that slipped through produced a confident wrong answer.
//!   - **Cosmetics change the portrait.** Across eight live bot drafts, GSI's
//!     own hero appeared in the read lineup only 4/8 times, and all four misses
//!     were arcana heroes the player owns (Rubick, Lina, Shadow Fiend,
//!     Earthshaker). No CDN art fixes this: the strip renders the arcana from
//!     the cosmetic's model and every guessable variant path 404s. `--harvest`
//!     saves those crops as extra exemplars instead, labelled by GSI so nothing
//!     is guessed. A hero's score is the best over its exemplars.
//!   - **ALLY/ENEMY in the geometry mean left/right, not friend/foe.** Radiant
//!     is always drawn left, so a Dire player's own team is the right-hand
//!     group — observed in 3 of 8 drafts. `player.team_name` resolves it.
//!   - **Nothing here knows what screen it is looking at.** Pointed at the main
//!     menu, this reported pangolier, wisp and dazzle for an item icon, a menu
//!     caption and a level badge. No threshold fixes it: a correct Pugna scored
//!     0.585 while Dazzle-on-menu-art scored 0.573, and contrast and margin
//!     overlap just as badly. A quorum rule ("require N slots") would reject
//!     legitimate early-draft frames, where only two allies are visible. The gate
//!     has to come from outside the image — see `examples/gsi_state_probe.rs`,
//!     which tests whether `map.game_state` supplies it.
//!
//! Usage:
//!   cargo run --example draft_match_probe
//!   cargo run --example draft_match_probe -- --capture logs/draft_capture/foo.png
//!   cargo run --example draft_match_probe -- --capture a.png --vote b.png --vote c.png
//!   cargo run --example draft_match_probe -- --sweep --dim 24 --trim 0.2
//!   cargo run --example draft_match_probe -- --confusion
//!
//! `--live` reads the running game instead of a saved capture, gated on GSI, and
//! is the end-to-end check that we look only while a draft is on screen. It
//! binds port 3000 (the shipped .cfg already points there), so **stop the main
//! app first**:
//!
//!   cargo run --release --example draft_match_probe -- --live

use image::{imageops, GenericImage, RgbaImage};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Slot geometry — measured off logs/draft_capture at 1920x1080.
//
// The strip proved exactly symmetric about the horizontal centre: the innermost
// ally slot ends 138px left of centre, the innermost enemy slot starts 138px
// right of it. Panorama scales with viewport height and centres horizontally, so
// these are expressed in units of height off the centre line rather than as
// fractions of width, which would drift with aspect ratio.
// ---------------------------------------------------------------------------

const CENTER_GAP: f32 = 138.0 / 1080.0;
const SLOT_WIDTH: f32 = 107.0 / 1080.0;
const SLOT_HEIGHT: f32 = 64.0 / 1080.0;
const PITCH: f32 = 124.0 / 1080.0;
const SLOTS_PER_TEAM: usize = 5;
const SLOT_ASPECT: f32 = 107.0 / 64.0;

/// Contrast below this means the slot holds no usable portrait.
///
/// Measured, not guessed. Across the captures in `logs/draft_capture`, a settled
/// portrait scores 32-62 while a slot caught mid-fade during the reveal
/// animation scores 11-17. The original threshold of 12 let those half-drawn
/// frames through, and every one of them produced a confident-looking wrong
/// answer. 25 sits in the empty band between the two populations.
const OCCUPIED_MIN_STDDEV: f32 = 25.0;

#[derive(Debug, Clone, Copy)]
struct SlotRect {
    label: &'static str,
    index: usize,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

/// `top_inset` is a fraction of client height skipped at the top of each slot.
///
/// Rows 0-5 of a slot are the player-colour bar: a block of fully saturated
/// colour that is fixed per slot *index* and identical across games. Left in, it
/// dominates the normalised descriptor, and a reference healed from slot N then
/// matches whatever occupies slot N in a later game regardless of hero. Skipping
/// it is what makes a fingerprint about the hero rather than the seat.
fn resolve_slots(client_w: u32, client_h: u32, top_inset: f32) -> Vec<SlotRect> {
    let h = client_h as f32;
    let centre = client_w as f32 / 2.0;
    let gap = CENTER_GAP * h;
    let w = (SLOT_WIDTH * h).round();
    let sh = (SLOT_HEIGHT * h).round();
    let pitch = PITCH * h;
    let inset = (top_inset * h).round().max(0.0);
    let sh = (sh - inset).max(1.0);

    let mut slots = Vec::with_capacity(SLOTS_PER_TEAM * 2);

    for index in 0..SLOTS_PER_TEAM {
        // Allies run inward from the centre, so index 4 is adjacent to the gap.
        let from_inner = (SLOTS_PER_TEAM - 1 - index) as f32;
        slots.push(SlotRect {
            label: "ALLY",
            index,
            x: (centre - gap - w - pitch * from_inner).round().max(0.0) as u32,
            y: inset as u32,
            w: w as u32,
            h: sh as u32,
        });
    }
    for index in 0..SLOTS_PER_TEAM {
        slots.push(SlotRect {
            label: "ENEMY",
            index,
            x: (centre + gap + pitch * index as f32).round().max(0.0) as u32,
            y: inset as u32,
            w: w as u32,
            h: sh as u32,
        });
    }

    slots
}

/// The horizontal band containing all ten slots, for a cheap partial capture.
///
/// Full-frame capture costs 59-61ms and 2.4MB; this band costs 13-14ms and
/// 144KB, which is what makes polling once a second during a draft free.
/// Returns `(x, y, width, height)` in client coordinates.
fn strip_region(client_width: u32, client_height: u32) -> (u32, u32, u32, u32) {
    let h = client_height as f32;
    let centre = client_width as f32 / 2.0;
    let half_span =
        CENTER_GAP * h + PITCH * h * (SLOTS_PER_TEAM as f32 - 1.0) + SLOT_WIDTH * h;
    let x = (centre - half_span).round().max(0.0);
    let width = (half_span * 2.0).round().min(client_width as f32 - x);
    let height = (SLOT_HEIGHT * h).round().min(client_height as f32);
    (x as u32, 0, width as u32, height as u32)
}

// ---------------------------------------------------------------------------
// Fingerprinting
//
// References are Valve CDN art; probes are crops of Dota's own draft strip. They
// differ in crop, scale, colour grading, and overlaid UI. Both sides reduce to
// the same descriptor: box-downscaled to DIM^2, normalised per channel to zero
// mean and unit variance, then unit length overall. The normalisation is what
// discards a global tint while keeping the structure that separates heroes.
// ---------------------------------------------------------------------------

fn box_downscale(pixels: &[u8], width: u32, height: u32, dim: usize) -> Vec<f32> {
    let (w, h) = (width as usize, height as usize);
    let mut out = vec![0f32; dim * dim * 3];

    for ty in 0..dim {
        let y0 = ty * h / dim;
        let y1 = (((ty + 1) * h) / dim).max(y0 + 1).min(h);
        for tx in 0..dim {
            let x0 = tx * w / dim;
            let x1 = (((tx + 1) * w) / dim).max(x0 + 1).min(w);

            let (mut r, mut g, mut b, mut n) = (0f32, 0f32, 0f32, 0f32);
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y * w + x) * 4;
                    r += pixels[i] as f32;
                    g += pixels[i + 1] as f32;
                    b += pixels[i + 2] as f32;
                    n += 1.0;
                }
            }

            let idx = (ty * dim + tx) * 3;
            out[idx] = r / n;
            out[idx + 1] = g / n;
            out[idx + 2] = b / n;
        }
    }

    out
}

fn fingerprint(pixels: &[u8], width: u32, height: u32, dim: usize) -> Option<Vec<f32>> {
    if width == 0 || height == 0 {
        return None;
    }
    if pixels.len() < width as usize * height as usize * 4 {
        return None;
    }

    let mut v = box_downscale(pixels, width, height, dim);
    let cells = dim * dim;

    for c in 0..3 {
        let mean = (0..cells).map(|i| v[i * 3 + c]).sum::<f32>() / cells as f32;
        let var =
            (0..cells).map(|i| (v[i * 3 + c] - mean).powi(2)).sum::<f32>() / cells as f32;
        let sd = var.sqrt().max(1e-6);
        for i in 0..cells {
            v[i * 3 + c] = (v[i * 3 + c] - mean) / sd;
        }
    }

    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    for x in v.iter_mut() {
        *x /= norm;
    }

    Some(v)
}

/// Both vectors are unit length, so the dot product is the cosine similarity.
fn similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Cosine similarity ignoring the worst-agreeing `trim` fraction of cells.
///
/// Dota paints things over portraits — the local player's badge, the FPS readout
/// if enabled, and whatever Valve adds next patch. Each corrupts a *contiguous
/// minority* of the descriptor while the rest still agrees perfectly.
///
/// Plain cosine averages that damage across the whole vector, so a 15% occlusion
/// drags the score down uniformly and can flip a ranking. Dropping the worst
/// cells before scoring recovers the match without needing to know where the
/// occlusion is — which is the point, since we cannot enumerate future UI.
///
/// The kept cells are renormalised so scores stay comparable between candidates
/// that lost different cells; without that, a hero that happens to disagree in
/// low-energy cells would score artificially high.
fn trimmed_similarity(a: &[f32], b: &[f32], trim: f32) -> f32 {
    if trim <= 0.0 {
        return similarity(a, b);
    }

    let cells = a.len() / 3;
    let mut per_cell: Vec<(f32, usize)> = (0..cells)
        .map(|i| {
            let dot = (0..3).map(|c| a[i * 3 + c] * b[i * 3 + c]).sum::<f32>();
            (dot, i)
        })
        .collect();

    // Ascending: the worst-agreeing cells sort to the front and get dropped.
    per_cell.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));

    let drop = ((cells as f32) * trim).round() as usize;
    let kept = &per_cell[drop.min(cells.saturating_sub(1))..];

    let mut dot = 0f32;
    let mut norm_a = 0f32;
    let mut norm_b = 0f32;
    for (cell_dot, i) in kept {
        dot += cell_dot;
        for c in 0..3 {
            norm_a += a[i * 3 + c] * a[i * 3 + c];
            norm_b += b[i * 3 + c] * b[i * 3 + c];
        }
    }

    let denom = (norm_a.sqrt() * norm_b.sqrt()).max(1e-6);
    dot / denom
}

/// Fingerprint only the left `keep` fraction of an image.
///
/// Dota paints things over the right-hand side of draft slots: the local
/// player's badge on ally slot 1, and — if the user has the performance display
/// on — the FPS/GPU readout across the rightmost enemy slots. Both are
/// *variable* pixels sitting in a region that is otherwise stable.
///
/// That matters most for healed references: baking a frame's FPS digits into a
/// reference makes it disagree with every future frame in exactly that region.
/// Ignoring the strip where overlays live costs a little hero detail and buys
/// back a reference that stays valid.
fn fingerprint_left(
    pixels: &[u8],
    width: u32,
    height: u32,
    dim: usize,
    keep: f32,
) -> Option<Vec<f32>> {
    if keep >= 1.0 {
        return fingerprint(pixels, width, height, dim);
    }

    let kept_w = ((width as f32 * keep).round() as u32).max(1);
    if pixels.len() < width as usize * height as usize * 4 {
        return None;
    }

    let mut out = Vec::with_capacity((kept_w * height * 4) as usize);
    for y in 0..height as usize {
        let start = (y * width as usize) * 4;
        out.extend_from_slice(&pixels[start..start + kept_w as usize * 4]);
    }

    fingerprint(&out, kept_w, height, dim)
}

/// Re-frame a reference portrait to match how Dota frames a draft slot.
///
/// The CDN art is a 256x144 landscape (aspect 1.778); a slot is 107x64 (1.672)
/// and visibly *tighter* on the hero. Feeding the full reference in compares two
/// different framings of the same art, which is why heroes that look identical
/// in the verification sheet still only score ~0.3.
///
/// `zoom` is the fraction of source height kept; `y_offset` shifts the window as
/// a fraction of source height, positive downward. Both are swept empirically
/// against known-correct pairs rather than guessed.
fn reframe(art: &RgbaImage, zoom: f32, y_offset: f32, slot_aspect: f32) -> (Vec<u8>, u32, u32) {
    let (sw, sh) = (art.width() as f32, art.height() as f32);

    let crop_h = (sh * zoom).clamp(1.0, sh);
    let crop_w = (crop_h * slot_aspect).clamp(1.0, sw);

    let x0 = ((sw - crop_w) / 2.0).max(0.0);
    let y0 = (((sh - crop_h) / 2.0) + y_offset * sh).clamp(0.0, sh - crop_h);

    let (x0, y0) = (x0 as u32, y0 as u32);
    let (cw, ch) = (crop_w as u32, crop_h as u32);

    let mut out = Vec::with_capacity((cw * ch * 4) as usize);
    for y in 0..ch {
        for x in 0..cw {
            out.extend_from_slice(&art.get_pixel(x0 + x, y0 + y).0);
        }
    }

    (out, cw, ch)
}

/// All ten heroes in the default capture, confirmed by eye against the CDN art.
/// ALLY 1 is corroborated independently: the strategy screen names it in text
/// ("ENTRANDO COMO OUTWORLD DESTROYER").
///
/// Having every slot labelled — not just the ones that matched — is what makes a
/// parameter sweep honest. With only the successes listed, "no wrong answers"
/// is true by construction and the margin gate looks better than it is.
const GROUND_TRUTH: &[(&str, usize, &str)] = &[
    ("ALLY", 0, "obsidian_destroyer"),
    ("ALLY", 1, "axe"),
    ("ALLY", 2, "lion"),
    ("ALLY", 3, "witch_doctor"),
    ("ALLY", 4, "juggernaut"),
    ("ENEMY", 0, "death_prophet"),
    ("ENEMY", 1, "tidehunter"),
    ("ENEMY", 2, "drow_ranger"),
    ("ENEMY", 3, "vengefulspirit"),
    ("ENEMY", 4, "warlock"),
];

/// One labelled slot: which capture, which slot, which hero.
type TruthTable = Vec<(String, usize, String)>;

/// Load labels for `capture_stem` from a whitespace-separated file:
///
/// ```text
/// # capture_stem   TEAM   index   hero
/// draft_1787768839_003  ALLY  0  vengefulspirit
/// ```
///
/// Lines for other captures are ignored, so one file covers every game. Slots
/// whose hero could not be identified with confidence are simply left out —
/// an unlabelled slot is scored as unknown rather than guessed at, which keeps
/// the accuracy figure honest.
fn load_truth(path: &Path, capture_stem: &str) -> TruthTable {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Cannot read truth file {}: {e}", path.display());
            std::process::exit(1);
        }
    };

    let mut out = TruthTable::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 4 {
            eprintln!("truth file line {}: expected 4 fields, got {}", n + 1, parts.len());
            continue;
        }
        if parts[0] != capture_stem {
            continue;
        }
        match parts[2].parse::<usize>() {
            Ok(index) => out.push((parts[1].to_string(), index, parts[3].to_string())),
            Err(_) => eprintln!("truth file line {}: bad slot index '{}'", n + 1, parts[2]),
        }
    }
    out
}

fn builtin_truth() -> TruthTable {
    GROUND_TRUTH
        .iter()
        .map(|(t, i, h)| (t.to_string(), *i, h.to_string()))
        .collect()
}

fn expected_in<'a>(truth: &'a TruthTable, slot: &SlotRect) -> Option<&'a str> {
    truth
        .iter()
        .find(|(l, i, _)| l == slot.label && *i == slot.index)
        .map(|(_, _, hero)| hero.as_str())
}

fn luma_std_dev(pixels: &[u8], width: u32, height: u32) -> f32 {
    let count = width as usize * height as usize;
    if count == 0 {
        return 0.0;
    }
    let lumas: Vec<f32> = (0..count)
        .map(|i| {
            let p = i * 4;
            0.299 * pixels[p] as f32 + 0.587 * pixels[p + 1] as f32 + 0.114 * pixels[p + 2] as f32
        })
        .collect();
    let mean = lumas.iter().sum::<f32>() / count as f32;
    (lumas.iter().map(|l| (l - mean).powi(2)).sum::<f32>() / count as f32).sqrt()
}

/// What one frame concluded about one slot.
struct SlotOutcome {
    slot: SlotRect,
    contrast: f32,
    /// Ranked candidates, best first. Empty when the slot held no portrait.
    ranked: Vec<(String, f32)>,
}

impl SlotOutcome {
    fn best(&self) -> Option<(&str, f32, f32)> {
        let (hero, score) = self.ranked.first()?;
        let runner_up = self.ranked.get(1).map(|(_, s)| *s).unwrap_or(0.0);
        Some((hero.as_str(), *score, score - runner_up))
    }
}

/// Match every slot in one frame. Pure: no printing, no scoring.
///
/// Split out so the single-frame table and the multi-frame vote run *identical*
/// matching, rather than two paths that can drift apart.
fn match_frame(
    frame: &RgbaImage,
    refs: &[(String, Vec<f32>, RgbaImage, bool)],
    args: &Args,
) -> Vec<SlotOutcome> {
    // A saved capture is the whole client rect, so the frame's own dimensions
    // are the client dimensions and slots need no shifting.
    match_frame_in(frame, frame.width(), frame.height(), 0, refs, args)
}

/// Match against geometry resolved for a client of `client_w` x `client_h`,
/// with slot x-coordinates shifted left by `origin_x`.
///
/// Live capture reads only the strip (x=219, 1482x64 at 1080p) rather than the
/// whole screen — 13-14ms instead of 59-61ms. The slots still have to be located
/// in *client* space and then translated into the crop, which is what `origin_x`
/// does. Resolving geometry from the strip's own dimensions would silently
/// produce nonsense.
fn match_frame_in(
    frame: &RgbaImage,
    client_w: u32,
    client_h: u32,
    origin_x: u32,
    refs: &[(String, Vec<f32>, RgbaImage, bool)],
    args: &Args,
) -> Vec<SlotOutcome> {
    resolve_slots(client_w, client_h, args.top_inset)
        .into_iter()
        .map(|mut slot| {
            slot.x = slot.x.saturating_sub(origin_x);

            let Some(px) = crop(frame, &slot) else {
                return SlotOutcome {
                    slot,
                    contrast: 0.0,
                    ranked: Vec::new(),
                };
            };

            let contrast = luma_std_dev(&px, slot.w, slot.h);
            if contrast < OCCUPIED_MIN_STDDEV {
                return SlotOutcome {
                    slot,
                    contrast,
                    ranked: Vec::new(),
                };
            }

            let Some(fp) = fingerprint_left(&px, slot.w, slot.h, args.dim, args.keep_left) else {
                return SlotOutcome {
                    slot,
                    contrast,
                    ranked: Vec::new(),
                };
            };

            // Best over each hero's exemplars, not per file: a hero with an
            // arcana exemplar would otherwise occupy two of the top slots and
            // squeeze a genuine rival out of the runner-up position, quietly
            // inflating the margin.
            let mut best: std::collections::HashMap<&str, f32> = std::collections::HashMap::new();
            for (n, rfp, _, harvested) in refs {
                // Harvested exemplars are crops of the draft strip itself, so
                // they share the probe's colour grading, scale and badge
                // overlay. That domain match inflates their score against *any*
                // real capture, not just their own hero's -- the Legion
                // Commander arcana crop outscored Pugna's own CDN portrait in
                // Pugna's slot, 0.623 to 0.585, and took the match.
                //
                // The penalty prices that advantage back out. It is a large
                // correction, but the gap it has to preserve is larger still: an
                // arcana exemplar matching its own portrait scores far above a
                // wrong hero's CDN art, so it still wins where it should.
                let s = trimmed_similarity(&fp, rfp, args.trim)
                    - if *harvested { args.harvest_penalty } else { 0.0 };
                let slot = best.entry(n.as_str()).or_insert(f32::NEG_INFINITY);
                if s > *slot {
                    *slot = s;
                }
            }
            let mut ranked: Vec<(String, f32)> =
                best.into_iter().map(|(n, s)| (n.to_string(), s)).collect();
            ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            // A leading underscore marks a NEGATIVE reference: things that
            // occupy a slot without being a hero. Two turned up in real
            // captures — the Dota logo Valve draws in an unfilled slot, and the
            // battle-pass badge visible for a beat after GSI reports
            // HERO_SELECTION but before the draft screen finishes rendering.
            //
            // Both carry plenty of contrast, so the occupancy gate passes them
            // and the matcher then returns the nearest of 127 heroes for a slot
            // holding no hero at all. Ranking them as candidates lets the slot
            // resolve to "empty" instead, which is the truthful answer and the
            // one that matters most while a draft is still in progress.
            if ranked.first().is_some_and(|(n, _)| n.starts_with('_')) {
                return SlotOutcome {
                    slot,
                    contrast,
                    ranked: Vec::new(),
                };
            }
            ranked.retain(|(n, _)| !n.starts_with('_'));
            ranked.truncate(8);

            SlotOutcome {
                slot,
                contrast,
                ranked,
            }
        })
        .collect()
}

fn crop(frame: &RgbaImage, r: &SlotRect) -> Option<Vec<u8>> {
    if r.x + r.w > frame.width() || r.y + r.h > frame.height() {
        return None;
    }
    let mut out = Vec::with_capacity((r.w * r.h * 4) as usize);
    for row in 0..r.h {
        for col in 0..r.w {
            let p = frame.get_pixel(r.x + col, r.y + row);
            out.extend_from_slice(&p.0);
        }
    }
    Some(out)
}

// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Live mode: GSI-gated reading of the running game.
//
// This is the end-to-end test of the one thing offline scoring cannot check —
// that we look *only* while a draft is on screen. Pointed at the main menu, the
// matcher happily returned pangolier, wisp and dazzle for an item icon, a menu
// caption and a level badge, and no threshold on contrast, score or margin
// separates that from a real draft. The gate has to come from GSI.
//
// Confirmed on the wire: map.game_state reaches a *player*, not just a
// spectator, and reads DOTA_GAMERULES_STATE_HERO_SELECTION during the draft and
// DOTA_GAMERULES_STATE_STRATEGY_TIME on the strategy screen that follows. Both
// show the ten-slot strip, so both are capture windows. At the main menu there
// is no map block at all.
// ---------------------------------------------------------------------------

/// States in which the ten-slot strip is on screen.
const DRAFT_STATES: &[&str] = &[
    "DOTA_GAMERULES_STATE_HERO_SELECTION",
    "DOTA_GAMERULES_STATE_STRATEGY_TIME",
];

#[derive(Clone, Default)]
struct Gate {
    game_state: String,
    /// Scopes a voting session. Votes must never carry across games.
    matchid: String,
    /// Set once the local player's own pick resolves.
    own_hero: String,
    /// "radiant" or "dire". The strip is always Radiant-left, Dire-right, so
    /// without this the geometry's ALLY/ENEMY labels are really just left/right
    /// and are inverted for a Dire player. Observed directly: own hero landed on
    /// the "ENEMY" side in 3 of 8 bot drafts.
    team_name: String,
    /// Bumped on every payload, so the loop can tell "no GSI yet" from "idle".
    payloads: u64,
}

impl Gate {
    fn is_open(&self) -> bool {
        DRAFT_STATES.contains(&self.game_state.as_str())
    }
}

/// Accumulated votes for one slot across the frames of a single draft.
#[derive(Default)]
struct SlotVotes {
    tally: std::collections::HashMap<String, u32>,
    best_score: f32,
    frames_seen: u32,
    /// Votes cast in this slot across the draft, however they were spread.
    total_votes: u32,
    /// Frames in which this slot clearly held *something* (contrast passed).
    occupied_frames: u32,
}

/// A slot must have looked occupied this many times before it is judged at all.
const MIN_OCCUPIED_FRAMES: u32 = 2;

/// Share of a slot's occupied frames that must agree on the same hero.
///
/// The metric is agreement *rate*, not a raw count. An absolute threshold looks
/// right on one sample and fails on the next, because both genuine and
/// unmatchable slots accumulate more votes as a draft runs longer: at 7-8 frames
/// the arcana slots drew 1 vote against 2-4 genuine, but at 12 frames one drew 2
/// -- clearing a `>= 2` bar and reporting Skywrath Mage for a Shadow Fiend
/// Arcana. As a rate the two populations stay cleanly apart at any length:
/// genuine slots agree on nearly every occupied frame, while an unmatchable
/// portrait keeps changing its mind (2 of 7 there, against 7 of 7).
const MIN_AGREEMENT: f32 = 0.6;

impl SlotVotes {
    fn winner(&self) -> Option<(&str, u32)> {
        self.tally
            .iter()
            .max_by_key(|(_, n)| **n)
            .map(|(h, n)| (h.as_str(), *n))
    }

    /// Whether the winner is trustworthy, and why not when it isn't.
    ///
    /// A draft helper that names the wrong hero is worse than one that says it
    /// does not know: the user acts on a counter-pick that isn't there. So an
    /// unstable slot reports as unknown rather than as its best guess.
    fn confident(&self) -> bool {
        match self.winner() {
            Some((_, n)) => {
                self.occupied_frames >= MIN_OCCUPIED_FRAMES && self.agreement() >= MIN_AGREEMENT
                    // A slot that split its votes evenly is not settled either.
                    && (n as f32) / (self.total_votes.max(1) as f32) >= MIN_AGREEMENT
            }
            None => false,
        }
    }

    /// Fraction of occupied frames that agreed on the winning hero.
    fn agreement(&self) -> f32 {
        match self.winner() {
            Some((_, n)) => (n as f32) / (self.occupied_frames.max(1) as f32),
            None => 0.0,
        }
    }
}

fn run_live(args: &Args, refs: &[(String, Vec<f32>, RgbaImage, bool)]) {
    use dota2_scripts::observability::minimap_capture_backend::{
        capture_window_region, find_dota2_window_rect, CaptureBackendResult,
    };

    let gate: Arc<Mutex<Gate>> = Arc::new(Mutex::new(Gate::default()));

    // The GSI listener runs on its own thread with its own runtime, so the
    // capture loop below stays plain synchronous code.
    spawn_gsi_listener(args.port, gate.clone());

    println!("Live draft reader");
    println!("  gsi port    : {}", args.port);
    println!("  poll        : every {}ms while the gate is open", args.poll_ms);
    println!("  references  : {} heroes", refs.len());
    println!("  open states : {}", DRAFT_STATES.join(", "));
    println!();
    println!("Enter and leave bot matches. Every state change is logged, and");
    println!("capture happens ONLY between GATE OPEN and GATE CLOSED.");
    println!("Ctrl+C when done.");
    println!();

    let mut was_open = false;
    let mut session_match = String::new();
    let mut votes: Vec<SlotVotes> = Vec::new();
    let mut last_state = String::new();
    let mut frames_this_draft = 0u32;
    let mut skipped_while_closed = 0u64;
    // One exemplar per hero, ever: seeded with every hero that already has a
    // same-domain exemplar on disk, so nothing is harvested twice across runs.
    let mut harvested: std::collections::HashSet<String> = refs
        .iter()
        .filter(|(_, _, _, h)| *h)
        .map(|(n, _, _, _)| n.clone())
        .collect();

    loop {
        let snapshot = gate.lock().map(|g| g.clone()).unwrap_or_default();

        if snapshot.game_state != last_state {
            let stamp = timestamp();
            if snapshot.game_state.is_empty() {
                println!("[{stamp}] GSI connected, no game_state yet");
            } else {
                println!(
                    "[{stamp}] state -> {}{}",
                    snapshot.game_state,
                    if snapshot.is_open() { "   (capture window)" } else { "" }
                );
            }
            last_state = snapshot.game_state.clone();
        }

        let open = snapshot.is_open();

        // --- gate transitions ---------------------------------------------
        if open && !was_open {
            session_match = snapshot.matchid.clone();
            votes = (0..SLOTS_PER_TEAM * 2).map(|_| SlotVotes::default()).collect();
            frames_this_draft = 0;
            println!(
                "[{}] GATE OPEN  match {}  -- capturing",
                timestamp(),
                if session_match.is_empty() { "?" } else { &session_match }
            );
        }
        if !open && was_open {
            println!(
                "[{}] GATE CLOSED after {frames_this_draft} frames",
                timestamp()
            );
            report_votes(&votes, &snapshot.own_hero, &snapshot.team_name);
            println!(
                "   (skipped {skipped_while_closed} polls while closed so far)"
            );
            println!();
        }
        was_open = open;

        // A new match while the gate is already open: reset rather than blend.
        if open && !snapshot.matchid.is_empty() && snapshot.matchid != session_match {
            println!(
                "[{}] match changed {} -> {}, resetting votes",
                timestamp(),
                if session_match.is_empty() { "?" } else { &session_match },
                snapshot.matchid
            );
            session_match = snapshot.matchid.clone();
            votes = (0..SLOTS_PER_TEAM * 2).map(|_| SlotVotes::default()).collect();
            frames_this_draft = 0;
        }

        if !open {
            skipped_while_closed += 1;
            std::thread::sleep(Duration::from_millis(args.poll_ms));
            continue;
        }

        // --- capture ------------------------------------------------------
        let CaptureBackendResult::Success { window_rect, .. } = find_dota2_window_rect() else {
            println!("[{}] Dota window vanished; waiting", timestamp());
            std::thread::sleep(Duration::from_millis(args.poll_ms));
            continue;
        };
        let (cw, ch) = (window_rect.width, window_rect.height);
        let (sx, sy, sw, sh) = strip_region(cw, ch);

        let started = Instant::now();
        let capture = capture_window_region(sx, sy, sw, sh);
        let elapsed = started.elapsed();

        let CaptureBackendResult::Success { pixels, width, height, .. } = capture else {
            println!("[{}] capture failed", timestamp());
            std::thread::sleep(Duration::from_millis(args.poll_ms));
            continue;
        };

        let Some(frame) = RgbaImage::from_raw(width, height, pixels) else {
            std::thread::sleep(Duration::from_millis(args.poll_ms));
            continue;
        };

        let outcomes = match_frame_in(&frame, cw, ch, sx, refs, args);
        frames_this_draft += 1;

        let mut read = 0;
        for (i, o) in outcomes.iter().enumerate() {
            votes[i].frames_seen += 1;
            if o.contrast >= OCCUPIED_MIN_STDDEV {
                votes[i].occupied_frames += 1;
            }
            if let Some((hero, score, margin)) = o.best() {
                if margin >= args.margin_min {
                    *votes[i].tally.entry(hero.to_string()).or_insert(0) += 1;
                    votes[i].total_votes += 1;
                    votes[i].best_score = votes[i].best_score.max(score);
                    read += 1;
                }
            }
        }

        if let Some(dir) = &args.harvest {
            // Two harvest paths, opposite triggers: the arcana path fires on
            // the one slot nothing matches (labelled by GSI), the confirmed
            // path on slots everything agrees about (labelled by the matcher).
            // Both run *after* the vote update so the current frame counts
            // toward the agreement its own harvest decision requires.
            try_harvest(dir, &frame, &outcomes, &snapshot.own_hero, args, &mut harvested);
            try_harvest_confirmed(dir, &frame, &outcomes, &votes, &mut harvested);
        }

        println!(
            "[{}] frame {frames_this_draft:>3}  {read:>2}/10 slots read  ({}ms capture)",
            timestamp(),
            elapsed.as_millis()
        );

        std::thread::sleep(Duration::from_millis(args.poll_ms));
    }
}

/// Save the local player's slot as a new exemplar when its portrait is one the
/// CDN art cannot match — an arcana or persona.
///
/// Measured: across eight bot drafts, GSI's own hero appeared in the read
/// lineup 4/8 times. The four misses were Rubick, Lina, Shadow Fiend and
/// Earthshaker — every one an arcana the player owns. Their slots also read
/// with 1 vote against 2-3 everywhere else, because an unmatchable portrait
/// flickers between candidates instead of settling.
///
/// There is no CDN art to fix this with: the draft strip renders the arcana
/// from the cosmetic's model, and every guessable variant path 404s. But GSI
/// names our own hero every game, so the crop can be labelled without a guess.
///
/// The guard is deliberately strict, because a mislabelled exemplar is worse
/// than none — it would actively teach the matcher a wrong association:
///   - GSI must name a hero
///   - that hero must be absent from every confident read (so we are genuinely
///     failing to see it, rather than it sitting happily in another slot)
///   - exactly one slot may be occupied-but-unresolved (so there is no
///     ambiguity about which slot is ours)
fn try_harvest(
    dir: &Path,
    frame: &RgbaImage,
    outcomes: &[SlotOutcome],
    own_hero: &str,
    args: &Args,
    already: &mut std::collections::HashSet<String>,
) {
    let short = own_hero.trim_start_matches("npc_dota_hero_");
    if short.is_empty() || already.contains(short) {
        return;
    }

    let mut confident: Vec<&str> = Vec::new();
    let mut unresolved: Vec<&SlotOutcome> = Vec::new();
    for o in outcomes {
        match o.best() {
            Some((hero, _, margin)) if margin >= args.margin_min => confident.push(hero),
            _ if o.contrast >= OCCUPIED_MIN_STDDEV => unresolved.push(o),
            _ => {}
        }
    }

    // The original guard asked only for "exactly one unresolved slot", which a
    // half-rendered menu satisfies trivially — it harvested the Dota logo twice
    // and a battle-pass badge once before this was tightened. Requiring the
    // other nine slots to be confidently known means we only ever harvest from
    // a settled, fully-revealed lineup, where the single odd slot really is our
    // own unmatchable portrait and nothing else.
    const MIN_CONFIDENT_NEIGHBOURS: usize = 8;

    if confident.contains(&short)
        || unresolved.len() != 1
        || confident.len() < MIN_CONFIDENT_NEIGHBOURS
    {
        return;
    }

    let slot = unresolved[0];
    let Some(px) = crop(frame, &slot.slot) else {
        return;
    };
    let Some(img) = RgbaImage::from_raw(slot.slot.w, slot.slot.h, px) else {
        return;
    };

    if let Err(e) = std::fs::create_dir_all(dir) {
        println!("   harvest: cannot create {}: {e}", dir.display());
        return;
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = dir.join(format!("{short}__{stamp}.png"));
    match img.save(&path) {
        Ok(()) => {
            already.insert(short.to_string());
            println!(
                "   HARVESTED {short} ({} {}) -> {}",
                slot.slot.label,
                slot.slot.index + 1,
                path.display()
            );
        }
        Err(e) => println!("   harvest: cannot write {}: {e}", path.display()),
    }
}

/// Save a *confirmed* slot's crop as a same-domain exemplar for its hero.
///
/// This is the actual path to near-exact matching. CDN art tops out at 0.6-0.8
/// against live crops because of the domain gap (crop, grading, overlay), and
/// every near-miss we have measured lives in the margin that gap compresses. A
/// crop from a previous game matches the same portrait at ~0.95+, because the
/// strip renders a given hero identically every game. Harvesting each hero the
/// matcher confirms migrates the library into the capture domain one game at a
/// time, widening margins for every future draft — CDN art remains only the
/// bootstrap for heroes never yet seen.
///
/// The bar is far stricter than mere reporting, because a wrong exemplar
/// poisons every later game rather than one read:
///   - across all labelled data, no wrong read ever carried a margin above
///     0.038 — 0.06 sits clear of the entire observed failure band;
///   - the slot's accumulated votes must already agree besides, so a lucky
///     single frame cannot harvest;
///   - one exemplar per hero, ever: heroes whose refs already include a
///     same-domain exemplar are seeded into `already` at startup.
const HARVEST_MIN_SCORE: f32 = 0.65;
const HARVEST_MIN_MARGIN: f32 = 0.06;
const HARVEST_MIN_AGREEMENT: f32 = 0.8;

fn try_harvest_confirmed(
    dir: &Path,
    frame: &RgbaImage,
    outcomes: &[SlotOutcome],
    votes: &[SlotVotes],
    already: &mut std::collections::HashSet<String>,
) {
    for (i, o) in outcomes.iter().enumerate() {
        let Some((hero, score, margin)) = o.best() else {
            continue;
        };
        if score < HARVEST_MIN_SCORE || margin < HARVEST_MIN_MARGIN {
            continue;
        }
        let v = &votes[i];
        let settled = v.occupied_frames >= MIN_OCCUPIED_FRAMES
            && v.agreement() >= HARVEST_MIN_AGREEMENT
            && v.winner().is_some_and(|(w, _)| w == hero);
        if !settled || already.contains(hero) {
            continue;
        }

        let hero = hero.to_string();
        let Some(px) = crop(frame, &o.slot) else { continue };
        let Some(img) = RgbaImage::from_raw(o.slot.w, o.slot.h, px) else {
            continue;
        };
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = dir.join(format!("{hero}__{stamp}.png"));
        match img.save(&path) {
            Ok(()) => {
                println!(
                    "   HARVESTED {hero} (confirmed, score {score:.3} margin {margin:.3}) -> {}",
                    path.display()
                );
                already.insert(hero);
            }
            Err(e) => println!("   harvest: cannot write {}: {e}", path.display()),
        }
    }
}

fn report_votes(votes: &[SlotVotes], own_hero: &str, team_name: &str) {
    if votes.iter().all(|v| v.tally.is_empty()) {
        println!("   no slots resolved");
        return;
    }

    // Geometry only knows left and right. Radiant is always drawn left, so a
    // Dire player's own team is the right-hand group.
    let dire = team_name.eq_ignore_ascii_case("dire");
    let (left, right) = if dire {
        ("ENEMY", "ALLY ")
    } else {
        ("ALLY ", "ENEMY")
    };

    if !own_hero.is_empty() {
        let short = own_hero.trim_start_matches("npc_dota_hero_");
        let found = votes
            .iter()
            .any(|v| v.confident() && v.winner().is_some_and(|(h, _)| h == short));
        println!(
            "   own hero (GSI): {short}  [{}]  {}",
            if team_name.is_empty() { "team ?" } else { team_name },
            if found { "seen in lineup" } else { "NOT SEEN -- likely arcana/persona" }
        );
    }

    for (i, v) in votes.iter().enumerate() {
        let team = if i < SLOTS_PER_TEAM { left } else { right };
        let idx = i % SLOTS_PER_TEAM + 1;
        match v.winner() {
            Some((hero, n)) if v.confident() => println!(
                "   {team} {idx}  {hero:<24} {n}/{} frames, best {:.3}",
                v.frames_seen, v.best_score
            ),
            // Shown, but never as an answer: an unstable slot is the signature
            // of a portrait we have no exemplar for — someone else's arcana.
            Some((hero, n)) => println!(
                "   {team} {idx}  {:<24} UNKNOWN (best guess {hero}, agreed {n}/{} occupied frames)",
                "?", v.occupied_frames
            ),
            None => println!("   {team} {idx}  -"),
        }
    }
}

fn timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{:02}:{:02}:{:02}", (now / 3600) % 24, (now / 60) % 60, now % 60)
}

/// Bind the GSI port on a background thread and keep `gate` current.
fn spawn_gsi_listener(port: u16, gate: Arc<Mutex<Gate>>) {
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Cannot start GSI runtime: {e}");
                std::process::exit(1);
            }
        };

        runtime.block_on(async move {
            let app = axum::Router::new()
                .route("/", axum::routing::post(gsi_ingest))
                .with_state(gate);

            let listener = match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Cannot bind GSI port {port}: {e}");
                    eprintln!("The main app is probably holding it -- stop it and retry.");
                    std::process::exit(1);
                }
            };

            if let Err(e) = axum::serve(listener, app).await {
                eprintln!("GSI listener failed: {e}");
                std::process::exit(1);
            }
        });
    });
}

async fn gsi_ingest(
    axum::extract::State(gate): axum::extract::State<Arc<Mutex<Gate>>>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> &'static str {
    let str_at = |a: &str, b: &str| -> String {
        body.get(a)
            .and_then(|v| v.get(b))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    // matchid arrives as a string in some builds and a number in others.
    let matchid = body
        .get("map")
        .and_then(|m| m.get("matchid"))
        .map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string()))
        .unwrap_or_default();

    if let Ok(mut g) = gate.lock() {
        g.game_state = str_at("map", "game_state");
        g.matchid = matchid;
        let hero = str_at("hero", "name");
        // Empty during hero selection; keep the last real value for the summary.
        if !hero.is_empty() && hero != "empty" {
            g.own_hero = hero;
        }
        let team = str_at("player", "team_name");
        if !team.is_empty() {
            g.team_name = team;
        }
        g.payloads += 1;
    }

    "ok"
}

/// Rank every hero by how close its nearest neighbour in reference space is.
///
/// Live accuracy is currently 66/66, but that is over the ~14 distinct heroes
/// four bot games happened to draw. The untested risk is a *pair* of heroes that
/// collapse to the same descriptor at `dim`x`dim` — for those, the runner-up
/// crowds the winner and the margin gate fires (or worse, doesn't).
///
/// Reference-to-reference similarity is a proxy for that, not a measurement of
/// it: both sides here are CDN art, whereas live matching compares a capture to
/// CDN art. It cannot tell us a hero will be misread. What it *can* do is order
/// 127 heroes by risk so captures get spent on the dangerous end rather than on
/// whichever heroes the bot pool keeps re-drawing.
fn report_confusion(refs: &[(String, Vec<f32>, RgbaImage, bool)], args: &Args) {
    let n = refs.len();
    println!("Reference confusion analysis");
    println!("  heroes      : {n}");
    println!("  descriptor  : {dim}x{dim}x3", dim = args.dim);
    println!("  trim        : {:.2}", args.trim);
    println!();

    // Nearest neighbour per hero, plus every pair for the global ranking.
    let mut nearest: Vec<(f32, &str, &str)> = Vec::with_capacity(n);
    let mut pairs: Vec<(f32, &str, &str)> = Vec::with_capacity(n * (n - 1) / 2);

    for i in 0..n {
        let mut best = (f32::NEG_INFINITY, "");
        for j in 0..n {
            if i == j {
                continue;
            }
            let s = trimmed_similarity(&refs[i].1, &refs[j].1, args.trim);
            if j > i {
                pairs.push((s, refs[i].0.as_str(), refs[j].0.as_str()));
            }
            if s > best.0 {
                best = (s, refs[j].0.as_str());
            }
        }
        nearest.push((best.0, refs[i].0.as_str(), best.1));
    }

    let desc = |v: &mut Vec<(f32, &str, &str)>| {
        v.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    };
    desc(&mut pairs);
    desc(&mut nearest);

    println!("Closest pairs (both directions could be confused):");
    for (s, a, b) in pairs.iter().take(15) {
        println!("  {s:.4}  {a} <-> {b}");
    }
    println!();

    // Full list, not a top-N: the point of this mode is to look up where a given
    // hero sits, and the heroes already covered by captures are the ones whose
    // rank tells us how much the 66/66 result actually generalises.
    println!("All heroes by nearest-neighbour similarity (riskiest first):");
    for (rank, (s, hero, near)) in nearest.iter().enumerate() {
        println!("  {:>3}. {s:.4}  {hero:<24} nearest: {near}", rank + 1);
    }
    println!();

    let mean = nearest.iter().map(|x| x.0).sum::<f32>() / n as f32;
    let median = nearest[n / 2].0;
    println!(
        "Nearest-neighbour similarity: max {:.4}, mean {:.4}, median {:.4}, min {:.4}",
        nearest[0].0,
        mean,
        median,
        nearest[n - 1].0
    );
}

fn main() {
    let args = Args::parse();

    if !args.sweep && !args.live {
        println!("Draft Match Probe");
        println!("  capture     : {}", args.capture.display());
        println!("  references  : {}", args.refs.display());
        println!("  descriptor  : {dim}x{dim}x3", dim = args.dim);
        println!(
            "  ref framing : zoom {:.2}, y-offset {:+.2}",
            args.ref_zoom, args.ref_yoff
        );
        println!();
    }

    // --- ground truth -----------------------------------------------------
    // Labels are keyed by capture stem so one file covers every game, and the
    // heal step below needs them too: it can only replace a reference for a
    // hero whose slot is actually labelled.
    let capture_stem = args
        .capture
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let truth: TruthTable = match &args.truth {
        Some(p) => load_truth(p, &capture_stem),
        None => builtin_truth(),
    };

    // --- references -------------------------------------------------------
    let mut refs: Vec<(String, Vec<f32>, RgbaImage, bool)> = Vec::new();
    let entries = match std::fs::read_dir(&args.refs) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Cannot read {}: {e}", args.refs.display());
            eprintln!("Run scripts/fetch-hero-portraits.ps1 first.");
            std::process::exit(1);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("png") {
            continue;
        }
        // `lina.png` and `lina__1787771234.png` are both Lina. The suffix marks
        // an extra exemplar — an arcana or persona portrait harvested live —
        // and scoring takes the best over all of a hero's exemplars, so adding
        // one can only ever help that hero.
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
        let harvested = stem.contains("__");
        let name = stem.split("__").next().unwrap_or("?").to_string();

        match image::open(&path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (px, w, h) = if args.ref_zoom < 1.0 || args.ref_yoff != 0.0 {
                    reframe(&rgba, args.ref_zoom, args.ref_yoff, SLOT_ASPECT)
                } else {
                    (rgba.as_raw().clone(), rgba.width(), rgba.height())
                };
                if let Some(fp) = fingerprint_left(&px, w, h, args.dim, args.keep_left) {
                    refs.push((name, fp, rgba, harvested));
                }
            }
            Err(e) => eprintln!("  skipping {}: {e}", path.display()),
        }
    }

    if refs.is_empty() {
        eprintln!("No reference portraits loaded.");
        std::process::exit(1);
    }

    if args.confusion {
        report_confusion(&refs, &args);
        return;
    }

    if args.live {
        run_live(&args, &refs);
        return;
    }

    // --- self-healing references -----------------------------------------
    // Simulates what the runtime would do after confirming a hero: throw away
    // the CDN art and keep the real captured crop instead. Only the ground-truth
    // heroes are replaced, so the matcher still has to pick them out of the full
    // 127-candidate field rather than a shortlist.
    let mut healed = 0usize;
    if let Some(heal_path) = &args.heal {
        // Labels for the *heal* capture, which is a different game from the one
        // being matched — using the target's labels here would heal the wrong
        // heroes and quietly invalidate the whole test.
        let heal_stem = heal_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let heal_truth: TruthTable = match &args.truth {
            Some(p) => load_truth(p, &heal_stem),
            None => builtin_truth(),
        };

        match image::open(heal_path) {
            Ok(img) => {
                let src = img.to_rgba8();
                for slot in resolve_slots(src.width(), src.height(), args.top_inset) {
                    let Some(hero) = expected_in(&heal_truth, &slot) else {
                        continue;
                    };
                    let Some(px) = crop(&src, &slot) else { continue };
                    let Some(fp) = fingerprint_left(&px, slot.w, slot.h, args.dim, args.keep_left) else {
                        continue;
                    };
                    if let Some(entry) = refs.iter_mut().find(|(n, _, _, _)| n == hero) {
                        entry.1 = fp;
                        // A healed fingerprint is a same-domain crop, so it gets
                        // the same score handicap a harvested exemplar does.
                        entry.3 = true;
                        healed += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("Cannot open heal capture {}: {e}", heal_path.display());
                std::process::exit(1);
            }
        }
    }
    if !args.sweep {
        println!("Loaded {} hero references.", refs.len());
    }

    // --- capture ----------------------------------------------------------
    let frame = match image::open(&args.capture) {
        Ok(i) => i.to_rgba8(),
        Err(e) => {
            eprintln!("Cannot open capture {}: {e}", args.capture.display());
            std::process::exit(1);
        }
    };
    if !args.sweep {
        println!("Capture is {}x{}.", frame.width(), frame.height());
        println!();
    }

    // --- match ------------------------------------------------------------
    // A draft phase lasts far longer than one capture interval, so the runtime
    // gets many looks at the same pick. Voting across frames turns a per-frame
    // accuracy figure into a per-pick one: a transient misread is outvoted, and
    // only an error that repeats every frame survives.
    let mut frames: Vec<(String, RgbaImage)> = vec![(capture_stem.clone(), frame)];
    for extra in &args.vote {
        match image::open(extra) {
            Ok(i) => frames.push((
                extra
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string(),
                i.to_rgba8(),
            )),
            Err(e) => eprintln!("Cannot open vote frame {}: {e}", extra.display()),
        }
    }

    // slot key -> hero -> (votes, best score seen)
    let mut tally: Vec<(SlotRect, Vec<(String, usize, f32)>)> = Vec::new();
    let mut per_frame: Vec<Vec<SlotOutcome>> = Vec::new();

    for (_, img) in &frames {
        let outcomes = match_frame(img, &refs, &args);
        for outcome in &outcomes {
            if tally.len() < outcomes.len() {
                tally.push((outcome.slot, Vec::new()));
            }
            let Some((hero, score, margin)) = outcome.best() else {
                continue;
            };
            // Only confident reads get a vote. An uncertain frame abstains
            // rather than dragging the consensus toward its guess.
            if margin < args.margin_min {
                continue;
            }
            let entry = &mut tally[outcome.slot.index
                + if outcome.slot.label == "ENEMY" {
                    SLOTS_PER_TEAM
                } else {
                    0
                }]
            .1;
            match entry.iter_mut().find(|(h, _, _)| h == hero) {
                Some(e) => {
                    e.1 += 1;
                    e.2 = e.2.max(score);
                }
                None => entry.push((hero.to_string(), 1, score)),
            }
        }
        per_frame.push(outcomes);
    }

    let mut results: Vec<(SlotRect, Option<String>)> = Vec::new();
    let mut correct = 0usize;
    let mut known = 0usize;
    let mut correct_score_sum = 0f32;
    let mut min_correct_margin = f32::INFINITY;
    let mut max_wrong_margin = 0f32;

    if !args.sweep {
        println!(
            "{:<8} {:>5}  {:>8}  {:<22} {:>7}  {:<20} {:>7}  {:>6}  {}",
            "SLOT", "x", "contrast", "best match", "score", "runner-up", "margin", "votes", ""
        );
        println!("{}", "-".repeat(104));
    }

    let single = per_frame[0].len();
    for i in 0..single {
        let outcome = &per_frame[0][i];
        let slot = outcome.slot;
        let name = format!("{} {}", slot.label, slot.index + 1);

        // Consensus across frames, falling back to this frame when only one was
        // supplied or every frame abstained.
        let votes = &tally[i].1;
        let consensus = votes
            .iter()
            .max_by_key(|(_, count, _)| *count)
            .map(|(h, c, s)| (h.clone(), *c, *s));

        let (best, best_score, margin, vote_count) = match (&consensus, outcome.best()) {
            (Some((h, c, s)), Some((_, _, m))) => (h.clone(), *s, m, *c),
            (Some((h, c, s)), None) => (h.clone(), *s, 0.0, *c),
            (None, Some((h, s, m))) => (h.to_string(), s, m, 0),
            (None, None) => {
                if !args.sweep {
                    println!(
                        "{:<8} {:>5}  {:>8.1}  (no confident read)",
                        name, slot.x, outcome.contrast
                    );
                }
                results.push((slot, None));
                continue;
            }
        };

        let second = outcome
            .ranked
            .get(1)
            .map(|(h, _)| h.as_str())
            .unwrap_or("-");

        let verdict = match expected_in(&truth, &slot) {
            Some(expected) => {
                known += 1;
                if expected == best {
                    correct += 1;
                    correct_score_sum += best_score;
                    min_correct_margin = min_correct_margin.min(margin);
                    "OK"
                } else {
                    max_wrong_margin = max_wrong_margin.max(margin);
                    "WRONG"
                }
            }
            None => "",
        };

        if !args.sweep {
            println!(
                "{:<8} {:>5}  {:>8.1}  {:<22} {:>7.3}  {:<20} {:>7.3}  {:>3}/{:<2} {}",
                name,
                slot.x,
                outcome.contrast,
                best,
                best_score,
                second,
                margin,
                vote_count,
                frames.len(),
                verdict
            );

            if args.top > 2 {
                for (n, s) in outcome.ranked.iter().take(args.top).skip(2) {
                    println!("{:<8} {:>5}  {:>8}  {:<22} {:>7.3}", "", "", "", n, s);
                }
            }
        }

        results.push((slot, Some(best)));
    }

    // --- score ------------------------------------------------------------
    let mean_correct = if correct > 0 {
        correct_score_sum / correct as f32
    } else {
        0.0
    };
    // A gate exists only if every correct match outranks every wrong one by
    // margin. When that ordering holds, the gap between them is the window a
    // threshold can live in.
    let separated = correct > 0 && min_correct_margin > max_wrong_margin;

    if args.sweep {
        println!(
            "dim={:<3} inset={:.0}px healed={:<3} correct={}/{}  mean_score={:.3}  min_ok_margin={:.3}  max_bad_margin={:.3}  separated={}",
            args.dim,
            args.top_inset * 1080.0,
            healed,
            correct,
            known,
            mean_correct,
            if min_correct_margin.is_finite() {
                min_correct_margin
            } else {
                0.0
            },
            max_wrong_margin,
            if separated { "yes" } else { "NO" }
        );
        return;
    }

    println!();
    println!(
        "Known heroes: {correct}/{known} correct, mean score {mean_correct:.3} on the hits."
    );
    if min_correct_margin.is_finite() {
        println!(
            "Margin window: lowest correct {:.3} vs highest wrong {:.3} -> {}",
            min_correct_margin,
            max_wrong_margin,
            if separated {
                "a threshold cleanly separates them"
            } else {
                "NO clean threshold exists"
            }
        );
    }

    // --- visual verification ---------------------------------------------
    // Numbers alone cannot say whether a confident match is the *right* hero.
    // Pairing each crop with the art it matched makes that checkable at a glance.
    let occupied: Vec<_> = results.iter().filter(|(_, m)| m.is_some()).collect();
    if !occupied.is_empty() {
        let cw = per_frame[0][0].slot.w;
        let ch = per_frame[0][0].slot.h;
        let pad = 4u32;
        let mut sheet = RgbaImage::from_pixel(
            cw * 2 + pad * 3,
            (ch + pad) * occupied.len() as u32 + pad,
            image::Rgba([18, 18, 22, 255]),
        );

        for (row, (slot, matched)) in occupied.iter().enumerate() {
            let y = pad + row as u32 * (ch + pad);

            if let Some(px) = crop(&frames[0].1, slot) {
                if let Some(img) = RgbaImage::from_raw(slot.w, slot.h, px) {
                    let _ = sheet.copy_from(&img, pad, y);
                }
            }

            if let Some(name) = matched {
                if let Some((_, _, art, _)) = refs.iter().find(|(n, _, _, _)| n == name) {
                    let scaled =
                        imageops::resize(art, cw, ch, imageops::FilterType::CatmullRom);
                    let _ = sheet.copy_from(&scaled, cw + pad * 2, y);
                }
            }
        }

        match sheet.save(&args.montage) {
            Ok(()) => {
                println!();
                println!("Verification sheet: {}", args.montage.display());
                println!("  left column  = captured slot");
                println!("  right column = the hero it matched, in row order:");
                for (slot, matched) in &occupied {
                    println!(
                        "    {} {} -> {}",
                        slot.label,
                        slot.index + 1,
                        matched.as_deref().unwrap_or("-")
                    );
                }
            }
            Err(e) => eprintln!("Could not write montage: {e}"),
        }
    }
}

struct Args {
    capture: PathBuf,
    refs: PathBuf,
    montage: PathBuf,
    dim: usize,
    top: usize,
    ref_zoom: f32,
    ref_yoff: f32,
    /// Print one summary line instead of the full table — for parameter sweeps.
    sweep: bool,
    /// Replace known heroes' references with real crops from this capture.
    heal: Option<PathBuf>,
    /// Slot labels keyed by capture stem; falls back to the built-in table.
    truth: Option<PathBuf>,
    /// Fraction of each slot's width to fingerprint, measured from the left.
    keep_left: f32,
    /// Fraction of client height skipped at the top of each slot.
    top_inset: f32,
    /// Fraction of worst-agreeing descriptor cells ignored when scoring.
    trim: f32,
    /// Extra frames of the same draft to vote across.
    vote: Vec<PathBuf>,
    /// Minimum margin for a frame's read to count as a vote.
    margin_min: f32,
    /// Report the closest reference pairs instead of matching a capture.
    confusion: bool,
    /// Read the live game, gated on GSI, instead of scoring a saved capture.
    live: bool,
    /// Port the GSI listener binds in `--live`.
    port: u16,
    /// How often to capture while the draft gate is open.
    poll_ms: u64,
    /// Save unmatchable own-hero portraits here as extra exemplars.
    harvest: Option<PathBuf>,
    /// Score penalty applied to harvested exemplars, correcting their
    /// same-domain advantage over CDN art.
    harvest_penalty: f32,
}

impl Args {
    fn parse() -> Self {
        let mut a = Self {
            capture: PathBuf::from("logs/draft_capture/draft_1787767284_006.png"),
            refs: PathBuf::from(".cache/hero_portraits"),
            montage: PathBuf::from("logs/draft_capture/match_verification.png"),
            dim: 16,
            top: 3,
            ref_zoom: 1.0,
            ref_yoff: 0.0,
            sweep: false,
            heal: None,
            truth: None,
            keep_left: 1.0,
            // 6px at 1080p: the player-colour bar occupies rows 0-5 of every slot.
            // Measured, not guessed - see the module header.
            top_inset: 6.0 / 1080.0,
            // 0.15 measured best: it takes the steady-frame set from 63/65 to
            // 65/65 and lifts mean score 0.61 -> 0.78. Values from 0.10 to 0.30
            // all reach 65/65, so this sits mid-plateau rather than on an edge.
            trim: 0.15,
            vote: Vec::new(),
            // Below the lowest observed correct margin (0.016), so a confident
            // read is never excluded from the vote.
            margin_min: 0.01,
            confusion: false,
            live: false,
            // Matches the shipped gamestate_integration_dotaevents.cfg, so no
            // Dota-side change is needed. The main app must be stopped first.
            port: 3000,
            poll_ms: 1000,
            harvest: None,
            // Sized from the one observed collision: the LC arcana crop beat
            // Pugna's CDN art by 0.038 in Pugna's slot. 0.10 clears that with
            // room to spare while staying far below the gap an exemplar enjoys
            // in its own slot.
            harvest_penalty: 0.10,
        };

        let raw: Vec<String> = env::args().collect();
        let mut i = 1;
        while i < raw.len() {
            let need = |i: usize| -> String {
                raw.get(i).cloned().unwrap_or_else(|| {
                    eprintln!("Error: {} requires a value", raw[i - 1]);
                    std::process::exit(1);
                })
            };
            match raw[i].as_str() {
                "--capture" => {
                    i += 1;
                    a.capture = PathBuf::from(need(i));
                }
                "--refs" => {
                    i += 1;
                    a.refs = PathBuf::from(need(i));
                }
                "--montage" => {
                    i += 1;
                    a.montage = PathBuf::from(need(i));
                }
                "--dim" => {
                    i += 1;
                    a.dim = need(i).parse().unwrap_or(16).clamp(4, 64);
                }
                "--top" => {
                    i += 1;
                    a.top = need(i).parse().unwrap_or(3).clamp(1, 10);
                }
                "--ref-zoom" => {
                    i += 1;
                    a.ref_zoom = need(i).parse::<f32>().unwrap_or(1.0).clamp(0.2, 1.0);
                }
                "--ref-yoff" => {
                    i += 1;
                    a.ref_yoff = need(i).parse::<f32>().unwrap_or(0.0).clamp(-0.5, 0.5);
                }
                "--sweep" => a.sweep = true,
                "--confusion" => a.confusion = true,
                "--live" => a.live = true,
                "--port" => {
                    i += 1;
                    a.port = need(i).parse().unwrap_or(3000);
                }
                "--poll-ms" => {
                    i += 1;
                    a.poll_ms = need(i).parse().unwrap_or(1000).clamp(100, 10_000);
                }
                "--harvest" => {
                    i += 1;
                    a.harvest = Some(PathBuf::from(need(i)));
                }
                "--harvest-penalty" => {
                    i += 1;
                    a.harvest_penalty = need(i).parse::<f32>().unwrap_or(0.10).clamp(0.0, 0.5);
                }
                "--trim" => {
                    i += 1;
                    a.trim = need(i).parse::<f32>().unwrap_or(0.0).clamp(0.0, 0.5);
                }
                "--vote" => {
                    i += 1;
                    a.vote.push(PathBuf::from(need(i)));
                }
                "--margin-min" => {
                    i += 1;
                    a.margin_min = need(i).parse::<f32>().unwrap_or(0.0).clamp(0.0, 1.0);
                }
                "--inset-px" => {
                    // Given in 1080p pixels for legibility; stored as a fraction.
                    i += 1;
                    a.top_inset = need(i).parse::<f32>().unwrap_or(0.0).clamp(0.0, 32.0) / 1080.0;
                }
                "--keep-left" => {
                    i += 1;
                    a.keep_left = need(i).parse::<f32>().unwrap_or(1.0).clamp(0.2, 1.0);
                }
                "--truth" => {
                    i += 1;
                    a.truth = Some(PathBuf::from(need(i)));
                }
                "--heal" => {
                    i += 1;
                    a.heal = Some(PathBuf::from(need(i)));
                }
                "--help" | "-h" => {
                    println!("Usage: cargo run --example draft_match_probe [OPTIONS]");
                    println!("  --capture <PNG>  Capture to read (default: a known strategy frame)");
                    println!("  --refs <DIR>     Reference portraits (default: .cache/hero_portraits)");
                    println!("  --montage <PNG>  Verification sheet output");
                    println!("  --dim <N>        Descriptor edge length (default: 16)");
                    println!("  --top <N>        Candidates to print per slot (default: 3)");
                    std::process::exit(0);
                }
                other => {
                    eprintln!("Unknown argument: {other}");
                    std::process::exit(1);
                }
            }
            i += 1;
        }

        a
    }
}
