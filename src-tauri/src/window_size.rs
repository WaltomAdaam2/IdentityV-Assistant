use serde::{Deserialize, Serialize};
use std::{fs, sync::Mutex, time::Duration};
use tauri::{AppHandle, LogicalSize, Manager};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Size {
    width: f64,
    height: f64,
}

const DEFAULT: Size = Size {
    width: 1000.0,
    height: 680.0,
};

#[derive(Default)]
pub struct WindowSizeState {
    size: Mutex<Option<Size>>,
    pending: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

fn path() -> Result<std::path::PathBuf, String> {
    Ok(crate::config::user_path()?.with_file_name("window-size.json"))
}

fn decode(text: &str) -> Option<Size> {
    let size: Size = serde_json::from_str(text).ok()?;
    (size.width.is_finite()
        && size.height.is_finite()
        && (DEFAULT.width..=16384.0).contains(&size.width)
        && (DEFAULT.height..=16384.0).contains(&size.height))
    .then_some(size)
}

pub fn restore(app: &AppHandle) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let mut size = fs::read_to_string(path()?)
        .ok()
        .and_then(|text| decode(&text))
        .unwrap_or(DEFAULT);
    if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
        let available = monitor
            .work_area()
            .size
            .to_logical::<f64>(monitor.scale_factor());
        size.width = size.width.min(available.width).max(DEFAULT.width);
        size.height = size.height.min(available.height).max(DEFAULT.height);
    }
    window
        .set_size(LogicalSize::new(size.width, size.height))
        .map_err(|e| e.to_string())?;
    *app.state::<WindowSizeState>().size.lock().unwrap() = Some(size);
    Ok(())
}

// Geometry is separate from settings: resizing must never trigger a saved toast.
pub fn resized(window: &tauri::Window) {
    if window.label() != "main"
        || window.is_minimized().unwrap_or(true)
        || window.is_maximized().unwrap_or(true)
    {
        return;
    }
    let (Ok(physical), Ok(scale)) = (window.inner_size(), window.scale_factor()) else {
        return;
    };
    let logical = physical.to_logical::<f64>(scale);
    let size = Size {
        width: logical.width,
        height: logical.height,
    };
    if size.width < DEFAULT.width || size.height < DEFAULT.height {
        return;
    }
    let app = window.app_handle();
    let state = app.state::<WindowSizeState>();
    let mut previous = state.size.lock().unwrap();
    if previous.is_none() || *previous == Some(size) {
        return;
    }
    *previous = Some(size);
    drop(previous);
    let mut pending = state.pending.lock().unwrap();
    if let Some(task) = pending.take() {
        task.abort();
    }
    let app = app.clone();
    *pending = Some(tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        flush(&app);
    }));
}

pub fn flush(app: &AppHandle) {
    let state = app.state::<WindowSizeState>();
    let size = state.size.lock().unwrap();
    let Some(size) = *size else { return };
    let result = (|| -> Result<(), String> {
        let path = path()?;
        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::write(path, serde_json::to_vec(&size).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    })();
    if let Err(error) = result {
        eprintln!("Unable to save assistant window size: {error}");
    }
}

#[tauri::command]
pub fn reset_window_size(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window is unavailable")?;
    window.unmaximize().map_err(|e| e.to_string())?;
    window
        .set_size(LogicalSize::new(DEFAULT.width, DEFAULT.height))
        .map_err(|e| e.to_string())?;
    *app.state::<WindowSizeState>().size.lock().unwrap() = Some(DEFAULT);
    flush(&app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_round_trip_and_invalid_geometry() {
        let size = Size {
            width: 1350.0,
            height: 820.0,
        };
        assert_eq!(decode(&serde_json::to_string(&size).unwrap()), Some(size));
        for value in [
            "{}",
            "broken",
            r#"{"width":0,"height":0}"#,
            r#"{"width":999,"height":680}"#,
            r#"{"width":20000,"height":800}"#,
        ] {
            assert_eq!(decode(value), None);
        }
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(
            config["app"]["windows"][0]["width"].as_f64(),
            Some(DEFAULT.width)
        );
        assert_eq!(
            config["app"]["windows"][0]["height"].as_f64(),
            Some(DEFAULT.height)
        );
    }
}
