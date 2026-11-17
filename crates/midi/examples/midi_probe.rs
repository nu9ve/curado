//! Explore a MIDI controller from a terminal, without the app. See docs/MIDI.md.
//!
//!   bun run midi:probe                          list ports
//!   bun run midi:probe DDJ-SB                   explore a port live for 5 minutes
//!   bun run midi:probe DDJ-SB --log take.log    and keep every message, as the fixtures are
//!   bun run midi:probe --replay take.log        explore a capture again
//!
//! Options: --seconds N (300), --baseline MS (3000), --quiet MS (400).

use curado_midi::detect::{parse_log, replay, Explorer};
use curado_midi::message::Message;
use curado_midi::preset;
use midir::{Ignore, MidiInput};
use std::collections::BTreeSet;
use std::io::Write;
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let option = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1).cloned())
    };
    let number =
        |name: &str, default: u64| option(name).and_then(|v| v.parse().ok()).unwrap_or(default);
    let baseline = number("--baseline", 3000);
    let quiet = number("--quiet", 400);
    if let Some(path) = option("--replay") {
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| fail(&format!("{path}: {e}")));
        let events = parse_log(&text);
        println!("{} messages in {path}", events.len());
        let found = replay(&events, baseline, quiet);
        for (at, d) in &found {
            println!("{at:>8} ms  {}", d.summary());
        }
        compare(None, found.iter().map(|(_, d)| d.key).collect());
        return;
    }
    let mut midi = MidiInput::new("curado probe").unwrap_or_else(|e| fail(&e.to_string()));
    midi.ignore(Ignore::All);
    let ports = midi.ports();
    let names: Vec<String> = ports
        .iter()
        .map(|p| midi.port_name(p).unwrap_or_default())
        .collect();
    let Some(wanted) = args.first().filter(|a| !a.starts_with("--")) else {
        println!("MIDI inputs:");
        for name in &names {
            let known =
                preset::for_port(name).map(|p| format!("  (preset: {} {})", p.vendor, p.name));
            println!("  {name}{}", known.unwrap_or_default());
        }
        if names.is_empty() {
            println!("  none. Is the controller plugged in, and closed in other DJ software?");
        }
        return;
    };
    let Some(index) = names
        .iter()
        .position(|n| n.to_lowercase().contains(&wanted.to_lowercase()))
    else {
        fail(&format!("no input matches {wanted:?}; found {names:?}"));
    };
    let name = names[index].clone();
    let mut log = option("--log")
        .map(|path| std::fs::File::create(&path).unwrap_or_else(|e| fail(&format!("{path}: {e}"))));
    let (tx, rx) = mpsc::channel::<(u64, Vec<u8>)>();
    let start = Instant::now();
    let _connection = midi
        .connect(
            &ports[index],
            "curado probe",
            move |_, bytes, _| {
                let _ = tx.send((start.elapsed().as_millis() as u64, bytes.to_vec()));
            },
            (),
        )
        .unwrap_or_else(|e| fail(&format!("{name}: {e} (another program may have it open)")));
    let seconds = number("--seconds", 300);
    println!("Listening to {name} for {seconds} s.");
    println!(
        "Touch nothing for {} s: controls that move by themselves are noted.",
        baseline / 1000
    );
    let mut explorer = Explorer::new(quiet);
    explorer.start_baseline(0, baseline);
    let mut announced = false;
    let mut found = BTreeSet::new();
    while start.elapsed() < Duration::from_secs(seconds) {
        let now = start.elapsed().as_millis() as u64;
        if !announced && explorer.baseline_left_ms(now).is_none() {
            announced = true;
            let noisy: Vec<String> = explorer.noisy().iter().map(|k| k.label()).collect();
            if noisy.is_empty() {
                println!("Nothing moved by itself.");
            } else {
                println!("Moving by themselves: {}", noisy.join(", "));
            }
            println!("Now move one control at a time, and let go before the next.");
        }
        if let Some(d) = explorer.poll(now, None) {
            println!("{now:>8} ms  {}", d.summary());
            found.insert(d.key);
        }
        let Ok((at, bytes)) = rx.recv_timeout(Duration::from_millis(20)) else {
            continue;
        };
        if let Some(file) = log.as_mut() {
            let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02X}")).collect();
            let _ = writeln!(file, "{at}\t{}", hex.join(" "));
        }
        if let Some(message) = Message::parse(&bytes) {
            explorer.feed(at, message);
        }
    }
    if let Some(d) = explorer.poll(u64::MAX, None) {
        println!("          {}", d.summary());
        found.insert(d.key);
    }
    println!("\nControls seen:");
    for seen in explorer.seen() {
        let noisy = if seen.noisy { "  moves by itself" } else { "" };
        println!(
            "  {:<24} {:>6} msgs  {}..{}{noisy}",
            seen.key.label(),
            seen.count,
            seen.min,
            seen.max
        );
    }
    compare(Some(&name), found);
}

/// Which gestures a bundled preset already knows, and which it lacks.
fn compare(port: Option<&str>, found: BTreeSet<curado_midi::Key>) {
    let Some(preset) = port.and_then(preset::for_port).or_else(|| {
        // A replay has no port: try every preset and keep the one that knows most.
        preset::bundled().into_iter().max_by_key(|p| {
            found
                .iter()
                .filter(|k| p.controls.iter().any(|c| c.key() == **k))
                .count()
        })
    }) else {
        return;
    };
    let known = found
        .iter()
        .filter(|k| preset.controls.iter().any(|c| c.key() == **k))
        .count();
    println!(
        "\nPreset {} {}: knows {known} of the {} controls found.",
        preset.vendor,
        preset.name,
        found.len()
    );
    for key in found
        .iter()
        .filter(|k| !preset.controls.iter().any(|c| c.key() == **k))
    {
        println!("  missing: {}", key.label());
    }
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}
