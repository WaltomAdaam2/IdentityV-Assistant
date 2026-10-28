pub(crate) mod diagnostics;
pub(crate) mod engine;
pub(crate) mod settings;

#[tauri::command]
pub fn open_project_repository() -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::core::{w, PCWSTR};
        use windows::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
        let result = unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                w!("https://github.com/WaltomAdaam2/IdentityV-Assistant"),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        if result.0 as isize <= 32 {
            return Err(format!(
                "Windows could not open the repository (code {})",
                result.0 as isize
            ));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    Err("Opening the repository is supported on Windows only".into())
}
