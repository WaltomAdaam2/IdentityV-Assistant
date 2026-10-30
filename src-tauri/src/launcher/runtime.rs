use super::component::{LoginRuntime, SUPPORTED_VERSION};
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};
use winreg::{enums::HKEY_CURRENT_USER, RegKey, RegValue};

const URI_KEY: &str = r"Software\Classes\idvlogin";

pub(super) fn managed_root() -> Result<PathBuf, String> {
    Ok(std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("助手程序目录无效")?
        .join("components/idv-login")
        .join(SUPPORTED_VERSION))
}

pub(super) fn runtime_from_selection(path: PathBuf) -> Result<LoginRuntime, String> {
    let python = if path.is_dir() {
        path.join("python-embed/python.exe")
    } else if path
        .file_name()
        .is_some_and(|name| name == "点我启动工具.bat")
    {
        path.parent()
            .ok_or("组件目录无效")?
            .join("python-embed/python.exe")
    } else {
        path
    };
    runtime_from_python(python)
}

pub(super) fn runtime_from_python(python: PathBuf) -> Result<LoginRuntime, String> {
    let root = python
        .parent()
        .and_then(Path::parent)
        .ok_or("登录组件路径无效")?;
    let script = root.join("src/main.pyc");
    if !python.is_absolute()
        || !python.is_file()
        || !python
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("python.exe"))
        || !python
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name.eq_ignore_ascii_case("python-embed"))
        || !script.is_file()
        || !root.join("src/buildinfo.pyc").is_file()
        || !root.join("assets").is_dir()
    {
        return Err("请选择包含“点我启动工具.bat”、python-embed 和 src 的 idv-login 文件夹".into());
    }
    Ok(LoginRuntime { python, script })
}

fn command_python(command: &str) -> Option<PathBuf> {
    let command = command.trim();
    let path = if let Some(quoted) = command.strip_prefix('"') {
        quoted.split_once('"')?.0
    } else {
        command.split_whitespace().next()?
    };
    Some(PathBuf::from(path))
}

pub(super) fn component_path(directory: &Path) -> Result<Option<PathBuf>, String> {
    let preference = directory.join("launcher.json");
    if preference.is_file() {
        if let Ok(path) = fs::read(preference)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<String>(&bytes).ok())
            .ok_or(())
        {
            if let Ok(runtime) = runtime_from_selection(PathBuf::from(path)) {
                return Ok(Some(runtime.python));
            }
        }
    }
    let mut roots = vec![managed_root()?];
    if let Some(program_data) = std::env::var_os("PROGRAMDATA") {
        let flag = PathBuf::from(program_data).join("idv-login/install_root.flag");
        if fs::metadata(&flag).is_ok_and(|meta| meta.len() <= 4096) {
            if let Ok(root) = fs::read_to_string(flag) {
                roots.push(PathBuf::from(root.trim()));
            }
        }
    }
    roots.extend([
        PathBuf::from(r"D:\ProgramData\IDV-Login"),
        PathBuf::from(r"C:\ProgramData\IDV-Login"),
    ]);
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(command) = current_user.open_subkey(format!(r"{URI_KEY}\shell\open\command")) {
        if let Ok(value) = command.get_value::<String, _>("") {
            if let Some(path) =
                command_python(&value).filter(|path| runtime_from_python(path.clone()).is_ok())
            {
                if let Some(root) = path.parent().and_then(Path::parent) {
                    roots.push(root.to_path_buf());
                }
            }
        }
    }
    // Official fallback is Public Documents, not the current user's Documents.
    use windows::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_PublicDocuments, SHGetKnownFolderPath, KF_FLAG_DEFAULT},
    };
    if let Ok(pointer) =
        unsafe { SHGetKnownFolderPath(&FOLDERID_PublicDocuments, KF_FLAG_DEFAULT, None) }
    {
        let documents = unsafe { pointer.to_string() };
        unsafe {
            CoTaskMemFree(Some(pointer.0.cast()));
        }
        if let Ok(documents) = documents {
            roots.push(PathBuf::from(documents).join("IDV-Login"));
        }
    }
    Ok(roots.into_iter().find_map(|root| {
        runtime_from_selection(root)
            .ok()
            .map(|runtime| runtime.python)
    }))
}

pub(super) async fn verify_version(runtime: &LoginRuntime) -> Result<(), String> {
    let mut command = tokio::process::Command::new(&runtime.python);
    command.args(["-I", "-c", "import sys; sys.path.insert(0, sys.argv[1]); import buildinfo; print(buildinfo.VERSION)"])
        .arg(runtime.script.parent().ok_or("登录组件目录无效")?)
        .env_remove("PYTHONHOME").env_remove("PYTHONPATH")
        .creation_flags(0x08000000).kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(5), command.output())
        .await
        .map_err(|_| "读取登录组件版本超时")?
        .map_err(|e| e.to_string())?;
    if !output.status.success()
        || String::from_utf8_lossy(&output.stdout).trim() != SUPPORTED_VERSION
    {
        return Err("当前仅支持官方 idv-login v6.3.2-stable，请选择对应版本".into());
    }
    Ok(())
}

pub(super) fn save_component(directory: &Path, python: &Path) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    fs::write(
        directory.join("launcher.json"),
        serde_json::to_vec(python.to_str().ok_or("登录组件路径无效")?)
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub(super) fn ensure_can_start() -> Result<(), String> {
    use windows::{
        core::w,
        Win32::{
            Foundation::{GetLastError, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND},
            System::Pipes::WaitNamedPipeW,
            UI::Shell::IsUserAnAdmin,
        },
    };
    if !unsafe { IsUserAnAdmin() }.as_bool() {
        return Err("请以管理员身份运行助手，以便持有登录组件的进程".into());
    }
    if unsafe { WaitNamedPipeW(w!(r"\\.\pipe\idvlogin_uri_signal"), 1) }.as_bool() {
        return Err("已有 idv-login 正在运行，请先关闭它，再从助手启动游戏".into());
    }
    let error = unsafe { GetLastError() };
    if error != ERROR_FILE_NOT_FOUND && error != ERROR_PATH_NOT_FOUND {
        return Err("已有登录组件或无法确认其状态，请关闭原工具后重试".into());
    }
    Ok(())
}

pub(super) fn prepare_data(program_data: &Path) -> Result<(), String> {
    let folder = program_data.join("idv-login");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let path = folder.join("config.json");
    let mut config: Value = if path.is_file() {
        serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
            .map_err(|_| "登录组件配置损坏；为保护已有数据，未覆盖该文件")?
    } else {
        json!({})
    };
    if !config.is_object() {
        return Err("登录组件配置格式无效".into());
    }
    if config.get("_SAVED_PROXY_ENV").is_some() {
        return Err("组件存在全局代理恢复记录，请先在官方工具中恢复网络设置".into());
    }
    config["proxy_mode"] = json!("process");
    for flag in [
        "proxy_mode_asked_0403",
        "proxy_mode_compat_prompted_0420",
        "hosts_cleanup_v600_done",
        "nrpt_env_cleanup_v603_done",
        "config_fixed_0403",
        "account_switching_wizard_632_offered",
    ] {
        config[flag] = json!(true);
    }
    config[format!("{SUPPORTED_VERSION}_first_use")] = json!(false);
    for key in [
        "game_settings",
        "game_installation_settings_v1",
        "installation_state_v1",
    ] {
        if let Some(settings) = config.get_mut(key) {
            disable_autostart(settings);
        }
    }
    // Write and sync successfully before opting out of upstream's destructive repair.
    let pending = folder.join(format!("assistant-config-{}.tmp", uuid::Uuid::new_v4()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)
        .map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(pending, path).map_err(|e| e.to_string())?;
    fs::write(
        program_data.join("assistant-bootstrap.py"),
        include_str!("bootstrap.py"),
    )
    .map_err(|e| e.to_string())
}

fn disable_autostart(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if key == "auto_start" || key == "should_auto_start" {
                    *value = json!(false);
                } else {
                    disable_autostart(value);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                disable_autostart(value);
            }
        }
        _ => {}
    }
}

pub(super) struct SchemeSnapshot(Option<Vec<(String, Vec<(String, RegValue)>)>>);

impl SchemeSnapshot {
    pub fn capture() -> Result<Self, String> {
        match RegKey::predef(HKEY_CURRENT_USER).open_subkey(URI_KEY) {
            Ok(key) => {
                let mut entries = Vec::new();
                read_key(&key, "", &mut entries, 0)?;
                Ok(Self(Some(entries)))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self(None)),
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn restore(self, python: &Path) -> Result<(), String> {
        let current_user = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = current_user.open_subkey(URI_KEY) {
            let current = key
                .open_subkey(r"shell\open\command")
                .and_then(|key| key.get_value::<String, _>(""));
            match current {
                Ok(command) => {
                    let ours = command_python(&command).and_then(|path| path.canonicalize().ok());
                    if ours.is_none() || ours != python.canonicalize().ok() {
                        return Ok(()); // Preserve a different application's registration.
                    }
                }
                // Upstream deletes command before root; extra old subkeys can prevent
                // its root deletion. Restore the captured registration in that case.
                Err(error) if error.kind() == std::io::ErrorKind::NotFound && self.0.is_some() => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error.to_string()),
            }
            if self.0.is_none() {
                // Only the explicit URI key, still pointing to our owned runtime.
                current_user
                    .delete_subkey_all(URI_KEY)
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
        if let Some(entries) = self.0 {
            let (root, _) = current_user
                .create_subkey(URI_KEY)
                .map_err(|e| e.to_string())?;
            for (path, values) in entries {
                let (key, _) = root.create_subkey(path).map_err(|e| e.to_string())?;
                for (name, value) in values {
                    key.set_raw_value(name, &value).map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }
}

fn read_key(
    key: &RegKey,
    path: &str,
    entries: &mut Vec<(String, Vec<(String, RegValue)>)>,
    depth: usize,
) -> Result<(), String> {
    if depth > 8 || entries.len() >= 128 {
        return Err("现有登录协议注册结构过大，未启动组件".into());
    }
    let values = key
        .enum_values()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    entries.push((path.into(), values));
    for name in key.enum_keys() {
        let name = name.map_err(|e| e.to_string())?;
        let child = key.open_subkey(&name).map_err(|e| e.to_string())?;
        let child_path = if path.is_empty() {
            name
        } else {
            format!(r"{path}\{name}")
        };
        read_key(&child, &child_path, entries, depth + 1)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_selection_accepts_the_official_folder_and_bat_entry() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        fs::create_dir_all(root.join("python-embed")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("assets")).unwrap();
        for name in [
            "python-embed/python.exe",
            "src/main.pyc",
            "src/buildinfo.pyc",
            "点我启动工具.bat",
        ] {
            fs::write(root.join(name), b"fixture").unwrap();
        }
        assert_eq!(
            runtime_from_selection(root.to_path_buf()).unwrap().python,
            root.join("python-embed/python.exe")
        );
        assert_eq!(
            runtime_from_selection(root.join("点我启动工具.bat"))
                .unwrap()
                .script,
            root.join("src/main.pyc")
        );
        assert!(runtime_from_selection(root.join("installer.exe")).is_err());
    }

    #[test]
    fn dedicated_config_preserves_accounts_and_disables_automatic_game_launch() {
        let directory = tempfile::tempdir().unwrap();
        prepare_data(directory.path()).unwrap();
        let path = directory.path().join("idv-login/config.json");
        fs::write(&path, serde_json::to_vec(&json!({
            "accounts": {"fixture": {"display_name": "test account"}},
            "game_settings": {"h55": {"should_auto_start": true}},
            "game_installation_settings_v1": {"h55": {"installations": [{"settings": {"auto_start": true}}]}}
        })).unwrap()).unwrap();
        prepare_data(directory.path()).unwrap();
        let config: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            config["accounts"]["fixture"]["display_name"],
            "test account"
        );
        assert_eq!(config["proxy_mode"], "process");
        assert_eq!(config["game_settings"]["h55"]["should_auto_start"], false);
        assert_eq!(
            config["game_installation_settings_v1"]["h55"]["installations"][0]["settings"]
                ["auto_start"],
            false
        );
        assert_eq!(config["v6.3.2-stable_first_use"], false);
        let recovery = br#"{"_SAVED_PROXY_ENV":{"HTTP_PROXY":"fixture"}}"#;
        fs::write(&path, recovery).unwrap();
        assert!(prepare_data(directory.path()).is_err());
        assert_eq!(fs::read(&path).unwrap(), recovery);
        fs::write(&path, b"invalid config").unwrap();
        assert!(prepare_data(directory.path()).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"invalid config");
    }
}
