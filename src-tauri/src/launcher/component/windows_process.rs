use std::{
    ffi::OsStr,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_TIMEOUT},
        NetworkManagement::IpHelper::{
            GetExtendedTcpTable, MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
        },
        Networking::WinSock::AF_INET,
        System::Threading::{
            CreateProcessW, TerminateProcess, WaitForSingleObject, CREATE_NEW_CONSOLE,
            CREATE_UNICODE_ENVIRONMENT, PROCESS_INFORMATION, STARTF_USESHOWWINDOW, STARTUPINFOW,
        },
    },
};

pub(super) struct OwnedProcess {
    handle: OwnedHandle,
    pub pid: u32,
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

impl OwnedProcess {
    pub fn spawn(
        python: &Path,
        script: &Path,
        port: u16,
        program_data: &Path,
    ) -> Result<Self, String> {
        if !python.is_absolute() || !script.is_absolute() || !python.is_file() || !script.is_file()
        {
            return Err("登录组件运行文件不存在".into());
        }
        let application = wide(python.as_os_str());
        // Both arguments are existing Windows file paths, so neither can contain a quote.
        let mut command = OsStr::new("\"").encode_wide().collect::<Vec<_>>();
        command.extend(python.as_os_str().encode_wide());
        command.extend(OsStr::new("\" \"").encode_wide());
        command.extend(script.as_os_str().encode_wide());
        command.extend(format!("\" --proxy-port {port}").encode_utf16());
        command.push(0);
        let cwd = wide(script.parent().ok_or("登录组件目录无效")?.as_os_str());
        let mut environment: Vec<_> = std::env::vars_os()
            .filter(|(key, _)| {
                !["PROGRAMDATA", "PYTHONHOME", "PYTHONPATH"]
                    .iter()
                    .any(|name| key.eq_ignore_ascii_case(name))
            })
            .collect();
        environment.push(("PROGRAMDATA".into(), program_data.as_os_str().to_owned()));
        environment.sort_by_key(|(key, _)| key.to_string_lossy().to_uppercase());
        let mut block = Vec::new();
        for (key, value) in environment {
            block.extend(key.encode_wide());
            block.push(b'=' as u16);
            block.extend(value.encode_wide());
            block.push(0);
        }
        block.push(0);
        let startup = STARTUPINFOW {
            cb: std::mem::size_of::<STARTUPINFOW>() as u32,
            dwFlags: STARTF_USESHOWWINDOW,
            wShowWindow: 0,
            ..Default::default()
        };
        let mut process = PROCESS_INFORMATION::default();
        unsafe {
            CreateProcessW(
                PCWSTR(application.as_ptr()),
                Some(PWSTR(command.as_mut_ptr())),
                None,
                None,
                false,
                CREATE_NEW_CONSOLE | CREATE_UNICODE_ENVIRONMENT,
                Some(block.as_ptr().cast()),
                PCWSTR(cwd.as_ptr()),
                &startup,
                &mut process,
            )
            .map_err(|error| format!("无法启动登录组件：{error}"))?;
            let _ = CloseHandle(process.hThread);
            Ok(Self {
                handle: OwnedHandle::from_raw_handle(process.hProcess.0),
                pid: process.dwProcessId,
            })
        }
    }

    pub fn is_running(&self) -> bool {
        self.wait(0) == WAIT_TIMEOUT
    }

    pub fn wait(&self, milliseconds: u32) -> windows::Win32::Foundation::WAIT_EVENT {
        unsafe { WaitForSingleObject(HANDLE(self.handle.as_raw_handle()), milliseconds) }
    }

    pub fn terminate(&self) -> Result<(), String> {
        if self.is_running() {
            unsafe { TerminateProcess(HANDLE(self.handle.as_raw_handle()), 1) }
                .map_err(|e| e.to_string())?;
            self.wait(2000);
        }
        Ok(())
    }

    pub fn owns_listener(&self, port: u16) -> bool {
        if !self.is_running() {
            return false;
        }
        let mut size = 0;
        unsafe {
            GetExtendedTcpTable(
                None,
                &mut size,
                false,
                AF_INET.0 as u32,
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            );
        }
        if size < 4 {
            return false;
        }
        // DWORD storage keeps the native table aligned. Retry later if the table grows.
        let mut buffer = vec![0u32; (size as usize).div_ceil(4)];
        let status = unsafe {
            GetExtendedTcpTable(
                Some(buffer.as_mut_ptr().cast()),
                &mut size,
                false,
                AF_INET.0 as u32,
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            )
        };
        if status != 0 {
            return false;
        }
        let table = buffer.as_ptr().cast::<MIB_TCPTABLE_OWNER_PID>();
        let count = unsafe { (*table).dwNumEntries } as usize;
        if count
            > (buffer.len() * 4 - 4)
                / std::mem::size_of::<
                    windows::Win32::NetworkManagement::IpHelper::MIB_TCPROW_OWNER_PID,
                >()
        {
            return false;
        }
        let rows =
            unsafe { std::slice::from_raw_parts(std::ptr::addr_of!((*table).table).cast(), count) };
        rows.iter().any(
            |row: &windows::Win32::NetworkManagement::IpHelper::MIB_TCPROW_OWNER_PID| {
                row.dwOwningPid == self.pid
                    && row.dwLocalAddr.to_ne_bytes() == [127, 0, 0, 1]
                    && u16::from_be(row.dwLocalPort as u16) == port
            },
        )
    }
}

impl Drop for OwnedProcess {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}
