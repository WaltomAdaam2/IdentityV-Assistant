use serde_json::Value;

#[tauri::command]
pub fn load_settings() -> Result<Value, String> {
    crate::config::load()
}

#[tauri::command]
pub fn save_settings(settings: Value) -> Result<(), String> {
    crate::config::save(&settings)
}
