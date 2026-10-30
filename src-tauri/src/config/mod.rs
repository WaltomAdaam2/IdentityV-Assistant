use std::{env, fs, path::PathBuf};

use serde_json::{json, Value};

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
    let mut settings: Value = serde_json::from_str(&text)
        .map_err(|error| format!("Invalid configuration in {}: {error}", path.display()))?;
    if migrate(&mut settings) {
        save(&settings)?;
    }
    Ok(settings)
}

fn migrate(settings: &mut Value) -> bool {
    let before = settings.clone();
    if let Some(app) = settings.get_mut("app").and_then(Value::as_object_mut) {
        if !app
            .get("check_component_updates_on_launch")
            .is_some_and(Value::is_boolean)
        {
            app.insert(
                "check_component_updates_on_launch".into(),
                Value::Bool(true),
            );
        }
        if !app
            .get("check_updates_on_start")
            .is_some_and(Value::is_boolean)
        {
            app.insert("check_updates_on_start".into(), Value::Bool(true));
        }
        if !app
            .get("update_source")
            .and_then(Value::as_str)
            .is_some_and(|value| ["direct", "proxy", "ghfast", "ghproxy_net"].contains(&value))
        {
            app.insert("update_source".into(), json!("direct"));
        }
        if !app.get("update_prereleases").is_some_and(Value::is_boolean) {
            app.insert("update_prereleases".into(), Value::Bool(false));
        }
    }
    if let Some(engine) = settings.get_mut("engine").and_then(Value::as_object_mut) {
        engine.remove("dry_run");
        engine.remove("mock_step_delay_seconds");
    }
    if let Some(window) = settings
        .get_mut("game_window")
        .and_then(Value::as_object_mut)
    {
        for key in ["background_mode", "restore_after_task"] {
            window.remove(key);
        }
        for (key, allowed, default) in [
            (
                "resolution",
                &["2560x1440", "1920x1080", "1600x900", "1280x720"][..],
                "1600x900",
            ),
            (
                "position",
                &[
                    "center",
                    "unrestricted",
                    "top_left",
                    "top_right",
                    "bottom_left",
                    "bottom_right",
                ][..],
                "top_right",
            ),
        ] {
            if !window
                .get(key)
                .and_then(Value::as_str)
                .is_some_and(|value| allowed.contains(&value))
            {
                window.insert(key.to_owned(), Value::from(default));
            }
        }
    }
    if let Some(root) = settings.as_object_mut() {
        let tasks = root
            .entry("tasks")
            .or_insert_with(|| json!({"enabled": ["login_lobby"]}));
        if let Some(enabled) = tasks.get_mut("enabled").and_then(Value::as_array_mut) {
            for id in enabled {
                if id.as_str() == Some("home_book") {
                    *id = Value::from("blackjack");
                }
            }
        }
    }
    *settings != before
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
    fn migrates_update_preferences_and_preserves_disabled_startup_check() {
        let mut settings =
            json!({"app": {"check_updates_on_start": false, "update_source": "ghfast"}});
        migrate(&mut settings);
        assert_eq!(settings["app"]["check_updates_on_start"], false);
        assert_eq!(settings["app"]["update_source"], "ghfast");
        settings["app"] = json!({"update_source": "unknown"});
        migrate(&mut settings);
        assert_eq!(settings["app"]["check_updates_on_start"], true);
        assert_eq!(settings["app"]["update_source"], "direct");
    }

    #[test]
    fn bundled_default_configuration_is_valid_json() {
        let settings: Value = serde_json::from_str(DEFAULT_SETTINGS).expect("default config");
        assert!(settings.get("engine").is_some());
    }

    #[test]
    fn preview_updates_default_off_and_preserve_explicit_choice() {
        let mut settings = json!({"app": {}});
        migrate(&mut settings);
        assert_eq!(settings["app"]["update_prereleases"], false);
        settings["app"]["update_prereleases"] = json!(true);
        migrate(&mut settings);
        assert_eq!(settings["app"]["update_prereleases"], true);
        settings["app"]["update_prereleases"] = json!("yes");
        migrate(&mut settings);
        assert_eq!(settings["app"]["update_prereleases"], false);
    }

    #[test]
    fn settings_path_uses_named_local_application_directory() {
        let path = user_path_from(PathBuf::from("local-data"));
        assert_eq!(
            path,
            PathBuf::from("local-data/IdentityV-Assistant/config.json")
        );
    }

    #[test]
    fn migrates_old_task_and_window_settings() {
        let mut settings = json!({"engine": {"dry_run": true}, "game_window": {"resolution": "1600x900"}, "tasks": {"enabled": ["home_book"]}});
        assert!(migrate(&mut settings));
        assert_eq!(settings["tasks"]["enabled"], json!(["blackjack"]));
        assert!(settings["engine"].get("dry_run").is_none());
        assert_eq!(settings["game_window"]["resolution"], "1600x900");
        assert_eq!(settings["game_window"]["position"], "top_right");
        assert!(!migrate(&mut settings));
    }

    #[test]
    fn preserves_valid_window_preferences_and_replaces_obsolete_values() {
        let mut settings =
            json!({"game_window": {"resolution": "2560x1440", "position": "unrestricted"}});
        migrate(&mut settings);
        assert_eq!(settings["game_window"]["resolution"], "2560x1440");
        assert_eq!(settings["game_window"]["position"], "unrestricted");
        settings["game_window"]["resolution"] = json!("old");
        settings["game_window"]["position"] = json!(null);
        migrate(&mut settings);
        assert_eq!(settings["game_window"]["resolution"], "1600x900");
        assert_eq!(settings["game_window"]["position"], "top_right");
    }
}
