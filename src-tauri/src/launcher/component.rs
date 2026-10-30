use std::{
    net::TcpListener,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

mod windows_process;
use windows_process::OwnedProcess;

pub(super) const SUPPORTED_VERSION: &str = "v6.3.2-stable";

#[derive(Clone)]
pub(super) struct LoginRuntime {
    pub python: PathBuf,
    pub script: PathBuf,
}

pub(super) struct ProxySession {
    process: OwnedProcess,
    pub port: u16,
    runtime: LoginRuntime,
    program_data: PathBuf,
    app: Option<tauri::AppHandle>,
    restarts: u8,
    health_failures: u8,
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

    #[cfg(test)]
    pub async fn start_cancellable(
        runtime: &LoginRuntime,
        program_data: &Path,
        timeout: Duration,
        cancelled: &AtomicBool,
    ) -> Result<Self, String> {
        Self::start_logged(runtime, program_data, timeout, cancelled, None, |_| {}).await
    }

    pub async fn start_logged(
        runtime: &LoginRuntime,
        program_data: &Path,
        timeout: Duration,
        cancelled: &AtomicBool,
        app: Option<&tauri::AppHandle>,
        progress: impl Fn(&'static str),
    ) -> Result<Self, String> {
        if cancelled.load(Ordering::Relaxed) {
            return Err("登录组件启动已取消".into());
        }
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        drop(listener);
        let process =
            OwnedProcess::spawn(&runtime.python, &runtime.script, port, program_data, app)?;
        let mut session = Self {
            process,
            port,
            runtime: runtime.clone(),
            program_data: program_data.to_path_buf(),
            app: app.cloned(),
            restarts: 0,
            health_failures: 0,
        };
        let result = session.wait_ready(timeout, cancelled, &progress).await;
        match result {
            Ok(()) => Ok(session),
            Err(error) => {
                let _ = tokio::task::spawn_blocking(move || session.stop()).await;
                Err(error)
            }
        }
    }

    async fn wait_ready(
        &mut self,
        timeout: Duration,
        cancelled: &AtomicBool,
        progress: &impl Fn(&'static str),
    ) -> Result<(), String> {
        let mut deadline = Instant::now() + timeout;
        loop {
            if cancelled.load(Ordering::Relaxed) {
                break Err("登录组件启动已取消".into());
            }
            if self.process.exit_code() == Some(75) {
                progress("restarting");
            }
            if self.restart_if_requested(cancelled).await? {
                deadline = Instant::now() + timeout;
            }
            let phase = self.process.phase();
            progress(phase);
            if !self.is_running() {
                break Err("登录组件在就绪前退出，可能发生了更新或重启".into());
            }
            // Official update prompts may wait for user input. Cancellation is
            // still checked above; allow a fresh initialization budget afterward.
            if phase == "updating" {
                deadline = Instant::now() + timeout;
            } else if !matches!(phase, "restarting" | "stopping")
                && self.process.owns_listener(self.port)
            {
                progress("checking");
                match self.verify_ready().await {
                    Ok(()) => {
                        self.health_failures = 0;
                        self.restarts = 0;
                        break Ok(());
                    }
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
        }
    }

    async fn restart_if_requested(&mut self, cancelled: &AtomicBool) -> Result<bool, String> {
        if self.process.exit_code() != Some(75) {
            return Ok(false);
        }
        super::install::check_cancel(cancelled)?;
        if self.restarts >= 3 {
            return Err("登录组件连续请求重启，请检查官方更新或回滚提示".into());
        }
        if self
            .runtime
            .script
            .extension()
            .is_some_and(|ext| ext == "pyc")
        {
            super::install::cancellable(super::runtime::verify_version(&self.runtime), cancelled)
                .await?;
            super::runtime::prepare_data(&self.program_data)?;
        }
        super::install::check_cancel(cancelled)?;
        self.process = OwnedProcess::spawn(
            &self.runtime.python,
            &self.runtime.script,
            self.port,
            &self.program_data,
            self.app.as_ref(),
        )?;
        self.restarts += 1;
        Ok(true)
    }

    pub async fn refresh(
        &mut self,
        cancelled: &AtomicBool,
        progress: impl Fn(&'static str),
    ) -> Result<&'static str, String> {
        if self.process.exit_code() == Some(75) {
            progress("restarting");
            self.restart_if_requested(cancelled).await?;
            self.wait_ready(Duration::from_secs(60), cancelled, &progress)
                .await?;
        }
        // An update/restart in progress is distinct from an unhealthy proxy.
        if self.is_running()
            && matches!(self.process.phase(), "updating" | "restarting" | "stopping")
        {
            return Ok(self.process.phase());
        }
        match self.verify_ready().await {
            Ok(()) => {
                self.health_failures = 0;
                Ok("ready")
            }
            Err(error) => {
                self.health_failures = self.health_failures.saturating_add(1);
                if !self.is_running()
                    || error.contains("代理模式")
                    || error.contains("组件版本")
                    || self.health_failures >= 3
                {
                    Err(error)
                } else {
                    Ok("checking")
                }
            }
        }
    }

    pub async fn ensure_ready(
        &mut self,
        cancelled: &AtomicBool,
        progress: impl Fn(&'static str),
    ) -> Result<(), String> {
        self.wait_ready(Duration::from_secs(10), cancelled, &progress)
            .await
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

    pub fn stop(mut self) -> Result<(), String> {
        if !self.is_running() {
            return Ok(());
        }
        let signal = self.process.request_stop();
        self.process.wait(15000);
        if self.is_running() {
            self.process.terminate()?;
            return Err(format!(
                "登录组件未响应退出，已结束本助手持有的进程（停止请求：{signal:?}）"
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
    use std::process::Command;

    #[tokio::test]
    async fn update_prompt_outlives_readiness_budget_and_remains_cancellable() {
        let data = tempfile::tempdir().unwrap();
        std::fs::write(data.path().join("fixture-mode"), "update-prompt").unwrap();
        let cancelled = AtomicBool::new(false);
        let updating_at = std::sync::Mutex::new(None::<Instant>);
        let progress = |phase| {
            if phase == "updating" {
                let mut started = updating_at.lock().unwrap();
                let started = started.get_or_insert_with(Instant::now);
                if started.elapsed() > Duration::from_secs(2) {
                    cancelled.store(true, Ordering::Relaxed);
                }
            }
        };
        let result = ProxySession::start_logged(
            &fixture_runtime(),
            data.path(),
            Duration::from_secs(1),
            &cancelled,
            None,
            progress,
        )
        .await;
        assert!(result.err().unwrap().contains("取消"));
        assert!(data.path().join("stopped").exists());
    }

    #[tokio::test]
    async fn managed_hotfix_restart_reaches_ready_and_remains_owned() {
        let data = tempfile::tempdir().unwrap();
        std::fs::write(data.path().join("fixture-mode"), "restart-once").unwrap();
        let session = ProxySession::start(&fixture_runtime(), data.path(), Duration::from_secs(10))
            .await
            .unwrap();
        assert!(session.verify_ready().await.is_ok());
        session.stop().unwrap();
        assert!(data.path().join("stopped").exists());
    }

    #[tokio::test]
    async fn runtime_restart_keeps_port_and_cancellation_prevents_respawn() {
        let data = tempfile::tempdir().unwrap();
        let mut session =
            ProxySession::start(&fixture_runtime(), data.path(), Duration::from_secs(10))
                .await
                .unwrap();
        let port = session.port;
        let pid = session.process.pid;
        std::fs::write(data.path().join("restart-request"), "first").unwrap();
        session.process.wait(3000);
        assert_eq!(session.process.exit_code(), Some(75));
        let stages = std::sync::Mutex::new(Vec::new());
        assert_eq!(
            session
                .refresh(&AtomicBool::new(false), |phase| stages
                    .lock()
                    .unwrap()
                    .push(phase))
                .await
                .unwrap(),
            "ready"
        );
        assert!(stages.lock().unwrap().contains(&"restarting"));
        assert_eq!(session.port, port);
        assert_ne!(session.process.pid, pid);
        std::fs::write(data.path().join("restart-request"), "second").unwrap();
        session.process.wait(3000);
        assert!(session
            .refresh(&AtomicBool::new(true), |_| {})
            .await
            .unwrap_err()
            .contains("取消"));
        assert!(!session.is_running());
    }

    #[tokio::test]
    async fn refresh_debounces_transient_failure_but_detects_persistent_failure_and_exit() {
        let data = tempfile::tempdir().unwrap();
        let mut session =
            ProxySession::start(&fixture_runtime(), data.path(), Duration::from_secs(10))
                .await
                .unwrap();
        let cancelled = AtomicBool::new(false);
        std::fs::write(data.path().join("unhealthy"), "busy").unwrap();
        assert_eq!(
            session.refresh(&cancelled, |_| {}).await.unwrap(),
            "checking"
        );
        std::fs::write(data.path().join("unhealthy"), "").unwrap();
        assert_eq!(session.refresh(&cancelled, |_| {}).await.unwrap(), "ready");
        std::fs::write(data.path().join("unhealthy"), "busy").unwrap();
        assert_eq!(
            session.refresh(&cancelled, |_| {}).await.unwrap(),
            "checking"
        );
        assert_eq!(
            session.refresh(&cancelled, |_| {}).await.unwrap(),
            "checking"
        );
        assert!(session.refresh(&cancelled, |_| {}).await.is_err());
        session.process.terminate().unwrap();
        assert!(session.refresh(&cancelled, |_| {}).await.is_err());
    }

    #[tokio::test]
    async fn normal_exit_does_not_restart_and_repeated_restart_is_bounded() {
        let data = tempfile::tempdir().unwrap();
        std::fs::write(data.path().join("fixture-mode"), "normal-exit").unwrap();
        assert!(
            ProxySession::start(&fixture_runtime(), data.path(), Duration::from_secs(10))
                .await
                .err()
                .unwrap()
                .contains("退出")
        );
        std::fs::write(data.path().join("fixture-mode"), "restart-always").unwrap();
        assert!(
            ProxySession::start(&fixture_runtime(), data.path(), Duration::from_secs(15))
                .await
                .err()
                .unwrap()
                .contains("连续请求重启")
        );
    }

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
        assert_eq!(
            std::fs::read_to_string(data.path().join("console-window")).unwrap(),
            "0"
        );
        std::fs::write(data.path().join("unhealthy"), "fixture").unwrap();
        assert!(
            owned.verify_ready().await.is_err(),
            "a live process alone is not a healthy proxy"
        );
        owned.stop().unwrap();
        assert!(
            data.path().join("stopped").is_file(),
            "component must receive graceful shutdown"
        );
        assert!(external.is_running(), "other component must remain running");
        external.process.terminate().unwrap();
        assert!(
            external.verify_ready().await.is_err(),
            "manual exit must invalidate readiness"
        );
        external.stop().unwrap();
        assert!(
            std::env::var_os("HTTP_PROXY") == before,
            "parent proxy environment changed"
        );
    }
}
