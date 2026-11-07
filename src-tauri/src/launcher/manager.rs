use super::{
    component::ProxySession,
    component_updates::{self, CheckedUpdate, ComponentUpdateInfo},
    install, launch_direct, locate_game,
    runtime::{self, SchemeSnapshot},
};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};
use tauri::Emitter;
use tokio::sync::Mutex as AsyncMutex;

#[derive(Clone, Serialize)]
pub struct LauncherStatus {
    phase: &'static str,
    proxy_phase: &'static str,
    component_path: Option<String>,
    message: Option<String>,
}

struct OwnedLogin {
    session: ProxySession,
    scheme: SchemeSnapshot,
    python: PathBuf,
}

impl OwnedLogin {
    async fn stop(self) -> Result<(), String> {
        tokio::task::spawn_blocking(move || {
            let stopped = self.session.stop();
            let restored = self.scheme.restore(&self.python);
            stopped.and(restored)
        })
        .await
        .map_err(|e| e.to_string())?
    }
}

struct UpdateGuard<'a>(&'a AtomicBool, &'a AtomicBool);
impl Drop for UpdateGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
        self.1.store(false, Ordering::Release);
    }
}

pub struct Launcher {
    operation: AsyncMutex<()>,
    owned: AsyncMutex<Option<OwnedLogin>>,
    status: Mutex<LauncherStatus>,
    closing: AtomicBool,
    update_busy: AtomicBool,
    applying: AtomicBool,
    cancelled: AtomicBool,
    update: Mutex<Option<CheckedUpdate>>,
    log_app: std::sync::OnceLock<tauri::AppHandle>,
}

impl Default for Launcher {
    fn default() -> Self {
        Self {
            operation: AsyncMutex::new(()),
            owned: AsyncMutex::new(None),
            status: Mutex::new(LauncherStatus {
                phase: "idle",
                proxy_phase: "idle",
                component_path: None,
                message: None,
            }),
            closing: AtomicBool::new(false),
            update_busy: AtomicBool::new(false),
            applying: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            update: Mutex::new(None),
            log_app: std::sync::OnceLock::new(),
        }
    }
}

fn directory() -> Result<PathBuf, String> {
    Ok(crate::config::user_path()?
        .parent()
        .ok_or("助手数据目录无效")?
        .to_path_buf())
}

impl Launcher {
    pub async fn check_updates(&self, source: &str) -> Result<ComponentUpdateInfo, String> {
        let _operation = self.operation.try_lock().map_err(|_| "组件操作正在进行")?;
        if self.closing.load(Ordering::Relaxed) {
            return Err("助手正在退出或更新".into());
        }
        self.cancelled.store(false, Ordering::Relaxed);
        *self.update.lock().unwrap() = None;
        let selected = crate::update_source::resolve(source)?;
        let source = selected.as_str();
        let directory = directory()?;
        let python = runtime::component_path(&directory)?.ok_or("请先配置官方登录组件")?;
        let runtime = runtime::runtime_from_python(python)?;
        let current =
            install::cancellable(runtime::read_version(&runtime), &self.cancelled).await?;
        let (info, release) = component_updates::check(&current, source, &self.cancelled).await?;
        *self.update.lock().unwrap() = Some(CheckedUpdate {
            runtime,
            info: info.clone(),
            release,
        });
        Ok(info)
    }

    pub fn update_busy(&self) -> bool {
        self.update_busy.load(Ordering::Acquire)
    }

    fn update_progress(&self, progress: serde_json::Value) {
        if let Some(app) = self.log_app.get() {
            let _ = app.emit("component-update-progress", &progress);
            crate::logs::record(
                app,
                "debug",
                "launcher.update",
                "组件更新进度 / Component update progress",
                progress,
            );
        }
    }

    pub async fn install_update(&self, source: &str, app: &tauri::AppHandle) -> Result<(), String> {
        let _operation = self.operation.try_lock().map_err(|_| "组件操作正在进行")?;
        if self.closing.load(Ordering::Relaxed) {
            return Err("助手正在退出或更新".into());
        }
        let checked = self
            .update
            .lock()
            .unwrap()
            .take()
            .ok_or("请先检查组件更新")?;
        if !checked.info.available {
            return Err("没有可用的组件更新 / No component update available".into());
        }
        let directory = directory()?;
        if runtime::component_path(&directory)?.as_ref() != Some(&checked.runtime.python) {
            return Err("组件路径已改变，请重新检查更新".into());
        }
        self.update_busy.store(true, Ordering::Release);
        let _busy = UpdateGuard(&self.update_busy, &self.applying);
        self.cancelled.store(false, Ordering::Relaxed);
        self.update_progress(
            serde_json::json!({"phase":"preparing","downloaded":0,"total":null,"cancellable":true}),
        );
        let result = async {
            // Leave the old component running during downloads. Installation targets a separate version directory.
            let replacement = install::install(&checked.release, source, &self.cancelled, |p| self.update_progress(p)).await?;
            install::check_cancel(&self.cancelled)?;
            self.applying.store(true, Ordering::Release);
            self.update_progress(serde_json::json!({"phase":"applying","downloaded":0,"total":null,"cancellable":false}));
            let mut owned = self.owned.lock().await;
            let was_running = owned.as_ref().is_some_and(|c| c.session.is_running());
            let applied = async {
                if let Some(mut component) = owned.take() {
                    let (component, paused) = tauri::async_runtime::spawn_blocking(move || {
                        let paused = component.session.pause();
                        (component, paused)
                    }).await.map_err(|error| error.to_string())?;
                    *owned = Some(component);
                    paused?;
                } else { runtime::ensure_can_start()?; }
                runtime::save_component(&directory, &replacement.python)?;
                if was_running {
                    self.update_progress(serde_json::json!({"phase":"restoring","downloaded":0,"total":null,"cancellable":false}));
                    let session = ProxySession::start_logged(&replacement, &directory.join("login-data"),
                        Duration::from_secs(60), &self.closing, Some(app), |phase| self.report_proxy(phase)).await?;
                    if let Some(component) = owned.as_mut() {
                        component.session = session;
                        component.python = replacement.python.clone();
                    }
                } else if let Some(component) = owned.take() {
                    component.stop().await?;
                }
                Ok::<(), String>(())
            }.await;
            if let Err(mut error) = applied {
                self.update_progress(serde_json::json!({"phase":"restoring","downloaded":0,"total":null,"cancellable":false}));
                if let Err(rollback) = runtime::save_component(&directory, &checked.runtime.python) {
                    error.push_str(&format!("；恢复组件路径失败：{rollback}"));
                }
                if was_running {
                    if let Some(component) = owned.as_mut() {
                        if let Err(rollback) = component.session.restart_after_update(&self.closing, |phase| self.report_proxy(phase)).await {
                            error.push_str(&format!("；恢复旧代理失败：{rollback}"));
                        }
                    }
                }
                return Err(error);
            }
            Ok(())
        }.await;
        self.finish(&result, "idle");
        crate::logs::record(
            app,
            if result.is_ok() { "info" } else { "error" },
            "launcher.update",
            "组件更新结束 / Component update finished",
            serde_json::json!({"success":result.is_ok(),"error":result.as_ref().err()}),
        );
        result
    }
    fn report(&self, phase: &'static str, message: Option<String>) {
        let mut status = self.status.lock().unwrap();
        if status.phase == phase && status.proxy_phase == phase && status.message == message {
            return;
        }
        status.phase = phase;
        status.proxy_phase = phase;
        status.message = message;
        if let Some(app) = self.log_app.get() {
            crate::logs::record(
                app,
                if phase == "error" { "error" } else { "info" },
                "launcher.status",
                "登录组件状态变化 / Login component status changed",
                serde_json::json!({"phase":phase,"message":status.message}),
            );
        }
    }

    fn report_proxy(&self, phase: &'static str) {
        let mut status = self.status.lock().unwrap();
        if status.proxy_phase == phase && status.message.is_none() {
            return;
        }
        status.proxy_phase = phase;
        status.message = None;
        if let Some(app) = self.log_app.get() {
            crate::logs::record(
                app,
                "info",
                "launcher.proxy",
                "登录代理状态变化 / Login proxy status changed",
                serde_json::json!({"phase":phase}),
            );
        }
    }

    async fn refresh_proxy(&self) {
        if self.closing.load(Ordering::Relaxed) {
            return;
        }
        // Skip startup/configuration, which owns this lock and publishes progress.
        let Ok(mut owned) = self.owned.try_lock() else {
            return;
        };
        if let Some(component) = owned.as_mut() {
            match component
                .session
                .refresh(&self.closing, |phase| self.report_proxy(phase))
                .await
            {
                Ok("ready") => self.report("ready", None),
                Ok(phase) => self.report_proxy(phase),
                Err(error) => self.report("error", Some(format!("登录代理不可用：{error}"))),
            }
        }
    }

    pub async fn status(&self) -> Result<LauncherStatus, String> {
        // A snapshot must not wait for network probes or report a stale startup error.
        let mut status = self.status.lock().unwrap().clone();
        let directory = directory()?;
        status.component_path = if let Some(python) = runtime::component_path(&directory)? {
            python
                .parent()
                .and_then(Path::parent)
                .map(|root| root.to_string_lossy().into_owned())
        } else {
            None
        };
        Ok(status)
    }

    pub fn cancel(&self) {
        if self.applying.load(Ordering::Acquire) {
            return;
        }
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub async fn configure(&self, download: bool) -> Result<(), String> {
        let _operation = self.operation.try_lock().map_err(|_| "组件操作正在进行")?;
        self.update_busy.store(true, Ordering::Release);
        let _busy = UpdateGuard(&self.update_busy, &self.applying);
        self.cancelled.store(false, Ordering::Relaxed);
        let owned = self.owned.lock().await;
        if self.closing.load(Ordering::Relaxed) {
            return Err("助手正在退出或更新".into());
        }
        if owned.is_some() {
            return Err("登录组件已启动，请先重启助手再更换组件".into());
        }
        self.report("configuring", Some("正在扫描官方登录组件…".into()));
        let result = async {
            let directory = directory()?;
            install::check_cancel(&self.cancelled)?;
            let found = match runtime::component_path(&directory)? {
                Some(python) => {
                    let candidate = runtime::runtime_from_python(python)?;
                    match install::cancellable(runtime::verify_version(&candidate), &self.cancelled)
                        .await
                    {
                        Ok(()) => Some(candidate),
                        Err(_) if download => None,
                        Err(error) => return Err(error),
                    }
                }
                None => None,
            };
            let component = match found {
                Some(candidate) => candidate,
                None if download => {
                    let config = crate::config::load()?;
                    let source = config["app"]["update_source"].as_str().unwrap_or("direct");
                    let release = component_updates::latest(source, &self.cancelled).await?;
                    install::install(&release, source, &self.cancelled, |progress| self.update_progress(progress))
                    .await?
                }
                None => return Err("未找到官方组件，请点击“官方下载”自动下载并配置".into()),
            };
            install::cancellable(runtime::verify_version(&component), &self.cancelled).await?;
            install::check_cancel(&self.cancelled)?;
            self.applying.store(true, Ordering::Release);
            self.update_progress(serde_json::json!({"phase":"applying","downloaded":0,"total":null,"cancellable":false}));
            runtime::save_component(&directory, &component.python)
        }
        .await;
        self.finish(&result, "idle");
        result
    }

    fn finish(&self, result: &Result<(), String>, success: &'static str) {
        if self.cancelled.load(Ordering::Relaxed) {
            self.report("idle", Some("操作已取消".into()));
        } else {
            match result {
                Ok(()) => self.report(success, None),
                Err(error) => self.report("error", Some(error.clone())),
            }
        }
    }

    pub async fn select_component(&self, python: PathBuf) -> Result<(), String> {
        let _operation = self.operation.try_lock().map_err(|_| "组件操作正在进行")?;
        let owned = self.owned.lock().await;
        if self.closing.load(Ordering::Relaxed) {
            return Err("助手正在退出或更新".into());
        }
        if owned.is_some() {
            return Err("登录组件已启动，请重启助手后更换组件".into());
        }
        let runtime = runtime::runtime_from_selection(python)?;
        runtime::verify_version(&runtime).await?;
        runtime::save_component(&directory()?, &runtime.python)?;
        self.report("idle", None);
        Ok(())
    }

    pub async fn launch(
        &self,
        game: Option<String>,
        app: Option<&tauri::AppHandle>,
    ) -> Result<(), String> {
        let _operation = self.operation.try_lock().map_err(|_| "启动操作正在进行")?;
        self.cancelled.store(false, Ordering::Relaxed);
        let mut owned =
            install::cancellable(async { Ok(self.owned.lock().await) }, &self.cancelled).await?;
        if self.closing.load(Ordering::Relaxed) {
            return Err("助手正在退出或更新".into());
        }
        self.report("starting", Some("正在检查游戏和登录组件…".into()));
        let started_at = std::time::Instant::now();
        let result = self.launch_locked(game, &mut owned, app).await;
        self.finish(&result, "ready");
        if let Some(app) = app {
            let message = match &result {
                Ok(()) => format!(
                    "登录代理检查通过，已启动游戏（耗时 {:.1} 秒）",
                    started_at.elapsed().as_secs_f32()
                ),
                Err(error) => format!(
                    "启动结束（耗时 {:.1} 秒）：{error}",
                    started_at.elapsed().as_secs_f32()
                ),
            };
            crate::logs::emit(
                app,
                serde_json::json!({"type":"log", "source":"launcher", "timestamp":chrono::Utc::now().to_rfc3339(), "level": if result.is_ok() {"info"} else {"warning"}, "message":message, "data":{"important":true}}),
            );
        }
        result
    }

    async fn launch_locked(
        &self,
        game: Option<String>,
        owned: &mut Option<OwnedLogin>,
        app: Option<&tauri::AppHandle>,
    ) -> Result<(), String> {
        install::check_cancel(&self.cancelled)?;
        let game = locate_game(game)?;
        let unhealthy = if let Some(component) = owned.as_mut() {
            component
                .session
                .ensure_ready(&self.cancelled, |phase| self.report_proxy(phase))
                .await
                .is_err()
        } else {
            false
        };
        install::check_cancel(&self.cancelled)?;
        if unhealthy {
            if let Some(previous) = owned.take() {
                previous.stop().await?;
            }
        }
        let mut started = false;
        if owned.is_none() {
            let directory = directory()?;
            let python = runtime::component_path(&directory)?
                .ok_or("请先在启动设置中选择官方 idv-login 组件")?;
            let runtime = runtime::runtime_from_python(python.clone())?;
            install::cancellable(runtime::verify_version(&runtime), &self.cancelled).await?;
            runtime::save_component(&directory, &runtime.python)?;
            runtime::ensure_can_start()?;
            let data = directory.join("login-data");
            runtime::prepare_data(&data)?;
            let scheme = SchemeSnapshot::capture()?;
            self.report(
                "starting",
                Some("正在初始化登录代理，等待端口和健康检查…".into()),
            );
            match ProxySession::start_logged(
                &runtime,
                &data,
                Duration::from_secs(60),
                &self.cancelled,
                app,
                |phase| self.report_proxy(phase),
            )
            .await
            {
                Ok(session) => {
                    *owned = Some(OwnedLogin {
                        session,
                        scheme,
                        python,
                    });
                    started = true;
                }
                Err(mut error) => {
                    if let Err(restore) = scheme.restore(&python) {
                        error.push_str(&format!("；恢复登录协议失败：{restore}"));
                    }
                    return Err(error);
                }
            }
        }
        let result = async {
            let component = owned.as_ref().ok_or("登录组件不可用")?;
            // start_logged/ensure_ready just checked health and ownership.
            if self.closing.load(Ordering::Relaxed) || self.cancelled.load(Ordering::Relaxed) {
                return Err("启动已取消".into());
            }
            self.report("starting", Some("代理已通过检查，正在启动第五人格…".into()));
            launch_direct(Path::new(&game), component.session.port)
        }
        .await;
        if result.is_err() && started {
            if let Some(component) = owned.take() {
                if let Err(cleanup) = component.stop().await {
                    return Err(format!("{}；{cleanup}", result.unwrap_err()));
                }
            }
        }
        result
    }

    pub async fn shutdown(&self) -> Result<(), String> {
        if self.update_busy() {
            return Err(
                "请先完成或取消组件更新 / Finish or cancel the component update first".into(),
            );
        }
        self.closing.store(true, Ordering::Relaxed);
        self.cancel();
        let mut owned = self.owned.lock().await;
        self.report("stopping", None);
        let result = match owned.take() {
            Some(component) => component.stop().await,
            None => Ok(()),
        };
        self.report("idle", result.as_ref().err().cloned());
        result
    }

    pub fn resume(&self) {
        self.closing.store(false, Ordering::Relaxed);
    }
}

#[tauri::command]
pub async fn check_login_updates(
    app: tauri::AppHandle,
    manager: tauri::State<'_, Launcher>,
    source: String,
) -> Result<ComponentUpdateInfo, String> {
    crate::logs::record(
        &app,
        "info",
        "launcher.update",
        "检查组件更新 / Checking component updates",
        serde_json::json!({"source":source}),
    );
    let result = manager.check_updates(&source).await;
    crate::logs::record(
        &app,
        if result.is_ok() { "info" } else { "error" },
        "launcher.update",
        "组件检查结束 / Component check ended",
        serde_json::json!({"success":result.is_ok(),"error":result.as_ref().err()}),
    );
    result
}

#[tauri::command]
pub async fn install_login_update(
    app: tauri::AppHandle,
    manager: tauri::State<'_, Launcher>,
    source: String,
) -> Result<(), String> {
    let source = crate::update_source::resolve(&source)?;
    manager.install_update(&source, &app).await
}

pub fn start_monitor(app: tauri::AppHandle) {
    use tauri::Manager;
    let _ = app.state::<Launcher>().log_app.set(app.clone());
    tauri::async_runtime::spawn(async move {
        loop {
            app.state::<Launcher>().refresh_proxy().await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

#[tauri::command]
pub async fn get_launcher_status(
    manager: tauri::State<'_, Launcher>,
) -> Result<LauncherStatus, String> {
    manager.status().await
}

#[tauri::command]
pub fn cancel_game_launch(app: tauri::AppHandle, manager: tauri::State<'_, Launcher>) {
    crate::logs::record(
        &app,
        "info",
        "launcher",
        "取消启动或组件操作 / Launch or component operation cancelled",
        serde_json::json!({}),
    );
    manager.cancel();
}

#[tauri::command]
pub async fn configure_login_component(
    app: tauri::AppHandle,
    manager: tauri::State<'_, Launcher>,
    download: bool,
) -> Result<(), String> {
    crate::logs::record(
        &app,
        "info",
        "launcher.configure",
        "开始配置登录组件 / Configuring login component",
        serde_json::json!({"download":download}),
    );
    let result = manager.configure(download).await;
    crate::logs::record(
        &app,
        if result.is_ok() { "info" } else { "error" },
        "launcher.configure",
        "组件配置结束 / Component configuration ended",
        serde_json::json!({"success":result.is_ok(),"error":result.as_ref().err()}),
    );
    result
}

#[tauri::command]
pub async fn set_login_component(
    app: tauri::AppHandle,
    manager: tauri::State<'_, Launcher>,
    executable_path: String,
) -> Result<(), String> {
    let result = manager
        .select_component(PathBuf::from(executable_path))
        .await;
    crate::logs::record(
        &app,
        if result.is_ok() { "info" } else { "error" },
        "launcher.configure",
        "选择登录组件 / Login component selection",
        serde_json::json!({"success":result.is_ok(),"error":result.as_ref().err()}),
    );
    result
}

#[tauri::command]
pub async fn launch_game(
    app: tauri::AppHandle,
    manager: tauri::State<'_, Launcher>,
    executable_path: Option<String>,
) -> Result<(), String> {
    let game = match executable_path {
        Some(path) => Some(path),
        None => crate::config::load()?["game_window"]["executable_path"]
            .as_str()
            .map(str::to_owned),
    };
    manager.launch(game, Some(&app)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancel_does_not_wait_for_background_proxy_recovery() {
        let manager = Launcher::default();
        let _background = manager.owned.lock().await;
        let cancel = async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            manager.cancel();
        };
        let (result, _) = tokio::time::timeout(Duration::from_secs(1), async {
            tokio::join!(manager.launch(Some("Fever.exe".into()), None), cancel)
        })
        .await
        .unwrap();
        assert!(result.unwrap_err().contains("取消"));
    }

    #[tokio::test]
    async fn health_poll_lock_does_not_reject_a_user_launch() {
        let manager = Launcher::default();
        let health_guard = manager.owned.lock().await;
        let release = async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            drop(health_guard);
        };
        let (result, _) = tokio::join!(manager.launch(Some("Fever.exe".into()), None), release);
        assert!(result.unwrap_err().contains("dwrg.exe"));
    }

    #[tokio::test]
    async fn rejects_platform_paths_and_blocks_launch_until_failed_update_is_resumed() {
        let manager = Launcher::default();
        assert!(manager
            .launch(Some("Fever.exe".into()), None)
            .await
            .unwrap_err()
            .contains("dwrg.exe"));
        manager.shutdown().await.unwrap();
        assert!(manager
            .launch(Some("Fever.exe".into()), None)
            .await
            .unwrap_err()
            .contains("退出或更新"));
        manager.resume();
        assert!(manager
            .launch(Some("Fever.exe".into()), None)
            .await
            .unwrap_err()
            .contains("dwrg.exe"));
        assert!(manager
            .select_component(PathBuf::from("installer.exe"))
            .await
            .is_err());
    }
}
