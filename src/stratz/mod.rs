//! STRATZ integration: turning the identified draft into pick advice.
//!
//! The split mirrors how the data actually behaves:
//!
//! - [`client`] talks to the API. Token-gated, rate-limited, secret-handling.
//! - [`fetch`] builds a dataset from it — ~132 requests, about a minute.
//! - [`dataset`] is that data at rest: four hero-by-hero matrices plus
//!   per-position win rates, cached on disk.
//! - [`advisor`] ranks a pick from the dataset. Pure, instant, offline.
//!
//! Matchup statistics are per-patch aggregates, so they are fetched once and
//! reused for every suggestion. That is what keeps the network out of the few
//! seconds where the user actually has to choose a hero.

// The headless binary compiles this module tree too, but runs only the
// refresh worker — ranking is consumed by the Tauri command layer, which
// links against the library. Without this, every ranking symbol is reported
// as dead code in the binary build. Both modules are covered by unit tests,
// so genuinely unreachable code still shows up as untested rather than
// hiding behind this allow.
#[allow(dead_code)]
pub mod advice;
#[allow(dead_code)]
pub mod advisor;
pub mod client;
pub mod dataset;
pub mod fetch;
pub mod worker;

// Only the handful of names used from outside the module are re-exported;
// the binary target compiles this module too, and unused re-exports there
// become build warnings.
pub use dataset::StratzDataset;
pub use worker::{start_stratz_worker, StratzStatusSnapshot};
