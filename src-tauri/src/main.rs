#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    // Re-run as the watchdog of a stem separation's process group: no window, no app.
    if let Some(code) = curado_app::job_watchdog() {
        std::process::exit(code);
    }
    curado_app::run();
}
