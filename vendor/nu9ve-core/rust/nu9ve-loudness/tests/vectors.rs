//! `data/audio/loudness-vectors-v1.json`, written by `scripts/loudness-vectors.py`:
//!
//! - goldens: rendered PCM in `data/audio/loudness-goldens/` with the integrated LUFS another
//!   engine measured, within 0.01 LU;
//! - ebu: the EBU Tech 3341 and 3342 signals that can be synthesised, synthesised here from their
//!   recipe and checked with the tolerance of the standard at every listed sample rate.
//!
//! The path is the same in nu9ve/core and in an app's pinned copy (`vendor/nu9ve-core/`): `../../data`.

use std::f64::consts::PI;
use std::path::PathBuf;

use nu9ve_loudness::{Meter, Report, to_dbtp, true_peak};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/audio")
}

fn vectors() -> Value {
    let path = data().join("loudness-vectors-v1.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "Missing {}: an app pins `data/audio` with `rust/nu9ve-loudness` (sync-core.sh)",
            path.display()
        )
    });
    let vectors: Value = serde_json::from_str(&text).expect("vectors are JSON");
    assert_eq!(vectors["piece"], "nu9ve-loudness");
    assert_eq!(vectors["version"], 1);
    vectors
}

fn f(value: &Value) -> f64 {
    value
        .as_f64()
        .unwrap_or_else(|| panic!("not a number: {value}"))
}

fn u(value: &Value) -> usize {
    value
        .as_u64()
        .unwrap_or_else(|| panic!("not a count: {value}")) as usize
}

#[test]
fn golden_renders_read_the_recorded_loudness() {
    let vectors = vectors();
    let goldens = &vectors["goldens"];
    let folder = data().join(goldens["folder"].as_str().unwrap());
    let rate = u(&goldens["sample_rate"]) as u32;
    let tolerance = f(&goldens["tolerance_lu"]);
    let cases = goldens["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 23);
    let mut worst = 0.0f64;
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let bytes = std::fs::read(folder.join(case["file"].as_str().unwrap()))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            case["sha256"].as_str().unwrap(),
            "{name}: the PCM changed"
        );
        let (channels, frames) = (u(&case["channels"]), u(&case["frames"]));
        let samples: Vec<f32> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        assert_eq!(samples.len(), channels * frames, "{name}");
        let planar: Vec<&[f32]> = samples.chunks_exact(frames).collect();
        let report = nu9ve_loudness::measure(&planar, rate).unwrap();
        let expected = f(&case["integrated_lufs"]);
        let measured = report
            .integrated_lufs
            .unwrap_or_else(|| panic!("{name}: no integrated LUFS"));
        assert!(
            (measured - expected).abs() <= tolerance,
            "{name}: {measured} LUFS, expected {expected}"
        );
        worst = worst.max((measured - expected).abs());
        match case["short_term_max_lufs"].as_f64() {
            Some(expected) => {
                let measured = report.short_term_max_lufs.unwrap();
                assert!(
                    (measured - expected).abs() <= tolerance,
                    "{name}: short-term {measured}"
                );
            }
            None => assert_eq!(report.short_term_max_lufs, None, "{name}"),
        }
    }
    eprintln!("goldens: largest difference {worst:.6} LU");
}

/// Planar channels synthesised from a recipe of `ebu.signals`.
fn synthesise(signal: &Value, rate: u32) -> (Vec<Vec<f32>>, Vec<f64>) {
    let rate_f = rate as f64;
    let fade = |samples: &mut [f64], fade_ms: f64, rate: f64| {
        let n = (fade_ms / 1000.0 * rate).round() as usize;
        let len = samples.len();
        for m in 0..n.min(len / 2) {
            let gain = 0.5 * (1.0 - (PI * m as f64 / n as f64).cos());
            samples[m] *= gain;
            samples[len - 1 - m] *= gain;
        }
    };
    match signal["kind"].as_str().unwrap() {
        "sine_segments" => {
            let weights: Vec<f64> = signal["weights"]
                .as_array()
                .unwrap()
                .iter()
                .map(f)
                .collect();
            let hz = f(&signal["hz"]);
            let mut channels = vec![Vec::new(); weights.len()];
            let mut start = 0.0;
            let mut n = 0usize;
            for segment in signal["segments"].as_array().unwrap() {
                let end = start + f(&segment["seconds"]);
                let count = (end * rate_f).round() as usize - (start * rate_f).round() as usize;
                let levels: Vec<Option<f64>> = match segment["dbfs"].as_array() {
                    Some(levels) => levels
                        .iter()
                        .map(|l| Some(10f64.powf(f(l) / 20.0)))
                        .collect(),
                    None => vec![None; weights.len()],
                };
                for i in n..n + count {
                    let phase = (2.0 * PI * hz * i as f64 / rate_f).sin();
                    for (channel, level) in channels.iter_mut().zip(&levels) {
                        channel.push(level.map_or(0.0, |a| (a * phase) as f32));
                    }
                }
                n += count;
                start = end;
            }
            (channels, weights)
        }
        "sine_ratio" => {
            let divisor = f(&signal["rate_divisor"]);
            let amplitude = f(&signal["amplitude"]);
            let phase = f(&signal["phase_degrees"]).to_radians();
            let len = (f(&signal["seconds"]) * rate_f).round() as usize;
            let mut wave: Vec<f64> = (0..len)
                .map(|i| amplitude * (2.0 * PI * i as f64 / divisor + phase).sin())
                .collect();
            fade(&mut wave, f(&signal["fade_ms"]), rate_f);
            let wave: Vec<f32> = wave.into_iter().map(|v| v as f32).collect();
            let channels = u(&signal["channels"]);
            (vec![wave; channels], vec![1.0; channels])
        }
        "oversampled_burst" => {
            let over = u(&signal["oversampling"]);
            let fast = rate_f * over as f64;
            let base_period = u(&signal["base_divisor"]) * over;
            let burst_period = u(&signal["burst_divisor"]) * over;
            let (base, burst) = (f(&signal["base_amplitude"]), f(&signal["burst_amplitude"]));
            let len = (f(&signal["seconds"]) * fast).round() as usize;
            let at = (len / 2 / base_period) * base_period;
            let mut wave: Vec<f64> = (0..len)
                .map(|m| {
                    let (amplitude, period, k) = if m < at {
                        (base, base_period, m)
                    } else if m < at + burst_period {
                        (burst, burst_period, m - at)
                    } else {
                        (base, base_period, m - at - burst_period)
                    };
                    amplitude * (2.0 * PI * k as f64 / period as f64).sin()
                })
                .collect();
            fade(&mut wave, f(&signal["fade_ms"]), fast);
            let taps = u(&signal["lowpass_taps"]);
            let centre = (taps / 2) as f64;
            let cutoff = 0.5 / over as f64;
            let mut kernel: Vec<f64> = (0..taps)
                .map(|k| {
                    let x = k as f64 - centre;
                    let sinc = if x == 0.0 {
                        1.0
                    } else {
                        (2.0 * PI * cutoff * x).sin() / (PI * x) / (2.0 * cutoff)
                    };
                    let hann = 0.5 * (1.0 - (2.0 * PI * k as f64 / (taps - 1) as f64).cos());
                    sinc * hann
                })
                .collect();
            let sum: f64 = kernel.iter().sum();
            kernel.iter_mut().for_each(|k| *k /= sum);
            let half = taps / 2;
            let offset = u(&signal["offset"]);
            let wave: Vec<f32> = (offset..len)
                .step_by(over)
                .map(|m| {
                    let mut acc = 0.0;
                    for (k, h) in kernel.iter().enumerate() {
                        if let Some(i) = (m + k).checked_sub(half).filter(|&i| i < len) {
                            acc += wave[i] * h;
                        }
                    }
                    acc as f32
                })
                .collect();
            let channels = u(&signal["channels"]);
            (vec![wave; channels], vec![1.0; channels])
        }
        other => panic!("unknown signal kind {other}"),
    }
}

fn reading(report: &Report, channels: &[Vec<f32>], key: &str) -> Option<f64> {
    match key {
        "integrated_lufs" => report.integrated_lufs,
        "momentary_max_lufs" => report.momentary_max_lufs,
        "short_term_max_lufs" => report.short_term_max_lufs,
        "loudness_range_lu" => report.loudness_range_lu,
        "true_peak_dbtp" => Some(to_dbtp(true_peak(channels))),
        other => panic!("unknown reading {other}"),
    }
}

#[test]
fn ebu_minimum_requirements() {
    let vectors = vectors();
    let cases = vectors["ebu"]["cases"].as_array().unwrap();
    assert!(cases.len() >= 60);
    let mut failures = Vec::new();
    // Largest distance from the expected value, per reading, to see the margin left.
    let mut worst: std::collections::BTreeMap<String, (f64, String)> = Default::default();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let (below, above) = (
            f(&case["tolerance"]["below"]),
            f(&case["tolerance"]["above"]),
        );
        for rate in case["sample_rates"].as_array().unwrap() {
            let rate = u(rate) as u32;
            let (channels, weights) = synthesise(&case["signal"], rate);
            let mut meter = Meter::with_weights(rate, &weights).unwrap();
            meter.add_planar(&channels).unwrap();
            let report = meter.report();
            for (key, expected) in case["expect"].as_object().unwrap() {
                let expected = f(expected);
                let measured = reading(&report, &channels, key);
                let ok = measured.is_some_and(|m| m >= expected - below && m <= expected + above);
                if let Some(m) = measured {
                    let entry = worst.entry(key.clone()).or_insert((0.0, String::new()));
                    if (m - expected).abs() > entry.0.abs() {
                        *entry = (m - expected, format!("{name} at {rate} Hz"));
                    }
                }
                if !ok {
                    failures.push(format!(
                        "{name} at {rate} Hz: {key} = {measured:?}, expected {expected}"
                    ));
                }
            }
        }
    }
    for (key, (difference, at)) in &worst {
        eprintln!("ebu: {key}: largest difference {difference:+.4} ({at})");
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
