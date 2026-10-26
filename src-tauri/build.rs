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
    tauri_build::build();
}
