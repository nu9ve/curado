//! Taps on a Force Touch trackpad. There is no such thing on a mouse or other systems, where these
//! quietly do nothing. Calls the one AppKit class directly rather than taking a dependency for it.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pattern {
    /// A distinct stop: the end of a range.
    Generic,
    /// A snap into place: a center or a guide.
    Alignment,
    /// A light notch: moving through steps.
    Level,
}
impl Pattern {
    #[cfg(target_os = "macos")]
    fn code(self) -> isize {
        match self {
            Self::Generic => 0,
            Self::Alignment => 1,
            Self::Level => 2,
        }
    }
}

#[cfg(target_os = "macos")]
pub fn perform(pattern: Pattern) {
    use std::ffi::{c_void, CStr};
    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const std::ffi::c_char) -> *mut c_void;
        fn sel_registerName(name: *const std::ffi::c_char) -> *mut c_void;
        fn objc_msgSend();
    }
    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}
    const NOW: usize = 1;
    const CLASS: &CStr = c"NSHapticFeedbackManager";
    const PERFORMER: &CStr = c"defaultPerformer";
    const PERFORM: &CStr = c"performFeedbackPattern:performanceTime:";
    // SAFETY: both selectors exist on every macOS curado supports, with these exact signatures,
    // and objc_msgSend has to be called through a pointer of the method's real type.
    unsafe {
        let class = objc_getClass(CLASS.as_ptr());
        if class.is_null() {
            return;
        }
        let get: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        let performer = get(class, sel_registerName(PERFORMER.as_ptr()));
        if performer.is_null() {
            return;
        }
        let run: unsafe extern "C" fn(*mut c_void, *mut c_void, isize, usize) =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        run(
            performer,
            sel_registerName(PERFORM.as_ptr()),
            pattern.code(),
            NOW,
        );
    }
}
#[cfg(not(target_os = "macos"))]
pub fn perform(_pattern: Pattern) {}
