use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    sync::Arc,
};
thread_local! { static ACTIVE: Cell<bool> = const { Cell::new(false) }; static COUNTS: Cell<(u64,u64)> = const { Cell::new((0,0)) }; }
// Rust's allocator never sees C++. On macOS the system allocator's logging hook does, so this
// also covers Signalsmith Stretch inside the callback.
#[cfg(target_os = "macos")]
mod system_allocator {
    use std::sync::atomic::{
        AtomicU64, AtomicUsize,
        Ordering::{Relaxed, SeqCst},
    };
    type Logger = unsafe extern "C" fn(u32, usize, usize, usize, usize, u32);
    extern "C" {
        static mut malloc_logger: Option<Logger>;
        fn pthread_self() -> usize;
    }
    static WATCHED_THREAD: AtomicUsize = AtomicUsize::new(0);
    static EVENTS: AtomicU64 = AtomicU64::new(0);
    // No thread-local in here: first touching one allocates, which would call this again.
    unsafe extern "C" fn count(kind: u32, _: usize, _: usize, _: usize, _: usize, _: u32) {
        if kind != 0 && WATCHED_THREAD.load(Relaxed) == pthread_self() {
            EVENTS.fetch_add(1, Relaxed);
        }
    }
    pub fn install() {
        unsafe { malloc_logger = Some(count) };
    }
    pub fn uninstall() {
        unsafe { malloc_logger = None };
    }
    pub fn set_watching(watching: bool) {
        let thread = if watching {
            unsafe { pthread_self() }
        } else {
            0
        };
        WATCHED_THREAD.store(thread, SeqCst);
    }
    pub fn events() -> u64 {
        EVENTS.load(SeqCst)
    }
}
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
            loudness: None,
            beats: None,
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
    // Key lock, transposition, loops, cues, pause and a load over pitched audio.
    let long = || {
        Arc::new(crate::decode::Clip {
            loudness: None,
            beats: None,
            id: 2,
            sample_rate: 44100,
            samples: vec![[0.2, -0.2]; 44100],
            waveform: vec![],
            title: None,
            artist: None,
            music: Default::default(),
            cues: Arc::new(crate::annotations::CueState::default()),
        })
    };
    handle.control(0, crate::Control::Master(0.75)).unwrap();
    handle.load_and_play(0, long()).unwrap();
    handle.load_and_play(1, long()).unwrap();
    #[cfg(target_os = "macos")]
    {
        // A hook that never fires would pass everything below, so prove it sees allocations.
        system_allocator::install();
        system_allocator::set_watching(true);
        std::hint::black_box(Vec::<u8>::with_capacity(1 << 16));
        system_allocator::set_watching(false);
        assert!(
            system_allocator::events() >= 2,
            "the hook sees the system allocator"
        );
    }
    #[cfg(target_os = "macos")]
    let baseline = system_allocator::events();
    let mut render = |blocks: usize| {
        ACTIVE.with(|a| a.set(true));
        #[cfg(target_os = "macos")]
        system_allocator::set_watching(true);
        for _ in 0..blocks {
            renderer.render(&mut output, 2);
        }
        #[cfg(target_os = "macos")]
        system_allocator::set_watching(false);
        ACTIVE.with(|a| a.set(false));
    };
    render(1);
    handle.control(0, crate::Control::Rate(1.1)).unwrap();
    handle.control(1, crate::Control::Transpose(3)).unwrap();
    render(40);
    handle.control(0, crate::Control::Quantize(false)).unwrap();
    handle.control(0, crate::Control::Seek(0.2)).unwrap();
    handle.control(0, crate::Control::LoopIn).unwrap();
    handle.control(0, crate::Control::Seek(0.25)).unwrap();
    handle.control(0, crate::Control::LoopOut).unwrap();
    render(60);
    for _ in 0..20 {
        handle.control(0, crate::Control::Cue(0)).unwrap();
        handle.control(1, crate::Control::Seek(0.3)).unwrap();
        render(3);
    }
    handle.control(1, crate::Control::TransposeStep(1)).unwrap();
    handle.control(0, crate::Control::KeyLock(false)).unwrap();
    handle.control(1, crate::Control::Pause).unwrap();
    render(10);
    handle.control(0, crate::Control::KeyLock(true)).unwrap();
    handle.control(1, crate::Control::Play).unwrap();
    render(10);
    handle.load_and_play(1, long()).unwrap();
    render(20);
    // Every effect on both channels and the master, with the transport moving under them.
    use crate::Control as C;
    for kind in 0..crate::fx::BeatKind::COUNT {
        for slot in 0..crate::BEAT_UNITS {
            handle.control(slot, C::BeatFx(kind)).unwrap();
            handle.control(slot, C::BeatFxBeats(0.25)).unwrap();
            handle.control(slot, C::BeatFxOn(true)).unwrap();
        }
        render(12);
    }
    for kind in 0..crate::fx::ColorKind::COUNT {
        for slot in 0..2 {
            handle.control(slot, C::ColorFx(kind)).unwrap();
            handle
                .control(slot, C::Color(if slot == 0 { -0.8 } else { 0.8 }))
                .unwrap();
        }
        render(12);
    }
    handle.control(0, C::Low(0.)).unwrap();
    handle.control(0, C::Filter(0.3)).unwrap();
    handle.control(0, C::HighPass(0.3)).unwrap();
    handle.control(0, C::Trim(1.5)).unwrap();
    handle.control(0, C::AutoGain(true)).unwrap();
    render(4);
    handle.control(0, C::Slip(true)).unwrap();
    handle.control(0, C::Roll(0.25)).unwrap();
    render(10);
    handle.control(0, C::RollEnd).unwrap();
    handle.control(0, C::Reverse(true)).unwrap();
    render(10);
    handle.control(0, C::Reverse(false)).unwrap();
    handle.control(0, C::BeatJump(4.)).unwrap();
    handle.control(0, C::LoopBeats(0.5)).unwrap();
    handle.control(0, C::LoopScale(2.)).unwrap();
    render(10);
    handle.control(0, C::JogTouch(true)).unwrap();
    handle.control(0, C::JogMove(-0.1)).unwrap();
    render(10);
    handle.control(0, C::JogTouch(false)).unwrap();
    handle.control(0, C::JogBend(0.1)).unwrap();
    render(10);
    handle.control(0, C::Brake(0.05)).unwrap();
    handle.control(0, C::Pause).unwrap();
    render(10);
    handle.control(0, C::Play).unwrap();
    render(4);
    for kind in 0..8 {
        handle.control(0, C::Release(kind)).unwrap();
        render(6);
        handle.control(0, C::ReleaseEnd).unwrap();
        handle.control(0, C::Play).unwrap();
        render(2);
    }
    handle.control(0, C::Cue2(true)).unwrap();
    handle.control(0, C::ToggleBoth).unwrap();
    render(4);
    handle.control(0, C::ToggleBoth).unwrap();
    render(4);
    handle.control(5, C::Sampler(0.5)).unwrap();
    handle.control(0, C::SetMode(false)).unwrap();
    render(10);
    assert_eq!(COUNTS.with(Cell::get), (0, 0));
    #[cfg(target_os = "macos")]
    {
        system_allocator::uninstall();
        assert_eq!(system_allocator::events(), baseline, "system allocator");
    }
    handle.collect();
}
