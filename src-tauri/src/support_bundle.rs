//! Local session diagnostics, frozen into a temporary ZIP before saving.
use crate::diagnostic_files::is_link;
use chrono::{DateTime, Duration, Local, NaiveDate, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Manager};

#[derive(Clone, Serialize)]
struct BundleFile {
    name: String,
    size: u64,
}
struct Bundle {
    id: String,
    archive: tempfile::NamedTempFile,
    files: Vec<BundleFile>,
    omissions: Vec<Value>,
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
const MAX_BUNDLE_BYTES: usize = 100_000_000;

#[derive(Clone, Serialize)]
struct DiagnosticSession {
    id: String,
    started: Option<DateTime<Utc>>,
    last_record: DateTime<Utc>,
    legacy: bool,
    current: bool,
    log_records: usize,
    reports: usize,
    images: usize,
}
fn session_key(stamp: DateTime<Utc>) -> String {
    json!(stamp).as_str().unwrap().to_owned()
}
fn stamp(value: &Value) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value.as_str()?)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}
fn log_paths(root: &Path, now: DateTime<Utc>) -> Result<Vec<PathBuf>, String> {
    let directory = root.join("logs");
    if !directory.exists() {
        return Ok(Vec::new());
    }
    if directory.ancestors().any(is_link) {
        return Err("Log directory cannot be a link".into());
    }
    let first_day = now.with_timezone(&Local).date_naive() - Duration::days(7);
    let mut paths: Vec<_> = fs::read_dir(directory)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && !is_link(p)
                && p.extension().and_then(|s| s.to_str()) == Some("jsonl")
                && p.file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                    .is_some_and(|d| d >= first_day)
        })
        .collect();
    paths.sort();
    Ok(paths)
}
fn read_logs(
    root: &Path,
    now: DateTime<Utc>,
    mut visit: impl FnMut(&str, &Value) -> Result<(), String>,
) -> Result<(), String> {
    for path in log_paths(root, now)? {
        let legacy = format!("legacy:{}", path.file_stem().unwrap().to_string_lossy());
        let file = fs::File::open(&path).map_err(|e| e.to_string())?;
        // A bounded line reader also prevents a malformed local file from
        // allocating memory proportional to the entire session history.
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        loop {
            buffer.clear();
            let count = reader
                .by_ref()
                .take(8 * 1024 * 1024 + 1)
                .read_until(b'\n', &mut buffer)
                .map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            if count > 8 * 1024 * 1024 {
                while !buffer.ends_with(b"\n") {
                    buffer.clear();
                    if reader
                        .by_ref()
                        .take(64 * 1024)
                        .read_until(b'\n', &mut buffer)
                        .map_err(|e| e.to_string())?
                        == 0
                    {
                        break;
                    }
                }
                return Err("日志单条记录过大，请检查日志 / A log record exceeds 8 MiB".into());
            }
            let Ok(value) = serde_json::from_slice::<Value>(&buffer) else {
                continue;
            };
            let key = stamp(&value["app_session"])
                .map(session_key)
                .unwrap_or_else(|| legacy.clone());
            visit(&key, &value)?;
        }
    }
    Ok(())
}
fn reports(
    root: &Path,
    now: DateTime<Utc>,
    omissions: &mut Vec<Value>,
) -> Result<Vec<(DateTime<Utc>, PathBuf)>, String> {
    let mut found = Vec::new();
    for path in crate::diagnostic_files::attempts(&root.join("debug"))? {
        let report = crate::diagnostic_files::read_report(&path);
        let started = report.as_ref().ok().and_then(|r| stamp(&r["started"]));
        let session = report.as_ref().ok().and_then(|r| stamp(&r["app_session"]));
        match (started, session) {
            (Some(started), Some(_)) if started >= now - Duration::days(7) => {
                found.push((started, path))
            }
            (Some(_), Some(_)) => {}
            _ => {
                // Ownership is unknown: disclose the missing report, but never
                // guess a session or attach its pictures to the selected one.
                let warning = json!({"group":path.file_name().unwrap().to_string_lossy(),"reason":"unreadable_report","scope":"unknown_session"});
                if !omissions.contains(&warning) {
                    omissions.push(warning);
                }
            }
        }
    }
    found.sort_by_key(|(s, _)| std::cmp::Reverse(*s));
    Ok(found)
}
fn owned_image(root: &Path, value: &Value, now: DateTime<Utc>) -> Option<PathBuf> {
    let category = value["diagnostic_image"]["category"].as_str()?;
    if !matches!(category, "captures" | "failures") {
        return None;
    }
    let name = value["diagnostic_image"]["file"].as_str()?;
    if Path::new(name).file_name()?.to_str()? != name {
        return None;
    }
    let path = root.join("debug").join(category).join(name);
    if path.ancestors().any(is_link) || !path.is_file() {
        return None;
    }
    crate::diagnostic_files::capture_time(&path).filter(|t| *t >= now - Duration::days(3))?;
    Some(path)
}
fn report_images(directory: &Path, report: &Value) -> Vec<(String, PathBuf)> {
    let id = directory.file_name().unwrap().to_string_lossy();
    report["images"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let name = entry["file"].as_str()?;
            name.strip_suffix(".png")
                .filter(|s| crate::diagnostic_files::image_kind(s))?;
            Some((format!("recognition/{id}/{name}"), directory.join(name)))
        })
        .collect()
}
fn catalog(
    root: &Path,
    current: DateTime<Utc>,
    now: DateTime<Utc>,
    omissions: &mut Vec<Value>,
) -> Result<Vec<DiagnosticSession>, String> {
    let current_key = session_key(current);
    let mut sessions: BTreeMap<String, DiagnosticSession> = BTreeMap::new();
    let mut add = |id: String, time: DateTime<Utc>, logs: usize, reports: usize, images: usize| {
        let started = DateTime::parse_from_rfc3339(&id)
            .ok()
            .map(|t| t.with_timezone(&Utc));
        let s = sessions
            .entry(id.clone())
            .or_insert_with(|| DiagnosticSession {
                legacy: started.is_none(),
                started,
                current: id == current_key,
                id,
                last_record: time,
                log_records: 0,
                reports: 0,
                images: 0,
            });
        s.last_record = s.last_record.max(time);
        s.log_records += logs;
        s.reports += reports;
        s.images += images;
    };
    add(current_key.clone(), current, 0, 0, 0);
    let mut seen_images = HashSet::new();
    read_logs(root, now, |id, value| {
        let time = stamp(&value["timestamp"]).unwrap_or(current);
        let image = if !id.starts_with("legacy:") {
            owned_image(root, value, now)
        } else {
            None
        };
        let image_count =
            usize::from(image.is_some_and(|p| seen_images.insert((id.to_owned(), p))));
        add(id.to_owned(), time, 1, 0, image_count);
        Ok(())
    })?;
    for (time, path) in reports(root, now, omissions)? {
        let report = crate::diagnostic_files::read_report(&path)?;
        let Some(session) = stamp(&report["app_session"]) else {
            continue;
        };
        let images = if time >= now - Duration::days(3) {
            report_images(&path, &report)
                .iter()
                .filter(|(_, p)| p.is_file() && !p.ancestors().any(is_link))
                .count()
        } else {
            0
        };
        add(session_key(session), time, 0, 1, images);
    }
    let mut values: Vec<_> = sessions.into_values().collect();
    values.sort_by_key(|s| {
        (
            std::cmp::Reverse(s.current),
            std::cmp::Reverse(s.started.unwrap_or(s.last_record)),
        )
    });
    Ok(values)
}
fn log_name(key: &str, current: &str) -> Result<String, String> {
    if key == current {
        return Ok("session.redacted.jsonl".into());
    }
    let name = if let Some(date) = key.strip_prefix("legacy:") {
        NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|e| e.to_string())?;
        format!("legacy-{date}")
    } else {
        DateTime::parse_from_rfc3339(key)
            .map_err(|e| e.to_string())?
            .format("%Y%m%dT%H%M%S%9fZ")
            .to_string()
    };
    Ok(format!("logs/{name}.redacted.jsonl"))
}
fn add_file(
    zip: &mut zip::ZipWriter<&mut fs::File>,
    files: &mut Vec<BundleFile>,
    name: String,
    mut source: impl Read,
    size: u64,
) -> Result<(), String> {
    zip.start_file(
        &name,
        zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .large_file(size >= u32::MAX as u64),
    )
    .map_err(|e| e.to_string())?;
    let written = io::copy(&mut source, &mut *zip).map_err(|e| e.to_string())?;
    if written != size {
        return Err(format!(
            "诊断文件在预览时变化，请重试 / Diagnostic file changed: {name}"
        ));
    }
    files.push(BundleFile { name, size });
    Ok(())
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

fn prepare_bundle(
    root: &Path,
    current: DateTime<Utc>,
    now: DateTime<Utc>,
    settings: &Value,
    snapshot: &Value,
    include_images: bool,
    historical: &[String],
    limit: Option<u64>,
) -> Result<Bundle, String> {
    let mut omissions = Vec::new();
    let available = catalog(root, current, now, &mut omissions)?;
    let current_key = session_key(current);
    let mut selected: HashSet<String> = historical.iter().cloned().collect();
    if selected
        .iter()
        .any(|id| !available.iter().any(|s| &s.id == id))
    {
        return Err(
            "部分会话已不可用，请返回重新选择 / Some sessions are no longer available".into(),
        );
    }
    selected.insert(current_key.clone());
    let sessions: Vec<_> = available
        .into_iter()
        .filter(|s| selected.contains(&s.id))
        .collect();
    let budget = limit
        .map(|n| n.saturating_sub(128 * 1024))
        .unwrap_or(u64::MAX);
    let mut archive = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    let mut used = 0u64;
    {
        let mut zip = zip::ZipWriter::new(archive.as_file_mut());
        for (name, value) in [
            (
                "environment.json",
                json!({"assistant_version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,
                "architecture":std::env::consts::ARCH,"session_started":current,"snapshot_scope":"current_at_preview",
                "previewed_at":now,"engine":redact(snapshot,"")}),
            ),
            (
                "settings.redacted.json",
                redact(&crate::support::backup(settings)?["settings"], ""),
            ),
        ] {
            let bytes = serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?;
            used += bytes.len() as u64;
            add_file(
                &mut zip,
                &mut files,
                name.into(),
                bytes.as_slice(),
                bytes.len() as u64,
            )?;
        }
        // Spool only selected, redacted log rows. Multi-session data never forms
        // one giant Vec in memory, and other sessions cannot consume its budget.
        let mut logs: BTreeMap<String, BufWriter<tempfile::NamedTempFile>> = BTreeMap::new();
        for id in &selected {
            logs.insert(
                id.clone(),
                BufWriter::new(tempfile::NamedTempFile::new().map_err(|e| e.to_string())?),
            );
        }
        let mut groups: Vec<(DateTime<Utc>, String, Vec<(String, PathBuf)>)> = Vec::new();
        let mut seen_images = HashSet::new();
        read_logs(root, now, |id, value| {
            if let Some(file) = logs.get_mut(id) {
                serde_json::to_writer(&mut *file, &redact(value, "")).map_err(|e| e.to_string())?;
                file.write_all(b"\n").map_err(|e| e.to_string())?;
                if include_images && !id.starts_with("legacy:") {
                    if let Some(path) =
                        owned_image(root, value, now).filter(|p| seen_images.insert(p.clone()))
                    {
                        let category = value["diagnostic_image"]["category"].as_str().unwrap();
                        let name = path.file_name().unwrap().to_string_lossy();
                        let key = format!("{category}/{name}");
                        groups.push((
                            crate::diagnostic_files::capture_time(&path).unwrap(),
                            key.clone(),
                            vec![(format!("images/{key}"), path)],
                        ));
                    }
                }
            }
            Ok(())
        })?;
        // Current logs take precedence for the bounded single-session mode.
        let mut log_keys: Vec<_> = logs.keys().cloned().collect();
        log_keys.sort_by_key(|id| id != &current_key);
        for id in log_keys {
            let writer = logs.get_mut(&id).unwrap();
            writer.flush().map_err(|e| e.to_string())?;
            let file = writer.get_mut();
            let total = file.as_file().metadata().map_err(|e| e.to_string())?.len();
            let remaining = budget.saturating_sub(used);
            let offset = total.saturating_sub(remaining);
            file.seek(SeekFrom::Start(offset))
                .map_err(|e| e.to_string())?;
            let mut reader = BufReader::new(file.as_file_mut());
            if offset > 0 {
                let mut partial = Vec::new();
                reader
                    .read_until(b'\n', &mut partial)
                    .map_err(|e| e.to_string())?;
                omissions.push(json!({"group":log_name(&id,&current_key)?,"reason":"size_limit"}));
            }
            let start = reader.stream_position().map_err(|e| e.to_string())?;
            let size = total - start;
            add_file(
                &mut zip,
                &mut files,
                log_name(&id, &current_key)?,
                reader.take(size),
                size,
            )?;
            used += size;
        }
        for (time, directory) in reports(root, now, &mut omissions)? {
            let report = crate::diagnostic_files::read_report(&directory)?;
            if !stamp(&report["app_session"]).is_some_and(|s| selected.contains(&session_key(s))) {
                continue;
            }
            let id = directory.file_name().unwrap().to_string_lossy().to_string();
            let name = format!("recognition/{id}/flow.redacted.json");
            let bytes =
                serde_json::to_vec_pretty(&redact(&report, "")).map_err(|e| e.to_string())?;
            if used + bytes.len() as u64 > budget {
                omissions.push(json!({"group":name,"reason":"size_limit"}));
                continue;
            }
            used += bytes.len() as u64;
            add_file(
                &mut zip,
                &mut files,
                name,
                bytes.as_slice(),
                bytes.len() as u64,
            )?;
            if include_images {
                let paths = report_images(&directory, &report);
                if time >= now - Duration::days(3) {
                    groups.push((time, id, paths));
                } else if !paths.is_empty() {
                    omissions.push(json!({"group":id,"reason":"images_expired"}));
                }
            }
        }
        groups.sort_by_key(|(time, _, _)| std::cmp::Reverse(*time));
        for (_, id, paths) in groups {
            if paths.is_empty() {
                continue;
            }
            let mut opened = Vec::new();
            let mut size = 0u64;
            let mut reason = None;
            for (name, path) in &paths {
                if path.ancestors().any(is_link) {
                    reason = Some("unsafe_file");
                    break;
                }
                let result = (|| -> io::Result<_> {
                    let mut file = fs::File::open(path)?;
                    let length = file.metadata()?.len();
                    let mut header = [0; 8];
                    file.read_exact(&mut header)?;
                    if header != *b"\x89PNG\r\n\x1a\n" {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid PNG"));
                    }
                    file.seek(SeekFrom::Start(0))?;
                    Ok((file, length))
                })();
                match result {
                    Ok((file, length)) => {
                        size = size.saturating_add(length);
                        opened.push((name.clone(), file, length));
                    }
                    Err(_) => {
                        reason = Some("missing_or_invalid_image");
                        break;
                    }
                }
            }
            if used.saturating_add(size) > budget {
                reason = Some("size_limit");
            }
            if let Some(reason) = reason {
                omissions.push(json!({"group":id,"reason":reason,"files":paths.iter().map(|(n,_)|n).collect::<Vec<_>>()}));
                continue;
            }
            for (name, file, length) in opened {
                add_file(&mut zip, &mut files, name, file.take(length), length)?;
            }
            used += size;
        }
        let manifest=serde_json::to_vec_pretty(&json!({"session_started":current,"selected_sessions":sessions,
            "previewed_at":now,"settings_scope":"current_at_preview","limit_bytes":limit,"include_images":include_images,
            "screenshot_retention_days":3,"log_retention_days":7,"omissions":omissions,
            "files":files.iter().map(|f|json!({"name":f.name,"bytes":f.size})).collect::<Vec<_>>()
        })).map_err(|e|e.to_string())?;
        if limit.is_some_and(|n| used + manifest.len() as u64 + files.len() as u64 * 256 > n) {
            return Err("诊断清单超过容量，请选择更小范围 / Manifest exceeds bundle limit".into());
        }
        add_file(
            &mut zip,
            &mut files,
            "manifest.json".into(),
            manifest.as_slice(),
            manifest.len() as u64,
        )?;
        zip.finish().map_err(|e| e.to_string())?;
    }
    if limit.is_some_and(|n| {
        archive
            .as_file()
            .metadata()
            .map(|m| m.len() > n)
            .unwrap_or(true)
    }) {
        return Err("Diagnostic ZIP exceeds limit".into());
    }
    Ok(Bundle {
        id: uuid::Uuid::new_v4().to_string(),
        archive,
        files,
        omissions,
    })
}

#[tauri::command]
pub async fn list_diagnostic_sessions(app: AppHandle) -> Result<Value, String> {
    crate::logs::record(
        &app,
        "info",
        "support.diagnostics",
        "正在读取诊断会话 / Reading diagnostic sessions",
        json!({}),
    );
    let reporter = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<BundleState>();
        app.state::<crate::journal_recognition::RecognitionState>()
            .save_diagnostics()?;
        let root = crate::config::user_path()?
            .parent()
            .ok_or("Missing data root")?
            .to_path_buf();
        let mut warnings = Vec::new();
        let sessions = catalog(&root, state.started, Utc::now(), &mut warnings)?;
        crate::logs::record(
            &app,
            "info",
            "support.diagnostics",
            "诊断会话列表就绪 / Diagnostic sessions ready",
            json!({"sessions":sessions.len(),"warnings":warnings}),
        );
        Ok(json!({"sessions":sessions,"current":session_key(state.started),"warnings":warnings}))
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|result| result);
    if let Err(error) = &result {
        crate::logs::record(
            &reporter,
            "error",
            "support.diagnostics",
            "诊断会话读取失败 / Reading diagnostic sessions failed",
            json!({"error":error}),
        );
    }
    result
}
#[tauri::command]
pub async fn preview_diagnostics(
    app: AppHandle,
    include_images: bool,
    historical_sessions: Option<Vec<String>>,
) -> Result<Value, String> {
    let historical = historical_sessions.unwrap_or_default();
    crate::logs::record(
        &app,
        "info",
        "support.diagnostics",
        "开始预览诊断包 / Preparing diagnostics",
        json!({"include_images":include_images,"historical_sessions":historical}),
    );
    let snapshot = serde_json::to_value(
        app.state::<crate::engine::EngineSupervisor>()
            .snapshot()
            .await,
    )
    .map_err(|e| e.to_string())?;
    let reporter = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let state=app.state::<BundleState>();
        let mut prepared=state.prepared.lock().map_err(|e|e.to_string())?;
        *prepared=None;
        let root=crate::config::user_path()?.parent().ok_or("Missing data root")?.to_path_buf();
        app.state::<crate::journal_recognition::RecognitionState>().save_diagnostics()?;
        if let Err(error)=crate::diagnostic_files::prune(&root.join("debug"),Utc::now()) {
            crate::logs::record(&app,"warning","support.diagnostics","旧截图暂时无法清理 / Some expired images could not be removed",json!({"error":error}));
        }
        let has_history=historical.iter().any(|s|s!=&session_key(state.started));
        let bundle=prepare_bundle(&root,state.started,Utc::now(),&crate::config::load()?,&snapshot,include_images,&historical,if has_history {None}else{Some(MAX_BUNDLE_BYTES as u64)})?;
        let response=json!({"id":bundle.id,"files":bundle.files,"truncated":!bundle.omissions.is_empty(),"omissions":bundle.omissions});
        crate::logs::record(&app,"info","support.diagnostics","诊断包预览就绪 / Diagnostics ready",json!({"files":bundle.files.len(),"bytes":bundle.files.iter().map(|f|f.size).sum::<u64>(),"historical_sessions":historical,"omissions":bundle.omissions}));
        *prepared=Some(bundle);Ok(response)
    }).await.map_err(|e|e.to_string()).and_then(|result|result);
    if let Err(error) = &result {
        crate::logs::record(
            &reporter,
            "error",
            "support.diagnostics",
            "诊断包预览失败 / Preparing diagnostics failed",
            json!({"error":error}),
        );
    }
    result
}
#[tauri::command]
pub async fn discard_diagnostics(app: AppHandle, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<BundleState>();
        let mut prepared = state.prepared.lock().map_err(|e| e.to_string())?;
        if prepared.as_ref().is_some_and(|b| b.id == id) {
            *prepared = None;
            crate::logs::record(
                &app,
                "info",
                "support.diagnostics",
                "临时诊断包已释放 / Diagnostic preview discarded",
                json!({"bundle_id":id}),
            );
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn export_diagnostics(app: AppHandle, id: String, path: String) -> Result<(), String> {
    crate::logs::record(
        &app,
        "info",
        "support.diagnostics",
        "正在保存诊断包 / Saving diagnostics",
        json!({"bundle_id":id}),
    );
    let reporter = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<BundleState>();
        let mut prepared = state.prepared.lock().map_err(|e| e.to_string())?;
        let bundle = prepared
            .as_mut()
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
        bundle
            .archive
            .seek(SeekFrom::Start(0))
            .map_err(|e| e.to_string())?;
        io::copy(bundle.archive.as_file_mut(), temporary.as_file_mut())
            .map_err(|e| e.to_string())?;
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
    .map_err(|e| e.to_string())
    .and_then(|result| result);
    if let Err(error) = &result {
        crate::logs::record(
            &reporter,
            "error",
            "support.diagnostics",
            "诊断包保存失败 / Saving diagnostics failed",
            json!({"error":error}),
        );
    }
    result
}

#[cfg(test)]
fn collect(
    root: &Path,
    started: DateTime<Utc>,
    settings: &Value,
    snapshot: &Value,
    images: bool,
    limit: usize,
) -> Result<(Vec<(String, Vec<u8>)>, Vec<Value>), String> {
    let b = prepare_bundle(
        root,
        started,
        started,
        settings,
        snapshot,
        images,
        &[],
        Some(limit as u64),
    )?;
    let mut zip = zip::ZipArchive::new(b.archive.reopen().unwrap()).unwrap();
    let mut files = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        let name = f.name().unwrap().to_string();
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes).unwrap();
        files.push((name, bytes));
    }
    Ok((files, b.omissions))
}
#[cfg(test)]
fn session_logs(
    directory: &Path,
    started: DateTime<Utc>,
    limit: usize,
) -> Result<(Vec<u8>, bool), String> {
    let key = session_key(started);
    let mut bytes = Vec::new();
    let mut truncated = false;
    read_logs(directory.parent().unwrap(), started, |id, value| {
        if id == key {
            let row = serde_json::to_vec(&redact(value, "")).unwrap();
            if bytes.len() + row.len() + 1 <= limit {
                bytes.extend(row);
                bytes.push(b'\n');
            } else {
                truncated = true;
            }
        }
        Ok(())
    })?;
    Ok((bytes, truncated))
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
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("logs");
        fs::create_dir(&directory).unwrap();
        let dir = directory.as_path();
        let session = Utc::now();
        let trace = "detail".repeat(1000);
        let rows = [
            json!({"timestamp":session,"app_session":session,"details":trace,"token":"secret-value"}),
            json!({"timestamp":session,"app_session":session - chrono::Duration::hours(1),"message":"previous-session"}),
        ];
        fs::write(
            dir.join(format!("{}.jsonl", Local::now().date_naive())),
            rows.iter().map(|r| format!("{r}\n")).collect::<String>(),
        )
        .unwrap();
        let (bytes, truncated) = session_logs(dir, session, MAX_BUNDLE_BYTES).unwrap();
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
    #[test]
    fn historical_selection_is_explicit_and_obeys_image_expiry() {
        let root = tempfile::tempdir().unwrap();
        let now = Utc::now();
        let recent = now - Duration::days(2);
        let older = now - Duration::days(4);
        let expired = now - Duration::days(8);
        let current_id = attempt(root.path(), now, now, &[40]);
        let recent_id = attempt(root.path(), recent, recent, &[40]);
        let older_id = attempt(root.path(), older, older, &[40]);
        let expired_id = attempt(root.path(), expired, expired, &[40]);
        let logs = root.path().join("logs");
        fs::create_dir(&logs).unwrap();
        fs::write(
            logs.join(format!("{}.jsonl", Local::now().date_naive())),
            format!(
                "{}\n{}\n{}\n",
                json!({"timestamp":now,"app_session":now,"message":"current"}),
                json!({"timestamp":now,"app_session":recent,"message":"old-app-concurrent"}),
                json!({"timestamp":now,"message":"legacy-unowned"})
            ),
        )
        .unwrap();
        let listed = catalog(root.path(), now, now, &mut Vec::new()).unwrap();
        assert_eq!(listed.len(), 4);
        assert_eq!(
            listed
                .iter()
                .find(|s| s.id == session_key(older))
                .unwrap()
                .images,
            0
        );
        let history = vec![
            session_key(recent),
            session_key(older),
            format!("legacy:{}", Local::now().date_naive()),
        ];
        let bundle = prepare_bundle(
            root.path(),
            now,
            now,
            &settings(),
            &json!({}),
            true,
            &history,
            None,
        )
        .unwrap();
        let names: Vec<_> = bundle.files.iter().map(|f| f.name.as_str()).collect();
        for id in [&current_id, &recent_id, &older_id] {
            assert!(names.contains(&format!("recognition/{id}/flow.redacted.json").as_str()));
        }
        assert!(names.contains(&format!("recognition/{recent_id}/region_0.png").as_str()));
        assert!(!names
            .iter()
            .any(|n| n.contains(&expired_id) || (n.contains(&older_id) && n.ends_with(".png"))));
        assert_eq!(names.iter().filter(|n| n.ends_with("jsonl")).count(), 4);
        assert!(prepare_bundle(
            root.path(),
            now,
            now,
            &settings(),
            &json!({}),
            true,
            &["invented".into()],
            None
        )
        .is_err());
        let (current, _) = collect(
            root.path(),
            now,
            &settings(),
            &json!({}),
            true,
            MAX_BUNDLE_BYTES,
        )
        .unwrap();
        let text = String::from_utf8(
            current
                .iter()
                .find(|(n, _)| n == "session.redacted.jsonl")
                .unwrap()
                .1
                .clone(),
        )
        .unwrap();
        assert!(!text.contains("old-app-concurrent") && !text.contains("legacy-unowned"));
        assert!(!current.iter().any(|(n, _)| n.contains(&recent_id)));
        let no_images = prepare_bundle(
            root.path(),
            now,
            now,
            &settings(),
            &json!({}),
            false,
            &history,
            None,
        )
        .unwrap();
        assert!(!no_images.files.iter().any(|f| f.name.ends_with(".png")));
    }
    #[test]
    fn unreadable_reports_are_disclosed_without_guessing_session_ownership() {
        let root = tempfile::tempdir().unwrap();
        let now = Utc::now();
        let id = attempt(root.path(), now, now, &[40]);
        fs::write(
            root.path()
                .join("debug/recognition")
                .join(&id)
                .join("flow.json"),
            "{broken",
        )
        .unwrap();
        let mut warnings = Vec::new();
        let sessions = catalog(root.path(), now, now, &mut warnings).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(warnings.len(), 1);
        let bundle = prepare_bundle(
            root.path(),
            now,
            now,
            &settings(),
            &json!({}),
            true,
            &[],
            Some(MAX_BUNDLE_BYTES as u64),
        )
        .unwrap();
        assert_eq!(bundle.omissions.len(), 1);
        assert_eq!(bundle.omissions[0]["reason"], "unreadable_report");
        assert_eq!(bundle.omissions[0]["scope"], "unknown_session");
        assert!(!bundle.files.iter().any(|f| f.name.contains(&id)));
    }
    #[test]
    fn multiple_sessions_stream_past_100mb_and_preview_remains_frozen() {
        let root = tempfile::tempdir().unwrap();
        let now = Utc::now();
        let old = now - Duration::hours(1);
        let current = attempt(root.path(), now, now, &[8]);
        let previous = attempt(root.path(), old, old, &[8]);
        let first = root
            .path()
            .join("debug/recognition")
            .join(&current)
            .join("region_0.png");
        let second = root
            .path()
            .join("debug/recognition")
            .join(&previous)
            .join("region_0.png");
        fs::OpenOptions::new()
            .write(true)
            .open(&first)
            .unwrap()
            .set_len(51_000_000)
            .unwrap();
        fs::OpenOptions::new()
            .write(true)
            .open(&second)
            .unwrap()
            .set_len(51_000_000)
            .unwrap();
        let bundle = prepare_bundle(
            root.path(),
            now,
            now,
            &settings(),
            &json!({}),
            true,
            &[session_key(old)],
            None,
        )
        .unwrap();
        assert!(bundle.files.iter().map(|f| f.size).sum::<u64>() > 100_000_000);
        let frozen = bundle.archive.path().to_path_buf();
        assert!(frozen.exists());
        fs::write(&first, b"changed after preview").unwrap();
        let mut zip = zip::ZipArchive::new(bundle.archive.reopen().unwrap()).unwrap();
        let mut entry = zip
            .by_name(&format!("recognition/{current}/region_0.png"))
            .unwrap();
        assert_eq!(entry.size(), 51_000_000);
        let mut header = [0; 8];
        entry.read_exact(&mut header).unwrap();
        assert_eq!(&header, b"\x89PNG\r\n\x1a\n");
        drop(entry);
        drop(zip);
        drop(bundle);
        assert!(!frozen.exists());
    }
}
