use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use chrono::Utc;
use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
    sync::{oneshot, Mutex},
};

use super::ipc::{normalize_command, validate_event};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineSnapshot {
    pub status: String,
    pub pid: Option<u32>,
    pub engine_version: Option<String>,
    pub task_status: String,
    pub last_heartbeat: Option<String>,
    pub heartbeat_age_seconds: Option<f64>,
    pub last_error: Option<String>,
}

struct EngineProcess {
    child: Child,
    stdin: ChildStdin,
}

struct PendingDiagnostic {
    request_id: String,
    result_event: &'static str,
    sender: oneshot::Sender<Result<(), String>>,
}

impl PendingDiagnostic {
    fn result(&self, message: &Value) -> Option<Result<(), String>> {
        if message.get("request_id").and_then(Value::as_str) != Some(self.request_id.as_str()) {
            return None;
        }
        match message.get("type").and_then(Value::as_str) {
            Some("error") => Some(Err(message["message"]
                .as_str()
                .unwrap_or("Diagnostic failed")
                .to_string())),
            Some(kind) if kind == self.result_event => {
                // A failed window lookup emits its explanation in the following error event.
                if kind == "window_diagnostics" && message["found"] != true {
                    None
                } else {
                    Some(Ok(()))
                }
            }
            _ => None,
        }
    }
}

struct EngineState {
    process: Option<EngineProcess>,
    process_generation: u64,
    status: String,
    engine_version: Option<String>,
    task_status: String,
    last_heartbeat: Option<chrono::DateTime<Utc>>,
    heartbeat_timeout_seconds: f64,
    last_error: Option<String>,
    pending_tasks: Vec<String>,
    active_task_id: Option<String>,
    diagnostic: Option<PendingDiagnostic>,
    diagnostic_autostop_generation: Option<u64>,
}

#[derive(Clone)]
pub struct EngineSupervisor {
    inner: Arc<Mutex<EngineState>>,
    lifecycle: Arc<Mutex<()>>,
    diagnostics: Arc<Mutex<()>>,
    updating: Arc<AtomicBool>,
}

impl Default for EngineSupervisor {
    fn default() -> Self {
        Self {
            updating: Arc::new(AtomicBool::new(false)),
            lifecycle: Arc::new(Mutex::new(())),
            diagnostics: Arc::new(Mutex::new(())),
            inner: Arc::new(Mutex::new(EngineState {
                process: None,
                process_generation: 0,
                status: "stopped".into(),
                engine_version: None,
                task_status: "idle".into(),
                last_heartbeat: None,
                heartbeat_timeout_seconds: 10.0,
                last_error: None,
                pending_tasks: Vec::new(),
                active_task_id: None,
                diagnostic: None,
                diagnostic_autostop_generation: None,
            })),
        }
    }
}

impl EngineSupervisor {
    pub async fn start(&self, app: AppHandle) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        // An explicit start adopts an engine temporarily started by diagnostics.
        self.inner.lock().await.diagnostic_autostop_generation = None;
        self.start_locked(app).await?;
        self.send(json!({"type": "enable_window_layout"})).await
    }

    pub async fn run_diagnostic(&self, app: AppHandle, command: Value) -> Result<(), String> {
        let result_event = match command["type"].as_str() {
            Some("detect_window") => "window_diagnostics",
            Some("capture_frame") => "debug_frame",
            _ => return Err("Unsupported temporary diagnostic".into()),
        };
        let request_id = command["request_id"]
            .as_str()
            .ok_or("Diagnostic request ID is required")?
            .to_string();
        // Serializing these two short operations avoids one test shutting down another's engine.
        let _diagnostics = self.diagnostics.lock().await;
        let (sender, receiver) = oneshot::channel();
        let (generation, sent) = {
            let _lifecycle = self.lifecycle.lock().await;
            let was_running = self.has_process().await;
            self.start_locked(app).await?;
            let generation = {
                let mut state = self.inner.lock().await;
                if !was_running {
                    state.diagnostic_autostop_generation = Some(state.process_generation);
                }
                state.diagnostic = Some(PendingDiagnostic {
                    request_id,
                    result_event,
                    sender,
                });
                state.process_generation
            };
            (generation, self.send(command).await)
        };
        let result = match sent {
            Err(error) => Err(error),
            Ok(()) => match tokio::time::timeout(Duration::from_secs(30), receiver).await {
                Ok(Ok(result)) => result,
                Ok(Err(_)) => Err("诊断已中断：自动化服务已关闭".into()),
                Err(_) => Err("诊断超时：30 秒内未收到结果".into()),
            },
        };
        let cleanup = self.finish_diagnostic(generation).await;
        result.and(cleanup)
    }

    async fn finish_diagnostic(&self, generation: u64) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        let should_stop = {
            let mut state = self.inner.lock().await;
            if state.process_generation != generation {
                return Ok(());
            }
            state.diagnostic = None;
            state.diagnostic_autostop_generation.take() == Some(generation)
        };
        if should_stop {
            // Use normal shutdown so Python restores the game window and releases inputs.
            self.stop_locked().await?;
        }
        Ok(())
    }

    async fn start_locked(&self, app: AppHandle) -> Result<(), String> {
        if self.updating.load(Ordering::SeqCst) {
            return Err("Update installation is in progress".into());
        }
        let wait_for_existing_start = {
            let mut state = self.inner.lock().await;
            if let Some(process) = state.process.as_mut() {
                if process
                    .child
                    .try_wait()
                    .map_err(|error| error.to_string())?
                    .is_none()
                {
                    if state.status == "starting" {
                        true
                    } else {
                        return Ok(());
                    }
                } else {
                    state.process = None;
                    false
                }
            } else {
                state.status == "starting"
            }
        };
        if wait_for_existing_start {
            return self.wait_until_ready().await;
        }

        let process_generation = {
            let mut state = self.inner.lock().await;
            state.status = "starting".into();
            state.last_error = None;
            state.last_heartbeat = None;
            state.process_generation += 1;
            state.process_generation
        };

        let engine_path = match app
            .path()
            .resource_dir()
            .map_err(|error| error.to_string())
            .and_then(|root| locate_engine(&root))
        {
            Ok(path) => path,
            Err(error) => {
                self.fail_start(&error).await;
                return Err(error);
            }
        };
        let mut command = engine_command(&engine_path);
        let config_path = match crate::config::active_path() {
            Ok(path) => path,
            Err(error) => {
                self.fail_start(&error).await;
                return Err(error);
            }
        };
        let mut child = match command
            .arg("--config")
            .arg(config_path)
            .current_dir(engine_path.parent().unwrap_or_else(|| Path::new(".")))
            .env("PYTHONIOENCODING", "utf-8")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                let message = format!("Unable to start engine {}: {error}", engine_path.display());
                self.fail_start(&message).await;
                return Err(message);
            }
        };

        let stdin = child.stdin.take().ok_or("Python stdin was not available")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Python stdout was not available")?;
        let stderr = child
            .stderr
            .take()
            .ok_or("Python stderr was not available")?;

        let pid = child
            .id()
            .ok_or("Python engine did not expose a process id")?;

        {
            let mut state = self.inner.lock().await;
            state.status = "starting".into();
            state.process = Some(EngineProcess { child, stdin });
        }

        let (ready_sender, ready_receiver) = oneshot::channel();
        self.spawn_stdout_reader(app.clone(), stdout, ready_sender, process_generation);
        self.spawn_stderr_reader(app.clone(), stderr, process_generation);
        self.spawn_exit_monitor(app, pid);
        match tokio::time::timeout(Duration::from_secs(8), ready_receiver).await {
            Ok(Ok(Ok(()))) => Ok(()),
            Ok(Ok(Err(error))) => {
                self.fail_start(&error).await;
                Err(error)
            }
            Ok(Err(_)) => {
                let error = self
                    .last_error()
                    .await
                    .unwrap_or_else(|| "Automation service exited before it was ready".into());
                self.fail_start(&error).await;
                Err(error)
            }
            Err(_) => {
                let error = "Automation service did not become ready within 8 seconds".to_string();
                self.fail_start(&error).await;
                Err(error)
            }
        }
    }

    fn spawn_stdout_reader(
        &self,
        app: AppHandle,
        stdout: tokio::process::ChildStdout,
        ready_sender: oneshot::Sender<Result<(), String>>,
        process_generation: u64,
    ) {
        let supervisor = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut ready_sender = Some(ready_sender);
            let mut reader = BufReader::new(stdout);
            let mut bytes = Vec::new();
            loop {
                bytes.clear();
                match reader.read_until(b'\n', &mut bytes).await {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(error) => {
                        crate::logs::emit(
                            &app,
                            protocol_error("IPC_READ_FAILED", &error.to_string()),
                        );
                        break;
                    }
                }
                // A malformed event must not close the pipe and crash the child on its next write.
                let line = match std::str::from_utf8(&bytes) {
                    Ok(line) => line,
                    Err(error) => {
                        crate::logs::emit(
                            &app,
                            protocol_error(
                                "IPC_ENCODING_ERROR",
                                &format!("Engine output must be UTF-8: {error}"),
                            ),
                        );
                        continue;
                    }
                };
                match serde_json::from_str::<Value>(line) {
                    Ok(message) => {
                        if let Err(error) = validate_event(&message) {
                            let code = if error.starts_with("Unsupported IPC version") {
                                "IPC_VERSION_MISMATCH"
                            } else {
                                "IPC_PARSE_ERROR"
                            };
                            crate::logs::emit(&app, protocol_error(code, &error));
                            continue;
                        }
                        let is_ready = message.get("type").and_then(Value::as_str) == Some("ready");
                        if !supervisor.apply_event(&message, process_generation).await {
                            // Preserve shutdown diagnostics without reviving the stopped UI state.
                            if let Err(error) = crate::logs::append(&message) {
                                eprintln!("Unable to save stopped engine log: {error}");
                            }
                            continue;
                        }
                        crate::logs::emit(&app, message);
                        if is_ready {
                            if let Some(sender) = ready_sender.take() {
                                let _ = sender.send(Ok(()));
                            }
                        }
                    }
                    Err(error) => {
                        let event = protocol_error("IPC_PARSE_ERROR", &format!("{error}: {line}"));
                        crate::logs::emit(&app, event);
                    }
                }
            }
            if let Some(sender) = ready_sender {
                let error = supervisor.last_error().await.unwrap_or_else(|| {
                    "Automation service closed its output before it was ready".into()
                });
                let _ = sender.send(Err(error));
            }
        });
    }

    fn spawn_stderr_reader(
        &self,
        app: AppHandle,
        stderr: tokio::process::ChildStderr,
        process_generation: u64,
    ) {
        let supervisor = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                {
                    let mut state = supervisor.inner.lock().await;
                    if state.process_generation == process_generation {
                        state.last_error = Some(line.clone());
                    }
                }
                crate::logs::emit(
                    &app,
                    json!({
                        "version": 1,
                        "type": "log",
                        "timestamp": Utc::now().to_rfc3339(),
                        "level": "error",
                        "source": "engine.stderr",
                        "message": line,
                        "data": {}
                    }),
                );
            }
        });
    }

    fn spawn_exit_monitor(&self, app: AppHandle, expected_pid: u32) {
        let supervisor = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let exit = {
                    let mut state = supervisor.inner.lock().await;
                    let result = match state.process.as_mut() {
                        Some(process) if process.child.id() == Some(expected_pid) => {
                            process.child.try_wait()
                        }
                        Some(_) | None => return,
                    };
                    match result {
                        Ok(Some(status)) => {
                            state.process = None;
                            state.diagnostic = None;
                            if state.status == "stopping" {
                                state.status = "stopped".into();
                                None
                            } else {
                                state.status = "error".into();
                                if !state.pending_tasks.is_empty() {
                                    state.task_status = "paused".into();
                                }
                                let message = state.last_error.clone().unwrap_or_else(|| {
                                    format!("Automation service exited with {status}")
                                });
                                state.last_error = Some(message.clone());
                                Some(message)
                            }
                        }
                        Ok(None) => None,
                        Err(error) => Some(format!("Unable to inspect Python engine: {error}")),
                    }
                };
                if let Some(message) = exit {
                    crate::logs::emit(&app, protocol_error("ENGINE_CRASHED", &message));
                    let state = supervisor.inner.lock().await;
                    if state.task_status == "paused" {
                        crate::logs::emit(
                            &app,
                            json!({
                                "version": 1, "type": "task_paused",
                                "timestamp": Utc::now().to_rfc3339(),
                                "task_id": state.active_task_id.as_ref().or(state.pending_tasks.first()),
                                "reason": "engine_error", "message": message
                            }),
                        );
                    }
                    return;
                }
            }
        });
    }

    async fn apply_event(&self, message: &Value, process_generation: u64) -> bool {
        let mut state = self.inner.lock().await;
        if state.process_generation != process_generation {
            return false;
        }
        match message.get("type").and_then(Value::as_str) {
            Some("ready") => {
                state.status = "idle".into();
                state.last_error = None;
                if let Some(interval) = message
                    .get("heartbeat_interval_seconds")
                    .and_then(Value::as_f64)
                {
                    state.heartbeat_timeout_seconds = (interval * 3.0).max(5.0);
                }
                state.engine_version = message
                    .get("engine_version")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
            }
            Some("heartbeat") => {
                state.last_heartbeat = Some(Utc::now());
                if let Some(status) = message.get("engine_status").and_then(Value::as_str) {
                    state.status = status.to_string();
                }
                if let Some(status) = message.get("task_status").and_then(Value::as_str) {
                    state.task_status = status.to_string();
                }
            }
            Some("task_started") => {
                state.status = "running".into();
                if let Some(task_id) = message.get("task_id").and_then(Value::as_str) {
                    state.active_task_id = Some(task_id.to_string());
                    if let Some(index) = state.pending_tasks.iter().position(|item| item == task_id)
                    {
                        state.pending_tasks.drain(..index);
                    } else if state.pending_tasks.is_empty() {
                        state.pending_tasks.push(task_id.to_string());
                    }
                }
                state.task_status = "running".into();
            }
            Some("task_paused") => {
                state.status = "paused".into();
                state.task_status = "paused".into();
                if let Some(message) = message.get("message").and_then(Value::as_str) {
                    state.last_error = Some(message.to_string());
                }
            }
            Some("task_resumed") => {
                state.status = "running".into();
                state.task_status = "running".into();
                state.last_error = None;
            }
            Some("task_finished") => {
                let reason = message
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if reason == "completed" {
                    if let Some(active) = state.active_task_id.take() {
                        if state.pending_tasks.first() == Some(&active) {
                            state.pending_tasks.remove(0);
                        } else {
                            state.pending_tasks.retain(|item| item != &active);
                        }
                    }
                } else {
                    state.pending_tasks.clear();
                    state.active_task_id = None;
                }
                state.task_status = "idle".into();
            }
            Some("error") if message.get("recoverable") == Some(&Value::Bool(false)) => {
                state.status = "error".into();
            }
            _ => {}
        }
        if let Some(result) = state
            .diagnostic
            .as_ref()
            .and_then(|pending| pending.result(message))
        {
            if let Some(pending) = state.diagnostic.take() {
                let _ = pending.sender.send(result);
            }
        }
        true
    }

    pub async fn send(&self, command: Value) -> Result<(), String> {
        let adopts_engine = matches!(
            command["type"].as_str(),
            Some("start_task" | "start_tasks" | "resume_task")
        );
        let _lifecycle = if adopts_engine {
            Some(self.lifecycle.lock().await)
        } else {
            None
        };
        let pending_tasks = task_queue_from_command(&command);
        if let Some(tasks) = &pending_tasks {
            if tasks.is_empty()
                || tasks.iter().any(|task| {
                    !matches!(
                        task.as_str(),
                        "login_lobby" | "blackjack" | "hunter_match" | "hunter_rank" | "hide_seek"
                    )
                })
            {
                return Err("Unknown or empty task queue".into());
            }
        }
        let line = normalize_command(command)?;
        let mut state = self.inner.lock().await;
        if pending_tasks.is_some()
            && matches!(
                state.task_status.as_str(),
                "starting" | "running" | "paused"
            )
        {
            return Err("A task is already running or paused".into());
        }
        let process = state.process.as_mut().ok_or("Engine is not running")?;
        process
            .stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|error| error.to_string())?;
        process
            .stdin
            .flush()
            .await
            .map_err(|error| error.to_string())?;
        if adopts_engine {
            state.diagnostic_autostop_generation = None;
        }
        if let Some(tasks) = pending_tasks {
            state.pending_tasks = tasks;
            state.active_task_id = None;
            state.task_status = "starting".into();
        }
        Ok(())
    }

    pub async fn has_process(&self) -> bool {
        let mut state = self.inner.lock().await;
        state
            .process
            .as_mut()
            .is_some_and(|process| matches!(process.child.try_wait(), Ok(None)))
    }

    pub async fn resume(&self, app: AppHandle) -> Result<(), String> {
        if self.has_process().await {
            return self.send(json!({"type": "resume_task"})).await;
        }
        let pending = self.inner.lock().await.pending_tasks.clone();
        if pending.is_empty() {
            return Err("There is no paused task to resume".into());
        }
        self.start(app).await?;
        self.inner.lock().await.task_status = "idle".into();
        if pending.len() == 1 {
            self.send(json!({"type": "start_task", "task": pending[0]}))
                .await
        } else {
            self.send(json!({"type": "start_tasks", "tasks": pending}))
                .await
        }
    }

    pub async fn toggle_engine(&self, app: AppHandle) -> Result<(), String> {
        // Decide under the same lock as start/stop so rapid F3 presses toggle in order.
        let _lifecycle = self.lifecycle.lock().await;
        let stopping = self.has_process().await;
        let emit_status = |status: &str| {
            crate::logs::emit(
                &app,
                json!({
                    "version": 1, "type": "log",
                    "timestamp": Utc::now().to_rfc3339(),
                    "level": "info", "source": "engine.lifecycle",
                    "message": format!("F3: automation service {status}"),
                    "engine_status": status, "data": {"important": true}
                }),
            );
        };
        emit_status(if stopping { "stopping" } else { "starting" });
        let result = if stopping {
            self.stop_locked_and_notify(&app).await
        } else {
            match self.start_locked(app.clone()).await {
                Ok(()) => self.send(json!({"type": "enable_window_layout"})).await,
                Err(error) => Err(error),
            }
        };
        emit_status(&self.snapshot().await.status);
        result
    }

    pub async fn stop_and_notify(&self, app: &AppHandle) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        self.stop_locked_and_notify(app).await
    }

    async fn stop_locked_and_notify(&self, app: &AppHandle) -> Result<(), String> {
        let task_id = {
            let state = self.inner.lock().await;
            state
                .active_task_id
                .as_ref()
                .or(state.pending_tasks.first())
                .cloned()
        };
        self.stop_locked().await?;
        crate::logs::emit(
            app,
            json!({
                "version": 1, "type": "task_finished",
                "timestamp": Utc::now().to_rfc3339(),
                "task_id": task_id, "reason": "user_stopped", "stats": {}
            }),
        );
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        self.stop_locked().await
    }

    pub async fn prepare_update(&self) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        self.updating.store(true, Ordering::SeqCst);
        self.stop_locked().await
    }

    pub fn finish_update(&self) {
        self.updating.store(false, Ordering::SeqCst);
    }

    async fn stop_locked(&self) -> Result<(), String> {
        let running = {
            let mut state = self.inner.lock().await;
            // Discard buffered events from the process being stopped, including after restart.
            state.process_generation += 1;
            state.diagnostic = None;
            state.diagnostic_autostop_generation = None;
            let running = state.process.is_some();
            if running {
                state.status = "stopping".into();
            }
            running
        };
        if !running {
            let mut state = self.inner.lock().await;
            state.pending_tasks.clear();
            state.active_task_id = None;
            state.task_status = "idle".into();
            state.status = "stopped".into();
            state.last_heartbeat = None;
            state.last_error = None;
            return Ok(());
        }
        let _ = self.send(json!({"type": "shutdown"})).await;
        // Give Python time to cancel tasks, release held inputs and restore the window.
        // Return as soon as it exits instead of killing it after a fixed two-second sleep.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            let exited = {
                let mut state = self.inner.lock().await;
                match state.process.as_mut() {
                    Some(process) => process
                        .child
                        .try_wait()
                        .map_err(|error| error.to_string())?
                        .is_some(),
                    None => true,
                }
            };
            if exited || tokio::time::Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let mut state = self.inner.lock().await;
        if let Some(mut process) = state.process.take() {
            if process
                .child
                .try_wait()
                .map_err(|error| error.to_string())?
                .is_none()
            {
                process
                    .child
                    .kill()
                    .await
                    .map_err(|error| error.to_string())?;
                let _ = process.child.wait().await;
            }
        }
        state.status = "stopped".into();
        state.task_status = "idle".into();
        state.last_heartbeat = None;
        state.last_error = None;
        state.pending_tasks.clear();
        state.active_task_id = None;
        Ok(())
    }

    pub async fn snapshot(&self) -> EngineSnapshot {
        let state = self.inner.lock().await;
        let last_heartbeat = state.last_heartbeat;
        let heartbeat_age_seconds = last_heartbeat
            .map(|value| (Utc::now() - value).num_milliseconds().max(0) as f64 / 1000.0);
        let is_unresponsive = state.process.is_some()
            && heartbeat_age_seconds.is_some_and(|age| age > state.heartbeat_timeout_seconds);
        EngineSnapshot {
            status: if is_unresponsive {
                "error".into()
            } else {
                state.status.clone()
            },
            pid: state
                .process
                .as_ref()
                .and_then(|process| process.child.id()),
            engine_version: state.engine_version.clone(),
            task_status: state.task_status.clone(),
            last_heartbeat: last_heartbeat.map(|value| value.to_rfc3339()),
            heartbeat_age_seconds,
            last_error: if is_unresponsive {
                Some("Automation service is not responding".into())
            } else {
                state.last_error.clone()
            },
        }
    }

    async fn last_error(&self) -> Option<String> {
        self.inner.lock().await.last_error.clone()
    }

    async fn wait_until_ready(&self) -> Result<(), String> {
        let wait = async {
            loop {
                {
                    let state = self.inner.lock().await;
                    match state.status.as_str() {
                        "idle" | "running" | "paused" => return Ok(()),
                        "error" | "stopped" => {
                            return Err(state.last_error.clone().unwrap_or_else(|| {
                                "Automation service stopped before it was ready".into()
                            }));
                        }
                        _ => {}
                    }
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        };
        tokio::time::timeout(Duration::from_secs(8), wait)
            .await
            .map_err(|_| "Automation service did not become ready within 8 seconds".to_string())?
    }

    async fn fail_start(&self, message: &str) {
        let process = {
            let mut state = self.inner.lock().await;
            state.status = "error".into();
            state.last_error = Some(message.to_string());
            state.process_generation += 1;
            state.diagnostic = None;
            state.diagnostic_autostop_generation = None;
            state.process.take()
        };
        if let Some(mut process) = process {
            let _ = process.child.kill().await;
            let _ = process.child.wait().await;
        }
    }
}

fn task_queue_from_command(command: &Value) -> Option<Vec<String>> {
    match command.get("type").and_then(Value::as_str) {
        Some("start_task") => command
            .get("task")
            .and_then(Value::as_str)
            .map(|task| vec![task.to_string()])
            .or_else(|| Some(Vec::new())),
        Some("start_tasks") => Some(
            command
                .get("tasks")
                .and_then(Value::as_array)
                .filter(|tasks| tasks.iter().all(Value::is_string))
                .map(|tasks| {
                    tasks
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
        ),
        _ => None,
    }
}

fn locate_engine(resource_directory: &Path) -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("IDENTITYV_ENGINE_PATH") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "IDENTITYV_ENGINE_PATH does not exist: {}",
            path.display()
        ));
    }

    if !cfg!(dev) {
        return installed_engine_path(resource_directory);
    }
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let candidates = [
        cwd.join("engine/main.py"),
        cwd.join("../engine/main.py"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../engine/main.py"),
    ];
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| "Unable to locate engine/main.py; set IDENTITYV_ENGINE_PATH".into())
}

fn locate_python(engine_path: &Path) -> PathBuf {
    if let Ok(path) = std::env::var("IDENTITYV_PYTHON") {
        return PathBuf::from(path);
    }
    if let Some(engine_dir) = engine_path.parent() {
        let virtual_environment = engine_dir.join(".venv/Scripts/python.exe");
        if virtual_environment.is_file() {
            return virtual_environment;
        }
    }
    PathBuf::from("python")
}

fn installed_engine_path(resource_directory: &Path) -> Result<PathBuf, String> {
    let path = resource_directory
        .join("engine")
        .join("identityv-engine.exe");
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!(
            "Installed engine is missing: {}. Please reinstall the application.",
            path.display()
        ))
    }
}

fn engine_command(engine_path: &Path) -> Command {
    let mut command = if engine_path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
    {
        Command::new(engine_path)
    } else {
        let mut python = Command::new(locate_python(engine_path));
        python.arg(engine_path);
        python
    };
    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW; keep piped NDJSON handles.
    command
}

fn protocol_error(code: &str, message: &str) -> Value {
    json!({
        "version": 1,
        "type": "error",
        "timestamp": Utc::now().to_rfc3339(),
        "code": code,
        "message": message,
        "recoverable": false,
        "details": {}
    })
}

#[cfg(test)]
mod tests {
    use super::{
        engine_command, installed_engine_path, task_queue_from_command, EngineProcess,
        EngineSupervisor, PendingDiagnostic,
    };
    use serde_json::json;
    use std::process::Stdio;
    use tokio::{process::Command, sync::oneshot};

    #[test]
    fn installed_engine_uses_installation_path_and_launches_directly() {
        let root = tempfile::tempdir().unwrap();
        assert!(installed_engine_path(root.path()).is_err());
        let folder = root.path().join("engine");
        std::fs::create_dir(&folder).unwrap();
        let executable = folder.join("identityv-engine.exe");
        std::fs::write(&executable, b"fixture").unwrap();
        let found = installed_engine_path(root.path()).unwrap();
        let command = engine_command(&found);
        assert_eq!(command.as_std().get_program(), executable.as_os_str());
        assert_eq!(command.as_std().get_args().count(), 0);
    }

    #[test]
    fn development_engine_still_passes_script_to_python() {
        let root = tempfile::tempdir().unwrap();
        let script = root.path().join("main.py");
        let command = engine_command(&script);
        assert_eq!(
            command.as_std().get_args().collect::<Vec<_>>(),
            [script.as_os_str()]
        );
    }

    #[test]
    fn extracts_single_task_for_crash_recovery() {
        assert_eq!(
            task_queue_from_command(&json!({"type": "start_task", "task": "hide_seek"})),
            Some(vec!["hide_seek".to_string()])
        );
    }

    #[test]
    fn preserves_ordered_task_queue_for_crash_recovery() {
        assert_eq!(
            task_queue_from_command(&json!({
                "type": "start_tasks",
                "tasks": ["login_lobby", "hide_seek"]
            })),
            Some(vec!["login_lobby".to_string(), "hide_seek".to_string()])
        );
    }

    #[test]
    fn ignores_non_start_commands() {
        assert_eq!(
            task_queue_from_command(&json!({"type": "pause_task"})),
            None
        );
    }

    #[test]
    fn invalid_task_array_is_not_partially_queued() {
        assert_eq!(
            task_queue_from_command(&json!({"type": "start_tasks", "tasks": ["login_lobby", 1]})),
            Some(Vec::new())
        );
    }

    #[test]
    fn paused_task_keeps_order_and_stopping_clears_crash_queue() {
        tauri::async_runtime::block_on(async {
            let supervisor = EngineSupervisor::default();
            supervisor.inner.lock().await.pending_tasks =
                vec!["blackjack".into(), "hide_seek".into()];
            supervisor
                .apply_event(&json!({"type": "task_started", "task_id": "blackjack"}), 0)
                .await;
            supervisor
                .apply_event(&json!({"type": "task_paused", "message": "window_lost"}), 0)
                .await;
            assert_eq!(supervisor.snapshot().await.task_status, "paused");
            assert_eq!(
                supervisor.inner.lock().await.pending_tasks,
                ["blackjack", "hide_seek"]
            );
            supervisor
                .apply_event(&json!({"type": "task_resumed"}), 0)
                .await;
            assert_eq!(supervisor.snapshot().await.task_status, "running");
            supervisor
                .apply_event(&json!({"type": "task_finished", "reason": "completed"}), 0)
                .await;
            assert_eq!(supervisor.inner.lock().await.pending_tasks, ["hide_seek"]);
            supervisor.stop().await.unwrap();
            assert!(supervisor.inner.lock().await.pending_tasks.is_empty());
            assert_eq!(supervisor.snapshot().await.task_status, "idle");
        });
    }

    #[test]
    fn stopped_process_events_cannot_restart_the_displayed_state() {
        tauri::async_runtime::block_on(async {
            let supervisor = EngineSupervisor::default();
            supervisor.stop().await.unwrap();
            assert!(!supervisor.apply_event(&json!({"type": "ready"}), 0).await);
            assert!(
                !supervisor
                    .apply_event(&json!({"type": "heartbeat", "engine_status": "running"}), 0)
                    .await
            );
            assert_eq!(supervisor.snapshot().await.status, "stopped");

            // A new process has its own generation; the old reader cannot overwrite it.
            supervisor.inner.lock().await.process_generation = 2;
            assert!(
                supervisor
                    .apply_event(&json!({"type": "ready", "engine_version": "new"}), 2)
                    .await
            );
            assert!(
                !supervisor
                    .apply_event(&json!({"type": "task_paused"}), 0)
                    .await
            );
            assert_eq!(supervisor.snapshot().await.status, "idle");
        });
    }

    #[test]
    fn stop_waits_for_in_progress_lifecycle_operation() {
        tauri::async_runtime::block_on(async {
            let supervisor = EngineSupervisor::default();
            let lifecycle = supervisor.lifecycle.lock().await;
            supervisor.inner.lock().await.status = "starting".into();
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(20), supervisor.stop())
                    .await
                    .is_err()
            );
            assert_eq!(supervisor.snapshot().await.status, "starting");
            drop(lifecycle);
            supervisor.stop().await.unwrap();
            assert_eq!(supervisor.snapshot().await.status, "stopped");
        });
    }

    #[test]
    fn diagnostic_waits_for_its_final_result_and_stop_cancels_the_wait() {
        tauri::async_runtime::block_on(async {
            let supervisor = EngineSupervisor::default();
            let (sender, mut receiver) = oneshot::channel();
            supervisor.inner.lock().await.diagnostic = Some(PendingDiagnostic {
                request_id: "capture".into(),
                result_event: "debug_frame",
                sender,
            });
            for event in [
                json!({"type": "window_diagnostics", "request_id": "capture", "found": true}),
                json!({"type": "debug_frame", "request_id": "other"}),
                json!({"type": "error", "request_id": "other", "message": "unrelated"}),
            ] {
                supervisor.apply_event(&event, 0).await;
                assert!(matches!(
                    receiver.try_recv(),
                    Err(oneshot::error::TryRecvError::Empty)
                ));
            }
            supervisor
                .apply_event(&json!({"type": "debug_frame", "request_id": "capture"}), 1)
                .await;
            assert!(matches!(
                receiver.try_recv(),
                Err(oneshot::error::TryRecvError::Empty)
            ));
            supervisor
                .apply_event(&json!({"type": "debug_frame", "request_id": "capture"}), 0)
                .await;
            assert_eq!(receiver.await.unwrap(), Ok(()));

            let (sender, mut receiver) = oneshot::channel();
            supervisor.inner.lock().await.diagnostic = Some(PendingDiagnostic {
                request_id: "detect".into(),
                result_event: "window_diagnostics",
                sender,
            });
            supervisor
                .apply_event(
                    &json!({"type": "window_diagnostics", "request_id": "detect", "found": false}),
                    0,
                )
                .await;
            assert!(matches!(
                receiver.try_recv(),
                Err(oneshot::error::TryRecvError::Empty)
            ));
            supervisor
                .apply_event(
                    &json!({"type": "error", "request_id": "detect", "message": "No window"}),
                    0,
                )
                .await;
            assert_eq!(receiver.await.unwrap(), Err("No window".into()));

            let (sender, receiver) = oneshot::channel();
            supervisor.inner.lock().await.diagnostic = Some(PendingDiagnostic {
                request_id: "cancel".into(),
                result_event: "debug_frame",
                sender,
            });
            supervisor.stop().await.unwrap();
            assert!(receiver.await.is_err());
        });
    }

    #[test]
    #[ignore = "subprocess fixture for diagnostic lifecycle"]
    fn diagnostic_engine_fixture() {
        use std::io::BufRead;
        for line in std::io::stdin().lock().lines() {
            if line.unwrap().contains("shutdown") {
                return;
            }
        }
    }

    #[test]
    fn diagnostics_stop_only_the_engine_they_started() {
        tauri::async_runtime::block_on(async {
            for case in [
                "temporary",
                "already_running",
                "task_started",
                "restarted",
                "no_result",
            ] {
                let mut child = Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--ignored",
                        "--exact",
                        "engine::process_manager::tests::diagnostic_engine_fixture",
                    ])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .kill_on_drop(true)
                    .spawn()
                    .unwrap();
                let stdin = child.stdin.take().unwrap();
                let supervisor = EngineSupervisor::default();
                {
                    let mut state = supervisor.inner.lock().await;
                    state.process = Some(EngineProcess { child, stdin });
                    state.status = "idle".into();
                    if case != "already_running" {
                        state.diagnostic_autostop_generation = Some(0);
                    }
                    if case == "restarted" {
                        state.process_generation = 1;
                    }
                    if case == "no_result" {
                        let (sender, _receiver) = oneshot::channel();
                        state.diagnostic = Some(PendingDiagnostic {
                            request_id: "timeout".into(),
                            result_event: "debug_frame",
                            sender,
                        });
                    }
                }
                if case == "task_started" {
                    supervisor
                        .send(json!({"type": "start_task", "task": "blackjack"}))
                        .await
                        .unwrap();
                }
                supervisor.finish_diagnostic(0).await.unwrap();
                let should_keep_running =
                    matches!(case, "already_running" | "task_started" | "restarted");
                assert_eq!(
                    supervisor.has_process().await,
                    should_keep_running,
                    "{case}"
                );
                if !should_keep_running {
                    assert_eq!(supervisor.snapshot().await.status, "stopped");
                }
                supervisor.stop().await.unwrap();
            }
        });
    }

    #[test]
    #[ignore = "subprocess fixture for graceful shutdown"]
    fn delayed_shutdown_fixture() {
        let marker = std::env::var_os("IDENTITYV_SHUTDOWN_TEST_MARKER")
            .expect("only run as the graceful shutdown test subprocess");
        let mut command = String::new();
        std::io::stdin().read_line(&mut command).unwrap();
        assert!(command.contains("shutdown"));
        std::thread::sleep(std::time::Duration::from_millis(2300));
        std::fs::write(marker, "restored").unwrap();
    }

    #[test]
    fn stop_allows_cleanup_longer_than_two_seconds() {
        tauri::async_runtime::block_on(async {
            let directory = tempfile::tempdir().unwrap();
            let marker = directory.path().join("restored.txt");
            let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "engine::process_manager::tests::delayed_shutdown_fixture",
                ])
                .env("IDENTITYV_SHUTDOWN_TEST_MARKER", &marker)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .unwrap();
            let stdin = child.stdin.take().unwrap();
            let supervisor = EngineSupervisor::default();
            {
                let mut state = supervisor.inner.lock().await;
                state.process = Some(EngineProcess { child, stdin });
                state.status = "running".into();
            }
            supervisor.stop().await.unwrap();
            assert_eq!(std::fs::read_to_string(marker).unwrap(), "restored");
            assert_eq!(supervisor.snapshot().await.status, "stopped");
        });
    }
}
