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
            started: crate::logs::session_started(),
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
        Value::Array(items) => Value::Array(items.iter().map(|v| redact(v, key)).collect()),
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
                json!(text)
            }
        }
        _ => value.clone(),
    }
}

use crate::diagnostic_files::is_link;
const MAX_BUNDLE_BYTES: usize = 100_000_000;

fn session_logs(
    directory: &Path,
    started: DateTime<Utc>,
    limit: usize,
) -> Result<(Vec<u8>, bool), String> {
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
    let mut remaining = limit as u64;
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
            if (value.get("app_session").is_none() || value["app_session"] == json!(started))
                && value["timestamp"]
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
        if output.len() + encoded.len() + 1 > limit {
            truncated = true;
            break;
        }
        output.extend(encoded);
        output.push(b'\n');
    }
    Ok((output, truncated))
}

fn collect(
    root: &Path,
    started: DateTime<Utc>,
    settings: &Value,
    snapshot: &Value,
    include_images: bool,
    limit: usize,
) -> Result<(Vec<(String, Vec<u8>)>, Vec<Value>), String> {
    // Leave space for ZIP headers and the explicit omission manifest.
    let budget = limit.saturating_sub(128 * 1024);
    let mut files = vec![
        ("environment.json".into(), serde_json::to_vec_pretty(&json!({
            "assistant_version":env!("CARGO_PKG_VERSION"), "os":std::env::consts::OS,
            "architecture":std::env::consts::ARCH, "session_started":started, "engine":redact(snapshot,"")
        })).map_err(|e| e.to_string())?),
        ("settings.redacted.json".into(), serde_json::to_vec_pretty(&redact(&crate::support::backup(settings)?["settings"], "")).map_err(|e| e.to_string())?),
    ];
    let mut omissions = Vec::new();
    let mut used = files.iter().map(|(_, b)| b.len()).sum::<usize>();
    let (logs, truncated) = session_logs(&root.join("logs"), started, budget.saturating_sub(used))?;
    let session_images: std::collections::HashSet<String> = String::from_utf8_lossy(&logs)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|record| {
            Some(format!(
                "{}/{}",
                record["diagnostic_image"]["category"].as_str()?,
                record["diagnostic_image"]["file"].as_str()?
            ))
        })
        .collect();
    if truncated {
        omissions.push(json!({"group":"session.redacted.jsonl","reason":"size_limit","description":"Only the newest complete log records fit."}));
    }
    used += logs.len();
    files.push(("session.redacted.jsonl".into(), logs));
    let debug = root.join("debug");
    let mut groups = Vec::new();
    let mut reports = Vec::new();
    for directory in crate::diagnostic_files::attempts(&debug)? {
        let report = match crate::diagnostic_files::read_report(&directory) {
            Ok(report) => report,
            Err(_) => {
                omissions.push(json!({"group":directory.file_name().unwrap().to_string_lossy(),"reason":"unreadable_report"}));
                continue;
            }
        };
        if report["app_session"] != json!(started) {
            continue;
        }
        let id = directory.file_name().unwrap().to_string_lossy().to_string();
        let stamp = report["started"]
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|s| s.with_timezone(&Utc))
            .unwrap_or(started);
        let mut images = Vec::new();
        if let Some(metadata) = report["images"].as_array() {
            for entry in metadata {
                let Some(name) = entry["file"].as_str() else {
                    continue;
                };
                if name
                    .strip_suffix(".png")
                    .is_some_and(crate::diagnostic_files::image_kind)
                {
                    images.push((format!("recognition/{id}/{name}"), directory.join(name)));
                }
            }
        }
        groups.push((stamp, id.clone(), images));
        reports.push((
            stamp,
            format!("recognition/{id}/flow.redacted.json"),
            serde_json::to_vec_pretty(&redact(&report, "")).map_err(|e| e.to_string())?,
        ));
    }
    reports.sort_by_key(|(stamp, _, _)| std::cmp::Reverse(*stamp));
    for (_, name, bytes) in reports {
        if used + bytes.len() > budget {
            omissions.push(json!({"group":name,"reason":"size_limit"}));
        } else {
            used += bytes.len();
            files.push((name, bytes));
        }
    }
    if include_images && debug.is_dir() && !is_link(&debug) {
        for category in ["failures", "captures"] {
            let directory = debug.join(category);
            if !directory.is_dir() || is_link(&directory) {
                continue;
            }
            for entry in fs::read_dir(directory)
                .map_err(|e| e.to_string())?
                .filter_map(Result::ok)
            {
                let path = entry.path();
                if !path.is_file() || is_link(&path) {
                    continue;
                }
                if let Some(stamp) =
                    crate::diagnostic_files::capture_time(&path).filter(|t| *t >= started)
                {
                    let name = path.file_name().unwrap().to_string_lossy();
                    if !session_images.contains(&format!("{category}/{name}")) {
                        continue;
                    }
                    groups.push((
                        stamp,
                        format!("{category}/{name}"),
                        vec![(format!("images/{category}/{name}"), path)],
                    ));
                }
            }
        }
        groups.sort_by_key(|(stamp, _, _)| std::cmp::Reverse(*stamp));
        for (_, id, paths) in groups {
            if paths.is_empty() {
                continue;
            }
            let mut group = Vec::new();
            let mut group_bytes = 0;
            let mut reason = None;
            for (name, path) in &paths {
                if path.ancestors().any(is_link) {
                    reason = Some("unsafe_file");
                    break;
                }
                let bytes = (|| {
                    let file = fs::File::open(path)?;
                    let size = file.metadata()?.len();
                    if size > budget.saturating_sub(used + group_bytes) as u64 {
                        return Ok(None);
                    }
                    let mut bytes = Vec::new();
                    file.take(size + 1).read_to_end(&mut bytes)?;
                    Ok::<_, std::io::Error>(Some(bytes))
                })();
                match bytes {
                    Ok(Some(bytes))
                        if bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                            && used + group_bytes + bytes.len() <= budget =>
                    {
                        group_bytes += bytes.len();
                        group.push((name.clone(), bytes));
                    }
                    Ok(None) => {
                        reason = Some("size_limit");
                        break;
                    }
                    _ => {
                        reason = Some("missing_or_invalid_image");
                        break;
                    }
                }
            }
            if let Some(reason) = reason {
                omissions.push(json!({"group":id,"reason":reason,"files":paths.iter().map(|(n,_)|n).collect::<Vec<_>>()}));
            } else {
                used += group_bytes;
                files.extend(group);
            }
        }
    }
    let manifest = serde_json::to_vec_pretty(&json!({"session_started":started,"limit_bytes":limit,
        "include_images":include_images,"screenshot_retention_days":3,"log_retention_days":7,
        "omissions":omissions,"files":files.iter().map(|(n,b)|json!({"name":n,"bytes":b.len()})).collect::<Vec<_>>()
    })).map_err(|e| e.to_string())?;
    if used + manifest.len() + files.len() * 256 > limit {
        return Err(
            "诊断清单超出容量，请缩小会话范围 / Diagnostic manifest exceeds the bundle limit"
                .into(),
        );
    }
    files.push(("manifest.json".into(), manifest));
    Ok((files, omissions))
}

#[tauri::command]
pub async fn preview_diagnostics(app: AppHandle, include_images: bool) -> Result<Value, String> {
    crate::logs::record(
        &app,
        "info",
        "support.diagnostics",
        "开始预览诊断包 / Preparing diagnostics",
        json!({"include_images":include_images}),
    );
    let snapshot = serde_json::to_value(
        app.state::<crate::engine::EngineSupervisor>()
            .snapshot()
            .await,
    )
    .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<BundleState>();
        let root = crate::config::user_path()?.parent().ok_or("Missing data root")?.to_path_buf();
        app.state::<crate::journal_recognition::RecognitionState>().save_diagnostics()?;
        if let Err(error) = crate::diagnostic_files::prune(&root.join("debug"), Utc::now()) {
            crate::logs::record(&app,"warning","support.diagnostics","旧截图暂时无法清理 / Some expired images could not be removed",json!({"error":error}));
        }
        let (files,omissions) = collect(&root,state.started,&crate::config::load()?,&snapshot,include_images,MAX_BUNDLE_BYTES)?;
        let id = uuid::Uuid::new_v4().to_string();
        let response = json!({"id":id,"files":files.iter().map(|(name,bytes)|json!({"name":name,"size":bytes.len()})).collect::<Vec<_>>(),"truncated":!omissions.is_empty(),"omissions":omissions});
        crate::logs::record(&app,"info","support.diagnostics","诊断包预览就绪 / Diagnostics ready",json!({"files":files.len(),"bytes":files.iter().map(|(_,b)|b.len()).sum::<usize>(),"omissions":response["omissions"]}));
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
        if temporary
            .as_file()
            .metadata()
            .map_err(|e| e.to_string())?
            .len()
            > MAX_BUNDLE_BYTES as u64
        {
            return Err("Diagnostic ZIP exceeds 100 MB".into());
        }
        temporary.as_file().sync_all().map_err(|e| e.to_string())?;
        temporary.persist(destination).map_err(|e| e.to_string())?;
        crate::logs::record(
            &app,
            "info",
            "support.diagnostics",
            "诊断包已导出 / Diagnostics exported",
            json!({"bundle_id":id,"files":bundle.files.len()}),
        );
        *prepared = None;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    fn settings() -> Value {
        serde_json::from_str(include_str!("../../config/default.json")).unwrap()
    }
    fn attempt(
        root: &Path,
        session: DateTime<Utc>,
        stamp: DateTime<Utc>,
        sizes: &[usize],
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let dir = root.join("debug/recognition").join(&id);
        fs::create_dir_all(&dir).unwrap();
        let images: Vec<_> = sizes
            .iter()
            .enumerate()
            .map(|(index, size)| {
                let name = format!("region_{index}.png");
                let mut data = b"\x89PNG\r\n\x1a\n".to_vec();
                data.resize(*size, 0);
                fs::write(dir.join(&name), data).unwrap();
                json!({"file":name})
            })
            .collect();
        fs::write(dir.join("flow.json"), json!({"app_session":session,"started":stamp,"request_id":id,"status":"cancelled","images":images,"password":"private-value"}).to_string()).unwrap();
        id
    }
    #[test]
    fn export_keeps_current_session_attempts_and_latest_complete_image_groups() {
        let dir = tempfile::tempdir().unwrap();
        let session = Utc::now();
        let old = attempt(
            dir.path(),
            session - chrono::Duration::hours(1),
            session,
            &[20],
        );
        let first = attempt(dir.path(), session, session, &[90000, 90000]);
        let second = attempt(
            dir.path(),
            session,
            session + chrono::Duration::seconds(1),
            &[90000, 90000],
        );
        for images in [false, true] {
            let (files, omissions) =
                collect(dir.path(), session, &settings(), &json!({}), images, 400000).unwrap();
            assert!(!files.iter().any(|(name, _)| name.contains(&old)));
            assert!(files
                .iter()
                .any(|(name, _)| name == &format!("recognition/{first}/flow.redacted.json")));
            assert!(files
                .iter()
                .any(|(name, _)| name == &format!("recognition/{second}/flow.redacted.json")));
            let pictures: Vec<_> = files
                .iter()
                .filter(|(name, _)| name.ends_with(".png"))
                .collect();
            assert_eq!(pictures.len(), if images { 2 } else { 0 });
            assert!(pictures.iter().all(|(name, _)| name.contains(&second)));
            assert_eq!(omissions.len(), usize::from(images));
            assert!(files.iter().map(|(_, b)| b.len()).sum::<usize>() < 400000);
            assert!(files
                .iter()
                .filter(|(n, _)| n.ends_with(".json"))
                .all(|(_, b)| !String::from_utf8_lossy(b).contains("private-value")));
        }
    }
    #[test]
    fn session_logs_keep_full_details_and_filter_previous_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let session = Utc::now();
        let trace = "detail".repeat(1000);
        let rows = [
            json!({"timestamp":session,"app_session":session,"details":trace,"token":"secret-value"}),
            json!({"timestamp":session,"app_session":session - chrono::Duration::hours(1),"message":"previous-session"}),
        ];
        fs::write(
            dir.path()
                .join(format!("{}.jsonl", Local::now().date_naive())),
            rows.iter().map(|r| format!("{r}\n")).collect::<String>(),
        )
        .unwrap();
        let (bytes, truncated) = session_logs(dir.path(), session, MAX_BUNDLE_BYTES).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(!truncated && text.contains(&trace));
        assert!(!text.contains("previous-session") && !text.contains("secret-value"));
        let many = json!((0..300).collect::<Vec<_>>());
        assert_eq!(redact(&many, ""), many);
    }
    #[test]
    fn screenshots_require_session_ownership_and_missing_groups_are_omitted_whole() {
        let dir = tempfile::tempdir().unwrap();
        let session = Utc::now();
        let captures = dir.path().join("debug/captures");
        let logs = dir.path().join("logs");
        fs::create_dir_all(&captures).unwrap();
        fs::create_dir(&logs).unwrap();
        let stamp = session + chrono::Duration::seconds(1);
        let own = format!("{}-aaaaaaaa.png", stamp.format("%Y%m%dT%H%M%S%6fZ"));
        let other = format!("{}-bbbbbbbb.png", stamp.format("%Y%m%dT%H%M%S%6fZ"));
        for name in [&own, &other] {
            fs::write(captures.join(name), b"\x89PNG\r\n\x1a\nfixture").unwrap();
        }
        let records = [
            json!({"app_session":session,"timestamp":stamp,"diagnostic_image":{"category":"captures","file":own}}),
            json!({"app_session":stamp,"timestamp":stamp,"diagnostic_image":{"category":"captures","file":other}}),
        ];
        fs::write(
            logs.join(format!("{}.jsonl", Local::now().date_naive())),
            records.iter().map(|r| format!("{r}\n")).collect::<String>(),
        )
        .unwrap();
        let id = attempt(dir.path(), session, stamp, &[50, 50]);
        fs::remove_file(
            dir.path()
                .join("debug/recognition")
                .join(&id)
                .join("region_1.png"),
        )
        .unwrap();
        let (files, omissions) = collect(
            dir.path(),
            session,
            &settings(),
            &json!({}),
            true,
            MAX_BUNDLE_BYTES,
        )
        .unwrap();
        let images: Vec<_> = files.iter().filter(|(n, _)| n.ends_with(".png")).collect();
        assert_eq!(images.len(), 1);
        assert!(images[0].0.ends_with(&own));
        assert!(omissions.iter().any(|o| o["group"] == id));
    }
}
