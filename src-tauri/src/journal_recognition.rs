//! A cancellable, owned helper process. It never starts/adopts the automation service.
use crate::engine::EngineSupervisor;
use base64::Engine;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    sync::watch,
};

#[derive(Default)]
struct Requests {
    active: Option<(String, watch::Sender<bool>)>,
    cancelled_before_start: HashSet<String>,
    latest: Option<RecognitionDiagnostics>,
    cancel_reason: Option<String>,
}

#[derive(Clone)]
pub(crate) struct RecognitionDiagnostics {
    pub report: Value,
    directory: PathBuf,
}
impl RecognitionDiagnostics {
    fn save(&self) -> Result<(), String> {
        use std::io::Write;
        crate::diagnostic_files::ensure_directory(&self.directory)?;
        let mut file =
            tempfile::NamedTempFile::new_in(&self.directory).map_err(|e| e.to_string())?;
        {
            // JSON serialization emits thousands of small tokens. Buffer them
            // without postponing the atomic, per-image diagnostic checkpoint.
            let mut writer = std::io::BufWriter::new(file.as_file_mut());
            serde_json::to_writer(&mut writer, &crate::logs::redacted(&self.report))
                .map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
        }
        file.persist(self.directory.join("flow.json"))
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
#[derive(Clone, Default)]
pub struct RecognitionState(Arc<Mutex<Requests>>);

struct RequestGuard(RecognitionState, String);
impl Drop for RequestGuard {
    fn drop(&mut self) {
        let mut state = self.0 .0.lock().unwrap();
        if state.active.as_ref().is_some_and(|(id, _)| id == &self.1) {
            state.active = None;
        }
    }
}
impl RecognitionState {
    fn start_diagnostics(&self, id: &str, filters: Value, root: &Path) -> Result<(), String> {
        uuid::Uuid::parse_str(id).map_err(|e| e.to_string())?;
        self.0.lock().unwrap().latest = Some(RecognitionDiagnostics {
            report: json!({"request_id":id,"started":chrono::Utc::now(),"filters":filters,
                "app_session":crate::logs::session_started(),"status":"running","steps":[],"images":[]}),
            directory: root.join("recognition").join(id),
        });
        self.save_diagnostics()
    }

    pub(crate) fn save_diagnostics(&self) -> Result<(), String> {
        if let Some(latest) = &self.0.lock().unwrap().latest {
            latest.save()?;
        }
        Ok(())
    }

    fn feedback(
        &self,
        root: &Path,
        id: &str,
        verdict: &str,
        correct_key: Option<String>,
    ) -> Result<Value, String> {
        uuid::Uuid::parse_str(id).map_err(|_| "Invalid recognition request ID")?;
        if !matches!(verdict, "correct" | "incorrect" | "unsure") {
            return Err("Invalid feedback".into());
        }
        let mut state = self.0.lock().unwrap();
        let directory = root.join("recognition").join(id);
        let mut report = crate::diagnostic_files::read_report(&directory)?;
        if report["app_session"] != json!(crate::logs::session_started())
            || report["status"] != "completed"
        {
            return Err("识图记录已不可用 / Recognition record is unavailable".into());
        }
        let keys = report["steps"]
            .as_array()
            .and_then(|steps| steps.iter().find(|s| s["stage"] == "indexed"))
            .map(|s| &s["template_keys"]);
        if let Some(key) = &correct_key {
            if verdict != "incorrect"
                || !keys
                    .and_then(Value::as_array)
                    .is_some_and(|keys| keys.iter().any(|v| v == key))
            {
                return Err("请选择本次识图的地图 / Choose a map from this recognition".into());
            }
        }
        let feedback =
            json!({"verdict":verdict,"correct_key":correct_key,"updated":chrono::Utc::now()});
        report["feedback"] = feedback.clone();
        RecognitionDiagnostics {
            report: report.clone(),
            directory,
        }
        .save()?;
        if let Some(latest) = state
            .latest
            .as_mut()
            .filter(|d| d.report["request_id"] == id)
        {
            latest.report = report;
        }
        Ok(feedback)
    }

    fn record(&self, id: &str, mut step: Value) {
        let mut state = self.0.lock().unwrap();
        let Some(latest) = state
            .latest
            .as_mut()
            .filter(|d| d.report["request_id"] == id)
        else {
            return;
        };
        if step["stage"] == "difficulty_detected" {
            latest.report["filters"]["difficulty"] = step["difficulty"].clone();
        }
        if step["stage"] == "floor_detected" {
            latest.report["filters"]["floor"] = step["floor"].clone();
        }
        step["timestamp"] = json!(chrono::Utc::now());
        step["started_at"] = step["timestamp"].clone();
        step["started_elapsed_ms"] = step["elapsed_ms"].clone();
        let steps = latest.report["steps"].as_array_mut().unwrap();
        // Keep stage transitions and the latest counters, without duplicating
        // every map's progress in this compact report. Full events remain logs.
        if steps
            .last()
            .is_some_and(|last| last["stage"] == step["stage"] && step.get("completed").is_some())
        {
            if let Some(previous) = steps.pop() {
                step["started_at"] = previous["started_at"].clone();
                step["started_elapsed_ms"] = previous["started_elapsed_ms"].clone();
            }
        }
        steps.push(step);
    }

    fn retain_image(&self, id: &str, message: &Value) -> Result<(), String> {
        let kind = message["image_kind"].as_str().unwrap_or("");
        let encoded = message["png"].as_str().ok_or("Missing diagnostic image")?;
        if !crate::diagnostic_files::image_kind(kind) || encoded.len() > 90 * 1024 * 1024 {
            return Err("Invalid diagnostic image".into());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| "Invalid diagnostic image encoding")?;
        if bytes.len() > 64 * 1024 * 1024 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("Invalid or oversized diagnostic image".into());
        }
        let mut state = self.0.lock().unwrap();
        let Some(latest) = state
            .latest
            .as_mut()
            .filter(|d| d.report["request_id"] == id)
        else {
            return Ok(());
        };
        let name = format!("{kind}.png");
        crate::diagnostic_files::ensure_directory(&latest.directory)?;
        let path = latest.directory.join(&name);
        if crate::diagnostic_files::is_link(&path) {
            return Err("Diagnostic image cannot be a link".into());
        }
        std::fs::write(path, &bytes).map_err(|e| e.to_string())?;
        latest.report["images"]
            .as_array_mut()
            .unwrap()
            .retain(|v| v["file"] != name);
        latest.report["images"].as_array_mut().unwrap().push(json!({
            "file":name,"width":message["width"],"height":message["height"],"bytes":bytes.len(),
            "elapsed_ms":message["elapsed_ms"]}));
        latest.save()
    }

    fn finish_diagnostics(&self, id: &str, status: &str) {
        if let Some(latest) = self
            .0
            .lock()
            .unwrap()
            .latest
            .as_mut()
            .filter(|d| d.report["request_id"] == id)
        {
            latest.report["status"] = json!(status);
        }
    }

    fn begin(&self, id: &str) -> Result<(RequestGuard, watch::Receiver<bool>), String> {
        uuid::Uuid::parse_str(id).map_err(|_| "Invalid recognition request ID")?;
        let mut state = self.0.lock().unwrap();
        if state.cancelled_before_start.remove(id) {
            return Err("识图已取消 / Recognition cancelled".into());
        }
        if state.active.is_some() {
            return Err("识图正在进行 / Recognition is already in progress".into());
        }
        let (sender, receiver) = watch::channel(false);
        state.cancel_reason = None;
        state.active = Some((id.to_string(), sender));
        Ok((RequestGuard(self.clone(), id.to_string()), receiver))
    }
    fn cancel(&self, id: &str, reason: &str) {
        let mut state = self.0.lock().unwrap();
        if let Some((active, sender)) = &state.active {
            if active == id {
                let _ = sender.send(true);
                state.cancel_reason = Some(reason.to_string());
                return;
            }
        }
        // Handles cancellation racing the async command's initial scheduling.
        state.cancelled_before_start.insert(id.to_string());
    }
    pub fn cancel_active(&self) {
        let mut state = self.0.lock().unwrap();
        if let Some((_, sender)) = &state.active {
            let _ = sender.send(true);
            state.cancel_reason = Some("application_shutdown".into());
        }
    }
}

#[tauri::command]
pub fn journal_cancel_recognition(
    request_id: String,
    reason: Option<String>,
    state: State<'_, RecognitionState>,
) {
    if uuid::Uuid::parse_str(&request_id).is_ok() {
        let reason = match reason.as_deref() {
            Some("filters_changed" | "page_left" | "view_disposed" | "restarted") => {
                reason.as_deref().unwrap()
            }
            _ => "user_cancelled",
        };
        state.cancel(&request_id, reason);
    }
}

#[tauri::command]
pub async fn journal_save_feedback(
    app: AppHandle,
    request_id: String,
    verdict: String,
    correct_key: Option<String>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let feedback = app.state::<RecognitionState>().feedback(
            &crate::diagnostic_files::root()?,
            &request_id,
            &verdict,
            correct_key,
        )?;
        crate::logs::record(
            &app,
            "info",
            "journal.feedback",
            "识图反馈已保存 / Recognition feedback saved",
            json!({"request_id":request_id,"feedback":feedback}),
        );
        Ok(feedback)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn journal_recognize(
    app: AppHandle,
    supervisor: State<'_, EngineSupervisor>,
    state: State<'_, RecognitionState>,
    request_id: String,
    season: String,
    difficulty: String,
    display: String,
    window_handle: Option<isize>,
) -> Result<Value, String> {
    let started = Instant::now();
    crate::logs::record(
        &app,
        "info",
        "journal.recognition",
        if season == "s1" {
            "开始识别 S1 当前楼层地图 / Starting S1 current-floor recognition"
        } else {
            "开始识别一楼地图 / Starting first-floor recognition"
        },
        json!({"request_id":request_id,"season":season,"difficulty":difficulty,"display":display,
            "trigger":if window_handle.is_some() {"shortcut"} else {"button"}}),
    );
    let (_request, mut cancelled) = state.begin(&request_id).inspect_err(|error| {
        crate::logs::record(
            &app,
            "warning",
            "journal.recognition",
            error,
            json!({"request_id":request_id,"stage":"rejected"}),
        );
    })?;
    let evidence = crate::diagnostic_files::root().and_then(|root| {
        diagnostic_warning(
            &app,
            &request_id,
            crate::diagnostic_files::prune(&root, chrono::Utc::now()),
        );
        state.start_diagnostics(
            &request_id,
            json!({"season":season,"difficulty":if season == "s1" {difficulty.as_str()} else {"auto"},"requested_difficulty":difficulty,"display":display,
        "floor":if season == "s1" {json!("auto")} else {json!(1)},"trigger":if window_handle.is_some() {"shortcut"} else {"button"}}),
            &root,
        )
    });
    diagnostic_warning(&app, &request_id, evidence);
    let work = async {
        let _automation_guard = tokio::select! {
            biased;
            _ = cancelled.changed() => return Err("识图已取消 / Recognition cancelled".into()),
            guard = supervisor.begin_recognition() => guard?,
        };
        recognize(
            &app,
            &state,
            &request_id,
            season,
            difficulty,
            display,
            window_handle,
            cancelled.clone(),
        )
        .await
    };
    let outcome = work.await;
    let elapsed = started.elapsed().as_millis();
    let status = if outcome.is_ok() {
        "completed"
    } else if *cancelled.borrow() {
        "cancelled"
    } else {
        "failed"
    };
    let cancel_reason = state.0.lock().unwrap().cancel_reason.clone();
    state.record(&request_id, match &outcome {
        Ok(result) => json!({"stage":status,"elapsed_ms":elapsed,"keys":result["keys"],"warnings":result["warnings"],"matches":result["matches"]}),
        Err(error) => json!({"stage":status,"elapsed_ms":elapsed,"error":error,"cancel_reason":cancel_reason}),
    });
    state.finish_diagnostics(&request_id, status);
    diagnostic_warning(&app, &request_id, state.save_diagnostics());
    match &outcome {
        Ok(result) => crate::logs::record(
            &app,
            if result["keys"]
                .as_array()
                .is_some_and(|keys| !keys.is_empty())
            {
                "info"
            } else {
                "warning"
            },
            "journal.recognition",
            if result["keys"]
                .as_array()
                .is_some_and(|keys| !keys.is_empty())
            {
                "识图完成，请核对候选 / Recognition completed; review candidates"
            } else {
                "未找到可靠地图候选 / No reliable map candidates"
            },
            json!({"request_id":request_id,"stage":"completed","elapsed_ms":elapsed,"keys":result["keys"],"warnings":result["warnings"],"matches":result["matches"]}),
        ),
        Err(error) => crate::logs::record(
            &app,
            if *cancelled.borrow() { "info" } else { "error" },
            "journal.recognition",
            error,
            json!({"request_id":request_id,"stage":if *cancelled.borrow() {"cancelled"} else {"failed"},"elapsed_ms":elapsed,"cancel_reason":cancel_reason}),
        ),
    }
    outcome
}

fn diagnostic_warning(app: &AppHandle, id: &str, result: Result<(), String>) {
    if let Err(error) = result {
        crate::logs::record(
            app,
            "warning",
            "journal.diagnostics",
            "识图证据保存失败 / Could not save recognition evidence",
            json!({"request_id":id,"error":error}),
        );
    }
}

async fn recognize(
    app: &AppHandle,
    state: &RecognitionState,
    id: &str,
    season: String,
    difficulty: String,
    display: String,
    window_handle: Option<isize>,
    mut cancelled: watch::Receiver<bool>,
) -> Result<Value, String> {
    let maps_app = app.clone();
    let request_season = season.clone();
    let request_difficulty = difficulty.clone();
    let templates = tauri::async_runtime::spawn_blocking(move || {
        crate::journal::ensure_builtins(&maps_app)?;
        if !["hard", "nightmare"].contains(&difficulty.as_str()) {
            return Err("无效地图难度 / Invalid map difficulty".to_string());
        }
        // S1 shares floor labels between difficulties; only index the selected pool.
        let hard = if season != "s1" || difficulty == "hard" {
            crate::journal::recognition_templates(&season, "hard", &display)?
        } else {
            json!([])
        };
        let nightmare = if season != "s1" || difficulty == "nightmare" {
            crate::journal::recognition_templates(&season, "nightmare", &display)?
        } else {
            json!([])
        };
        Ok::<_, String>(json!({"hard":hard,"nightmare":nightmare}))
    })
    .await
    .map_err(|error| error.to_string())??;
    crate::logs::record(
        app,
        "debug",
        "journal.recognition",
        "地图模板索引就绪 / Map template pools indexed",
        json!({"request_id":id,"hard":templates["hard"].as_array().map(Vec::len),"nightmare":templates["nightmare"].as_array().map(Vec::len)}),
    );
    let path = crate::engine::locate_engine(
        &app.path()
            .resource_dir()
            .map_err(|error| error.to_string())?,
    )?;
    if *cancelled.borrow() {
        return Err("识图已取消 / Recognition cancelled".into());
    }
    let mut child = crate::engine::engine_command(&path)
        .arg("--config")
        .arg(crate::config::active_path()?)
        .arg("--recognize-journal")
        .env("PYTHONIOENCODING", "utf-8")
        .env("OPENCV_IO_MAX_IMAGE_PIXELS", "16777216")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("无法启动识图进程 / Could not start recognition: {error}"))?;
    crate::logs::record(
        app,
        "debug",
        "journal.recognition",
        "独立识图进程已启动 / Recognition helper started",
        json!({"request_id":id,"pid":child.id(),"automation_started":false}),
    );
    let work = async {
        let stderr = child.stderr.take().ok_or("Missing recognition stderr")?;
        let stderr_app = app.clone();
        let stderr_id = id.to_string();
        let stderr_reader = tauri::async_runtime::spawn(async move {
            let mut errors = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = errors.next_line().await {
                let mut record = crate::logs::stderr_record(&line, "journal.stderr");
                record["request_id"] = json!(stderr_id);
                crate::logs::emit(&stderr_app, record);
            }
        });
        let mut input = child.stdin.take().ok_or("Missing recognition input")?;
        let request = json!({"request_id": id, "season":request_season, "difficulty":request_difficulty,
            "templates_by_difficulty": templates, "window_handle": window_handle});
        input
            .write_all(format!("{request}\n").as_bytes())
            .await
            .map_err(|error| error.to_string())?;
        drop(input);
        let mut output = BufReader::new(child.stdout.take().ok_or("Missing recognition output")?);
        let mut result = None;
        loop {
            let mut line = String::new();
            let read = (&mut output)
                .take(96 * 1024 * 1024)
                .read_line(&mut line)
                .await
                .map_err(|error| error.to_string())?;
            if read == 0 {
                break;
            }
            if read >= 96 * 1024 * 1024 {
                return Err("Recognition response exceeds limit".into());
            }
            let message: Value = serde_json::from_str(&line)
                .map_err(|error| format!("Invalid recognition response: {error}"))?;
            if message["request_id"] != id {
                return Err("Recognition request ID mismatch".into());
            }
            match message["type"].as_str() {
                Some("journal_recognition_progress") => {
                    let stage = message["stage"].as_str().unwrap_or("unknown");
                    let label = match stage {
                        "capturing" => "正在截图 / Capturing game window",
                        "difficulty" => "正在判断地图难度 / Detecting map difficulty",
                        "locating" => "正在定位地图 / Locating map region",
                        "matching" => "正在匹配地图 / Matching maps",
                        _ => "识图进度 / Recognition progress",
                    };
                    crate::logs::record(
                        app,
                        if stage == "matching" && message["completed"].as_u64().unwrap_or(0) > 0 {
                            "debug"
                        } else {
                            "info"
                        },
                        "journal.recognition",
                        label,
                        json!({"request_id":id,"stage":stage,"completed":message["completed"],"total":message["total"],"elapsed_ms":message["elapsed_ms"]}),
                    );
                    state.record(id, json!({"stage":stage,"completed":message["completed"],"total":message["total"],"elapsed_ms":message["elapsed_ms"]}));
                    let _ = app.emit("journal-recognition-progress", &message);
                }
                Some("journal_recognition_image") => {
                    diagnostic_warning(app, id, state.retain_image(id, &message));
                    let metadata = json!({"request_id":id,"stage":format!("{}_image",message["image_kind"].as_str().unwrap_or("")),
                    "width":message["width"],"height":message["height"],"elapsed_ms":message["elapsed_ms"]});
                    state.record(id, metadata.clone());
                    crate::logs::record(
                        app,
                        "debug",
                        "journal.recognition",
                        "收到识图截图 / Recognition image received",
                        metadata,
                    );
                }
                Some("journal_recognition_diagnostic") => {
                    state.record(id, message.clone());
                    crate::logs::record(
                        app,
                        "debug",
                        "journal.recognition",
                        "识图分析详情 / Recognition analysis",
                        message,
                    );
                }
                Some("journal_recognition_preview") => {
                    let preview = message["preview"].as_str().unwrap_or("");
                    if preview.len() > 1024 * 1024
                        || !preview.starts_with("data:image/jpeg;base64,")
                    {
                        return Err("Invalid recognition preview".into());
                    }
                    let _ = app.emit("journal-recognition-preview", &message);
                }
                Some("log") => crate::logs::emit(app, message),
                Some("journal_recognition_result") => result = Some(message),
                Some("error") => {
                    return Err(message["message"]
                        .as_str()
                        .unwrap_or("Recognition failed")
                        .to_string())
                }
                _ => return Err("Unexpected recognition response".into()),
            }
        }
        let status = child.wait().await.map_err(|error| error.to_string())?;
        let _ = stderr_reader.await;
        if !status.success() {
            return Err("识图进程异常退出 / Recognition process exited unexpectedly".into());
        }
        result.ok_or_else(|| "识图未返回结果 / Recognition returned no result".into())
    };
    let outcome = tokio::select! {
        biased;
        _ = cancelled.changed() => Err("识图已取消 / Recognition cancelled".into()),
        result = tokio::time::timeout(Duration::from_secs(120), work) =>
            result.unwrap_or_else(|_| Err("识图超时，请重试 / Recognition timed out".to_string())),
    };
    // Do not release the request/automation guards until the helper has actually exited.
    // This makes P restarts safe even while the old helper is doing CPU-bound matching.
    if child.try_wait().map_err(|e| e.to_string())?.is_none() {
        child
            .kill()
            .await
            .map_err(|e| format!("停止识图进程失败 / Could not stop recognition helper: {e}"))?;
        child.wait().await.map_err(|e| e.to_string())?;
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn large_diagnostic_checkpoint_is_complete_and_redacted_before_replacement() {
        let root = tempfile::tempdir().unwrap();
        let report = json!({"steps": (0..500).map(|index| json!({
            "stage":"template_region", "key":format!("triple:{index}"),
            "detail":"地图匹配记录", "access_token":"never-persist"
        })).collect::<Vec<_>>()});
        let mut diagnostics = RecognitionDiagnostics {
            report,
            directory: root.path().to_path_buf(),
        };
        for status in ["running", "completed"] {
            diagnostics.report["status"] = json!(status);
            diagnostics.save().unwrap();
            let bytes = std::fs::read(root.path().join("flow.json")).unwrap();
            assert!(bytes.len() > 8192);
            assert_eq!(
                serde_json::from_slice::<Value>(&bytes).unwrap(),
                crate::logs::redacted(&diagnostics.report)
            );
        }
    }

    #[test]
    fn feedback_is_editable_and_bound_to_a_completed_session_attempt() {
        let root = tempfile::tempdir().unwrap();
        let state = RecognitionState::default();
        let id = uuid::Uuid::new_v4().to_string();
        state
            .start_diagnostics(&id, json!({}), root.path())
            .unwrap();
        assert!(state.feedback(root.path(), &id, "correct", None).is_err());
        state.record(
            &id,
            json!({"stage":"indexed","template_keys":["lf:1","rf:2"]}),
        );
        state.finish_diagnostics(&id, "completed");
        state.save_diagnostics().unwrap();
        assert!(state
            .feedback(root.path(), &id, "incorrect", Some("lf:99".into()))
            .is_err());
        state
            .feedback(root.path(), &id, "incorrect", Some("rf:2".into()))
            .unwrap();
        state.save_diagnostics().unwrap();
        let directory = root.path().join("recognition").join(&id);
        assert_eq!(
            crate::diagnostic_files::read_report(&directory).unwrap()["feedback"]["correct_key"],
            "rf:2"
        );
        state.feedback(root.path(), &id, "unsure", None).unwrap();
        let mut old = crate::diagnostic_files::read_report(&directory).unwrap();
        assert_eq!(old["feedback"]["verdict"], "unsure");
        old["app_session"] = json!("previous");
        RecognitionDiagnostics {
            report: old,
            directory,
        }
        .save()
        .unwrap();
        assert!(state.feedback(root.path(), &id, "correct", None).is_err());
    }
    #[test]
    fn diagnostics_preserve_each_attempt_and_reject_late_images() {
        let root = tempfile::tempdir().unwrap();
        let state = RecognitionState::default();
        let first = uuid::Uuid::new_v4().to_string();
        state
            .start_diagnostics(&first, json!({"season":"s0","floor":1}), root.path())
            .unwrap();
        let image = json!({"image_kind":"full","width":320,"height":200,"elapsed_ms":15,
            "png":base64::engine::general_purpose::STANDARD.encode(b"\x89PNG\r\n\x1a\nfixture")});
        state.retain_image(&first, &image).unwrap();
        for completed in 0..1000 {
            state.record(&first, json!({"stage":"matching","completed":completed}));
        }
        state.finish_diagnostics(&first, "cancelled");
        state.save_diagnostics().unwrap();
        let first_dir = root.path().join("recognition").join(&first);
        let flow = crate::diagnostic_files::read_report(&first_dir).unwrap();
        assert_eq!(flow["status"], "cancelled");
        assert_eq!(flow["steps"][0]["completed"], 999);
        assert!(!flow.to_string().contains(image["png"].as_str().unwrap()));
        let second = uuid::Uuid::new_v4().to_string();
        state
            .start_diagnostics(&second, json!({"season":"s1","floor":"auto"}), root.path())
            .unwrap();
        state.record(
            &second,
            json!({"stage":"floor_detected","floor":"2","slot_count":4}),
        );
        state.save_diagnostics().unwrap();
        assert_eq!(
            crate::diagnostic_files::read_report(&root.path().join("recognition").join(&second))
                .unwrap()["filters"]["floor"],
            "2"
        );
        state.retain_image(&first, &image).unwrap();
        assert!(first_dir.join("full.png").exists());
        assert!(!root
            .path()
            .join("recognition")
            .join(&second)
            .join("full.png")
            .exists());
        assert_eq!(
            crate::diagnostic_files::attempts(root.path())
                .unwrap()
                .len(),
            2
        );
        assert!(state
            .retain_image(&second, &json!({"image_kind":"../full","png":"invalid"}))
            .is_err());
    }

    #[test]
    fn cancellation_is_request_scoped_and_pre_start_safe() {
        let state = RecognitionState::default();
        let first = uuid::Uuid::new_v4().to_string();
        state.cancel(&first, "user_cancelled");
        let other = uuid::Uuid::new_v4().to_string();
        state.cancel(&other, "user_cancelled");
        assert!(state.begin(&first).is_err());
        assert!(state.begin(&other).is_err());
        let second = uuid::Uuid::new_v4().to_string();
        let (guard, receiver) = state.begin(&second).unwrap();
        state.cancel(&first, "user_cancelled");
        assert!(!*receiver.borrow());
        assert!(state.begin(&uuid::Uuid::new_v4().to_string()).is_err());
        state.cancel(&second, "user_cancelled");
        assert!(*receiver.borrow());
        drop(guard);
        assert!(state.begin(&uuid::Uuid::new_v4().to_string()).is_ok());
    }
}
