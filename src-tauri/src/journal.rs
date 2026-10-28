//! App-local journal packs. No paths from another assistant are consulted.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    sync::Mutex,
};

static PACK_WRITE: Mutex<()> = Mutex::new(());
const MAX_IMAGE: u64 = 20 * 1024 * 1024;
const MAX_PACK: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalMap {
    pub index: i64,
    pub category: String,
    #[serde(default)]
    pub name: String,
    pub floors: BTreeMap<String, String>,
    #[serde(default)]
    pub display: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalPack {
    #[serde(alias = "packId")]
    pub id: String,
    #[serde(alias = "packName")]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(alias = "seasonId")]
    pub season_id: String,
    #[serde(alias = "seasonName", default)]
    pub season_name: String,
    pub difficulty: String,
    pub maps: Vec<JournalMap>,
}
#[derive(Debug, Serialize)]
pub struct JournalLibrary {
    pub directory: String,
    pub packs: Vec<JournalPack>,
    pub errors: Vec<String>,
}

fn packs_root() -> Result<PathBuf, String> {
    let journal = crate::config::user_path()?
        .parent()
        .ok_or("无法定位应用数据目录")?
        .join("journal");
    fs::create_dir_all(&journal).map_err(io)?;
    reject_link(&journal)?;
    Ok(journal.join("packs"))
}
fn io(error: std::io::Error) -> String {
    error.to_string()
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 100
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
fn reject_link(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(io)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("不支持链接或重解析点".into());
        }
    }
    if metadata.file_type().is_symlink() {
        return Err("不支持符号链接".into());
    }
    Ok(())
}
fn inside(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.is_empty() || relative.contains(':') || relative.contains('\\') {
        return Err("地图路径必须是包内相对路径，使用 / 分隔".into());
    }
    let path = Path::new(relative);
    if path
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("地图路径不能越出地图包".into());
    }
    reject_link(root)?;
    let root = root.canonicalize().map_err(io)?;
    let mut target = root.clone();
    for component in path.components() {
        target.push(component);
        reject_link(&target)?;
    }
    let target = target.canonicalize().map_err(io)?;
    if !target.starts_with(root) {
        return Err("地图路径不能越出地图包".into());
    }
    Ok(target)
}
fn mime(path: &Path) -> Result<&'static str, String> {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Ok("image/png"),
        "jpg" | "jpeg" => Ok("image/jpeg"),
        "webp" => Ok("image/webp"),
        _ => Err("仅支持 PNG、JPEG 和 WebP 地图".into()),
    }
}
fn image_file(root: &Path, path: &str) -> Result<(PathBuf, u64), String> {
    let resolved = inside(root, path)?;
    mime(&resolved)?;
    let metadata = fs::metadata(&resolved).map_err(io)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_IMAGE {
        return Err(format!("地图图片必须是 20 MiB 以内的非空文件：{path}"));
    }
    Ok((resolved, metadata.len()))
}
fn load_pack(root: &Path) -> Result<JournalPack, String> {
    let descriptor =
        inside(root, "pack.json").map_err(|error| format!("需要 pack.json 格式地图包：{error}"))?;
    if fs::metadata(&descriptor).map_err(io)?.len() > 2 * 1024 * 1024 {
        return Err("地图描述文件超过 2 MiB".into());
    }
    let mut pack: JournalPack = serde_json::from_slice(&fs::read(descriptor).map_err(io)?)
        .map_err(|e| format!("地图描述格式错误：{e}"))?;
    if !valid_id(&pack.id) {
        return Err("packId 只能包含英文字母、数字、下划线和短横线".into());
    }
    if !["hard", "nightmare"].contains(&pack.difficulty.as_str()) {
        return Err("地图难度必须是 hard 或 nightmare".into());
    }
    if pack.season_id.is_empty() || pack.season_id == "doom" {
        pack.season_id = "s0".into();
    }
    if pack.maps.is_empty() || pack.maps.len() > 1000 {
        return Err("每个地图包需包含 1–1000 张地图".into());
    }
    let mut keys = BTreeSet::new();
    let mut files = BTreeSet::new();
    let mut total = 0;
    for map in &mut pack.maps {
        if map.index < 1
            || !["lf", "rf", "lr", "triple", "td"].contains(&map.category.as_str())
            || !keys.insert((map.category.clone(), map.index))
        {
            return Err("地图编号或门型无效，或同门型编号重复".into());
        }
        if map.floors.is_empty() {
            return Err("地图缺少 floors 识别层".into());
        }
        for layers in [&mut map.floors, &mut map.display] {
            if let Some(path) = layers.remove("3") {
                layers.entry("b1f".into()).or_insert(path);
            }
            for (floor, path) in layers {
                if !["1", "2", "b1f"].contains(&floor.as_str()) {
                    return Err(format!("不支持楼层 {floor}"));
                }
                if files.insert(path.clone()) {
                    total += image_file(root, path)?.1;
                    if total > MAX_PACK {
                        return Err("地图包图片总大小超过 512 MiB".into());
                    }
                }
            }
        }
    }
    Ok(pack)
}
fn list(root: &Path) -> Result<JournalLibrary, String> {
    fs::create_dir_all(root).map_err(io)?;
    reject_link(root)?;
    let mut library = JournalLibrary {
        directory: root.display().to_string(),
        packs: vec![],
        errors: vec![],
    };
    for entry in fs::read_dir(root).map_err(io)? {
        let entry = entry.map_err(io)?;
        if !entry.file_type().map_err(io)?.is_dir() {
            continue;
        }
        match load_pack(&entry.path()) {
            Ok(pack) if entry.file_name().to_str() == Some(&pack.id) => library.packs.push(pack),
            Ok(_) => library.errors.push(format!(
                "{}：包目录名与 packId 不一致",
                entry.file_name().to_string_lossy()
            )),
            Err(e) => library
                .errors
                .push(format!("{}：{e}", entry.file_name().to_string_lossy())),
        }
    }
    library.packs.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(library)
}
fn import(root: &Path, source: &Path) -> Result<JournalLibrary, String> {
    let _lock = PACK_WRITE.lock().map_err(|_| "地图包操作锁异常")?;
    let pack = load_pack(source)?;
    fs::create_dir_all(root).map_err(io)?;
    reject_link(root)?;
    let destination = root.join(&pack.id);
    if destination.exists() {
        return Err("同名地图包已经存在，请先移除旧包".into());
    }
    let imports = root.parent().ok_or("无效包目录")?.join("imports");
    fs::create_dir_all(&imports).map_err(io)?;
    reject_link(&imports)?;
    let staging = imports.join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&staging).map_err(io)?;
    let files: BTreeSet<_> = pack
        .maps
        .iter()
        .flat_map(|m| m.floors.values().chain(m.display.values()))
        .collect();
    for path in files {
        let (source_image, _) = image_file(source, path)?;
        let target = staging.join(path);
        fs::create_dir_all(target.parent().ok_or("无效图片目录")?).map_err(io)?;
        fs::copy(source_image, target).map_err(io)?;
    }
    fs::write(
        staging.join("pack.json"),
        serde_json::to_vec_pretty(&pack).map_err(|e| e.to_string())?,
    )
    .map_err(io)?;
    load_pack(&staging)?;
    fs::rename(staging, destination).map_err(io)?;
    list(root)
}
#[tauri::command]
pub async fn journal_list_packs() -> Result<JournalLibrary, String> {
    tauri::async_runtime::spawn_blocking(|| list(&packs_root()?))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn journal_import_pack(source_path: String) -> Result<JournalLibrary, String> {
    tauri::async_runtime::spawn_blocking(move || import(&packs_root()?, Path::new(&source_path)))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn journal_read_image(pack_id: String, path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if !valid_id(&pack_id) {
            return Err("无效地图包 ID".into());
        }
        let root = packs_root()?;
        let pack_root = inside(&root, &pack_id)?;
        let (file, _) = image_file(&pack_root, &path)?;
        let data = fs::read(&file).map_err(io)?;
        if data.len() as u64 > MAX_IMAGE {
            return Err("图片超过大小限制".into());
        }
        Ok(format!(
            "data:{};base64,{}",
            mime(&file)?,
            STANDARD.encode(data)
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn journal_open_directory() -> Result<String, String> {
    let root = packs_root()?;
    fs::create_dir_all(&root).map_err(io)?;
    #[cfg(windows)]
    std::process::Command::new("explorer.exe")
        .arg(&root)
        .spawn()
        .map_err(io)?;
    #[cfg(not(windows))]
    return Err("仅 Windows 支持打开目录".into());
    #[cfg(windows)]
    Ok(root.display().to_string())
}
#[tauri::command]
pub async fn journal_choose_pack_folder() -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let result = std::process::Command::new("powershell.exe")
                .args(["-NoProfile", "-STA", "-Command", "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Add-Type -AssemblyName System.Windows.Forms; $picker = New-Object System.Windows.Forms.FolderBrowserDialog; $picker.Description = '选择包含 pack.json 的地图包文件夹'; $picker.ShowNewFolderButton = $false; try { if ($picker.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { [Console]::Write($picker.SelectedPath) } } finally { $picker.Dispose() }"])
                .creation_flags(0x08000000).output().map_err(io)?;
            if !result.status.success() { return Err("无法打开文件夹选择器".into()); }
            let path = String::from_utf8(result.stdout).map_err(|e| e.to_string())?;
            let path = path.trim_start_matches('\u{feff}').trim().to_string();
            Ok(if path.is_empty() { None } else { Some(path) })
        }
        #[cfg(not(windows))]
        Err("仅 Windows 支持目录选择器".into())
    }).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(root: &Path) {
        fs::create_dir_all(root).unwrap();
        fs::write(root.join("map.png"), b"png fixture").unwrap();
        fs::write(root.join("pack.json"), r#"{"packId":"test_pack","packName":"测试","seasonId":"s0","difficulty":"nightmare","maps":[{"index":1,"category":"triple","floors":{"1":"map.png","3":"map.png"}}]}"#).unwrap();
    }
    #[test]
    fn import_reload_and_reject_duplicate() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let root = dir.path().join("journal/packs");
        fixture(&source);
        fs::write(source.join("unrelated.txt"), b"do not copy").unwrap();
        let library = import(&root, &source).unwrap();
        assert_eq!(library.packs.len(), 1);
        assert!(library.packs[0].maps[0].floors.contains_key("b1f"));
        assert!(!root.join("test_pack/unrelated.txt").exists());
        assert_eq!(list(&root).unwrap().packs[0].name, "测试");
        assert!(import(&root, &source).unwrap_err().contains("已经存在"));
        assert!(source.join("map.png").is_file());
    }
    #[test]
    fn rejects_escaping_paths_missing_images_and_invalid_id() {
        let dir = tempfile::tempdir().unwrap();
        fixture(dir.path());
        assert!(inside(dir.path(), "../outside.png").is_err());
        assert!(inside(dir.path(), "C:/outside.png").is_err());
        assert!(inside(dir.path(), "nested\\outside.png").is_err());
        assert!(image_file(dir.path(), "missing.png").is_err());
        assert!(!valid_id("../escape"));
        let mut pack = load_pack(dir.path()).unwrap();
        pack.maps[0]
            .display
            .insert("1".into(), "missing.png".into());
        fs::write(
            dir.path().join("pack.json"),
            serde_json::to_vec(&pack).unwrap(),
        )
        .unwrap();
        assert!(load_pack(dir.path()).is_err());
    }
}
