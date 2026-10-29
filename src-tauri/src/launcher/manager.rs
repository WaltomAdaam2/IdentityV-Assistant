use super::{
    component::ProxySession,
    launch_direct, locate_game,
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
    owned: AsyncMutex<Option<OwnedLogin>>,
    status: Mutex<LauncherStatus>,
    closing: AtomicBool,
}

impl Default for Launcher {
    fn default() -> Self {
        Self {
            owned: AsyncMutex::new(None),
            status: Mutex::new(LauncherStatus {
                phase: "idle",
                component_path: None,
                message: None,
            }),
            closing: AtomicBool::new(false),
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
    fn report(&self, phase: &'static str, message: Option<String>) {
        let mut status = self.status.lock().unwrap();
        status.phase = phase;
        status.message = message;
    }

    pub fn status(&self) -> Result<LauncherStatus, String> {
        // Never block status polling behind a component that is still starting.
        if let Ok(owned) = self.owned.try_lock() {
            if owned
                .as_ref()
                .is_some_and(|owned| !owned.session.is_running())
            {
                self.report(
                    "error",
                    Some("登录组件已退出，请重新启动游戏以恢复代理".into()),
                );
            }
        }
        let mut status = self.status.lock().unwrap().clone();
        status.component_path =
            runtime::component_path(&directory()?)?.map(|path| path.to_string_lossy().into_owned());
        Ok(status)
    }

    pub async fn select_component(&self, python: PathBuf) -> Result<(), String> {
        let owned = self.owned.try_lock().map_err(|_| "启动操作正在进行")?;
        if self.closing.load(Ordering::Relaxed) {
            return Err("助手正在退出或更新".into());
        }
        if owned.is_some() {
            return Err("登录组件已启动，请重启助手后更换组件".into());
        }
        let runtime = runtime::runtime_from_python(python)?;
        runtime::verify_version(&runtime).await?;
        runtime::save_component(&directory()?, &runtime.python)?;
        self.report("idle", None);
        Ok(())
    }

    pub async fn launch(&self, game: Option<String>) -> Result<(), String> {
        let mut owned = self.owned.try_lock().map_err(|_| "启动操作正在进行")?;
        if self.closing.load(Ordering::Relaxed) {
            return Err("助手正在退出或更新".into());
        }
        self.report("starting", None);
        let result = self.launch_locked(game, &mut owned).await;
        match &result {
            Ok(()) => self.report("ready", None),
            Err(error) => self.report("error", Some(error.clone())),
        }
        result
    }

    async fn launch_locked(
        &self,
        game: Option<String>,
        owned: &mut Option<OwnedLogin>,
    ) -> Result<(), String> {
        let game = locate_game(game)?;
        if owned
            .as_ref()
            .is_some_and(|owned| !owned.session.is_running())
        {
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
            runtime::verify_version(&runtime).await?;
            runtime::ensure_can_start()?;
            let data = directory.join("login-data");
            runtime::prepare_data(&data)?;
            let scheme = SchemeSnapshot::capture()?;
            match ProxySession::start_cancellable(
                &runtime,
                &data,
                Duration::from_secs(60),
                &self.closing,
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
            component.session.verify_ready().await?;
            if self.closing.load(Ordering::Relaxed) {
                return Err("启动已取消".into());
            }
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
pub fn get_launcher_status(manager: tauri::State<'_, Launcher>) -> Result<LauncherStatus, String> {
    manager.status()
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
    manager: tauri::State<'_, Launcher>,
    executable_path: Option<String>,
) -> Result<(), String> {
    let game = match executable_path {
        Some(path) => Some(path),
        None => crate::config::load()?["game_window"]["executable_path"]
            .as_str()
            .map(str::to_owned),
    };
    manager.launch(game).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_platform_paths_and_blocks_launch_until_failed_update_is_resumed() {
        let manager = Launcher::default();
        assert!(manager
            .launch(Some("Fever.exe".into()))
            .await
            .unwrap_err()
            .contains("dwrg.exe"));
        manager.shutdown().await.unwrap();
        assert!(manager
            .launch(Some("Fever.exe".into()))
            .await
            .unwrap_err()
            .contains("退出或更新"));
        manager.resume();
        assert!(manager
            .launch(Some("Fever.exe".into()))
            .await
            .unwrap_err()
            .contains("dwrg.exe"));
        assert!(manager
            .select_component(PathBuf::from("installer.exe"))
            .await
            .is_err());
    }
}
