use std::{fs, path::PathBuf};

use serde_json::Value;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn default_path() -> PathBuf {
    repository_root().join("config/default.json")
}

pub fn user_path() -> PathBuf {
    repository_root().join("config/user.json")
}

pub fn load() -> Result<Value, String> {
    let path = active_path();
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Unable to read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("Invalid configuration in {}: {error}", path.display()))
}

pub fn active_path() -> PathBuf {
    if user_path().is_file() {
        user_path()
    } else {
        default_path()
    }
}

pub fn save(settings: &Value) -> Result<(), String> {
    if !settings.is_object() {
        return Err("Settings must be a JSON object".into());
    }
    let path = user_path();
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
        let settings = load().expect("default config should load");
        assert!(settings.get("engine").is_some());
    }
}
