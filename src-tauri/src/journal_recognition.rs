//! A cancellable, owned helper process. It never starts/adopts the automation service.
use crate::engine::EngineSupervisor;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
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
        state.active = Some((id.to_string(), sender));
        Ok((RequestGuard(self.clone(), id.to_string()), receiver))
    }
    fn cancel(&self, id: &str) {
        let mut state = self.0.lock().unwrap();
        if let Some((active, sender)) = &state.active {
            if active == id {
                let _ = sender.send(true);
                return;
            }
        }
        // Handles cancellation racing the async command's initial scheduling.
        state.cancelled_before_start.insert(id.to_string());
    }
    pub fn cancel_active(&self) {
        if let Some((_, sender)) = &self.0.lock().unwrap().active {
            let _ = sender.send(true);
        }
    }
}

#[tauri::command]
pub fn journal_cancel_recognition(request_id: String, state: State<'_, RecognitionState>) {
    if uuid::Uuid::parse_str(&request_id).is_ok() {
        state.cancel(&request_id);
    }
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
        "开始识别一楼地图 / Starting first-floor recognition",
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
    let work = async {
        let _automation_guard = supervisor.begin_recognition().await?;
        recognize(
            &app,
            &request_id,
            season,
            difficulty,
            display,
            window_handle,
        )
        .await
    };
    let outcome = tokio::select! {
        biased;
        _ = cancelled.changed() => Err("识图已取消 / Recognition cancelled".into()),
        result = tokio::time::timeout(Duration::from_secs(120), work) =>
            result.unwrap_or_else(|_| Err("识图超时，请缩小地图包分类后重试 / Recognition timed out".to_string())),
    };
    let elapsed = started.elapsed().as_millis();
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
            json!({"request_id":request_id,"stage":if *cancelled.borrow() {"cancelled"} else {"failed"},"elapsed_ms":elapsed}),
        ),
    }
    outcome
}

async fn recognize(
    app: &AppHandle,
    id: &str,
    season: String,
    difficulty: String,
    display: String,
    window_handle: Option<isize>,
) -> Result<Value, String> {
    let templates = tauri::async_runtime::spawn_blocking(move || {
        crate::journal::recognition_templates(&season, &difficulty, &display)
    })
    .await
    .map_err(|error| error.to_string())??;
    crate::logs::record(
        app,
        "debug",
        "journal.recognition",
        "地图模板索引就绪 / Map templates indexed",
        json!({"request_id":id,"template_count":templates.as_array().map(Vec::len).unwrap_or(0)}),
    );
    let path = crate::engine::locate_engine(
        &app.path()
            .resource_dir()
            .map_err(|error| error.to_string())?,
    )?;
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
    let request = json!({"request_id": id, "templates": templates, "window_handle": window_handle});
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
            .take(2 * 1024 * 1024)
            .read_line(&mut line)
            .await
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        if read >= 2 * 1024 * 1024 {
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
                    json!({"request_id":id,"stage":stage,"completed":message["completed"],"total":message["total"]}),
                );
                let _ = app.emit("journal-recognition-progress", &message);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_is_request_scoped_and_pre_start_safe() {
        let state = RecognitionState::default();
        let first = uuid::Uuid::new_v4().to_string();
        state.cancel(&first);
        let other = uuid::Uuid::new_v4().to_string();
        state.cancel(&other);
        assert!(state.begin(&first).is_err());
        assert!(state.begin(&other).is_err());
        let second = uuid::Uuid::new_v4().to_string();
        let (guard, receiver) = state.begin(&second).unwrap();
        state.cancel(&first);
        assert!(!*receiver.borrow());
        assert!(state.begin(&uuid::Uuid::new_v4().to_string()).is_err());
        state.cancel(&second);
        assert!(*receiver.borrow());
        drop(guard);
        assert!(state.begin(&uuid::Uuid::new_v4().to_string()).is_ok());
    }
}
