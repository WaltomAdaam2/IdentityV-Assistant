//! Modern Windows common item dialogs; no helper process or custom file listing.

#[tauri::command]
pub async fn support_choose_path(
    window: tauri::WebviewWindow,
    kind: String,
) -> Result<Option<String>, String> {
    if !["import-settings", "export-settings", "export-diagnostics"].contains(&kind.as_str()) {
        return Err("Unsupported file dialog".into());
    }
    #[cfg(windows)]
    {
        let owner = window.hwnd().map_err(|e| e.to_string())?.0 as usize;
        let (send, receive) = tokio::sync::oneshot::channel();
        std::thread::Builder::new()
            .name("support-file-dialog".into())
            .spawn(move || {
                let _ = send.send(native::choose_support(owner, &kind));
            })
            .map_err(|e| e.to_string())?;
        receive.await.map_err(|e| e.to_string())?
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        Err("Only Windows is supported".into())
    }
}

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

    pub fn choose_support(owner: usize, kind: &str) -> Result<Option<String>, String> {
        use windows::{
            core::Interface,
            Win32::UI::Shell::{FileSaveDialog, IFileDialog, IFileSaveDialog, FOS_OVERWRITEPROMPT},
        };
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)
                .ok()
                .map_err(|e| e.to_string())?;
            let _apartment = ComApartment;
            let previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            if previous.0.is_null() {
                return Err("Cannot set file dialog DPI awareness".into());
            }
            let _dpi = DpiContext(previous);
            let importing = kind == "import-settings";
            let zip = kind == "export-diagnostics";
            let dialog: IFileDialog = if importing {
                let open: IFileOpenDialog =
                    CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
                        .map_err(|e| e.to_string())?;
                open.cast().map_err(|e| e.to_string())?
            } else {
                let save: IFileSaveDialog =
                    CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)
                        .map_err(|e| e.to_string())?;
                save.cast().map_err(|e| e.to_string())?
            };
            dialog
                .SetOptions(
                    FOS_FORCEFILESYSTEM
                        | FOS_PATHMUSTEXIST
                        | FOS_NOCHANGEDIR
                        | if importing {
                            FOS_FILEMUSTEXIST
                        } else {
                            FOS_OVERWRITEPROMPT
                        },
                )
                .map_err(|e| e.to_string())?;
            dialog
                .SetTitle(if importing {
                    w!("导入助手设置 / Import settings")
                } else if zip {
                    w!("导出诊断包 / Export diagnostics")
                } else {
                    w!("导出助手设置 / Export settings")
                })
                .map_err(|e| e.to_string())?;
            dialog
                .SetFileTypes(&[COMDLG_FILTERSPEC {
                    pszName: if zip {
                        w!("ZIP (*.zip)")
                    } else {
                        w!("JSON (*.json)")
                    },
                    pszSpec: if zip { w!("*.zip") } else { w!("*.json") },
                }])
                .map_err(|e| e.to_string())?;
            dialog
                .SetDefaultExtension(if zip { w!("zip") } else { w!("json") })
                .map_err(|e| e.to_string())?;
            if !importing {
                dialog
                    .SetFileName(if zip {
                        w!("IdentityV-diagnostics.zip")
                    } else {
                        w!("IdentityV-settings.json")
                    })
                    .map_err(|e| e.to_string())?;
            }
            if let Err(error) = dialog.Show(Some(HWND(owner as *mut _))) {
                if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                    return Ok(None);
                }
                return Err(error.to_string());
            }
            let raw = dialog
                .GetResult()
                .and_then(|item| item.GetDisplayName(SIGDN_FILESYSPATH))
                .map_err(|e| e.to_string())?;
            let path = raw.to_string().map_err(|e| e.to_string());
            CoTaskMemFree(Some(raw.0.cast()));
            path.map(Some)
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
                use windows::Win32::UI::Shell::{FOS_FILEMUSTEXIST, FOS_PICKFOLDERS};
                dialog
                    .SetOptions(
                        FOS_FORCEFILESYSTEM
                            | FOS_PATHMUSTEXIST
                            | if login {
                                FOS_PICKFOLDERS
                            } else {
                                FOS_FILEMUSTEXIST
                            }
                            | FOS_NOCHANGEDIR,
                    )
                    .map_err(|e| e.to_string())?;
                dialog
                    .SetTitle(if login {
                        w!("选择包含“点我启动工具.bat”的 idv-login 文件夹")
                    } else {
                        w!("选择第五人格游戏程序")
                    })
                    .map_err(|e| e.to_string())?;
                if !login {
                    dialog
                        .SetFileTypes(&[COMDLG_FILTERSPEC {
                            pszName: w!("程序 (*.exe)"),
                            pszSpec: w!("*.exe"),
                        }])
                        .map_err(|e| e.to_string())?;
                }
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
