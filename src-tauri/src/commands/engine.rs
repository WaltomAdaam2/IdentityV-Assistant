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
    command: Value,
    supervisor: State<'_, EngineSupervisor>,
) -> Result<(), String> {
    supervisor.send(command).await
}

#[tauri::command]
pub async fn get_engine_snapshot(
    supervisor: State<'_, EngineSupervisor>,
) -> Result<EngineSnapshot, String> {
    Ok(supervisor.snapshot().await)
}
