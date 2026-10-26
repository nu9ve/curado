use std::{
    io::Write,
    path::{Path, PathBuf},
};
pub fn create(dir: &Path) -> Result<Vec<PathBuf>, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    [
        ("01 - Orbita - 120.wav", 120., 110.),
        ("02 - Litoral - 124.wav", 124., 146.83),
        ("03 - Pulso - kick.wav", 0., 55.),
        ("04 - Niebla - clap.wav", 0., 880.),
        ("05 - Destello - hat.wav", 0., 4400.),
        ("06 - Marea - chord.wav", 0., 220.),
    ]
    .iter()
    .enumerate()
    .map(|(index, (name, bpm, root))| {
        let path = dir.join(name);
        if path.exists() {
            return Ok(path);
        }
        let sr = 48000u32;
        let length = if *bpm > 0. { 32. } else { 1. };
        let n = (length * sr as f64) as u32;
        let size = n * 4;
        let mut f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
        f.write_all(b"RIFF")
            .and_then(|_| f.write_all(&(36 + size).to_le_bytes()))
            .and_then(|_| f.write_all(b"WAVEfmt "))
            .and_then(|_| f.write_all(&16u32.to_le_bytes()))
            .and_then(|_| f.write_all(&1u16.to_le_bytes()))
            .and_then(|_| f.write_all(&2u16.to_le_bytes()))
            .and_then(|_| f.write_all(&sr.to_le_bytes()))
            .and_then(|_| f.write_all(&(sr * 4).to_le_bytes()))
            .and_then(|_| f.write_all(&4u16.to_le_bytes()))
            .and_then(|_| f.write_all(&16u16.to_le_bytes()))
            .and_then(|_| f.write_all(b"data"))
            .and_then(|_| f.write_all(&size.to_le_bytes()))
            .map_err(|e| e.to_string())?;
        let mut pcm = Vec::with_capacity(size as usize);
        for i in 0..n {
            let t = i as f64 / sr as f64;
            let phase = if *bpm > 0. {
                (t * bpm / 60.).fract() * 60. / bpm
            } else {
                t
            };
            let kick = (std::f64::consts::TAU * (52. * phase + 8. * (1. - (-phase * 30.).exp())))
                .sin()
                * (-phase * 18.).exp();
            let tone = (std::f64::consts::TAU * root * t).sin() * 0.22
                + (std::f64::consts::TAU * root * 1.5 * t).sin() * 0.1;
            let sample = if *bpm > 0. {
                kick * 0.5 + tone * (t * 0.5).sin().abs()
            } else if index == 2 {
                kick
            } else {
                tone * (-t * if index == 4 { 30. } else { 6. }).exp()
            };
            let fade = (t * 100.).min(1.) * ((length - t) * 50.).min(1.);
            let value = (sample * fade * 0.65 * i16::MAX as f64) as i16;
            pcm.extend_from_slice(&value.to_le_bytes());
            pcm.extend_from_slice(&value.to_le_bytes());
        }
        f.write_all(&pcm).map_err(|e| e.to_string())?;
        Ok(path)
    })
    .collect()
}
