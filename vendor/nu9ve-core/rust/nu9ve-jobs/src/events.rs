//! NDJSON event stream on stdout; a closed stdout means the parent is gone.
//!
//! The app can reshape the stream without the crate knowing what its events mean: a [`Transform`]
//! renames, rewrites or drops each event before it is written, and a [`Sink`] takes the events in
//! process instead of stdout (a driver that runs a job's phases inside its own process).
use serde_json::{Map, Value};
use std::io::{self, Write};
use std::time::Instant;

/// Renders an `f64` as a JSON number (`None` becomes `null`). The app picks it:
/// an engine that must match Python passes its `repr(float)` rendering.
pub type Number = fn(f64) -> Option<Value>;

/// Rewrites one event before it goes out: `Some((event, fields))` to emit it, possibly renamed or
/// changed, `None` to drop it. Applied once per event.
pub type Transform<'a> =
    Box<dyn FnMut(&str, Map<String, Value>) -> Option<(String, Map<String, Value>)> + 'a>;

/// Where the events go instead of stdout.
pub type Sink<'a> = Box<dyn FnMut(&str, &Map<String, Value>) + 'a>;

pub struct Events<'a> {
    /// Applied to every event, before the sink or stdout.
    pub transform: Option<Transform<'a>>,
    /// Events captured in process (stdout untouched).
    pub sink: Option<Sink<'a>>,
    parent_gone: bool,
    started: Instant,
    number: Number,
}

impl<'a> Events<'a> {
    pub fn new(number: Number) -> Self {
        Self {
            transform: None,
            sink: None,
            parent_gone: false,
            started: Instant::now(),
            number,
        }
    }

    /// One line `{"event": <event>, ...fields}`. A failed write marks the parent gone.
    pub fn emit(&mut self, event: &str, fields: Map<String, Value>) {
        let (event, mut fields) = match self.transform.as_mut() {
            Some(transform) => match transform(event, fields) {
                Some(changed) => changed,
                None => return,
            },
            None => (event.to_owned(), fields),
        };
        if let Some(sink) = self.sink.as_mut() {
            sink(&event, &fields);
            return;
        }
        let mut line = Map::new();
        line.insert("event".into(), Value::from(event));
        line.append(&mut fields);
        let Ok(mut text) = serde_json::to_string(&Value::Object(line)) else {
            return;
        };
        text.push('\n');
        let mut stdout = io::stdout().lock();
        if stdout
            .write_all(text.as_bytes())
            .and_then(|()| stdout.flush())
            .is_err()
        {
            self.parent_gone = true;
        }
    }

    pub fn parent_gone(&self) -> bool {
        self.parent_gone
    }

    /// Restart the clock (a long-lived runner, once per job).
    pub fn restart(&mut self) {
        self.started = Instant::now();
    }

    pub fn elapsed_seconds(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    /// Seconds since the stream (or the last restart) began, as a JSON number.
    pub fn elapsed(&self) -> Value {
        self.number(self.elapsed_seconds())
    }

    pub fn number(&self, value: f64) -> Value {
        (self.number)(value).unwrap_or(Value::Null)
    }
}

/// The fields of a `json!({...})` object; anything else is empty.
pub fn fields(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

/// The plain JSON number of `value`, or `None` when it is not finite.
pub fn plain_number(value: f64) -> Option<Value> {
    serde_json::Number::from_f64(value).map(Value::Number)
}
