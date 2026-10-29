use std::{
    net::TcpListener,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

mod windows_process;
use windows_process::OwnedProcess;

pub(super) const SUPPORTED_VERSION: &str = "v6.3.2-stable";

pub(super) struct LoginRuntime {
    pub python: PathBuf,
    pub script: PathBuf,
}

pub(super) struct ProxySession {
    process: OwnedProcess,
    python: PathBuf,
    pub port: u16,
}

impl ProxySession {
    #[cfg(test)]
    pub async fn start(
        runtime: &LoginRuntime,
        program_data: &Path,
        timeout: Duration,
    ) -> Result<Self, String> {
        Self::start_cancellable(runtime, program_data, timeout, &AtomicBool::new(false)).await
    }

    pub async fn start_cancellable(
        runtime: &LoginRuntime,
        program_data: &Path,
        timeout: Duration,
        cancelled: &AtomicBool,
    ) -> Result<Self, String> {
        if cancelled.load(Ordering::Relaxed) {
            return Err("登录组件启动已取消".into());
        }
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        drop(listener);
        let process = OwnedProcess::spawn(&runtime.python, &runtime.script, port, program_data)?;
        let session = Self {
            process,
            python: runtime.python.clone(),
            port,
        };
        let deadline = Instant::now() + timeout;
        let result = loop {
            if cancelled.load(Ordering::Relaxed) {
                break Err("登录组件启动已取消".into());
            }
            if !session.is_running() {
                break Err("登录组件在就绪前退出，可能发生了更新或重启".into());
            }
            if session.process.owns_listener(port) {
                match session.verify_ready().await {
                    Ok(()) => break Ok(()),
                    Err(error) if error.contains("代理模式") || error.contains("组件版本") => {
                        break Err(error)
                    }
                    Err(_) => {}
                }
            }
            if Instant::now() >= deadline {
                break Err("等待登录组件超时，请检查其更新或登录提示".into());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        };
        match result {
            Ok(()) => Ok(session),
            Err(error) => {
                let _ = tokio::task::spawn_blocking(move || session.stop()).await;
                Err(error)
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.process.is_running()
    }

    pub async fn verify_ready(&self) -> Result<(), String> {
        if !self.process.owns_listener(self.port) {
            return Err("登录组件尚未监听指定端口".into());
        }
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::Client::builder()
            .no_proxy()
            .proxy(
                reqwest::Proxy::http(format!("http://127.0.0.1:{}", self.port))
                    .map_err(|e| e.to_string())?,
            )
            .timeout(Duration::from_secs(1))
            .build()
            .map_err(|e| e.to_string())?;
        let health = local_json(&client, "health").await?;
        if health["success"] != true || health["status"] != "ok" {
            return Err("登录组件尚未就绪".into());
        }
        if health["version"] != SUPPORTED_VERSION {
            return Err("登录组件版本不受支持，请使用 v6.3.2-stable".into());
        }
        let mode = local_json(&client, "proxy-mode").await?;
        if mode["success"] != true || mode["mode"] != "process" {
            return Err("登录组件必须使用进程代理模式".into());
        }
        // Do not accept a response if the original process exited during the request.
        if !self.process.owns_listener(self.port) {
            return Err("登录组件已退出".into());
        }
        Ok(())
    }

    pub fn stop(self) -> Result<(), String> {
        if !self.is_running() {
            return Ok(());
        }
        // A short-lived Python process attaches to the component's hidden console.
        // This leaves the assistant's own console/stdio untouched in GUI and test runs.
        // Signal the dedicated console. The signal helper ignores the event;
        // no game or external instance shares this console.
        const SIGNAL: &str = "import ctypes,sys,time; k=ctypes.windll.kernel32; p=int(sys.argv[1]); k.FreeConsole(); attached=k.AttachConsole(p); handler=ctypes.WINFUNCTYPE(ctypes.c_int,ctypes.c_uint)(lambda event:1); k.SetConsoleCtrlHandler(handler,True); sent=attached and k.GenerateConsoleCtrlEvent(1,0); time.sleep(0.1); k.FreeConsole() if attached else None; sys.exit(0 if sent else 1)";
        let mut signal = Command::new(&self.python)
            .args(["-c", SIGNAL, &self.process.pid.to_string()])
            .env_remove("PYTHONHOME")
            .env_remove("PYTHONPATH")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(3);
        let signal_status = loop {
            if let Some(status) = signal.try_wait().map_err(|e| e.to_string())? {
                break Some(status);
            }
            if Instant::now() >= deadline {
                let _ = signal.kill();
                let _ = signal.wait();
                break None;
            }
            std::thread::sleep(Duration::from_millis(25));
        };
        self.process.wait(6000);
        if self.is_running() {
            self.process.terminate()?;
            return Err(format!(
                "登录组件未响应退出，已结束本助手持有的进程（退出信号：{signal_status:?}）"
            ));
        }
        Ok(())
    }
}

async fn local_json(client: &reqwest::Client, action: &str) -> Result<serde_json::Value, String> {
    let mut response = client
        .get(format!("http://localhost/_idv-login/{action}"))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| e.to_string())?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if body.len() + chunk.len() > 16 * 1024 {
            return Err("登录组件响应过大".into());
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancelling_startup_stops_the_owned_component_promptly() {
        let data = tempfile::tempdir().unwrap();
        std::fs::write(data.path().join("fixture-mode"), "not-ready").unwrap();
        let cancelled = std::sync::atomic::AtomicBool::new(false);
        let runtime = fixture_runtime();
        let start = ProxySession::start_cancellable(
            &runtime,
            data.path(),
            Duration::from_secs(60),
            &cancelled,
        );
        let cancel = async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
        };
        let (result, _) = tokio::time::timeout(Duration::from_secs(12), async {
            tokio::join!(start, cancel)
        })
        .await
        .unwrap();
        assert!(result.err().unwrap().contains("取消"));
        assert!(data.path().join("stopped").is_file());
    }

    fn fixture_runtime() -> LoginRuntime {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        // Windows venv python.exe is a redirector. Use the real interpreter,
        // matching the upstream embedded runtime's direct process ownership.
        let python = Command::new(root.join("../engine/.venv/Scripts/python.exe"))
            .args(["-c", "import sys; print(sys._base_executable)"])
            .output()
            .unwrap();
        assert!(python.status.success());
        LoginRuntime {
            python: PathBuf::from(String::from_utf8(python.stdout).unwrap().trim()),
            script: root.join("tests/fixtures/login_component.py"),
        }
    }

    #[tokio::test]
    async fn rejects_global_mode_and_cleans_up_the_owned_component() {
        let data = tempfile::tempdir().unwrap();
        std::fs::write(data.path().join("fixture-mode"), "global").unwrap();
        let result =
            ProxySession::start(&fixture_runtime(), data.path(), Duration::from_secs(10)).await;
        assert!(result.err().unwrap().contains("进程代理模式"));
        assert!(data.path().join("stopped").is_file());
    }

    #[tokio::test]
    async fn reports_early_exit_and_cleans_up_after_readiness_timeout() {
        let data = tempfile::tempdir().unwrap();
        std::fs::write(data.path().join("fixture-mode"), "exit").unwrap();
        let result =
            ProxySession::start(&fixture_runtime(), data.path(), Duration::from_secs(3)).await;
        assert!(result.err().unwrap().contains("退出"));
        std::fs::write(data.path().join("fixture-mode"), "not-ready").unwrap();
        let result =
            ProxySession::start(&fixture_runtime(), data.path(), Duration::from_secs(3)).await;
        assert!(result.err().unwrap().contains("超时"));
        assert!(data.path().join("stopped").is_file());
    }

    #[tokio::test]
    async fn waits_for_ready_then_gracefully_stops_only_the_owned_component() {
        let data = tempfile::tempdir().unwrap();
        let external_data = tempfile::tempdir().unwrap();
        let before = std::env::var_os("HTTP_PROXY");
        let external = ProxySession::start(
            &fixture_runtime(),
            external_data.path(),
            Duration::from_secs(10),
        )
        .await
        .unwrap();
        let owned = ProxySession::start(&fixture_runtime(), data.path(), Duration::from_secs(10))
            .await
            .unwrap();
        assert!(owned.is_running());
        owned.stop().unwrap();
        assert!(
            data.path().join("stopped").is_file(),
            "component must receive graceful shutdown"
        );
        assert!(external.is_running(), "other component must remain running");
        external.stop().unwrap();
        assert!(
            std::env::var_os("HTTP_PROXY") == before,
            "parent proxy environment changed"
        );
    }
}
