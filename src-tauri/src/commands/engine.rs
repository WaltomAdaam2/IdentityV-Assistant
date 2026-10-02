use serde_json::Value;
use tauri::{AppHandle, State};

use crate::engine::{EngineSnapshot, EngineSupervisor};

#[tauri::command]
pub async fn start_engine(
    app: AppHandle,
    supervisor: State<'_, EngineSupervisor>,
) -> Result<(), String> {
    supervisor.start(app).await
}

#[tauri::command]
pub async fn stop_engine(supervisor: State<'_, EngineSupervisor>) -> Result<(), String> {
    supervisor.stop().await
}

#[tauri::command]
pub async fn restart_engine(
    app: AppHandle,
    supervisor: State<'_, EngineSupervisor>,
) -> Result<(), String> {
    supervisor.stop().await?;
    supervisor.start(app).await
}

#[tauri::command]
pub async fn send_engine_command(
    app: AppHandle,
    command: Value,
    supervisor: State<'_, EngineSupervisor>,
) -> Result<(), String> {
    match command.get("type").and_then(Value::as_str) {
        Some("start_task" | "start_tasks") => supervisor.start(app).await?,
        Some("resume_task") if !supervisor.has_process().await => {
            return supervisor.resume(app).await;
        }
        Some("stop_task") if !supervisor.has_process().await => {
            supervisor.stop().await?;
            crate::logs::emit(
                &app,
                serde_json::json!({
                    "version": 1, "type": "task_finished",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "reason": "user_stopped", "stats": {}
                }),
            );
            return Ok(());
        }
        _ => {}
    }
    supervisor.send(command).await
}

#[tauri::command]
pub async fn get_engine_snapshot(
    supervisor: State<'_, EngineSupervisor>,
) -> Result<EngineSnapshot, String> {
    Ok(supervisor.snapshot().await)
}
