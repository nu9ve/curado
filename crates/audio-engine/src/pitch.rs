//! Real-time key lock and transposition for a music deck.
//!
//! Runs in the audio callback, which is safe here because everything Signalsmith Stretch needs
//! is allocated when the voice is created: `process`, `seek`, `reset` and the transpose setters
//! allocate nothing (`allocation_tests` audits this, including C++, on macOS). Its cost is about
//! 1.3 % of a core per voice, with peaks of about 0.25 ms per chunk and 0.5 ms per start.
//!
//! The processor needs audio from about 85 ms ahead of what it outputs. PCM is already in memory,
//! so the voice simply reads that far ahead of the deck's position and nothing is delayed.
use signalsmith_stretch::Stretch;

/// Output frames rendered at once. Small enough that a start lands within about 1 ms.
const CHUNK: usize = 64;
/// Bound on input per chunk: 1.5× tempo on a file at more than four times the output rate.
const MAX_INPUT: usize = 512;
/// Window at 48 kHz. Measured on kicks and tones, 2048 kept no more of the attack and made
/// transposition errors twice as wide, and windows above this softened kicks again.
const BLOCK_SECONDS: f64 = 4096. / 48000.;
const HOPS_PER_BLOCK: usize = 8;

pub(crate) struct PitchVoice {
    stretch: Stretch,
    input: Box<[f32]>,
    output: Box<[f32]>,
    history: Box<[f32]>,
    /// Next output frame to hand out; `CHUNK` means a new chunk is needed.
    cursor: usize,
    /// Next source position to feed. Can be negative just after a start near time zero.
    read: f64,
    /// Source frames per fed frame. The file is resampled before processing, because a
    /// transposition is less exact than a tempo change: key lock alone is exact in pitch.
    source_step: f64,
    input_latency: f64,
    output_latency: f64,
    /// Hidden output before a start: half a window is enough for overlap to be complete, so
    /// the first audible sample is already at full level.
    warm_frames: usize,
}
impl PitchVoice {
    pub fn new(sample_rate: u32) -> Self {
        let block = ((sample_rate as f64 * BLOCK_SECONDS).round() as usize).max(HOPS_PER_BLOCK);
        let interval = block / HOPS_PER_BLOCK;
        let stretch = Stretch::new(2, block, interval);
        let input_latency = stretch.input_latency() as f64;
        let output_latency = stretch.output_latency() as f64;
        Self {
            stretch,
            input: vec![0.; MAX_INPUT * 2].into_boxed_slice(),
            output: vec![0.; CHUNK * 2].into_boxed_slice(),
            history: vec![0.; (block + interval) * 2].into_boxed_slice(),
            cursor: CHUNK,
            read: 0.,
            source_step: 1.,
            input_latency,
            output_latency,
            warm_frames: block / 2,
        }
    }
    pub fn set_transpose(&mut self, factor: f32) {
        self.stretch.set_transpose_factor(factor, None);
    }
    /// Silently moves to `position`, discarding whatever the voice was doing. `step` is the
    /// deck's source frames per output frame and `source_step` the part that is sample rate.
    pub fn start(
        &mut self,
        samples: &[[f32; 2]],
        position: f64,
        step: f64,
        source_step: f64,
        transpose: f32,
    ) {
        self.stretch.reset();
        self.set_transpose(transpose);
        self.source_step = source_step;
        let early = position - self.warm_frames as f64 * step;
        self.read = early + self.lead(step);
        let frames = self.history.len() / 2;
        for (frame, value) in self.history.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            *value = frame_at(samples, self.read - (frames - frame) as f64 * source_step);
        }
        self.stretch.seek(&self.history, step / source_step);
        let mut done = 0;
        while done < self.warm_frames {
            self.render(samples, early + done as f64 * step, step);
            done += CHUNK;
        }
        self.cursor = CHUNK;
    }
    /// The output frame that sounds at `position`.
    #[inline]
    pub fn next(&mut self, samples: &[[f32; 2]], position: f64, step: f64) -> [f32; 2] {
        if self.cursor == CHUNK {
            self.render(samples, position, step);
            self.cursor = 0;
        }
        let index = self.cursor * 2;
        self.cursor += 1;
        [self.output[index], self.output[index + 1]]
    }
    /// How far ahead of the audible position the input has to be, in source frames.
    fn lead(&self, step: f64) -> f64 {
        self.input_latency * self.source_step + self.output_latency * step
    }
    fn render(&mut self, samples: &[[f32; 2]], position: f64, step: f64) {
        let target = position + CHUNK as f64 * step + self.lead(step);
        let frames = (((target - self.read) / self.source_step).floor() as i64)
            .clamp(0, MAX_INPUT as i64) as usize;
        for (frame, value) in self.input[..frames * 2]
            .as_chunks_mut::<2>()
            .0
            .iter_mut()
            .enumerate()
        {
            *value = frame_at(samples, self.read + frame as f64 * self.source_step);
        }
        self.read += frames as f64 * self.source_step;
        self.stretch
            .process(&self.input[..frames * 2], &mut self.output[..]);
    }
}
#[inline]
fn frame_at(samples: &[[f32; 2]], position: f64) -> [f32; 2] {
    if position < 0. {
        return [0.; 2];
    }
    crate::read(samples, position)
}

#[cfg(test)]
mod tests {
    use crate::{decode::Clip, engine, limiter::Limiter, Control, EngineHandle, Renderer};
    use std::{f64::consts::TAU, sync::Arc};

    const SR: u32 = 48000;
    const GAIN: f32 = 0.8 * 0.75;

    fn clip(source_rate: u32, samples: Vec<[f32; 2]>) -> Arc<Clip> {
        Arc::new(Clip {
            id: 1,
            cues: Default::default(),
            samples,
            sample_rate: source_rate,
            waveform: vec![],
            title: None,
            artist: None,
            music: Default::default(),
        })
    }
    fn sine(source_rate: u32, hz: f64, seconds: usize, amplitude: f32) -> Vec<[f32; 2]> {
        (0..source_rate as usize * seconds)
            .map(|i| {
                let x = (TAU * hz * i as f64 / source_rate as f64).sin() as f32 * amplitude;
                [x, x * 0.5]
            })
            .collect()
    }
    fn render(r: &mut Renderer, frames: usize) -> Vec<[f32; 2]> {
        let mut out = vec![0_f32; frames * 2];
        for block in out.chunks_mut(512) {
            r.render(block, 2);
        }
        out.as_chunks::<2>().0.to_vec()
    }
    fn settled(source: Arc<Clip>) -> (EngineHandle, Renderer) {
        let (h, mut r) = engine(SR);
        h.control(0, Control::Crossfader(0.)).unwrap();
        render(&mut r, SR as usize);
        h.load_and_play(0, source).unwrap();
        (h, r)
    }
    /// Frequency from rising zero crossings with interpolation, immune to steady gain errors.
    fn frequency(output: &[[f32; 2]]) -> f64 {
        let crossings: Vec<f64> = output
            .windows(2)
            .enumerate()
            .filter(|(_, w)| w[0][0] <= 0. && w[1][0] > 0.)
            .map(|(i, w)| i as f64 + (-w[0][0] / (w[1][0] - w[0][0])) as f64)
            .collect();
        let periods = (crossings.len() - 1) as f64;
        periods / (crossings[crossings.len() - 1] - crossings[0]) * SR as f64
    }
    fn largest_step(output: &[[f32; 2]]) -> f32 {
        output
            .windows(2)
            .fold(0., |max, w| max.max((w[1][0] - w[0][0]).abs()))
    }
    fn rms(output: &[[f32; 2]]) -> f64 {
        (output.iter().map(|f| (f[0] as f64).powi(2)).sum::<f64>() / output.len() as f64).sqrt()
    }

    #[test]
    fn key_lock_keeps_pitch_at_any_tempo_immediately() {
        for rate in [0.8_f32, 0.94, 1.03, 1.06, 1.25] {
            let (h, mut r) = settled(clip(SR, sine(SR, 440., 6, 0.5)));
            render(&mut r, 2048);
            h.control(0, Control::Rate(rate)).unwrap();
            render(&mut r, 256);
            let s = h.snapshot().slots[0].clone();
            assert_eq!(s.rate, rate, "the new tempo applies in the same block");
            let before = s.position;
            let output = render(&mut r, 24000);
            let hz = frequency(&output[4800..]);
            assert!((hz - 440.).abs() < 0.5, "rate {rate}: {hz} Hz");
            let moved = h.snapshot().slots[0].position - before;
            assert!((moved - 0.5 * rate as f64).abs() < 1e-5, "{moved}");
            let expected = 0.5 * GAIN as f64 / 2_f64.sqrt();
            assert!((rms(&output[4800..]) / expected - 1.).abs() < 0.08);
        }
    }

    #[test]
    fn transposition_and_varispeed_combine_as_documented() {
        let semitone = |n: f64| 2_f64.powf(n / 12.);
        for (rate, semitones, lock, expected) in [
            (1., 12, true, 880.),
            (1., -12, true, 220.),
            (1., 3, true, 440. * semitone(3.)),
            (1.1, 3, true, 440. * semitone(3.)),
            (1.1, 0, false, 484.),
            (1.1, 3, false, 484. * semitone(3.)),
        ] {
            let (h, mut r) = settled(clip(SR, sine(SR, 440., 6, 0.5)));
            h.control(0, Control::KeyLock(lock)).unwrap();
            h.control(0, Control::Rate(rate)).unwrap();
            h.control(0, Control::Transpose(semitones)).unwrap();
            render(&mut r, 9600);
            let hz = frequency(&render(&mut r, 24000));
            assert!(
                (hz / expected - 1.).abs() < 0.005,
                "rate {rate} st {semitones} lock {lock}: {hz} instead of {expected}"
            );
        }
    }

    #[test]
    fn a_file_at_another_sample_rate_keeps_its_pitch() {
        let (h, mut r) = settled(clip(44100, sine(44100, 440., 6, 0.5)));
        h.control(0, Control::Rate(1.06)).unwrap();
        render(&mut r, 9600);
        let before = h.snapshot().slots[0].position;
        let hz = frequency(&render(&mut r, 24000));
        assert!((hz - 440.).abs() < 0.5, "{hz}");
        let moved = h.snapshot().slots[0].position - before;
        assert!((moved - 0.53).abs() < 1e-4, "{moved}");
    }

    #[test]
    fn stereo_balance_survives_the_voice() {
        let (h, mut r) = settled(clip(SR, sine(SR, 440., 6, 0.5)));
        h.control(0, Control::Rate(1.06)).unwrap();
        render(&mut r, 9600);
        let output = render(&mut r, 9600);
        let left = rms(&output);
        let right = (output.iter().map(|f| (f[1] as f64).powi(2)).sum::<f64>()
            / output.len() as f64)
            .sqrt();
        assert!((right / left - 0.5).abs() < 0.01, "{}", right / left);
    }

    fn clicks(period: usize, count: usize) -> Vec<[f32; 2]> {
        let mut samples = vec![[0.; 2]; period * (count + 1)];
        for beat in 1..=count {
            for i in 0..2000 {
                let t = i as f64 / SR as f64;
                let value = ((TAU * 90. * t).sin() * (-t / 0.02).exp()) as f32 * 0.8;
                samples[beat * period + i] = [value; 2];
            }
        }
        samples
    }

    #[test]
    fn attacks_stay_where_the_engine_says_they_are() {
        let period = 24000;
        let source = clicks(period, 40);
        for rate in [0.94_f32, 1.06, 1.2] {
            let (h, mut r) = settled(clip(SR, source.clone()));
            h.control(0, Control::Rate(rate)).unwrap();
            render(&mut r, 4800);
            let start = h.snapshot().slots[0].position * SR as f64;
            let output = render(&mut r, SR as usize * 6);
            let latency = Limiter::new(SR).latency() as f64;
            let threshold = 0.1 * GAIN;
            let mut late = Vec::new();
            let mut beat = (start / period as f64).ceil() as usize;
            while beat <= 30 {
                let expected = (beat as f64 * period as f64 - start) / rate as f64 + latency;
                if let Some(window) = output.get(expected as usize - 1200..expected as usize + 1200)
                {
                    let found = window.iter().position(|f| f[0].abs() > threshold).unwrap();
                    late.push((expected as i64 - 1200 + found as i64) as f64 - expected);
                }
                beat += 1;
            }
            late.sort_by(f64::total_cmp);
            let median = late[late.len() / 2] / SR as f64 * 1000.;
            assert!(late.len() > 4);
            assert!(median.abs() < 3., "rate {rate}: attacks {median:.2} ms off");
        }
    }

    #[test]
    fn a_hot_cue_on_an_attack_stays_intact_with_key_lock() {
        let period = 24000;
        let source = clip(SR, clicks(period, 20));
        source.cues.set(0, 5.);
        let dry: Vec<f32> = source.samples[period * 10..period * 10 + 960]
            .iter()
            .map(|f| f[0] * GAIN)
            .collect();
        let (h, mut r) = settled(source);
        h.control(0, Control::Pause).unwrap();
        h.control(0, Control::Rate(1.02)).unwrap();
        render(&mut r, 9600);
        h.control(0, Control::Cue(0)).unwrap();
        let output = render(&mut r, 9600);
        let delay = (SR as f64 * crate::JUMP_FADE_SECONDS) as usize + Limiter::new(SR).latency();
        let energy =
            |signal: &mut dyn Iterator<Item = f32>| signal.map(|v| (v as f64).powi(2)).sum::<f64>();
        let got = energy(&mut output[delay..delay + 240].iter().map(|f| f[0]));
        let want = energy(&mut dry[..240].iter().copied());
        let db = 10. * (got / want).log10();
        assert!(db > -1.5, "first 5 ms: {db:.1} dB");
    }

    #[test]
    fn engaging_and_loops_and_pause_never_step_the_audio() {
        let natural = (TAU * 55. / SR as f64) as f32 * 0.8 * GAIN;
        let (h, mut r) = settled(clip(SR, sine(SR, 55., 20, 0.8)));
        render(&mut r, 9600);
        // Plain playback into pitched audio and back.
        h.control(0, Control::Rate(1.02)).unwrap();
        let engaging = render(&mut r, 9600);
        assert!(largest_step(&engaging) <= natural * 3., "engage");
        h.control(0, Control::Rate(1.)).unwrap();
        let leaving = render(&mut r, 9600);
        assert!(largest_step(&leaving) <= natural * 3., "leave");
        // A loop in pitched audio.
        h.control(0, Control::Rate(1.03)).unwrap();
        render(&mut r, 9600);
        let position = h.snapshot().slots[0].position;
        h.control(0, Control::LoopIn).unwrap();
        h.control(0, Control::Seek(position + 0.4)).unwrap();
        h.control(0, Control::LoopOut).unwrap();
        render(&mut r, 4800);
        let looped = render(&mut r, 48000);
        assert!(h.snapshot().slots[0].looping);
        assert!(largest_step(&looped) <= natural * 3., "loop");
        // Pausing and starting again.
        h.control(0, Control::Pause).unwrap();
        let paused = render(&mut r, 4800);
        assert!(largest_step(&paused) <= natural * 3., "pause");
        assert!(paused.last().unwrap()[0].abs() < 1e-3);
        h.control(0, Control::Play).unwrap();
        let resumed = render(&mut r, 9600);
        assert!(largest_step(&resumed) <= natural * 3., "resume");
        // Retriggers faster than a fade stay finite and within the ceiling.
        for _ in 0..30 {
            h.control(0, Control::Seek(1.)).unwrap();
            let burst = render(&mut r, 64);
            assert!(burst.iter().flatten().all(|v| v.is_finite()));
            assert!(burst
                .iter()
                .flatten()
                .all(|v| v.abs() <= crate::limiter::CEILING));
        }
    }

    #[test]
    fn sync_with_key_lock_follows_at_once_and_stays_in_phase() {
        let make = |id, bpm: f64| {
            let mut source = clip(SR, sine(SR, 220., 40, 0.3));
            let c = Arc::get_mut(&mut source).unwrap();
            c.id = id;
            c.music.bpm = Some(bpm);
            source
        };
        let (h, mut r) = engine(SR);
        h.load_and_play(0, make(1, 126.)).unwrap();
        h.load_and_play(1, make(2, 120.)).unwrap();
        render(&mut r, 256);
        h.control(1, Control::Sync(true)).unwrap();
        render(&mut r, 256);
        let s = h.snapshot();
        assert!(s.slots[1].tuning.key_lock);
        assert_eq!(s.slots[1].rate, 1.05, "no preparation to wait for");
        let phase =
            |s: &crate::SlotSnapshot| (s.position * s.music.bpm.unwrap() / 60.).rem_euclid(1.);
        assert!((phase(&s.slots[0]) - phase(&s.slots[1])).abs() < 1e-5);
        let hz = frequency(&render(&mut r, SR as usize * 2)[9600..]);
        assert!((hz - 220.).abs() < 1., "{hz}");
        let s = h.snapshot();
        assert!((phase(&s.slots[0]) - phase(&s.slots[1])).abs() < 1e-3);
    }

    #[test]
    fn a_track_with_pitched_audio_can_be_replaced_while_playing() {
        let (h, mut r) = settled(clip(SR, sine(SR, 220., 8, 0.5)));
        let old = clip(SR, sine(SR, 220., 8, 0.5));
        h.load_and_play(0, old.clone()).unwrap();
        render(&mut r, 256);
        h.control(0, Control::Rate(1.1)).unwrap();
        render(&mut r, 9600);
        h.load_and_play(0, clip(SR, vec![[0.; 2]; SR as usize]))
            .unwrap();
        render(&mut r, 64);
        h.collect();
        assert!(Arc::strong_count(&old) >= 2, "the fading voice reads it");
        render(&mut r, 4800);
        h.collect();
        assert_eq!(Arc::strong_count(&old), 1);
        let s = h.snapshot().slots[0].clone();
        assert_eq!(s.rate, 1.);
        assert!(s.tuning.key_lock);
    }
}
