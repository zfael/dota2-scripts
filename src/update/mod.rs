//! Auto-update module for checking and applying updates from GitHub Releases.
//!
//! Uses the `self_update` crate to:
//! 1. Check for newer versions on GitHub Releases
//! 2. Download the latest MSI installer and config template
//! 3. Launch a silent MSI upgrade handoff that relaunches the app

mod msi;

use self_update::cargo_crate_version;
use self_update::version::bump_is_greater;
use serde::Deserialize;
use tracing::{error, info, warn};

use crate::config::storage::{
    bootstrap_live_config, merge_template_with_local, ConfigPaths, EMBEDDED_CONFIG_TEMPLATE,
};

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

#[derive(Debug, Clone, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    prerelease: bool,
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

fn fetch_releases() -> Result<Vec<GitHubRelease>, String> {
    let releases_api_url = format!(
        "https://api.github.com/repos/{}/{}/releases",
        REPO_OWNER, REPO_NAME
    );

    reqwest::blocking::Client::new()
        .get(releases_api_url)
        .header(reqwest::header::USER_AGENT, "dota2-scripts-updater")
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|e| format!("Failed to fetch releases: {}", e))?
        .json::<Vec<GitHubRelease>>()
        .map_err(|e| format!("Failed to deserialize releases: {}", e))
}

fn latest_eligible_release(include_prereleases: bool) -> Result<GitHubRelease, String> {
    let releases = fetch_releases()?;

    let filtered_releases = if include_prereleases {
        releases
    } else {
        releases
            .into_iter()
            .filter(|release| !release.prerelease && !is_prerelease(&release.tag_name))
            .collect()
    };

    filtered_releases
        .into_iter()
        .next()
        .ok_or_else(|| "No eligible releases found".to_string())
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

    let latest = match latest_eligible_release(include_prereleases) {
        Ok(release) => release,
        Err(msg) => {
            warn!("{}", msg);
            return UpdateCheckResult::Error(msg);
        }
    };
    let latest_version = latest.tag_name.trim_start_matches('v');

    match bump_is_greater(current_version, latest_version) {
        Ok(true) => {
            info!(
                "✨ Update available: v{} -> v{}",
                current_version, latest_version
            );
            UpdateCheckResult::Available(UpdateInfo {
                version: latest_version.to_string(),
                release_notes: latest.body,
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

fn is_newer_than_current(tag_name: &str) -> Result<bool, String> {
    bump_is_greater(cargo_crate_version!(), tag_name.trim_start_matches('v'))
        .map_err(|e| format!("Failed to compare versions: {}", e))
}

/// Download the latest MSI/template assets, merge config, and hand off to msiexec.
///
/// This will:
/// 1. Download the latest release MSI + config template assets
/// 2. Merge the new template into the LocalAppData live config
/// 3. Spawn a PowerShell handoff that upgrades and relaunches the app
pub fn apply_update(include_prereleases: bool) -> ApplyUpdateResult {
    info!("📥 Downloading and applying update...");

    let latest = match latest_eligible_release(include_prereleases) {
        Ok(release) => release,
        Err(msg) => return ApplyUpdateResult::Error(msg),
    };

    match is_newer_than_current(&latest.tag_name) {
        Ok(false) => return ApplyUpdateResult::UpToDate,
        Err(msg) => return ApplyUpdateResult::Error(msg),
        Ok(true) => {}
    }

    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            let msg = format!("Failed to resolve current exe: {}", e);
            error!("{}", msg);
            return ApplyUpdateResult::Error(msg);
        }
    };

    if let Err(msg) = msi::ensure_msi_managed_install(&current_exe) {
        return ApplyUpdateResult::Error(msg);
    }

    let assets = latest
        .assets
        .iter()
        .map(|asset| msi::ReleaseAssetRef {
            name: asset.name.as_str(),
            download_url: asset.browser_download_url.as_str(),
        })
        .collect::<Vec<_>>();
    let install_assets =
        match msi::select_release_assets(&latest.tag_name, "x86_64-pc-windows-msvc", &assets) {
            Ok(assets) => assets,
            Err(msg) => return ApplyUpdateResult::Error(msg),
        };

    let template_contents = match reqwest::blocking::get(&install_assets.template_url)
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.text())
    {
        Ok(contents) => contents,
        Err(e) => {
            let msg = format!("Failed to download config template: {}", e);
            error!("{}", msg);
            return ApplyUpdateResult::Error(msg);
        }
    };

    let paths = match ConfigPaths::detect() {
        Ok(paths) => paths,
        Err(msg) => return ApplyUpdateResult::Error(msg),
    };
    let live_config_path = match bootstrap_live_config(&paths, EMBEDDED_CONFIG_TEMPLATE) {
        Ok(path) => path,
        Err(msg) => return ApplyUpdateResult::Error(msg),
    };
    let error_log_path = live_config_path
        .parent()
        .and_then(|config_dir| config_dir.parent())
        .map(|app_dir| app_dir.join("logs").join("update-error.log"))
        .unwrap_or_else(|| {
            std::env::temp_dir()
                .join("dota2-scripts")
                .join("logs")
                .join("update-error.log")
        });
    let local_contents = match std::fs::read_to_string(&live_config_path) {
        Ok(contents) => contents,
        Err(e) => {
            let msg = format!("Failed to read live config {}: {}", live_config_path.display(), e);
            error!("{}", msg);
            return ApplyUpdateResult::Error(msg);
        }
    };
    let merged_contents = match merge_template_with_local(&template_contents, &local_contents) {
        Ok(contents) => contents,
        Err(msg) => return ApplyUpdateResult::Error(msg),
    };

    let msi_path = match msi::download_to_temp(&install_assets.msi_url, "msi") {
        Ok(path) => path,
        Err(msg) => return ApplyUpdateResult::Error(msg),
    };
    let staged_config_path = match msi::write_temp_contents(&merged_contents, "toml") {
        Ok(path) => path,
        Err(msg) => return ApplyUpdateResult::Error(msg),
    };
    if let Err(msg) = msi::launch_msi_handoff(
        std::process::id(),
        &msi_path,
        &staged_config_path,
        &live_config_path,
        &error_log_path,
        &current_exe,
    ) {
        error!("{}", msg);
        return ApplyUpdateResult::Error(msg);
    }

    info!(
        "✅ Launched MSI update handoff for {} using {}",
        latest.tag_name,
        msi_path.display()
    );
    ApplyUpdateResult::Success {
        new_version: latest.tag_name.trim_start_matches('v').to_string(),
    }
}
