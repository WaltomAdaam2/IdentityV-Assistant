use super::install;
use serde::Serialize;
use serde_json::Value;
use std::{sync::atomic::AtomicBool, time::Duration};

#[derive(Clone, Serialize)]
pub struct ComponentUpdateInfo {
    pub current_version: String,
    pub version: String,
    pub notes: String,
    pub available: bool,
}
#[derive(Clone)]
pub(super) struct ComponentRelease {
    pub version: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    pub notes: String,
}
pub(super) struct CheckedUpdate {
    pub runtime: super::component::LoginRuntime,
    pub info: ComponentUpdateInfo,
    pub release: ComponentRelease,
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
    serde_json::from_slice(&bytes).map_err(|_| "组件更新源未返回有效 JSON，请切换更新源后重试 / Update source returned invalid JSON; change source and retry".into())
}

fn release_asset(release: &Value) -> Result<ComponentRelease, String> {
    let version = release["tag_name"].as_str().ok_or("官方发布版本无效")?;
    super::runtime::parse_version(version)?;
    if release["draft"] == true {
        return Err("官方组件尚未发布".into());
    }
    let name = format!("idv-login-{version}.exe");
    let url = format!("https://github.com/KKeygen/idv-login/releases/download/{version}/{name}");
    let assets = release["assets"].as_array().ok_or("官方组件缺少下载附件")?;
    let asset = assets
        .iter()
        .find(|asset| asset["name"] == name)
        .ok_or("官方组件缺少 Windows 安装包")?;
    if asset["browser_download_url"] != url {
        return Err("组件下载地址与官方项目或版本不符".into());
    }
    let size = asset["size"]
        .as_u64()
        .filter(|size| *size > 0 && *size <= 2 * 1024 * 1024 * 1024)
        .ok_or("官方组件附件大小无效")?;
    let sha256 = asset["digest"]
        .as_str()
        .and_then(|digest| digest.strip_prefix("sha256:"))
        .filter(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or("官方组件缺少 SHA-256 校验信息，请稍后重试")?
        .to_ascii_lowercase();
    Ok(ComponentRelease {
        version: version.into(),
        url,
        size,
        sha256,
        notes: release["body"]
            .as_str()
            .unwrap_or("")
            .chars()
            .take(16000)
            .collect(),
    })
}

fn describe(current: &str, release: &ComponentRelease) -> Result<ComponentUpdateInfo, String> {
    Ok(ComponentUpdateInfo {
        current_version: current.into(),
        version: release.version.clone(),
        available: super::runtime::parse_version(current)?
            < super::runtime::parse_version(&release.version)?,
        notes: release.notes.clone(),
    })
}
pub(super) async fn latest(
    source: &str,
    cancelled: &AtomicBool,
) -> Result<ComponentRelease, String> {
    // Module hotfix metadata and account configuration are deliberately never read.
    let release = install::cancellable(
        metadata(
            "https://api.github.com/repos/KKeygen/idv-login/releases/latest",
            source,
        ),
        cancelled,
    )
    .await?;
    release_asset(&release)
}
pub(super) async fn check(
    current: &str,
    source: &str,
    cancelled: &AtomicBool,
) -> Result<(ComponentUpdateInfo, ComponentRelease), String> {
    let release = latest(source, cancelled).await?;
    Ok((describe(current, &release)?, release))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn official_new_versions_are_available_without_a_compatibility_allowlist() {
        let fixture = json!({"tag_name":"v6.3.3-beta","body":"release notes","assets":[{
            "name":"idv-login-v6.3.3-beta.exe","size":62819224,
            "browser_download_url":"https://github.com/KKeygen/idv-login/releases/download/v6.3.3-beta/idv-login-v6.3.3-beta.exe",
            "digest":"sha256:e37f3dc788170252671cd9855ffb0af22940a04d4bab7fe86a507c7cfcba65f8"}]});
        let release = release_asset(&fixture).unwrap();
        assert!(describe("v6.3.2-stable", &release).unwrap().available);
        assert!(!describe("v6.3.3-beta", &release).unwrap().available);
        assert!(!describe("v7.0.0", &release).unwrap().available);
        assert_eq!(release.version, "v6.3.3-beta");
        for (field, value) in [
            ("digest", json!(null)),
            ("size", json!(0)),
            (
                "browser_download_url",
                json!("https://example.com/untrusted.exe"),
            ),
        ] {
            let mut invalid = fixture.clone();
            invalid["assets"][0][field] = value;
            assert!(release_asset(&invalid).is_err());
        }
        assert!(release_asset(&json!({"message":"rate limited"})).is_err());
    }
}
