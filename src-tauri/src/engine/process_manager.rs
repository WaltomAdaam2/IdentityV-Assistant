use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use chrono::Utc;
use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
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

struct EngineState {
    process: Option<EngineProcess>,
    status: String,
    engine_version: Option<String>,
    task_status: String,
    last_heartbeat: Option<chrono::DateTime<Utc>>,
    last_error: Option<String>,
}

#[derive(Clone)]
pub struct EngineSupervisor {
    inner: Arc<Mutex<EngineState>>,
}

impl Default for EngineSupervisor {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(EngineState {
                process: None,
                status: "stopped".into(),
                engine_version: None,
                task_status: "idle".into(),
                last_heartbeat: None,
                last_error: None,
            })),
        }
    }
}

impl EngineSupervisor {
    pub async fn start(&self, app: AppHandle) -> Result<(), String> {
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

        {
            let mut state = self.inner.lock().await;
            state.status = "starting".into();
            state.last_error = None;
        }

        let engine_path = match locate_engine() {
            Ok(path) => path,
            Err(error) => {
                self.fail_start(&error).await;
                return Err(error);
            }
        };
        let python = locate_python(&engine_path);
        let mut child = match Command::new(&python)
            .arg(&engine_path)
            .arg("--config")
            .arg(crate::config::active_path()?)
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
                let message = format!(
                    "Unable to start Python engine with {}: {error}",
                    python.display()
                );
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
        self.spawn_stdout_reader(app.clone(), stdout, ready_sender);
        self.spawn_stderr_reader(app.clone(), stderr);
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
    ) {
        let supervisor = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut ready_sender = Some(ready_sender);
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                match serde_json::from_str::<Value>(&line) {
                    Ok(message) => {
                        if let Err(error) = validate_event(&message) {
                            let code = if error.starts_with("Unsupported IPC version") {
                                "IPC_VERSION_MISMATCH"
                            } else {
                                "IPC_PARSE_ERROR"
                            };
                            let _ = app.emit("engine-event", protocol_error(code, &error));
                            continue;
                        }
                        let is_ready = message.get("type").and_then(Value::as_str) == Some("ready");
                        supervisor.apply_event(&message).await;
                        let _ = app.emit("engine-event", message);
                        if is_ready {
                            if let Some(sender) = ready_sender.take() {
                                let _ = sender.send(Ok(()));
                            }
                        }
                    }
                    Err(error) => {
                        let event = protocol_error("IPC_PARSE_ERROR", &format!("{error}: {line}"));
                        let _ = app.emit("engine-event", event);
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

    fn spawn_stderr_reader(&self, app: AppHandle, stderr: tokio::process::ChildStderr) {
        let supervisor = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                {
                    let mut state = supervisor.inner.lock().await;
                    state.last_error = Some(line.clone());
                }
                let _ = app.emit(
                    "engine-event",
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
                            if state.status == "stopping" {
                                state.status = "stopped".into();
                                None
                            } else {
                                state.status = "error".into();
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
                    let _ = app.emit("engine-event", protocol_error("ENGINE_CRASHED", &message));
                    return;
                }
            }
        });
    }

    async fn apply_event(&self, message: &Value) {
        let mut state = self.inner.lock().await;
        match message.get("type").and_then(Value::as_str) {
            Some("ready") => {
                state.status = "idle".into();
                state.last_error = None;
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
            Some("task_finished") => state.task_status = "idle".into(),
            Some("error") if message.get("recoverable") == Some(&Value::Bool(false)) => {
                state.status = "error".into();
            }
            _ => {}
        }
    }

    pub async fn send(&self, command: Value) -> Result<(), String> {
        let line = normalize_command(command)?;
        let mut state = self.inner.lock().await;
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
            .map_err(|error| error.to_string())
    }

    pub async fn stop(&self) -> Result<(), String> {
        let running = self.inner.lock().await.process.is_some();
        if !running {
            return Ok(());
        }
        {
            let mut state = self.inner.lock().await;
            state.status = "stopping".into();
        }
        let _ = self.send(json!({"type": "shutdown"})).await;
        tokio::time::sleep(Duration::from_secs(2)).await;

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
        Ok(())
    }

    pub async fn snapshot(&self) -> EngineSnapshot {
        let state = self.inner.lock().await;
        let last_heartbeat = state.last_heartbeat;
        EngineSnapshot {
            status: state.status.clone(),
            pid: state
                .process
                .as_ref()
                .and_then(|process| process.child.id()),
            engine_version: state.engine_version.clone(),
            task_status: state.task_status.clone(),
            last_heartbeat: last_heartbeat.map(|value| value.to_rfc3339()),
            heartbeat_age_seconds: last_heartbeat
                .map(|value| (Utc::now() - value).num_milliseconds().max(0) as f64 / 1000.0),
            last_error: state.last_error.clone(),
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
                        "idle" | "running" => return Ok(()),
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
            state.process.take()
        };
        if let Some(mut process) = process {
            let _ = process.child.kill().await;
            let _ = process.child.wait().await;
        }
    }
}

fn locate_engine() -> Result<PathBuf, String> {
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
