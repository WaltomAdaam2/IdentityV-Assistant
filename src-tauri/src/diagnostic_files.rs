//! Only application-generated diagnostic files; never walk arbitrary data trees.
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn is_link(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

pub(crate) fn root() -> Result<PathBuf, String> {
    Ok(crate::config::user_path()?
        .parent()
        .ok_or("Missing data root")?
        .join("debug"))
}

pub(crate) fn ensure_directory(path: &Path) -> Result<(), String> {
    // Check ancestors too: a junction at debug must not redirect writes/deletes.
    if path.ancestors().any(is_link) {
        return Err("Diagnostic directory cannot be a link".into());
    }
    fs::create_dir_all(path).map_err(|e| e.to_string())
}

pub(crate) fn image_kind(kind: &str) -> bool {
    if matches!(kind, "full" | "region") {
        return true;
    }
    kind.split_once('_').is_some_and(|(prefix, index)| {
        matches!(prefix, "region" | "edges" | "mask" | "template")
            && !index.is_empty()
            && index.len() <= 6
            && index.bytes().all(|c| c.is_ascii_digit())
    })
}

pub(crate) fn capture_time(path: &Path) -> Option<DateTime<Utc>> {
    if path.extension()?.to_str()? != "png" {
        return None;
    }
    let stem = path
        .file_stem()?
        .to_str()?
        .strip_prefix("failure-")
        .unwrap_or(path.file_stem()?.to_str()?);
    let (stamp, id) = stem.split_once('-')?;
    if stamp.len() != 22 || id.len() != 8 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    NaiveDateTime::parse_from_str(stamp, "%Y%m%dT%H%M%S%6fZ")
        .ok()
        .map(|t| t.and_utc())
}

pub(crate) fn attempts(root: &Path) -> Result<Vec<PathBuf>, String> {
    let directory = root.join("recognition");
    if !directory.exists() {
        return Ok(Vec::new());
    }
    if directory.ancestors().any(is_link) {
        return Err("Diagnostic directory cannot be a link".into());
    }
    Ok(fs::read_dir(directory)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && !is_link(p)
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| uuid::Uuid::parse_str(s).is_ok())
        })
        .collect())
}

pub(crate) fn read_report(directory: &Path) -> Result<serde_json::Value, String> {
    use std::io::Read;
    let path = directory.join("flow.json");
    if path.ancestors().any(is_link) {
        return Err("Diagnostic file cannot be a link".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Diagnostic report exceeds limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub(crate) fn prune(root: &Path, now: DateTime<Utc>) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    if root.ancestors().any(is_link) {
        return Err("Diagnostic directory cannot be a link".into());
    }
    let images_before = now - Duration::days(3);
    let mut failure = None;
    for name in ["failures", "captures"] {
        let directory = root.join(name);
        if !directory.is_dir() || is_link(&directory) {
            continue;
        }
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) => {
                failure = Some(error.to_string());
                continue;
            }
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file()
                && !is_link(&path)
                && capture_time(&path).is_some_and(|t| t < images_before)
            {
                if let Err(error) = fs::remove_file(path) {
                    failure = Some(error.to_string());
                }
            }
        }
    }
    for directory in attempts(root)? {
        let Ok(report) = read_report(&directory) else {
            continue;
        };
        let Some(started) = report["started"]
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        else {
            continue;
        };
        if started >= images_before {
            continue;
        }
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                failure = Some(error.to_string());
                continue;
            }
        };
        let mut pending_image = false;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let owned_image = path.extension().and_then(|s| s.to_str()) == Some("png")
                && path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(image_kind);
            if path.is_file() && !is_link(&path) && owned_image {
                if let Err(error) = fs::remove_file(path) {
                    pending_image = true;
                    failure = Some(error.to_string());
                }
            }
        }
        // Keep the timestamp report while a locked image still needs a later retry.
        if started < now - Duration::days(7) && !pending_image {
            if let Err(error) = fs::remove_file(directory.join("flow.json")) {
                failure = Some(error.to_string());
            }
        }
        if fs::read_dir(&directory)
            .map_err(|e| e.to_string())?
            .next()
            .is_none()
        {
            if let Err(error) = fs::remove_dir(directory) {
                failure = Some(error.to_string());
            }
        }
    }
    failure.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn locked_images_keep_their_report_for_a_later_cleanup_retry() {
        use std::os::windows::fs::OpenOptionsExt;
        let root = tempfile::tempdir().unwrap();
        let directory = root
            .path()
            .join("recognition")
            .join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("flow.json"),
            r#"{"started":"2026-10-01T00:00:00Z"}"#,
        )
        .unwrap();
        fs::write(directory.join("full.png"), "fixture").unwrap();
        let locked = fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(directory.join("full.png"))
            .unwrap();
        let now = DateTime::parse_from_rfc3339("2026-11-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(prune(root.path(), now).is_err());
        assert!(directory.join("flow.json").exists());
        drop(locked);
        prune(root.path(), now).unwrap();
        assert!(!directory.exists());
    }
    #[test]
    fn retention_only_removes_owned_images_after_three_days_and_reports_after_seven() {
        let dir = tempfile::tempdir().unwrap();
        let now = DateTime::parse_from_rfc3339("2026-11-01T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let captures = dir.path().join("captures");
        fs::create_dir(&captures).unwrap();
        for name in [
            "20261029T115959000000Z-aabb0011.png",
            "20261029T120000000000Z-aabb0011.png",
            "manual.png",
        ] {
            fs::write(captures.join(name), "fixture").unwrap();
        }
        let attempt = dir
            .path()
            .join("recognition")
            .join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&attempt).unwrap();
        fs::write(
            attempt.join("flow.json"),
            r#"{"started":"2026-10-28T12:00:00Z"}"#,
        )
        .unwrap();
        fs::write(attempt.join("full.png"), "fixture").unwrap();
        fs::write(attempt.join("manual.png"), "keep").unwrap();
        prune(dir.path(), now).unwrap();
        assert!(!captures
            .join("20261029T115959000000Z-aabb0011.png")
            .exists());
        assert!(captures
            .join("20261029T120000000000Z-aabb0011.png")
            .exists());
        assert!(captures.join("manual.png").exists());
        assert!(!attempt.join("full.png").exists());
        assert!(attempt.join("flow.json").exists());
        prune(dir.path(), now + Duration::days(4)).unwrap();
        assert!(!attempt.join("flow.json").exists());
        assert!(attempt.join("manual.png").exists());
    }
}
