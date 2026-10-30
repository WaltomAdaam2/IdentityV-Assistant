use super::{component::LoginRuntime, runtime};
use sha2::{Digest, Sha256};
use std::{
    fs,
    future::Future,
    io::{Read, Write},
    path::Path,
    process::Stdio,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

const RELEASE: &str = "https://github.com/KKeygen/idv-login/releases/download/v6.3.2-stable/idv-login-v6.3.2-stable.exe";
const UNPACKER: &str = "https://github.com/jrathlev/InnoUnpacker-Windows-GUI/releases/download/ui_2_2_13/innounp-2.zip";

#[cfg(test)]
fn verify_download(bytes: &[u8], size: u64, sha256: &str) -> Result<(), String> {
    verify_digest(
        bytes.len() as u64,
        &format!("{:x}", Sha256::digest(bytes)),
        size,
        sha256,
    )
}

fn verify_digest(actual_size: u64, actual_hash: &str, size: u64, hash: &str) -> Result<(), String> {
    if actual_size != size || actual_hash != hash {
        return Err("组件下载校验失败，未执行或配置下载内容，请重试".into());
    }
    Ok(())
}

pub(super) fn check_cancel(cancelled: &AtomicBool) -> Result<(), String> {
    if cancelled.load(Ordering::Relaxed) {
        Err("操作已取消".into())
    } else {
        Ok(())
    }
}

pub(super) async fn cancellable<T>(
    future: impl Future<Output = Result<T, String>>,
    cancelled: &AtomicBool,
) -> Result<T, String> {
    tokio::pin!(future);
    loop {
        check_cancel(cancelled)?;
        tokio::select! {
            result = &mut future => return result,
            _ = tokio::time::sleep(Duration::from_millis(100)) => {}
        }
    }
}

async fn download(
    url: &str,
    source: &str,
    target: &Path,
    size: u64,
    hash: &str,
    cancelled: &AtomicBool,
    progress: &impl Fn(String),
) -> Result<(), String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .user_agent("IdentityV-Assistant")
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;
    let url = crate::updates::route(url, source)?;
    let mut response = cancellable(
        async {
            client
                .get(url)
                .send()
                .await
                .and_then(reqwest::Response::error_for_status)
                .map_err(|e| e.to_string())
        },
        cancelled,
    )
    .await?;
    let mut file = fs::File::create(target).map_err(|e| e.to_string())?;
    let mut count = 0;
    let mut digest = Sha256::new();
    let mut percent = u64::MAX;
    while let Some(chunk) = cancellable(
        async { response.chunk().await.map_err(|e| e.to_string()) },
        cancelled,
    )
    .await?
    {
        count += chunk.len() as u64;
        if count > size {
            return Err("组件下载大小超过官方发布值".into());
        }
        digest.update(&chunk);
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        if count * 100 / size != percent {
            percent = count * 100 / size;
            progress(format!("正在下载并校验组件：{percent}%"));
        }
    }
    verify_digest(count, &format!("{:x}", digest.finalize()), size, hash)?;
    file.sync_all().map_err(|e| e.to_string())
}

pub(super) async fn install(
    source: &str,
    cancelled: &AtomicBool,
    progress: impl Fn(String),
) -> Result<LoginRuntime, String> {
    let destination = runtime::managed_root()?;
    if destination.exists() {
        let runtime = runtime::runtime_from_selection(destination)?;
        runtime::verify_version(&runtime).await?;
        return Ok(runtime);
    }
    let parent = destination.parent().ok_or("组件安装目录无效")?;
    fs::create_dir_all(parent).map_err(|e| format!("无法写入助手程序目录：{e}"))?;
    let staging = tempfile::Builder::new()
        .prefix(".download-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let installer = staging.path().join("release.exe");
    download(
        RELEASE,
        source,
        &installer,
        204067592,
        "87fb99ae511dc5870a8f2d8ca6b4ac3983636521fe93db901512d054068d9083",
        cancelled,
        &progress,
    )
    .await?;
    let unpacker_zip = staging.path().join("unpacker.zip");
    download(
        UNPACKER,
        source,
        &unpacker_zip,
        618221,
        "10fce27b6344b7674d5625cc34afca1c88c12183d957c6d93e401097fa2556cd",
        cancelled,
        &progress,
    )
    .await?;
    let mut archive =
        zip::ZipArchive::new(fs::File::open(unpacker_zip).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let mut entry = archive.by_name("innounp.exe").map_err(|e| e.to_string())?;
    if entry.size() > 4 * 1024 * 1024 {
        return Err("解包工具大小无效".into());
    }
    let unpacker = staging.path().join("innounp.exe");
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    fs::write(&unpacker, bytes).map_err(|e| e.to_string())?;
    check_cancel(cancelled)?;
    progress("下载校验通过，正在解包官方组件…".into());
    let extracted = staging.path().join("extracted");
    let mut command = tokio::process::Command::new(unpacker);
    command
        .args(["-x", "-b", "-q"])
        .arg(format!("-d{}", extracted.display()))
        .arg(installer)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let result = cancellable(
        async {
            tokio::time::timeout(Duration::from_secs(180), child.wait())
                .await
                .map_err(|_| "组件解包超时".to_string())?
                .map_err(|e| e.to_string())
        },
        cancelled,
    )
    .await;
    let status = match result {
        Ok(status) => status,
        Err(error) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(error);
        }
    };
    if !status.success() {
        return Err(format!("官方组件解包失败：{status}"));
    }
    let root = extracted.join("{app}");
    let runtime = runtime::runtime_from_selection(root.clone())?;
    progress("正在校验组件版本…".into());
    cancellable(runtime::verify_version(&runtime), cancelled).await?;
    check_cancel(cancelled)?;
    fs::rename(root, &destination).map_err(|e| format!("无法保存官方组件：{e}"))?;
    runtime::runtime_from_selection(destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downloaded_component_must_match_both_published_size_and_digest() {
        let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(verify_download(b"abc", 3, hash).is_ok());
        assert!(verify_download(b"abd", 3, hash).is_err());
        assert!(verify_download(b"ab", 3, hash).is_err());
    }
}
