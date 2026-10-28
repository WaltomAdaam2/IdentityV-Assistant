//! Modern Windows common item dialogs; no helper process or custom file listing.

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
    use std::{cell::RefCell, rc::Rc};
    use windows::{
        core::{implement, w, Interface, Ref, BOOL, HRESULT},
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
                    Common::COMDLG_FILTERSPEC, FileOpenDialog, IFileDialog,
                    IFileDialogControlEvents, IFileDialogControlEvents_Impl, IFileDialogCustomize,
                    IFileDialogEvents, IFileDialogEvents_Impl, IFileOpenDialog, IShellItem,
                    IShellItemArray, FDEOR_DEFAULT, FDESVR_DEFAULT, FDE_OVERWRITE_RESPONSE,
                    FDE_SHAREVIOLATION_RESPONSE, FOS_ALLOWMULTISELECT, FOS_FORCEFILESYSTEM,
                    FOS_NOCHANGEDIR, FOS_PATHMUSTEXIST, SIGDN_FILESYSPATH,
                },
            },
        },
    };

    const IMPORT_SELECTION: u32 = 100;

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

    // The standard Open button navigates into folders. A native custom button
    // accepts the current shell selection, including mixed folders and ZIPs.
    #[implement(IFileDialogEvents, IFileDialogControlEvents)]
    struct SelectionEvents {
        selected: Rc<RefCell<Option<Vec<String>>>>,
    }

    impl IFileDialogControlEvents_Impl for SelectionEvents_Impl {
        fn OnButtonClicked(
            &self,
            customize: Ref<IFileDialogCustomize>,
            id: u32,
        ) -> windows::core::Result<()> {
            if id == IMPORT_SELECTION {
                let dialog: IFileOpenDialog = customize.ok()?.cast()?;
                unsafe {
                    let selected = paths(&dialog.GetSelectedItems()?)?;
                    if !selected.is_empty() {
                        *self.selected.borrow_mut() = Some(selected);
                        dialog.Close(HRESULT(0))?;
                    }
                }
            }
            Ok(())
        }
        fn OnItemSelected(
            &self,
            _: Ref<IFileDialogCustomize>,
            _: u32,
            _: u32,
        ) -> windows::core::Result<()> {
            Ok(())
        }
        fn OnCheckButtonToggled(
            &self,
            _: Ref<IFileDialogCustomize>,
            _: u32,
            _: BOOL,
        ) -> windows::core::Result<()> {
            Ok(())
        }
        fn OnControlActivating(
            &self,
            _: Ref<IFileDialogCustomize>,
            _: u32,
        ) -> windows::core::Result<()> {
            Ok(())
        }
    }

    impl IFileDialogEvents_Impl for SelectionEvents_Impl {
        fn OnFileOk(&self, _: Ref<IFileDialog>) -> windows::core::Result<()> {
            Ok(())
        }
        fn OnFolderChanging(
            &self,
            _: Ref<IFileDialog>,
            _: Ref<IShellItem>,
        ) -> windows::core::Result<()> {
            Ok(())
        }
        fn OnFolderChange(&self, _: Ref<IFileDialog>) -> windows::core::Result<()> {
            Ok(())
        }
        fn OnSelectionChange(&self, _: Ref<IFileDialog>) -> windows::core::Result<()> {
            Ok(())
        }
        fn OnShareViolation(
            &self,
            _: Ref<IFileDialog>,
            _: Ref<IShellItem>,
        ) -> windows::core::Result<FDE_SHAREVIOLATION_RESPONSE> {
            Ok(FDESVR_DEFAULT)
        }
        fn OnTypeChange(&self, _: Ref<IFileDialog>) -> windows::core::Result<()> {
            Ok(())
        }
        fn OnOverwrite(
            &self,
            _: Ref<IFileDialog>,
            _: Ref<IShellItem>,
        ) -> windows::core::Result<FDE_OVERWRITE_RESPONSE> {
            Ok(FDEOR_DEFAULT)
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
            let options = dialog.GetOptions().map_err(|error| error.to_string())?
                | FOS_FORCEFILESYSTEM
                | FOS_PATHMUSTEXIST
                | FOS_NOCHANGEDIR
                | FOS_ALLOWMULTISELECT;
            dialog
                .SetTitle(w!("选择地图包 — Ctrl / Shift 多选文件夹或 ZIP"))
                .map_err(|error| error.to_string())?;
            dialog
                .SetFileTypes(&[COMDLG_FILTERSPEC {
                    pszName: w!("地图包（文件夹或 ZIP）"),
                    pszSpec: w!("*.zip"),
                }])
                .map_err(|error| error.to_string())?;
            let customize: IFileDialogCustomize =
                dialog.cast().map_err(|error| error.to_string())?;
            customize
                .AddPushButton(IMPORT_SELECTION, w!("导入所选项目"))
                .map_err(|error| error.to_string())?;
            dialog
                .SetOptions(options)
                .map_err(|error| error.to_string())?;
            let selected = Rc::new(RefCell::new(None));
            let events: IFileDialogEvents = SelectionEvents {
                selected: selected.clone(),
            }
            .into();
            let cookie = dialog.Advise(&events).map_err(|error| error.to_string())?;
            // Keep the shell's remembered folder, grouping, sorting and view mode.
            let shown = dialog.Show(Some(HWND(owner as *mut _)));
            let _ = dialog.Unadvise(cookie);
            if let Err(error) = shown {
                if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                    return Ok(None);
                }
                return Err(format!("打开文件选择器失败：{error}"));
            }
            if let Some(paths) = selected.borrow_mut().take() {
                return Ok(Some(paths));
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
