fn main() {
    // Native Tauri tests also link TaskDialogIndirect, which requires Common Controls v6.
    // Unlike the application's resource manifest, this does not request elevation.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
    }
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
