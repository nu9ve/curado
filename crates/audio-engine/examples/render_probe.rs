//! Offline CPU probe. This is not a hardware latency or dropout measurement.
use curado_audio::{decode::Clip, engine, Control};
use std::{sync::Arc, time::Instant};
fn main() {
    let (handle, mut renderer) = engine(48000);
    let clip = Arc::new(Clip {
        id: 1,
        sample_rate: 48000,
        samples: (0..48000)
            .map(|i| {
                let x = (i as f32 * 220. * std::f32::consts::TAU / 48000.).sin() * 0.15;
                [x, x]
            })
            .collect(),
        waveform: vec![],
        title: None,
        artist: None,
        music: Default::default(),
        cues: Arc::new(curado_audio::annotations::CueState::default()),
    });
    let mut output = [0.0_f32; 512];
    for slot in 0..6 {
        handle.load(slot, clip.clone()).unwrap();
        handle.control(slot, Control::Play).unwrap();
        handle.control(slot, Control::Loop).unwrap();
        handle.control(slot, Control::Echo(0.6)).unwrap();
        handle.control(slot, Control::Filter(0.7)).unwrap();
        renderer.render(&mut output, 2);
    }
    let blocks = 48000 * 60 / 256;
    let mut timings = Vec::with_capacity(blocks);
    for _ in 0..blocks {
        let started = Instant::now();
        renderer.render(&mut output, 2);
        timings.push(started.elapsed().as_secs_f64() * 1_000_000.);
    }
    timings.sort_by(f64::total_cmp);
    let snap = handle.snapshot();
    println!("{{\"kind\":\"offline CPU probe, not hardware latency\",\"sampleRate\":48000,\"frames\":256,\"slots\":6,\"simulatedSeconds\":60,\"p50Us\":{:.2},\"p95Us\":{:.2},\"p99Us\":{:.2},\"maxUs\":{:.2},\"budgetUs\":5333.33,\"clippedSamples\":{}}}",timings[blocks/2],timings[blocks*95/100],timings[blocks*99/100],timings[blocks-1],snap.clipped);
}
