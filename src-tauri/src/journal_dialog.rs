//! Modern Windows common item dialogs; no helper process or custom file listing.

#[tauri::command]
pub async fn choose_game_executable(
    window: tauri::WebviewWindow,
) -> Result<Option<String>, String> {
    choose_executable(window, false).await
}

#[tauri::command]
pub async fn choose_login_component(
    window: tauri::WebviewWindow,
) -> Result<Option<String>, String> {
    choose_executable(window, true).await
}

async fn choose_executable(
    window: tauri::WebviewWindow,
    login: bool,
) -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        let owner = window.hwnd().map_err(|e| e.to_string())?.0 as usize;
        let (send, receive) = tokio::sync::oneshot::channel();
        std::thread::Builder::new()
            .name("game-executable-dialog".into())
            .spawn(move || {
                let _ = send.send(native::choose_executable(owner, login));
            })
            .map_err(|e| e.to_string())?;
        receive.await.map_err(|e| e.to_string())?
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        Err("游戏文件选择器仅支持 Windows".into())
    }
}

#[tauri::command]
pub async fn journal_choose_pack_source(
    window: tauri::WebviewWindow,
) -> Result<Option<Vec<String>>, String> {
    #[cfg(windows)]
    {
        let owner = window.hwnd().map_err(|error| error.to_string())?.0 as usize;
        let (send, receive) = tokio::sync::oneshot::channel();
        // A dedicated thread guarantees STA initialization, unlike a reused I/O pool.
        std::thread::Builder::new()
            .name("journal-file-dialog".into())
            .spawn(move || {
                let _ = send.send(native::choose(owner));
            })
            .map_err(|error| error.to_string())?;
        receive.await.map_err(|error| error.to_string())?
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        Err("仅 Windows 支持原生地图包选择器".into())
    }
}

#[cfg(windows)]
mod native {
    use windows::{
        core::{w, HRESULT},
        Win32::{
            Foundation::{ERROR_CANCELLED, HWND},
            System::Com::{
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize,
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
            },
            UI::{
                HiDpi::{
                    SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT,
                    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
                },
                Shell::{
                    Common::COMDLG_FILTERSPEC, FileOpenDialog, IFileOpenDialog, IShellItemArray,
                    FOS_ALLOWMULTISELECT, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_NOCHANGEDIR,
                    FOS_PATHMUSTEXIST, FOS_STRICTFILETYPES, SIGDN_FILESYSPATH,
                },
            },
        },
    };

    fn paths(items: &IShellItemArray) -> windows::core::Result<Vec<String>> {
        unsafe {
            (0..items.GetCount()?)
                .map(|index| {
                    let raw = items.GetItemAt(index)?.GetDisplayName(SIGDN_FILESYSPATH)?;
                    let path = raw.to_string();
                    CoTaskMemFree(Some(raw.0.cast()));
                    path.map_err(|error| {
                        windows::core::Error::new(HRESULT(0x80070057u32 as i32), error.to_string())
                    })
                })
                .collect()
        }
    }

    struct ComApartment;
    impl Drop for ComApartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    struct DpiContext(DPI_AWARENESS_CONTEXT);
    impl Drop for DpiContext {
        fn drop(&mut self) {
            unsafe { SetThreadDpiAwarenessContext(self.0) };
        }
    }

    pub fn choose(owner: usize) -> Result<Option<Vec<String>>, String> {
        choose_kind(owner, None)
    }

    pub fn choose_executable(owner: usize, login: bool) -> Result<Option<String>, String> {
        choose_kind(owner, Some(login))
            .map(|paths| paths.and_then(|paths| paths.into_iter().next()))
    }

    fn choose_kind(owner: usize, executable: Option<bool>) -> Result<Option<Vec<String>>, String> {
        // COM objects and returned strings stay on this STA thread and are released
        // before its apartment is uninitialized. Show owns the modal message loop.
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)
                .ok()
                .map_err(|error| format!("初始化文件选择器失败：{error}"))?;
            let _apartment = ComApartment;
            let previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            if previous.0.is_null() {
                return Err("无法设置文件选择器的 DPI 缩放".into());
            }
            let _dpi = DpiContext(previous);
            let dialog: IFileOpenDialog =
                CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
                    .map_err(|error| error.to_string())?;
            if let Some(login) = executable {
                use windows::Win32::UI::Shell::FOS_FILEMUSTEXIST;
                dialog
                    .SetOptions(
                        FOS_FORCEFILESYSTEM
                            | FOS_PATHMUSTEXIST
                            | FOS_FILEMUSTEXIST
                            | FOS_NOCHANGEDIR,
                    )
                    .map_err(|e| e.to_string())?;
                dialog
                    .SetTitle(if login {
                        w!("选择 idv-login\\python-embed\\python.exe")
                    } else {
                        w!("选择第五人格游戏程序")
                    })
                    .map_err(|e| e.to_string())?;
                dialog
                    .SetFileTypes(&[COMDLG_FILTERSPEC {
                        pszName: w!("程序 (*.exe)"),
                        pszSpec: w!("*.exe"),
                    }])
                    .map_err(|e| e.to_string())?;
                if let Err(error) = dialog.Show(Some(HWND(owner as *mut _))) {
                    if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                        return Ok(None);
                    }
                    return Err(error.to_string());
                }
                return paths(&dialog.GetResults().map_err(|e| e.to_string())?)
                    .map(Some)
                    .map_err(|e| e.to_string());
            }
            let options = dialog.GetOptions().map_err(|error| error.to_string())?
                | FOS_FORCEFILESYSTEM
                | FOS_PATHMUSTEXIST
                | FOS_NOCHANGEDIR
                | FOS_ALLOWMULTISELECT
                | FOS_FILEMUSTEXIST
                | FOS_STRICTFILETYPES;
            dialog
                .SetTitle(w!("选择 ZIP 地图包 — Ctrl / Shift 可多选"))
                .map_err(|error| error.to_string())?;
            dialog
                .SetFileTypes(&[COMDLG_FILTERSPEC {
                    pszName: w!("ZIP 地图包 (*.zip)"),
                    pszSpec: w!("*.zip"),
                }])
                .map_err(|error| error.to_string())?;
            dialog
                .SetOptions(options)
                .map_err(|error| error.to_string())?;
            // Keep the shell's remembered folder, grouping, sorting and view mode.
            let shown = dialog.Show(Some(HWND(owner as *mut _)));
            if let Err(error) = shown {
                if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                    return Ok(None);
                }
                return Err(format!("打开文件选择器失败：{error}"));
            }
            paths(&dialog.GetResults().map_err(|error| error.to_string())?)
                .map(Some)
                .map_err(|error| error.to_string())
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    #[ignore = "Opens the native ZIP picker; close it with Cancel to complete"]
    fn native_zip_picker_cancel() {
        assert_eq!(super::native::choose(0).unwrap(), None);
    }
}
