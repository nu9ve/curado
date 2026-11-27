//! nu9ve-music against data/music/music-vectors-v1.json: synthesizes each recipe and checks the answer
//! against its truth, or against what the vectors record as a known miss. NU9VE_PRINT_MUSIC=1 prints them.

use nu9ve_music::analyze;
use serde_json::Value;
use std::f64::consts::TAU;
use std::path::Path;

const RATE: f64 = 48000.;

/// xorshift64*, seeded per signal: the same noise in every language.
struct Noise(u64);
impl Noise {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let x = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (x >> 11) as f64 / (1u64 << 53) as f64 * 2. - 1.
    }
}

fn seed(name: &str) -> u64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
    }) | 1
}

fn add(buffer: &mut [f64], start: f64, signal: &[f64], gain: f64) {
    let first = (start * RATE) as usize;
    for (k, v) in signal.iter().enumerate() {
        let Some(out) = buffer.get_mut(first + k) else {
            break;
        };
        *out += v * gain;
    }
}

fn kick() -> Vec<f64> {
    (0..(0.4 * RATE) as usize)
        .map(|i| {
            let t = i as f64 / RATE;
            (TAU * (50. * t + 120. * (1. - (-t / 0.03).exp()) * 0.03)).sin() * (-t / 0.25).exp()
        })
        .collect()
}

fn snare(noise: &mut Noise) -> Vec<f64> {
    (0..(0.25 * RATE) as usize)
        .map(|i| {
            let t = i as f64 / RATE;
            (noise.next() * 0.7 + 0.3 * (TAU * 190. * t).sin()) * (-t / 0.12).exp()
        })
        .collect()
}

fn hat(noise: &mut Noise) -> Vec<f64> {
    (0..(0.06 * RATE) as usize)
        .map(|i| noise.next() * (-(i as f64 / RATE) / 0.02).exp())
        .collect()
}

fn drums(style: &str, bpm: f64, start: f64, seconds: f64, noise: &mut Noise) -> Vec<f64> {
    let mut b = vec![0.; (seconds * RATE) as usize];
    let (k, s, h) = (kick(), snare(noise), hat(noise));
    let beat = 60. / bpm;
    let (mut t, mut n) = (start, 0usize);
    while t < seconds - 0.5 {
        let bar = n % 4;
        match style {
            "four" => {
                add(&mut b, t, &k, if bar == 0 { 0.9 } else { 0.5 });
                add(&mut b, t + beat / 2., &h, 0.25);
            }
            "hiphop" => {
                if bar == 0 || bar == 2 {
                    add(&mut b, t, &k, 0.9);
                }
                if bar == 1 || bar == 3 {
                    add(&mut b, t, &s, 0.6);
                }
                add(&mut b, t, &h, 0.2);
                add(&mut b, t + beat / 2., &h, 0.15);
            }
            _ => {
                if bar == 0 {
                    add(&mut b, t, &k, 0.9);
                }
                if bar == 2 {
                    add(&mut b, t + beat / 2., &k, 0.8);
                }
                if bar == 1 || bar == 3 {
                    add(&mut b, t, &s, 0.6);
                }
                add(&mut b, t + beat / 2., &h, 0.15);
            }
        }
        t += beat;
        n += 1;
    }
    b
}

fn tone(midi: f64, seconds: f64, gain: f64) -> Vec<f64> {
    let f = 440. * 2f64.powf((midi - 69.) / 12.);
    (0..(seconds * RATE) as usize)
        .map(|i| {
            let t = i as f64 / RATE;
            let harmonics: f64 = [1., 2., 3.]
                .iter()
                .map(|h| (TAU * f * h * t).sin() / h)
                .sum();
            gain * harmonics * (t / 0.01).min(1.) * (-t / 1.5).exp()
        })
        .collect()
}

/// I–IV–V–I (the V is major in both, as in harmonic minor), two seconds a chord, with the root an octave down.
fn chords(tonic: f64, minor: bool, seconds: f64) -> Vec<f64> {
    let third = if minor { 3. } else { 4. };
    let degrees = [
        [0., third, 7.],
        [5., 5. + third, 12.],
        [7., 11., 14.],
        [0., third, 7.],
    ];
    let mut b = vec![0.; (seconds * RATE) as usize];
    let (mut t, mut k) = (0., 0);
    while t < seconds - 2. {
        let chord = degrees[k % 4];
        for d in chord {
            add(&mut b, t, &tone(48. + tonic + d, 2., 0.12), 1.);
        }
        add(&mut b, t, &tone(36. + tonic + chord[0], 2., 0.15), 1.);
        t += 2.;
        k += 1;
    }
    b
}

fn synthesize(signal: &Value) -> Vec<[f32; 2]> {
    let name = signal["name"].as_str().unwrap();
    let seconds = signal["seconds"].as_f64().unwrap();
    let mut noise = Noise(seed(name));
    let mono = match signal["style"].as_str().unwrap() {
        "noise" => (0..(seconds * RATE) as usize)
            .map(|_| noise.next() * 0.3)
            .collect(),
        "chords" => chords(
            signal["tonic"].as_f64().unwrap(),
            signal["minor"].as_bool().unwrap(),
            seconds,
        ),
        style => drums(
            style,
            signal["bpm"].as_f64().unwrap(),
            signal["start"].as_f64().unwrap(),
            seconds,
            &mut noise,
        ),
    };
    mono.into_iter().map(|x| [x as f32, x as f32]).collect()
}

/// Distance between two instants on a grid of `period`, wrapping.
fn on_grid(a: f64, b: f64, period: f64) -> f64 {
    let d = (a - b).rem_euclid(period);
    d.min(period - d)
}

#[test]
fn signals_match_their_truth_or_their_known_miss() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/music/music-vectors-v1.json");
    let vectors: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let bpm_tolerance = vectors["tolerance"]["bpm"].as_f64().unwrap();
    let seconds_tolerance = vectors["tolerance"]["seconds"].as_f64().unwrap();
    let print = std::env::var("NU9VE_PRINT_MUSIC").is_ok();
    let mut failures = Vec::new();
    for signal in vectors["signals"].as_array().unwrap() {
        let name = signal["name"].as_str().unwrap();
        let analysis = analyze(&synthesize(signal), RATE as u32);
        let key = analysis.key.map(|k| (k.tonic, k.minor));
        if print {
            println!(
                "{name}: bpm={:?} offset={:.4} downbeat={:.4} key={key:?} conf={:.2}/{:.2}",
                analysis.bpm,
                analysis.beat_offset,
                analysis.downbeat,
                analysis.bpm_confidence,
                analysis.key_confidence
            );
        }
        let known = &signal["known"];
        let truth = &signal["truth"];
        // A known miss pins today's answer instead of the truth.
        let want = if known.is_object() { known } else { truth };
        if let Some(bpm) = want.get("bpm") {
            match (bpm.as_f64(), analysis.bpm) {
                (None, None) => {}
                (Some(w), Some(got)) if (w - got).abs() <= bpm_tolerance => {}
                (w, got) => failures.push(format!("{name}: bpm {got:?}, want {w:?}")),
            }
        }
        let period = analysis.bpm.map(|b| 60. / b);
        if let (Some(beat), Some(period)) = (want.get("beat").and_then(Value::as_f64), period)
            && on_grid(analysis.beat_offset, beat, period) > seconds_tolerance
        {
            failures.push(format!(
                "{name}: beat offset {:.4}, want {beat} on the grid",
                analysis.beat_offset
            ));
        }
        if let (Some(downbeat), Some(period)) =
            (want.get("downbeat").and_then(Value::as_f64), period)
            && on_grid(analysis.downbeat, downbeat, 4. * period) > seconds_tolerance
        {
            failures.push(format!(
                "{name}: downbeat {:.4}, want {downbeat} on the bar",
                analysis.downbeat
            ));
        }
        if let Some(k) = want.get("key").and_then(Value::as_array) {
            let w = (k[0].as_u64().unwrap() as u8, k[1].as_bool().unwrap());
            if key != Some(w) {
                failures.push(format!("{name}: key {key:?}, want {w:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
