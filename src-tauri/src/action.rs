//! Long actions and their progress, as `nu9ve-action/1` records (vendor/nu9ve-core/contracts/nu9ve-action-v1.md).
//!
//! Whoever launches an action names it with a `progress_id`; the action writes its record through a
//! [`Meter`] and the frontend reads it with the `action_progress` command. Records live in memory: an
//! action and the reader share this process, so a record is never seen half written, and a finished one
//! is kept for [`KEEP_FINISHED`] so a view that comes back still sees how it ended.
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const CONTRACT: &str = "nu9ve-action/1";
/// §2: a finished record is served at least this long after its last write.
pub const KEEP_FINISHED: Duration = Duration::from_secs(10 * 60);
/// §3.3: a running record is written at least every 10 s; this beats well inside that.
pub const HEARTBEAT: Duration = Duration::from_secs(4);
const MAX_LOG: usize = 60;
const MAX_WARNINGS: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Files,
    Items,
    Work,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Running,
    Complete,
    Failed,
    Cancelled,
}
#[derive(Clone, Debug, Serialize)]
pub struct Counter {
    pub index: u64,
    pub count: u64,
    pub label: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Current {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct LogEntry {
    pub at: String,
    pub kind: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Warning {
    pub code: String,
    pub message: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Outcome {
    pub summary: String,
    pub warnings: Vec<Warning>,
    pub receipt: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Failure {
    pub code: String,
    pub message: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Record {
    pub contract: &'static str,
    pub progress_id: String,
    pub action: String,
    pub label: String,
    pub state: State,
    pub stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<Counter>,
    pub overall_done: u64,
    pub overall_total: Option<u64>,
    pub unit: Unit,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_done: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<Current>,
    pub log: Vec<LogEntry>,
    pub cancellable: bool,
    pub started_at: String,
    pub progressed_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Outcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Failure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resumable: Option<bool>,
}
struct Entry {
    record: Record,
    updated: SystemTime,
    cancel: Arc<AtomicBool>,
}
#[derive(Default)]
pub struct Actions {
    entries: Mutex<HashMap<String, Entry>>,
    beating: AtomicBool,
}

/// `^[a-z0-9][a-z0-9._:-]{0,127}$`, the id pattern of nu9ve-data §4.1.
pub fn valid_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    (1..=128).contains(&bytes.len())
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes.iter().all(|&b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b':' | b'-')
        })
}
fn valid_word(word: &str) -> bool {
    let bytes = word.as_bytes();
    (1..=64).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|&b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
/// At most `limit` characters, never empty, without NUL.
fn short(text: &str, limit: usize) -> String {
    let clean: String = text.chars().filter(|&c| c != '\0').take(limit).collect();
    if clean.is_empty() {
        "—".into()
    } else {
        clean
    }
}
/// RFC 3339 in UTC with `Z` and milliseconds.
pub fn instant(time: SystemTime) -> String {
    let since = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let seconds = since.as_secs();
    let (days, rest) = ((seconds / 86_400) as i64, seconds % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        rest / 3600,
        rest / 60 % 60,
        rest % 60,
        since.subsec_millis()
    )
}

impl Actions {
    /// Starts an action under `id`. An id that already runs an action is refused and that one goes on;
    /// a finished one is replaced (§1).
    pub fn begin(
        self: &Arc<Self>,
        id: &str,
        action: &str,
        label: &str,
        unit: Unit,
        cancellable: bool,
    ) -> Result<Meter, String> {
        if !valid_id(id) || !valid_word(action) {
            return Err("invalid_params: progress_id no válido".into());
        }
        let now = SystemTime::now();
        let stamp = instant(now);
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut entries = self.entries.lock().unwrap();
            entries.retain(|_, entry| {
                entry.record.state == State::Running
                    || now.duration_since(entry.updated).unwrap_or_default() < KEEP_FINISHED
            });
            if entries
                .get(id)
                .is_some_and(|entry| entry.record.state == State::Running)
            {
                return Err("action_running: esa acción ya está en curso".into());
            }
            entries.insert(
                id.into(),
                Entry {
                    record: Record {
                        contract: CONTRACT,
                        progress_id: id.into(),
                        action: action.into(),
                        label: short(label, 200),
                        state: State::Running,
                        stage: "preparing".into(),
                        phase: None,
                        overall_done: 0,
                        overall_total: None,
                        unit,
                        files_done: None,
                        files_total: None,
                        current: None,
                        log: Vec::new(),
                        cancellable,
                        started_at: stamp.clone(),
                        progressed_at: stamp.clone(),
                        updated_at: stamp,
                        outcome: None,
                        error: None,
                        resumable: None,
                    },
                    updated: now,
                    cancel: cancel.clone(),
                },
            );
        }
        self.start_heartbeat();
        Ok(Meter {
            actions: self.clone(),
            id: id.into(),
            cancel,
            closed: false,
        })
    }
    /// The data of an `action_progress` reply: the record, or `unknown` for an id with none (§2). A read:
    /// it changes nothing.
    pub fn progress(&self, id: &str) -> Result<serde_json::Value, String> {
        if !valid_id(id) {
            return Err("invalid_params: progress_id no válido".into());
        }
        let entries = self.entries.lock().unwrap();
        Ok(match entries.get(id) {
            Some(entry) => serde_json::to_value(&entry.record).map_err(|e| e.to_string())?,
            None => {
                serde_json::json!({"contract": CONTRACT, "progress_id": id, "state": "unknown"})
            }
        })
    }
    /// Asks a running, cancellable action to stop. Returns its `action` when it was asked.
    pub fn cancel(&self, id: &str) -> Option<String> {
        let entries = self.entries.lock().unwrap();
        let entry = entries.get(id)?;
        if entry.record.state != State::Running || !entry.record.cancellable {
            return None;
        }
        entry.cancel.store(true, Ordering::SeqCst);
        Some(entry.record.action.clone())
    }
    fn edit(&self, id: &str, change: impl FnOnce(&mut Record, &str)) {
        let now = SystemTime::now();
        let stamp = instant(now);
        if let Some(entry) = self.entries.lock().unwrap().get_mut(id) {
            if entry.record.state != State::Running {
                return; // §3.4: a finished record does not run again.
            }
            change(&mut entry.record, &stamp);
            entry.record.updated_at = stamp;
            entry.updated = now;
        }
    }
    /// One thread for every running record, so a slow step still shows a living action (§3.3).
    fn start_heartbeat(self: &Arc<Self>) {
        if self.beating.swap(true, Ordering::SeqCst) {
            return;
        }
        let actions = Arc::downgrade(self);
        let spawned = std::thread::Builder::new()
            .name("curado-action-heartbeat".into())
            .spawn(move || loop {
                std::thread::sleep(HEARTBEAT);
                let Some(actions) = actions.upgrade() else {
                    return;
                };
                let mut entries = actions.entries.lock().unwrap();
                if !entries.values().any(|e| e.record.state == State::Running) {
                    actions.beating.store(false, Ordering::SeqCst);
                    return;
                }
                let now = SystemTime::now();
                let stamp = instant(now);
                for entry in entries.values_mut() {
                    if entry.record.state == State::Running {
                        entry.record.updated_at = stamp.clone();
                        entry.updated = now;
                    }
                }
            });
        if spawned.is_err() {
            self.beating.store(false, Ordering::SeqCst);
        }
    }
}

/// Writes one action's record. Dropping it without a close records a failure, never a success (§3.5).
pub struct Meter {
    actions: Arc<Actions>,
    id: String,
    cancel: Arc<AtomicBool>,
    closed: bool,
}
impl Meter {
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
    /// A shared stage, or `curado.<stage>` for one of curado's own.
    pub fn stage(&self, stage: &str) {
        self.actions.edit(&self.id, |r, _| r.stage = stage.into());
    }
    pub fn phase(&self, index: u64, count: u64, label: &str) {
        let count = count.max(1);
        let phase = Counter {
            index: index.clamp(1, count),
            count,
            label: short(label, 200),
        };
        self.actions.edit(&self.id, |r, _| r.phase = Some(phase));
    }
    /// Sets or changes the total without moving the fraction back (§3.3): what is done is scaled up to
    /// the same share of the new total when it shrinks the fraction.
    pub fn total(&self, total: Option<u64>) {
        self.actions.edit(&self.id, |r, _| {
            if let (Some(old), Some(new)) = (r.overall_total.filter(|&t| t > 0), total) {
                let scaled = (r.overall_done as u128 * new as u128).div_ceil(old as u128) as u64;
                r.overall_done = r.overall_done.max(scaled);
            }
            r.overall_total = total;
            r.overall_done = running_cap(r.overall_done, total);
        });
    }
    /// Moves the measure to `done`, never back, and short of 100 % while running.
    pub fn reach(&self, done: u64) {
        self.actions.edit(&self.id, |r, stamp| {
            let done = running_cap(done, r.overall_total);
            if done > r.overall_done {
                r.overall_done = done;
                r.progressed_at = stamp.into();
            }
        });
    }
    pub fn files(&self, done: u64, total: u64) {
        self.actions.edit(&self.id, |r, _| {
            r.files_total = Some(total);
            r.files_done = Some(done.min(total));
        });
    }
    /// The file in hand, by its name only.
    pub fn current(&self, name: Option<&str>, action: Option<&str>) {
        let current = name.map(|name| Current {
            name: short(&name.replace(['/', '\\'], "∕"), 255),
            action: action.map(str::to_owned),
        });
        self.actions.edit(&self.id, |r, _| r.current = current);
    }
    pub fn log(&self, kind: &str, name: &str, text: Option<&str>) {
        if !valid_word(kind) {
            return;
        }
        self.actions.edit(&self.id, |r, stamp| {
            if r.log.len() == MAX_LOG {
                r.log.remove(0);
            }
            r.log.push(LogEntry {
                at: stamp.into(),
                kind: kind.into(),
                name: name.into(),
                text: text.map(|t| short(t, 500)),
            });
        });
    }
    pub fn complete(mut self, summary: &str, warnings: Vec<Warning>) {
        let outcome = outcome(summary, warnings);
        self.close(State::Complete, |r| {
            if let Some(total) = r.overall_total {
                r.overall_done = total;
            }
            r.outcome = Some(outcome);
        });
    }
    pub fn fail(mut self, code: &str, message: &str, done: Option<&str>, resumable: bool) {
        let code = if valid_word(code) {
            code
        } else {
            "internal_error"
        };
        let error = Failure {
            code: code.into(),
            message: short(message, 500),
        };
        let done = done.map(|summary| outcome(summary, Vec::new()));
        self.close(State::Failed, |r| {
            r.error = Some(error);
            r.outcome = done;
            r.resumable = Some(resumable);
        });
    }
    pub fn cancelled_with(mut self, done: Option<&str>) {
        let done = done.map(|summary| outcome(summary, Vec::new()));
        self.close(State::Cancelled, |r| {
            r.outcome = done;
            r.resumable = Some(false);
        });
    }
    fn close(&mut self, state: State, change: impl FnOnce(&mut Record)) {
        self.closed = true;
        self.actions.edit(&self.id, |r, _| {
            change(r);
            r.state = state;
            r.cancellable = false;
            r.current = None;
        });
    }
}
impl Drop for Meter {
    fn drop(&mut self) {
        if !self.closed {
            let message = "La acción terminó sin escribir su cierre";
            self.close(State::Failed, |r| {
                r.error = Some(Failure {
                    code: "internal_error".into(),
                    message: message.into(),
                });
                r.resumable = Some(false);
            });
        }
    }
}
fn running_cap(done: u64, total: Option<u64>) -> u64 {
    match total {
        Some(total) if total > 0 => done.min(total - 1),
        _ => done,
    }
}
fn outcome(summary: &str, mut warnings: Vec<Warning>) -> Outcome {
    warnings.truncate(MAX_WARNINGS);
    for warning in &mut warnings {
        if !valid_word(&warning.code) {
            warning.code = "warning".into();
        }
        warning.message = short(&warning.message, 500);
    }
    Outcome {
        summary: short(summary, 500),
        warnings,
        receipt: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn read(actions: &Actions, id: &str) -> serde_json::Value {
        actions.progress(id).unwrap()
    }
    #[test]
    fn instants_are_rfc3339_utc() {
        assert_eq!(instant(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
        let t = UNIX_EPOCH + Duration::from_millis(1_795_716_131_250);
        assert_eq!(instant(t), "2026-11-26T18:02:11.250Z");
        let leap = UNIX_EPOCH + Duration::from_secs(951_782_400);
        assert_eq!(instant(leap), "2000-02-29T00:00:00.000Z");
    }
    #[test]
    fn ids_follow_the_data_pattern() {
        assert!(valid_id("import-2026-11-27-1"));
        assert!(valid_id("a:b.c_d"));
        assert!(!valid_id(""));
        assert!(!valid_id("Import"));
        assert!(!valid_id("-x"));
        assert!(!valid_id(&"a".repeat(129)));
    }
    #[test]
    fn unknown_ids_read_as_unknown_and_bad_ones_are_refused() {
        let actions = Arc::new(Actions::default());
        assert_eq!(
            read(&actions, "nada-1"),
            serde_json::json!({"contract": CONTRACT, "progress_id": "nada-1", "state": "unknown"})
        );
        assert!(actions.progress("Bad Id").is_err());
        assert!(actions
            .begin("Bad Id", "import", "x", Unit::Files, true)
            .is_err());
    }
    #[test]
    fn the_measure_never_goes_back_nor_fills_while_running() {
        let actions = Arc::new(Actions::default());
        let meter = actions
            .begin("import-1", "import", "Importando", Unit::Files, true)
            .unwrap();
        meter.total(Some(10));
        meter.reach(5);
        meter.reach(3);
        assert_eq!(read(&actions, "import-1")["overall_done"], 5);
        meter.reach(10);
        assert_eq!(read(&actions, "import-1")["overall_done"], 9);
        // A larger total keeps the fraction: 9/10 stays at least 90 %.
        meter.total(Some(20));
        assert_eq!(read(&actions, "import-1")["overall_done"], 18);
        meter.complete("20 pistas", Vec::new());
        let record = read(&actions, "import-1");
        assert_eq!(record["state"], "complete");
        assert_eq!(record["overall_done"], 20);
        assert_eq!(record["cancellable"], false);
        assert!(record.get("current").is_none());
        assert_eq!(record["outcome"]["warnings"], serde_json::json!([]));
        assert!(record["outcome"]["receipt"].is_null());
    }
    #[test]
    fn a_running_id_is_refused_and_a_finished_one_is_replaced() {
        let actions = Arc::new(Actions::default());
        let first = actions
            .begin("same", "import", "a", Unit::Files, true)
            .unwrap();
        assert!(actions
            .begin("same", "import", "b", Unit::Files, true)
            .err()
            .unwrap()
            .starts_with("action_running"));
        first.cancelled_with(None);
        let again = actions
            .begin("same", "analyze", "c", Unit::Items, false)
            .unwrap();
        assert_eq!(read(&actions, "same")["action"], "analyze");
        drop(again);
    }
    #[test]
    fn a_dropped_meter_fails_and_a_finished_record_stays() {
        let actions = Arc::new(Actions::default());
        {
            let meter = actions
                .begin("lost", "import", "x", Unit::Files, true)
                .unwrap();
            meter.current(Some("a/b.wav"), Some("analyze"));
        }
        let record = read(&actions, "lost");
        assert_eq!(record["state"], "failed");
        assert_eq!(record["error"]["code"], "internal_error");
        assert_eq!(record["resumable"], false);
        // Closed records do not change any more.
        assert!(actions.cancel("lost").is_none());
    }
    #[test]
    fn cancel_reaches_only_cancellable_running_actions() {
        let actions = Arc::new(Actions::default());
        let fixed = actions
            .begin("fixed", "analyze", "x", Unit::Items, false)
            .unwrap();
        assert!(actions.cancel("fixed").is_none());
        let free = actions
            .begin("free", "separate", "x", Unit::Work, true)
            .unwrap();
        assert_eq!(actions.cancel("free").as_deref(), Some("separate"));
        assert!(free.cancelled());
        assert!(!fixed.cancelled());
        fixed.complete("ok", Vec::new());
        free.cancelled_with(Some("nada guardado"));
        assert_eq!(read(&actions, "free")["state"], "cancelled");
    }
    #[test]
    fn the_log_keeps_the_last_sixty_and_names_lose_their_folders() {
        let actions = Arc::new(Actions::default());
        let meter = actions
            .begin("log", "import", "x", Unit::Files, true)
            .unwrap();
        for n in 0..70 {
            meter.log("imported", &format!("{n}.wav"), None);
        }
        meter.log("Not A Kind", "x", None);
        meter.current(Some("/Music/a.wav"), None);
        let record = read(&actions, "log");
        let log = record["log"].as_array().unwrap();
        assert_eq!(log.len(), 60);
        assert_eq!(log[0]["name"], "10.wav");
        assert!(!record["current"]["name"].as_str().unwrap().contains('/'));
        meter.complete("ok", Vec::new());
    }
}
