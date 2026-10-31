use super::{
    component::{LoginRuntime, SUPPORTED_VERSION},
    install,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::Path,
    process::Stdio,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Hotfix {
    pub target_module: String,
    pub target_commit: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    need_hotfix_version: Value,
}

#[derive(Clone, Serialize)]
pub struct ComponentUpdateInfo {
    pub current_version: String,
    pub version: String,
    pub notes: String,
    pub release_url: Option<String>,
    pub(super) hotfixes: Vec<Hotfix>,
}

pub(super) struct CheckedUpdate {
    pub runtime: LoginRuntime,
    pub info: ComponentUpdateInfo,
}

async fn metadata(url: &str, source: &str) -> Result<Value, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .user_agent("IdentityV-Assistant")
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(crate::updates::route(url, source)?)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > 4 * 1024 * 1024 {
            return Err("组件更新信息过大".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

fn candidates(cloud: &Value, config: &Value, source_dir: &Path) -> Vec<Hotfix> {
    cloud["hotfix"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| {
            let item: Hotfix = serde_json::from_value(value.clone()).ok()?;
            let module = &item.target_module;
            if !module.split('.').all(|part| {
                !part.is_empty()
                    && part.bytes().enumerate().all(|(i, b)| {
                        b == b'_' || b.is_ascii_alphabetic() || i > 0 && b.is_ascii_digit()
                    })
            }) {
                return None;
            }
            if !(7..=40).contains(&item.target_commit.len())
                || !item.target_commit.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return None;
            }
            let needed = &item.need_hotfix_version;
            if needed.is_string() && needed != SUPPORTED_VERSION
                || needed.as_array().is_some_and(|versions| {
                    !versions.is_empty() && !versions.iter().any(|v| v == SUPPORTED_VERSION)
                })
            {
                return None;
            }
            let id = format!("{SUPPORTED_VERSION}|{}@{}", module, item.target_commit);
            if config["hotfix_skipped"]
                .as_array()
                .is_some_and(|items| items.iter().any(|v| v == &id))
                || matches!(
                    config["hotfix_records"][&id]["status"].as_str(),
                    Some("applied" | "pending_validate")
                )
            {
                return None;
            }
            if !source_dir
                .join(format!("{}.pyc", module.replace('.', "/")))
                .is_file()
            {
                return None;
            }
            Some(item)
        })
        .take(100)
        .collect()
}

pub(super) async fn check(
    runtime: &LoginRuntime,
    data: &Path,
    source: &str,
    cancelled: &AtomicBool,
) -> Result<ComponentUpdateInfo, String> {
    let (cloud, release) = install::cancellable(
        async {
            tokio::try_join!(
                metadata(
                    "https://raw.githubusercontent.com/KKeygen/idv-login/main/assets/cloudRes.json",
                    source
                ),
                metadata(
                    "https://api.github.com/repos/KKeygen/idv-login/releases/latest",
                    source
                )
            )
        },
        cancelled,
    )
    .await?;
    if !cloud.is_object() || !cloud["version"].is_string() {
        return Err("官方组件更新信息无效".into());
    }
    let config_path = data.join("idv-login/config.json");
    let config: Value = if config_path.exists() {
        serde_json::from_slice(&std::fs::read(config_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
    } else {
        json!({})
    };
    let version = release["tag_name"].as_str().ok_or("官方发布版本无效")?;
    let newer = semver::Version::parse(version.trim_start_matches('v'))
        .map_err(|e| e.to_string())?
        > semver::Version::parse(SUPPORTED_VERSION.trim_start_matches('v')).unwrap();
    let release_url = if newer {
        let prefix = format!("https://github.com/KKeygen/idv-login/releases/download/{version}/");
        release["assets"]
            .as_array()
            .and_then(|assets| {
                assets
                    .iter()
                    .find(|asset| asset["name"] == format!("idv-login-{version}.exe"))
            })
            .and_then(|asset| asset["browser_download_url"].as_str())
            .filter(|url| url.starts_with(&prefix))
            .map(str::to_owned)
    } else {
        None
    };
    if newer && release_url.is_none() {
        return Err("官方新版尚未提供 Windows 安装包，请稍后重试".into());
    }
    Ok(ComponentUpdateInfo {
        current_version: SUPPORTED_VERSION.into(),
        version: version.into(),
        notes: if newer {
            release["body"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(16000)
                .collect()
        } else {
            String::new()
        },
        release_url,
        hotfixes: candidates(
            &cloud,
            &config,
            runtime.script.parent().ok_or("组件目录无效")?,
        ),
    })
}

pub(super) async fn apply(
    checked: &CheckedUpdate,
    data: &Path,
    source: &str,
    closing: &AtomicBool,
    app: &tauri::AppHandle,
) -> Result<(), String> {
    crate::updates::route("https://github.com/", source)?;
    let worker = data.join("assistant-hotfix.py");
    std::fs::write(&worker, include_str!("hotfix_worker.py")).map_err(|e| e.to_string())?;
    let mut child = tokio::process::Command::new(&checked.runtime.python)
        .args(["-I", "-B", "-u"])
        .arg(worker)
        .arg(checked.runtime.script.parent().ok_or("组件目录无效")?)
        .arg(data.join("idv-login"))
        .current_dir(data)
        .env("PROGRAMDATA", data)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| e.to_string())?;
    let request = json!({"current_version":checked.info.current_version, "hotfixes":checked.info.hotfixes, "source":source});
    let mut input = child.stdin.take().ok_or("无法发送更新请求")?;
    input
        .write_all(format!("{request}\n").as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or("无法读取组件更新进度")?;
    let read = async {
        let mut lines = BufReader::new(stdout).lines();
        let mut result = None;
        while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
            if let Some(payload) = line.strip_prefix("IDV_UPDATE_PROGRESS ") {
                if let Some(progress) = parse_progress(payload) {
                    let _ = app.emit("component-update-progress", progress);
                }
            } else if let Some(payload) = line.strip_prefix("IDV_UPDATE_RESULT ") {
                result = serde_json::from_str::<Value>(payload).ok();
            }
        }
        Ok::<_, String>(result)
    };
    let wait = async {
        tokio::try_join!(
            async { child.wait().await.map_err(|e| e.to_string()) },
            read
        )
    };
    tokio::pin!(wait);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    let (status, result) = loop {
        if closing.load(Ordering::Relaxed) || tokio::time::Instant::now() >= deadline {
            // The worker checks cancellation before and after downloads, before
            // replacing a module. Give an in-flight HTTP request time to return.
            let _ = input.write_all(b"cancel\n").await;
            let _ = tokio::time::timeout(Duration::from_secs(30), &mut wait).await;
            return Err("组件更新已取消或超时".into());
        }
        tokio::select! {
            output = &mut wait => break output.map_err(|e| e.to_string())?,
            _ = tokio::time::sleep(Duration::from_millis(100)) => {}
        }
    };
    let result = result.ok_or("组件未返回更新结果")?;
    if !status.success() || result["ok"] != true {
        return Err(result["error"]
            .as_str()
            .unwrap_or("组件热更新失败")
            .to_owned());
    }
    Ok(())
}

fn parse_progress(payload: &str) -> Option<Value> {
    let value: Value = serde_json::from_str(payload).ok()?;
    let phase = value["phase"].as_str()?;
    if !matches!(phase, "downloading" | "applying") {
        return None;
    }
    let downloaded = value["downloaded"].as_u64()?;
    if downloaded > 4 * 1024 * 1024 {
        return None;
    }
    let module = value["module"].as_str()?;
    if module.len() > 256 {
        return None;
    }
    Some(json!({"phase": phase, "downloaded": downloaded,
        "total": value["total"].as_u64().filter(|n| *n > 0 && *n <= 4 * 1024 * 1024 && *n >= downloaded),
        "module": module, "commit": value["commit"].as_str().unwrap_or("")}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progress_rejects_invalid_counts_and_does_not_invent_unknown_totals() {
        let value = json!({"phase":"downloading","module":"fixture","commit":"abc1234","downloaded":2048,"total":null});
        let parsed = parse_progress(&value.to_string()).unwrap();
        assert_eq!(parsed["downloaded"], 2048);
        assert!(parsed["total"].is_null());
        let mut invalid = value.clone();
        invalid["downloaded"] = json!(-1);
        assert!(parse_progress(&invalid.to_string()).is_none());
        invalid = value.clone();
        invalid["phase"] = json!("finished");
        assert!(parse_progress(&invalid.to_string()).is_none());
        invalid = value;
        invalid["total"] = json!(1000);
        assert!(parse_progress(&invalid.to_string()).unwrap()["total"].is_null());
    }
    #[tokio::test]
    #[ignore = "Reads public official update metadata; no component is started or updated"]
    async fn public_update_metadata_uses_official_feed() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("main.pyc");
        std::fs::write(&script, b"fixture").unwrap();
        let runtime = LoginRuntime {
            python: "unused".into(),
            script,
        };
        let source =
            std::env::var("COMPONENT_UPDATE_TEST_SOURCE").unwrap_or_else(|_| "direct".into());
        let info = check(&runtime, dir.path(), &source, &AtomicBool::new(false))
            .await
            .unwrap();
        assert_eq!(info.current_version, SUPPORTED_VERSION);
        assert!(!info.version.is_empty());
        assert!(!dir.path().join("idv-login/config.json").exists());
    }
    #[test]
    fn offers_only_applicable_unapplied_modules_inside_the_runtime() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.pyc"), b"fixture").unwrap();
        let item = json!({"target_module":"main", "target_commit":"abcdef1234567", "need_hotfix_version":[SUPPORTED_VERSION]});
        let cloud = json!({"hotfix":[item, {"target_module":"../escape", "target_commit":"abcdef1234567"}]});
        assert_eq!(candidates(&cloud, &json!({}), dir.path()).len(), 1);
        let id = format!("{SUPPORTED_VERSION}|main@abcdef1234567");
        assert!(candidates(
            &cloud,
            &json!({"hotfix_records":{id:{"status":"pending_validate"}}}),
            dir.path()
        )
        .is_empty());
    }
}
