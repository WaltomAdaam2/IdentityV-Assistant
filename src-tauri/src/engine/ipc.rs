use chrono::Utc;
use serde_json::{Map, Value};
use uuid::Uuid;

pub const IPC_VERSION: u64 = 1;

pub fn normalize_command(mut command: Value) -> Result<String, String> {
    let object = command
        .as_object_mut()
        .ok_or_else(|| "Engine command must be a JSON object".to_string())?;
    insert_default(object, "version", Value::from(IPC_VERSION));
    insert_default(object, "timestamp", Value::from(Utc::now().to_rfc3339()));
    insert_default(
        object,
        "request_id",
        Value::from(Uuid::new_v4().to_string()),
    );
    if !object.get("type").is_some_and(Value::is_string) {
        return Err("Engine command requires a string type".to_string());
    }
    serde_json::to_string(&command)
        .map(|line| format!("{line}\n"))
        .map_err(|error| error.to_string())
}

pub fn validate_event(message: &Value) -> Result<(), String> {
    let object = message
        .as_object()
        .ok_or_else(|| "Engine event must be a JSON object".to_string())?;
    if object.get("version").and_then(Value::as_u64) != Some(IPC_VERSION) {
        return Err(format!(
            "Unsupported IPC version: {:?}",
            object.get("version")
        ));
    }
    if !object.get("type").is_some_and(Value::is_string) {
        return Err("Engine event requires a string type".to_string());
    }
    if !object.get("timestamp").is_some_and(Value::is_string) {
        return Err("Engine event requires a string timestamp".to_string());
    }
    Ok(())
}

fn insert_default(object: &mut Map<String, Value>, key: &str, value: Value) {
    object.entry(key.to_string()).or_insert(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_has_required_envelope() {
        let line = normalize_command(serde_json::json!({"type": "ping"})).unwrap();
        let message: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(message["version"], 1);
        assert!(message["timestamp"].is_string());
        assert!(message["request_id"].is_string());
    }

    #[test]
    fn command_type_is_required() {
        assert!(normalize_command(serde_json::json!({})).is_err());
    }

    #[test]
    fn event_version_is_checked() {
        let event = serde_json::json!({
            "version": 2,
            "type": "ready",
            "timestamp": "2026-01-01T00:00:00Z"
        });
        assert!(validate_event(&event).is_err());
    }
}
