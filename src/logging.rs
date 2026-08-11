//! Tracing setup: console plus a log file on disk.
//!
//! The file half is what makes the app diagnosable at all. Both binaries used
//! to log to stdout only, which is invisible for the desktop build — it is a
//! GUI process with nowhere for stdout to go — so "turn on debug logging and
//! send me the output" had no answer short of running the headless binary from
//! a terminal.
//!
//! Files land next to the live config in `%LOCALAPPDATA%\dota2-scripts\logs\`,
//! for the same reason the config does: it is the one location that works
//! regardless of how the app was launched.

use std::path::PathBuf;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

use crate::config::storage::resolve_log_dir;

/// Base name for the rolling log. `tracing-appender` appends the date.
const LOG_FILE_PREFIX: &str = "dota2-scripts.log";

/// Keeps the non-blocking writer alive.
///
/// Dropping this flushes and stops the writer thread, so the caller has to hold
/// it for the lifetime of the process or the log file stops mid-run.
pub struct LoggingHandle {
    _guard: Option<WorkerGuard>,
    log_dir: Option<PathBuf>,
}

impl LoggingHandle {
    /// Where the log file is being written, if file logging came up.
    pub fn log_dir(&self) -> Option<&PathBuf> {
        self.log_dir.as_ref()
    }
}

/// Initialise tracing to stdout, and to a daily log file when `file_enabled`.
///
/// Falls back to stdout alone if the log directory cannot be created, rather
/// than failing to start — a missing log file must never cost the user the app.
pub fn init(level: &str, file_enabled: bool) -> LoggingHandle {
    if !file_enabled {
        tracing_subscriber::registry()
            .with(EnvFilter::new(level))
            .with(fmt::layer().with_writer(std::io::stdout))
            .init();

        return LoggingHandle {
            _guard: None,
            log_dir: None,
        };
    }

    let log_dir = resolve_log_dir();

    match std::fs::create_dir_all(&log_dir) {
        Ok(()) => {
            let file_appender = tracing_appender::rolling::daily(&log_dir, LOG_FILE_PREFIX);
            let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

            tracing_subscriber::registry()
                .with(EnvFilter::new(level))
                .with(fmt::layer().with_writer(std::io::stdout))
                // No ANSI in the file: escape codes make it unreadable in a
                // text editor, which is where it will actually be read.
                .with(fmt::layer().with_ansi(false).with_writer(file_writer))
                .init();

            LoggingHandle {
                _guard: Some(guard),
                log_dir: Some(log_dir),
            }
        }
        Err(e) => {
            tracing_subscriber::registry()
                .with(EnvFilter::new(level))
                .with(fmt::layer().with_writer(std::io::stdout))
                .init();

            tracing::warn!(
                "Could not create the log directory {}: {}. Logging to console only.",
                log_dir.display(),
                e
            );

            LoggingHandle {
                _guard: None,
                log_dir: None,
            }
        }
    }
}
