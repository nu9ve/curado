//! Minimal libc bindings for group signalling and fork-time parent identity.
//!
//! No new dependency: both symbols are in the C library `std` already links.
//! Signal numbers below are identical on macOS and Linux.
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::ffi::CString;
use std::io;
use std::os::fd::RawFd;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::atomic::{AtomicI32, Ordering};

pub const SIGHUP: i32 = 1;
pub const SIGINT: i32 = 2;
pub const SIGKILL: i32 = 9;
pub const SIGTERM: i32 = 15;
/// `EXDEV` (cross-device link) on macOS and Linux.
pub const EXDEV: i32 = 18;

/// The last cancellation signal received (0 = none).
static RECEIVED: AtomicI32 = AtomicI32::new(0);

unsafe extern "C" {
    fn signal(signum: i32, handler: extern "C" fn(i32)) -> usize;
    fn kill(pid: i32, sig: i32) -> i32;
    fn getpid() -> i32;
    fn getppid() -> i32;
    fn fcntl(fd: i32, cmd: i32, ...) -> i32;
    #[cfg(target_os = "macos")]
    fn renamex_np(from: *const std::ffi::c_char, to: *const std::ffi::c_char, flags: u32) -> i32;
    #[cfg(target_os = "linux")]
    fn renameat2(
        from_fd: i32,
        from: *const std::ffi::c_char,
        to_fd: i32,
        to: *const std::ffi::c_char,
        flags: u32,
    ) -> i32;
}

/// Async-signal-safe: one atomic store.
extern "C" fn record(signum: i32) {
    RECEIVED.store(signum, Ordering::SeqCst);
}

/// Turn SIGTERM/SIGINT/SIGHUP into a cancellation request instead of dying
/// with the child still running.
pub fn install_cancellation_handlers() {
    for signum in [SIGHUP, SIGINT, SIGTERM] {
        // SAFETY: `record` only stores into an atomic; `signal` has no other preconditions.
        unsafe {
            signal(signum, record);
        }
    }
}

pub fn received() -> i32 {
    RECEIVED.load(Ordering::SeqCst)
}

/// `kill(-group, sig)`: signal every process of the child's group.
pub fn kill_group(group: i32, sig: i32) -> bool {
    if group <= 1 {
        return false;
    }
    // SAFETY: plain syscall; a negative pid addresses the process group.
    unsafe { kill(-group, sig) == 0 }
}

pub fn pid() -> i32 {
    // SAFETY: getpid has no preconditions.
    unsafe { getpid() }
}

pub fn parent_pid() -> i32 {
    // SAFETY: getppid has no preconditions and is async-signal-safe.
    unsafe { getppid() }
}

/// Clear FD_CLOEXEC in the watchdog child only. F_SETFD is 2 on macOS/Linux.
pub(crate) fn inherit(fd: RawFd) -> io::Result<()> {
    // SAFETY: the inherited fd is open in this child; fcntl is async-signal-safe.
    if unsafe { fcntl(fd, 2, 0_i32) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Atomic exclusive placement. Hard links are preferred where available.
pub fn place_exclusive(source: &Path, target: &Path) -> io::Result<()> {
    match std::fs::hard_link(source, target) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Err(error),
        Err(_) => {}
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let from = CString::new(source.as_os_str().as_bytes())?;
        let to = CString::new(target.as_os_str().as_bytes())?;
        // SAFETY: both pointers are NUL-terminated and remain live for the call.
        #[cfg(target_os = "macos")]
        let result = unsafe { renamex_np(from.as_ptr(), to.as_ptr(), 4) }; // RENAME_EXCL
        // SAFETY: AT_FDCWD (-100) and RENAME_NOREPLACE (1) are Linux ABI constants.
        #[cfg(target_os = "linux")]
        let result = unsafe { renameat2(-100, from.as_ptr(), -100, to.as_ptr(), 1) };
        if result == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    Err(io::Error::from(io::ErrorKind::Unsupported))
}
