//! Modern Windows common item dialogs; no helper process or custom file listing.

#[tauri::command]
pub async fn journal_choose_pack_source(
    window: tauri::WebviewWindow,
    kind: String,
) -> Result<Option<String>, String> {
    if !matches!(kind.as_str(), "folder" | "zip") {
        return Err("请选择文件夹或 ZIP 文件".into());
    }
    #[cfg(windows)]
    {
        let owner = window.hwnd().map_err(|error| error.to_string())?.0 as usize;
        let (send, receive) = tokio::sync::oneshot::channel();
        // A dedicated thread guarantees STA initialization, unlike a reused I/O pool.
        std::thread::Builder::new()
            .name("journal-file-dialog".into())
            .spawn(move || {
                let _ = send.send(native::choose(owner, kind == "folder"));
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
                    Common::COMDLG_FILTERSPEC, FileOpenDialog, IFileOpenDialog,
                    FOS_FORCEFILESYSTEM, FOS_NOCHANGEDIR, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS,
                    SIGDN_FILESYSPATH,
                },
            },
        },
    };

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

    pub fn choose(owner: usize, folder: bool) -> Result<Option<String>, String> {
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
            let mut options = dialog.GetOptions().map_err(|error| error.to_string())?
                | FOS_FORCEFILESYSTEM
                | FOS_PATHMUSTEXIST
                | FOS_NOCHANGEDIR;
            if folder {
                options |= FOS_PICKFOLDERS;
                dialog
                    .SetTitle(w!("选择地图包文件夹"))
                    .map_err(|error| error.to_string())?;
            } else {
                dialog
                    .SetTitle(w!("选择 ZIP 地图包"))
                    .map_err(|error| error.to_string())?;
                dialog
                    .SetFileTypes(&[COMDLG_FILTERSPEC {
                        pszName: w!("ZIP 地图包 (*.zip)"),
                        pszSpec: w!("*.zip"),
                    }])
                    .map_err(|error| error.to_string())?;
            }
            dialog
                .SetOptions(options)
                .map_err(|error| error.to_string())?;
            // Keep the shell's remembered folder, grouping, sorting and view mode.
            if let Err(error) = dialog.Show(Some(HWND(owner as *mut _))) {
                if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                    return Ok(None);
                }
                return Err(format!("打开文件选择器失败：{error}"));
            }
            let item = dialog.GetResult().map_err(|error| error.to_string())?;
            let raw = item
                .GetDisplayName(SIGDN_FILESYSPATH)
                .map_err(|error| error.to_string())?;
            let path = raw.to_string();
            CoTaskMemFree(Some(raw.0.cast()));
            path.map(Some).map_err(|error| error.to_string())
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    #[ignore = "Opens the native ZIP picker; close it with Cancel to complete"]
    fn native_zip_picker_cancel() {
        assert_eq!(super::native::choose(0, false).unwrap(), None);
    }
}
