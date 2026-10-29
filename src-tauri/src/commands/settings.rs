use serde_json::Value;
use tauri::State;

#[tauri::command]
pub fn load_settings() -> Result<Value, String> {
    crate::config::load()
}

#[tauri::command]
pub async fn save_settings(app: tauri::AppHandle, settings: Value) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || crate::desktop::save(&app, &settings))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn append_ui_log(entry: Value) -> Result<(), String> {
    crate::logs::append(&entry)
}

#[tauri::command]
pub fn set_power_prevention(
    manager: State<'_, crate::power::PowerManager>,
    system_sleep: bool,
    display_sleep: bool,
) -> Result<(), String> {
    manager.apply(system_sleep, display_sleep)
}
