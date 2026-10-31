mod commands;
mod config;
mod desktop;
mod engine;
mod journal;
mod journal_dialog;
mod journal_recognition;
mod journal_shortcut;
mod launcher;
mod launcher_news;
mod logs;
mod power;
mod support;
mod support_bundle;
mod update_source;
mod updates;
mod window_size;

use engine::EngineSupervisor;
use serde_json::json;
use tauri::Manager;
use tauri_plugin_global_shortcut::ShortcutState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
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
        .manage(journal_recognition::RecognitionState::default())
        .manage(launcher::Launcher::default())
        .manage(power::PowerManager::default())
        .manage(updates::UpdateManager::default())
        .manage(desktop::DesktopSettings::default())
        .manage(window_size::WindowSizeState::default())
        .manage(support::SupportState::default())
        .manage(support_bundle::BundleState::default())
        .invoke_handler(tauri::generate_handler![
            commands::open_project_repository,
            commands::open_release_link,
            launcher::manager::launch_game,
            launcher::locate_game,
            launcher::manager::get_launcher_status,
            launcher::manager::set_login_component,
            launcher::manager::configure_login_component,
            launcher::manager::cancel_game_launch,
            launcher::manager::check_login_updates,
            launcher::manager::apply_login_hotfixes,
            launcher::manager::open_login_update,
            launcher_news::get_game_announcements,
            launcher_news::open_game_announcement,
            desktop::open_logs_directory,
            desktop::minimize_to_taskbar,
            window_size::reset_window_size,
            journal_dialog::choose_game_executable,
            journal_dialog::support_choose_path,
            journal_dialog::choose_login_component,
            updates::get_update_status,
            updates::check_update,
            updates::download_update,
            updates::cancel_update,
            updates::install_update,
            commands::engine::start_engine,
            commands::engine::stop_engine,
            commands::engine::restart_engine,
            commands::engine::send_engine_command,
            commands::engine::get_engine_snapshot,
            commands::settings::load_settings,
            commands::settings::save_settings,
            support::export_settings,
            support::preview_settings_restore,
            support::restore_settings,
            support_bundle::preview_diagnostics,
            support_bundle::export_diagnostics,
            commands::settings::append_ui_log,
            commands::settings::set_power_prevention,
            journal::journal_list_packs,
            journal::journal_import_pack,
            journal::journal_read_image,
            journal::journal_open_directory,
            journal_recognition::journal_recognize,
            journal_recognition::journal_cancel_recognition,
            journal_dialog::journal_choose_pack_source,
            commands::diagnostics::detect_window,
            commands::diagnostics::task_preflight,
            commands::diagnostics::capture_frame,
            commands::diagnostics::test_template,
            commands::diagnostics::input_test,
            commands::diagnostics::read_debug_frame,
        ])
        .setup(|app| {
            logs::prune().map_err(std::io::Error::other)?;
            let settings = config::load().map_err(std::io::Error::other)?;
            window_size::restore(app.handle()).map_err(std::io::Error::other)?;
            desktop::create_tray(app.handle())?;
            launcher::manager::start_monitor(app.handle().clone());
            if let Err(error) = desktop::initialize(app.handle(), &settings) {
                logs::emit(app.handle(), json!({"version":1,"type":"error","timestamp":chrono::Utc::now().to_rfc3339(),"code":"DESKTOP_SETTINGS_FAILED","message":error,"recoverable":true,"details":{}}));
            }
            if let Err(error) = journal_shortcut::start(app.handle().clone(), &settings) {
                logs::emit(app.handle(), json!({"version":1,"type":"error","timestamp":chrono::Utc::now().to_rfc3339(),"code":"RECOGNITION_SHORTCUT_FAILED","message":error,"recoverable":true,"details":{}}));
            }
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
            if matches!(event, tauri::WindowEvent::Resized(_)) {
                window_size::resized(window);
            }
            let minimize_to_tray = window.state::<desktop::DesktopSettings>().tray.load(std::sync::atomic::Ordering::Relaxed);
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if minimize_to_tray {
                    let _ = window.hide();
                } else {
                    desktop::quit(window.app_handle());
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Identity V Assistant");
}
