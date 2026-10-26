use super::*;
#[test]
fn obsolete_tuning_batch_is_acknowledged_and_an_unchanged_deck_can_retry() {
    let (h, mut r) = engine(48000);
    let a = clip(1, 120.);
    let b = clip(2, 120.);
    h.load(0, a.clone()).unwrap();
    h.load(1, b.clone()).unwrap();
    h.control(0, Control::TransposeStep(1)).unwrap();
    h.control(1, Control::TransposeStep(1)).unwrap();
    render(&mut r, 1);
    let snap = h.snapshot();
    let job = |i: usize, source: Arc<Clip>| TunedClip {
        clip: source,
        load_ticket: snap.slots[i].load_ticket,
        revision: snap.slots[i].tune_revision,
        tuning: snap.slots[i].tuning,
    };
    let batch = [Some(job(0, a)), Some(job(1, b))];
    h.control(0, Control::TransposeStep(1)).unwrap();
    let ticket = h.apply_tuning(batch.clone()).unwrap();
    render(&mut r, 1);
    assert!(h.tuning_acknowledged(ticket));
    let s = h.snapshot();
    assert_eq!(s.slots[1].applied_tuning.semitones, 0);
    let ticket = h.apply_tuning([None, batch[1].clone()]).unwrap();
    render(&mut r, 1);
    assert!(h.tuning_acknowledged(ticket));
    assert_eq!(h.snapshot().slots[1].applied_tuning.semitones, 1);
}
#[test]
fn synced_quantized_cue_waits_for_the_master_beat_and_panic_cancels_it() {
    let (h, mut r) = engine(48000);
    h.load_and_play(0, clip(1, 120.)).unwrap();
    h.load_and_play(1, clip(2, 120.)).unwrap();
    h.control(0, Control::Seek(0.375)).unwrap();
    h.control(1, Control::Seek(1.375)).unwrap();
    h.control(1, Control::Sync(true)).unwrap();
    render(&mut r, 1);
    h.control(1, Control::Cue(0)).unwrap();
    render(&mut r, 11998);
    assert!(h.snapshot().slots[1].queued);
    assert!(h.snapshot().slots[1].position > 1.5);
    render(&mut r, 3);
    let s = h.snapshot();
    assert!(!s.slots[1].queued);
    assert!((s.slots[1].position - 0.125).abs() < 0.0001);
    h.control(0, Control::Seek(0.375)).unwrap();
    h.control(1, Control::Cue(0)).unwrap();
    render(&mut r, 1);
    assert!(h.snapshot().slots[1].queued);
    h.control(0, Control::Panic).unwrap();
    render(&mut r, 48000);
    assert!(!h.snapshot().slots[1].queued);
    assert!(!h.snapshot().slots[1].playing);
}
fn clip(id: u64, bpm: f64) -> Arc<Clip> {
    Arc::new(Clip {
        id,
        cues: Default::default(),
        samples: vec![[0.1, 0.1]; 480000],
        sample_rate: 48000,
        waveform: vec![],
        title: None,
        artist: None,
        music: MusicAnalysis {
            version: 1,
            bpm: Some(bpm),
            beat_offset: 0.125,
            bpm_confidence: 1.,
            key: Some(music::MusicalKey {
                tonic: 0,
                minor: false,
            }),
            key_confidence: 1.,
            manual: true,
        },
    })
}
fn render(r: &mut Renderer, n: usize) {
    let mut out = vec![0_f32; n * 2];
    r.render(&mut out, 2);
}
#[test]
fn quantized_loops_use_source_beats_and_never_create_partial_tail_loops() {
    let (h, mut r) = engine(48000);
    h.load(0, clip(1, 120.)).unwrap();
    render(&mut r, 1);
    assert!(h.snapshot().slots[0].quantize);
    h.control(0, Control::Seek(2.8)).unwrap();
    h.control(0, Control::LoopBeats(4.)).unwrap();
    render(&mut r, 1);
    let s = h.snapshot();
    assert_eq!(s.slots[0].loop_in, 2.625);
    assert_eq!(s.slots[0].loop_out, 4.625);
    h.control(0, Control::Seek(9.9)).unwrap();
    h.control(0, Control::LoopBeats(4.)).unwrap();
    render(&mut r, 1);
    let s = h.snapshot();
    assert_eq!(s.slots[0].loop_in, 7.625);
    assert_eq!(s.slots[0].loop_out, 9.625);
    h.control(0, Control::Seek(0.34)).unwrap();
    h.control(0, Control::SetCue(0)).unwrap();
    render(&mut r, 1);
    assert_eq!(h.snapshot().slots[0].cues[0], 0.125);
    h.control(0, Control::Quantize(false)).unwrap();
    h.control(0, Control::SetCue(1)).unwrap();
    render(&mut r, 1);
    assert_eq!(h.snapshot().slots[0].cues[1], 0.34);
}
#[test]
fn native_sync_matches_tempo_and_beat_phase_and_survives_quantized_loops() {
    let (h, mut r) = engine(48000);
    h.load_and_play(0, clip(1, 120.)).unwrap();
    h.load_and_play(1, clip(2, 100.)).unwrap();
    h.control(0, Control::KeyLock(false)).unwrap();
    h.control(1, Control::KeyLock(false)).unwrap();
    h.control(0, Control::Seek(0.375)).unwrap();
    h.control(1, Control::Seek(1.5)).unwrap();
    h.control(1, Control::Sync(true)).unwrap();
    render(&mut r, 256);
    let s = h.snapshot();
    assert!((s.slots[1].rate - 1.2).abs() < 1e-6);
    let phase = |s: &SlotSnapshot| {
        ((s.position - s.music.beat_offset) * s.music.bpm.unwrap() / 60.).rem_euclid(1.)
    };
    assert!((phase(&s.slots[0]) - phase(&s.slots[1])).abs() < 1e-6);
    h.control(0, Control::Tempo(126.)).unwrap();
    h.control(1, Control::LoopBeats(4.)).unwrap();
    render(&mut r, 240000);
    let s = h.snapshot();
    assert!((s.slots[0].rate as f64 * 120. - s.slots[1].rate as f64 * 100.).abs() < 1e-4);
    assert!((phase(&s.slots[0]) - phase(&s.slots[1])).abs() < 1e-5);
    h.control(1, Control::Rate(1.)).unwrap();
    render(&mut r, 1);
    assert!(!h.snapshot().slots[1].sync);
    h.control(0, Control::SetMode(false)).unwrap();
    render(&mut r, 1);
    assert_eq!(h.snapshot().master_deck, 0);
}
#[test]
fn prepared_audio_preserves_transport_and_old_same_track_jobs_are_rejected() {
    let (h, mut r) = engine(48000);
    let source = clip(1, 120.);
    h.load_and_play(0, source.clone()).unwrap();
    render(&mut r, 1000);
    for _ in 0..3 {
        h.control(0, Control::TransposeStep(1)).unwrap();
    }
    h.control(0, Control::Tempo(144.)).unwrap();
    render(&mut r, 1000);
    let s = h.snapshot().slots[0].clone();
    assert_eq!(s.tuning.semitones, 3);
    assert_eq!(s.applied_tuning.semitones, 0);
    assert_eq!(s.rate, 1.);
    let shifted = tuning::prepare(&source, s.tuning, || false).unwrap();
    let job = TunedClip {
        clip: shifted,
        load_ticket: s.load_ticket,
        revision: s.tune_revision,
        tuning: s.tuning,
    };
    let ticket = h.apply_tuning([Some(job.clone()), None]).unwrap();
    assert!(!h.tuning_acknowledged(ticket));
    render(&mut r, 128);
    let a = h.snapshot().slots[0].clone();
    assert!(h.tuning_acknowledged(ticket));
    assert_eq!(a.applied_tuning.semitones, 3);
    assert!(a.playing);
    assert!((a.position - s.position - 128. / 48000. * 1.2).abs() < 1e-7);
    h.load(0, source).unwrap();
    let ticket = h.apply_tuning([Some(job), None]).unwrap();
    render(&mut r, 1);
    let a = h.snapshot().slots[0].clone();
    assert!(h.tuning_acknowledged(ticket));
    assert_eq!(a.applied_tuning.semitones, 0);
    assert!(!a.playing);
    assert_eq!(a.id, 1);
}
#[test]
fn harmonic_sync_uses_relative_keys_and_validates_missing_metadata() {
    let (h, mut r) = engine(48000);
    assert!(h
        .action(Action {
            slot: 1,
            kind: "sync".into(),
            value: 1.
        })
        .is_err());
    let a = clip(1, 120.);
    let mut b = clip(2, 120.);
    Arc::get_mut(&mut b).unwrap().music.key = Some(music::MusicalKey {
        tonic: 11,
        minor: true,
    });
    h.load(0, a).unwrap();
    h.load(1, b).unwrap();
    render(&mut r, 1);
    h.control(1, Control::KeySync).unwrap();
    render(&mut r, 1);
    assert_eq!(
        h.snapshot().slots[1].tuning.semitones,
        -2,
        "B minor -> A minor, relative of C major"
    );
    assert!(h.control(2, Control::MasterDeck).is_err());
    for value in [f64::NAN, -13., 0.5, 13.] {
        assert!(h
            .action(Action {
                slot: 0,
                kind: "transpose".into(),
                value
            })
            .is_err());
    }
}
