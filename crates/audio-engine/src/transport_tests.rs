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
        loudness: None,
        beats: None,
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

#[test]
fn all_eight_hot_cues_work_and_a_saved_loop_plays_as_a_loop() {
    let clip = clip_of(1, vec![[0.1; 2]; SR as usize * 20], false);
    for cue in 0..annotations::HOT_CUES {
        clip.cues.set(cue, 1. + cue as f64);
    }
    let mut values = [0.; annotations::HOT_CUES];
    let mut ends = [0.; annotations::HOT_CUES];
    values[7] = 8.;
    ends[7] = 8.5;
    clip.cues.replace(values, ends);
    let (h, mut r) = engine(SR);
    h.load(0, clip.clone()).unwrap();
    render(&mut r, 1);
    h.control(0, Control::Cue(7)).unwrap();
    render(&mut r, SR as usize * 2);
    let s = h.snapshot().slots[0].clone();
    assert!(s.playing && s.looping);
    assert_eq!((s.cues[7], s.cue_ends[7]), (8., 8.5));
    assert_eq!((s.loop_in, s.loop_out), (8., 8.5));
    // A loop's crossfade starts a few milliseconds before IN, then stays inside it.
    assert!((7.99..8.5).contains(&s.position), "{}", s.position);
    assert!(h.control(0, Control::Cue(annotations::HOT_CUES)).is_err());
    // Saving there makes it a plain cue, and jumping to it leaves the loop.
    h.control(0, Control::Quantize(false)).unwrap();
    h.control(0, Control::Seek(3.)).unwrap();
    h.control(0, Control::SetCue(7)).unwrap();
    h.control(0, Control::Cue(7)).unwrap();
    render(&mut r, 256);
    let s = h.snapshot().slots[0].clone();
    assert_eq!(s.cue_ends[7], 0.);
    assert!(!s.looping);
}

#[test]
fn an_imported_grid_gets_its_first_sound_from_the_audio() {
    let path = std::env::temp_dir().join(format!("curado-first-{}.wav", std::process::id()));
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 8000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&path, spec).unwrap();
    for frame in 0..8000 * 2 {
        let value = if frame < 4000 { 0_i16 } else { 8000 };
        writer.write_sample(value).unwrap();
        writer.write_sample(value).unwrap();
    }
    writer.finalize().unwrap();
    let music = MusicAnalysis {
        version: 1,
        bpm: Some(120.),
        imported: true,
        ..Default::default()
    };
    let clip = decode::decode_with_music(&path, 1, Some(music)).unwrap();
    assert_eq!(clip.music.first_sound, 0.5);
    assert_eq!(clip.music.bpm, Some(120.), "the export's grid is kept");
    std::fs::remove_file(path).unwrap();
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

fn position(h: &EngineHandle) -> f64 {
    h.snapshot().slots[0].position
}
fn playing_deck(seconds: usize) -> (EngineHandle, Renderer) {
    let (h, mut r) = settled();
    h.load_and_play(0, clip_of(1, sine(seconds, 110., 0.5), true))
        .unwrap();
    render(&mut r, SR as usize);
    (h, r)
}

#[test]
fn slip_resumes_where_the_track_would_have_been() {
    let (h, mut r) = playing_deck(30);
    h.control(0, Control::Slip(true)).unwrap();
    h.control(0, Control::LoopBeats(1.)).unwrap();
    render(&mut r, 512);
    let start = position(&h);
    render(&mut r, SR as usize * 3);
    assert!(h.snapshot().slots[0].looping);
    assert!(position(&h) < start + 1., "it stays in the loop");
    h.control(0, Control::Loop).unwrap();
    render(&mut r, 512);
    let after = position(&h);
    assert!((after - (start + 3.)).abs() < 0.05, "{after} from {start}");
}

#[test]
fn without_slip_a_loop_ends_where_it_is() {
    let (h, mut r) = playing_deck(30);
    h.control(0, Control::LoopBeats(1.)).unwrap();
    render(&mut r, SR as usize * 3);
    h.control(0, Control::Loop).unwrap();
    render(&mut r, 512);
    assert!(position(&h) < 2.5, "{}", position(&h));
}

#[test]
fn a_roll_repeats_on_the_grid_then_returns_and_restores_the_loop() {
    let (h, mut r) = playing_deck(30);
    let before = position(&h);
    h.control(0, Control::Roll(0.25)).unwrap();
    render(&mut r, 512);
    let s = h.snapshot().slots[0].clone();
    assert!(s.rolling && s.looping);
    assert!((s.loop_out - s.loop_in - 0.125).abs() < 1e-6);
    // Quantized: the slice starts on a multiple of its own length from the grid.
    let cells = (s.loop_in - OFFSET) / 0.125;
    assert!((cells - cells.round()).abs() < 1e-6, "{cells}");
    render(&mut r, SR as usize * 2);
    assert!(position(&h) < s.loop_out + 0.01);
    h.control(0, Control::RollEnd).unwrap();
    render(&mut r, 512);
    let s = h.snapshot().slots[0].clone();
    assert!(!s.rolling && !s.looping);
    assert!((s.position - (before + 2.)).abs() < 0.05, "{}", s.position);
    // A roll on a paused deck has nothing to repeat, and a stray release does nothing.
    h.control(0, Control::Pause).unwrap();
    render(&mut r, 512);
    h.control(0, Control::Roll(1.)).unwrap();
    h.control(0, Control::RollEnd).unwrap();
    render(&mut r, 512);
    assert!(!h.snapshot().slots[0].rolling);
}

#[test]
fn a_new_roll_size_keeps_the_same_return_point() {
    let (h, mut r) = playing_deck(30);
    let before = position(&h);
    h.control(0, Control::Roll(1.)).unwrap();
    render(&mut r, SR as usize);
    h.control(0, Control::Roll(0.5)).unwrap();
    render(&mut r, SR as usize);
    h.control(0, Control::RollEnd).unwrap();
    render(&mut r, 512);
    assert!((position(&h) - (before + 2.)).abs() < 0.05);
}

#[test]
fn a_beat_jump_moves_whole_beats_and_takes_the_loop_along() {
    let (h, mut r) = playing_deck(30);
    let before = position(&h);
    h.control(0, Control::BeatJump(8.)).unwrap();
    render(&mut r, 512);
    assert!((position(&h) - (before + 4.)).abs() < 0.02);
    h.control(0, Control::LoopBeats(4.)).unwrap();
    render(&mut r, 512);
    let loop_in = h.snapshot().slots[0].loop_in;
    h.control(0, Control::BeatJump(-4.)).unwrap();
    render(&mut r, 512);
    let s = h.snapshot().slots[0].clone();
    assert!(s.looping);
    assert!((s.loop_in - (loop_in - 2.)).abs() < 1e-6);
    // Off the start of the track the jump lands at zero instead of wrapping.
    h.control(0, Control::BeatJump(-64.)).unwrap();
    render(&mut r, 512);
    assert!(position(&h) < 0.1);
    assert!(!h.snapshot().slots[0].looping);
}

#[test]
fn a_loop_can_double_and_halve_within_reason() {
    let (h, mut r) = playing_deck(30);
    h.control(0, Control::LoopBeats(4.)).unwrap();
    render(&mut r, 512);
    let span = |h: &EngineHandle| {
        let s = h.snapshot().slots[0].clone();
        s.loop_out - s.loop_in
    };
    h.control(0, Control::LoopScale(0.5)).unwrap();
    render(&mut r, 512);
    assert!((span(&h) - 1.).abs() < 1e-6);
    assert_eq!(h.snapshot().slots[0].loop_beats, 2.);
    h.control(0, Control::LoopScale(2.)).unwrap();
    render(&mut r, 512);
    assert!((span(&h) - 2.).abs() < 1e-6);
    for _ in 0..12 {
        h.control(0, Control::LoopScale(0.5)).unwrap();
    }
    render(&mut r, 512);
    assert!(span(&h) * SR as f64 >= (SR as f64 * LOOP_FADE_SECONDS).round() * 2. - 1.);
}

#[test]
fn reverse_plays_backward_and_stops_at_the_start() {
    let (h, mut r) = playing_deck(10);
    h.control(0, Control::Reverse(true)).unwrap();
    let before = position(&h);
    let output = render(&mut r, SR as usize / 2);
    assert!(h.snapshot().slots[0].reverse);
    assert!((position(&h) - (before - 0.5)).abs() < 0.01);
    assert!(output.iter().skip(1000).any(|v| v.abs() > 0.1));
    render(&mut r, SR as usize);
    assert!(!h.snapshot().slots[0].playing);
    assert_eq!(position(&h), 0.);
}

#[test]
fn reverse_stays_inside_a_loop() {
    let (h, mut r) = playing_deck(30);
    h.control(0, Control::LoopBeats(2.)).unwrap();
    render(&mut r, 512);
    let s = h.snapshot().slots[0].clone();
    h.control(0, Control::Reverse(true)).unwrap();
    for _ in 0..40 {
        render(&mut r, 2400);
        let at = position(&h);
        assert!(at >= s.loop_in - 0.02 && at <= s.loop_out + 0.02, "{at}");
    }
    assert!(h.snapshot().slots[0].playing);
}

#[test]
fn slip_and_reverse_carry_on_underneath() {
    let (h, mut r) = playing_deck(30);
    h.control(0, Control::Slip(true)).unwrap();
    let before = position(&h);
    h.control(0, Control::Reverse(true)).unwrap();
    render(&mut r, SR as usize);
    h.control(0, Control::Reverse(false)).unwrap();
    render(&mut r, 512);
    assert!(
        (position(&h) - (before + 1.)).abs() < 0.05,
        "{}",
        position(&h)
    );
}

#[test]
fn key_lock_gives_way_to_reverse_and_comes_back() {
    let (h, mut r) = playing_deck(10);
    h.control(0, Control::KeyLock(true)).unwrap();
    h.control(0, Control::Rate(1.1)).unwrap();
    render(&mut r, 9600);
    assert!(r.decks[0].main_pitched);
    h.control(0, Control::Reverse(true)).unwrap();
    render(&mut r, 4800);
    assert!(!r.decks[0].main_pitched);
    h.control(0, Control::Reverse(false)).unwrap();
    let output = render(&mut r, 48000);
    assert!(r.decks[0].main_pitched);
    assert!(output.iter().all(|v| v.is_finite()));
}

#[test]
fn a_brake_winds_down_and_play_catches_it() {
    let (h, mut r) = playing_deck(10);
    h.control(0, Control::Brake(1.)).unwrap();
    h.control(0, Control::Pause).unwrap();
    render(&mut r, 512);
    let a = position(&h);
    render(&mut r, SR as usize / 2);
    let moved = position(&h) - a;
    // A linear wind-down over one second covers less than the half second at full speed.
    assert!(moved > 0.2 && moved < 0.45, "{moved}");
    assert!(h.snapshot().slots[0].playing);
    render(&mut r, SR as usize);
    assert!(!h.snapshot().slots[0].playing);
    let stopped = position(&h);
    render(&mut r, 512);
    assert_eq!(position(&h), stopped);

    let (h, mut r) = playing_deck(10);
    h.control(0, Control::Brake(1.)).unwrap();
    h.control(0, Control::Pause).unwrap();
    render(&mut r, SR as usize / 4);
    h.control(0, Control::Play).unwrap();
    render(&mut r, SR as usize);
    assert!(h.snapshot().slots[0].playing);
    assert_eq!(r.decks[0].ramp, 1.);

    let (h, mut r) = playing_deck(10);
    h.control(0, Control::Brake(3.)).unwrap();
    h.control(0, Control::Panic).unwrap();
    render(&mut r, 512);
    assert!(
        !h.snapshot().slots[0].playing,
        "panic does not wait for a brake"
    );
}

#[test]
fn the_platter_follows_the_hand_and_lets_go() {
    let (h, mut r) = playing_deck(10);
    h.control(0, Control::Pause).unwrap();
    render(&mut r, 2048);
    let start = position(&h);
    h.control(0, Control::JogTouch(true)).unwrap();
    h.control(0, Control::JogMove(-0.5)).unwrap();
    let moving = render(&mut r, SR as usize / 4);
    assert!(
        (position(&h) - (start - 0.5)).abs() < 0.02,
        "{}",
        position(&h)
    );
    assert!(
        moving.iter().any(|v| v.abs() > 0.05),
        "a moving record sounds"
    );
    let held = render(&mut r, SR as usize / 4);
    assert!(
        held[SR as usize / 8..].iter().all(|v| v.abs() < 1e-3),
        "a held record is silent"
    );
    h.control(0, Control::JogMove(100.)).unwrap();
    render(&mut r, SR as usize);
    assert!(position(&h) <= h.snapshot().slots[0].duration);
    h.control(0, Control::JogTouch(false)).unwrap();
    h.control(0, Control::JogMove(-1.)).unwrap();
    render(&mut r, 512);
    assert!(!r.decks[0].jog_touch);
}

#[test]
fn a_pitch_bend_speeds_up_and_relaxes_without_a_release() {
    let (h, mut r) = playing_deck(10);
    let before = position(&h);
    h.control(0, Control::JogBend(0.2)).unwrap();
    render(&mut r, 2400);
    let bent = position(&h) - before;
    assert!(bent > 2400. / SR as f64 * 1.02, "{bent}");
    render(&mut r, SR as usize);
    assert_eq!(r.decks[0].bend, 0.);
}

#[test]
fn the_player_clears_what_it_has_no_controls_for() {
    let (h, mut r) = playing_deck(10);
    h.control(0, Control::Slip(true)).unwrap();
    h.control(0, Control::Reverse(true)).unwrap();
    h.control(0, Control::Color(0.8)).unwrap();
    h.control(0, Control::ColorFx(6)).unwrap();
    h.control(0, Control::BeatFxOn(true)).unwrap();
    h.control(MASTER_UNIT, Control::BeatFxOn(true)).unwrap();
    render(&mut r, 512);
    assert!(h.snapshot().beat_fx[2].on);
    h.control(0, Control::SetMode(false)).unwrap();
    render(&mut r, 512);
    let s = h.snapshot();
    assert!(!s.slots[0].slip && !s.slots[0].reverse);
    assert_eq!((s.slots[0].color, s.slots[0].color_fx), (0., 0));
    assert!(s.beat_fx.iter().all(|unit| !unit.on));
}

#[test]
fn each_channel_and_the_master_keep_their_own_effects() {
    let (h, mut r) = settled();
    h.control(0, Control::BeatFx(3)).unwrap();
    h.control(0, Control::BeatFxBeats(2.)).unwrap();
    h.control(1, Control::BeatFx(7)).unwrap();
    h.control(1, Control::BeatFxOn(true)).unwrap();
    h.control(MASTER_UNIT, Control::BeatFx(4)).unwrap();
    h.control(MASTER_UNIT, Control::BeatFxDepth(0.25)).unwrap();
    h.control(0, Control::ColorFx(1)).unwrap();
    h.control(1, Control::ColorFx(5)).unwrap();
    render(&mut r, 512);
    let s = h.snapshot();
    let kinds: Vec<u8> = s.beat_fx.iter().map(|unit| unit.kind).collect();
    assert_eq!(kinds, [3, 7, 4]);
    assert!(!s.beat_fx[0].on && s.beat_fx[1].on && !s.beat_fx[2].on);
    assert_eq!((s.beat_fx[0].beats, s.beat_fx[2].depth), (2., 0.25));
    assert_eq!((s.slots[0].color_fx, s.slots[1].color_fx), (1, 5));
    // Samples have no channel effects, and only slot 2 stands for the master unit.
    assert!(h.control(3, Control::BeatFxOn(true)).is_err());
    assert!(h.control(2, Control::ColorFx(1)).is_err());
    let rejected = |slot: usize, kind: &str, value: f64| {
        h.action(Action {
            slot,
            kind: kind.into(),
            value,
        })
        .is_err()
    };
    assert!(rejected(0, "colorFx", 7.));
    assert!(rejected(0, "beatFx", 14.));
    assert!(!rejected(0, "beatFx", 13.));
    assert!(rejected(0, "crossfaderCurve", 3.));
    assert!(rejected(0, "clearCue", 8.));
    assert!(rejected(0, "beatFxBeats", 3.));
    assert!(rejected(0, "color", 1.5));
    assert!(rejected(2, "roll", 1.));
    assert!(rejected(0, "roll", 3.));
    assert!(rejected(0, "beatJump", 0.));
    assert!(rejected(0, "loopScale", 3.));
    assert!(!rejected(2, "beatFxOn", 1.));
}

#[test]
fn a_master_effect_reaches_the_output_of_both_decks() {
    let (h, mut r) = engine(SR);
    h.load_and_play(0, clip_of(1, sine(4, 220., 0.5), false))
        .unwrap();
    h.control(0, Control::Crossfader(0.)).unwrap();
    render(&mut r, SR as usize);
    let plain = largest_step(&render(&mut r, 4800));
    h.control(MASTER_UNIT, Control::BeatFx(7)).unwrap();
    h.control(MASTER_UNIT, Control::BeatFxDepth(1.)).unwrap();
    h.control(MASTER_UNIT, Control::BeatFxBeats(0.25)).unwrap();
    h.control(MASTER_UNIT, Control::BeatFxOn(true)).unwrap();
    let gated = render(&mut r, SR as usize);
    assert!(gated.iter().filter(|v| v.abs() < 1e-3).count() > SR as usize / 4);
    assert!(largest_step(&gated) < plain * 3., "the gate does not click");
}

#[test]
fn auto_gain_brings_tracks_to_the_same_level_and_is_off_by_default() {
    let measure = |amplitude: f32, auto: bool| {
        let (h, mut r) = settled();
        let samples = sine(4, 997., amplitude);
        let clip = Clip {
            loudness: crate::loudness::integrated(&samples, SR),
            beats: None,
            id: 1,
            cues: Default::default(),
            samples,
            sample_rate: SR,
            waveform: vec![],
            title: None,
            artist: None,
            music: Default::default(),
        };
        h.control(0, Control::Master(1.)).unwrap();
        h.control(0, Control::AutoGain(auto)).unwrap();
        h.load_and_play(0, Arc::new(clip)).unwrap();
        render(&mut r, SR as usize);
        render(&mut r, SR as usize)
            .iter()
            .fold(0_f32, |peak, v| peak.max(v.abs()))
    };
    let (quiet, loud) = (measure(0.08, false), measure(0.2, false));
    assert!((loud / quiet - 2.5).abs() < 0.1, "off keeps the difference");
    let (quiet, loud) = (measure(0.08, true), measure(0.2, true));
    assert!((loud / quiet - 1.).abs() < 0.02, "{quiet} {loud}");
}

#[test]
fn eight_samples_play_under_a_shared_level() {
    let level = |sampler: f32| {
        let (h, mut r) = settled();
        for slot in [2, 9] {
            h.load(slot, clip_of(slot as u64, sine(2, 440., 0.2), false))
                .unwrap();
            render(&mut r, 256);
            h.control(slot, Control::Trigger).unwrap();
        }
        h.control(2, Control::Sampler(sampler)).unwrap();
        render(&mut r, SR as usize / 4);
        let snapshot = h.snapshot();
        assert!(snapshot.slots[9].playing);
        assert_eq!(snapshot.sampler, sampler);
        render(&mut r, 4800)
            .iter()
            .fold(0_f32, |peak, v| peak.max(v.abs()))
    };
    assert!(level(1.) > 0.01);
    assert!(level(0.) < 1e-3);
}

#[test]
fn a_cue_saved_at_the_start_is_a_cue() {
    let mut samples = vec![[0.; 2]; SR as usize * 2];
    samples[0] = [0.5; 2];
    samples[SR as usize / 2..].fill([0.2; 2]);
    let (h, mut r) = settled();
    let clip = clip_of(1, samples, false);
    let mut music = clip.music;
    music.first_sound = 0.5;
    h.load(0, clip.clone()).unwrap();
    render(&mut r, 256);
    h.set_music(1, music).unwrap();
    render(&mut r, 256);
    // Unset, a cue goes to the first sound; saved at 0:00, it stays there.
    h.control(0, Control::Cue(0)).unwrap();
    render(&mut r, 256);
    assert!((position(&h) - 0.5).abs() < 0.02, "{}", position(&h));
    h.control(0, Control::Pause).unwrap();
    h.control(0, Control::Seek(0.)).unwrap();
    render(&mut r, 256);
    h.control(0, Control::SetCue(0)).unwrap();
    render(&mut r, 256);
    let saved = h.snapshot().slots[0].cues[0];
    assert!(saved > 0. && saved < 1e-3, "{saved}");
    h.control(0, Control::Seek(1.)).unwrap();
    render(&mut r, 256);
    h.control(0, Control::Cue(0)).unwrap();
    render(&mut r, 256);
    assert!(position(&h) < 0.02, "{}", position(&h));
}

#[test]
fn live_positions_follow_the_engine_and_stop_with_it() {
    let (h, mut r) = playing_deck(10);
    let first = h.live_positions().unwrap();
    assert_eq!(first[0], position(&h));
    render(&mut r, 2400);
    let second = h.live_positions().unwrap();
    assert!((second[0] - first[0] - 0.05).abs() < 1e-3);
    h.control(0, Control::Pause).unwrap();
    render(&mut r, 2400);
    assert_eq!(h.live_positions(), None);
}

#[test]
fn a_hot_cue_can_be_deleted() {
    let (h, mut r) = playing_deck(10);
    h.control(0, Control::Quantize(false)).unwrap();
    h.control(0, Control::SetCue(2)).unwrap();
    render(&mut r, 512);
    assert!(h.snapshot().slots[0].cues[2] > 0.);
    h.control(0, Control::ClearCue(2)).unwrap();
    render(&mut r, 512);
    assert_eq!(h.snapshot().slots[0].cues[2], 0.);
}

fn mapped(id: u64, seconds: usize, beats: Vec<f64>) -> Arc<Clip> {
    let clip = clip_of(id, sine(seconds, 110., 0.5), true);
    let clip = Arc::try_unwrap(clip).unwrap_or_else(|_| unreachable!());
    Arc::new(Clip {
        beats: Some(beats.into()),
        music: MusicAnalysis {
            manual: false,
            ..clip.music
        },
        ..clip
    })
}
/// Beats that speed up from 120 to about 132 BPM over the track.
fn rising(seconds: f64) -> Vec<f64> {
    let (mut time, mut beats) = (OFFSET, Vec::new());
    while time < seconds {
        beats.push(time);
        time += 60. / (120. + 12. * time / seconds);
    }
    beats
}

#[test]
fn a_beat_map_drives_quantize_loops_and_jumps() {
    let beats = rising(30.);
    let (h, mut r) = settled();
    h.load_and_play(0, mapped(1, 30, beats.clone())).unwrap();
    render(&mut r, SR as usize * 20);
    h.control(0, Control::LoopBeats(4.)).unwrap();
    render(&mut r, 256);
    let s = h.snapshot().slots[0].clone();
    assert!(s.looping);
    // The loop starts on a real beat and lasts exactly four of them, wherever the tempo is.
    let at = beats
        .iter()
        .position(|b| (b - s.loop_in).abs() < 1e-3)
        .expect("on a beat");
    assert!(
        (s.loop_out - beats[at + 4]).abs() < 1e-3,
        "{} {}",
        s.loop_out,
        beats[at + 4]
    );
    h.control(0, Control::Loop).unwrap();
    render(&mut r, 256);
    let before = position(&h);
    let index = beats.partition_point(|b| *b <= before) - 1;
    let inside = before - beats[index];
    h.control(0, Control::BeatJump(-8.)).unwrap();
    render(&mut r, 256);
    let expected = beats[index - 8]
        + inside / (beats[index + 1] - beats[index]) * (beats[index - 7] - beats[index - 8]);
    assert!(
        (position(&h) - expected).abs() < 0.02,
        "{} {expected}",
        position(&h)
    );
}

#[test]
fn a_correction_by_hand_replaces_a_beat_map() {
    let (h, mut r) = settled();
    let clip = mapped(1, 20, rising(20.));
    h.load_and_play(0, clip.clone()).unwrap();
    render(&mut r, SR as usize);
    assert!(r.decks[0].beat_map().is_some());
    h.set_music(
        1,
        MusicAnalysis {
            version: crate::music::ANALYSIS_VERSION,
            bpm: Some(120.),
            manual: true,
            ..Default::default()
        },
    )
    .unwrap();
    render(&mut r, 512);
    assert!(r.decks[0].beat_map().is_none());
}

#[test]
fn sync_stays_on_the_beat_of_a_moving_tempo() {
    let (h, mut r) = engine(SR);
    h.load_and_play(0, mapped(1, 40, rising(40.))).unwrap();
    render(&mut r, 512);
    h.load_and_play(1, clip_of(2, sine(60, 110., 0.5), true))
        .unwrap();
    render(&mut r, 512);
    h.control(1, Control::Sync(true)).unwrap();
    render(&mut r, SR as usize * 30);
    let s = h.snapshot();
    assert!(s.slots[1].sync);
    // The follower has moved off its own 120 BPM to keep up.
    assert!(s.slots[1].tuning.rate > 1.03, "{}", s.slots[1].tuning.rate);
    let lead = r.decks[0].beats_position().unwrap();
    let follow = r.decks[1].beats_position().unwrap();
    let gap = (follow - lead + 0.5).rem_euclid(1.) - 0.5;
    let beat_ms = 60_000. / r.decks[0].local_bpm().unwrap();
    assert!(gap.abs() * beat_ms < 15., "{} ms apart", gap * beat_ms);
}

fn run_to_silence(h: &EngineHandle, r: &mut Renderer) {
    for _ in 0..100 {
        if !h.snapshot().slots[0].playing {
            return;
        }
        render(r, 4800);
    }
    panic!("the deck never stopped");
}

#[test]
fn a_vinyl_brake_release_winds_down_and_stops() {
    for (kind, least) in [(0., 0.1), (1., 0.5)] {
        let (h, mut r) = playing_deck(30);
        let before = position(&h);
        h.action(Action {
            slot: 0,
            kind: "releaseFx".into(),
            value: kind,
        })
        .unwrap();
        render(&mut r, 256);
        assert!(h.snapshot().slots[0].playing, "it does not cut");
        run_to_silence(&h, &mut r);
        let moved = position(&h) - before;
        assert!(moved > least, "{moved}");
    }
}

#[test]
fn a_backspin_goes_backward_and_stops() {
    let (h, mut r) = playing_deck(30);
    render(&mut r, SR as usize * 4);
    let before = position(&h);
    h.control(0, Control::Release(3)).unwrap();
    let output = render(&mut r, 9600);
    assert!(position(&h) < before, "{}", position(&h));
    assert!(output.iter().any(|v| v.abs() > 0.05), "it is audible");
    run_to_silence(&h, &mut r);
    assert!(position(&h) < before - 0.5);
    render(&mut r, 2400);
    h.control(0, Control::Play).unwrap();
    render(&mut r, 2400);
    assert!(r.decks[0].spin_left == 0 && h.snapshot().slots[0].playing);
}

#[test]
fn echo_out_stops_the_track_keeps_the_repeats_and_restores_the_unit() {
    let (h, mut r) = playing_deck(30);
    h.control(0, Control::BeatFx(7)).unwrap();
    render(&mut r, 512);
    h.control(0, Control::Release(4)).unwrap();
    render(&mut r, SR as usize / 2);
    assert!(!h.snapshot().slots[0].playing);
    let tail = render(&mut r, 4800);
    assert!(tail.iter().any(|v| v.abs() > 0.01), "the repeats ring on");
    render(&mut r, SR as usize * 7);
    render(&mut r, 512);
    assert_eq!(
        h.snapshot().beat_fx[0].kind,
        7,
        "the unit is back as it was"
    );
}

#[test]
fn build_up_and_ducking_last_while_held() {
    let measure = |kind: u8| {
        let (h, mut r) = playing_deck(30);
        let peak = |v: &[f32]| v.iter().fold(0_f32, |p, x| p.max(x.abs()));
        let before = peak(&render(&mut r, 4800));
        h.control(0, Control::Release(kind)).unwrap();
        render(&mut r, SR as usize * 8);
        let held: Vec<f32> = render(&mut r, 24000);
        let during = held.iter().map(|v| v * v).sum::<f32>() / held.len() as f32;
        h.control(0, Control::ReleaseEnd).unwrap();
        render(&mut r, 4800);
        let after = peak(&render(&mut r, 4800));
        assert!(h.snapshot().slots[0].playing, "the deck keeps playing");
        (before, during.sqrt(), after)
    };
    for kind in [5, 7] {
        let (before, during, after) = measure(kind);
        assert!(during < before * 0.6, "{kind}: {during} against {before}");
        assert!(
            (after - before).abs() < before * 0.1,
            "{kind}: back to {after}"
        );
    }
    let (h, mut r) = playing_deck(30);
    h.control(0, Control::Release(6)).unwrap();
    render(&mut r, 512);
    assert!(!h.snapshot().slots[0].playing, "mute cuts at once");
}

#[test]
fn the_space_bar_action_pauses_anything_playing_or_starts_both() {
    let (h, mut r) = engine(SR);
    h.load(0, clip_of(1, sine(10, 110., 0.5), true)).unwrap();
    render(&mut r, 256);
    h.load(1, clip_of(2, sine(10, 110., 0.5), true)).unwrap();
    render(&mut r, 256);
    h.control(0, Control::ToggleBoth).unwrap();
    render(&mut r, 2048);
    let s = h.snapshot();
    assert!(s.slots[0].playing && s.slots[1].playing);
    h.control(1, Control::Pause).unwrap();
    render(&mut r, 2048);
    h.control(0, Control::ToggleBoth).unwrap();
    render(&mut r, 2048);
    let s = h.snapshot();
    assert!(
        !s.slots[0].playing && !s.slots[1].playing,
        "one playing is enough to pause"
    );
}

fn render_channels(r: &mut Renderer, frames: usize, channels: usize) -> Vec<f32> {
    let mut out = vec![0_f32; frames * channels];
    for block in out.chunks_mut(256 * channels) {
        r.render(block, channels);
    }
    out
}

#[test]
fn a_cue_bus_goes_out_on_channels_three_and_four() {
    let (h, mut r) = settled();
    h.load_and_play(0, clip_of(1, sine(10, 220., 0.5), true))
        .unwrap();
    h.control(0, Control::Gain(0.)).unwrap();
    render_channels(&mut r, 4800, 4);
    let level = |out: &[f32], c: usize| out.chunks(4).fold(0_f32, |p, f| p.max(f[c].abs()));
    // Fader down: nothing on the mains and nothing cued.
    let silent = render_channels(&mut r, 4800, 4);
    assert!(level(&silent, 0) < 1e-3 && level(&silent, 2) < 1e-3);
    // The cue is before the fader, so it is heard with the fader closed.
    h.control(0, Control::Cue2(true)).unwrap();
    render_channels(&mut r, 4800, 4);
    let cued = render_channels(&mut r, 4800, 4);
    assert!(level(&cued, 0) < 1e-3, "mains stay silent");
    assert!(level(&cued, 2) > 0.2, "{}", level(&cued, 2));
    h.control(0, Control::CueLevel(0.)).unwrap();
    render_channels(&mut r, 256, 4);
    assert!(level(&render_channels(&mut r, 2048, 4), 2) < 1e-3);
}

#[test]
fn a_preview_goes_to_the_cue_bus_or_quietly_to_the_mains() {
    let run = |channels: usize| {
        let (h, mut r) = settled();
        h.load(PREVIEW_SLOT, clip_of(9, sine(5, 220., 0.5), false))
            .unwrap();
        render_channels(&mut r, 256, channels);
        h.control(PREVIEW_SLOT, Control::Play).unwrap();
        render_channels(&mut r, 2400, channels);
        let out = render_channels(&mut r, 2400, channels);
        let level = |c: usize| out.chunks(channels).fold(0_f32, |p, f| p.max(f[c].abs()));
        (level(0), if channels >= 4 { level(2) } else { 0. })
    };
    let (mains, phones) = run(4);
    assert!(mains < 1e-3, "{mains}");
    assert!(phones > 0.1, "{phones}");
    let (mains, _) = run(2);
    assert!(mains > 0.05, "{mains}");
}

fn timed_deck() -> (EngineHandle, Renderer) {
    let (h, mut r) = settled();
    h.load(0, clip_of(1, sine(40, 55., 0.5), true)).unwrap();
    render(&mut r, 2048);
    (h, r)
}

#[test]
fn auto_loop_starts_and_ends_with_one_press_and_keeps_its_start_when_resized() {
    let (h, mut r) = timed_deck();
    h.control(0, Control::Seek(OFFSET + 8. * BEAT)).unwrap();
    h.control(0, Control::Play).unwrap();
    render(&mut r, 2048);
    h.control(0, Control::AutoLoop).unwrap();
    render(&mut r, 1);
    let first = h.snapshot().slots[0].clone();
    assert!(first.looping);
    assert_eq!(first.loop_beats, 4.);
    assert!((first.loop_out - first.loop_in - 4. * BEAT).abs() < 1e-6);

    // Longer and shorter, from the same start, and the read head stays inside.
    for beats in [8., 1., 0.25] {
        h.control(0, Control::LoopBeats(beats)).unwrap();
        render(&mut r, 4096);
        let s = h.snapshot().slots[0].clone();
        assert!(s.looping);
        assert_eq!(s.loop_in, first.loop_in);
        assert!((s.loop_out - s.loop_in - beats * BEAT).abs() < 1e-6);
        assert!(s.position >= s.loop_in && s.position <= s.loop_out + 0.01);
    }

    h.control(0, Control::AutoLoop).unwrap();
    render(&mut r, 1);
    assert!(!h.snapshot().slots[0].looping);
    // Again later: a new loop from where the music is, of the last length, not the old one.
    let from = h.snapshot().slots[0].position;
    render(&mut r, frame_at(4. * BEAT));
    h.control(0, Control::AutoLoop).unwrap();
    render(&mut r, 1);
    let again = h.snapshot().slots[0].clone();
    assert!(again.looping);
    assert_eq!(again.loop_beats, 0.25);
    assert!(again.loop_in > from);
}

#[test]
fn the_size_of_the_next_auto_loop_can_be_chosen_before_there_is_a_loop() {
    let (h, mut r) = timed_deck();
    h.control(0, Control::LoopScale(0.5)).unwrap();
    render(&mut r, 1);
    assert_eq!(h.snapshot().slots[0].loop_beats, 2.);
    for _ in 0..10 {
        h.control(0, Control::LoopScale(2.)).unwrap();
    }
    render(&mut r, 1);
    assert_eq!(
        h.snapshot().slots[0].loop_beats,
        32.,
        "goes no further than the pads"
    );
    h.control(0, Control::Seek(OFFSET + 8. * BEAT)).unwrap();
    h.control(0, Control::AutoLoop).unwrap();
    render(&mut r, 1);
    let s = h.snapshot().slots[0].clone();
    assert!(s.looping);
    assert!((s.loop_out - s.loop_in - 32. * BEAT).abs() < 1e-6);
}

#[test]
fn only_decks_take_an_auto_loop() {
    let action = |slot| Action {
        slot,
        kind: "autoLoop".into(),
        value: 0.,
    };
    assert!(action(1).validate().is_ok());
    assert!(action(2).validate().is_err());
}

#[test]
fn a_hot_cue_can_hold_a_loop() {
    let (h, mut r) = timed_deck();
    // With no loop: saved from here with the last length, and the track carries on.
    h.control(0, Control::Seek(OFFSET + 8. * BEAT)).unwrap();
    h.control(0, Control::Play).unwrap();
    h.control(0, Control::SetLoopCue(2)).unwrap();
    render(&mut r, 1024);
    let saved = h.snapshot().slots[0].clone();
    assert!(!saved.looping, "saving does not trap the track");
    assert!((saved.cues[2] - (OFFSET + 8. * BEAT)).abs() < 1e-6);
    assert!((saved.cue_ends[2] - saved.cues[2] - 4. * BEAT).abs() < 1e-6);

    // Calling it from elsewhere plays the loop.
    h.control(0, Control::Seek(OFFSET + 30. * BEAT)).unwrap();
    h.control(0, Control::Cue(2)).unwrap();
    render(&mut r, 4096);
    let called = h.snapshot().slots[0].clone();
    assert!(called.looping);
    assert!((called.loop_in - saved.cues[2]).abs() < 1e-6);
    assert!((called.loop_out - saved.cue_ends[2]).abs() < 1e-6);

    // During a loop, a plain save keeps the loop, whatever length it has now.
    h.control(0, Control::LoopBeats(2.)).unwrap();
    h.control(0, Control::SetCue(5)).unwrap();
    render(&mut r, 1);
    let looped = h.snapshot().slots[0].clone();
    assert!((looped.cues[5] - looped.loop_in).abs() < 1e-6);
    assert!((looped.cue_ends[5] - looped.cues[5] - 2. * BEAT).abs() < 1e-6);
    assert!(looped.looping, "saving does not leave it either");

    // Out of a loop, a plain save is a plain cue again, even over a loop cue.
    h.control(0, Control::AutoLoop).unwrap();
    h.control(0, Control::SetCue(5)).unwrap();
    render(&mut r, 1);
    assert_eq!(h.snapshot().slots[0].cue_ends[5], 0.);
    let action = |slot| Action {
        slot,
        kind: "setLoopCue".into(),
        value: 1.,
    };
    assert!(action(1).validate().is_ok());
    assert!(action(2).validate().is_err());
}
