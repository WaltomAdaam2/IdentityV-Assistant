pub(crate) mod diagnostics;
pub(crate) mod engine;
pub(crate) mod settings;

#[tauri::command]
pub fn open_project_repository() -> Result<(), String> {
    open_url("https://github.com/WaltomAdaam2/IdentityV-Assistant/releases")
}

fn release_link(url: &str) -> Result<reqwest::Url, String> {
    let parsed = reqwest::Url::parse(url).map_err(|_| "Invalid release link")?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err("Release links must use HTTP or HTTPS".into());
    }
    Ok(parsed)
}

#[tauri::command]
pub fn open_release_link(url: String) -> Result<(), String> {
    open_url(release_link(&url)?.as_str())
}

#[cfg(test)]
mod tests {
    use super::release_link;

    #[test]
    fn release_links_only_open_web_urls() {
        for url in [
            "https://github.com/WaltomAdaam2/IdentityV-Assistant/releases",
            "http://example.com/notes",
        ] {
            assert!(release_link(url).is_ok());
        }
        for url in [
            "file:///C:/Windows/system32/cmd.exe",
            "javascript:alert(1)",
            "data:text/html,test",
            "ms-settings:",
            "//example.com",
            "invalid",
        ] {
            assert!(release_link(url).is_err());
        }
    }
}

pub(crate) fn open_url(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::core::{w, PCWSTR};
        use windows::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
        let url: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
        let result = unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                PCWSTR(url.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        if result.0 as isize <= 32 {
            return Err(format!(
                "Windows could not open the link (code {})",
                result.0 as isize
            ));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    Err("Opening the repository is supported on Windows only".into())
}
