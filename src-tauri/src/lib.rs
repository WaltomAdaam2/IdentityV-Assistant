mod commands;
mod config;
mod engine;
mod logs;
mod power;

use engine::EngineSupervisor;
use serde_json::json;
use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let app = app.clone();
                    let supervisor = app.state::<EngineSupervisor>().inner().clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = supervisor.toggle_engine(app.clone()).await {
                            logs::emit(
                                &app,
                                json!({
                                    "version": 1,
                                    "type": "error",
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                    "code": "HOTKEY_FAILED",
                                    "message": error,
                                    "recoverable": true,
                                    "details": {}
                                }),
                            );
                        }
                    });
                })
                .build(),
        )
        .manage(EngineSupervisor::default())
        .manage(power::PowerManager::default())
        .invoke_handler(tauri::generate_handler![
            commands::engine::start_engine,
            commands::engine::stop_engine,
            commands::engine::restart_engine,
            commands::engine::send_engine_command,
            commands::engine::get_engine_snapshot,
            commands::settings::load_settings,
            commands::settings::save_settings,
            commands::settings::append_ui_log,
            commands::settings::set_power_prevention,
            commands::diagnostics::detect_window,
            commands::diagnostics::capture_frame,
            commands::diagnostics::test_template,
            commands::diagnostics::input_test,
            commands::diagnostics::read_debug_frame,
        ])
        .setup(|app| {
            if let Err(error) = app.global_shortcut().register("F3") {
                logs::emit(
                    app.handle(),
                    json!({
                        "version": 1,
                        "type": "error",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "code": "HOTKEY_UNAVAILABLE",
                        "message": format!("Could not register F3: {error}"),
                        "recoverable": true,
                        "details": {}
                    }),
                );
            }
            logs::prune().map_err(std::io::Error::other)?;
            let settings = config::load().map_err(std::io::Error::other)?;
            let system = settings["app"]["prevent_system_sleep"]
                .as_bool()
                .unwrap_or(false);
            let display = settings["app"]["prevent_display_sleep"]
                .as_bool()
                .unwrap_or(false);
            app.state::<power::PowerManager>()
                .apply(system, display)
                .map_err(std::io::Error::other)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.state::<power::PowerManager>().apply(false, false);
                let supervisor = window.state::<EngineSupervisor>().inner().clone();
                let window = window.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = supervisor.stop().await;
                    let _ = window.destroy();
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Identity V Assistant");
}
