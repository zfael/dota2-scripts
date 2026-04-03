use std::fs;
use std::path::PathBuf;

pub const EMBEDDED_CONFIG_TEMPLATE: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/config.toml"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPaths {
    local_app_data_dir: PathBuf,
    exe_dir: PathBuf,
}

impl ConfigPaths {
    pub fn detect() -> Result<Self, String> {
        let local_app_data_dir = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| "LOCALAPPDATA is not set".to_string())?;
        let exe_dir = std::env::current_exe()
            .map_err(|e| format!("Failed to resolve current exe: {e}"))?
            .parent()
            .ok_or_else(|| "Current exe has no parent directory".to_string())?
            .to_path_buf();

        Ok(Self::from_parts(local_app_data_dir, exe_dir))
    }

    pub fn from_parts(local_app_data_dir: PathBuf, exe_dir: PathBuf) -> Self {
        Self {
            local_app_data_dir,
            exe_dir,
        }
    }

    pub fn live_config_path(&self) -> PathBuf {
        self.local_app_data_dir
            .join("dota2-scripts")
            .join("config")
            .join("config.toml")
    }

    pub fn legacy_install_config_path(&self) -> PathBuf {
        self.exe_dir.join("config").join("config.toml")
    }
}

pub fn bootstrap_live_config(
    paths: &ConfigPaths,
    embedded_template: &str,
) -> Result<PathBuf, String> {
    let live_path = paths.live_config_path();

    if let Some(parent) = live_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {e}"))?;
    }

    if live_path.exists() {
        return Ok(live_path);
    }

    let seed = if paths.legacy_install_config_path().exists() {
        fs::read_to_string(paths.legacy_install_config_path())
            .map_err(|e| format!("Failed to import legacy config: {e}"))?
    } else {
        embedded_template.to_string()
    };

    fs::write(&live_path, seed).map_err(|e| format!("Failed to write live config: {e}"))?;

    Ok(live_path)
}

pub fn merge_template_with_local(
    template_contents: &str,
    local_contents: &str,
) -> Result<String, String> {
    let mut template_value: toml::Value =
        toml::from_str(template_contents).map_err(|e| format!("Template TOML error: {e}"))?;
    let local_value: toml::Value =
        toml::from_str(local_contents).map_err(|e| format!("Local TOML error: {e}"))?;

    merge_values(&mut template_value, &local_value);

    toml::to_string_pretty(&template_value).map_err(|e| format!("TOML serialization error: {e}"))
}

pub fn merge_saved_settings_with_existing(
    existing_contents: &str,
    desired_contents: &str,
) -> Result<String, String> {
    if existing_contents.trim().is_empty() {
        return Ok(desired_contents.to_string());
    }

    let mut existing_value: toml::Value =
        toml::from_str(existing_contents).map_err(|e| format!("Existing TOML error: {e}"))?;
    let desired_value: toml::Value =
        toml::from_str(desired_contents).map_err(|e| format!("Desired TOML error: {e}"))?;

    merge_values(&mut existing_value, &desired_value);

    toml::to_string_pretty(&existing_value).map_err(|e| format!("TOML serialization error: {e}"))
}

pub fn persist_live_config(
    paths: &ConfigPaths,
    desired_contents: &str,
    embedded_template: &str,
) -> Result<PathBuf, String> {
    let live_path = bootstrap_live_config(paths, embedded_template)?;
    let existing_contents = fs::read_to_string(&live_path).unwrap_or_default();
    let merged_contents = merge_saved_settings_with_existing(&existing_contents, desired_contents)?;

    fs::write(&live_path, merged_contents)
        .map_err(|e| format!("Failed to write live config: {e}"))?;

    Ok(live_path)
}

fn merge_values(base: &mut toml::Value, overlay: &toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
            for (key, overlay_value) in overlay_table {
                match base_table.get_mut(key) {
                    Some(base_value) => merge_values(base_value, overlay_value),
                    None => {
                        base_table.insert(key.clone(), overlay_value.clone());
                    }
                }
            }
        }
        (base_slot, overlay_value) => {
            *base_slot = overlay_value.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolves_live_config_into_local_app_data() {
        let temp = tempdir().unwrap();
        let paths = ConfigPaths::from_parts(
            temp.path().join("LocalAppData"),
            temp.path().join("install-root"),
        );

        assert_eq!(
            paths.live_config_path(),
            temp.path()
                .join("LocalAppData")
                .join("dota2-scripts")
                .join("config")
                .join("config.toml")
        );
    }

    #[test]
    fn imports_legacy_install_config_when_live_file_is_missing() {
        let temp = tempdir().unwrap();
        let paths = ConfigPaths::from_parts(
            temp.path().join("LocalAppData"),
            temp.path().join("install-root"),
        );

        std::fs::create_dir_all(paths.legacy_install_config_path().parent().unwrap()).unwrap();
        std::fs::write(
            paths.legacy_install_config_path(),
            "[updates]\ncheck_on_startup = false\n",
        )
        .unwrap();

        let created =
            bootstrap_live_config(&paths, "[updates]\ncheck_on_startup = true\n").unwrap();

        assert_eq!(created, paths.live_config_path());
        assert_eq!(
            std::fs::read_to_string(paths.live_config_path()).unwrap(),
            "[updates]\ncheck_on_startup = false\n"
        );
    }

    #[test]
    fn merge_template_keeps_local_values_and_adds_new_template_keys() {
        let template = r#"
[updates]
check_on_startup = true
include_prereleases = false

[logging]
level = "info"
"#;

        let local = r#"
[updates]
check_on_startup = false

[custom]
keep_me = true
"#;

        let merged = merge_template_with_local(template, local).unwrap();
        let merged_value: toml::Value = toml::from_str(&merged).unwrap();

        assert_eq!(
            merged_value["updates"]["check_on_startup"].as_bool(),
            Some(false)
        );
        assert_eq!(
            merged_value["updates"]["include_prereleases"].as_bool(),
            Some(false)
        );
        assert_eq!(merged_value["custom"]["keep_me"].as_bool(), Some(true));
    }

    #[test]
    fn merge_template_preserves_local_only_nested_tables() {
        let template = r#"
[heroes.huskar]
standalone_key = "Home"
"#;

        let local = r#"
[heroes.huskar]
standalone_key = "End"

[heroes.custom_hero]
enabled = true
"#;

        let merged = merge_template_with_local(template, local).unwrap();
        let merged_value: toml::Value = toml::from_str(&merged).unwrap();

        assert_eq!(
            merged_value["heroes"]["huskar"]["standalone_key"].as_str(),
            Some("End")
        );
        assert_eq!(
            merged_value["heroes"]["custom_hero"]["enabled"].as_bool(),
            Some(true)
        );
    }

    #[test]
    fn saving_settings_keeps_unknown_keys_from_the_existing_live_file() {
        let existing = r#"
[updates]
check_on_startup = false

[custom]
keep_me = true
"#;

        let desired = r#"
[updates]
check_on_startup = true
include_prereleases = false
"#;

        let merged = merge_saved_settings_with_existing(existing, desired).unwrap();
        let merged_value: toml::Value = toml::from_str(&merged).unwrap();

        assert_eq!(
            merged_value["updates"]["check_on_startup"].as_bool(),
            Some(true)
        );
        assert_eq!(
            merged_value["updates"]["include_prereleases"].as_bool(),
            Some(false)
        );
        assert_eq!(merged_value["custom"]["keep_me"].as_bool(), Some(true));
    }

    #[test]
    fn persist_live_config_writes_to_local_app_data_and_preserves_unknown_keys() {
        let temp = tempdir().unwrap();
        let paths = ConfigPaths::from_parts(
            temp.path().join("LocalAppData"),
            temp.path().join("install-root"),
        );

        std::fs::create_dir_all(paths.live_config_path().parent().unwrap()).unwrap();
        std::fs::write(
            paths.live_config_path(),
            "[custom]\nkeep_me = true\n[updates]\ncheck_on_startup = false\n",
        )
        .unwrap();

        persist_live_config(
            &paths,
            "[updates]\ncheck_on_startup = true\ninclude_prereleases = false\n",
            EMBEDDED_CONFIG_TEMPLATE,
        )
        .unwrap();

        let merged_value: toml::Value =
            toml::from_str(&std::fs::read_to_string(paths.live_config_path()).unwrap()).unwrap();

        assert_eq!(
            merged_value["updates"]["check_on_startup"].as_bool(),
            Some(true)
        );
        assert_eq!(
            merged_value["updates"]["include_prereleases"].as_bool(),
            Some(false)
        );
        assert_eq!(merged_value["custom"]["keep_me"].as_bool(), Some(true));
    }
}
