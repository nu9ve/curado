//! Offline waveform analysis. Runs in the decode worker, never in the audio callback.
//!
//! Each bin stores `[peak, low, mid, high]` as linear amplitude quantized to `u8`.
//! Bands use the same one-pole splits as the deck EQ (220 Hz / 2600 Hz), so the
//! colors on screen correspond to what the EQ knobs actually cut or boost.

/// Bins per second of audio. 20 ms per bin: detailed enough for a zoomed deck view.
pub const WAVEFORM_RATE: u32 = 50;
/// Wire header: format version, bins per second, reserved.
pub const WAVEFORM_HEADER: [u8; 4] = [1, WAVEFORM_RATE as u8, 0, 0];

pub struct WaveformBuilder {
    frames_per_bin: usize,
    count: usize,
    low_a: f32,
    high_a: f32,
    low: f32,
    split: f32,
    max: [f32; 4],
    bins: Vec<[u8; 4]>,
}

impl WaveformBuilder {
    pub fn new(sample_rate: u32) -> Self {
        let sr = sample_rate.max(1);
        Self {
            frames_per_bin: (sr / WAVEFORM_RATE).max(1) as usize,
            count: 0,
            low_a: coeff(220., sr),
            high_a: coeff(2600., sr),
            low: 0.,
            split: 0.,
            max: [0.; 4],
            bins: Vec::new(),
        }
    }
    pub fn push(&mut self, frame: [f32; 2]) {
        let mono = (frame[0] + frame[1]) * 0.5;
        self.low += self.low_a * (mono - self.low);
        self.split += self.high_a * (mono - self.split);
        let bands = [
            frame[0].abs().max(frame[1].abs()),
            self.low.abs(),
            (self.split - self.low).abs(),
            (mono - self.split).abs(),
        ];
        for (max, value) in self.max.iter_mut().zip(bands) {
            *max = max.max(value);
        }
        self.count += 1;
        if self.count == self.frames_per_bin {
            self.flush();
        }
    }
    fn flush(&mut self) {
        self.bins.push(self.max.map(quantize));
        self.max = [0.; 4];
        self.count = 0;
    }
    pub fn finish(mut self) -> Vec<[u8; 4]> {
        if self.count > 0 {
            self.flush();
        }
        self.bins
    }
}

fn quantize(value: f32) -> u8 {
    (value.clamp(0., 1.) * 255.).round() as u8
}

fn coeff(hz: f32, sr: u32) -> f32 {
    1. - (-std::f32::consts::TAU * hz / sr as f32).exp()
}

/// Compact binary form for SQLite and IPC: header followed by 4 bytes per bin.
pub fn encode(bins: &[[u8; 4]]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(4 + bins.len() * 4);
    bytes.extend_from_slice(&WAVEFORM_HEADER);
    for bin in bins {
        bytes.extend_from_slice(bin);
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tone(hz: f32, sr: u32, seconds: f32) -> Vec<u8> {
        let mut b = WaveformBuilder::new(sr);
        for i in 0..(sr as f32 * seconds) as usize {
            let x = (i as f32 * hz * std::f32::consts::TAU / sr as f32).sin() * 0.5;
            b.push([x, x]);
        }
        encode(&b.finish())
    }
    #[test]
    fn bins_follow_fixed_rate_and_bands_separate_frequencies() {
        let bass = tone(60., 48000, 1.);
        let hats = tone(9000., 44100, 1.);
        assert_eq!(&bass[..4], &WAVEFORM_HEADER);
        assert_eq!((bass.len() - 4) / 4, 50);
        assert_eq!((hats.len() - 4) / 4, 50);
        // Skip the first bins while the filters settle.
        fn bin(data: &[u8], i: usize) -> &[u8] {
            &data[4 + i * 4..8 + i * 4]
        }
        let (b, h) = (bin(&bass, 25), bin(&hats, 25));
        assert!((b[0] as i32 - 128).abs() <= 2, "peak follows amplitude");
        assert!(b[1] > b[3] * 4, "60 Hz lives in the low band: {b:?}");
        assert!(h[3] > h[1] * 4, "9 kHz lives in the high band: {h:?}");
    }
}
