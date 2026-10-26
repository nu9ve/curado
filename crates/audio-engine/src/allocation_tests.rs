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
            peaks: vec![],
            title: None,
            artist: None,
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
    ACTIVE.with(|a| a.set(true));
    for _ in 0..10 {
        renderer.render(&mut output, 2);
    }
    ACTIVE.with(|a| a.set(false));
    assert_eq!(COUNTS.with(Cell::get), (0, 0));
    handle.collect();
}
