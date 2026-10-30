use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

#[derive(Default)]
pub struct DesktopSettings {
    hotkey: Mutex<Option<String>>,
    pub tray: AtomicBool,
    pub taskbar_minimize: AtomicBool,
    quitting: AtomicBool,
}

#[tauri::command]
pub fn minimize_to_taskbar(window: tauri::WebviewWindow) -> Result<(), String> {
    let settings = window.state::<DesktopSettings>();
    settings.taskbar_minimize.store(true, Ordering::Relaxed);
    if let Err(error) = window.minimize() {
        settings.taskbar_minimize.store(false, Ordering::Relaxed);
        return Err(error.to_string());
    }
    Ok(())
}

fn preferences(settings: &Value) -> Result<(String, u64, bool, bool), String> {
    let hotkey = settings["app"]["engine_hotkey"]
        .as_str()
        .unwrap_or("F3")
        .trim()
        .to_string();
    hotkey
        .parse::<Shortcut>()
        .map_err(|_| "快捷键格式无效，例如 F3 或 Ctrl+Shift+F3")?;
    let scale = settings["app"]["ui_scale_percent"].as_u64().unwrap_or(100);
    if !(75..=150).contains(&scale) {
        return Err("界面缩放必须在 75% 至 150% 之间".into());
    }
    Ok((
        hotkey,
        scale,
        settings["app"]["launch_on_login"]
            .as_bool()
            .unwrap_or(false),
        settings["app"]["minimize_to_tray"]
            .as_bool()
            .unwrap_or(false),
    ))
}

pub fn initialize(app: &AppHandle, settings: &Value) -> Result<(), String> {
    let (hotkey, scale, startup, tray) = preferences(settings)?;
    let mut errors = Vec::new();
    match app.global_shortcut().register(hotkey.as_str()) {
        Ok(()) => *app.state::<DesktopSettings>().hotkey.lock().unwrap() = Some(hotkey),
        Err(error) => errors.push(format!("无法注册快捷键 {hotkey}：{error}")),
    }
    app.state::<DesktopSettings>()
        .tray
        .store(tray, Ordering::Relaxed);
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.set_zoom(scale as f64 / 100.0) {
            errors.push(error.to_string());
        }
    }
    // Refresh a previously enabled task after an install-directory change.
    if startup {
        if let Err(error) = set_startup(true) {
            errors.push(error);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("；"))
    }
}

pub fn save(app: &AppHandle, settings: &Value) -> Result<(), String> {
    let (hotkey, scale, startup, tray) = preferences(settings)?;
    let manager = app.state::<DesktopSettings>();
    let mut registered = manager.hotkey.lock().unwrap();
    let old = crate::config::load()?;
    let (old_hotkey, old_scale, old_startup, _) = preferences(&old)?;
    // A failed startup registration must not block unrelated setting saves.
    // Retry registration when the user actually chooses another shortcut.
    let changed = registered.as_deref().unwrap_or(&old_hotkey) != hotkey;
    // Reserve the replacement first: an occupied key must leave the old key usable.
    if changed {
        app.global_shortcut()
            .register(hotkey.as_str())
            .map_err(|e| format!("快捷键已被占用或无法注册：{e}"))?;
    }
    let mut startup_changed = false;
    let result = (|| {
        if startup != old_startup {
            set_startup(startup)?;
            startup_changed = true;
        }
        if let Some(window) = app.get_webview_window("main") {
            window
                .set_zoom(scale as f64 / 100.0)
                .map_err(|e| e.to_string())?;
        }
        crate::config::save(settings)
    })();
    if let Err(mut error) = result {
        if startup_changed {
            if let Err(rollback) = set_startup(old_startup) {
                error.push_str(&format!("；开机启动回滚失败：{rollback}"));
            }
        }
        if let Some(window) = app.get_webview_window("main") {
            if let Err(rollback) = window.set_zoom(old_scale as f64 / 100.0) {
                error.push_str(&format!("；缩放回滚失败：{rollback}"));
            }
        }
        if changed {
            if let Err(rollback) = app.global_shortcut().unregister(hotkey.as_str()) {
                error.push_str(&format!("；快捷键回滚失败：{rollback}"));
            }
        }
        return Err(error);
    }
    if changed {
        if let Some(previous) = registered.as_deref() {
            if let Err(error) = app.global_shortcut().unregister(previous) {
                // Never report a successful save while both shortcuts remain registered.
                let _ = app.global_shortcut().unregister(hotkey.as_str());
                let mut failures = format!("无法解除旧快捷键：{error}");
                if let Err(e) = crate::config::save(&old) {
                    failures.push_str(&format!("；配置回滚失败：{e}"));
                }
                if startup_changed {
                    if let Err(e) = set_startup(old_startup) {
                        failures.push_str(&format!("；启动项回滚失败：{e}"));
                    }
                }
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_zoom(old_scale as f64 / 100.0);
                }
                return Err(failures);
            }
        }
        *registered = Some(hotkey);
    }
    manager.tray.store(tray, Ordering::Relaxed);
    Ok(())
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn startup_xml(sid: &str, executable: &str) -> String {
    let sid = xml_escape(sid);
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?><Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task"><Triggers><LogonTrigger><Enabled>true</Enabled><UserId>{sid}</UserId></LogonTrigger></Triggers><Principals><Principal id="Author"><UserId>{sid}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>HighestAvailable</RunLevel></Principal></Principals><Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy><DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>false</StopIfGoingOnBatteries><ExecutionTimeLimit>PT0S</ExecutionTimeLimit></Settings><Actions Context="Author"><Exec><Command>{}</Command></Exec></Actions></Task>"#,
        xml_escape(executable)
    )
}

#[cfg(windows)]
fn hidden_command(program: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    let mut command = std::process::Command::new(program);
    command.creation_flags(0x08000000);
    command
}

#[cfg(windows)]
fn set_startup(enabled: bool) -> Result<(), String> {
    let identity = hidden_command("whoami.exe")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .map_err(|e| e.to_string())?;
    let output = String::from_utf8_lossy(&identity.stdout);
    let sid = output
        .trim()
        .rsplit(',')
        .next()
        .unwrap_or("")
        .trim_matches('"');
    if !identity.status.success()
        || !sid.starts_with("S-1-")
        || !sid
            .bytes()
            .all(|c| c.is_ascii_digit() || c == b'S' || c == b'-')
    {
        return Err("无法获取当前 Windows 用户标识".into());
    }
    let task = format!("IdentityV-Assistant-{sid}");
    let result = if enabled {
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let folder = crate::config::user_path()?.parent().unwrap().to_path_buf();
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let file = folder.join("startup-task.xml");
        let xml = startup_xml(sid, &executable.to_string_lossy());
        let bytes: Vec<u8> = std::iter::once(0xfeffu16)
            .chain(xml.encode_utf16())
            .flat_map(u16::to_le_bytes)
            .collect();
        std::fs::write(&file, bytes).map_err(|e| e.to_string())?;
        hidden_command("schtasks.exe")
            .args(["/Create", "/TN", &task, "/XML"])
            .arg(&file)
            .arg("/F")
            .output()
    } else {
        // Missing means disabled; enumeration failures must remain visible.
        let script = format!("$ErrorActionPreference='Stop'; $task=Get-ScheduledTask -TaskPath '\\' | Where-Object {{ $_.TaskName -eq '{task}' }}; if ($task) {{ $task | Unregister-ScheduledTask -Confirm:$false }}");
        hidden_command("powershell.exe").args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", &script]).output()
    }
    .map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err(format!(
            "更新开机启动失败：{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn set_startup(_: bool) -> Result<(), String> {
    Err("开机启动仅支持 Windows".into())
}

pub fn quit(app: &AppHandle) {
    if app
        .state::<DesktopSettings>()
        .quitting
        .swap(true, Ordering::AcqRel)
    {
        return;
    }
    crate::window_size::flush(app);
    let app = app.clone();
    let supervisor = app
        .state::<crate::engine::EngineSupervisor>()
        .inner()
        .clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = app.state::<crate::launcher::Launcher>().shutdown().await {
            crate::logs::emit(
                &app,
                serde_json::json!({"version":1,"type":"error","timestamp":chrono::Utc::now().to_rfc3339(),"code":"LOGIN_CLEANUP_FAILED","message":error,"recoverable":false,"details":{}}),
            );
        }
        let _ = supervisor.stop().await;
        let _ = app
            .state::<crate::power::PowerManager>()
            .apply(false, false);
        // Let WebView2 close its controller before the event loop exits.
        // close() would re-enter CloseRequested; destroy() bypasses that hook.
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.destroy();
        }
        app.exit(0);
    });
}

fn handle_tray_click(event: tauri::tray::TrayIconEvent, restore: impl FnOnce()) {
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
    if matches!(
        event,
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } | TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        }
    ) {
        restore();
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::TrayIconBuilder,
    };
    let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "exit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &exit])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Identity V Assistant")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            handle_tray_click(event, || show_main_window(tray.app_handle()));
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "exit" => quit(app),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[tauri::command]
pub fn open_logs_directory() -> Result<(), String> {
    let folder = crate::config::user_path()?.parent().unwrap().join("logs");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        hidden_command("explorer.exe")
            .arg(folder)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("打开日志目录仅支持 Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_click_and_double_click_request_restore_but_right_click_does_not() {
        use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
        let rect = tauri::Rect {
            position: tauri::PhysicalPosition::new(0, 0).into(),
            size: tauri::PhysicalSize::new(16, 16).into(),
        };
        let click = |button, button_state| TrayIconEvent::Click {
            id: "main".into(),
            position: tauri::PhysicalPosition::new(0.0, 0.0),
            rect,
            button,
            button_state,
        };
        let double_click = |button| TrayIconEvent::DoubleClick {
            id: "main".into(),
            position: tauri::PhysicalPosition::new(0.0, 0.0),
            rect,
            button,
        };
        for (event, expected) in [
            (click(MouseButton::Left, MouseButtonState::Up), true),
            (double_click(MouseButton::Left), true),
            (click(MouseButton::Left, MouseButtonState::Down), false),
            (click(MouseButton::Right, MouseButtonState::Up), false),
            (double_click(MouseButton::Right), false),
        ] {
            let mut restored = false;
            handle_tray_click(event, || restored = true);
            assert_eq!(restored, expected);
        }
    }
    use serde_json::json;
    #[test]
    fn validates_preferences_and_task_user_scope() {
        assert_eq!(
            preferences(&json!({})).unwrap(),
            ("F3".into(), 100, false, false)
        );
        assert!(preferences(&json!({"app":{"engine_hotkey":"bad key"}})).is_err());
        assert!(preferences(&json!({"app":{"ui_scale_percent":200}})).is_err());
        let xml = startup_xml("S-1-5-21", "C:\\a & b\\app.exe");
        assert!(xml.contains("<LogonTrigger>"));
        assert!(xml.contains("<RunLevel>HighestAvailable</RunLevel>"));
        assert!(xml.contains("C:\\a &amp; b\\app.exe"));
        assert_eq!(xml.matches("<UserId>S-1-5-21</UserId>").count(), 2);
    }
}
