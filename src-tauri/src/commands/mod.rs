pub(crate) mod diagnostics;
pub(crate) mod engine;
pub(crate) mod settings;

#[tauri::command]
pub fn open_project_repository() -> Result<(), String> {
    open_url("https://github.com/WaltomAdaam2/IdentityV-Assistant/releases")
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
