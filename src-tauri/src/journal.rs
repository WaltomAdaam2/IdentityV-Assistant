//! App-local journal packs. No paths from another assistant are consulted.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    sync::Mutex,
};

static PACK_WRITE: Mutex<()> = Mutex::new(());
const MAX_IMAGE: u64 = 20 * 1024 * 1024;
const MAX_PACK: u64 = 512 * 1024 * 1024;
const MAX_DESCRIPTOR: u64 = 2 * 1024 * 1024;
const MAX_ENTRIES: usize = 20_000;
const MAX_DEPTH: usize = 16;

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
fn validate_relative(relative: &str) -> Result<(), String> {
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
    for part in relative.split('/') {
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || part
                .chars()
                .any(|c| c.is_control() || "<>\"|?*".contains(c))
            || ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        {
            return Err("地图路径包含不安全的文件名".into());
        }
    }
    Ok(())
}
fn inside(root: &Path, relative: &str) -> Result<PathBuf, String> {
    validate_relative(relative)?;
    let path = Path::new(relative);
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
    parse_pack(
        &read_bounded(fs::File::open(descriptor).map_err(io)?, MAX_DESCRIPTOR)?,
        |path| Ok(image_file(root, path)?.1),
    )
}
fn read_bounded(reader: impl Read, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes).map_err(io)?;
    if bytes.len() as u64 > limit {
        return Err("地图文件超过大小限制".into());
    }
    Ok(bytes)
}
fn parse_pack(
    bytes: &[u8],
    mut image_size: impl FnMut(&str) -> Result<u64, String>,
) -> Result<JournalPack, String> {
    // Windows-exported descriptors commonly include a UTF-8 BOM.
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let mut pack: JournalPack =
        serde_json::from_slice(bytes).map_err(|e| format!("地图描述格式错误：{e}"))?;
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
                    validate_relative(path)?;
                    mime(Path::new(path))?;
                    let size = image_size(path)?;
                    if size == 0 || size > MAX_IMAGE {
                        return Err(format!("地图图片必须是 20 MiB 以内的非空文件：{path}"));
                    }
                    total += size;
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

/// Recognition identity spans door categories; display eligibility stays strict.
pub(crate) fn recognition_templates(
    season: &str,
    difficulty: &str,
    display: &str,
) -> Result<serde_json::Value, String> {
    recognition_templates_at(&packs_root()?, season, difficulty, display)
}

fn recognition_templates_at(
    root: &Path,
    season: &str,
    difficulty: &str,
    display: &str,
) -> Result<serde_json::Value, String> {
    if season.is_empty()
        || season.len() > 100
        || !["hard", "nightmare"].contains(&difficulty)
        || !["pure", "route"].contains(&display)
    {
        return Err("无效识图条件 / Invalid recognition filters".into());
    }
    let library = list(root)?;
    let packs: Vec<_> = library
        .packs
        .iter()
        .filter(|pack| pack.season_id == season && pack.difficulty == difficulty)
        .collect();
    let mut eligible = BTreeSet::new();
    let mut sources = BTreeMap::new();
    for pack in packs {
        for map in &pack.maps {
            let key = format!("{}:{}", map.category, map.index);
            let layers = if display == "route" {
                &map.display
            } else {
                &map.floors
            };
            if layers.contains_key("1") {
                eligible.insert(key.clone());
            }
            if let Some(path) = map.floors.get("1") {
                // Prefer an unannotated base pack for geometry, even in route view.
                let annotated = !map.display.is_empty();
                let replace = sources
                    .get(&key)
                    .is_none_or(|(old, _, _)| *old && !annotated);
                if replace {
                    sources.insert(key, (annotated, pack.id.clone(), path.clone()));
                }
            }
        }
    }
    let mut templates = Vec::new();
    for key in eligible {
        if let Some((_, pack_id, path)) = sources.get(&key) {
            let pack_root = inside(&root, pack_id)?;
            let (path, _) = image_file(&pack_root, path)?;
            templates.push(serde_json::json!({"key": key, "path": path}));
        }
    }
    if templates.is_empty() {
        return Err("当前分类缺少一楼地图，请导入对应地图包 / Import first-floor maps for the selected category".into());
    }
    Ok(serde_json::Value::Array(templates))
}
fn find_pack_folder(source: &Path) -> Result<PathBuf, String> {
    reject_link(source)?;
    if source.join("pack.json").exists() {
        return Ok(source.to_path_buf());
    }
    let mut stack = vec![(source.to_path_buf(), 0)];
    let mut candidates = Vec::new();
    let mut visited = 0;
    while let Some((directory, depth)) = stack.pop() {
        reject_link(&directory)?;
        if directory.join("pack.json").exists() {
            candidates.push(directory);
            if candidates.len() > 1 {
                return Err("找到多个地图包，请选择具体地图包文件夹".into());
            }
            continue;
        }
        for entry in fs::read_dir(directory).map_err(io)? {
            let entry = entry.map_err(io)?;
            visited += 1;
            if visited > MAX_ENTRIES {
                return Err("地图目录条目超过 20000，请选择更具体的文件夹".into());
            }
            reject_link(&entry.path())?;
            if entry.file_type().map_err(io)?.is_dir() {
                if depth >= MAX_DEPTH {
                    return Err("地图目录嵌套超过 16 层，请选择更具体的文件夹".into());
                }
                stack.push((entry.path(), depth + 1));
            }
        }
    }
    candidates
        .pop()
        .ok_or_else(|| "所选目录及子目录中没有 pack.json".into())
}
fn create_staging(root: &Path, pack: &JournalPack) -> Result<PathBuf, String> {
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
    Ok(staging)
}
fn referenced_images(pack: &JournalPack) -> BTreeSet<&String> {
    pack.maps
        .iter()
        .flat_map(|m| m.floors.values().chain(m.display.values()))
        .collect()
}
fn finish_import(
    root: &Path,
    staging: &Path,
    pack: &JournalPack,
) -> Result<JournalLibrary, String> {
    fs::write(
        staging.join("pack.json"),
        serde_json::to_vec_pretty(&pack).map_err(|e| e.to_string())?,
    )
    .map_err(io)?;
    load_pack(&staging)?;
    fs::rename(staging, root.join(&pack.id)).map_err(io)?;
    list(root)
}
fn import(root: &Path, source: &Path) -> Result<JournalLibrary, String> {
    let _lock = PACK_WRITE.lock().map_err(|_| "地图包操作锁异常")?;
    reject_link(source)?;
    if source.is_file() {
        if source
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("zip"))
        {
            return import_zip(root, source);
        }
        return Err("请选择地图包文件夹或 ZIP 文件".into());
    }
    let source = find_pack_folder(source)?;
    let pack = load_pack(&source)?;
    let staging = create_staging(root, &pack)?;
    let mut copied_bytes = 0u64;
    for path in referenced_images(&pack) {
        let (source_image, _) = image_file(&source, path)?;
        let target = staging.join(path);
        fs::create_dir_all(target.parent().ok_or("无效图片目录")?).map_err(io)?;
        let bytes = read_bounded(fs::File::open(source_image).map_err(io)?, MAX_IMAGE)?;
        copied_bytes += bytes.len() as u64;
        if copied_bytes > MAX_PACK {
            return Err("地图包图片总大小超过 512 MiB".into());
        }
        fs::write(target, bytes).map_err(io)?;
    }
    finish_import(root, &staging, &pack)
}

fn import_zip(root: &Path, source: &Path) -> Result<JournalLibrary, String> {
    if fs::metadata(source).map_err(io)?.len() > MAX_PACK {
        return Err("ZIP 文件超过 512 MiB".into());
    }
    let mut archive = zip::ZipArchive::new(fs::File::open(source).map_err(io)?)
        .map_err(|e| format!("无法读取 ZIP：{e}"))?;
    if archive.len() > MAX_ENTRIES {
        return Err("ZIP 条目超过 20000".into());
    }
    let mut entries = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut descriptors = Vec::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(|e| e.to_string())?;
        let name = file.name().map_err(|e| e.to_string())?.into_owned();
        let path = name.trim_end_matches('/');
        validate_relative(path)?;
        if path.split('/').count() > MAX_DEPTH + 1 {
            return Err("ZIP 目录嵌套超过 16 层".into());
        }
        if !seen.insert(path.to_lowercase()) {
            return Err("ZIP 包含重复或大小写冲突路径".into());
        }
        if file.is_symlink()
            || file.unix_mode().is_some_and(|m| {
                m & 0o170000 != 0 && ![0o100000, 0o040000].contains(&(m & 0o170000))
            })
        {
            return Err("ZIP 不支持链接或特殊文件".into());
        }
        total = total.checked_add(file.size()).ok_or("ZIP 大小溢出")?;
        if total > MAX_PACK || file.size() > MAX_IMAGE {
            return Err("ZIP 解压大小超过限制".into());
        }
        if file.is_dir() {
            continue;
        }
        if path == "pack.json" || path.ends_with("/pack.json") {
            descriptors.push(name.clone());
        }
        entries.insert(name, (index, file.size()));
    }
    let descriptor = if descriptors.iter().any(|p| p == "pack.json") {
        "pack.json".to_owned()
    } else if descriptors.len() == 1 {
        descriptors.pop().unwrap()
    } else if descriptors.is_empty() {
        return Err("ZIP 中没有 pack.json".into());
    } else {
        return Err("ZIP 中有多个地图包，请分别打包或选择具体地图包文件夹".into());
    };
    let prefix = descriptor.strip_suffix("pack.json").unwrap();
    let (descriptor_index, size) = entries[&descriptor];
    if size > MAX_DESCRIPTOR {
        return Err("地图描述文件超过 2 MiB".into());
    }
    let bytes = read_bounded(
        archive
            .by_index(descriptor_index)
            .map_err(|e| e.to_string())?,
        MAX_DESCRIPTOR,
    )?;
    let pack = parse_pack(&bytes, |path| {
        entries
            .get(&format!("{prefix}{path}"))
            .map(|(_, size)| *size)
            .ok_or_else(|| format!("ZIP 缺少图片：{path}"))
    })?;
    let staging = create_staging(root, &pack)?;
    let mut copied_bytes = 0u64;
    for path in referenced_images(&pack) {
        let (index, _) = entries[&format!("{prefix}{path}")];
        let bytes = read_bounded(
            archive.by_index(index).map_err(|e| e.to_string())?,
            MAX_IMAGE,
        )?;
        copied_bytes += bytes.len() as u64;
        if copied_bytes > MAX_PACK {
            return Err("ZIP 实际解压大小超过 512 MiB".into());
        }
        let target = staging.join(path);
        fs::create_dir_all(target.parent().ok_or("无效图片目录")?).map_err(io)?;
        fs::write(target, bytes).map_err(io)?;
    }
    finish_import(root, &staging, &pack)
}
#[tauri::command]
pub async fn journal_list_packs() -> Result<JournalLibrary, String> {
    tauri::async_runtime::spawn_blocking(|| list(&packs_root()?))
        .await
        .map_err(|e| e.to_string())?
}
fn validate_zip_source(source: &Path) -> Result<(), String> {
    if source.is_file()
        && source
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("zip"))
    {
        Ok(())
    } else {
        Err("仅支持导入 ZIP 地图包，请选择一个或多个 .zip 文件".into())
    }
}
#[tauri::command]
pub async fn journal_import_pack(source_path: String) -> Result<JournalLibrary, String> {
    validate_zip_source(Path::new(&source_path))?;
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
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn zip_fixture(path: &Path, prefix: &str, extra: Option<&str>) {
        let source = tempfile::tempdir().unwrap();
        fixture(source.path());
        let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for name in ["pack.json", "map.png"] {
            zip.start_file(format!("{prefix}{name}"), options).unwrap();
            zip.write_all(&fs::read(source.path().join(name)).unwrap())
                .unwrap();
        }
        if let Some(extra) = extra {
            zip.start_file(extra, options).unwrap();
            zip.write_all(b"unrelated").unwrap();
        }
        zip.finish().unwrap();
    }
    fn fixture(root: &Path) {
        fs::create_dir_all(root).unwrap();
        fs::write(root.join("map.png"), b"png fixture").unwrap();
        fs::write(root.join("pack.json"), r#"{"packId":"test_pack","packName":"测试","seasonId":"s0","difficulty":"nightmare","maps":[{"index":1,"category":"triple","floors":{"1":"map.png","3":"map.png"}}]}"#).unwrap();
    }
    #[test]
    fn import_command_accepts_only_zip_files() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("pack.ZIP");
        fs::write(&zip, b"test").unwrap();
        assert!(validate_zip_source(&zip).is_ok());
        assert!(validate_zip_source(dir.path()).is_err());
        let json = dir.path().join("pack.json");
        fs::write(&json, b"{}").unwrap();
        assert!(validate_zip_source(&json).is_err());
        assert!(validate_zip_source(&dir.path().join("missing.zip")).is_err());
    }

    #[test]
    fn recognition_index_is_first_floor_display_strict_and_prefers_clean_geometry() {
        let root = tempfile::tempdir().unwrap();
        for (id, maps) in [
            (
                "base",
                serde_json::json!([
                    {"index":1,"category":"lf","floors":{"1":"map.png"}},
                    {"index":1,"category":"rf","floors":{"1":"map.png"}},
                    {"index":2,"category":"rf","floors":{"2":"map.png"}}
                ]),
            ),
            (
                "a_route",
                serde_json::json!([
                    {"index":1,"category":"rf","floors":{"1":"map.png"},"display":{"1":"map.png"}},
                    {"index":1,"category":"lf","floors":{"1":"map.png"},"display":{"2":"map.png"}}
                ]),
            ),
        ] {
            let path = root.path().join(id);
            fs::create_dir(&path).unwrap();
            fs::write(path.join("map.png"), b"fixture").unwrap();
            fs::write(path.join("pack.json"), serde_json::json!({"id":id,"name":id,"season_id":"s0","difficulty":"hard","maps":maps}).to_string()).unwrap();
        }
        let pure = recognition_templates_at(root.path(), "s0", "hard", "pure").unwrap();
        assert_eq!(pure.as_array().unwrap().len(), 2);
        let route = recognition_templates_at(root.path(), "s0", "hard", "route").unwrap();
        assert_eq!(route.as_array().unwrap().len(), 1);
        assert_eq!(route[0]["key"], "rf:1");
        assert_eq!(
            Path::new(route[0]["path"].as_str().unwrap()),
            root.path().join("base/map.png").canonicalize().unwrap()
        );
        assert!(recognition_templates_at(root.path(), "s1", "hard", "route").is_err());
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
    #[test]
    fn discovers_nested_folder_and_rejects_ambiguity() {
        let source = tempfile::tempdir().unwrap();
        fixture(&source.path().join("one/two/three/four"));
        let descriptor = source.path().join("one/two/three/four/pack.json");
        let mut bom_json = vec![0xef, 0xbb, 0xbf];
        bom_json.extend(fs::read(&descriptor).unwrap());
        fs::write(descriptor, bom_json).unwrap();
        assert_eq!(
            find_pack_folder(source.path()).unwrap(),
            source.path().join("one/two/three/four")
        );
        let destination = tempfile::tempdir().unwrap();
        assert_eq!(
            import(&destination.path().join("packs"), source.path())
                .unwrap()
                .packs
                .len(),
            1
        );
        fixture(&source.path().join("second"));
        assert!(find_pack_folder(source.path())
            .unwrap_err()
            .contains("多个"));
        fixture(source.path());
        assert_eq!(find_pack_folder(source.path()).unwrap(), source.path());
    }
    #[test]
    fn imports_zip_root_and_nested_without_unrelated_files() {
        for prefix in ["", "outer/inner/third/"] {
            let dir = tempfile::tempdir().unwrap();
            let archive = dir.path().join("sample.zip");
            zip_fixture(&archive, prefix, Some("unrelated.txt"));
            let root = dir.path().join("packs");
            assert_eq!(import(&root, &archive).unwrap().packs.len(), 1);
            assert!(root.join("test_pack/map.png").is_file());
            assert!(!root.join("test_pack/unrelated.txt").exists());
            assert!(import(&root, &archive).unwrap_err().contains("已经存在"));
        }
    }
    #[test]
    fn rejects_zip_traversal_ambiguity_and_resource_limits() {
        let dir = tempfile::tempdir().unwrap();
        for extra in [
            "../escape.png",
            "/absolute.png",
            "C:/absolute.png",
            "nested/../../escape.png",
        ] {
            let archive = dir.path().join("unsafe.zip");
            zip_fixture(&archive, "nested/", Some(extra));
            assert!(
                import(&dir.path().join("packs"), &archive).is_err(),
                "{extra}"
            );
        }
        let ambiguous = dir.path().join("ambiguous.zip");
        zip_fixture(&ambiguous, "one/", Some("two/pack.json"));
        assert!(import(&dir.path().join("packs"), &ambiguous)
            .unwrap_err()
            .contains("多个"));
        assert!(read_bounded(&b"12345"[..], 4).is_err());
        let oversized = dir.path().join("oversized.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&oversized).unwrap());
        zip.start_file(
            "huge.png",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
        let chunk = [0u8; 1024];
        for _ in 0..(MAX_IMAGE / 1024 + 1) {
            zip.write_all(&chunk).unwrap();
        }
        zip.finish().unwrap();
        assert!(import(&dir.path().join("packs"), &oversized)
            .unwrap_err()
            .contains("限制"));
    }
    #[test]
    fn rejects_malformed_descriptor_and_deep_folders() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("pack.json"), b"not json").unwrap();
        assert!(load_pack(dir.path()).unwrap_err().contains("格式错误"));
        let nested = tempfile::tempdir().unwrap();
        let mut leaf = nested.path().to_path_buf();
        for _ in 0..MAX_DEPTH + 2 {
            leaf.push("level");
        }
        fs::create_dir_all(leaf).unwrap();
        assert!(find_pack_folder(nested.path()).unwrap_err().contains("16"));
    }
    #[test]
    #[ignore = "Set JOURNAL_SAMPLE_ROOT to the folder containing the five user-supplied samples"]
    fn import_user_samples_to_temporary_storage() {
        let source =
            PathBuf::from(std::env::var_os("JOURNAL_SAMPLE_ROOT").expect("JOURNAL_SAMPLE_ROOT"));
        for name in [
            "s0_hard_president",
            "s0_hard_speedrun",
            "s0_nightmare_speedrun",
            "s0_nightmare_speedrun.zip",
            "s0_nightmare_maps.zip",
        ] {
            let destination = tempfile::tempdir().unwrap();
            let library = import(&destination.path().join("packs"), &source.join(name))
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(library.packs.len(), 1);
            assert!(library.errors.is_empty());
            println!("{name}: {} maps", library.packs[0].maps.len());
        }
    }
}
