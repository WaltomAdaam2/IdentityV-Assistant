use serde_json::Value;
use tauri::State;

#[tauri::command]
pub fn load_settings() -> Result<Value, String> {
    crate::config::load()
}

#[tauri::command]
pub fn save_settings(settings: Value) -> Result<(), String> {
    crate::config::save(&settings)
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
