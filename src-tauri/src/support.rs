//! Explicit local settings backup and restore. Account/runtime data never enters this path.
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    sync::Mutex,
};
use tauri::{AppHandle, Manager, State};

pub static SETTINGS_OPERATION: Mutex<()> = Mutex::new(());

pub struct RestorePreview {
    id: String,
    previous: Value,
    settings: Value,
}

#[derive(Default)]
pub struct SupportState {
    restore: Mutex<Option<RestorePreview>>,
}

fn defaults() -> Value {
    let mut value: Value = serde_json::from_str(include_str!("../../config/default.json")).unwrap();
    value["journal"] = json!({"season":"s0","difficulty":"hard","door":"lf","floor":"1","display":"pure","collapsed":false});
    value
}

fn validate_tree(value: &Value, template: &Value, path: &str) -> Result<(), String> {
    let invalid = || format!("无效设置 / Invalid setting: {path}");
    match template {
        Value::Object(fields) => {
            let object = value.as_object().ok_or_else(invalid)?;
            for (key, child) in object {
                let expected = fields
                    .get(key)
                    .ok_or_else(|| format!("未知设置 / Unknown setting: {path}.{key}"))?;
                validate_tree(child, expected, &format!("{path}.{key}"))?;
            }
        }
        Value::Bool(_) if !value.is_boolean() => return Err(invalid()),
        Value::String(_) => {
            if !value
                .as_str()
                .is_some_and(|s| s.len() <= 4096 && !s.contains('\0'))
            {
                return Err(invalid());
            }
        }
        Value::Number(_) => {
            let number = value
                .as_f64()
                .filter(|n| n.is_finite())
                .ok_or_else(invalid)?;
            let (min, max, integral) = match path {
                "settings.app.ui_scale_percent" => (75., 150., true),
                "settings.engine.heartbeat_interval_seconds" => (0.1, 60., false),
                "settings.capture.idle_fps" | "settings.capture.active_fps" => (1., 120., true),
                "settings.vision.template_threshold" => (0., 1., false),
                "settings.vision.state_confirmation_count" => (1., 100., true),
                "settings.automation.login_timeout_seconds" => (30., 900., true),
                "settings.automation.camera_turn_scale_percent" => (20., 300., true),
                "settings.automation.click_position_jitter_px" => (1., 50., true),
                "settings.automation.click_interval_jitter_ms"
                | "settings.automation.click_duration_jitter_ms" => (1., 2000., true),
                _ => (0., 10000., true),
            };
            if number < min || number > max || integral && value.as_u64().is_none() {
                return Err(invalid());
            }
        }
        Value::Array(_) => {
            let items = value.as_array().ok_or_else(invalid)?;
            if items.len() > 64
                || items.iter().any(|v| {
                    !v.as_str()
                        .is_some_and(|s| !s.is_empty() && s.len() <= 256 && !s.contains('\0'))
                })
            {
                return Err(invalid());
            }
            if path == "settings.tasks.enabled"
                && items.iter().any(|v| {
                    ![
                        "login_lobby",
                        "blackjack",
                        "hunter_match",
                        "hunter_rank",
                        "hide_seek",
                    ]
                    .contains(&v.as_str().unwrap_or(""))
                })
            {
                return Err(invalid());
            }
        }
        _ => {}
    }
    let allowed: &[&str] = match path {
        "settings.app.language" => &["zh-CN", "en-US"],
        "settings.app.theme" => &["system", "light", "dark"],
        "settings.app.update_source" => &["direct", "proxy", "ghfast", "ghproxy_net"],
        "settings.game_window.resolution" => &["2560x1440", "1920x1080", "1600x900", "1280x720"],
        "settings.game_window.position" => &[
            "center",
            "unrestricted",
            "top_left",
            "top_right",
            "bottom_left",
            "bottom_right",
        ],
        "settings.capture.backend" => &["auto", "wgc", "dxcam", "mss"],
        "settings.automation.login_platform" => &["android", "ios"],
        _ => &[],
    };
    if !allowed.is_empty() && !allowed.contains(&value.as_str().unwrap_or("")) {
        return Err(invalid());
    }
    Ok(())
}

fn merge_known(target: &mut Value, source: &Value) {
    if let (Some(target), Some(source)) = (target.as_object_mut(), source.as_object()) {
        for (key, value) in source {
            if let Some(existing) = target.get_mut(key) {
                if existing.is_object() {
                    merge_known(existing, value);
                } else {
                    *existing = value.clone();
                }
            }
        }
    }
}

pub fn backup(settings: &Value) -> Result<Value, String> {
    let mut known = defaults();
    merge_known(&mut known, settings);
    validate_tree(&known, &defaults(), "settings")?;
    Ok(
        json!({"format":"identityv-assistant-settings","schema_version":1,
        "app_version":env!("CARGO_PKG_VERSION"),"settings":known}),
    )
}

fn import_backup(value: &Value) -> Result<Value, String> {
    if value["format"] != "identityv-assistant-settings" || value["schema_version"] != 1 {
        return Err("不支持的备份格式 / Unsupported backup format".into());
    }
    let version = semver::Version::parse(
        value["app_version"]
            .as_str()
            .ok_or("备份缺少版本 / Missing backup version")?,
    )
    .map_err(|e| e.to_string())?;
    if version > semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap() {
        return Err("此备份来自更新的助手版本 / Backup requires a newer assistant".into());
    }
    validate_tree(&value["settings"], &defaults(), "settings")?;
    let mut result = defaults();
    merge_known(&mut result, &value["settings"]);
    // Parse the hotkey before any desktop effect or file replacement.
    result["app"]["engine_hotkey"]
        .as_str()
        .unwrap_or("")
        .parse::<tauri_plugin_global_shortcut::Shortcut>()
        .map_err(|e| e.to_string())?;
    Ok(result)
}

fn differences(old: &Value, new: &Value, path: &str, output: &mut Vec<Value>) {
    if let Some(object) = new.as_object() {
        for (key, value) in object {
            differences(&old[key], value, &format!("{path}.{key}"), output);
        }
    } else if old != new {
        output.push(json!({"field":path,"before":old,"after":new}));
    }
}

pub fn write_export(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid export directory")?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn check_export_destination(path: &Path, data_root: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("Invalid export directory")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let root = data_root.canonicalize().map_err(|e| e.to_string())?;
    if parent.starts_with(root) {
        return Err("请保存在助手数据目录以外，避免覆盖运行数据。 / Save outside the assistant data directory to protect runtime data.".into());
    }
    Ok(())
}

#[tauri::command]
pub fn export_settings(path: String) -> Result<(), String> {
    let _guard = SETTINGS_OPERATION.lock().map_err(|e| e.to_string())?;
    let bytes =
        serde_json::to_vec_pretty(&backup(&crate::config::load()?)?).map_err(|e| e.to_string())?;
    check_export_destination(
        Path::new(&path),
        crate::config::user_path()?
            .parent()
            .ok_or("Missing data root")?,
    )?;
    write_export(Path::new(&path), &bytes)
}

#[tauri::command]
pub fn preview_settings_restore(
    path: String,
    state: State<'_, SupportState>,
) -> Result<Value, String> {
    let _guard = SETTINGS_OPERATION.lock().map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("备份超过 1 MiB / Backup exceeds 1 MiB".into());
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let mut settings = import_backup(&value)?;
    let previous = crate::config::load()?;
    let mut warnings = Vec::new();
    let executable = settings["game_window"]["executable_path"]
        .as_str()
        .unwrap_or("");
    if !executable.is_empty()
        && (!Path::new(executable).is_file()
            || !Path::new(executable)
                .file_name()
                .and_then(|v| v.to_str())
                .is_some_and(|s| {
                    s.eq_ignore_ascii_case("dwrg.exe") || s.eq_ignore_ascii_case("IdentityV.exe")
                }))
    {
        settings["game_window"]["executable_path"] = json!(previous["game_window"]
            ["executable_path"]
            .as_str()
            .unwrap_or(""));
        warnings.push("备份中的游戏路径在本机不可用，保留当前路径。 / Imported game path is unavailable; keeping the current path.");
    }
    let mut changes = Vec::new();
    differences(&previous, &settings, "settings", &mut changes);
    let id = uuid::Uuid::new_v4().to_string();
    let preview = json!({"id":id,"changes":changes,"warnings":warnings});
    *state.restore.lock().map_err(|e| e.to_string())? = Some(RestorePreview {
        id,
        previous,
        settings,
    });
    Ok(preview)
}

#[tauri::command]
pub async fn restore_settings(id: String, app: AppHandle) -> Result<Value, String> {
    let supervisor = app
        .state::<crate::engine::EngineSupervisor>()
        .inner()
        .clone();
    let _lifecycle = supervisor.lock_idle_for_restore().await?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = SETTINGS_OPERATION.lock().map_err(|e| e.to_string())?;
        let state = app.state::<SupportState>();
        let mut pending = state.restore.lock().map_err(|e| e.to_string())?;
        let preview = pending
            .as_ref()
            .filter(|p| p.id == id)
            .ok_or("请重新预览备份 / Preview the backup again")?;
        if crate::config::load()? != preview.previous {
            return Err("设置已变化，请重新预览 / Settings changed; preview again".into());
        }
        let directory = crate::config::user_path()?
            .parent()
            .ok_or("Missing data directory")?
            .join("backups");
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let original =
            serde_json::to_vec_pretty(&backup(&preview.previous)?).map_err(|e| e.to_string())?;
        write_export(
            &directory.join(format!("before-restore-{}.json", uuid::Uuid::new_v4())),
            &original,
        )?;
        crate::desktop::save(&app, &preview.settings)?;
        let settings = preview.settings.clone();
        *pending = None;
        Ok(settings)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backup_round_trip_uses_known_settings_and_rejects_external_schema_errors() {
        let mut source = defaults();
        source["password"] = json!("never-export");
        let value = backup(&source).unwrap();
        assert!(!value.to_string().contains("never-export"));
        assert_eq!(import_backup(&value).unwrap(), defaults());
        for (path, bad) in [
            ("ui_scale_percent", json!(999)),
            ("language", json!("invalid")),
            ("launch_on_login", json!("true")),
        ] {
            let mut invalid = value.clone();
            invalid["settings"]["app"][path] = bad;
            assert!(import_backup(&invalid).is_err());
        }
        let mut future = value;
        future["schema_version"] = json!(2);
        assert!(import_backup(&future).is_err());
    }
    #[test]
    fn imported_integer_and_window_keywords_match_engine_strict_types() {
        let mut value = backup(&defaults()).unwrap();
        value["settings"]["capture"]["active_fps"] = json!(3.0);
        assert!(import_backup(&value).is_err());
        value["settings"]["capture"]["active_fps"] = json!(3);
        value["settings"]["game_window"]["process_names"] = json!([""]);
        assert!(import_backup(&value).is_err());
        value["settings"]["game_window"]["process_names"] = json!(["dwrg.exe"]);
        assert!(import_backup(&value).is_ok());
    }

    #[test]
    fn atomic_export_replaces_file_with_complete_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, "original").unwrap();
        write_export(&path, b"{\"complete\":true}").unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), "{\"complete\":true}");
    }

    #[test]
    fn export_cannot_overwrite_live_data_and_failed_replace_keeps_original() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        fs::create_dir(&data).unwrap();
        assert!(check_export_destination(&data.join("config.json"), &data).is_err());
        assert!(check_export_destination(&dir.path().join("backup.json"), &data).is_ok());
        let original = data.join("config.json");
        fs::write(&original, "original").unwrap();
        assert!(write_export(&data, b"replacement").is_err());
        assert_eq!(fs::read_to_string(original).unwrap(), "original");
    }
}
