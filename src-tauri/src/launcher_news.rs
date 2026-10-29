use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
pub struct Announcement {
    title: String,
    url: String,
    release_time: i64,
}

#[derive(Deserialize)]
struct Response {
    code: u16,
    data: Library,
}

#[derive(Deserialize)]
struct Library {
    activities_and_news: News,
}

#[derive(Deserialize)]
struct News {
    tops: Vec<Announcement>,
}

#[tauri::command]
pub async fn get_game_announcements() -> Result<Vec<Announcement>, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get("https://loadingbaycn.webapp.163.com/app/v1/game_library/app?force=1&app_id=73")
        .header("channel", "mkt-h55")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| e.to_string())?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if body.len() + chunk.len() > 1024 * 1024 {
            return Err("Announcement response is too large".into());
        }
        body.extend_from_slice(&chunk);
    }
    let response: Response = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
    if response.code != 200 {
        return Err("Announcement service is unavailable".into());
    }
    Ok(response
        .data
        .activities_and_news
        .tops
        .into_iter()
        .filter(|item| !item.title.trim().is_empty() && official_news_url(&item.url))
        .take(4)
        .collect())
}

fn official_news_url(url: &str) -> bool {
    url.len() <= 2048 && url.starts_with("https://id5.163.com/news/")
}

#[tauri::command]
pub fn open_game_announcement(url: String) -> Result<(), String> {
    if !official_news_url(&url) {
        return Err("Only official Identity V news links can be opened".into());
    }
    #[cfg(windows)]
    {
        use windows::core::{w, PCWSTR};
        use windows::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
        let target: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
        let result = unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                PCWSTR(target.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        if result.0 as isize <= 32 {
            return Err(format!(
                "Could not open announcement (code {})",
                result.0 as isize
            ));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    Err("Opening announcements is supported on Windows only".into())
}
