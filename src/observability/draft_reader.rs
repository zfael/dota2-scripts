//! Draft reader: GSI-gated live identification of the ten drafted heroes.
//!
//! Runs as a background worker (mirroring `minimap_capture`), off unless
//! `[draft] enabled = true`. The division of labour:
//!
//! - **GSI decides when to look.** The vision side cannot tell a draft screen
//!   from the main menu (it once read three heroes off menu icons), so capture
//!   happens only while `map.game_state` is a draft state — verified over
//!   eight enter/leave cycles with zero captures outside the gate.
//! - **`draft_vision` decides what it sees.** Geometry, descriptors, gates and
//!   vote aggregation, all measured in `examples/draft_match_probe.rs`.
//! - **This module owns the session.** A draft session opens when the gate
//!   opens, is scoped by `matchid` so votes never bleed across games, and
//!   closes when the gate closes, leaving the final lineup on display.
//!
//! Every session also produces evaluation data (`telemetry_dir`): a JSONL of
//! per-frame reads plus periodic strip PNGs, so ranked games become labelled
//! test material. UI feedback (right/wrong votes, corrections) is appended to
//! the same session directory.

use crate::config::Settings;
use crate::observability::draft_vision::{
    self, crop_slot, draft_gate_open, match_frame, slot_is_ally, strip_region, Reference,
    SlotOutcome, SlotVotes, MARGIN_MIN, OCCUPIED_MIN_STDDEV, TOTAL_SLOTS,
};
use crate::observability::minimap_capture_backend::{
    capture_window_region, find_dota2_window_rect, CaptureBackendResult,
};
use crate::state::AppState;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Harvest thresholds
// ---------------------------------------------------------------------------

/// A confirmed slot may be harvested only above this score and margin.
///
/// Far stricter than reporting: across all labelled data no wrong read ever
/// carried a margin above 0.038, so 0.06 sits clear of the entire observed
/// failure band. A wrong exemplar poisons every later game, not one read.
const HARVEST_MIN_SCORE: f32 = 0.65;
const HARVEST_MIN_MARGIN: f32 = 0.06;
const HARVEST_MIN_AGREEMENT: f32 = 0.8;

/// A hero that already owns exemplars may still harvest a new look while its
/// best read stays below this. A same-domain exemplar reads 0.85+ even after
/// [`draft_vision::HARVESTED_PENALTY`], so staying below means the current
/// exemplars are for a different look (plain Shadow Fiend read 0.74 forever
/// because the arcana exemplar owned the name). Self-limiting: one harvest
/// lifts the read above the ceiling.
const HARVEST_REFRESH_BELOW: f32 = 0.82;

/// Cap on exemplar files per hero, so the many-looks heroes (base + arcana
/// styles + persona) cannot grow without bound.
const MAX_EXEMPLARS_PER_HERO: usize = 4;

/// The arcana path may only fire when this many other slots are confidently
/// known. "Exactly one unresolved slot" alone is satisfied by a half-rendered
/// menu — the loose version of this guard harvested the Dota logo twice and a
/// battle-pass badge once before it was tightened.
const ARCANA_MIN_CONFIDENT_NEIGHBOURS: usize = 8;

// ---------------------------------------------------------------------------
// Snapshot published to the UI
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DraftSlotSnapshot {
    /// 0-9 in strip order (left team first).
    pub index: usize,
    pub is_ally: bool,
    /// The settled hero, present only when the vote is trustworthy.
    pub hero: Option<String>,
    /// Occupied but unresolvable — the signature of a portrait we have no
    /// exemplar for (someone else's arcana). Shown as "?" rather than a guess.
    pub unknown: bool,
    pub agreement: f32,
    pub best_score: f32,
    pub occupied_frames: u32,
}

#[derive(Debug, Clone, Default)]
pub struct DraftStatusSnapshot {
    pub enabled: bool,
    /// Gate open right now (draft on screen).
    pub active: bool,
    pub game_state: String,
    /// Identity of the current (or most recent) draft. Changes on every new
    /// draft; unlike `matchid`, which bot matches always report as `"0"`.
    /// The UI keys per-draft state (the user's slot verdicts) on this.
    pub session_id: String,
    pub matchid: String,
    pub team_name: String,
    pub own_hero: String,
    pub frames: u32,
    /// Current (or most recent) telemetry session directory; feedback from the
    /// UI is appended here, so it outlives the gate closing.
    pub session_dir: Option<String>,
    pub slots: Vec<DraftSlotSnapshot>,
}

fn build_slot_snapshots(votes: &[SlotVotes], team_name: &str) -> Vec<DraftSlotSnapshot> {
    votes
        .iter()
        .enumerate()
        .map(|(index, v)| {
            let confident = v.confident();
            DraftSlotSnapshot {
                index,
                is_ally: slot_is_ally(index, team_name),
                hero: if confident {
                    v.winner().map(|(h, _)| h.to_string())
                } else {
                    None
                },
                unknown: !confident && v.occupied_frames >= draft_vision::MIN_OCCUPIED_FRAMES,
                agreement: v.agreement(),
                best_score: v.best_score,
                occupied_frames: v.occupied_frames,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Harvest candidate selection (pure, so the guards are testable)
// ---------------------------------------------------------------------------

/// The one slot that must be our own unmatchable portrait, if any.
///
/// Requires: GSI names a hero; that hero is absent from every confident read;
/// exactly one slot is occupied-but-unresolved; and at least
/// [`ARCANA_MIN_CONFIDENT_NEIGHBOURS`] slots are confidently read, so a
/// half-rendered screen can never qualify.
fn arcana_harvest_candidate(outcomes: &[SlotOutcome], own_short: &str) -> Option<usize> {
    if own_short.is_empty() {
        return None;
    }

    let mut confident: Vec<&str> = Vec::new();
    let mut unresolved: Vec<usize> = Vec::new();
    for (i, o) in outcomes.iter().enumerate() {
        match o.best() {
            Some((hero, _, margin)) if margin >= MARGIN_MIN => confident.push(hero),
            _ if o.contrast >= OCCUPIED_MIN_STDDEV => unresolved.push(i),
            _ => {}
        }
    }

    if confident.contains(&own_short)
        || unresolved.len() != 1
        || confident.len() < ARCANA_MIN_CONFIDENT_NEIGHBOURS
    {
        return None;
    }
    Some(unresolved[0])
}

/// Slots whose settled, high-margin read qualifies as a same-domain exemplar.
///
/// This is the actual path to near-exact matching: CDN art tops out at 0.6-0.8
/// against live crops, a previous game's crop at ~0.95+. Harvesting confirmed
/// heroes migrates the library into the capture domain one game at a time.
fn confirmed_harvest_candidates(
    outcomes: &[SlotOutcome],
    votes: &[SlotVotes],
    already: &std::collections::HashSet<String>,
) -> Vec<(usize, String)> {
    outcomes
        .iter()
        .enumerate()
        .filter_map(|(i, o)| {
            let (hero, score, margin) = o.best()?;
            if score < HARVEST_MIN_SCORE || margin < HARVEST_MIN_MARGIN {
                return None;
            }
            let v = &votes[i];
            let settled = v.occupied_frames >= draft_vision::MIN_OCCUPIED_FRAMES
                && v.agreement() >= HARVEST_MIN_AGREEMENT
                && v.winner().is_some_and(|(w, _)| w == hero);
            if settled && (!already.contains(hero) || score < HARVEST_REFRESH_BELOW) {
                Some((i, hero.to_string()))
            } else {
                None
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Worker
// ---------------------------------------------------------------------------

/// One draft session's mutable state.
struct Session {
    /// Identity of *this* draft, unique per session.
    ///
    /// Not `matchid`: bot matches report `matchid` as `"0"` for every game, so
    /// anything keyed on it (the UI's per-slot verdicts, for one) never
    /// noticed a new draft starting.
    id: String,
    matchid: String,
    votes: Vec<SlotVotes>,
    frames: u32,
    own_hero: String,
    team_name: String,
    dir: Option<PathBuf>,
    /// The arcana path fires at most once per session.
    arcana_harvested: bool,
    /// Heroes already harvested in this session. Without it the refresh rule
    /// can re-fire every frame: a fresh exemplar that lands just under
    /// [`HARVEST_REFRESH_BELOW`] still qualifies as "reading badly".
    harvested_this_session: std::collections::HashSet<String>,
}

impl Session {
    fn new(id: String, matchid: String, dir: Option<PathBuf>) -> Self {
        Self {
            id,
            matchid,
            votes: (0..TOTAL_SLOTS).map(|_| SlotVotes::default()).collect(),
            frames: 0,
            own_hero: String::new(),
            team_name: String::new(),
            dir,
            arcana_harvested: false,
            harvested_this_session: std::collections::HashSet::new(),
        }
    }
}

/// Existing exemplar files for one hero (`hero__*.png`).
fn exemplar_count(dir: &Path, hero: &str) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let prefix = format!("{hero}__");
    entries
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with(&prefix) && n.ends_with(".png"))
        })
        .count()
}

/// Take the slot corrections queued by the UI's ✗ votes.
fn drain_corrections(app_state: &Arc<Mutex<AppState>>) -> Vec<(usize, String)> {
    app_state
        .lock()
        .map(|mut s| std::mem::take(&mut s.draft_corrections))
        .unwrap_or_default()
}

/// Load the shipped pack plus any locally harvested exemplars.
fn load_references(exemplar_dir: &Path) -> Vec<Reference> {
    let mut refs = match draft_vision::builtin_references() {
        Ok(r) => r,
        Err(e) => {
            // A broken embedded pack is a build defect; run without references
            // rather than crash the whole app over the draft helper.
            warn!("Draft reference pack failed to load: {e}");
            Vec::new()
        }
    };

    let Ok(entries) = std::fs::read_dir(exemplar_dir) else {
        return refs; // No exemplars harvested yet.
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("png") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
        let name = stem.split("__").next().unwrap_or("?").to_string();
        if let Ok(img) = image::open(&path) {
            let rgba = img.to_rgba8();
            if let Some(fp) =
                draft_vision::fingerprint(rgba.as_raw(), rgba.width(), rgba.height())
            {
                refs.push(Reference {
                    name,
                    fingerprint: fp,
                    harvested: true,
                });
            }
        }
    }
    refs
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn append_jsonl(dir: &Path, file: &str, line: &serde_json::Value) {
    let path = dir.join(file);
    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut f) => {
            let _ = writeln!(f, "{line}");
        }
        Err(e) => warn!("Draft telemetry: cannot append {}: {e}", path.display()),
    }
}

/// Blocking worker loop; spawn on a dedicated thread.
pub fn start_draft_reader_worker(
    settings: Arc<Mutex<Settings>>,
    app_state: Arc<Mutex<AppState>>,
) {
    let mut refs: Vec<Reference> = Vec::new();
    let mut session: Option<Session> = None;
    let mut harvested: std::collections::HashSet<String> = std::collections::HashSet::new();

    loop {
        let config = {
            let guard = settings.lock().unwrap();
            guard.draft.clone()
        };

        if !config.enabled {
            if session.take().is_some() {
                info!("Draft reader: disabled mid-session, closing");
            }
            // Publish "off" so the UI can say so, then idle cheaply.
            if let Ok(mut state) = app_state.lock() {
                if state.draft.as_ref().is_none_or(|d| d.enabled) {
                    state.draft = Some(DraftStatusSnapshot {
                        enabled: false,
                        ..Default::default()
                    });
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
            continue;
        }

        // --- gate, from the last GSI event -------------------------------
        let (game_state, matchid, team_name, gsi_hero) = {
            let state = app_state.lock().unwrap();
            // A stale event must not hold the gate open: if Dota exits
            // mid-draft, game_state would otherwise stay HERO_SELECTION
            // forever and the worker would capture whatever replaces it.
            if !state.has_recent_gsi_activity() {
                (String::new(), String::new(), String::new(), String::new())
            } else {
                match &state.last_event {
                    Some(e) => (
                        e.map.game_state.clone(),
                        e.map.matchid.clone(),
                        e.player
                            .as_ref()
                            .and_then(|p| p.team_name.clone())
                            .unwrap_or_default(),
                        e.hero.name.clone(),
                    ),
                    None => (String::new(), String::new(), String::new(), String::new()),
                }
            }
        };
        let open = draft_gate_open(&game_state);

        // --- session transitions ------------------------------------------
        if open {
            let stale = session
                .as_ref()
                .is_some_and(|s| !matchid.is_empty() && s.matchid != matchid);
            if stale {
                info!("Draft reader: match changed, resetting session");
                close_session(&mut session, &app_state);
            }
            if session.is_none() {
                let exemplar_dir =
                    crate::config::storage::resolve_app_relative_path(&config.exemplar_dir);
                refs = load_references(&exemplar_dir);
                harvested = refs
                    .iter()
                    .filter(|r| r.harvested && !r.is_negative())
                    .map(|r| r.name.clone())
                    .collect();

                // Minted here, not derived from matchid, so it is unique per
                // draft even when Dota reports no usable match id.
                let session_id = format!(
                    "{}_{}",
                    unix_now(),
                    if matchid.is_empty() { "unknown" } else { &matchid }
                );

                let dir = if config.telemetry_enabled {
                    let base =
                        crate::config::storage::resolve_app_relative_path(&config.telemetry_dir);
                    let dir = base.join(&session_id);
                    match std::fs::create_dir_all(&dir) {
                        Ok(()) => Some(dir),
                        Err(e) => {
                            warn!("Draft telemetry: cannot create {}: {e}", dir.display());
                            None
                        }
                    }
                } else {
                    None
                };
                info!(
                    "Draft reader: session open (match {}, {} refs)",
                    if matchid.is_empty() { "?" } else { &matchid },
                    refs.len()
                );
                // Corrections queued for a previous session must not label
                // this one's crops.
                let _ = drain_corrections(&app_state);
                session = Some(Session::new(session_id, matchid.clone(), dir));
            }
        } else if session.is_some() {
            info!("Draft reader: gate closed, finalising session");
            close_session(&mut session, &app_state);
        }

        let Some(s) = session.as_mut() else {
            publish_idle(&app_state, &game_state);
            std::thread::sleep(std::time::Duration::from_millis(config.poll_ms));
            continue;
        };

        // GSI's own hero and team are empty during selection and fill in
        // later; keep the last real values for the whole session.
        if !gsi_hero.is_empty() && gsi_hero != "empty" {
            s.own_hero = gsi_hero.clone();
        }
        if !team_name.is_empty() {
            s.team_name = team_name.clone();
        }

        // --- capture -------------------------------------------------------
        let CaptureBackendResult::Success { window_rect, .. } = find_dota2_window_rect() else {
            std::thread::sleep(std::time::Duration::from_millis(config.poll_ms));
            continue;
        };
        let (cw, ch) = (window_rect.width, window_rect.height);
        let (sx, sy, sw, sh) = strip_region(cw, ch);
        let CaptureBackendResult::Success {
            pixels,
            width,
            height,
            ..
        } = capture_window_region(sx, sy, sw, sh)
        else {
            std::thread::sleep(std::time::Duration::from_millis(config.poll_ms));
            continue;
        };
        let Some(frame) = image::RgbaImage::from_raw(width, height, pixels) else {
            std::thread::sleep(std::time::Duration::from_millis(config.poll_ms));
            continue;
        };

        // --- match, vote, record ------------------------------------------
        let outcomes = match_frame(&frame, cw, ch, sx, &refs);
        s.frames += 1;
        for (i, o) in outcomes.iter().enumerate() {
            s.votes[i].observe(o);
        }

        if let Some(dir) = s.dir.clone() {
            let slots: Vec<serde_json::Value> = outcomes
                .iter()
                .map(|o| {
                    let best = o.best();
                    serde_json::json!({
                        "contrast": o.contrast,
                        "hero": best.map(|(h, _, _)| h),
                        "score": best.map(|(_, s, _)| s),
                        "margin": best.map(|(_, _, m)| m),
                    })
                })
                .collect();
            append_jsonl(
                &dir,
                "session.jsonl",
                &serde_json::json!({
                    "ts": unix_now(),
                    "frame": s.frames,
                    "game_state": game_state,
                    "own_hero": s.own_hero,
                    "team": s.team_name,
                    "client": [cw, ch],
                    "slots": slots,
                }),
            );
            if s.frames % config.telemetry_save_every_n.max(1) == 1 {
                let path = dir.join(format!("frame_{:04}.png", s.frames));
                if let Err(e) = frame.save(&path) {
                    warn!("Draft telemetry: cannot save {}: {e}", path.display());
                }
            }
        }

        // --- harvest -------------------------------------------------------
        let exemplar_dir =
            crate::config::storage::resolve_app_relative_path(&config.exemplar_dir);
        let own_short = s.own_hero.trim_start_matches("npc_dota_hero_").to_string();

        // (slot, hero, reset_votes)
        let mut to_save: Vec<(usize, String, bool)> = Vec::new();
        if config.harvest_enabled {
            // Deliberately not gated on the hero already owning exemplars:
            // the arcana guards only pass when every existing exemplar failed
            // to match, which is exactly the new-look case (Rubick's arcana
            // went unharvested for a whole session because a stale exemplar
            // of a different look blocked it by name).
            if !s.arcana_harvested {
                if let Some(i) = arcana_harvest_candidate(&outcomes, &own_short) {
                    s.arcana_harvested = true;
                    if exemplar_count(&exemplar_dir, &own_short) < MAX_EXEMPLARS_PER_HERO {
                        to_save.push((i, own_short.clone(), true));
                    }
                }
            }
            to_save.extend(
                confirmed_harvest_candidates(&outcomes, &s.votes, &harvested)
                    .into_iter()
                    .filter(|(_, h)| {
                        !s.harvested_this_session.contains(h)
                            && exemplar_count(&exemplar_dir, h) < MAX_EXEMPLARS_PER_HERO
                    })
                    .map(|(i, h)| (i, h, false)),
            );
        }

        // A ✗ correction from the UI is an explicit label for what is on
        // screen right now — harvest it even with automatic harvesting off.
        for (i, hero) in drain_corrections(&app_state) {
            if i >= outcomes.len() {
                continue;
            }
            if !refs.iter().any(|r| !r.harvested && r.name == hero) {
                warn!("Draft correction: unknown hero '{hero}', not harvesting");
                continue;
            }
            if outcomes[i].contrast < OCCUPIED_MIN_STDDEV {
                warn!("Draft correction: slot {i} reads empty, not harvesting");
                continue;
            }
            if exemplar_count(&exemplar_dir, &hero) < MAX_EXEMPLARS_PER_HERO {
                to_save.push((i, hero, true));
            }
        }

        for (i, hero, reset_votes) in to_save {
            let Some(img) = crop_slot(&frame, &outcomes[i].slot) else {
                continue;
            };
            if std::fs::create_dir_all(&exemplar_dir).is_err() {
                break;
            }
            let path = exemplar_dir.join(format!("{hero}__{}.png", unix_now()));
            match img.save(&path) {
                Ok(()) => {
                    info!("Draft reader: harvested {hero} -> {}", path.display());
                    harvested.insert(hero.clone());
                    s.harvested_this_session.insert(hero.clone());
                    // Fold the exemplar into the live reference set — without
                    // this, Snapfire's arcana was harvested on frame 3 and
                    // the slot still read "?" for the remaining 28 frames.
                    if let Some(fp) =
                        draft_vision::fingerprint(img.as_raw(), img.width(), img.height())
                    {
                        refs.push(Reference {
                            name: hero,
                            fingerprint: fp,
                            harvested: true,
                        });
                    }
                    if reset_votes {
                        // The old votes were cast by a library that
                        // demonstrably could not read this slot.
                        s.votes[i] = SlotVotes::default();
                    }
                }
                Err(e) => warn!("Draft harvest: cannot save {}: {e}", path.display()),
            }
        }

        // --- publish -------------------------------------------------------
        if let Ok(mut state) = app_state.lock() {
            state.draft = Some(DraftStatusSnapshot {
                enabled: true,
                active: true,
                game_state: game_state.clone(),
                session_id: s.id.clone(),
                matchid: s.matchid.clone(),
                team_name: s.team_name.clone(),
                own_hero: s.own_hero.clone(),
                frames: s.frames,
                session_dir: s.dir.as_ref().map(|d| d.display().to_string()),
                slots: build_slot_snapshots(&s.votes, &s.team_name),
            });
        }

        std::thread::sleep(std::time::Duration::from_millis(config.poll_ms));
    }
}

/// Finalise: write the session summary, publish the closing snapshot with the
/// final lineup left on display, drop the session.
fn close_session(session: &mut Option<Session>, app_state: &Arc<Mutex<AppState>>) {
    let Some(s) = session.take() else { return };

    if let Some(dir) = &s.dir {
        let lineup: Vec<serde_json::Value> = build_slot_snapshots(&s.votes, &s.team_name)
            .iter()
            .map(|slot| {
                serde_json::json!({
                    "index": slot.index,
                    "is_ally": slot.is_ally,
                    "hero": slot.hero,
                    "unknown": slot.unknown,
                    "agreement": slot.agreement,
                    "best_score": slot.best_score,
                    // How many frames the slot held a portrait at all. A real
                    // dark portrait persists for 13-28 frames; a reveal fade
                    // brushes the occupancy gate for exactly one.
                    "occupied_frames": slot.occupied_frames,
                })
            })
            .collect();
        append_jsonl(
            dir,
            "session.jsonl",
            &serde_json::json!({
                "ts": unix_now(),
                "final": true,
                "frames": s.frames,
                "own_hero": s.own_hero,
                "team": s.team_name,
                "lineup": lineup,
            }),
        );
    }

    if let Ok(mut state) = app_state.lock() {
        state.draft = Some(DraftStatusSnapshot {
            enabled: true,
            active: false,
            game_state: String::new(),
            session_id: s.id,
            matchid: s.matchid,
            team_name: s.team_name.clone(),
            own_hero: s.own_hero,
            frames: s.frames,
            session_dir: s.dir.as_ref().map(|d| d.display().to_string()),
            slots: build_slot_snapshots(&s.votes, &s.team_name),
        });
    }
}

fn publish_idle(app_state: &Arc<Mutex<AppState>>, game_state: &str) {
    if let Ok(mut state) = app_state.lock() {
        let keep = state.draft.take();
        state.draft = Some(match keep {
            // Keep the last finished lineup on display between drafts; only
            // refresh the live fields.
            Some(mut d) if d.enabled => {
                d.active = false;
                d.game_state = game_state.to_string();
                d
            }
            _ => DraftStatusSnapshot {
                enabled: true,
                game_state: game_state.to_string(),
                ..Default::default()
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::draft_vision::resolve_slots;

    fn outcome(index: usize, hero: Option<(&str, f32, f32)>, contrast: f32) -> SlotOutcome {
        SlotOutcome {
            slot: resolve_slots(1920, 1080)[index],
            contrast,
            ranked: match hero {
                Some((h, score, margin)) => {
                    vec![(h.to_string(), score), ("runner".to_string(), score - margin)]
                }
                None => Vec::new(),
            },
        }
    }

    /// Nine confident slots plus one occupied-but-unread slot at `hole`.
    fn nine_plus_hole(hole: usize) -> Vec<SlotOutcome> {
        (0..TOTAL_SLOTS)
            .map(|i| {
                if i == hole {
                    outcome(i, None, 45.0)
                } else {
                    outcome(i, Some((["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"][i], 0.8, 0.1)), 50.0)
                }
            })
            .collect()
    }

    #[test]
    fn arcana_candidate_is_the_single_unresolved_slot() {
        let outcomes = nine_plus_hole(3);
        assert_eq!(arcana_harvest_candidate(&outcomes, "nevermore"), Some(3));
    }

    #[test]
    fn arcana_candidate_rejects_half_rendered_screens() {
        // Only two slots read, one unresolved — the exact shape of the menu
        // frames that harvested the Dota logo before the guard was tightened.
        let mut outcomes: Vec<SlotOutcome> =
            (0..TOTAL_SLOTS).map(|i| outcome(i, None, 10.0)).collect();
        outcomes[0] = outcome(0, Some(("axe", 0.8, 0.1)), 50.0);
        outcomes[1] = outcome(1, Some(("lina", 0.8, 0.1)), 50.0);
        outcomes[2] = outcome(2, None, 45.0);
        assert_eq!(arcana_harvest_candidate(&outcomes, "nevermore"), None);
    }

    #[test]
    fn arcana_candidate_rejects_when_own_hero_already_read() {
        // Our hero visible elsewhere means the hole is NOT our portrait.
        let mut outcomes = nine_plus_hole(3);
        outcomes[0] = outcome(0, Some(("nevermore", 0.8, 0.1)), 50.0);
        assert_eq!(arcana_harvest_candidate(&outcomes, "nevermore"), None);
    }

    fn settled_votes(hero: &str, frames: u32) -> SlotVotes {
        let mut v = SlotVotes::default();
        for _ in 0..frames {
            v.observe(&outcome(0, Some((hero, 0.8, 0.1)), 50.0));
        }
        v
    }

    #[test]
    fn confirmed_harvest_requires_margin_above_the_failure_band() {
        let votes: Vec<SlotVotes> = (0..TOTAL_SLOTS).map(|_| settled_votes("axe", 5)).collect();
        let already = std::collections::HashSet::new();

        // Margin 0.038 was the worst observed steal — must NOT harvest.
        let low: Vec<SlotOutcome> = (0..TOTAL_SLOTS)
            .map(|i| outcome(i, Some(("axe", 0.8, 0.038)), 50.0))
            .collect();
        assert!(confirmed_harvest_candidates(&low, &votes, &already).is_empty());

        // Above the band, with settled votes agreeing: harvest.
        let high: Vec<SlotOutcome> = (0..TOTAL_SLOTS)
            .map(|i| outcome(i, Some(("axe", 0.8, 0.2)), 50.0))
            .collect();
        let picked = confirmed_harvest_candidates(&high, &votes, &already);
        assert!(!picked.is_empty());
        assert!(picked.iter().all(|(_, h)| h == "axe"));
    }

    #[test]
    fn exemplar_count_counts_only_that_heros_files() {
        let dir = std::env::temp_dir().join(format!("draft_exemplar_count_{}", unix_now()));
        std::fs::create_dir_all(&dir).unwrap();
        for name in [
            "axe__1.png",
            "axe__2.png",
            "axe_rider__1.png", // different hero, shares a prefix
            "lina__1.png",
            "axe__3.txt", // not a png
        ] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }

        assert_eq!(exemplar_count(&dir, "axe"), 2);
        assert_eq!(exemplar_count(&dir, "lina"), 1);
        assert_eq!(exemplar_count(&dir, "sven"), 0);
        // A missing directory is simply "none harvested yet".
        assert_eq!(exemplar_count(&dir.join("nope"), "axe"), 0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn confirmed_harvest_refreshes_owned_heroes_only_below_the_ceiling() {
        let votes: Vec<SlotVotes> = (0..TOTAL_SLOTS).map(|_| settled_votes("axe", 5)).collect();
        let already: std::collections::HashSet<String> = ["axe".to_string()].into();

        // Reading through a same-domain exemplar (0.85+ post-penalty): an
        // owned hero is left alone.
        let same_domain: Vec<SlotOutcome> = (0..TOTAL_SLOTS)
            .map(|i| outcome(i, Some(("axe", 0.85, 0.2)), 50.0))
            .collect();
        assert!(confirmed_harvest_candidates(&same_domain, &votes, &already).is_empty());

        // A confident read stuck at cross-domain scores means the owned
        // exemplars are for a *different look* (plain Shadow Fiend was locked
        // out of the name by the arcana exemplar): harvest the new look.
        let new_look: Vec<SlotOutcome> = (0..TOTAL_SLOTS)
            .map(|i| outcome(i, Some(("axe", 0.8, 0.2)), 50.0))
            .collect();
        let picked = confirmed_harvest_candidates(&new_look, &votes, &already);
        assert!(!picked.is_empty());
    }

    #[test]
    fn confirmed_harvest_requires_votes_to_agree_with_the_frame() {
        // Frame says lina, accumulated votes say axe: refuse.
        let votes: Vec<SlotVotes> = (0..TOTAL_SLOTS).map(|_| settled_votes("axe", 5)).collect();
        let outcomes: Vec<SlotOutcome> = (0..TOTAL_SLOTS)
            .map(|i| outcome(i, Some(("lina", 0.8, 0.2)), 50.0))
            .collect();
        let already = std::collections::HashSet::new();
        assert!(confirmed_harvest_candidates(&outcomes, &votes, &already).is_empty());
    }

    #[test]
    fn slot_snapshots_flip_sides_for_dire_and_flag_unknowns() {
        let mut votes: Vec<SlotVotes> = (0..TOTAL_SLOTS).map(|_| SlotVotes::default()).collect();
        votes[0] = settled_votes("axe", 5);
        // Slot 1: occupied every frame but scattered — the arcana signature.
        for i in 0..5u32 {
            votes[1].observe(&outcome(1, Some((&format!("g{i}"), 0.6, 0.02), ), 50.0));
        }

        let radiant = build_slot_snapshots(&votes, "radiant");
        assert_eq!(radiant[0].hero.as_deref(), Some("axe"));
        assert!(radiant[0].is_ally && !radiant[5].is_ally);
        assert!(radiant[1].unknown && radiant[1].hero.is_none());
        // Untouched slots are neither known nor unknown — just empty.
        assert!(!radiant[9].unknown && radiant[9].hero.is_none());

        let dire = build_slot_snapshots(&votes, "dire");
        assert!(!dire[0].is_ally && dire[5].is_ally);
    }
}
