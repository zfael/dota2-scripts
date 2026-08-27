//! Background refresh of the cached STRATZ dataset.
//!
//! Loads the cache at startup and publishes it for the advice command to use.
//! When the cache is missing or stale it rebuilds in the background — a
//! minute of throttled requests — and republishes.
//!
//! A draft never waits on this. If the dataset is absent the Draft page says
//! so and identification carries on regardless; advice is an addition to the
//! reader, not a dependency of it.

use super::client::{StratzClient, StratzError};
use super::dataset::{self, StratzDataset};
use super::fetch::{build_dataset, Bracket};
use crate::config::Settings;
use crate::state::AppState;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

/// What the UI shows about the dataset behind the advice.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StratzStatusSnapshot {
    pub enabled: bool,
    /// A token is configured (from config or the environment).
    pub has_token: bool,
    /// A usable dataset is loaded.
    pub ready: bool,
    /// Refresh currently running, with progress 0-100.
    pub refreshing: bool,
    pub progress: u8,
    pub hero_count: usize,
    pub bracket: String,
    /// Unix seconds the loaded dataset was built.
    pub built_at: u64,
    /// Last failure, for showing the user why advice is missing.
    pub last_error: Option<String>,
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Blocking worker loop; spawn on a dedicated thread.
pub fn start_stratz_worker(settings: Arc<Mutex<Settings>>, app_state: Arc<Mutex<AppState>>) {
    // Remembers what we last tried, so a failing refresh does not spin.
    let mut last_attempt: Option<std::time::Instant> = None;
    let mut loaded_bracket = String::new();

    loop {
        let config = {
            let guard = settings.lock().unwrap();
            guard.stratz.clone()
        };

        if !config.enabled {
            publish(&app_state, |s| {
                *s = StratzStatusSnapshot { enabled: false, ..Default::default() };
            });
            std::thread::sleep(Duration::from_millis(1000));
            continue;
        }

        let token = StratzClient::resolve_token(&config.api_token);
        let cache_path = crate::config::storage::resolve_app_relative_path(&config.cache_path);
        let bracket = Bracket::from_name(&config.bracket);

        // --- load from disk if we have nothing in memory -------------------
        let have_dataset = {
            let state = app_state.lock().unwrap();
            state.stratz_dataset.is_some()
        };
        if !have_dataset && cache_path.exists() {
            match dataset::load(&cache_path) {
                Ok(d) => {
                    info!(
                        "STRATZ: loaded cached dataset ({} heroes, bracket {})",
                        d.len(),
                        d.bracket
                    );
                    loaded_bracket = d.bracket.clone();
                    publish_dataset(&app_state, d);
                }
                Err(e) => {
                    // A corrupt or outdated cache is something to rebuild,
                    // not something to keep reporting forever.
                    warn!("STRATZ: cached dataset unusable ({e}); will rebuild");
                    let _ = std::fs::remove_file(&cache_path);
                }
            }
        }

        // --- decide whether to refresh ------------------------------------
        let (needs_refresh, hero_count, built_at, bracket_name) = {
            let state = app_state.lock().unwrap();
            match &state.stratz_dataset {
                Some(d) => {
                    // A bracket change invalidates the data even if it is fresh.
                    let stale = !d.is_fresh(config.cache_ttl_hours, unix_now())
                        || loaded_bracket != bracket.basic;
                    (stale, d.len(), d.built_at, d.bracket.clone())
                }
                None => (true, 0, 0, String::new()),
            }
        };

        publish(&app_state, |s| {
            s.enabled = true;
            s.has_token = !token.is_empty();
            s.ready = hero_count > 0;
            s.hero_count = hero_count;
            s.bracket = bracket_name.clone();
            s.built_at = built_at;
        });

        if needs_refresh && !token.is_empty() {
            // Back off after a failure rather than hammering the API.
            let ready_to_retry = last_attempt.is_none_or(|t| t.elapsed() > Duration::from_secs(300));
            if ready_to_retry {
                last_attempt = Some(std::time::Instant::now());
                match refresh(&app_state, &token, &bracket) {
                    Ok(d) => {
                        info!("STRATZ: refreshed dataset ({} heroes)", d.len());
                        if let Err(e) = dataset::save(&d, &cache_path) {
                            warn!("STRATZ: cannot save dataset cache: {e}");
                        }
                        loaded_bracket = d.bracket.clone();
                        publish_dataset(&app_state, d);
                        publish(&app_state, |s| s.last_error = None);
                    }
                    Err(e) => {
                        // Never log the error's Debug — it can carry request
                        // details. Display is the sanitised form.
                        warn!("STRATZ: refresh failed: {e}");
                        publish(&app_state, |s| {
                            s.refreshing = false;
                            s.progress = 0;
                            s.last_error = Some(e.to_string());
                        });
                    }
                }
            }
        } else if token.is_empty() {
            publish(&app_state, |s| {
                s.last_error = Some(
                    "No STRATZ API token configured — set one to enable draft advice".to_string(),
                );
            });
        }

        std::thread::sleep(Duration::from_secs(30));
    }
}

fn refresh(
    app_state: &Arc<Mutex<AppState>>,
    token: &str,
    bracket: &Bracket,
) -> Result<StratzDataset, StratzError> {
    info!("STRATZ: building dataset (about a minute at the free rate limit)");
    publish(app_state, |s| {
        s.refreshing = true;
        s.progress = 0;
    });

    let mut client = StratzClient::new(token);
    let state_for_progress = Arc::clone(app_state);
    let mut on_progress = move |done: usize, total: usize, stage: &str| {
        let pct = if total == 0 {
            0
        } else {
            ((done as f32 / total as f32) * 100.0).clamp(0.0, 100.0) as u8
        };
        publish(&state_for_progress, |s| {
            s.refreshing = true;
            // Heroes and positions are a small prelude to the matchup pass;
            // showing their progress as the whole bar would stall at 100%.
            s.progress = if stage == "matchups" { pct } else { 0 };
        });
    };

    let result = build_dataset(&mut client, bracket, unix_now(), &mut on_progress);
    publish(app_state, |s| {
        s.refreshing = false;
        s.progress = 0;
    });
    result
}

fn publish_dataset(app_state: &Arc<Mutex<AppState>>, d: StratzDataset) {
    if let Ok(mut state) = app_state.lock() {
        state.stratz_dataset = Some(Arc::new(d));
    }
}

fn publish(app_state: &Arc<Mutex<AppState>>, edit: impl FnOnce(&mut StratzStatusSnapshot)) {
    if let Ok(mut state) = app_state.lock() {
        let mut snapshot = state.stratz_status.clone().unwrap_or_default();
        edit(&mut snapshot);
        state.stratz_status = Some(snapshot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_merges_into_the_existing_snapshot() {
        let state = AppState::new();
        publish(&state, |s| {
            s.enabled = true;
            s.hero_count = 127;
        });
        publish(&state, |s| s.refreshing = true);

        let snapshot = state.lock().unwrap().stratz_status.clone().unwrap();
        // The second publish must not wipe what the first set, or progress
        // updates would blank the hero count on every tick.
        assert!(snapshot.enabled);
        assert_eq!(snapshot.hero_count, 127);
        assert!(snapshot.refreshing);
    }

    #[test]
    fn publishing_a_dataset_makes_it_available_to_readers() {
        let state = AppState::new();
        publish_dataset(&state, dataset::sample_dataset(&["axe", "lina"]));
        let guard = state.lock().unwrap();
        assert_eq!(guard.stratz_dataset.as_ref().unwrap().len(), 2);
    }
}
