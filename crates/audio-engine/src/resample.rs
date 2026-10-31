//! Windowed-sinc playback interpolation. Linear interpolation dulled any audio not played at
//! its own rate and folded what lay above the new Nyquist back into the audible range.
//! Tables are built once, before the first callback; reading them takes no lock.
use std::sync::LazyLock;

const TAPS: usize = 24;
const PHASES: usize = 128;
/// Read speeds each table covers, and the cutoff that keeps audio above the new Nyquist out.
const SPEEDS: [f64; 6] = [1.02, 1.1, 1.25, 1.5, 2., 3.];
const PASSBAND: f64 = 0.92;
const KAISER_BETA: f64 = 9.;

type Table = Vec<[f32; TAPS]>;
static TABLES: LazyLock<[Table; SPEEDS.len()]> =
    LazyLock::new(|| std::array::from_fn(|i| build(PASSBAND / SPEEDS[i])));

/// Call outside the callback: the first read would otherwise allocate there.
pub fn prepare() {
    LazyLock::force(&TABLES);
}

fn bessel_i0(x: f64) -> f64 {
    let (mut sum, mut term) = (1., 1.);
    for k in 1..32 {
        term *= (x / 2. / k as f64).powi(2);
        sum += term;
    }
    sum
}

/// Phase `p` holds the kernel for a read `p / PHASES` of the way past a sample. One extra phase
/// lets the last one interpolate towards the next sample without a special case.
fn build(cutoff: f64) -> Table {
    (0..=PHASES)
        .map(|phase| {
            let offset = phase as f64 / PHASES as f64;
            let mut taps = [0.; TAPS];
            let mut sum = 0.;
            for (i, tap) in taps.iter_mut().enumerate() {
                let x = i as f64 - (TAPS / 2 - 1) as f64 - offset;
                let sinc = if x.abs() < 1e-9 {
                    1.
                } else {
                    (std::f64::consts::PI * x * cutoff).sin() / (std::f64::consts::PI * x * cutoff)
                };
                let edge = x / (TAPS / 2) as f64;
                let window = if edge.abs() >= 1. {
                    0.
                } else {
                    bessel_i0(KAISER_BETA * (1. - edge * edge).sqrt()) / bessel_i0(KAISER_BETA)
                };
                let value = sinc * window;
                sum += value;
                *tap = value as f32;
            }
            // Unity at DC, so a level never depends on the phase of a read.
            taps.map(|tap| (tap as f64 / sum) as f32)
        })
        .collect()
}

/// Reads at a fractional position. `speed` is how many source frames pass per output frame,
/// and a sample read in place at normal speed comes back untouched.
#[inline]
pub fn read(samples: &[[f32; 2]], position: f64, speed: f64) -> [f32; 2] {
    let index = position.floor();
    let frac = position - index;
    let index = index as isize;
    if frac == 0. && speed == 1. {
        return usize::try_from(index)
            .ok()
            .and_then(|i| samples.get(i))
            .copied()
            .unwrap_or([0.; 2]);
    }
    let table = SPEEDS.iter().position(|limit| speed <= *limit);
    let table = &TABLES[table.unwrap_or(SPEEDS.len() - 1)];
    let scaled = frac * PHASES as f64;
    let phase = scaled as usize;
    let blend = (scaled - phase as f64) as f32;
    let (near, far) = (&table[phase], &table[phase + 1]);
    let first = index - (TAPS / 2 - 1) as isize;
    let mut out = [0_f32; 2];
    if first >= 0 && first as usize + TAPS <= samples.len() {
        let window = &samples[first as usize..first as usize + TAPS];
        for (i, frame) in window.iter().enumerate() {
            let weight = near[i] + (far[i] - near[i]) * blend;
            out[0] += frame[0] * weight;
            out[1] += frame[1] * weight;
        }
    } else {
        // Past either end of the file there is only silence.
        for i in 0..TAPS {
            let Some(frame) = usize::try_from(first + i as isize)
                .ok()
                .and_then(|at| samples.get(at))
            else {
                continue;
            };
            let weight = near[i] + (far[i] - near[i]) * blend;
            out[0] += frame[0] * weight;
            out[1] += frame[1] * weight;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    fn tone(hz: f64, sr: f64, seconds: f64) -> Vec<[f32; 2]> {
        (0..(sr * seconds) as usize)
            .map(|i| {
                let v = (TAU * hz * i as f64 / sr).sin() as f32 * 0.5;
                [v, v]
            })
            .collect()
    }
    /// Amplitude at `hz` by correlation, and the level of everything else.
    fn measure(samples: &[[f32; 2]], speed: f64, hz: f64, sr: f64) -> (f64, f64) {
        let output: Vec<f64> = (0..(sr * 1.) as usize)
            .map(|i| read(samples, 4000. + i as f64 * speed, speed)[0] as f64)
            .collect();
        let n = output.len() as f64;
        let (mut re, mut im, mut power) = (0., 0., 0.);
        for (i, v) in output.iter().enumerate() {
            let t = TAU * hz * i as f64 / sr;
            re += v * t.cos();
            im += v * t.sin();
            power += v * v;
        }
        let amplitude = 2. * (re * re + im * im).sqrt() / n;
        let rest = (power / n - amplitude * amplitude / 2.).max(1e-20).sqrt();
        (amplitude, rest)
    }
    fn db(ratio: f64) -> f64 {
        20. * ratio.log10()
    }

    #[test]
    fn a_sample_read_in_place_comes_back_exactly() {
        let samples = tone(997., 48000., 0.1);
        for i in [0_usize, 1, 500, samples.len() - 1] {
            assert_eq!(read(&samples, i as f64, 1.), samples[i]);
        }
        assert_eq!(read(&samples, samples.len() as f64, 1.), [0.; 2]);
        assert_eq!(read(&samples, -3., 1.), [0.; 2]);
    }

    #[test]
    fn reading_never_leaves_the_buffer_and_ends_in_silence() {
        let samples = vec![[1., -1.]; 40];
        for at in [-100., -12.5, -0.5, 0.25, 11.9, 28.1, 39.5, 51.5, 1e9] {
            assert!(read(&samples, at, 1.3).iter().all(|v| v.is_finite()));
        }
        assert_eq!(read(&samples, 80.5, 1.3), [0.; 2]);
        assert!(
            (read(&samples, 20.5, 1.)[0] - 1.).abs() < 0.001,
            "unity at DC"
        );
    }

    #[test]
    fn highs_stay_when_the_rate_changes() {
        let sr = 44100.;
        // Linear interpolation loses about 2 dB at 15 kHz here and adds images near -20 dB.
        let samples = tone(15000., sr, 1.2);
        let (amplitude, rest) = measure(&samples, 0.97, 15000. * 0.97, sr);
        assert!(
            db(amplitude / 0.5).abs() < 0.2,
            "{} dB",
            db(amplitude / 0.5)
        );
        assert!(db(rest / amplitude) < -70., "{} dB", db(rest / amplitude));
    }

    #[test]
    fn audio_above_the_new_nyquist_does_not_fold_back() {
        let sr = 48000.;
        // At 1.5x, 20 kHz would land at 30 kHz and fold down to 18 kHz.
        let samples = tone(20000., sr, 1.8);
        let (_, rest) = measure(&samples, 1.5, 18000., sr);
        assert!(db(rest / 0.5) < -60., "{} dB", db(rest / 0.5));
        // A tone that fits stays.
        let samples = tone(2000., sr, 1.8);
        let (amplitude, _) = measure(&samples, 1.5, 3000., sr);
        assert!(db(amplitude / 0.5).abs() < 0.1);
    }
}
