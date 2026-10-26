use curado_audio::Snapshot;
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
use std::{io::Write, process::Child};
pub struct Visuals {
    child: Option<Child>,
}
impl Visuals {
    pub fn new() -> Self {
        Self { child: None }
    }
    pub fn open(&mut self, cache: &std::path::Path) -> Result<(), String> {
        if let Some(child) = self.child.as_mut() {
            if child.try_wait().map_err(|e| e.to_string())?.is_none() {
                return Ok(());
            }
        }
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::PermissionsExt;
            let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/curado-visuals"));
            let bundle = cache.join("Curado Visuals.app");
            let contents = bundle.join("Contents");
            let executable_dir = contents.join("MacOS");
            std::fs::create_dir_all(&executable_dir).map_err(|e| e.to_string())?;
            std::fs::write(contents.join("Info.plist"), r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict><key>CFBundleExecutable</key><string>curado-visuals</string><key>CFBundleIdentifier</key><string>studio.curado.visuals</string><key>CFBundleName</key><string>Curado Visuals</string><key>CFBundlePackageType</key><string>APPL</string><key>NSPrincipalClass</key><string>NSApplication</string><key>LSMinimumSystemVersion</key><string>12.0</string></dict></plist>"#).map_err(|e| e.to_string())?;
            let path = executable_dir.join("curado-visuals");
            std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
            self.child = Some(
                Command::new(path)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .spawn()
                    .map_err(|e| e.to_string())?,
            );
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = cache;
            Err("El renderizador Metal requiere macOS. Usa el visualizador integrado.".into())
        }
    }
    pub fn send(&mut self, frame: &Snapshot, theme: &crate::preferences::Theme) {
        if let Some(child) = self.child.as_mut() {
            if child.try_wait().ok().flatten().is_some() {
                self.child = None;
                return;
            }
            if let Some(stdin) = child.stdin.as_mut() {
                let data = serde_json::json!({"rms": frame.rms, "peak": frame.peak, "waveform": frame.waveform,"accent":theme.rgb()});
                // This runs on a dedicated visual worker; a blocked pipe cannot delay audio or MIDI.
                if writeln!(stdin, "{data}").is_err() {
                    let _ = child.kill();
                    self.child = None;
                }
            }
        }
    }
}
impl Drop for Visuals {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
