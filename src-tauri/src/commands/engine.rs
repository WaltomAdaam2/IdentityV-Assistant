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
pub async fn stop_engine(
    app: AppHandle,
    supervisor: State<'_, EngineSupervisor>,
) -> Result<(), String> {
    supervisor.stop_and_notify(&app).await
}

#[tauri::command]
pub async fn restart_engine(
    app: AppHandle,
    supervisor: State<'_, EngineSupervisor>,
) -> Result<(), String> {
    supervisor.stop_and_notify(&app).await?;
    supervisor.start(app).await
}

#[tauri::command]
pub async fn send_engine_command(
    app: AppHandle,
    command: Value,
    supervisor: State<'_, EngineSupervisor>,
) -> Result<(), String> {
    let details = serde_json::json!({"command":command["type"],"task_id":command["task_id"],"task_ids":command["task_ids"]});
    crate::logs::record(
        &app,
        "info",
        "engine.command",
        "提交任务命令 / Submitting task command",
        details.clone(),
    );
    let outcome = async {
        match command.get("type").and_then(Value::as_str) {
            Some("start_task" | "start_tasks") => supervisor.start(app.clone()).await?,
            Some("resume_task") if !supervisor.has_process().await => {
                return supervisor.resume(app.clone()).await;
            }
            Some("stop_task") if !supervisor.has_process().await => {
                return supervisor.stop_and_notify(&app).await;
            }
            _ => {}
        }
        supervisor.send(command).await
    }
    .await;
    crate::logs::record(
        &app,
        if outcome.is_ok() { "debug" } else { "error" },
        "engine.command",
        "任务命令处理结果 / Task command result",
        serde_json::json!({"request":details,"accepted":outcome.is_ok(),"error":outcome.as_ref().err()}),
    );
    outcome
}

#[tauri::command]
pub async fn get_engine_snapshot(
    supervisor: State<'_, EngineSupervisor>,
) -> Result<EngineSnapshot, String> {
    Ok(supervisor.snapshot().await)
}
