use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::{oneshot, Mutex as AsyncMutex};

const RELEASES: &str = "https://github.com/WaltomAdaam2/IdentityV-Assistant/releases/";
const PROXY: &str = "https://gh-proxy.com/";
const MAX_INSTALLER_SIZE: u64 = 512 * 1024 * 1024;

#[derive(Clone, Serialize)]
pub struct UpdateInfo {
    version: String,
    current_version: String,
    notes: String,
    date: Option<String>,
    sha256: String,
    size: u64,
}

#[derive(Clone, Serialize)]
pub struct UpdateStatus {
    phase: String,
    info: Option<UpdateInfo>,
    downloaded: u64,
    total: Option<u64>,
    error: Option<String>,
    source: String,
}

impl Default for UpdateStatus {
    fn default() -> Self {
        Self {
            phase: "idle".into(),
            info: None,
            downloaded: 0,
            total: None,
            error: None,
            source: "direct".into(),
        }
    }
}

#[derive(Default)]
struct Session {
    status: UpdateStatus,
    update: Option<Update>,
    bytes: Option<Vec<u8>>,
    cancel: Option<oneshot::Sender<()>>,
}

#[derive(Default)]
pub struct UpdateManager {
    session: Mutex<Session>,
    operation: AsyncMutex<()>,
}

impl UpdateManager {
    fn report(&self, app: &AppHandle, phase: &str, error: Option<String>) {
        let status = {
            let mut session = self.session.lock().unwrap();
            session.status.phase = phase.into();
            session.status.error = error;
            session.status.clone()
        };
        let _ = app.emit("update-status", status);
    }
}

fn route(url: &str, source: &str) -> Result<String, String> {
    match source {
        "direct" => Ok(url.into()),
        "proxy" => Ok(format!("{PROXY}{url}")),
        _ => Err("Unknown update source".into()),
    }
}

fn validate_asset(url: &str, version: &str) -> Result<(), String> {
    if url != format!("{RELEASES}download/v{version}/IdentityV-Assistant_{version}_x64-setup.exe") {
        return Err("Installer URL must match this project's release and version".into());
    }
    Ok(())
}

fn expected_integrity(raw: &serde_json::Value) -> Result<(String, u64), String> {
    let platform = &raw["platforms"]["windows-x86_64"];
    let hash = platform["sha256"]
        .as_str()
        .ok_or("Missing installer SHA-256")?;
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid installer SHA-256".into());
    }
    let size = platform["size"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= MAX_INSTALLER_SIZE)
        .ok_or("Invalid installer size")?;
    Ok((hash.to_ascii_lowercase(), size))
}

fn verify_hash(bytes: &[u8], info: &UpdateInfo) -> Result<(), String> {
    if bytes.len() as u64 != info.size {
        return Err("Installer size does not match the release manifest".into());
    }
    if format!("{:x}", Sha256::digest(bytes)) != info.sha256 {
        return Err("SHA-256 verification failed; installation blocked".into());
    }
    Ok(())
}

#[tauri::command]
pub fn get_update_status(manager: State<'_, UpdateManager>) -> UpdateStatus {
    manager.session.lock().unwrap().status.clone()
}

#[tauri::command]
pub async fn check_update(
    app: AppHandle,
    manager: State<'_, UpdateManager>,
    source: String,
) -> Result<Option<UpdateInfo>, String> {
    let _operation = manager
        .operation
        .try_lock()
        .map_err(|_| "An update operation is in progress")?;
    let endpoint = route(&format!("{RELEASES}latest/download/latest.json"), &source)?;
    {
        let mut session = manager.session.lock().unwrap();
        *session = Session::default();
        session.status.source = source;
    }
    manager.report(&app, "checking", None);
    let result = async {
        let updater = app
            .updater_builder()
            .target("windows-x86_64")
            .endpoints(vec![endpoint
                .parse()
                .map_err(|e| format!("Invalid endpoint: {e}"))?])
            .map_err(|e| e.to_string())?
            .timeout(Duration::from_secs(25))
            .build()
            .map_err(|e| e.to_string())?;
        let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        validate_asset(update.download_url.as_str(), &update.version)?;
        let (sha256, size) = expected_integrity(&update.raw_json)?;
        let info = UpdateInfo {
            version: update.version.clone(),
            current_version: update.current_version.clone(),
            notes: update.body.clone().unwrap_or_default(),
            date: update.date.map(|d| d.to_string()),
            sha256,
            size,
        };
        let mut session = manager.session.lock().unwrap();
        session.status.info = Some(info.clone());
        session.status.total = Some(size);
        session.update = Some(update);
        Ok::<_, String>(Some(info))
    }
    .await;
    match &result {
        Ok(Some(_)) => manager.report(&app, "available", None),
        Ok(None) => manager.report(&app, "current", None),
        Err(error) => manager.report(&app, "error", Some(error.clone())),
    }
    result
}

#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    manager: State<'_, UpdateManager>,
    source: String,
) -> Result<(), String> {
    let _operation = manager
        .operation
        .try_lock()
        .map_err(|_| "An update operation is in progress")?;
    let (mut update, info, mut cancelled) = {
        let mut session = manager.session.lock().unwrap();
        let mut update = session.update.clone().ok_or("Check for updates first")?;
        let info = session
            .status
            .info
            .clone()
            .ok_or("Missing update information")?;
        update.download_url = route(update.download_url.as_str(), &source)?
            .parse()
            .map_err(|e| format!("Invalid URL: {e}"))?;
        let (tx, rx) = oneshot::channel();
        session.cancel = Some(tx);
        session.bytes = None;
        session.status.source = source;
        session.status.downloaded = 0;
        (update, info, rx)
    };
    update.timeout = Some(Duration::from_secs(600));
    manager.report(&app, "downloading", None);
    let mut last_report = Instant::now();
    let download = update.download(
        |length, _| {
            let mut session = manager.session.lock().unwrap();
            session.status.downloaded += length as u64;
            if session.status.downloaded > info.size {
                if let Some(cancel) = session.cancel.take() {
                    let _ = cancel.send(());
                }
            }
            if last_report.elapsed() >= Duration::from_millis(100) {
                let _ = app.emit("update-status", session.status.clone());
                last_report = Instant::now();
            }
        },
        || manager.report(&app, "verifying", None),
    );
    let result = tokio::select! {
        result = download => result.map_err(|e| e.to_string()),
        _ = &mut cancelled => Err("Download cancelled".to_owned()),
    };
    manager.session.lock().unwrap().cancel = None;
    let result = if manager.session.lock().unwrap().status.downloaded > info.size {
        Err("Installer exceeds the declared release size; installation blocked".into())
    } else {
        result.and_then(|bytes| {
            verify_hash(&bytes, &info)?;
            Ok(bytes)
        })
    };
    match result {
        Ok(bytes) => {
            let mut session = manager.session.lock().unwrap();
            session.status.downloaded = bytes.len() as u64;
            session.bytes = Some(bytes);
            drop(session);
            manager.report(&app, "ready", None);
            Ok(())
        }
        Err(error) => {
            let phase = if error == "Download cancelled" {
                "cancelled"
            } else {
                "error"
            };
            manager.report(&app, phase, Some(error.clone()));
            if phase == "cancelled" {
                Ok(())
            } else {
                Err(error)
            }
        }
    }
}

#[tauri::command]
pub fn cancel_update(manager: State<'_, UpdateManager>) {
    if let Some(cancel) = manager.session.lock().unwrap().cancel.take() {
        let _ = cancel.send(());
    }
}

#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    manager: State<'_, UpdateManager>,
) -> Result<(), String> {
    let _operation = manager
        .operation
        .try_lock()
        .map_err(|_| "An update operation is in progress")?;
    let (update, bytes, info) = {
        let mut session = manager.session.lock().unwrap();
        let update = session.update.clone().ok_or("Check for updates first")?;
        let info = session
            .status
            .info
            .clone()
            .ok_or("Missing update information")?;
        let bytes = session
            .bytes
            .take()
            .ok_or("Download and verify the update first")?;
        (update, bytes, info)
    };
    // Keep verified bytes in memory: no mutable executable path can be swapped before install.
    verify_hash(&bytes, &info)?;
    manager.report(&app, "installing", None);
    let supervisor = app.state::<crate::engine::EngineSupervisor>();
    let result = async {
        supervisor.prepare_update().await?;
        app.state::<crate::power::PowerManager>()
            .apply(false, false)?;
        tauri::async_runtime::spawn_blocking(move || {
            update.install(bytes).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }
    .await;
    if let Err(error) = &result {
        supervisor.finish_update();
        if let Ok(settings) = crate::config::load() {
            let _ = app.state::<crate::power::PowerManager>().apply(
                settings["app"]["prevent_system_sleep"]
                    .as_bool()
                    .unwrap_or(false),
                settings["app"]["prevent_display_sleep"]
                    .as_bool()
                    .unwrap_or(false),
            );
        }
        manager.report(&app, "error", Some(error.clone()));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_app() -> tauri::App<tauri::test::MockRuntime> {
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        context
            .config_mut()
            .plugins
            .0
            .insert("updater".into(), config["plugins"]["updater"].clone());
        tauri::test::mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .build(context)
            .unwrap()
    }

    #[test]
    fn official_verifier_rejects_modified_payload_signature_and_announced_version() {
        let app = test_app();
        let updater = app
            .updater_builder()
            .target("windows-x86_64")
            .version_comparator(|_, _| true)
            .build()
            .unwrap();
        let payload = include_bytes!("../tests/fixtures/update-payload.txt");
        let raw = json!({"version":"0.1.1", "platforms":{"windows-x86_64":{
            "url":format!("{RELEASES}download/v0.1.1/IdentityV-Assistant_0.1.1_x64-setup.exe"),
            "signature":include_str!("../tests/fixtures/update-payload.txt.sig").trim()}}});
        let update = updater.restore_update(raw.clone()).unwrap().unwrap();
        update.verify(payload).unwrap();
        assert!(update.verify(b"modified installer").is_err());
        let mut altered_version = raw.clone();
        altered_version["version"] = json!("99.0.0");
        assert!(updater
            .restore_update(altered_version)
            .unwrap()
            .unwrap()
            .verify(payload)
            .is_err());
        let mut altered_signature = raw;
        altered_signature["platforms"]["windows-x86_64"]["signature"] = json!("invalid");
        assert!(updater
            .restore_update(altered_signature)
            .unwrap()
            .unwrap()
            .verify(payload)
            .is_err());
    }

    #[test]
    #[ignore = "Downloads the published installer; set UPDATE_TEST_SOURCE=direct or proxy"]
    fn published_update_download_and_integrity() {
        let source = std::env::var("UPDATE_TEST_SOURCE").unwrap();
        let app = test_app();
        tauri::async_runtime::block_on(async {
            let updater = app
                .updater_builder()
                .target("windows-x86_64")
                .version_comparator(|_, _| true)
                .endpoints(vec![route(
                    &format!("{RELEASES}latest/download/latest.json"),
                    &source,
                )
                .unwrap()
                .parse()
                .unwrap()])
                .unwrap()
                .timeout(Duration::from_secs(600))
                .build()
                .unwrap();
            let mut update = updater.check().await.unwrap().unwrap();
            update.timeout = Some(Duration::from_secs(600));
            validate_asset(update.download_url.as_str(), &update.version).unwrap();
            let (sha256, size) = expected_integrity(&update.raw_json).unwrap();
            update.download_url = route(update.download_url.as_str(), &source)
                .unwrap()
                .parse()
                .unwrap();
            let bytes = update.download(|_, _| {}, || {}).await.unwrap();
            verify_hash(
                &bytes,
                &UpdateInfo {
                    version: update.version,
                    current_version: update.current_version,
                    notes: String::new(),
                    date: None,
                    sha256,
                    size,
                },
            )
            .unwrap();
        });
    }

    #[test]
    fn accepts_only_project_installers_and_known_routes() {
        let url = format!("{RELEASES}download/v0.2.0/IdentityV-Assistant_0.2.0_x64-setup.exe");
        assert!(validate_asset(&url, "0.2.0").is_ok());
        for bad in [
            url.replace("github.com/", "github.com.evil/"),
            url.replace("IdentityV-Assistant/", "other/"),
            format!("{url}?redirect=other"),
            url.replace(".exe", ".zip"),
        ] {
            assert!(validate_asset(&bad, "0.2.0").is_err());
        }
        assert!(validate_asset(&url, "0.3.0").is_err());
        assert_eq!(route(&url, "direct").unwrap(), url);
        assert_eq!(route(&url, "proxy").unwrap(), format!("{PROXY}{url}"));
        assert!(route(&url, "other").is_err());
    }

    #[test]
    fn rejects_missing_malformed_oversize_or_modified_downloads() {
        let hash = format!("{:x}", Sha256::digest(b"installer"));
        let raw = json!({"platforms":{"windows-x86_64":{"sha256":hash, "size":9}}});
        let (sha256, size) = expected_integrity(&raw).unwrap();
        let info = UpdateInfo {
            version: "1.0.0".into(),
            current_version: "0.1.0".into(),
            notes: "".into(),
            date: None,
            sha256,
            size,
        };
        assert!(verify_hash(b"installer", &info).is_ok());
        assert!(verify_hash(b"Installer", &info).is_err());
        assert!(verify_hash(b"short", &info).is_err());
        assert!(expected_integrity(&json!({})).is_err());
        let mut raw = raw;
        raw["platforms"]["windows-x86_64"]["size"] = json!(MAX_INSTALLER_SIZE + 1);
        assert!(expected_integrity(&raw).is_err());
    }
}
