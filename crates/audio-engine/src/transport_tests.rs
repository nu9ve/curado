//! Transport discontinuities, attacks, quantize and sync phase, with audio of known timing.
use super::*;
use std::f64::consts::TAU;

const SR: u32 = 48000;
const BPM: f64 = 120.;
const BEAT: f64 = 0.5;
const OFFSET: f64 = 0.125;
/// Channel gain and master defaults with the crossfader on A.
const GAIN: f32 = 0.8 * 0.75;

fn clip_of(id: u64, samples: Vec<[f32; 2]>, grid: bool) -> Arc<Clip> {
    Arc::new(Clip {
        id,
        cues: Default::default(),
        samples,
        sample_rate: SR,
        waveform: vec![],
        title: None,
        artist: None,
        music: MusicAnalysis {
            version: 1,
            bpm: grid.then_some(BPM),
            beat_offset: if grid { OFFSET } else { 0. },
            bpm_confidence: 1.,
            key: None,
            key_confidence: 0.,
            manual: true,
            ..Default::default()
        },
    })
}
fn sine(seconds: usize, hz: f64, amplitude: f32) -> Vec<[f32; 2]> {
    (0..SR as usize * seconds)
        .map(|i| {
            let x = (TAU * hz * i as f64 / SR as f64).sin() as f32 * amplitude;
            [x, x]
        })
        .collect()
}
/// Left channel of real-sized callbacks, so commands land on block boundaries.
fn render(r: &mut Renderer, frames: usize) -> Vec<f32> {
    let mut out = vec![0_f32; frames * 2];
    for block in out.chunks_mut(512) {
        r.render(block, 2);
    }
    out.chunks(2).map(|frame| frame[0]).collect()
}
/// An engine with deck A alone at settled levels, so only transport changes move the output.
fn settled() -> (EngineHandle, Renderer) {
    let (h, mut r) = engine(SR);
    h.control(0, Control::Crossfader(0.)).unwrap();
    render(&mut r, SR as usize);
    (h, r)
}
fn largest_step(output: &[f32]) -> f32 {
    output
        .windows(2)
        .fold(0., |max, pair| max.max((pair[1] - pair[0]).abs()))
}
fn jump_frames() -> usize {
    (SR as f64 * JUMP_FADE_SECONDS).round() as usize
}
fn limiter_latency() -> usize {
    Limiter::new(SR).latency()
}
fn frame_at(seconds: f64) -> usize {
    (seconds * SR as f64).round() as usize
}

#[test]
fn loop_boundary_stays_continuous_on_sustained_bass() {
    let amplitude = 0.8;
    let natural = (TAU * 55. / SR as f64) as f32 * amplitude * GAIN;
    for beats in [1., 2., 4., 8.] {
        let (h, mut r) = settled();
        h.load(0, clip_of(1, sine(40, 55., amplitude), true))
            .unwrap();
        h.control(0, Control::Seek(OFFSET + 16. * BEAT)).unwrap();
        h.control(0, Control::LoopBeats(beats)).unwrap();
        render(&mut r, 2048);
        h.control(0, Control::Play).unwrap();
        render(&mut r, 2048);
        let output = render(&mut r, frame_at(beats * BEAT) * 3 + 4096);
        assert!(h.snapshot().slots[0].looping);
        let step = largest_step(&output);
        // The hard wrap this replaces stepped up to 170 times the sine's own slope.
        assert!(step <= natural * 3., "{beats} beats: {step} vs {natural}");
    }
}

#[test]
fn loop_fade_holds_when_the_file_is_resampled_and_tempo_changes() {
    // The usual case: a 44.1 kHz file on a 48 kHz device, with the pitch fader moved.
    let source_rate = 44100.;
    let samples: Vec<_> = (0..source_rate as usize * 20)
        .map(|i| {
            let x = (TAU * 55. * i as f64 / source_rate).sin() as f32 * 0.8;
            [x, x]
        })
        .collect();
    let mut clip = clip_of(1, samples, true);
    Arc::get_mut(&mut clip).unwrap().sample_rate = source_rate as u32;
    let rate = 1.06;
    let (h, mut r) = settled();
    h.load(0, clip).unwrap();
    h.control(0, Control::KeyLock(false)).unwrap();
    h.control(0, Control::Rate(rate)).unwrap();
    h.control(0, Control::Seek(OFFSET + 8. * BEAT)).unwrap();
    h.control(0, Control::LoopBeats(1.)).unwrap();
    render(&mut r, 2048);
    h.control(0, Control::Play).unwrap();
    render(&mut r, 2048);
    let output = render(&mut r, SR as usize * 2);
    let s = h.snapshot().slots[0].clone();
    assert!(s.looping && s.position >= s.loop_in - 0.01 && s.position <= s.loop_out);
    let natural = (TAU * 55. * rate as f64 / SR as f64) as f32 * 0.8 * GAIN;
    let step = largest_step(&output);
    assert!(step <= natural * 3., "{step} vs {natural}");
}

#[test]
fn loop_reaches_its_start_at_full_gain_and_never_plays_past_out() {
    let start = frame_at(OFFSET + 2. * BEAT);
    let end = frame_at(OFFSET + 3. * BEAT);
    // Silence, an attack at IN, and loud audio past OUT that must never be heard.
    let mut samples = vec![[0.; 2]; SR as usize * 4];
    samples[start..start + 480].fill([0.5; 2]);
    samples[end..].fill([0.5; 2]);
    let (h, mut r) = settled();
    h.load(0, clip_of(1, samples, true)).unwrap();
    h.control(0, Control::Seek(OFFSET + 2. * BEAT)).unwrap();
    h.control(0, Control::LoopBeats(1.)).unwrap();
    render(&mut r, 256);
    h.control(0, Control::Play).unwrap();
    let output = render(&mut r, frame_at(BEAT) * 5);
    let attack = 0.5 * GAIN;
    let attacks = output
        .windows(2)
        .filter(|pair| pair[0].abs() < 1e-4 && (pair[1] - attack).abs() < 1e-3)
        .count();
    assert!(attacks >= 4, "{attacks} full-gain attacks");
    let mid_level = output
        .iter()
        .filter(|v| v.abs() > 1e-3 && (**v - attack).abs() > 1e-3)
        .count();
    assert_eq!(mid_level, 0, "an attack was faded or OUT leaked");
    let ones = output
        .iter()
        .filter(|v| (**v - attack).abs() < 1e-3)
        .count();
    assert!(ones <= 480 * 6, "audio past OUT was audible: {ones}");
}

#[test]
fn cue_and_play_keep_the_attack_at_their_target() {
    let target = 1.;
    let mut samples = vec![[0.; 2]; SR as usize * 4];
    samples[frame_at(target)..].fill([0.5; 2]);
    let shared = clip_of(1, samples, false);
    shared.cues.set(0, target);
    let expected = jump_frames() + limiter_latency();
    for playing in [false, true] {
        let (h, mut r) = settled();
        h.load(0, shared.clone()).unwrap();
        render(&mut r, 256);
        if playing {
            h.control(0, Control::Play).unwrap();
            render(&mut r, 2048);
        }
        h.control(0, Control::Cue(0)).unwrap();
        let output = render(&mut r, 2048);
        assert!(output[..expected].iter().all(|v| v.abs() < 1e-4), "early");
        assert!(
            (output[expected] - 0.5 * GAIN).abs() < 1e-3,
            "playing={playing}: first sample {} instead of {}",
            output[expected],
            0.5 * GAIN
        );
    }
    // Pausing on the attack and pressing play must also start at full gain.
    let (h, mut r) = settled();
    h.load(0, shared).unwrap();
    h.control(0, Control::Seek(target)).unwrap();
    render(&mut r, 256);
    h.control(0, Control::Play).unwrap();
    let output = render(&mut r, 2048);
    assert!((output[expected] - 0.5 * GAIN).abs() < 1e-3);
}

#[test]
fn start_of_file_plays_at_full_gain_without_delay() {
    let (h, mut r) = settled();
    h.load(0, clip_of(1, vec![[0.5; 2]; SR as usize], false))
        .unwrap();
    h.control(0, Control::Trigger).unwrap();
    let output = render(&mut r, 512);
    assert!((output[limiter_latency()] - 0.5 * GAIN).abs() < 1e-3);
}

#[test]
fn pause_fades_real_audio_and_keeps_the_exact_position() {
    let (h, mut r) = settled();
    h.load_and_play(0, clip_of(1, sine(4, 55., 0.8), false))
        .unwrap();
    render(&mut r, SR as usize);
    let before = render(&mut r, 2048);
    let natural = largest_step(&before);
    let position = h.snapshot().slots[0].position;
    h.control(0, Control::Pause).unwrap();
    let output = render(&mut r, 4096);
    let s = h.snapshot().slots[0].clone();
    assert!(!s.playing);
    assert_eq!(s.position, position);
    assert!(largest_step(&output) <= natural * 2.);
    assert!(output[jump_frames() + limiter_latency() + 8..]
        .iter()
        .all(|v| v.abs() < 1e-5));
}

#[test]
fn seek_while_playing_crossfades_opposite_phases() {
    let (h, mut r) = settled();
    h.load_and_play(0, clip_of(1, sine(4, 55., 0.8), false))
        .unwrap();
    render(&mut r, SR as usize);
    let natural = largest_step(&render(&mut r, 2048));
    let position = h.snapshot().slots[0].position;
    h.control(0, Control::Seek(position + 0.5 / 55.)).unwrap();
    let output = render(&mut r, 2048);
    assert!(largest_step(&output) <= natural * 4.);
}

#[test]
fn replaced_pcm_fades_out_and_returns_only_after_its_voice_ends() {
    let (h, mut r) = settled();
    let old = clip_of(1, vec![[0.5; 2]; SR as usize * 2], false);
    h.load_and_play(0, old.clone()).unwrap();
    render(&mut r, SR as usize / 2);
    assert_eq!(Arc::strong_count(&old), 2);
    h.load_and_play(0, clip_of(2, vec![[0.; 2]; SR as usize], false))
        .unwrap();
    let output = render(&mut r, 64);
    h.collect();
    assert!(
        Arc::strong_count(&old) == 2,
        "the outgoing voice still reads the old PCM"
    );
    let rest = render(&mut r, 2048);
    let full = [output, rest].concat();
    let fade_slope = 0.5 * GAIN * std::f32::consts::PI / (2. * jump_frames() as f32);
    assert!(largest_step(&full) <= fade_slope * 1.1);
    assert!(full.last().unwrap().abs() < 1e-5);
    h.collect();
    assert_eq!(Arc::strong_count(&old), 1, "the callback never drops PCM");
}

#[test]
fn set_cue_and_loop_edges_use_the_nearest_beat() {
    let (h, mut r) = engine(SR);
    h.load(0, clip_of(1, vec![[0.1; 2]; SR as usize * 20], true))
        .unwrap();
    render(&mut r, 1);
    let beat = |n: f64| OFFSET + n * BEAT;
    h.control(0, Control::Seek(beat(10.) - 0.005)).unwrap();
    h.control(0, Control::SetCue(0)).unwrap();
    render(&mut r, 1);
    assert_eq!(h.snapshot().slots[0].cues[0], beat(10.));
    h.control(0, Control::Seek(beat(4.) + 0.01)).unwrap();
    h.control(0, Control::LoopIn).unwrap();
    h.control(0, Control::Seek(beat(6.) + 0.01)).unwrap();
    h.control(0, Control::LoopOut).unwrap();
    render(&mut r, 1);
    let s = h.snapshot().slots[0].clone();
    assert_eq!(s.loop_in, beat(4.));
    assert_eq!(s.loop_out, beat(6.));
    // OUT pressed on IN's own beat still produces a whole beat.
    h.control(0, Control::Seek(beat(4.) + 0.02)).unwrap();
    h.control(0, Control::LoopOut).unwrap();
    render(&mut r, 1);
    assert_eq!(h.snapshot().slots[0].loop_out, beat(5.));
    // A beat loop pressed just before a beat starts on it instead of a beat earlier.
    h.control(0, Control::Seek(beat(8.) - 0.01)).unwrap();
    h.control(0, Control::LoopBeats(2.)).unwrap();
    render(&mut r, 1);
    assert_eq!(h.snapshot().slots[0].loop_in, beat(8.));
}

#[test]
fn quantize_never_moves_a_saved_cue() {
    let (h, mut r) = engine(SR);
    h.load(0, clip_of(1, vec![[0.1; 2]; SR as usize * 20], true))
        .unwrap();
    render(&mut r, 1);
    h.control(0, Control::Quantize(false)).unwrap();
    h.control(0, Control::Seek(10.3)).unwrap();
    h.control(0, Control::SetCue(1)).unwrap();
    h.control(0, Control::Quantize(true)).unwrap();
    h.control(0, Control::Cue(1)).unwrap();
    render(&mut r, jump_frames());
    let s = h.snapshot().slots[0].clone();
    assert_eq!(s.cues[1], 10.3);
    assert!((s.position - 10.3).abs() < 1e-6, "{}", s.position);
    // A cue that was never saved reads 0:00 and starts at the first beat.
    h.control(0, Control::Cue(2)).unwrap();
    render(&mut r, jump_frames());
    assert!((h.snapshot().slots[0].position - OFFSET).abs() < 1e-6);
}

#[test]
fn an_unsaved_cue_starts_at_the_first_sound() {
    let mut clip = clip_of(1, vec![[0.1; 2]; SR as usize * 20], true);
    let music = &mut Arc::get_mut(&mut clip).unwrap().music;
    music.first_sound = 3.2;
    music.downbeat = OFFSET + BEAT;
    let (h, mut r) = engine(SR);
    h.load(0, clip).unwrap();
    h.control(0, Control::Cue(0)).unwrap();
    render(&mut r, jump_frames());
    let s = h.snapshot().slots[0].clone();
    // Q puts it on the nearest beat: 3.125 is the grid line closest to 3.2.
    assert!((s.position - 3.125).abs() < 1e-6, "{}", s.position);
    assert_eq!(s.music.first_sound, 3.2);
    assert_eq!(s.music.downbeat, OFFSET + BEAT);
    h.control(0, Control::Quantize(false)).unwrap();
    h.control(0, Control::Cue(0)).unwrap();
    render(&mut r, jump_frames());
    assert!((h.snapshot().slots[0].position - 3.2).abs() < 1e-6);
}

fn two_decks(follower_seconds: f64, master_seconds: f64) -> (EngineHandle, Renderer) {
    let (h, mut r) = engine(SR);
    let make = |id| clip_of(id, vec![[0.1; 2]; SR as usize * 40], true);
    h.load_and_play(0, make(1)).unwrap();
    h.load_and_play(1, make(2)).unwrap();
    h.control(0, Control::KeyLock(false)).unwrap();
    h.control(1, Control::KeyLock(false)).unwrap();
    h.control(0, Control::Seek(master_seconds)).unwrap();
    h.control(1, Control::Seek(follower_seconds)).unwrap();
    render(&mut r, 256);
    (h, r)
}
fn phase(slot: &SlotSnapshot) -> f64 {
    ((slot.position - OFFSET) / BEAT).rem_euclid(1.)
}

#[test]
fn sync_takes_the_short_way_to_the_master_phase() {
    // Follower 0.1 beat behind across a beat line: the old floor moved it back 0.9 beats.
    let (h, mut r) = two_decks(OFFSET + 20. - 0.025, OFFSET + 10. + 0.025);
    let before = h.snapshot().slots[1].position;
    h.control(1, Control::Sync(true)).unwrap();
    render(&mut r, 256);
    let s = h.snapshot();
    let moved = s.slots[1].position - before - 256. / SR as f64;
    assert!((moved - 0.05).abs() < 1e-6, "moved {moved}");
    assert!((phase(&s.slots[0]) - phase(&s.slots[1])).abs() < 1e-6);
}

#[test]
fn nudge_changes_the_phase_of_a_synced_deck() {
    let (h, mut r) = two_decks(OFFSET + 20., OFFSET + 10.);
    h.control(1, Control::Sync(true)).unwrap();
    render(&mut r, 256);
    let before = h.snapshot().slots[1].position;
    h.control(1, Control::Nudge(0.02)).unwrap();
    render(&mut r, 256);
    let s = h.snapshot();
    assert!(s.slots[1].sync);
    assert!((s.slots[1].position - before - 256. / SR as f64 - 0.02).abs() < 1e-6);
    let offset = (phase(&s.slots[1]) - phase(&s.slots[0])) * BEAT;
    assert!((offset - 0.02).abs() < 1e-6, "{offset}");
    // Nudging the master moves the relative phase too, without dragging the follower along.
    let before = h.snapshot().slots[1].position;
    h.control(0, Control::Nudge(0.02)).unwrap();
    render(&mut r, 256);
    let s = h.snapshot();
    assert!((s.slots[1].position - before - 256. / SR as f64).abs() < 1e-6);
}

#[test]
fn paused_follower_keeps_its_position_until_it_starts() {
    let (h, mut r) = two_decks(OFFSET + 20.1, OFFSET + 10.);
    h.control(1, Control::Sync(true)).unwrap();
    render(&mut r, 256);
    h.control(1, Control::Pause).unwrap();
    render(&mut r, 256);
    let paused = h.snapshot().slots[1].position;
    render(&mut r, 256);
    assert_eq!(h.snapshot().slots[1].position, paused);
    h.control(1, Control::Seek(25.3)).unwrap();
    render(&mut r, 256);
    assert_eq!(h.snapshot().slots[1].position, 25.3);
    h.control(1, Control::Play).unwrap();
    render(&mut r, 256);
    let s = h.snapshot();
    assert!((phase(&s.slots[0]) - phase(&s.slots[1])).abs() < 1e-6);
}

#[test]
fn synced_cue_lands_on_the_master_beat_and_on_its_own_grid() {
    let (h, mut r) = two_decks(OFFSET + 20., OFFSET + 10. + BEAT * 0.5);
    let saved = OFFSET + 4. * BEAT + 0.02;
    h.control(1, Control::Quantize(false)).unwrap();
    h.control(1, Control::Seek(saved)).unwrap();
    h.control(1, Control::SetCue(0)).unwrap();
    h.control(1, Control::Quantize(true)).unwrap();
    h.control(1, Control::Sync(true)).unwrap();
    render(&mut r, 256);
    assert_eq!(h.snapshot().slots[1].cues[0], saved, "phase needs a copy");
    h.control(1, Control::Cue(0)).unwrap();
    render(&mut r, 256);
    assert!(h.snapshot().slots[1].queued);
    // Run until the master crosses its next beat; both must then be exactly on a beat.
    let target = OFFSET + 10.5;
    while h.snapshot().slots[0].position < target - 0.001 {
        render(&mut r, 16);
    }
    assert!(!h.snapshot().slots[1].queued);
    render(&mut r, 256);
    let s = h.snapshot();
    let lag = (s.slots[0].position - target) - (s.slots[1].position - (OFFSET + 4. * BEAT));
    assert!(lag.abs() < 1e-3, "{lag}");
}

#[test]
fn hot_mix_is_limited_without_clipping_and_the_indicator_recovers() {
    let (h, mut r) = engine(SR);
    let hot: Vec<_> = sine(6, 55., 1.)
        .iter()
        .enumerate()
        .map(|(i, frame)| {
            let kick = if i % 24000 < 2400 { 1. } else { 0.6 };
            [frame[0] * kick, frame[1] * kick]
        })
        .collect();
    let make = |id| clip_of(id, hot.clone(), true);
    h.load_and_play(0, make(1)).unwrap();
    h.load_and_play(1, make(2)).unwrap();
    for slot in 0..2 {
        h.control(slot, Control::Low(2.)).unwrap();
        h.control(slot, Control::Gain(1.)).unwrap();
    }
    h.control(0, Control::Master(1.)).unwrap();
    let output = render(&mut r, SR as usize * 3);
    let s = h.snapshot();
    let peak = output.iter().fold(0_f32, |max, v| max.max(v.abs()));
    assert!(peak <= limiter::CEILING, "{peak}");
    assert_eq!(s.clipped, 0);
    assert!(s.limited > 0);
    assert!(s.limiter_db > 3., "{}", s.limiter_db);
    h.control(0, Control::PauseBoth).unwrap();
    render(&mut r, SR as usize * 2);
    let s = h.snapshot();
    assert_eq!(s.limiter_db, 0.);
}

#[test]
fn quiet_mix_never_touches_the_limiter() {
    let (h, mut r) = engine(SR);
    h.load_and_play(0, clip_of(1, sine(2, 220., 0.5), false))
        .unwrap();
    render(&mut r, SR as usize);
    let s = h.snapshot();
    assert_eq!(s.limited, 0);
    assert_eq!(s.limiter_db, 0.);
}
