mod app_commands;
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let arch = if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("aarch64") {
            "arm64"
        } else {
            "x86_64"
        };
        let target = format!("{arch}-apple-macosx12.0");
        let status = std::process::Command::new("xcrun")
            .args([
                "swiftc",
                "-O",
                "-framework",
                "AppKit",
                "-framework",
                "MetalKit",
                "native/CuradoVisuals.swift",
                "-o",
            ])
            .arg(out.join("curado-visuals"))
            .arg("-module-cache-path")
            .arg(out.join("swift-module-cache"))
            .args(["-target", &target])
            .status()
            .expect("Xcode Swift compiler is required for Metal visuals");
        assert!(status.success(), "Metal visualizer compilation failed");
        println!("cargo:rerun-if-changed=native/CuradoVisuals.swift");
    }
    println!("cargo:rerun-if-changed=app_commands.rs");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(app_commands::COMMANDS)),
    )
    .expect("Application command permissions must compile");
}
