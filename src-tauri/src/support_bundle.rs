//! Bounded, opt-in local diagnostic exports. No recursive directory collection.
use chrono::{DateTime, Local, Utc};
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::Mutex,
};
use tauri::{AppHandle, Manager};

struct Bundle {
    id: String,
    files: Vec<(String, Vec<u8>)>,
}
pub struct BundleState {
    started: DateTime<Utc>,
    prepared: Mutex<Option<Bundle>>,
}
impl Default for BundleState {
    fn default() -> Self {
        Self {
            started: Utc::now(),
            prepared: Mutex::new(None),
        }
    }
}

fn redact(value: &Value, key: &str) -> Value {
    let lower = key.to_ascii_lowercase();
    if [
        "password",
        "token",
        "secret",
        "cookie",
        "authorization",
        "account",
        "username",
        "credential",
        "private",
        "title",
        "path",
        "api_key",
        "apikey",
        "session_id",
        "email",
    ]
    .iter()
    .any(|k| lower.contains(k))
    {
        return json!("<redacted>");
    }
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(k, v)| (k.clone(), redact(v, k)))
                .collect(),
        ),
        Value::Array(items) => {
            Value::Array(items.iter().take(256).map(|v| redact(v, key)).collect())
        }
        Value::String(text) => {
            let lower = text.to_ascii_lowercase();
            if text.contains(":\\")
                || text.contains(":/")
                || text.starts_with('/')
                || text.contains("\\\\")
                || [
                    "password",
                    "token=",
                    "token:",
                    "cookie",
                    "authorization",
                    "bearer ",
                    "secret=",
                    "secret:",
                    "api_key",
                    "apikey",
                    "session_id",
                    "账号",
                    "密码",
                ]
                .iter()
                .any(|s| lower.contains(s))
            {
                json!("<redacted text containing paths, URLs or credentials>")
            } else {
                json!(text.chars().take(2000).collect::<String>())
            }
        }
        _ => value.clone(),
    }
}

fn is_link(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return true;
    };
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

fn session_logs(directory: &Path, started: DateTime<Utc>) -> Result<(Vec<u8>, bool), String> {
    if !directory.exists() {
        return Ok((Vec::new(), false));
    }
    if is_link(directory) {
        return Err("日志目录不能是链接 / Log directory cannot be a link".into());
    }
    let first_day = started.with_timezone(&Local).date_naive();
    let mut paths: Vec<_> = fs::read_dir(directory)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|s| s.to_str()) == Some("jsonl")
                && !is_link(p)
                && p.is_file()
                && p.file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                    .is_some_and(|d| d >= first_day)
        })
        .collect();
    paths.sort();
    paths.reverse();
    let mut remaining = 4 * 1024 * 1024u64;
    let mut truncated = false;
    let mut lines = Vec::new();
    for path in paths {
        if remaining == 0 {
            truncated = true;
            break;
        }
        let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
        let length = file.metadata().map_err(|e| e.to_string())?.len();
        let offset = length.saturating_sub(remaining);
        if offset > 0 {
            file.seek(SeekFrom::Start(offset))
                .map_err(|e| e.to_string())?;
            truncated = true;
        }
        let mut bytes = Vec::new();
        file.take(remaining)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        remaining = remaining.saturating_sub(bytes.len() as u64);
        for (index, line) in String::from_utf8_lossy(&bytes).lines().enumerate() {
            if index == 0 && offset > 0 {
                continue;
            }
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if value["timestamp"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .is_some_and(|t| t.with_timezone(&Utc) >= started)
            {
                lines.push(redact(&value, ""));
            }
        }
    }
    lines.sort_by(|a, b| a["timestamp"].as_str().cmp(&b["timestamp"].as_str()));
    let mut output = Vec::new();
    for line in lines {
        let encoded = serde_json::to_vec(&line).map_err(|e| e.to_string())?;
        if output.len() + encoded.len() + 1 > 4 * 1024 * 1024 {
            truncated = true;
            break;
        }
        output.extend(encoded);
        output.push(b'\n');
    }
    Ok((output, truncated))
}

fn generated_capture(path: &Path) -> bool {
    if path.extension().and_then(|s| s.to_str()) != Some("png") {
        return false;
    }
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    let stem = stem.strip_prefix("failure-").unwrap_or(stem);
    let Some((stamp, id)) = stem.split_once('-') else {
        return false;
    };
    stamp.len() == 22
        && stamp.bytes().enumerate().all(|(i, b)| {
            if i == 8 {
                b == b'T'
            } else if i == 21 {
                b == b'Z'
            } else {
                b.is_ascii_digit()
            }
        })
        && id.len() == 8
        && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn collect(
    root: &Path,
    started: DateTime<Utc>,
    settings: &Value,
    snapshot: &Value,
    include_images: bool,
) -> Result<(Vec<(String, Vec<u8>)>, bool), String> {
    let (logs, truncated) = session_logs(&root.join("logs"), started)?;
    let environment = json!({"assistant_version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"architecture":std::env::consts::ARCH,"session_started":started,"engine":redact(snapshot,""),"logs_truncated":truncated,"privacy":"Review before sharing; screenshots can contain names or chat. No account/runtime data is included."});
    let mut files = vec![
        (
            "environment.json".into(),
            serde_json::to_vec_pretty(&environment).map_err(|e| e.to_string())?,
        ),
        (
            "settings.redacted.json".into(),
            serde_json::to_vec_pretty(&redact(&crate::support::backup(settings)?["settings"], ""))
                .map_err(|e| e.to_string())?,
        ),
        ("session.redacted.jsonl".into(), logs),
    ];
    if include_images && root.join("debug").exists() && !is_link(&root.join("debug")) {
        let mut images = Vec::new();
        for subdirectory in ["failures", "captures"] {
            let directory = root.join("debug").join(subdirectory);
            if !directory.is_dir() || is_link(&directory) {
                continue;
            }
            for entry in fs::read_dir(directory)
                .map_err(|e| e.to_string())?
                .filter_map(Result::ok)
            {
                let path = entry.path();
                if generated_capture(&path) && !is_link(&path) && path.is_file() {
                    images.push(path);
                }
            }
        }
        images.sort_by_key(|p| std::cmp::Reverse(fs::metadata(p).and_then(|m| m.modified()).ok()));
        let mut total = 0;
        for path in images.into_iter().take(20) {
            let mut bytes = Vec::new();
            fs::File::open(&path)
                .map_err(|e| e.to_string())?
                .take(20 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 20 * 1024 * 1024
                || total + bytes.len() > 100 * 1024 * 1024
                || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
            {
                continue;
            }
            total += bytes.len();
            files.push((
                format!("images/{}", path.file_name().unwrap().to_string_lossy()),
                bytes,
            ));
        }
    }
    Ok((files, truncated))
}

#[tauri::command]
pub async fn preview_diagnostics(app: AppHandle, include_images: bool) -> Result<Value, String> {
    let snapshot = serde_json::to_value(
        app.state::<crate::engine::EngineSupervisor>()
            .snapshot()
            .await,
    )
    .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<BundleState>();
        let root = crate::config::user_path()?.parent().ok_or("Missing data root")?.to_path_buf();
        let (files,truncated) = collect(&root,state.started,&crate::config::load()?,&snapshot,include_images)?;
        let id = uuid::Uuid::new_v4().to_string();
        let response = json!({"id":id,"files":files.iter().map(|(name,bytes)|json!({"name":name,"size":bytes.len()})).collect::<Vec<_>>(),"truncated":truncated});
        *state.prepared.lock().map_err(|e|e.to_string())? = Some(Bundle {id,files});
        Ok(response)
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn export_diagnostics(app: AppHandle, id: String, path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<BundleState>();
        let mut prepared = state.prepared.lock().map_err(|e| e.to_string())?;
        let bundle = prepared
            .as_ref()
            .filter(|b| b.id == id)
            .ok_or("请重新预览诊断包 / Preview diagnostics again")?;
        let destination = Path::new(&path);
        crate::support::check_export_destination(
            destination,
            crate::config::user_path()?
                .parent()
                .ok_or("Missing data root")?,
        )?;
        let mut temporary = tempfile::NamedTempFile::new_in(
            destination.parent().ok_or("Invalid export directory")?,
        )
        .map_err(|e| e.to_string())?;
        {
            let mut zip = zip::ZipWriter::new(temporary.as_file_mut());
            for (name, bytes) in &bundle.files {
                zip.start_file(
                    name,
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated),
                )
                .map_err(|e| e.to_string())?;
                zip.write_all(bytes).map_err(|e| e.to_string())?;
            }
            zip.finish().map_err(|e| e.to_string())?;
        }
        temporary.as_file().sync_all().map_err(|e| e.to_string())?;
        temporary.persist(destination).map_err(|e| e.to_string())?;
        *prepared = None;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_bundle_uses_explicit_sources_and_redacts_nested_secrets() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("logs")).unwrap();
        fs::create_dir(dir.path().join("login-data")).unwrap();
        fs::write(dir.path().join("login-data/secret.txt"), "private-account").unwrap();
        let now = Utc::now();
        let record = json!({"timestamp":now,"type":"error","data":{"token":"private-token"},"message":"C:\\Users\\Name\\secret"});
        fs::write(
            dir.path()
                .join(format!("logs/{}.jsonl", Local::now().date_naive())),
            format!("{record}\n"),
        )
        .unwrap();
        let settings: Value =
            serde_json::from_str(include_str!("../../config/default.json")).unwrap();
        let (files, _) = collect(
            dir.path(),
            now - chrono::Duration::seconds(1),
            &settings,
            &json!({}),
            false,
        )
        .unwrap();
        assert_eq!(files.len(), 3);
        let text = files
            .iter()
            .map(|(_, bytes)| String::from_utf8_lossy(bytes))
            .collect::<Vec<_>>()
            .join("\n");
        for secret in ["private-account", "private-token", "C:\\\\Users"] {
            assert!(!text.contains(secret));
        }
        assert!(text.contains("redacted"));
        assert!(generated_capture(Path::new(
            "failure-20261031T010203123456Z-aabb0011.png"
        )));
        assert!(!generated_capture(Path::new("account.png")));
    }

    #[test]
    fn bundle_filters_old_sessions_and_only_includes_images_when_selected() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("debug/failures")).unwrap();
        fs::create_dir(dir.path().join("logs")).unwrap();
        let now = Utc::now();
        let old = json!({"timestamp":now-chrono::Duration::hours(1),"message":"old-session"});
        let current = json!({"timestamp":now,"message":"current-session","api_key":"private-api"});
        fs::write(
            dir.path()
                .join(format!("logs/{}.jsonl", Local::now().date_naive())),
            format!("{old}\n{current}\n"),
        )
        .unwrap();
        fs::write(
            dir.path()
                .join("debug/failures/failure-20261031T010203123456Z-aabb0011.png"),
            b"\x89PNG\r\n\x1a\nfixture",
        )
        .unwrap();
        fs::write(
            dir.path().join("debug/failures/account.png"),
            b"\x89PNG\r\n\x1a\nprivate",
        )
        .unwrap();
        let settings: Value =
            serde_json::from_str(include_str!("../../config/default.json")).unwrap();
        for include in [false, true] {
            let (files, truncated) = collect(
                dir.path(),
                now - chrono::Duration::seconds(1),
                &settings,
                &json!({}),
                include,
            )
            .unwrap();
            assert!(!truncated);
            assert_eq!(files.len(), if include { 4 } else { 3 });
            let logs = String::from_utf8_lossy(&files[2].1);
            assert!(logs.contains("current-session"));
            assert!(!logs.contains("old-session") && !logs.contains("private-api"));
            assert!(files.iter().all(|(name, _)| !name.contains("account")));
        }
    }
}
