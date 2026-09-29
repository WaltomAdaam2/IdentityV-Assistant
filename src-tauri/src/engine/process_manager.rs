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
    sync::Mutex,
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
            })),
        }
    }
}

impl EngineSupervisor {
    pub async fn start(&self, app: AppHandle) -> Result<(), String> {
        {
            let mut state = self.inner.lock().await;
            if let Some(process) = state.process.as_mut() {
                if process
                    .child
                    .try_wait()
                    .map_err(|error| error.to_string())?
                    .is_none()
                {
                    return Err("Engine is already running".into());
                }
                state.process = None;
            }
            state.status = "starting".into();
        }

        let engine_path = locate_engine()?;
        let python = locate_python(&engine_path);
        let mut child = Command::new(&python)
            .arg(&engine_path)
            .arg("--config")
            .arg(crate::config::active_path())
            .current_dir(engine_path.parent().unwrap_or_else(|| Path::new(".")))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| {
                format!(
                    "Unable to start Python engine with {}: {error}",
                    python.display()
                )
            })?;

        let stdin = child.stdin.take().ok_or("Python stdin was not available")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Python stdout was not available")?;
        let stderr = child
            .stderr
            .take()
            .ok_or("Python stderr was not available")?;

        {
            let mut state = self.inner.lock().await;
            state.status = "starting".into();
            state.process = Some(EngineProcess { child, stdin });
        }

        self.spawn_stdout_reader(app.clone(), stdout);
        self.spawn_stderr_reader(app.clone(), stderr);
        self.spawn_exit_monitor(app);
        Ok(())
    }

    fn spawn_stdout_reader(&self, app: AppHandle, stdout: tokio::process::ChildStdout) {
        let supervisor = self.clone();
        tauri::async_runtime::spawn(async move {
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
                        supervisor.apply_event(&message).await;
                        let _ = app.emit("engine-event", message);
                    }
                    Err(error) => {
                        let event = protocol_error("IPC_PARSE_ERROR", &format!("{error}: {line}"));
                        let _ = app.emit("engine-event", event);
                    }
                }
            }
        });
    }

    fn spawn_stderr_reader(&self, app: AppHandle, stderr: tokio::process::ChildStderr) {
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
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

    fn spawn_exit_monitor(&self, app: AppHandle) {
        let supervisor = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let exit = {
                    let mut state = supervisor.inner.lock().await;
                    let result = match state.process.as_mut() {
                        Some(process) => process.child.try_wait(),
                        None => return,
                    };
                    match result {
                        Ok(Some(status)) => {
                            state.process = None;
                            if state.status == "stopping" {
                                state.status = "stopped".into();
                                None
                            } else {
                                state.status = "error".into();
                                Some(format!("Python engine exited with {status}"))
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
