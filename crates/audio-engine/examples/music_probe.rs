//! Synthetic offline analysis/preparation probe. Does not measure DAC latency or quality.
use curado_audio::{
    decode::Clip,
    music,
    tuning::{self, Tuning},
};
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
    let started = Instant::now();
    let prepared = tuning::prepare(
        &source,
        Tuning {
            rate: 1.25,
            semitones: 2,
            key_lock: true,
        },
        || false,
    )
    .unwrap();
    println!("{{\"kind\":\"synthetic offline analysis and pitch preparation\",\"sampleRate\":{sr},\"seconds\":30,\"analysisMs\":{analysis_ms:.2},\"prepareMs\":{:.2},\"detectedBpm\":{:.3},\"sourceAndPreparedPcmBytes\":{},\"preparedFrames\":{}}}",started.elapsed().as_secs_f64()*1000.,analysis.bpm.unwrap_or(0.),(source.samples.len()+prepared.samples.len())*8,prepared.samples.len());
}
