//! Start one command in its own process group and supervise it until it exits
//! or must stop: cancellation (signal, parent gone, cancel file or an
//! app-specific source), timeout or stall. Stopping is SIGTERM to the group,
//! SIGKILL after the grace period, then a final sweep of the group.
use crate::events::{Events, fields};
use crate::group::Group;
use crate::sys;
use serde_json::json;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// A source file that must not change between enqueue and start.
pub struct Pin {
    pub path: String,
    pub size: i128,
    pub mtime_ns: i128,
}

#[derive(Debug)]
pub enum PinError {
    Changed,
    Missing,
}

pub fn mtime_ns(meta: &fs::Metadata) -> i128 {
    i128::from(meta.mtime()) * 1_000_000_000 + i128::from(meta.mtime_nsec())
}

impl Pin {
    pub fn check(&self) -> Result<(), PinError> {
        match fs::metadata(&self.path) {
            Ok(meta)
                if i128::from(meta.size()) == self.size && mtime_ns(&meta) == self.mtime_ns =>
            {
                Ok(())
            }
            Ok(_) => Err(PinError::Changed),
            Err(_) => Err(PinError::Missing),
        }
    }
}

/// The generic cancellation sources, in order: a received signal, a closed
/// event stream, an existing cancel file.
pub fn cancel_reason(events: &Events, cancel_file: Option<&Path>) -> Option<&'static str> {
    if sys::received() != 0 {
        return Some("signal");
    }
    if events.parent_gone() {
        return Some("parent_gone");
    }
    if cancel_file.is_some_and(|path| path.exists()) {
        return Some("cancel_file");
    }
    None
}

/// The exact command: no inherited environment, stdin closed.
pub struct Spawn<'a> {
    pub command: &'a str,
    pub args: &'a [String],
    pub cwd: &'a str,
    pub env: &'a [(String, String)],
    /// stdout and stderr go here; without a log both are discarded.
    pub log: Option<&'a Path>,
    pub log_append: bool,
    pub stdout: Option<&'a Path>,
    /// With a log: stdout is read line by line instead (stderr stays in the log).
    pub pipe_stdout: bool,
    /// A producer whose stdout becomes the command's stdin, in the same group.
    pub feeder: Option<Feeder<'a>>,
}

/// The first process of a two-process pipeline (same cwd and environment).
pub struct Feeder<'a> {
    pub command: &'a str,
    pub args: &'a [String],
    /// Its stderr; its stdout is the pipe.
    pub log: &'a Path,
}

pub type Spawned = (Child, Group, Option<Receiver<String>>, Option<Child>);

/// How long a producer may take to exit after its consumer did (it writes a
/// report or sees EPIPE) before the group sweep stops it.
const FEEDER_EXIT_SECONDS: u64 = 30;

pub fn spawn(spec: &Spawn, leases: &[&File], leader_args: &[&str]) -> Result<Spawned, String> {
    let mut command = Command::new(spec.command);
    command
        .args(spec.args)
        .current_dir(spec.cwd)
        .env_clear()
        .envs(spec.env.iter().map(|(key, value)| (key, value)))
        .stdin(Stdio::null());
    match spec.log {
        Some(path) => {
            let log = OpenOptions::new()
                .write(true)
                .create(true)
                .append(spec.log_append)
                .truncate(!spec.log_append)
                .open(path)
                .map_err(|error| format!("cannot open the log {}: {error}", path.display()))?;
            let copy = log
                .try_clone()
                .map_err(|error| format!("cannot share the log: {error}"))?;
            command.stderr(Stdio::from(copy));
            if spec.pipe_stdout {
                command.stdout(Stdio::piped());
            } else {
                command.stdout(Stdio::from(log));
            }
        }
        None => {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
    }
    if let Some(path) = spec.stdout {
        let stdout = File::create(path)
            .map_err(|error| format!("cannot open stdout {}: {error}", path.display()))?;
        command.stdout(Stdio::from(stdout));
    }
    let group = Group::start(&mut command, leases, leader_args)
        .map_err(|error| format!("Could not start the process group watchdog: {error}."))?;
    let feeder = match &spec.feeder {
        None => None,
        Some(feeder) => {
            let log = File::create(feeder.log).map_err(|error| {
                format!("cannot open the log {}: {error}", feeder.log.display())
            })?;
            let mut producer = Command::new(feeder.command);
            producer
                .args(feeder.args)
                .current_dir(spec.cwd)
                .env_clear()
                .envs(spec.env.iter().map(|(key, value)| (key, value)))
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::from(log));
            group.adopt(&mut producer);
            let mut producer = producer
                .spawn()
                .map_err(|error| format!("Could not start {}: {error}.", feeder.command))?;
            let pipe = producer.stdout.take().ok_or("missing feeder pipe")?;
            // Only the consumer holds the read end: if it exits, the producer sees EPIPE.
            command.stdin(Stdio::from(pipe));
            Some(producer)
        }
    };
    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not start {}: {error}.", spec.command))?;
    let lines = match child.stdout.take() {
        Some(stdout) if spec.pipe_stdout => {
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines() {
                    let Ok(line) = line else { break };
                    if sender.send(line).is_err() {
                        break;
                    }
                }
            });
            Some(receiver)
        }
        _ => None,
    };
    Ok((child, group, lines, feeder))
}

/// Wait for a pipeline's producer once its consumer is done.
fn wait_feeder(feeder: &mut Child) -> Option<ExitStatus> {
    let deadline = Instant::now() + Duration::from_secs(FEEDER_EXIT_SECONDS);
    loop {
        match feeder.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            _ => break,
        }
    }
    let _ = feeder.kill();
    feeder.wait().ok()
}

/// SIGTERM to the group, SIGKILL after `grace`, then a final group sweep.
fn terminate(
    child: &mut Child,
    group: i32,
    grace: f64,
    events: &mut Events,
) -> io::Result<ExitStatus> {
    if let Some(status) = child.try_wait()? {
        sys::kill_group(group, sys::SIGKILL);
        return Ok(status);
    }
    sys::kill_group(group, sys::SIGTERM);
    events.emit("signal", fields(json!({"signal": "SIGTERM"})));
    let deadline = Instant::now() + Duration::from_secs_f64(grace);
    let mut status = None;
    while Instant::now() < deadline {
        if let Some(done) = child.try_wait()? {
            status = Some(done);
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    if status.is_none() {
        sys::kill_group(group, sys::SIGKILL);
        events.emit("signal", fields(json!({"signal": "SIGKILL"})));
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if let Some(done) = child.try_wait()? {
                status = Some(done);
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    // A cooperative leader may exit before an uncooperative grandchild.
    sys::kill_group(group, sys::SIGKILL);
    status.ok_or_else(|| io::Error::other("the process did not exit after SIGKILL"))
}

/// Seconds; `None` disables a limit.
pub struct Limits<'a> {
    pub timeout: Option<f64>,
    pub stall: Option<f64>,
    pub grace: f64,
    pub poll: f64,
    pub heartbeat: f64,
    pub cancel_file: Option<&'a Path>,
}

/// What the app adds to the loop: progress, heartbeats and its own cancel source.
pub trait Monitor {
    /// Once per poll while the process runs, before the heartbeat.
    fn poll(&mut self, events: &mut Events);
    /// Every `heartbeat` seconds, before the `heartbeat` event.
    fn heartbeat(&mut self, events: &mut Events);
    /// Checked after the generic sources of [`cancel_reason`].
    fn cancel_reason(&self) -> Option<&'static str>;
    /// When progress last advanced; `None` disables the stall limit.
    fn advanced_at(&self) -> Option<Instant>;
}

/// Why the supervisor stopped the process.
#[derive(Debug)]
pub enum Stop {
    Cancelled(&'static str),
    Timeout(f64),
    Stalled(f64),
    /// Polling the child failed.
    Error(String),
}

impl Stop {
    pub fn name(&self) -> &'static str {
        match self {
            Stop::Cancelled(_) => "cancelled",
            Stop::Timeout(_) => "timeout",
            Stop::Stalled(_) => "stalled",
            Stop::Error(_) => "supervisor_error",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Stop::Cancelled(reason) => format!("Cancelled ({reason})."),
            Stop::Timeout(limit) => format!("Timed out after {limit}s."),
            Stop::Stalled(limit) => format!("No progress for {limit}s."),
            Stop::Error(error) => format!("Supervisor error: {error}."),
        }
    }
}

/// Poll `child` until it exits or a limit stops it. Emits `heartbeat`,
/// `cancelling` and `signal` events. `started` is when the child was spawned.
/// A pipeline's `feeder` is awaited after the child, then the group is swept
/// and released before returning.
pub fn supervise(
    mut child: Child,
    group: Group,
    feeder: Option<Child>,
    started: Instant,
    limits: &Limits,
    monitor: &mut dyn Monitor,
    events: &mut Events,
) -> (io::Result<ExitStatus>, Option<Stop>, Option<ExitStatus>) {
    let mut last_heartbeat = started;
    let mut stop = None;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {}
            Err(error) => {
                stop = Some(Stop::Error(error.to_string()));
                break terminate(&mut child, group.id(), limits.grace, events);
            }
        }
        monitor.poll(events);
        if last_heartbeat.elapsed().as_secs_f64() >= limits.heartbeat {
            last_heartbeat = Instant::now();
            monitor.heartbeat(events);
            let elapsed = events.number(started.elapsed().as_secs_f64());
            events.emit("heartbeat", fields(json!({"elapsed_seconds": elapsed})));
        }
        // In order: cancellation, then timeout, then stall.
        let reason = cancel_reason(events, limits.cancel_file)
            .or_else(|| monitor.cancel_reason())
            .map(Stop::Cancelled)
            .or_else(|| {
                limits
                    .timeout
                    .filter(|limit| started.elapsed().as_secs_f64() >= *limit)
                    .map(Stop::Timeout)
            })
            .or_else(|| {
                limits
                    .stall
                    .filter(|limit| {
                        monitor
                            .advanced_at()
                            .is_some_and(|at| at.elapsed().as_secs_f64() >= *limit)
                    })
                    .map(Stop::Stalled)
            });
        if let Some(reason) = reason {
            events.emit(
                "cancelling",
                fields(json!({"status": reason.name(), "message": reason.message()})),
            );
            stop = Some(reason);
            break terminate(&mut child, group.id(), limits.grace, events);
        }
        std::thread::sleep(Duration::from_secs_f64(limits.poll));
    };
    let feeder = feeder.and_then(|mut feeder| wait_feeder(&mut feeder));
    drop(group);
    (status, stop, feeder)
}

/// The last 16 KiB of a log, lossily decoded (empty when there is none).
pub fn read_tail(path: Option<&Path>) -> String {
    let Some(path) = path else {
        return String::new();
    };
    let Ok(mut file) = File::open(path) else {
        return String::new();
    };
    let length = file.metadata().map_or(0, |meta| meta.len());
    let _ = file.seek(SeekFrom::Start(length.saturating_sub(16_384)));
    let mut bytes = Vec::new();
    let _ = file.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}
