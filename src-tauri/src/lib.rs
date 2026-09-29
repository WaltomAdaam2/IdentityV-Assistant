mod commands;
mod config;
mod engine;

use engine::EngineSupervisor;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(EngineSupervisor::default())
        .invoke_handler(tauri::generate_handler![
            commands::engine::start_engine,
            commands::engine::stop_engine,
            commands::engine::restart_engine,
            commands::engine::send_engine_command,
            commands::engine::get_engine_snapshot,
            commands::settings::load_settings,
            commands::settings::save_settings,
            commands::diagnostics::detect_window,
            commands::diagnostics::capture_frame,
            commands::diagnostics::test_template,
            commands::diagnostics::input_test,
            commands::diagnostics::read_debug_frame,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
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
