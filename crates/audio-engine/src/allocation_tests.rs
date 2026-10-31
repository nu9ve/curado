use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    sync::Arc,
};
thread_local! { static ACTIVE: Cell<bool> = const { Cell::new(false) }; static COUNTS: Cell<(u64,u64)> = const { Cell::new((0,0)) }; }
struct Audit;
unsafe impl GlobalAlloc for Audit {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ACTIVE.try_with(Cell::get).unwrap_or(false) {
            let _ = COUNTS.try_with(|c| {
                let (a, d) = c.get();
                c.set((a + 1, d));
            });
        }
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ACTIVE.try_with(Cell::get).unwrap_or(false) {
            let _ = COUNTS.try_with(|c| {
                let (a, d) = c.get();
                c.set((a, d + 1));
            });
        }
        System.dealloc(ptr, layout);
    }
}
#[global_allocator]
static ALLOCATOR: Audit = Audit;
#[test]
fn callback_does_not_allocate_or_free_even_when_replacing_clips() {
    let (handle, mut renderer) = crate::engine(48000);
    let clip = || {
        Arc::new(crate::decode::Clip {
            id: 1,
            sample_rate: 48000,
            samples: vec![[0.2, 0.2]; 4800],
            waveform: vec![],
            title: None,
            artist: None,
            music: Default::default(),
            cues: Arc::new(crate::annotations::CueState::default()),
        })
    };
    let mut output = [0.0_f32; 512];
    handle.load(0, clip()).unwrap();
    handle.control(0, crate::Control::Play).unwrap();
    ACTIVE.with(|a| a.set(true));
    renderer.render(&mut output, 2);
    ACTIVE.with(|a| a.set(false));
    handle.load(0, clip()).unwrap();
    handle.control(0, crate::Control::Seek(0.05)).unwrap();
    handle.control(0, crate::Control::Filter(0.3)).unwrap();
    handle.control(0, crate::Control::SetCue(1)).unwrap();
    assert!(handle.visual_samples().is_none());
    ACTIVE.with(|a| a.set(true));
    for _ in 0..10 {
        renderer.render(&mut output, 2);
    }
    ACTIVE.with(|a| a.set(false));
    assert_eq!(COUNTS.with(Cell::get), (0, 0));
    assert!(handle.visual_samples().is_some());
    handle.stop_visual_capture();
    handle.collect();
    // Exercise linked transport and hot-cue restarts with both decks and a sample.
    handle.load_and_play(1, clip()).unwrap();
    handle.load(2, clip()).unwrap();
    handle.control(2, crate::Control::Trigger).unwrap();
    ACTIVE.with(|a| a.set(true));
    renderer.render(&mut output, 2);
    ACTIVE.with(|a| a.set(false));
    for _ in 0..50 {
        handle.control(0, crate::Control::CueBoth(0)).unwrap();
        handle.control(0, crate::Control::Cue(1)).unwrap();
        handle.control(1, crate::Control::Cue(2)).unwrap();
        handle.control(0, crate::Control::PauseBoth).unwrap();
        handle.control(0, crate::Control::PlayBoth).unwrap();
        ACTIVE.with(|a| a.set(true));
        renderer.render(&mut output, 2);
        ACTIVE.with(|a| a.set(false));
    }
    handle
        .set_music(
            1,
            crate::music::MusicAnalysis {
                version: 1,
                bpm: Some(120.),
                key: Some(crate::music::MusicalKey {
                    tonic: 0,
                    minor: false,
                }),
                ..Default::default()
            },
        )
        .unwrap();
    handle.control(0, crate::Control::HighPass(0.5)).unwrap();
    handle.control(0, crate::Control::EchoBeats(0.75)).unwrap();
    handle.control(0, crate::Control::LoopBeats(1.)).unwrap();
    handle.control(1, crate::Control::Sync(true)).unwrap();
    handle.control(0, crate::Control::TransposeStep(1)).unwrap();
    ACTIVE.with(|a| a.set(true));
    renderer.render(&mut output, 2);
    ACTIVE.with(|a| a.set(false));
    let slot = handle.snapshot().slots[0].clone();
    let batch = [
        Some(crate::tuning::TunedClip {
            clip: clip(),
            load_ticket: slot.load_ticket,
            revision: slot.tune_revision,
            tuning: slot.tuning,
        }),
        None,
    ];
    handle.apply_tuning(batch).unwrap();
    ACTIVE.with(|a| a.set(true));
    renderer.render(&mut output, 2);
    ACTIVE.with(|a| a.set(false));
    assert_eq!(COUNTS.with(Cell::get), (0, 0));
    // Loop fades, a pause fade, PCM replaced under a fading voice and its later return.
    handle.control(0, crate::Control::Quantize(false)).unwrap();
    handle.control(0, crate::Control::Seek(0.01)).unwrap();
    handle.control(0, crate::Control::LoopIn).unwrap();
    handle.control(0, crate::Control::Seek(0.03)).unwrap();
    handle.control(0, crate::Control::LoopOut).unwrap();
    handle.control(0, crate::Control::Play).unwrap();
    ACTIVE.with(|a| a.set(true));
    for _ in 0..40 {
        renderer.render(&mut output, 2);
    }
    ACTIVE.with(|a| a.set(false));
    handle.load_and_play(0, clip()).unwrap();
    handle.load(0, clip()).unwrap();
    handle.control(1, crate::Control::Pause).unwrap();
    ACTIVE.with(|a| a.set(true));
    for _ in 0..10 {
        renderer.render(&mut output, 2);
    }
    ACTIVE.with(|a| a.set(false));
    handle.control(0, crate::Control::Panic).unwrap();
    ACTIVE.with(|a| a.set(true));
    for _ in 0..10 {
        renderer.render(&mut output, 2);
    }
    ACTIVE.with(|a| a.set(false));
    assert_eq!(COUNTS.with(Cell::get), (0, 0));
    handle.collect();
}
