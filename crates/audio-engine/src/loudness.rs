//! Integrated loudness after ITU-R BS.1770 with its gating, for matching levels between tracks.
//! Runs on the loading worker. The measurement is the suite's `nu9ve-loudness`, so every
//! nu9ve app reads the same number from the same PCM.

/// LUFS, or nothing for a track with no blocks above the gate or an unsupported sample rate.
pub fn integrated(samples: &[[f32; 2]], sample_rate: u32) -> Option<f32> {
    let mut meter = nu9ve_loudness::Meter::new(sample_rate, 2).ok()?;
    meter.add_interleaved(samples.as_flattened()).ok()?;
    meter.report().integrated_lufs.map(|lufs| lufs as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(amplitude: f32, seconds: f32, sr: u32) -> Vec<[f32; 2]> {
        (0..(seconds * sr as f32) as usize)
            .map(|i| {
                let v = (std::f32::consts::TAU * 997. * i as f32 / sr as f32).sin() * amplitude;
                [v, v]
            })
            .collect()
    }

    #[test]
    fn a_full_scale_tone_reads_as_the_standard_says() {
        // BS.1770: 997 Hz at 0 dBFS on both channels is about -0 LUFS (each is -3.01).
        for sr in [44100, 48000, 96000] {
            let level = integrated(&tone(1., 3., sr), sr).unwrap();
            assert!(level.abs() < 0.3, "{level} LUFS at {sr}");
        }
    }

    #[test]
    fn level_follows_gain_and_silence_does_not_count() {
        let sr = 48000;
        let loud = integrated(&tone(0.5, 3., sr), sr).unwrap();
        let quiet = integrated(&tone(0.05, 3., sr), sr).unwrap();
        assert!((loud - quiet - 20.).abs() < 0.1);
        let mut gapped = tone(0.5, 3., sr);
        gapped.extend(vec![[0.; 2]; sr as usize * 30]);
        let level = integrated(&gapped, sr).unwrap();
        assert!((level - loud).abs() < 0.5, "{level} against {loud}");
        assert_eq!(integrated(&vec![[0.; 2]; sr as usize * 2], sr), None);
        assert_eq!(integrated(&[], sr), None);
    }
}
