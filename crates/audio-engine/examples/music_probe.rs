//! Synthetic analysis and real-time key lock probe. Does not measure DAC latency or quality.
use curado_audio::{decode::Clip, engine, music, Control};
use std::{sync::Arc, time::Instant};
fn main() {
    let sr = 48000;
    let samples: Vec<_> = (0..sr * 30)
        .map(|i| {
            let time = i as f64 / sr as f64;
            let mut x = 0.;
            for hz in [261.625565, 329.627557, 391.995436] {
                x += (std::f64::consts::TAU * hz * time).sin() * 0.12;
            }
            let phase = (time - 0.12).rem_euclid(0.5);
            x += (std::f64::consts::TAU * 70. * phase).sin() * (-phase * 55.).exp() * 0.5;
            [x as f32, x as f32]
        })
        .collect();
    let started = Instant::now();
    let analysis = music::analyze(&samples, sr as u32);
    let analysis_ms = started.elapsed().as_secs_f64() * 1000.;
    let source = Arc::new(Clip {
        id: 1,
        cues: Default::default(),
        samples,
        sample_rate: sr as u32,
        waveform: vec![],
        title: None,
        artist: None,
        music: analysis,
    });
    let (handle, mut renderer) = engine(sr as u32);
    handle.load_and_play(0, source.clone()).unwrap();
    handle.load_and_play(1, source).unwrap();
    handle.control(0, Control::Rate(1.25)).unwrap();
    handle.control(0, Control::Transpose(2)).unwrap();
    handle.control(1, Control::Rate(0.94)).unwrap();
    let mut output = [0.0_f32; 512];
    let blocks = sr * 20 / 256;
    let mut timings = Vec::with_capacity(blocks);
    for _ in 0..blocks {
        let started = Instant::now();
        renderer.render(&mut output, 2);
        timings.push(started.elapsed().as_secs_f64() * 1_000_000.);
    }
    timings.sort_by(f64::total_cmp);
    println!("{{\"kind\":\"synthetic analysis and two key-locked decks, not hardware latency\",\"sampleRate\":{sr},\"seconds\":30,\"analysisMs\":{analysis_ms:.2},\"detectedBpm\":{:.3},\"p50Us\":{:.2},\"p99Us\":{:.2},\"maxUs\":{:.2},\"budgetUs\":5333.33}}",analysis.bpm.unwrap_or(0.),timings[blocks/2],timings[blocks*99/100],timings[blocks-1]);
}
