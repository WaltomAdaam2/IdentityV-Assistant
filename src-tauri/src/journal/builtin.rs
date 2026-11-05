use super::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Cursor;

const PACKS: [(&str, &[u8]); 2] = [
    (
        "s0_hard_11-05",
        include_bytes!("../../../assets/journal/s0_hard_11-05.zip"),
    ),
    (
        "s0_nightmare_11-05",
        include_bytes!("../../../assets/journal/s0_nightmare_11-05.zip"),
    ),
];
static SEEDED: Mutex<bool> = Mutex::new(false);

pub(super) fn ensure(app: &tauri::AppHandle) -> Result<(), String> {
    let mut seeded = SEEDED.lock().map_err(|_| "地图初始化锁异常")?;
    if *seeded {
        return Ok(());
    }
    let root = packs_root()?;
    let _write = PACK_WRITE.lock().map_err(|_| "地图包操作锁异常")?;
    let result = seed(&root);
    match &result {
        Ok(events) => {
            for event in events {
                crate::logs::record(
                    app,
                    if event["conflict"] == true {
                        "warning"
                    } else {
                        "info"
                    },
                    "journal.builtin",
                    "内置地图检查 / Bundled map check",
                    event.clone(),
                );
            }
            *seeded = true;
        }
        Err(error) => crate::logs::record(
            app,
            "error",
            "journal.builtin",
            "内置地图导入失败 / Bundled map import failed",
            json!({"error":error}),
        ),
    }
    result.map(|_| ())
}

fn seed(root: &Path) -> Result<Vec<Value>, String> {
    fs::create_dir_all(root).map_err(io)?;
    reject_link(root)?;
    let mut events = Vec::new();
    let mut available = BTreeSet::new();
    for (id, bytes) in PACKS {
        let destination = root.join(id);
        if !destination.exists() {
            import_archive(root, Cursor::new(bytes))?;
            events.push(json!({"pack":id,"action":"imported"}));
            available.insert(id);
            continue;
        }
        reject_link(&destination)?;
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
        let descriptor = read_bounded(
            archive.by_name("pack.json").map_err(|e| e.to_string())?,
            MAX_DESCRIPTOR,
        )?;
        let expected: JournalPack =
            serde_json::from_slice(&descriptor).map_err(|e| e.to_string())?;
        // Read only the descriptor: missing images are repaired below.
        let descriptor = (|| -> Result<JournalPack, String> {
            let bytes = read_bounded(
                fs::File::open(inside(&destination, "pack.json")?).map_err(io)?,
                MAX_DESCRIPTOR,
            )?;
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())
        })();
        let existing = match descriptor {
            Ok(pack) => pack,
            Err(error) => {
                events.push(json!({"pack":id,"conflict":true,
                    "action":"unreadable_manifest_preserved","error":error}));
                continue;
            }
        };
        if serde_json::to_value(&existing).unwrap() != serde_json::to_value(&expected).unwrap() {
            events.push(json!({"pack":id,"conflict":true,"action":"custom_manifest_preserved"}));
            continue;
        }
        let mut conflict = false;
        let mut repaired = 0;
        for path in referenced_images(&expected) {
            let data = read_bounded(archive.by_name(path).map_err(|e| e.to_string())?, MAX_IMAGE)?;
            let target = destination.join(path);
            if target.exists() {
                let (safe, _) = image_file(&destination, path)?;
                let current = read_bounded(fs::File::open(safe).map_err(io)?, MAX_IMAGE)?;
                if current != data {
                    conflict = true;
                }
            } else {
                // Validate every existing ancestor before creating a missing image.
                validate_relative(path)?;
                let mut ancestor = destination.clone();
                for part in Path::new(path).parent().unwrap().components() {
                    ancestor.push(part);
                    if ancestor.exists() {
                        reject_link(&ancestor)?;
                    } else {
                        fs::create_dir(&ancestor).map_err(io)?;
                    }
                }
                use std::io::Write;
                let mut pending =
                    tempfile::NamedTempFile::new_in(target.parent().unwrap()).map_err(io)?;
                pending.write_all(&data).map_err(io)?;
                pending.as_file().sync_all().map_err(io)?;
                pending
                    .persist_noclobber(target)
                    .map_err(|e| e.to_string())?;
                repaired += 1;
            }
        }
        if !conflict {
            available.insert(id);
        }
        events.push(
            json!({"pack":id,"action":"checked","repaired_images":repaired,"conflict":conflict}),
        );
    }
    let sources: Vec<Value> =
        serde_json::from_str(include_str!("../../../assets/journal/sources.json"))
            .map_err(|e| e.to_string())?;
    for source in sources {
        let pack: JournalPack =
            serde_json::from_value(source["pack"].clone()).map_err(|e| e.to_string())?;
        if !available.contains(format!("s0_{}_11-05", pack.difficulty).as_str()) {
            continue;
        }
        let old = root.join(&pack.id);
        if !old.exists() {
            continue;
        }
        let matches = (|| {
            let imported = load_pack(&old)?;
            if serde_json::to_value(imported).unwrap() != source["pack"] {
                return Ok(false);
            }
            for (path, hash) in source["files"]
                .as_object()
                .ok_or("Invalid source fingerprints")?
            {
                let (file, _) = image_file(&old, path)?;
                let bytes = read_bounded(fs::File::open(file).map_err(io)?, MAX_IMAGE)?;
                if json!(format!("{:x}", Sha256::digest(bytes))) != *hash {
                    return Ok(false);
                }
            }
            let known: BTreeSet<_> = source["files"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .chain(std::iter::once("pack.json".into()))
                .collect();
            let mut directories = vec![old.clone()];
            let mut count = 0;
            while let Some(directory) = directories.pop() {
                for entry in fs::read_dir(directory).map_err(io)? {
                    let entry = entry.map_err(io)?;
                    reject_link(&entry.path())?;
                    count += 1;
                    if count > MAX_ENTRIES {
                        return Ok(false);
                    }
                    if entry.file_type().map_err(io)?.is_dir() {
                        directories.push(entry.path());
                    } else {
                        let relative = entry
                            .path()
                            .strip_prefix(&old)
                            .map_err(|e| e.to_string())?
                            .to_string_lossy()
                            .replace('\\', "/");
                        if !known.contains(&relative) {
                            return Ok(false);
                        }
                    }
                }
            }
            Ok::<_, String>(true)
        })()
        .unwrap_or(false);
        if !matches {
            events.push(json!({"pack":pack.id,"conflict":true,"action":"custom_source_preserved"}));
            continue;
        }
        let backups = root.parent().ok_or("Invalid map root")?.join("backups");
        fs::create_dir_all(&backups).map_err(io)?;
        reject_link(&backups)?;
        // Rename is an atomic backup, outside the active pack catalogue. Never delete source data.
        let backup = backups.join(format!("{}-{}", pack.id, uuid::Uuid::new_v4()));
        let safe = inside(root, &pack.id)?;
        fs::rename(safe, &backup).map_err(io)?;
        events.push(json!({"pack":pack.id,"action":"backed_up_and_consolidated","backup":backup}));
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seeds_both_packs_and_repairs_missing_images_without_overwriting_custom_content() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("packs");
        seed(&root).unwrap();
        let library = list(&root).unwrap();
        assert_eq!(
            library
                .packs
                .iter()
                .map(|p| p.maps.len())
                .collect::<Vec<_>>(),
            vec![29, 26]
        );
        let first = &library.packs[0];
        let image = root.join(&first.id).join(&first.maps[0].floors["1"]);
        let original = fs::read(&image).unwrap();
        fs::remove_file(&image).unwrap();
        assert!(seed(&root)
            .unwrap()
            .iter()
            .any(|e| e["repaired_images"] == 1));
        assert_eq!(fs::read(&image).unwrap(), original);
        fs::write(&image, b"custom image").unwrap();
        assert!(seed(&root).unwrap().iter().any(|e| e["conflict"] == true));
        assert_eq!(fs::read(image).unwrap(), b"custom image");
        assert_eq!(list(&root).unwrap().packs.len(), 2);
        let descriptor = root.join(&first.id).join("pack.json");
        fs::write(&descriptor, b"user's unfinished edit").unwrap();
        assert!(seed(&root)
            .unwrap()
            .iter()
            .any(|event| event["action"] == "unreadable_manifest_preserved"));
        assert_eq!(fs::read(descriptor).unwrap(), b"user's unfinished edit");
        let remaining = list(&root).unwrap();
        assert_eq!(remaining.packs.len(), 1);
        assert_eq!(remaining.packs[0].difficulty, "nightmare");
        assert_eq!(remaining.packs[0].maps.len(), 26);
    }

    #[test]
    fn original_source_is_backed_up_but_extra_user_files_keep_the_pack_active() {
        for customized in [false, true] {
            let temporary = tempfile::tempdir().unwrap();
            let root = temporary.path().join("packs");
            seed(&root).unwrap();
            let sources: Vec<Value> =
                serde_json::from_str(include_str!("../../../assets/journal/sources.json")).unwrap();
            let source: JournalPack = serde_json::from_value(sources[0]["pack"].clone()).unwrap();
            let merged = load_pack(&root.join("s0_nightmare_11-05")).unwrap();
            let old = root.join(&source.id);
            fs::create_dir(&old).unwrap();
            fs::write(old.join("pack.json"), serde_json::to_vec(&source).unwrap()).unwrap();
            for map in &source.maps {
                let item = merged
                    .maps
                    .iter()
                    .find(|m| m.category == map.category && m.index == map.index)
                    .unwrap();
                for (floor, relative) in &map.floors {
                    let target = old.join(relative);
                    fs::create_dir_all(target.parent().unwrap()).unwrap();
                    fs::copy(root.join(&merged.id).join(&item.floors[floor]), target).unwrap();
                }
            }
            if customized {
                fs::write(old.join("my-notes.txt"), b"keep me").unwrap();
            }
            let events = seed(&root).unwrap();
            if customized {
                assert!(old.join("my-notes.txt").is_file());
                assert_eq!(list(&root).unwrap().packs[0].id, source.id);
                assert!(events
                    .iter()
                    .any(|e| e["action"] == "custom_source_preserved"));
            } else {
                assert!(!old.exists());
                let backup = events
                    .iter()
                    .find(|e| e["action"] == "backed_up_and_consolidated")
                    .unwrap();
                assert!(Path::new(backup["backup"].as_str().unwrap())
                    .join("pack.json")
                    .is_file());
                assert_eq!(list(&root).unwrap().packs.len(), 2);
            }
        }
    }
}
