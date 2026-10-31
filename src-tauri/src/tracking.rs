//! Optional hand and face tracking, in a helper process that only exists while it is wanted.
//! Stopping kills it, which is what releases the camera whatever it was doing; if it dies or
//! loses the device while still wanted, a new one starts. Images stay on this machine.
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering::Relaxed},
        Arc, Mutex,
    },
    time::Duration,
};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Hands,
    Face,
    /// Both at once, which costs about twice the Vision work.
    All,
}
impl Mode {
    fn argument(self) -> &'static str {
        match self {
            Self::Hands => "hands",
            Self::Face => "face",
            Self::All => "all",
        }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Hand {
    pub x: f64,
    pub y: f64,
    pub pinch: f64,
    pub size: f64,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Face {
    pub x: f64,
    pub y: f64,
    pub size: f64,
}
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Frame {
    #[serde(default)]
    pub hands: Vec<Hand>,
    #[serde(default)]
    pub face: Option<Face>,
    #[serde(default)]
    pub jpeg: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}
const MAX_LINE: usize = 512 * 1024;
const MAX_HANDS: usize = 2;
fn valid_unit(value: f64) -> bool {
    value.is_finite() && (-0.5..=1.5).contains(&value)
}
/// The helper is ours, but what reaches the interface still has to make sense.
pub fn parse_line(line: &str) -> Option<Frame> {
    if line.len() > MAX_LINE {
        return None;
    }
    let mut frame: Frame = serde_json::from_str(line).ok()?;
    frame.hands.truncate(MAX_HANDS);
    frame.hands.retain(|hand| {
        valid_unit(hand.x)
            && valid_unit(hand.y)
            && hand.pinch.is_finite()
            && hand.pinch >= 0.
            && hand.size.is_finite()
            && hand.size > 0.
    });
    frame
        .face
        .take_if(|face| !(valid_unit(face.x) && valid_unit(face.y) && valid_unit(face.size)));
    frame.jpeg.take_if(|jpeg| {
        !jpeg
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b))
    });
    Some(frame)
}
/// What ending the helper means for the person who asked for it.
#[derive(Debug, PartialEq)]
pub enum Ending {
    /// Nothing to retry until something changes outside curado.
    Final(&'static str),
    /// Start again, after a moment.
    Retry,
}
pub fn ending(code: Option<i32>) -> Ending {
    match code {
        Some(3) => Ending::Final("denied"),
        Some(4) => Ending::Final("nocamera"),
        _ => Ending::Retry,
    }
}
const RETRY_FIRST: Duration = Duration::from_secs(1);
const RETRY_MOST: Duration = Duration::from_secs(10);

#[derive(Default)]
struct Running {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
}
#[derive(Default)]
pub struct Tracking {
    /// Each start or stop takes a new number, and a worker with an old one ends.
    generation: AtomicU64,
    running: Mutex<Running>,
    preview: Mutex<bool>,
}
impl Tracking {
    fn kill(&self) {
        let mut running = self.running.lock().unwrap();
        running.stdin = None;
        if let Some(mut child) = running.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    pub fn stop(&self) {
        self.generation.fetch_add(1, Relaxed);
        self.kill();
    }
    pub fn set_preview(&self, on: bool) {
        *self.preview.lock().unwrap() = on;
        self.send_preview(on);
    }
    fn send_preview(&self, on: bool) {
        if let Some(stdin) = self.running.lock().unwrap().stdin.as_mut() {
            let _ = writeln!(stdin, "preview {}", u8::from(on));
        }
    }
    /// `report` hears statuses and frames, from a worker of its own.
    pub fn start(
        self: &Arc<Self>,
        program: std::path::PathBuf,
        mode: Mode,
        report: impl Fn(Frame) + Send + 'static,
    ) {
        let generation = self.generation.fetch_add(1, Relaxed) + 1;
        self.kill();
        let tracking = self.clone();
        std::thread::spawn(move || {
            let mut wait = RETRY_FIRST;
            let status = |name: &str| Frame {
                status: Some(name.into()),
                ..Default::default()
            };
            while tracking.generation.load(Relaxed) == generation {
                report(status("starting"));
                match tracking.run_once(&program, mode, generation, &report) {
                    Ending::Final(name) => {
                        report(status(name));
                        return;
                    }
                    Ending::Retry => {
                        if tracking.generation.load(Relaxed) != generation {
                            return;
                        }
                        report(status("lost"));
                    }
                }
                std::thread::sleep(wait);
                wait = (wait * 2).min(RETRY_MOST);
            }
        });
    }
    fn run_once(
        &self,
        program: &Path,
        mode: Mode,
        generation: u64,
        report: &impl Fn(Frame),
    ) -> Ending {
        let spawned = Command::new(program)
            .arg(mode.argument())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let Ok(mut child) = spawned else {
            return Ending::Retry;
        };
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Ending::Retry;
        };
        {
            let mut running = self.running.lock().unwrap();
            // A stop or a new start got in while this one was spawning.
            if self.generation.load(Relaxed) != generation {
                let _ = child.kill();
                let _ = child.wait();
                return Ending::Retry;
            }
            running.stdin = child.stdin.take();
            running.child = Some(child);
        }
        self.send_preview(*self.preview.lock().unwrap());
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else {
                break;
            };
            if self.generation.load(Relaxed) != generation {
                break;
            }
            if let Some(frame) = parse_line(&line) {
                report(frame);
            }
        }
        let child = {
            let mut running = self.running.lock().unwrap();
            if self.generation.load(Relaxed) != generation {
                return Ending::Retry;
            }
            running.stdin = None;
            running.child.take()
        };
        let code = child.and_then(|mut child| {
            let _ = child.kill();
            child.wait().ok().and_then(|status| status.code())
        });
        ending(code)
    }
}
impl Drop for Tracking {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The helper is compiled into the app, and written out only when first needed.
#[cfg(target_os = "macos")]
pub fn helper(cache: &Path) -> Result<std::path::PathBuf, String> {
    use std::os::unix::fs::PermissionsExt;
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/curado-tracker"));
    let bundle = cache.join("curado tracker.app");
    let contents = bundle.join("Contents");
    let directory = contents.join("MacOS");
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    std::fs::write(contents.join("Info.plist"), r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict><key>CFBundleExecutable</key><string>curado-tracker</string><key>CFBundleIdentifier</key><string>studio.curado.tracker</string><key>CFBundleName</key><string>curado tracker</string><key>CFBundlePackageType</key><string>APPL</string><key>NSCameraUsageDescription</key><string>curado usa la cámara solo si activas el seguimiento de manos o cara.</string><key>LSMinimumSystemVersion</key><string>12.0</string></dict></plist>"#).map_err(|e| e.to_string())?;
    let path = directory.join("curado-tracker");
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| e.to_string())?;
    Ok(path)
}
#[cfg(not(target_os = "macos"))]
pub fn helper(_cache: &Path) -> Result<std::path::PathBuf, String> {
    Err("El seguimiento con cámara requiere macOS.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Instant;

    #[test]
    fn lines_from_the_helper_are_checked() {
        let frame = parse_line(r#"{"hands":[{"x":0.5,"y":0.2,"pinch":0.3,"size":0.2}]}"#).unwrap();
        assert_eq!(frame.hands.len(), 1);
        assert_eq!(parse_line("not json"), None);
        assert_eq!(
            parse_line(&format!("{{\"jpeg\":\"{}\"}}", "a".repeat(MAX_LINE))),
            None
        );
        let wild = parse_line(r#"{"hands":[{"x":9,"y":0.2,"pinch":0.3,"size":0.2},{"x":0.1,"y":0.2,"pinch":-1,"size":0.2}],"face":{"x":0.5,"y":2,"size":0.2},"jpeg":"<script>"}"#).unwrap();
        assert!(wild.hands.is_empty() && wild.face.is_none() && wild.jpeg.is_none());
        let many = r#"{"hands":[{"x":0,"y":0,"pinch":0,"size":1},{"x":0,"y":0,"pinch":0,"size":1},{"x":0,"y":0,"pinch":0,"size":1}]}"#;
        assert_eq!(parse_line(many).unwrap().hands.len(), 2);
    }

    #[test]
    fn only_a_clear_no_stops_retrying() {
        assert_eq!(ending(Some(3)), Ending::Final("denied"));
        assert_eq!(ending(Some(4)), Ending::Final("nocamera"));
        assert_eq!(ending(Some(1)), Ending::Retry);
        assert_eq!(ending(None), Ending::Retry);
    }

    /// A stand-in helper: a script, so these prove the lifecycle without touching a camera.
    fn helper_script(name: &str, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!("curado-{name}-{}", std::process::id()));
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
    fn collect(rx: &mpsc::Receiver<Frame>, want: &str, timeout: Duration) -> bool {
        let started = Instant::now();
        while started.elapsed() < timeout {
            if let Ok(frame) = rx.recv_timeout(Duration::from_millis(50)) {
                if frame.status.as_deref() == Some(want) {
                    return true;
                }
            }
        }
        false
    }
    fn alive(pid: u32) -> bool {
        std::process::Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    #[test]
    fn stopping_ends_the_process_at_once() {
        let script = helper_script(
            "stop",
            r#"echo '{"status":"running"}'; while true; do sleep 1; done"#,
        );
        let tracking = Arc::new(Tracking::default());
        let (tx, rx) = mpsc::channel();
        tracking.start(script.clone(), Mode::Hands, move |frame| {
            let _ = tx.send(frame);
        });
        assert!(collect(&rx, "running", Duration::from_secs(3)));
        let pid = tracking
            .running
            .lock()
            .unwrap()
            .child
            .as_ref()
            .unwrap()
            .id();
        assert!(alive(pid));
        tracking.stop();
        assert!(!alive(pid), "the helper is gone, and the camera with it");
        assert!(tracking.running.lock().unwrap().child.is_none());
        let _ = std::fs::remove_file(script);
    }

    #[test]
    fn a_helper_that_dies_comes_back_and_a_denied_one_does_not() {
        let dies = helper_script("dies", r#"echo '{"status":"running"}'; exit 1"#);
        let tracking = Arc::new(Tracking::default());
        let (tx, rx) = mpsc::channel();
        tracking.start(dies.clone(), Mode::Face, move |frame| {
            let _ = tx.send(frame);
        });
        assert!(collect(&rx, "lost", Duration::from_secs(3)));
        assert!(
            collect(&rx, "running", Duration::from_secs(4)),
            "it starts again"
        );
        tracking.stop();
        let denied = helper_script("denied", "exit 3");
        let (tx, rx) = mpsc::channel();
        tracking.start(denied.clone(), Mode::Hands, move |frame| {
            let _ = tx.send(frame);
        });
        assert!(collect(&rx, "denied", Duration::from_secs(3)));
        std::thread::sleep(Duration::from_millis(1500));
        assert!(
            !rx.try_iter()
                .any(|f| f.status.as_deref() == Some("starting")),
            "no retry"
        );
        for path in [dies, denied] {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn a_new_start_replaces_the_old_helper() {
        let script = helper_script(
            "again",
            r#"echo '{"status":"running"}'; while true; do sleep 1; done"#,
        );
        let tracking = Arc::new(Tracking::default());
        let (tx, rx) = mpsc::channel();
        let report = tx.clone();
        tracking.start(script.clone(), Mode::Hands, move |frame| {
            let _ = report.send(frame);
        });
        assert!(collect(&rx, "running", Duration::from_secs(3)));
        let first = tracking
            .running
            .lock()
            .unwrap()
            .child
            .as_ref()
            .unwrap()
            .id();
        tracking.start(script.clone(), Mode::Face, move |frame| {
            let _ = tx.send(frame);
        });
        assert!(!alive(first));
        assert!(collect(&rx, "running", Duration::from_secs(3)));
        tracking.stop();
        let _ = std::fs::remove_file(script);
    }

    #[test]
    fn preview_requests_reach_a_helper_on_its_input() {
        let out = std::env::temp_dir().join(format!("curado-preview-out-{}", std::process::id()));
        let script = helper_script(
            "preview",
            &format!(
                r#"echo '{{"status":"running"}}'; while read line; do echo "$line" >> '{}'; done"#,
                out.display()
            ),
        );
        let tracking = Arc::new(Tracking::default());
        let (tx, rx) = mpsc::channel();
        tracking.set_preview(true);
        tracking.start(script.clone(), Mode::Hands, move |frame| {
            let _ = tx.send(frame);
        });
        assert!(collect(&rx, "running", Duration::from_secs(3)));
        tracking.set_preview(false);
        std::thread::sleep(Duration::from_millis(300));
        let lines = std::fs::read_to_string(&out).unwrap_or_default();
        assert_eq!(lines, "preview 1\npreview 0\n");
        tracking.stop();
        let _ = std::fs::remove_file(script);
        let _ = std::fs::remove_file(out);
    }
}
