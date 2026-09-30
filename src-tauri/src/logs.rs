use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use chrono::{Duration, Local, NaiveDate};
use serde_json::Value;
use tauri::{AppHandle, Emitter};

static WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

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
    let line = serde_json::to_string(value).map_err(|error| error.to_string())?;
    writeln!(file, "{line}").map_err(|error| error.to_string())
}

pub fn emit(app: &AppHandle, value: Value) {
    if let Err(error) = append(&value) {
        eprintln!("Unable to save application log: {error}");
    }
    let _ = app.emit("engine-event", value);
}

#[cfg(test)]
mod tests {
    use super::*;

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
