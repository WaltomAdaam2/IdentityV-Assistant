fn main() {
    // Development and bridge tests use Python source, so a fresh checkout does
    // not need to freeze the engine just to satisfy the resource directory map.
    if tauri_build::is_dev() || std::env::var("PROFILE").as_deref() == Ok("debug") {
        std::fs::create_dir_all("../engine/dist/identityv-engine")
            .expect("failed to create engine build output directory");
    }
    let windows = tauri_build::WindowsAttributes::new()
        .app_manifest(include_str!("windows-app-manifest.xml"));
    let attributes = tauri_build::Attributes::new().windows_attributes(windows);
    tauri_build::try_build(attributes).expect("failed to run Tauri build script");
}
