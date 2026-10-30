use std::{
    io::{Read, Write},
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::Path,
    process::{Child, Command, Stdio},
};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_TIMEOUT},
    NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
    },
    Networking::WinSock::AF_INET,
    System::Threading::{TerminateProcess, WaitForSingleObject},
};

pub(super) struct OwnedProcess {
    child: Child,
    pub pid: u32,
}

fn log_record(line: &[u8]) -> Option<serde_json::Value> {
    let line = std::str::from_utf8(line)
        .ok()?
        .strip_prefix("IDV_ASSISTANT_LOG ")?;
    let record: serde_json::Value = serde_json::from_str(line).ok()?;
    let severity = record["level"].as_str()?;
    let level = match severity {
        "INFO" => "info",
        "WARNING" => "warning",
        "ERROR" | "CRITICAL" => "error",
        _ => return None,
    };
    let message: String = record["message"].as_str()?.chars().take(4000).collect();
    Some(
        serde_json::json!({ "type": "log", "source": "idv-login", "timestamp": chrono::Utc::now().to_rfc3339(), "level": level,
        "message": if severity == "CRITICAL" { format!("[CRITICAL] {message}") } else { message }, "data": {"important": true, "severity": severity} }),
    )
}

fn drain(mut reader: impl Read + Send + 'static, app: Option<tauri::AppHandle>) {
    std::thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        let mut line = Vec::new();
        let mut oversized = false;
        while let Ok(count) = reader.read(&mut buffer) {
            if count == 0 {
                break;
            }
            for byte in &buffer[..count] {
                if *byte == b'\n' {
                    if !oversized {
                        if let (Some(app), Some(record)) = (&app, log_record(&line)) {
                            crate::logs::emit(app, record);
                        }
                    }
                    line.clear();
                    oversized = false;
                } else if line.len() < 32768 {
                    line.push(*byte);
                } else {
                    oversized = true;
                }
            }
        }
    });
}

impl OwnedProcess {
    pub fn spawn(
        python: &Path,
        script: &Path,
        port: u16,
        program_data: &Path,
        app: Option<&tauri::AppHandle>,
    ) -> Result<Self, String> {
        if !python.is_absolute() || !script.is_absolute() || !python.is_file() || !script.is_file()
        {
            return Err("登录组件运行文件不存在".into());
        }
        let source = script.parent().ok_or("登录组件目录无效")?;
        let bootstrap = program_data.join("assistant-bootstrap.py");
        let entry = if script.extension().is_some_and(|ext| ext == "pyc") {
            bootstrap.as_path()
        } else {
            script
        };
        let mut child = Command::new(python)
            .args(["-I", "-B", "-u"])
            .arg(entry)
            .args(["--proxy-port", &port.to_string()])
            .current_dir(program_data)
            .env("PROGRAMDATA", program_data)
            .env("IDV_ASSISTANT_SRC", source)
            .env("PYTHONIOENCODING", "utf-8")
            .env_remove("PYTHONHOME")
            .env_remove("PYTHONPATH")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| format!("无法启动登录组件：{e}"))?;
        drain(child.stdout.take().ok_or("无法读取组件输出")?, app.cloned());
        drain(
            child.stderr.take().ok_or("无法读取组件错误输出")?,
            app.cloned(),
        );
        Ok(Self {
            pid: child.id(),
            child,
        })
    }

    pub fn request_stop(&mut self) -> Result<(), String> {
        if let Some(mut input) = self.child.stdin.take() {
            input.write_all(b"stop\n").map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.wait(0) == WAIT_TIMEOUT
    }

    pub fn wait(&self, milliseconds: u32) -> windows::Win32::Foundation::WAIT_EVENT {
        unsafe { WaitForSingleObject(HANDLE(self.child.as_raw_handle()), milliseconds) }
    }

    pub fn terminate(&self) -> Result<(), String> {
        if self.is_running() {
            unsafe { TerminateProcess(HANDLE(self.child.as_raw_handle()), 1) }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forwards_supported_log_levels_and_marks_critical() {
        assert!(log_record(br#"IDV_ASSISTANT_LOG {"level":"DEBUG","message":"skip"}"#).is_none());
        let event =
            log_record(br#"IDV_ASSISTANT_LOG {"level":"CRITICAL","message":"failed"}"#).unwrap();
        assert_eq!(event["level"], "error");
        assert_eq!(event["message"], "[CRITICAL] failed");
        assert_eq!(event["data"]["important"], true);
        assert!(log_record(b"unstructured stdout").is_none());
    }
}
