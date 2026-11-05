use super::{component::SUPPORTED_VERSION, install};
use serde::Serialize;
use serde_json::Value;
use std::{sync::atomic::AtomicBool, time::Duration};

#[derive(Clone, Serialize)]
pub struct ComponentUpdateInfo {
    pub current_version: String,
    pub version: String,
    pub notes: String,
    pub available: bool,
    pub unsupported_version: Option<String>,
}
pub(super) struct CheckedUpdate {
    pub runtime: super::component::LoginRuntime,
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
    serde_json::from_slice(&bytes).map_err(|_| "组件更新源未返回有效 JSON，请切换更新源后重试 / Update source returned invalid JSON; change source and retry".into())
}

fn describe(current: &str, release: &Value) -> Result<ComponentUpdateInfo, String> {
    let latest = release["tag_name"].as_str().ok_or("官方发布版本无效")?;
    let version = |value: &str| {
        semver::Version::parse(value.trim_start_matches('v')).map_err(|e| e.to_string())
    };
    let supported = version(SUPPORTED_VERSION)?;
    let available = version(current)? < supported;
    Ok(ComponentUpdateInfo {
        current_version: current.into(),
        version: SUPPORTED_VERSION.into(),
        available,
        unsupported_version: (version(latest)? > supported).then(|| latest.to_string()),
        notes: if latest == SUPPORTED_VERSION {
            release["body"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(16000)
                .collect()
        } else {
            String::new()
        },
    })
}
pub(super) async fn check(
    current: &str,
    source: &str,
    cancelled: &AtomicBool,
) -> Result<ComponentUpdateInfo, String> {
    // Module hotfix metadata and account configuration are deliberately never read.
    let release = install::cancellable(
        metadata(
            "https://api.github.com/repos/KKeygen/idv-login/releases/latest",
            source,
        ),
        cancelled,
    )
    .await?;
    describe(current, &release)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn only_pinned_compatible_release_can_be_installed() {
        let future = json!({"tag_name":"v99.0.0","body":"future"});
        let current = describe(SUPPORTED_VERSION, &future).unwrap();
        assert!(!current.available);
        assert_eq!(current.unsupported_version.as_deref(), Some("v99.0.0"));
        let older = describe("v6.2.0", &future).unwrap();
        assert!(older.available);
        assert_eq!(older.version, SUPPORTED_VERSION);
        assert!(describe(SUPPORTED_VERSION, &json!({"message":"rate limited"})).is_err());
    }
}
