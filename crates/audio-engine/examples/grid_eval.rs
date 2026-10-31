//! Compares curado's beat analysis with the grids rekordbox stored on this machine.
//!
//! Read-only. It parses `ANLZ0000.DAT` beat grids (never `master.db`), finds the audio by unique
//! file name and prints aggregates only: no titles or paths, so results can be recorded in docs.
//! rekordbox is a reference and a de facto convention, not ground truth.
//!
//!   cargo run --release -p curado-audio --example grid_eval -- \
//!     --music ~/Music --split dev --limit 300 [--anlz DIR] [--threads 6] [--json out.json]
//!
//! `--split` hashes each file name, so `dev` (used for tuning) and `holdout` never overlap.
use curado_audio::{decode, music};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    time::Instant,
};

struct Reference {
    name: String,
    /// `(beat within bar, tempo × 100, milliseconds)` per grid line.
    beats: Vec<(u16, u16, u32)>,
    modified: std::time::SystemTime,
}
#[derive(Default, Clone)]
struct Row {
    format: String,
    reference_bpm: f64,
    bpm: f64,
    confidence: f64,
    /// Our grid minus rekordbox's, near the start and end of the track.
    error_start_ms: f64,
    error_end_ms: f64,
    tempo_family: Family,
    bar_match: Option<bool>,
    analysis_ms: f64,
    audio_seconds: f64,
}
#[derive(Default, Clone, Copy, PartialEq)]
enum Family {
    Same,
    Double,
    Half,
    Ratio,
    #[default]
    Other,
    Unknown,
}

fn parse(path: &Path) -> Option<Reference> {
    let data = fs::read(path).ok()?;
    if data.get(..4)? != b"PMAI" {
        return None;
    }
    let u32_at = |at: usize| -> Option<usize> {
        Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?) as usize)
    };
    let mut position = u32_at(4)?;
    let mut name = None;
    let mut beats = Vec::new();
    while position + 12 <= data.len() {
        let header = u32_at(position + 4)?;
        let length = u32_at(position + 8)?;
        if length < 12 {
            break;
        }
        match data.get(position..position + 4)? {
            b"PPTH" => {
                let size = u32_at(position + 12)?;
                let raw = data.get(position + header..position + header + size)?;
                let wide: Vec<u16> = raw
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_be_bytes(*c))
                    .collect();
                name = Some(
                    String::from_utf16_lossy(&wide)
                        .trim_end_matches('\0')
                        .to_string(),
                );
            }
            b"PQTZ" => {
                let count = u32_at(position + 20)?;
                let start = position + header;
                beats.reserve(count.min(100_000));
                for chunk in data.get(start..start + count * 8)?.as_chunks::<8>().0 {
                    beats.push((
                        u16::from_be_bytes([chunk[0], chunk[1]]),
                        u16::from_be_bytes([chunk[2], chunk[3]]),
                        u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]),
                    ));
                }
            }
            _ => {}
        }
        position += length;
    }
    let name = name?.rsplit('/').next()?.to_string();
    (beats.len() >= 16 && !name.is_empty()).then(|| Reference {
        name,
        beats,
        modified: fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH),
    })
}
fn walk(root: &Path, visit: &mut dyn FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let hidden = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'));
        if hidden {
            continue;
        }
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => walk(&path, visit),
            Ok(kind) if kind.is_file() => visit(&path),
            _ => {}
        }
    }
}
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ byte as u64).wrapping_mul(0x100000001b3)
    })
}

fn evaluate(reference: &Reference, path: &Path) -> Result<Option<Row>, String> {
    let clip = decode::decode_with_limit(path, 1, Some(Default::default()), usize::MAX / 2)?;
    let beats = &reference.beats;
    let last = beats.last().unwrap().2 as f64 / 1000.;
    // A different version of the track under the same file name would be a false comparison.
    if (last - clip.duration()).abs() > 3. && last > clip.duration() + 0.5 {
        return Ok(None);
    }
    if last < clip.duration() * 0.5 {
        return Ok(None);
    }
    let started = Instant::now();
    let analysis = music::analyze(&clip.samples, clip.sample_rate);
    let analysis_ms = started.elapsed().as_secs_f64() * 1000.;
    let first = beats[0].2 as f64 / 1000.;
    let reference_period = (last - first) / (beats.len() - 1) as f64;
    let reference_bpm = beats[0].1 as f64 / 100.;
    let mut row = Row {
        format: path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("?")
            .to_ascii_lowercase(),
        reference_bpm,
        confidence: analysis.bpm_confidence as f64,
        analysis_ms,
        audio_seconds: clip.duration(),
        tempo_family: Family::Unknown,
        ..Default::default()
    };
    let Some(bpm) = analysis.bpm else {
        return Ok(Some(row));
    };
    row.bpm = bpm;
    let close = |ratio: f64| (bpm * ratio - reference_bpm).abs() <= 0.5;
    row.tempo_family = if close(1.) {
        Family::Same
    } else if close(0.5) {
        Family::Double
    } else if close(2.) {
        Family::Half
    } else if [1.5, 2. / 3., 4. / 3., 0.75].into_iter().any(close) {
        Family::Ratio
    } else {
        Family::Other
    };
    let period = 60. / bpm;
    let wrap = period.min(reference_period);
    let error_at = |time: f64| {
        let difference = (analysis.beat_offset - time).rem_euclid(wrap);
        (if difference > wrap / 2. {
            difference - wrap
        } else {
            difference
        }) * 1000.
    };
    row.error_start_ms = error_at(beats[beats.len().min(8) - 1].2 as f64 / 1000.);
    row.error_end_ms = error_at(beats[beats.len() - 8].2 as f64 / 1000.);
    if row.tempo_family == Family::Same && row.error_start_ms.abs() < reference_period * 250. {
        if let Some(line) = beats.iter().find(|b| b.0 == 1) {
            let reference_bar = line.2 as f64 / 1000.;
            let beats_apart = ((analysis.downbeat - reference_bar) / period).round() as i64;
            row.bar_match = Some(beats_apart.rem_euclid(4) == 0);
        }
    }
    Ok(Some(row))
}

fn percent(part: usize, whole: usize) -> f64 {
    part as f64 / whole.max(1) as f64 * 100.
}
fn sorted(values: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut values: Vec<f64> = values.collect();
    values.sort_by(f64::total_cmp);
    values
}
fn quantile(values: &[f64], q: f64) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values[((values.len() - 1) as f64 * q).round() as usize]
}
fn phase_summary(rows: &[&Row]) -> String {
    let errors = sorted(rows.iter().map(|r| r.error_start_ms));
    let within = |limit: f64| {
        percent(
            rows.iter()
                .filter(|r| r.error_start_ms.abs() < limit)
                .count(),
            rows.len(),
        )
    };
    format!(
        "{{\"n\":{},\"medianMs\":{:.1},\"p10Ms\":{:.1},\"p90Ms\":{:.1},\"within5\":{:.1},\"within10\":{:.1},\"within20\":{:.1},\"within50\":{:.1}}}",
        rows.len(),
        quantile(&errors, 0.5),
        quantile(&errors, 0.1),
        quantile(&errors, 0.9),
        within(5.),
        within(10.),
        within(20.),
        within(50.)
    )
}
fn summary(rows: &[Row]) -> String {
    let n = rows.len();
    let known: Vec<&Row> = rows
        .iter()
        .filter(|r| r.tempo_family != Family::Unknown)
        .collect();
    let family = |f: Family| percent(rows.iter().filter(|r| r.tempo_family == f).count(), n);
    let within = |limit: f64| {
        percent(
            known
                .iter()
                .filter(|r| (r.bpm - r.reference_bpm).abs() <= limit)
                .count(),
            n,
        )
    };
    let same: Vec<&Row> = rows
        .iter()
        .filter(|r| r.tempo_family == Family::Same)
        .collect();
    let tight: Vec<&Row> = same
        .iter()
        .copied()
        .filter(|r| (r.bpm - r.reference_bpm).abs() <= 0.1)
        .collect();
    let drift = sorted(
        tight
            .iter()
            .map(|r| (r.error_end_ms - r.error_start_ms).abs()),
    );
    let bars: Vec<bool> = same.iter().filter_map(|r| r.bar_match).collect();
    let right = sorted(same.iter().map(|r| r.confidence));
    let wrong = sorted(
        known
            .iter()
            .filter(|r| r.tempo_family != Family::Same)
            .map(|r| r.confidence),
    );
    let mut formats: Vec<String> = rows.iter().map(|r| r.format.clone()).collect();
    formats.sort();
    formats.dedup();
    let by_format: Vec<String> = formats
        .iter()
        .map(|format| {
            let selected: Vec<&Row> = tight
                .iter()
                .copied()
                .filter(|r| &r.format == format)
                .collect();
            format!("\"{format}\":{}", phase_summary(&selected))
        })
        .collect();
    let realtime: f64 = rows.iter().map(|r| r.audio_seconds).sum::<f64>()
        / (rows.iter().map(|r| r.analysis_ms).sum::<f64>() / 1000.).max(1e-9);
    let analysis = sorted(rows.iter().map(|r| r.analysis_ms));
    format!(
        "{{\"tracks\":{n},\"noBpm\":{:.1},\"bpmWithin0.02\":{:.1},\"bpmWithin0.1\":{:.1},\"bpmWithin0.5\":{:.1},\"double\":{:.1},\"half\":{:.1},\"ratio\":{:.1},\"other\":{:.1},\"confidenceMedianRight\":{:.2},\"confidenceMedianWrong\":{:.2},\"phaseTight\":{},\"phaseByFormat\":{{{}}},\"driftMedianMs\":{:.1},\"driftP90Ms\":{:.1},\"barsCompared\":{},\"barMatch\":{:.1},\"analysisMedianMs\":{:.0},\"analysisMaxMs\":{:.0},\"timesRealtime\":{:.0}}}",
        family(Family::Unknown),
        within(0.02),
        within(0.1),
        within(0.5),
        family(Family::Double),
        family(Family::Half),
        family(Family::Ratio),
        family(Family::Other),
        quantile(&right, 0.5),
        quantile(&wrong, 0.5),
        phase_summary(&tight),
        by_format.join(","),
        quantile(&drift, 0.5),
        quantile(&drift, 0.9),
        bars.len(),
        percent(bars.iter().filter(|b| **b).count(), bars.len()),
        quantile(&analysis, 0.5),
        analysis.last().copied().unwrap_or(f64::NAN),
        realtime
    )
}

fn main() {
    let mut anlz = dirs_home().join("Library/Pioneer/rekordbox/share/PIONEER/USBANLZ");
    let mut music_roots: Vec<PathBuf> = Vec::new();
    let mut split = "dev".to_string();
    let mut limit = 200;
    let mut threads = 6;
    let mut json = None;
    let mut csv = None;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args.next().unwrap_or_default();
        match flag.as_str() {
            "--anlz" => anlz = value.into(),
            "--music" => music_roots.push(value.into()),
            "--split" => split = value,
            "--limit" => limit = value.parse().expect("--limit"),
            "--threads" => threads = value.parse().expect("--threads"),
            "--json" => json = Some(value),
            "--rows" => csv = Some(value),
            other => panic!("unknown flag {other}"),
        }
    }
    if music_roots.is_empty() {
        music_roots.push(dirs_home().join("Music"));
    }
    let mut newest: HashMap<String, Reference> = HashMap::new();
    walk(&anlz, &mut |path| {
        if path.file_name().is_some_and(|n| n == "ANLZ0000.DAT") {
            if let Some(reference) = parse(path) {
                let keep = newest
                    .get(&reference.name)
                    .is_none_or(|old| reference.modified > old.modified);
                if keep {
                    newest.insert(reference.name.clone(), reference);
                }
            }
        }
    });
    let mut located: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for root in &music_roots {
        walk(root, &mut |path| {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if newest.contains_key(name) {
                    located
                        .entry(name.to_string())
                        .or_default()
                        .push(path.to_path_buf());
                }
            }
        });
    }
    let mut jobs: Vec<(u64, String, PathBuf)> = located
        .into_iter()
        .filter(|(_, paths)| paths.len() == 1)
        .map(|(name, mut paths)| (fnv(&name), name, paths.remove(0)))
        .filter(|(hash, ..)| match split.as_str() {
            "dev" => hash % 2 == 0,
            "holdout" => hash % 2 == 1,
            _ => true,
        })
        .collect();
    jobs.sort_by_key(|job| job.0);
    let available = jobs.len();
    jobs.truncate(limit);
    eprintln!(
        "{} reference grids, {available} located in split {split}, evaluating {}",
        newest.len(),
        jobs.len()
    );
    let next = AtomicUsize::new(0);
    let rows = Mutex::new(Vec::new());
    let skipped = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some((_, name, path)) = jobs.get(index) else {
                    break;
                };
                match evaluate(&newest[name], path) {
                    Ok(Some(row)) => rows.lock().unwrap().push(row),
                    Ok(None) => {
                        skipped.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }
    });
    let rows = rows.into_inner().unwrap();
    let report = format!(
        "{{\"kind\":\"agreement with stored rekordbox grids, not ground truth\",\"split\":\"{split}\",\"located\":{available},\"differentVersionSkipped\":{},\"decodeFailed\":{},\"result\":{}}}",
        skipped.load(Ordering::Relaxed),
        failed.load(Ordering::Relaxed),
        summary(&rows)
    );
    println!("{report}");
    if let Some(path) = json {
        fs::write(path, &report).expect("write report");
    }
    if let Some(path) = csv {
        let lines: Vec<String> = rows
            .iter()
            .map(|r| {
                format!(
                    "{},{:.2},{:.2},{:.2},{:.1},{:.1},{:?}",
                    r.format,
                    r.reference_bpm,
                    r.bpm,
                    r.confidence,
                    r.error_start_ms,
                    r.error_end_ms,
                    r.bar_match
                )
            })
            .collect();
        fs::write(path, lines.join("\n")).expect("write rows");
    }
}
fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}
