use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAssetRef<'a> {
    pub name: &'a str,
    pub download_url: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInstallAssets {
    pub msi_url: String,
    pub template_url: String,
}

pub fn select_release_assets(
    version_tag: &str,
    target: &str,
    assets: &[ReleaseAssetRef<'_>],
) -> Result<ReleaseInstallAssets, String> {
    let msi_name = format!("dota2-scripts-{version_tag}-{target}.msi");
    let template_name = format!("dota2-scripts-{version_tag}-config.template.toml");

    let msi_url = assets
        .iter()
        .find(|asset| asset.name == msi_name)
        .map(|asset| asset.download_url.to_string())
        .ok_or_else(|| format!("Missing MSI asset: {msi_name}"))?;
    let template_url = assets
        .iter()
        .find(|asset| asset.name == template_name)
        .map(|asset| asset.download_url.to_string())
        .ok_or_else(|| format!("Missing config template asset: {template_name}"))?;

    Ok(ReleaseInstallAssets {
        msi_url,
        template_url,
    })
}

pub fn build_msi_handoff_script(
    current_pid: u32,
    msi_path: &Path,
    staged_config_path: &Path,
    live_config_path: &Path,
    error_log_path: &Path,
    relaunch_exe: &Path,
) -> String {
    let msi = ps_quote(msi_path);
    let staged_config = ps_quote(staged_config_path);
    let live_config = ps_quote(live_config_path);
    let error_log = ps_quote(error_log_path);
    let relaunch = ps_quote(relaunch_exe);

    format!(
        "$ErrorActionPreference = 'Stop'; \
         $liveConfigDir = Split-Path -Parent {live_config}; \
         $errorLogDir = Split-Path -Parent {error_log}; \
         New-Item -ItemType Directory -Force -Path $liveConfigDir | Out-Null; \
         New-Item -ItemType Directory -Force -Path $errorLogDir | Out-Null; \
         Remove-Item -Path {error_log} -ErrorAction SilentlyContinue; \
         try {{ \
           Wait-Process -Id {current_pid} -ErrorAction SilentlyContinue; \
           $process = Start-Process msiexec.exe -ArgumentList @('/i', {msi}, '/qn', '/norestart') -Wait -PassThru; \
           if ($process.ExitCode -ne 0) {{ \
             Set-Content -Path {error_log} -Value \"MSI upgrade failed with exit code $($process.ExitCode)\"; \
             exit $process.ExitCode; \
           }}; \
           Copy-Item -Path {staged_config} -Destination {live_config} -Force; \
           Remove-Item -Path {staged_config} -ErrorAction SilentlyContinue; \
           Start-Process {relaunch}; \
         }} catch {{ \
           Set-Content -Path {error_log} -Value $_; \
           exit 1; \
         }}",
    )
}

pub fn ensure_msi_managed_install(current_exe: &Path) -> Result<(), String> {
    let install_dir = current_exe
        .parent()
        .ok_or_else(|| "Current exe has no parent directory".to_string())?
        .to_path_buf();
    let zip_style_config = install_dir
        .join("config")
        .join("config.toml");

    if zip_style_config.exists() {
        return Err(
            "This app still appears to be running from a ZIP-style layout. Install the MSI manually once, then use in-app updates from there."
                .to_string(),
        );
    }

    Ok(())
}

pub fn download_to_temp(url: &str, extension: &str) -> Result<PathBuf, String> {
    let response = reqwest::blocking::get(url)
        .and_then(|response| response.error_for_status())
        .map_err(|e| format!("Download failed: {e}"))?;
    let bytes = response
        .bytes()
        .map_err(|e| format!("Failed to read download bytes: {e}"))?;

    let path = std::env::temp_dir().join(format!(
        "dota2-scripts-update-{}.{}",
        rand::random::<u64>(),
        extension
    ));
    std::fs::write(&path, &bytes).map_err(|e| format!("Failed to write {}: {e}", path.display()))?;

    Ok(path)
}

pub fn write_temp_contents(contents: &str, extension: &str) -> Result<PathBuf, String> {
    let path = std::env::temp_dir().join(format!(
        "dota2-scripts-update-{}.{}",
        rand::random::<u64>(),
        extension
    ));
    std::fs::write(&path, contents.as_bytes())
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;

    Ok(path)
}

pub fn launch_msi_handoff(
    current_pid: u32,
    msi_path: &Path,
    staged_config_path: &Path,
    live_config_path: &Path,
    error_log_path: &Path,
    relaunch_exe: &Path,
) -> Result<(), String> {
    let script = build_msi_handoff_script(
        current_pid,
        msi_path,
        staged_config_path,
        live_config_path,
        error_log_path,
        relaunch_exe,
    );

    Command::new("powershell.exe")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
        .spawn()
        .map_err(|e| format!("Failed to launch MSI handoff: {e}"))?;

    Ok(())
}

fn ps_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_matching_msi_and_template_assets() {
        let assets = vec![
            ReleaseAssetRef {
                name: "dota2-scripts-v0.15.0-x86_64-pc-windows-msvc.msi",
                download_url: "https://example.invalid/app.msi",
            },
            ReleaseAssetRef {
                name: "dota2-scripts-v0.15.0-config.template.toml",
                download_url: "https://example.invalid/config.template.toml",
            },
        ];

        let selected =
            select_release_assets("v0.15.0", "x86_64-pc-windows-msvc", &assets).unwrap();

        assert_eq!(selected.msi_url, "https://example.invalid/app.msi");
        assert_eq!(
            selected.template_url,
            "https://example.invalid/config.template.toml"
        );
    }

    #[test]
    fn builds_hidden_powershell_handoff_script() {
        let script = build_msi_handoff_script(
            4242,
            Path::new(r"C:\Temp\dota2-scripts.msi"),
            Path::new(r"C:\Temp\merged-config.toml"),
            Path::new(r"C:\Users\pc\AppData\Local\dota2-scripts\config\config.toml"),
            Path::new(r"C:\Users\pc\AppData\Local\dota2-scripts\logs\update-error.log"),
            Path::new(r"C:\Program Files\dota2-scripts\dota2-scripts.exe"),
        );

        assert!(script.contains("Wait-Process -Id 4242 -ErrorAction SilentlyContinue"));
        assert!(script.contains("msiexec.exe"));
        assert!(script.contains("Copy-Item -Path"));
        assert!(script.contains("-Force"));
        assert!(script.contains("Set-Content"));
        assert!(script.contains("Start-Process"));
    }

    #[test]
    fn rejects_zip_style_layout_for_in_app_updates() {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = temp.path().join("zip-install");
        std::fs::create_dir_all(install_dir.join("config")).unwrap();
        std::fs::write(install_dir.join("config").join("config.toml"), "[updates]\n").unwrap();

        let error = ensure_msi_managed_install(&install_dir.join("dota2-scripts.exe")).unwrap_err();

        assert!(error.contains("Install the MSI manually once"));
    }

    #[test]
    fn allows_custom_install_path_without_zip_config_folder() {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = temp.path().join("custom-msi-install");
        std::fs::create_dir_all(&install_dir).unwrap();

        assert!(ensure_msi_managed_install(&install_dir.join("dota2-scripts.exe")).is_ok());
    }
}
