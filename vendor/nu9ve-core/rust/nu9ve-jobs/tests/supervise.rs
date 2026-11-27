//! The supervisor against real processes. Its own main (`harness = false`): the watchdog re-runs the
//! current executable with the leader arguments, so this binary is also the group leader when it gets
//! `--watchdog`.

#[cfg(unix)]
fn main() {
    unix::main();
}

#[cfg(not(unix))]
fn main() {}

#[cfg(unix)]
mod unix {
    use nu9ve_jobs::{
        Events, Limits, Monitor, Output, Pin, PinError, PublishError, Spawn, Stop, fields,
        plain_number, publish, read_tail, spawn, supervise, sys, watch,
    };
    use serde_json::{Map, Value, json};
    use std::cell::RefCell;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    const LEADER: &[&str] = &["--watchdog"];

    pub fn main() {
        if std::env::args().any(|arg| arg == "--watchdog") {
            std::process::exit(watch());
        }
        let tests: &[(&str, fn())] = &[
            ("exit status and log", exit_status_and_log),
            (
                "timeout stops the whole group",
                timeout_stops_the_whole_group,
            ),
            ("cancel file", cancel_file),
            ("stall from the monitor", stall_from_the_monitor),
            ("SIGKILL after the grace", sigkill_after_the_grace),
            ("a pipeline shares the group", a_pipeline_shares_the_group),
            ("dropping the group sweeps it", dropping_the_group_sweeps_it),
            ("events: transform and sink", events_transform_and_sink),
            ("pins", pins),
            (
                "publish exclusive and replace",
                publish_exclusive_and_replace,
            ),
            ("publish rolls back", publish_rolls_back),
            (
                "publish rejects a short partial",
                publish_rejects_a_short_partial,
            ),
        ];
        let filter: Vec<String> = std::env::args()
            .skip(1)
            .filter(|a| !a.starts_with('-'))
            .collect();
        let mut passed = 0;
        for (name, test) in tests {
            if !filter.is_empty() && !filter.iter().any(|f| name.contains(f.as_str())) {
                continue;
            }
            test();
            println!("test {name} ... ok");
            passed += 1;
        }
        println!("test result: ok. {passed} passed");
    }

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("nu9ve-jobs-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[derive(Default)]
    struct Quiet {
        advanced_at: Option<Instant>,
        cancel: Option<&'static str>,
    }

    impl Monitor for Quiet {
        fn poll(&mut self, _: &mut Events) {}
        fn heartbeat(&mut self, _: &mut Events) {}
        fn cancel_reason(&self) -> Option<&'static str> {
            self.cancel
        }
        fn advanced_at(&self) -> Option<Instant> {
            self.advanced_at
        }
    }

    fn limits(cancel_file: Option<&Path>) -> Limits<'_> {
        Limits {
            timeout: None,
            stall: None,
            grace: 2.0,
            poll: 0.02,
            heartbeat: 60.0,
            cancel_file,
        }
    }

    /// Events captured in process, so the test's stdout stays clean.
    type Seen = Rc<RefCell<Vec<(String, Map<String, Value>)>>>;

    fn captured() -> (Events<'static>, Seen) {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let mut events = Events::new(plain_number);
        let sink = Rc::clone(&seen);
        events.sink = Some(Box::new(move |event, fields| {
            sink.borrow_mut().push((event.to_owned(), fields.clone()));
        }));
        (events, seen)
    }

    fn shell(script: &str) -> Vec<String> {
        vec!["-c".to_owned(), script.to_owned()]
    }

    fn sh<'a>(args: &'a [String], cwd: &'a str) -> Spawn<'a> {
        Spawn {
            command: "/bin/sh",
            args,
            cwd,
            env: &[],
            log: None,
            log_append: false,
            stdout: None,
            pipe_stdout: false,
            feeder: None,
        }
    }

    fn alive(pid: i32) -> bool {
        // Signal 0 only checks that the process exists.
        std::process::Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn wait_for_file(path: &Path) -> String {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Ok(text) = fs::read_to_string(path)
                && text.ends_with('\n')
            {
                return text.trim().to_owned();
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("{} never appeared", path.display());
    }

    fn exit_status_and_log() {
        let scratch = Scratch::new("exit");
        let log = scratch.path("log.txt");
        let args = shell("echo out; echo err >&2; exit 3");
        let cwd = scratch.0.to_string_lossy().into_owned();
        let mut spec = sh(&args, &cwd);
        spec.log = Some(&log);
        let (child, group, _, feeder) = spawn(&spec, &[], LEADER).unwrap();
        let (mut events, _) = captured();
        let (status, stop, _) = supervise(
            child,
            group,
            feeder,
            Instant::now(),
            &limits(None),
            &mut Quiet::default(),
            &mut events,
        );
        assert_eq!(status.unwrap().code(), Some(3));
        assert!(stop.is_none());
        let tail = read_tail(Some(&log));
        assert!(tail.contains("out") && tail.contains("err"), "{tail}");
    }

    fn timeout_stops_the_whole_group() {
        let scratch = Scratch::new("timeout");
        let pidfile = scratch.path("grandchild");
        // A grandchild that would outlive its parent without the group kill.
        let args = shell(&format!(
            "sleep 30 & echo $! > '{}'; wait",
            pidfile.display()
        ));
        let cwd = scratch.0.to_string_lossy().into_owned();
        let (child, group, _, feeder) = spawn(&sh(&args, &cwd), &[], LEADER).unwrap();
        let grandchild: i32 = wait_for_file(&pidfile).parse().unwrap();
        let mut limit = limits(None);
        limit.timeout = Some(0.3);
        let (mut events, seen) = captured();
        let started = Instant::now();
        let (status, stop, _) = supervise(
            child,
            group,
            feeder,
            started,
            &limit,
            &mut Quiet::default(),
            &mut events,
        );
        assert!(status.is_ok());
        assert!(
            matches!(stop, Some(Stop::Timeout(_))),
            "{:?}",
            stop.map(|s| s.name())
        );
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(100));
        assert!(!alive(grandchild), "the grandchild survived the group");
        let seen = seen.borrow();
        let names: Vec<&str> = seen.iter().map(|(event, _)| event.as_str()).collect();
        assert_eq!(names.first(), Some(&"cancelling"), "{names:?}");
        assert_eq!(seen[0].1["status"], "timeout");
        assert!(names.contains(&"signal"));
    }

    fn cancel_file() {
        let scratch = Scratch::new("cancel");
        let cancel = scratch.path("cancel");
        let args = shell("sleep 30");
        let cwd = scratch.0.to_string_lossy().into_owned();
        let (child, group, _, feeder) = spawn(&sh(&args, &cwd), &[], LEADER).unwrap();
        let file = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            fs::write(file, b"").unwrap();
        });
        let (mut events, _) = captured();
        let (_, stop, _) = supervise(
            child,
            group,
            feeder,
            Instant::now(),
            &limits(Some(&cancel)),
            &mut Quiet::default(),
            &mut events,
        );
        match stop {
            Some(Stop::Cancelled(reason)) => assert_eq!(reason, "cancel_file"),
            other => panic!("expected cancelled, got {:?}", other.map(|s| s.name())),
        }
        // The app's own source comes after the generic ones.
        let args = shell("sleep 30");
        let (child, group, _, feeder) = spawn(&sh(&args, &cwd), &[], LEADER).unwrap();
        let mut monitor = Quiet {
            cancel: Some("queue_cancelled"),
            ..Quiet::default()
        };
        let (_, stop, _) = supervise(
            child,
            group,
            feeder,
            Instant::now(),
            &limits(None),
            &mut monitor,
            &mut events,
        );
        assert!(matches!(stop, Some(Stop::Cancelled("queue_cancelled"))));
    }

    fn stall_from_the_monitor() {
        let scratch = Scratch::new("stall");
        let args = shell("sleep 30");
        let cwd = scratch.0.to_string_lossy().into_owned();
        let (child, group, _, feeder) = spawn(&sh(&args, &cwd), &[], LEADER).unwrap();
        let mut limit = limits(None);
        limit.stall = Some(0.2);
        let mut monitor = Quiet {
            advanced_at: Some(Instant::now()),
            ..Quiet::default()
        };
        let (mut events, _) = captured();
        let (_, stop, _) = supervise(
            child,
            group,
            feeder,
            Instant::now(),
            &limit,
            &mut monitor,
            &mut events,
        );
        assert!(matches!(stop, Some(Stop::Stalled(_))));
        // Without an advance time, the stall limit is off: the timeout ends it instead.
        let (child, group, _, feeder) = spawn(&sh(&args, &cwd), &[], LEADER).unwrap();
        limit.timeout = Some(0.4);
        let (_, stop, _) = supervise(
            child,
            group,
            feeder,
            Instant::now(),
            &limit,
            &mut Quiet::default(),
            &mut events,
        );
        assert!(matches!(stop, Some(Stop::Timeout(_))));
    }

    fn sigkill_after_the_grace() {
        let scratch = Scratch::new("grace");
        let ready = scratch.path("ready");
        let args = shell(&format!(
            "trap '' TERM; echo ok > '{}'; while :; do sleep 1; done",
            ready.display()
        ));
        let cwd = scratch.0.to_string_lossy().into_owned();
        let (child, group, _, feeder) = spawn(&sh(&args, &cwd), &[], LEADER).unwrap();
        wait_for_file(&ready);
        let mut limit = limits(None);
        limit.timeout = Some(0.05);
        limit.grace = 0.2;
        let (mut events, seen) = captured();
        let (status, stop, _) = supervise(
            child,
            group,
            feeder,
            Instant::now(),
            &limit,
            &mut Quiet::default(),
            &mut events,
        );
        assert!(matches!(stop, Some(Stop::Timeout(_))));
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(status.unwrap().signal(), Some(sys::SIGKILL));
        let signals: Vec<Value> = seen
            .borrow()
            .iter()
            .filter(|(event, _)| event == "signal")
            .map(|(_, fields)| fields["signal"].clone())
            .collect();
        assert_eq!(signals, [json!("SIGTERM"), json!("SIGKILL")]);
    }

    fn a_pipeline_shares_the_group() {
        let scratch = Scratch::new("pipe");
        let log = scratch.path("log.txt");
        let feeder_log = scratch.path("feeder.txt");
        let producer = shell("printf 'a\\nb\\n'");
        let consumer = vec![];
        let cwd = scratch.0.to_string_lossy().into_owned();
        let mut spec = Spawn {
            command: "/usr/bin/wc",
            args: &consumer,
            cwd: &cwd,
            env: &[],
            log: Some(&log),
            log_append: false,
            stdout: None,
            pipe_stdout: true,
            feeder: Some(nu9ve_jobs::Feeder {
                command: "/bin/sh",
                args: &producer,
                log: &feeder_log,
            }),
        };
        spec.log_append = false;
        let (child, group, lines, feeder) = spawn(&spec, &[], LEADER).unwrap();
        let (mut events, _) = captured();
        let (status, stop, fed) = supervise(
            child,
            group,
            feeder,
            Instant::now(),
            &limits(None),
            &mut Quiet::default(),
            &mut events,
        );
        assert!(status.unwrap().success() && stop.is_none());
        assert!(fed.is_some_and(|s| s.success()));
        let out: Vec<String> = lines.unwrap().iter().collect();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].split_whitespace().next(), Some("2"), "{out:?}");
    }

    fn dropping_the_group_sweeps_it() {
        let scratch = Scratch::new("drop");
        let pidfile = scratch.path("child");
        let args = shell(&format!("echo $$ > '{}'; sleep 30", pidfile.display()));
        let cwd = scratch.0.to_string_lossy().into_owned();
        let (mut child, group, _, _) = spawn(&sh(&args, &cwd), &[], LEADER).unwrap();
        let pid: i32 = wait_for_file(&pidfile).parse().unwrap();
        assert!(alive(pid));
        drop(group);
        let _ = child.wait();
        assert!(!alive(pid));
    }

    fn events_transform_and_sink() {
        let (mut events, seen) = captured();
        // The app reshapes its own events: progress becomes its stage, a heartbeat is dropped.
        events.transform = Some(Box::new(|event, mut fields| match event {
            "progress" => {
                fields.insert("stage".into(), json!("render"));
                Some(("stage_progress".to_owned(), fields))
            }
            "heartbeat" => None,
            _ => Some((event.to_owned(), fields)),
        }));
        events.emit("progress", fields(json!({"progress": 0.5})));
        events.emit("heartbeat", Map::new());
        events.emit("signal", fields(json!({"signal": "SIGTERM"})));
        let seen = seen.borrow();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].0, "stage_progress");
        assert_eq!(seen[0].1["stage"], "render");
        assert_eq!(seen[1].0, "signal");
        assert_eq!(events.number(f64::NAN), Value::Null);
        assert_eq!(events.number(1.5), json!(1.5));
        assert!(!events.parent_gone());
    }

    fn pins() {
        let scratch = Scratch::new("pin");
        let path = scratch.path("source.txt");
        fs::write(&path, b"one").unwrap();
        let meta = fs::metadata(&path).unwrap();
        use std::os::unix::fs::MetadataExt;
        let pin = Pin {
            path: path.to_string_lossy().into_owned(),
            size: i128::from(meta.size()),
            mtime_ns: nu9ve_jobs::mtime_ns(&meta),
        };
        assert!(pin.check().is_ok());
        fs::write(&path, b"three").unwrap();
        assert!(matches!(pin.check(), Err(PinError::Changed)));
        fs::remove_file(&path).unwrap();
        assert!(matches!(pin.check(), Err(PinError::Missing)));
    }

    fn output(scratch: &Scratch, name: &str, body: &[u8], replace: bool) -> Output {
        let partial = scratch.path(&format!("{name}.partial"));
        fs::write(&partial, body).unwrap();
        Output {
            partial,
            target: scratch.path(&format!("out/{name}")),
            min_bytes: 1,
            replace,
        }
    }

    fn publish_exclusive_and_replace() {
        let scratch = Scratch::new("publish");
        fs::create_dir_all(scratch.path("out")).unwrap();
        fs::write(scratch.path("out/b.txt"), b"old").unwrap();
        let outputs = [
            output(&scratch, "a.txt", b"new a", false),
            output(&scratch, "b.txt", b"new b", true),
        ];
        let published = publish(&outputs).unwrap();
        assert_eq!(published.len(), 2);
        assert!(published[0].ends_with("a.txt") && published[1].ends_with("b.txt"));
        assert_eq!(fs::read(scratch.path("out/a.txt")).unwrap(), b"new a");
        assert_eq!(fs::read(scratch.path("out/b.txt")).unwrap(), b"new b");
        assert!(!scratch.path("a.txt.partial").exists() && !scratch.path("b.txt.partial").exists());
    }

    fn publish_rolls_back() {
        let scratch = Scratch::new("rollback");
        fs::create_dir_all(scratch.path("out")).unwrap();
        fs::write(scratch.path("out/b.txt"), b"someone else's").unwrap();
        // a is placed, b exists and is exclusive: a is taken back and b is untouched.
        let outputs = [
            output(&scratch, "a.txt", b"a", false),
            output(&scratch, "b.txt", b"b", false),
        ];
        match publish(&outputs) {
            Err(PublishError::Place(target, _)) => assert!(target.ends_with("b.txt")),
            _ => panic!("expected a placement error"),
        }
        assert!(!scratch.path("out/a.txt").exists());
        assert!(scratch.path("a.txt.partial").exists());
        assert_eq!(
            fs::read(scratch.path("out/b.txt")).unwrap(),
            b"someone else's"
        );
    }

    fn publish_rejects_a_short_partial() {
        let scratch = Scratch::new("short");
        let mut short = output(&scratch, "a.txt", b"abc", false);
        short.min_bytes = 10;
        assert!(matches!(publish(&[short]), Err(PublishError::Invalid(_))));
        assert!(!scratch.path("out/a.txt").exists());
    }
}
