//! Time per callback with everything on, against the time the block lasts. Run in release:
//! `cargo run --release -p curado-audio --example fx_budget`
use curado_audio::{annotations::CueState, decode::Clip, engine, Control, MASTER_UNIT};
use std::{sync::Arc, time::Instant};

const SR: u32 = 48000;
fn clip(id: u64, seconds: usize) -> Arc<Clip> {
    let samples = (0..44100 * seconds)
        .map(|i| {
            let t = i as f32 / 44100.;
            let v = (t * 311.).sin() * 0.3 + (t * 2093.).sin() * 0.1;
            [v, v * 0.9]
        })
        .collect();
    Arc::new(Clip {
        loudness: Some(-12.),
        id,
        cues: Arc::new(CueState::default()),
        samples,
        sample_rate: 44100,
        waveform: vec![],
        title: None,
        artist: None,
        music: curado_audio::music::MusicAnalysis {
            bpm: Some(124.),
            beat_offset: 0.1,
            version: curado_audio::music::ANALYSIS_VERSION,
            ..Default::default()
        },
    })
}
fn measure(name: &str, setup: &[(usize, Control)], frames: usize) {
    let (handle, mut renderer) = engine(SR);
    let mut output = vec![0_f32; frames * 2];
    handle.load_and_play(0, clip(1, 240)).unwrap();
    renderer.render(&mut output, 2);
    handle.load_and_play(1, clip(2, 240)).unwrap();
    renderer.render(&mut output, 2);
    // Long enough to keep sounding through the measurement.
    for slot in 2..10 {
        handle
            .load_and_play(slot, clip(slot as u64 + 2, 30))
            .unwrap();
        renderer.render(&mut output, 2);
    }
    for (slot, control) in setup {
        handle.control(*slot, *control).unwrap();
    }
    for _ in 0..200 {
        renderer.render(&mut output, 2);
    }
    let mut times: Vec<f64> = (0..4000)
        .map(|_| {
            let started = Instant::now();
            renderer.render(&mut output, 2);
            std::hint::black_box(&output);
            started.elapsed().as_secs_f64()
        })
        .collect();
    times.sort_by(f64::total_cmp);
    let budget = frames as f64 / SR as f64;
    let share = |value: f64| value / budget * 100.;
    println!(
        "{name:<44} median {:5.1} %  p99 {:5.1} %  worst {:5.1} %  of {:.2} ms",
        share(times[times.len() / 2]),
        share(times[times.len() * 99 / 100]),
        share(times[times.len() - 1]),
        budget * 1000.
    );
}
fn main() {
    use Control as C;
    let everything = [
        (0, C::Rate(1.04)),
        (1, C::Rate(0.97)),
        (0, C::Low(0.5)),
        (1, C::High(0.5)),
        (0, C::Filter(0.6)),
        (1, C::HighPass(0.3)),
        (0, C::ColorFx(1)),
        (1, C::ColorFx(1)),
        (0, C::Color(0.7)),
        (1, C::Color(-0.7)),
        (0, C::BeatFx(3)),
        (1, C::BeatFx(3)),
        (MASTER_UNIT, C::BeatFx(3)),
        (0, C::BeatFxOn(true)),
        (1, C::BeatFxOn(true)),
        (MASTER_UNIT, C::BeatFxOn(true)),
    ];
    measure("two decks, no effects", &[], 256);
    measure(
        "two decks, tempo moved (sinc)",
        &[(0, C::Rate(1.04)), (1, C::Rate(0.97))],
        256,
    );
    measure("all reverbs, EQ and filters", &everything, 256);
    measure("all reverbs, EQ and filters", &everything, 64);
    let mut locked = everything.to_vec();
    locked.extend([(0, C::KeyLock(true)), (1, C::KeyLock(true))]);
    measure("all of that with key lock on both", &locked, 256);
}
