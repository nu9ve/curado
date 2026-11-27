//! `TruePeakDetector::push` never allocates, so it can run in a real-time callback. A counting
//! allocator, in its own test binary so that nothing else shares it, watches every push.

// A global allocator is an unsafe trait; the crate itself has no unsafe code.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use nu9ve_loudness::TruePeakDetector;

struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: forwarded unchanged to the system allocator.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the pointer came from `alloc` above.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: forwarded unchanged to the system allocator.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

#[test]
fn pushing_samples_does_not_allocate() {
    let mut detectors = [TruePeakDetector::new(), TruePeakDetector::new()];
    let before = ALLOCATIONS.load(Ordering::SeqCst);
    let mut peak = 0.0f32;
    for i in 0..96_000 {
        let x = (i as f32 * 0.37).sin() * 1.3;
        for (detector, sample) in detectors.iter_mut().zip([x, -x * 0.5]) {
            peak = peak.max(detector.push(sample));
        }
        if i == 50_000 {
            detectors[1].push(f32::NAN);
            detectors[0].reset();
        }
    }
    let after = ALLOCATIONS.load(Ordering::SeqCst);
    assert!(peak > 1.25);
    assert_eq!(after, before, "push allocated {} times", after - before);
}
