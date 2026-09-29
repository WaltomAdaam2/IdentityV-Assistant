use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use tauri::State;
use uuid::Uuid;

use crate::engine::EngineSupervisor;

async fn request(
    supervisor: State<'_, EngineSupervisor>,
    command_type: &str,
    extra: Value,
) -> Result<Value, String> {
    let request_id = Uuid::new_v4().to_string();
    let mut command = json!({"type": command_type, "request_id": request_id});
    if let (Some(command), Some(extra)) = (command.as_object_mut(), extra.as_object()) {
        command.extend(extra.clone());
    }
    supervisor.send(command).await?;
    Ok(json!({"requested": true, "request_id": request_id}))
}

#[tauri::command]
pub async fn detect_window(supervisor: State<'_, EngineSupervisor>) -> Result<Value, String> {
    request(supervisor, "detect_window", json!({})).await
}

#[tauri::command]
pub async fn capture_frame(supervisor: State<'_, EngineSupervisor>) -> Result<Value, String> {
    request(supervisor, "capture_frame", json!({})).await
}

#[tauri::command]
pub async fn test_template(
    template: String,
    supervisor: State<'_, EngineSupervisor>,
) -> Result<Value, String> {
    request(supervisor, "test_template", json!({"template": template})).await
}

#[tauri::command]
pub async fn input_test(
    action: Value,
    supervisor: State<'_, EngineSupervisor>,
) -> Result<Value, String> {
    request(supervisor, "input_test", json!({"action": action})).await
}

#[tauri::command]
pub fn read_debug_frame(path: String) -> Result<String, String> {
    let requested = PathBuf::from(&path)
        .canonicalize()
        .map_err(|error| format!("Unable to resolve debug frame: {error}"))?;
    let debug_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Unable to resolve repository root")?
        .join("debug")
        .canonicalize()
        .map_err(|error| format!("Unable to resolve debug directory: {error}"))?;
    if !requested.starts_with(&debug_root) {
        return Err("Debug frame path is outside the project debug directory".into());
    }
    let metadata = fs::metadata(&requested).map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("Debug frame path is not a regular file".into());
    }
    if metadata.len() > 20 * 1024 * 1024 {
        return Err("Debug frame exceeds the 20 MiB limit".into());
    }
    let extension = requested
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    let mime = match extension.as_deref() {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        _ => return Err("Only PNG and JPEG debug frames are supported".into()),
    };
    let bytes = fs::read(requested).map_err(|error| error.to_string())?;
    Ok(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
}
