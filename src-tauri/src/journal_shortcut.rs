//! Observe one configured key without consuming it or activating the assistant.
use serde_json::Value;
use std::sync::atomic::{AtomicU32, Ordering};

static KEY: AtomicU32 = AtomicU32::new(b'P' as u32);
static THREAD: AtomicU32 = AtomicU32::new(0);

fn observe_key(held: &AtomicU32, configured: u32, key: u32, down: bool) -> bool {
    if !down {
        let _ = held.compare_exchange(key, 0, Ordering::Relaxed, Ordering::Relaxed);
        false
    } else {
        key == configured && held.swap(key, Ordering::Relaxed) != key
    }
}

pub(crate) fn validate(settings: &Value) -> Result<u32, String> {
    let key = settings["app"]["recognition_hotkey"]
        .as_str()
        .unwrap_or("P")
        .to_ascii_uppercase();
    let vk = if key.len() == 1 && key.as_bytes()[0].is_ascii_uppercase() {
        key.as_bytes()[0] as u32
    } else if let Some(number) = key
        .strip_prefix('F')
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|n| (1..=12).contains(n))
    {
        0x70 + number - 1
    } else {
        return Err("识图快捷键需为 A–Z 或 F1–F12 / Choose A–Z or F1–F12".into());
    };
    let engine = settings["app"]["engine_hotkey"].as_str().unwrap_or("F3");
    let recognition = key
        .parse::<tauri_plugin_global_shortcut::Shortcut>()
        .map_err(|e| e.to_string())?;
    if engine
        .parse::<tauri_plugin_global_shortcut::Shortcut>()
        .is_ok_and(|key| key == recognition)
    {
        return Err(
            "识图快捷键不能与引擎快捷键相同 / Recognition and engine shortcuts must differ".into(),
        );
    }
    Ok(vk)
}
pub(crate) fn configure(settings: &Value) -> Result<(), String> {
    KEY.store(validate(settings)?, Ordering::Relaxed);
    Ok(())
}

fn matches_game(settings: &Value, process: &str, title: &str, class: &str) -> bool {
    let window = &settings["game_window"];
    let names = window["process_names"].as_array();
    if let Some(names) = names.filter(|names| !names.is_empty()) {
        return names
            .iter()
            .filter_map(Value::as_str)
            .any(|name| name.eq_ignore_ascii_case(process));
    }
    window["title_contains"].as_array().is_some_and(|values| {
        values
            .iter()
            .filter_map(Value::as_str)
            .any(|part| !part.is_empty() && title.to_lowercase().contains(&part.to_lowercase()))
    }) || window["class_names"].as_array().is_some_and(|values| {
        values
            .iter()
            .filter_map(Value::as_str)
            .any(|name| name.eq_ignore_ascii_case(class))
    })
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::{
        sync::{
            mpsc::{sync_channel, SyncSender},
            OnceLock,
        },
        time::Duration,
    };
    use tauri::Emitter;
    use windows::{
        core::PWSTR,
        Win32::{
            Foundation::{CloseHandle, HWND, LPARAM, LRESULT, WPARAM},
            System::{
                LibraryLoader::GetModuleHandleW,
                Threading::{
                    GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
                    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
                },
            },
            UI::{Input::KeyboardAndMouse::GetAsyncKeyState, WindowsAndMessaging::*},
        },
    };

    static EVENTS: OnceLock<SyncSender<isize>> = OnceLock::new();
    static DOWN: AtomicU32 = AtomicU32::new(0);

    unsafe extern "system" fn keyboard(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let event = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            if event.flags.0 & LLKHF_INJECTED.0 == 0 {
                match wparam.0 as u32 {
                    WM_KEYUP | WM_SYSKEYUP => {
                        observe_key(&DOWN, KEY.load(Ordering::Relaxed), event.vkCode, false);
                    }
                    WM_KEYDOWN | WM_SYSKEYDOWN
                        if observe_key(&DOWN, KEY.load(Ordering::Relaxed), event.vkCode, true) =>
                    {
                        // Key-only shortcut; modified key combinations retain their own meaning.
                        if [0x10, 0x11, 0x12, 0x5b, 0x5c]
                            .iter()
                            .all(|key| GetAsyncKeyState(*key) >= 0)
                        {
                            if let Some(events) = EVENTS.get() {
                                let _ = events.try_send(GetForegroundWindow().0 as isize);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // Always preserve the original input, including in the game.
        CallNextHookEx(None, code, wparam, lparam)
    }

    unsafe fn game_is_foreground(handle: isize, settings: &Value) -> bool {
        let hwnd = HWND(handle as *mut _);
        if handle == 0 || GetForegroundWindow() != hwnd || !IsWindowVisible(hwnd).as_bool() {
            return false;
        }
        let mut title = [0u16; 1024];
        let mut class = [0u16; 256];
        let title_len = GetWindowTextW(hwnd, &mut title).max(0) as usize;
        let class_len = GetClassNameW(hwnd, &mut class).max(0) as usize;
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let mut name = String::new();
        if let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut path = [0u16; 32768];
            let mut size = path.len() as u32;
            if QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(path.as_mut_ptr()),
                &mut size,
            )
            .is_ok()
            {
                name = String::from_utf16_lossy(&path[..size as usize])
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or("")
                    .to_string();
            }
            let _ = CloseHandle(process);
        }
        matches_game(
            settings,
            &name,
            &String::from_utf16_lossy(&title[..title_len]),
            &String::from_utf16_lossy(&class[..class_len]),
        )
    }

    pub(super) fn start(app: tauri::AppHandle) -> Result<(), String> {
        let (events, receiver) = sync_channel(1);
        EVENTS
            .set(events)
            .map_err(|_| "Recognition shortcut already initialized")?;
        let (ready, started) = sync_channel(1);
        std::thread::spawn(move || unsafe {
            let module = GetModuleHandleW(None).map(|module| module.into());
            let hook = module.and_then(|module| {
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard), Some(module), 0)
            });
            match hook {
                Ok(hook) => {
                    // Ensure the thread message queue exists before exposing its ID.
                    let mut message = MSG::default();
                    let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
                    THREAD.store(GetCurrentThreadId(), Ordering::Release);
                    let _ = ready.send(Ok(()));
                    while GetMessageW(&mut message, None, 0, 0).0 > 0 {}
                    let _ = UnhookWindowsHookEx(hook);
                    THREAD.store(0, Ordering::Release);
                }
                Err(error) => {
                    let _ = ready.send(Err(error.to_string()));
                }
            }
        });
        started
            .recv_timeout(Duration::from_secs(3))
            .map_err(|e| e.to_string())??;
        std::thread::spawn(move || {
            while THREAD.load(Ordering::Acquire) != 0 {
                let Ok(hwnd) = receiver.recv_timeout(Duration::from_millis(250)) else {
                    continue;
                };
                if let Ok(settings) = crate::config::load() {
                    if unsafe { game_is_foreground(hwnd, &settings) } {
                        let _ = app.emit(
                            "journal-recognition-shortcut",
                            serde_json::json!({"hwnd": hwnd}),
                        );
                    }
                }
            }
        });
        Ok(())
    }
    pub(super) fn stop() {
        let thread = THREAD.load(Ordering::Acquire);
        if thread != 0 {
            unsafe {
                let _ = PostThreadMessageW(thread, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
    }
}

pub(crate) fn start(app: tauri::AppHandle, settings: &Value) -> Result<(), String> {
    configure(settings)?;
    #[cfg(windows)]
    native::start(app)?;
    Ok(())
}
pub(crate) fn stop() {
    #[cfg(windows)]
    native::stop();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn changing_shortcut_while_held_does_not_lose_the_next_press() {
        let held = AtomicU32::new(0);
        assert!(observe_key(&held, 80, 80, true));
        assert!(!observe_key(&held, 80, 80, true));
        assert!(!observe_key(&held, 117, 80, false));
        assert!(observe_key(&held, 117, 117, true));
        assert!(!observe_key(&held, 117, 117, true));
        assert!(!observe_key(&held, 117, 80, false));
        assert!(!observe_key(&held, 117, 117, true));
        assert!(!observe_key(&held, 117, 117, false));
        assert!(observe_key(&held, 117, 117, true));
    }
    #[test]
    fn configured_game_process_is_required_and_shortcuts_must_differ() {
        let settings = json!({"game_window":{"process_names":["dwrg.exe"],"title_contains":["Identity V"]},"app":{"engine_hotkey":"F3","recognition_hotkey":"P"}});
        assert_eq!(validate(&settings).unwrap(), b'P' as u32);
        assert!(matches_game(&settings, "DWRG.EXE", "", ""));
        assert!(!matches_game(&settings, "notepad.exe", "Identity V", ""));
        assert!(
            validate(&json!({"app":{"engine_hotkey":"F3","recognition_hotkey":"F3"}})).is_err()
        );
        assert!(validate(&json!({"app":{"recognition_hotkey":"Ctrl+P"}})).is_err());
    }
}
