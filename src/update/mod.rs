//! Auto-update module for checking and applying updates from GitHub Releases.
//!
//! Uses the `self_update` crate to:
//! 1. Check for newer versions on GitHub Releases
//! 2. Download and replace the executable
//! 3. Restart the application

use self_update::backends::github::ReleaseList;
use self_update::cargo_crate_version;
use self_update::version::bump_is_greater;
use std::process::Command;
use tracing::{error, info, warn};

/// GitHub repository owner
const REPO_OWNER: &str = "zfael";
/// GitHub repository name
const REPO_NAME: &str = "dota2-scripts";

/// Information about an available update
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub release_notes: Option<String>,
}

/// Result of checking for updates
#[derive(Debug)]
pub enum UpdateCheckResult {
    /// A new version is available
    Available(UpdateInfo),
    /// Already running the latest version
    UpToDate,
    /// Error occurred during check
    Error(String),
}

/// Check for available updates on GitHub Releases.
///
/// # Arguments
/// * `include_prereleases` - If true, include RC/alpha/beta versions
///
/// # Returns
/// `UpdateCheckResult` indicating whether an update is available
pub fn check_for_update(include_prereleases: bool) -> UpdateCheckResult {
    let current_version = cargo_crate_version!();
    info!(
        "🔄 Checking for updates... (current: v{})",
        current_version
    );

    // Fetch releases from GitHub
    let releases = match ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
    {
        Ok(list) => match list.fetch() {
            Ok(releases) => releases,
            Err(e) => {
                let msg = format!("Failed to fetch releases: {}", e);
                warn!("{}", msg);
                return UpdateCheckResult::Error(msg);
            }
        },
        Err(e) => {
            let msg = format!("Failed to configure release list: {}", e);
            error!("{}", msg);
            return UpdateCheckResult::Error(msg);
        }
    };

    if releases.is_empty() {
        info!("No releases found on GitHub");
        return UpdateCheckResult::UpToDate;
    }

    // Filter releases based on prerelease preference
    let filtered_releases: Vec<_> = if include_prereleases {
        releases
    } else {
        releases
            .into_iter()
            .filter(|r| !is_prerelease(&r.version))
            .collect()
    };

    if filtered_releases.is_empty() {
        info!("No stable releases found (prereleases excluded)");
        return UpdateCheckResult::UpToDate;
    }

    // Check if the latest release is newer than current version
    let latest = &filtered_releases[0];
    let latest_version = latest.version.trim_start_matches('v');

    match bump_is_greater(current_version, latest_version) {
        Ok(true) => {
            info!(
                "✨ Update available: v{} -> v{}",
                current_version, latest_version
            );
            UpdateCheckResult::Available(UpdateInfo {
                version: latest_version.to_string(),
                release_notes: latest.body.clone(),
            })
        }
        Ok(false) => {
            info!("✅ Already running the latest version (v{})", current_version);
            UpdateCheckResult::UpToDate
        }
        Err(e) => {
            let msg = format!("Failed to compare versions: {}", e);
            warn!("{}", msg);
            UpdateCheckResult::Error(msg)
        }
    }
}

/// Check if a version string indicates a prerelease
fn is_prerelease(version: &str) -> bool {
    let lower = version.to_lowercase();
    lower.contains("-rc")
        || lower.contains("-alpha")
        || lower.contains("-beta")
        || lower.contains("-dev")
}

/// Result of applying an update
#[derive(Debug)]
pub enum ApplyUpdateResult {
    /// Update was applied successfully, restart needed
    Success { new_version: String },
    /// Already up to date
    UpToDate,
    /// Error occurred during update
    Error(String),
}

/// Download and apply the update.
///
/// This will:
/// 1. Download the latest release asset
/// 2. Extract and replace the current executable
/// 3. Return success status (caller should restart)
pub fn apply_update() -> ApplyUpdateResult {
    info!("📥 Downloading and applying update...");

    let result = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("dota2-scripts")
        .show_download_progress(false) // We show our own spinner in UI
        .no_confirm(true) // UI already prompted user
        .current_version(cargo_crate_version!())
        .build();

    match result {
        Ok(updater) => match updater.update() {
            Ok(status) => {
                let new_version = status.version().to_string();
                if status.updated() {
                    info!("✅ Update applied successfully! New version: v{}", new_version);
                    ApplyUpdateResult::Success { new_version }
                } else {
                    info!("Already up to date");
                    ApplyUpdateResult::UpToDate
                }
            }
            Err(e) => {
                let msg = format!("Failed to apply update: {}", e);
                error!("{}", msg);
                ApplyUpdateResult::Error(msg)
            }
        },
        Err(e) => {
            let msg = format!("Failed to configure updater: {}", e);
            error!("{}", msg);
            ApplyUpdateResult::Error(msg)
        }
    }
}

/// Restart the application by spawning a new process and exiting.
///
/// # Returns
/// This function does not return on success (exits the process).
/// Returns an error string if restart fails.
pub fn restart_application() -> Result<(), String> {
    info!("🔄 Restarting application...");

    let current_exe = std::env::current_exe().map_err(|e| format!("Failed to get current exe: {}", e))?;

    // Spawn new process
    Command::new(&current_exe)
        .spawn()
        .map_err(|e| format!("Failed to spawn new process: {}", e))?;

    // Exit current process
    info!("Exiting current process for restart...");
    std::process::exit(0);
}
