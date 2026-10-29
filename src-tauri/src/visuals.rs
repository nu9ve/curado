use curado_audio::{EngineHandle, VISUAL_SAMPLES};
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
use std::{io::Write, process::Child};
pub struct Visuals {
    child: Option<Child>,
    spectrum: [f32; 24],
}
impl Visuals {
    pub fn new() -> Self {
        Self {
            child: None,
            spectrum: [0.; 24],
        }
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
            let bundle = cache.join("curado visuals.app");
            let contents = bundle.join("Contents");
            let executable_dir = contents.join("MacOS");
            std::fs::create_dir_all(&executable_dir).map_err(|e| e.to_string())?;
            std::fs::write(contents.join("Info.plist"), r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict><key>CFBundleExecutable</key><string>curado-visuals</string><key>CFBundleIdentifier</key><string>studio.curado.visuals</string><key>CFBundleName</key><string>curado visuals</string><key>CFBundlePackageType</key><string>APPL</string><key>NSPrincipalClass</key><string>NSApplication</string><key>LSMinimumSystemVersion</key><string>12.0</string></dict></plist>"#).map_err(|e| e.to_string())?;
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
    pub fn send(&mut self, handle: &EngineHandle, theme: &crate::preferences::Theme) {
        if let Some(child) = self.child.as_mut() {
            if child.try_wait().ok().flatten().is_some() {
                self.child = None;
            }
        }
        if let Some(child) = self.child.as_mut() {
            if let Some((samples, rate)) = handle.visual_samples() {
                let bins = spectrum(&samples, rate);
                for (current, target) in self.spectrum.iter_mut().zip(bins) {
                    let smoothing = if target > *current { 0.65 } else { 0.22 };
                    *current += (target - *current) * smoothing;
                }
            }
            let frame = handle.snapshot();
            if let Some(stdin) = child.stdin.as_mut() {
                let data = serde_json::json!({"rms": frame.rms, "peak": frame.peak, "waveform": frame.waveform, "spectrum": self.spectrum, "accent":theme.rgb()});
                // This runs on a dedicated visual worker; a blocked pipe cannot delay audio or MIDI.
                if writeln!(stdin, "{data}").is_err() {
                    let _ = child.kill();
                    self.child = None;
                }
            }
        } else {
            handle.stop_visual_capture();
            self.spectrum = [0.; 24];
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

/// Hann-windowed radix-2 FFT, used only by the visual worker. Log bins are
/// approximate display magnitudes; this is not calibrated audio metering.
fn spectrum(samples: &[f32; VISUAL_SAMPLES], sample_rate: u32) -> [f32; 24] {
    let n = VISUAL_SAMPLES;
    let mut real = [0.0_f32; VISUAL_SAMPLES];
    let mut imag = [0.0_f32; VISUAL_SAMPLES];
    let bits = n.trailing_zeros();
    for (i, sample) in samples.iter().enumerate() {
        let window = 0.5 - 0.5 * (2. * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos();
        let reversed = i.reverse_bits() >> (usize::BITS - bits);
        real[reversed] = sample * window;
    }
    let mut size = 2;
    while size <= n {
        let angle = -2. * std::f32::consts::PI / size as f32;
        let (step_i, step_r) = angle.sin_cos();
        for base in (0..n).step_by(size) {
            let (mut wr, mut wi) = (1., 0.);
            for j in 0..size / 2 {
                let a = base + j;
                let b = a + size / 2;
                let tr = wr * real[b] - wi * imag[b];
                let ti = wr * imag[b] + wi * real[b];
                real[b] = real[a] - tr;
                imag[b] = imag[a] - ti;
                real[a] += tr;
                imag[a] += ti;
                (wr, wi) = (wr * step_r - wi * step_i, wr * step_i + wi * step_r);
            }
        }
        size *= 2;
    }
    let high = 16000.0_f32.min(sample_rate as f32 * 0.5);
    std::array::from_fn(|bin| {
        let low_hz = 40. * (high / 40.).powf(bin as f32 / 24.);
        let high_hz = 40. * (high / 40.).powf((bin + 1) as f32 / 24.);
        let first = ((low_hz * n as f32 / sample_rate as f32).floor() as usize).clamp(1, n / 2 - 1);
        let last =
            ((high_hz * n as f32 / sample_rate as f32).ceil() as usize).clamp(first + 1, n / 2);
        let magnitude = (first..last)
            .map(|i| real[i].hypot(imag[i]) * 4. / n as f32)
            .fold(0., f32::max);
        (magnitude * 2.).sqrt().clamp(0., 1.)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spectrum_tracks_bass_mid_and_treble_and_silence() {
        assert_eq!(spectrum(&[0.; VISUAL_SAMPLES], 48000), [0.; 24]);
        for frequency in [100., 1000., 8000.] {
            let samples = std::array::from_fn(|i| {
                (2. * std::f32::consts::PI * frequency * i as f32 / 48000.).sin() * 0.5
            });
            let bins = spectrum(&samples, 48000);
            let peak = bins
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .unwrap()
                .0;
            let center = 40. * (16000.0_f32 / 40.).powf((peak as f32 + 0.5) / 24.);
            assert!(
                (center / frequency).ln().abs() < 0.45,
                "{frequency} Hz mapped to {center} Hz"
            );
            assert!(bins[peak] > 0.7);
        }
    }
}
