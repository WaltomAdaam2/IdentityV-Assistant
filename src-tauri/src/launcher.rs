use std::path::{Path, PathBuf};

#[cfg(windows)]
mod component;
#[cfg(windows)]
mod install;
#[cfg(windows)]
pub(crate) mod manager;
#[cfg(windows)]
mod runtime;
#[cfg(windows)]
pub use manager::Launcher;

#[tauri::command]
pub fn locate_game(executable_path: Option<String>) -> Result<String, String> {
    if let Some(path) = executable_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        let path = Path::new(path);
        if !valid_game(path) {
            return Err("请选择第五人格的 dwrg.exe，不能选择发烧平台或安装程序".into());
        }
        return Ok(path.to_string_lossy().into_owned());
    }
    if let Some(path) = installed_game().filter(|path| valid_game(path)) {
        return Ok(path.to_string_lossy().into_owned());
    }
    Err("未找到第五人格，请选择 dwrg.exe".into())
}

fn valid_game(path: &Path) -> bool {
    path.is_absolute()
        && path.is_file()
        && path
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("dwrg.exe"))
}

#[cfg(windows)]
fn launch_direct(path: &Path, proxy_port: u16) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    if !valid_game(path) || proxy_port == 0 {
        return Err("游戏程序或登录代理端口无效".into());
    }
    let proxy = format!("http://127.0.0.1:{proxy_port}");
    std::process::Command::new(path)
        .args(["--start_from_launcher=1", "--is_multi_start"])
        .current_dir(path.parent().ok_or("游戏目录无效")?)
        .env("HTTP_PROXY", &proxy)
        .env("HTTPS_PROXY", &proxy)
        .env("NO_PROXY", "localhost,127.0.0.1,::1")
        .env_remove("ALL_PROXY")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x00000008 | 0x00000200)
        .spawn()
        .map_err(|error| format!("启动第五人格失败：{error}"))?;
    Ok(())
}

#[cfg(windows)]
fn installed_game() -> Option<PathBuf> {
    use winreg::{
        enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY},
        RegKey,
    };
    // Distribution 73 is the standard Chinese client; 134 is the separate test server.
    let key = r"Software\FeverGames\FeverGamesInstaller\game\73";
    for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
        let Ok(game) =
            RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(key, KEY_READ | view)
        else {
            continue;
        };
        let Ok(directory) = game.get_value::<String, _>("LastInstallPath") else {
            continue;
        };
        let path = PathBuf::from(directory).join("dwrg.exe");
        if valid_game(&path) {
            return Some(path);
        }
    }
    None
}

#[cfg(not(windows))]
fn installed_game() -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn direct_launch_uses_only_the_game_with_standalone_arguments_and_child_proxy() {
        let directory = tempfile::tempdir().unwrap();
        let game = directory.path().join("dwrg.exe");
        let compile = std::process::Command::new("rustc")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/game_launch.rs"))
            .arg("-o")
            .arg(&game)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let before = std::env::var_os("HTTP_PROXY");
        launch_direct(&game, 54321).unwrap();
        let evidence = directory.path().join("launch-evidence.txt");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !evidence.is_file() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let output = std::fs::read_to_string(evidence).unwrap();
        let lines: Vec<_> = output.lines().collect();
        assert_eq!(lines[0], game.to_string_lossy());
        assert_eq!(lines[1], directory.path().to_string_lossy());
        assert_eq!(
            &lines[2..],
            &[
                "http://127.0.0.1:54321",
                "http://127.0.0.1:54321",
                "--start_from_launcher=1",
                "--is_multi_start"
            ]
        );
        assert!(
            std::env::var_os("HTTP_PROXY") == before,
            "parent proxy environment changed"
        );
    }

    #[test]
    fn locate_accepts_the_game_but_rejects_the_platform_executable() {
        let directory = tempfile::tempdir().unwrap();
        let game = directory.path().join("dwrg.exe");
        let platform = directory.path().join("Fever.exe");
        std::fs::write(&game, b"fixture").unwrap();
        std::fs::write(&platform, b"fixture").unwrap();
        assert_eq!(
            locate_game(Some(game.to_string_lossy().into_owned())).unwrap(),
            game.to_string_lossy()
        );
        assert!(locate_game(Some(platform.to_string_lossy().into_owned())).is_err());
        assert!(locate_game(Some(
            directory
                .path()
                .join("missing.exe")
                .to_string_lossy()
                .into_owned()
        ))
        .is_err());
    }
}
