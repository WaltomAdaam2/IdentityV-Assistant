use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use chrono::{Duration, Local, NaiveDate};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

static WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) fn redacted(value: &Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| {
                    let lower = key.to_ascii_lowercase();
                    let private = [
                        "password",
                        "token",
                        "secret",
                        "cookie",
                        "authorization",
                        "credential",
                        "private_key",
                        "account",
                        "username",
                        "email",
                        "api_key",
                        "apikey",
                        "ticket",
                        "session_id",
                        "passwd",
                    ]
                    .iter()
                    .any(|name| lower.contains(name));
                    let image = ["preview", "image_base64", "image_data", "screenshot_base64"]
                        .contains(&lower.as_str());
                    (
                        key.clone(),
                        if private || image {
                            json!("<redacted>")
                        } else {
                            redacted(value)
                        },
                    )
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redacted).collect()),
        Value::String(text) => {
            let lower: String = text
                .to_ascii_lowercase()
                .chars()
                .filter(|c| !c.is_ascii_whitespace() && *c != '\'' && *c != '"')
                .collect();
            if [
                "password=",
                "password:",
                "token=",
                "token:",
                "cookie:",
                "cookie=",
                "authorization",
                "bearer",
                "secret=",
                "secret:",
                "api_key",
                "apikey",
                "privatekey",
                "private_key",
                "private-key",
                "credential:",
                "credential=",
                "username:",
                "username=",
                "account:",
                "account=",
                "email:",
                "email=",
                "ticket=",
                "ticket:",
                "session_id=",
                "session_id:",
                "passwd=",
                "passwd:",
                "密码:",
                "密码：",
            ]
            .iter()
            .any(|marker| lower.contains(marker))
                || lower.contains("data:image/")
            {
                json!("<redacted sensitive text>")
            } else {
                value.clone()
            }
        }
        _ => value.clone(),
    }
}

pub fn record(app: &AppHandle, level: &str, source: &str, message: &str, details: Value) {
    emit(
        app,
        json!({"version":1,"type":"log","timestamp":chrono::Utc::now().to_rfc3339(),
        "level":level,"source":source,"message":message,"details":details,
        "data":{"important":level != "debug"}}),
    );
}

pub fn stderr_record(line: &str, source: &str) -> Value {
    if let Ok(value) = serde_json::from_str::<Value>(line) {
        if value["type"] == "log"
            && value["version"] == 1
            && value["timestamp"].is_string()
            && value["message"].is_string()
            && matches!(
                value["level"].as_str(),
                Some("debug" | "info" | "warning" | "error")
            )
        {
            return value;
        }
    }
    json!({"version":1,"type":"log","timestamp":chrono::Utc::now().to_rfc3339(),
        "level":"error","source":source,"message":line,"data":{"important":true}})
}

fn root() -> Result<PathBuf, String> {
    crate::config::user_path()?
        .parent()
        .map(|path| path.join("logs"))
        .ok_or_else(|| "Unable to locate application data".into())
}

fn prune_at(directory: &std::path::Path, today: NaiveDate) -> Result<(), String> {
    let cutoff = today - Duration::days(30);
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let Ok(date) = NaiveDate::parse_from_str(stem, "%Y-%m-%d") else {
            continue;
        };
        if date < cutoff {
            fs::remove_file(path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub fn prune() -> Result<(), String> {
    let directory = root()?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    prune_at(&directory, Local::now().date_naive())
}

pub fn append(value: &Value) -> Result<(), String> {
    let _guard = WRITE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|error| error.to_string())?;
    let directory = root()?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let today = Local::now().date_naive();
    prune_at(&directory, today)?;
    let path = directory.join(format!("{today}.jsonl"));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    let line = serde_json::to_string(&redacted(value)).map_err(|error| error.to_string())?;
    writeln!(file, "{line}").map_err(|error| error.to_string())
}

pub fn emit(app: &AppHandle, value: Value) {
    let value = redacted(&value);
    if let Err(error) = append(&value) {
        eprintln!("Unable to save application log: {error}");
    }
    let _ = app.emit("engine-event", value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_records_preserve_diagnostics_without_credentials_or_images() {
        let details = "trace line\n".repeat(400);
        let record = redacted(
            &json!({"type":"log","source":"journal","message":"识图已完成",
            "details":{"request_id":"request-1","trace":details,"count":29,"password":"private", "nested":{"access_token":"secret"}},
            "preview":"data:image/jpeg;base64,private", "error":"Authorization: Bearer private"}),
        );
        assert_eq!(record["details"]["request_id"], "request-1");
        assert_eq!(record["details"]["trace"], details);
        assert_eq!(record["details"]["count"], 29);
        let text = record.to_string();
        assert!(!text.contains("Bearer private"));
        assert!(!text.contains("base64,private"));
        assert_eq!(record["details"]["nested"]["access_token"], "<redacted>");
    }

    #[test]
    fn stderr_preserves_structured_severity_and_unstructured_errors() {
        let event = stderr_record(
            r#"{"version":1,"type":"log","timestamp":"now","level":"debug","source":"window.automation","message":"details"}"#,
            "engine.stderr",
        );
        assert_eq!(event["level"], "debug");
        assert_eq!(event["source"], "window.automation");
        assert_eq!(
            stderr_record("Traceback: fixture", "journal.stderr")["level"],
            "error"
        );
    }

    #[test]
    fn removes_only_expired_dated_logs() {
        let directory =
            std::env::temp_dir().join(format!("identityv-log-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("2026-08-30.jsonl"), "old").unwrap();
        fs::write(directory.join("2026-09-01.jsonl"), "new").unwrap();
        fs::write(directory.join("notes.txt"), "keep").unwrap();
        prune_at(&directory, NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()).unwrap();
        assert!(!directory.join("2026-08-30.jsonl").exists());
        assert!(directory.join("2026-09-01.jsonl").exists());
        assert!(directory.join("notes.txt").exists());
        fs::remove_file(directory.join("2026-09-01.jsonl")).unwrap();
        fs::remove_file(directory.join("notes.txt")).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
