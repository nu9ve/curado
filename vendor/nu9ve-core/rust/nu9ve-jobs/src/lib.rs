//! Supervise one process of a job: its own process group with a watchdog, cancellation (signal,
//! parent gone, cancel file or the app's own source), timeout and stall, SIGTERM then SIGKILL, an
//! NDJSON event stream, and atomic publication of the files it wrote.
//!
//! Boundary: nothing here knows an app. No manifest, queue, exit codes, schema names, progress
//! formats or Python semantics: the app passes them. The watchdog's leader arguments (the app routes
//! them to [`watch`]), the JSON rendering of numbers in [`Events`], a [`Monitor`] for progress,
//! heartbeats and extra cancel sources, and a [`Transform`] for its own event shapes.
//!
//! Unix only (macOS and Linux) for now: on Windows the crate is empty. Cancelling a worker on
//! Windows is the client's (a Job Object and a cancel file, nu9ve-engine-v1 §6.2).
//!
//! * [`sys`]: signals, group kill, exclusive placement.
//! * `group`: the process group and its watchdog (lifetime pipe).
//! * `events`: NDJSON events on stdout.
//! * `process`: spawn, cancel, timeout, stall, SIGTERM then SIGKILL, pins.
//! * `publish`: atomic publication of `partial` outputs.
#[cfg(unix)]
mod events;
#[cfg(unix)]
mod group;
#[cfg(unix)]
mod process;
#[cfg(unix)]
mod publish;
#[cfg(unix)]
pub mod sys;

#[cfg(unix)]
pub use events::{Events, Number, Sink, Transform, fields, plain_number};
#[cfg(unix)]
pub use group::{Group, watch};
#[cfg(unix)]
pub use process::{
    Feeder, Limits, Monitor, Pin, PinError, Spawn, Spawned, Stop, cancel_reason, mtime_ns,
    read_tail, spawn, supervise,
};
#[cfg(unix)]
pub use publish::{Output, PublishError, publish};
