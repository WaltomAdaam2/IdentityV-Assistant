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

pub struct Launcher {
    operation: AsyncMutex<()>,
    owned: AsyncMutex<Option<OwnedLogin>>,
    status: Mutex<LauncherStatus>,
    closing: AtomicBool,
    cancelled: AtomicBool,
    update: Mutex<Option<CheckedUpdate>>,
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
            cancelled: AtomicBool::new(false),
            update: Mutex::new(None),
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
        install::cancellable(runtime::verify_version(&runtime), &self.cancelled).await?;
        let info = component_updates::check(
            &runtime,
            &directory.join("login-data"),
            source,
            &self.cancelled,
        )
        .await?;
        *self.update.lock().unwrap() = Some(CheckedUpdate {
            runtime,
            info: info.clone(),
        });
        Ok(info)
    }

    pub async fn apply_hotfixes(&self, source: &str, app: &tauri::AppHandle) -> Result<(), String> {
        crate::updates::route("https://github.com/", source)?;
        let _operation = self.operation.try_lock().map_err(|_| "组件操作正在进行")?;
        if self.closing.load(Ordering::Relaxed) {
            return Err("助手正在退出或更新".into());
        }
        let mut checked = self
            .update
            .lock()
            .unwrap()
            .take()
            .ok_or("请先检查组件更新")?;
        let directory = directory()?;
        if runtime::component_path(&directory)?.as_ref() != Some(&checked.runtime.python) {
            return Err("组件路径已改变，请重新检查更新".into());
        }
        if checked.info.hotfixes.is_empty() {
            return Ok(());
        }
        let mut owned = self.owned.lock().await;
        if self.closing.load(Ordering::Relaxed) {
            return Err("助手正在退出或更新".into());
        }
        self.report_proxy("updating");
        let was_running = owned
            .as_ref()
            .is_some_and(|component| component.session.is_running());
        let mut result = async {
            if let Some(mut component) = owned.take() {
                let (component, result) = tokio::task::spawn_blocking(move || {
                    let result = component.session.pause();
                    (component, result)
                })
                .await
                .map_err(|e| e.to_string())?;
                *owned = Some(component);
                result?;
            } else {
                runtime::ensure_can_start()?;
            }
            let data = directory.join("login-data");
            runtime::prepare_data(&data)?;
            component_updates::apply(&checked, &data, source, &self.closing).await
        }
        .await;
        if was_running && !self.closing.load(Ordering::Relaxed) {
            if let Some(component) = owned
                .as_mut()
                .filter(|component| !component.session.is_running())
            {
                self.report_proxy("restarting");
                if let Err(recovery) = component
                    .session
                    .restart_after_update(&self.closing, |phase| self.report_proxy(phase))
                    .await
                {
                    result = Err(match result {
                        Err(error) => format!("{error}；代理恢复失败：{recovery}"),
                        Ok(()) => recovery,
                    });
                }
            }
        }
        self.report(
            if owned.is_some() && result.is_ok() {
                "ready"
            } else if result.is_err() {
                "error"
            } else {
                "idle"
            },
            result.as_ref().err().cloned(),
        );
        if result.is_ok() {
            checked.info.hotfixes.clear();
            *self.update.lock().unwrap() = Some(checked);
        }
        crate::logs::emit(
            app,
            serde_json::json!({"type":"log", "source":"idv-login", "timestamp":chrono::Utc::now().to_rfc3339(), "level":if result.is_ok() {"info"} else {"error"}, "message":result.as_ref().err().cloned().unwrap_or_else(|| "官方热更新已应用".into()), "data":{"important":true}}),
        );
        result
    }
    fn report(&self, phase: &'static str, message: Option<String>) {
        let mut status = self.status.lock().unwrap();
        status.phase = phase;
        status.proxy_phase = phase;
        status.message = message;
    }

    fn report_proxy(&self, phase: &'static str) {
        let mut status = self.status.lock().unwrap();
        status.proxy_phase = phase;
        status.message = None;
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
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub async fn configure(&self, download: bool) -> Result<(), String> {
        let _operation = self.operation.try_lock().map_err(|_| "组件操作正在进行")?;
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
                    install::install(source, &self.cancelled, |message| {
                        self.report("configuring", Some(message))
                    })
                    .await?
                }
                None => return Err("未找到官方组件，请点击“官方下载”自动下载并配置".into()),
            };
            install::cancellable(runtime::verify_version(&component), &self.cancelled).await?;
            install::check_cancel(&self.cancelled)?;
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
    manager: tauri::State<'_, Launcher>,
    source: String,
) -> Result<ComponentUpdateInfo, String> {
    manager.check_updates(&source).await
}

#[tauri::command]
pub async fn apply_login_hotfixes(
    app: tauri::AppHandle,
    manager: tauri::State<'_, Launcher>,
    source: String,
) -> Result<(), String> {
    let source = crate::update_source::resolve(&source)?;
    manager.apply_hotfixes(&source, &app).await
}

#[tauri::command]
pub async fn open_login_update(
    manager: tauri::State<'_, Launcher>,
    source: String,
) -> Result<(), String> {
    let source = crate::update_source::resolve(&source)?;
    let update = manager.update.lock().unwrap();
    let url = update
        .as_ref()
        .and_then(|update| update.info.release_url.as_deref())
        .ok_or("请重新检查官方新版")?;
    crate::commands::open_url(&crate::updates::route(url, &source)?)
}

pub fn start_monitor(app: tauri::AppHandle) {
    use tauri::Manager;
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
pub fn cancel_game_launch(manager: tauri::State<'_, Launcher>) {
    manager.cancel();
}

#[tauri::command]
pub async fn configure_login_component(
    manager: tauri::State<'_, Launcher>,
    download: bool,
) -> Result<(), String> {
    manager.configure(download).await
}

#[tauri::command]
pub async fn set_login_component(
    manager: tauri::State<'_, Launcher>,
    executable_path: String,
) -> Result<(), String> {
    manager
        .select_component(PathBuf::from(executable_path))
        .await
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
