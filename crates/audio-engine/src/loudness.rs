//! Integrated loudness after ITU-R BS.1770 with its gating, for matching levels between tracks.
//! Runs on the loading worker.
use crate::dsp::{Biquad, BiquadCoef};

const BLOCK_SECONDS: f64 = 0.4;
const STEP_SECONDS: f64 = 0.1;
const ABSOLUTE_GATE: f64 = -70.;

fn lufs(energy: f64) -> f64 {
    -0.691 + 10. * energy.max(1e-12).log10()
}

/// LUFS, or nothing for a track with no blocks above the gate.
pub fn integrated(samples: &[[f32; 2]], sample_rate: u32) -> Option<f32> {
    let sr = sample_rate as f64;
    // The standard's K-weighting: a shelf for the head, then a high-pass.
    let shelf = BiquadCoef::high_shelf(1681.97, 3.9998, 0.7072, sr);
    let high_pass = BiquadCoef::high_pass(38.13, 0.5003, sr);
    let mut filters = [[Biquad::default(); 2]; 2];
    let step = (sr * STEP_SECONDS) as usize;
    let per_block = (BLOCK_SECONDS / STEP_SECONDS) as usize;
    if step == 0 {
        return None;
    }
    let mut steps = Vec::with_capacity(samples.len() / step + 1);
    for chunk in samples.chunks(step) {
        let mut energy = 0.;
        for frame in chunk {
            for (c, filter) in filters.iter_mut().enumerate() {
                let shelved = filter[0].tick(&shelf, frame[c] as f64);
                let weighted = filter[1].tick(&high_pass, shelved);
                energy += weighted * weighted;
            }
        }
        steps.push(energy / chunk.len() as f64);
    }
    let blocks: Vec<f64> = steps
        .windows(per_block)
        .map(|window| window.iter().sum::<f64>() / per_block as f64)
        .filter(|energy| lufs(*energy) > ABSOLUTE_GATE)
        .collect();
    if blocks.is_empty() {
        return None;
    }
    let mean = blocks.iter().sum::<f64>() / blocks.len() as f64;
    let relative = lufs(mean) - 10.;
    let kept: Vec<f64> = blocks
        .into_iter()
        .filter(|energy| lufs(*energy) > relative)
        .collect();
    if kept.is_empty() {
        return None;
    }
    Some(lufs(kept.iter().sum::<f64>() / kept.len() as f64) as f32)
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
