use std::{env, fs, path::PathBuf};

use serde_json::Value;

const DEFAULT_SETTINGS: &str = include_str!("../../../config/default.json");
const APP_DIRECTORY: &str = "IdentityV-Assistant";
const SETTINGS_FILE: &str = "config.json";

fn local_data_root() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("LOCALAPPDATA") {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path));
    }
    env::var_os("HOME")
        .map(|path| PathBuf::from(path).join(".local/share"))
        .ok_or_else(|| "Unable to locate the local application data directory".into())
}

fn user_path_from(root: PathBuf) -> PathBuf {
    root.join(APP_DIRECTORY).join(SETTINGS_FILE)
}

pub fn user_path() -> Result<PathBuf, String> {
    local_data_root().map(user_path_from)
}

fn ensure_user_config() -> Result<PathBuf, String> {
    let path = user_path()?;
    if path.is_file() {
        return Ok(path);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&path, format!("{}\n", DEFAULT_SETTINGS.trim_end()))
        .map_err(|error| format!("Unable to write {}: {error}", path.display()))?;
    Ok(path)
}

pub fn load() -> Result<Value, String> {
    let path = ensure_user_config()?;
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Unable to read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("Invalid configuration in {}: {error}", path.display()))
}

pub fn active_path() -> Result<PathBuf, String> {
    ensure_user_config()
}

pub fn save(settings: &Value) -> Result<(), String> {
    if !settings.is_object() {
        return Err("Settings must be a JSON object".into());
    }
    let path = user_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    fs::write(&path, format!("{text}\n"))
        .map_err(|error| format!("Unable to write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_default_configuration_is_valid_json() {
        let settings: Value = serde_json::from_str(DEFAULT_SETTINGS).expect("default config");
        assert!(settings.get("engine").is_some());
    }

    #[test]
    fn settings_path_uses_named_local_application_directory() {
        let path = user_path_from(PathBuf::from("local-data"));
        assert_eq!(
            path,
            PathBuf::from("local-data/IdentityV-Assistant/config.json")
        );
    }
}
