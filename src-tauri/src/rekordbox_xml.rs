//! Writes a rekordbox.xml, the one route rekordbox itself offers for bringing data in. curado
//! never writes to a rekordbox drive or database: this file goes where the user chooses, and
//! rekordbox decides what to import from it.
use crate::devices::{self, CueInfo, DeviceRecord};
use crate::library::Database;
use curado_audio::annotations::HOT_CUES;
use curado_audio::music::{MusicAnalysis, MusicalKey};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A single import of a track replaces what rekordbox has, so this is its own playlist: the
/// tracks that bring something, without the ones that are only listed in a curado playlist.
const CHANGES_PLAYLIST: &str = "curado cues";
const FOLDER: &str = "curado";
const MAX_SEARCH_ENTRIES: usize = 2_000_000;
const MAX_SEARCH_TIME: Duration = Duration::from_secs(120);
/// Written when a file has no artist tag. It must not replace what rekordbox knows.
const UNKNOWN_ARTIST: &str = "Artista desconocido";
/// Classic notation. `Tonality` is free text, and drives also hold Camelot and Open Key names.
const MAJOR_KEYS: [&str; 12] = [
    "C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B",
];
const MINOR_KEYS: [&str; 12] = [
    "Cm", "C#m", "Dm", "Ebm", "Em", "Fm", "F#m", "Gm", "G#m", "Am", "Bbm", "Bm",
];

#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub saved: bool,
    pub tracks: usize,
    pub changed: usize,
    pub hot_cues: usize,
    pub memory_cues: usize,
    pub grids: usize,
    pub playlists: usize,
    /// Tracks from a drive whose original file was found on this computer.
    pub relocated: usize,
    /// Tracks from a drive with no certain original: rekordbox will see a different file.
    pub on_device: usize,
    pub search_complete: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct Mark {
    name: String,
    start: f64,
    end: Option<f64>,
    /// Hot cue A is 0; a memory cue is -1.
    num: i32,
    color: Option<[u8; 3]>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Grid {
    bpm: f64,
    start: f64,
    beat_in_bar: i64,
}
#[derive(Clone, Debug)]
struct Item {
    id: u64,
    title: String,
    artist: String,
    path: String,
    duration: f64,
    sample_rate: u32,
    size: Option<u64>,
    marks: Vec<Mark>,
    grid: Option<Grid>,
    key: Option<MusicalKey>,
    from_device: bool,
    changed: bool,
}
#[derive(Debug, Default)]
pub struct Plan {
    items: Vec<Item>,
    playlists: Vec<(String, Vec<u64>)>,
}

fn marks(cues: &[f64; HOT_CUES], ends: &[f64; HOT_CUES], info: &CueInfo) -> Vec<Mark> {
    let mut marks = Vec::new();
    for i in 0..HOT_CUES {
        // Zero is how curado stores a cue that is not set.
        if cues[i] <= 0. || !cues[i].is_finite() {
            continue;
        }
        let style = info.hot.get(i).and_then(Option::as_ref);
        marks.push(Mark {
            name: style.and_then(|s| s.label.clone()).unwrap_or_default(),
            start: cues[i],
            end: (ends[i] > cues[i] && ends[i].is_finite()).then_some(ends[i]),
            num: i as i32,
            color: style.and_then(|s| s.color),
        });
    }
    for cue in &info.memory {
        if cue.start.is_finite() && cue.start >= 0. {
            marks.push(Mark {
                name: cue.label.clone().unwrap_or_default(),
                start: cue.start,
                end: cue.end.filter(|end| end.is_finite() && *end > cue.start),
                num: -1,
                color: cue.color,
            });
        }
    }
    marks
}

fn grid(music: &MusicAnalysis) -> Option<Grid> {
    let bpm = music.bpm.filter(|bpm| bpm.is_finite() && *bpm > 0.)?;
    let period = 60. / bpm;
    let start = music.beat_offset.rem_euclid(period);
    let beats_from_bar = ((start - music.downbeat) / period).round() as i64;
    Some(Grid {
        bpm,
        start,
        beat_in_bar: beats_from_bar.rem_euclid(4) + 1,
    })
}

/// Decides what goes in the file. Only a correction by hand replaces rekordbox's grid and key:
/// its analysis handles variable tempo and an estimate from here would overwrite it. A drive's
/// track that nobody edited has nothing to send, and sending it could undo newer work there.
pub fn plan(db: &Database) -> Result<Plan, String> {
    let library = db.library()?;
    let listed: HashSet<u64> = library
        .playlists
        .iter()
        .flat_map(|playlist| playlist.tracks.iter().copied())
        .collect();
    let mut items = Vec::new();
    for track in library.tracks {
        let (cues, ends) = db.saved_cues(track.id);
        let record: Option<DeviceRecord> = db.setting(&format!("device-track:{}", track.id));
        let edited = match &record {
            Some(record) => !devices::unchanged_cues(record, &cues, &ends),
            None => cues.iter().any(|cue| *cue > 0.),
        };
        let manual = track.music.as_ref().filter(|music| music.manual);
        let changed = edited || manual.is_some();
        if !changed && !listed.contains(&track.id) {
            continue;
        }
        let info = devices::current_cue_info(db, track.id);
        items.push(Item {
            id: track.id,
            title: track.title,
            artist: track.artist,
            size: fs::metadata(&track.path).ok().map(|file| file.len()),
            path: track.path,
            duration: track.duration,
            sample_rate: track.sample_rate,
            marks: marks(&cues, &ends, &info),
            grid: manual.and_then(grid),
            key: manual.and_then(|music| music.key),
            from_device: record.is_some(),
            changed,
        });
    }
    let playlists = library
        .playlists
        .into_iter()
        .map(|playlist| (playlist.name, playlist.tracks))
        .collect();
    Ok(Plan { items, playlists })
}

type Wanted = (OsString, u64);
/// Every file with one of these names and sizes under the roots. Links are not followed, so a
/// loop on a drive cannot trap it, and the limits keep a slow network drive from holding it up.
fn search(
    wanted: &HashSet<Wanted>,
    roots: &[PathBuf],
    max_entries: usize,
    max_time: Duration,
) -> (HashMap<Wanted, Vec<PathBuf>>, bool) {
    let mut found: HashMap<Wanted, Vec<PathBuf>> = HashMap::new();
    let names: HashSet<&OsString> = wanted.iter().map(|(name, _)| name).collect();
    let started = Instant::now();
    let mut pending: Vec<PathBuf> = roots.to_vec();
    let mut entries = 0;
    while let Some(folder) = pending.pop() {
        let Ok(listing) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in listing.flatten() {
            entries += 1;
            if entries > max_entries || started.elapsed() > max_time {
                return (found, false);
            }
            let name = entry.file_name();
            if name.to_string_lossy().starts_with('.') {
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() && names.contains(&name) {
                let Ok(size) = entry.metadata().map(|file| file.len()) else {
                    continue;
                };
                let key = (name, size);
                if wanted.contains(&key) {
                    found.entry(key).or_default().push(entry.path());
                }
            }
        }
    }
    (found, true)
}

/// rekordbox identifies a track by its location, and its collection knows the file on the
/// computer rather than the copy on the drive. A file with the same name and size is taken as
/// that original only when there is exactly one: two copies leave no way to know which it uses.
fn relocate(plan: &mut Plan, report: &mut ExportReport, roots: &[PathBuf]) {
    let key = |item: &Item| -> Option<Wanted> {
        Some((Path::new(&item.path).file_name()?.to_owned(), item.size?))
    };
    let wanted: HashSet<Wanted> = plan
        .items
        .iter()
        .filter(|item| item.from_device)
        .filter_map(key)
        .collect();
    report.search_complete = true;
    if wanted.is_empty() {
        report.on_device = plan.items.iter().filter(|item| item.from_device).count();
        return;
    }
    let (found, complete) = search(&wanted, roots, MAX_SEARCH_ENTRIES, MAX_SEARCH_TIME);
    report.search_complete = complete;
    for item in plan.items.iter_mut().filter(|item| item.from_device) {
        let original = key(item)
            .and_then(|key| found.get(&key))
            .filter(|paths| paths.len() == 1)
            .and_then(|paths| paths[0].to_str());
        match original {
            Some(path) => {
                item.path = path.to_owned();
                report.relocated += 1;
            }
            None => report.on_device += 1,
        }
    }
}

/// Where originals may be: the music folder and drives that are not rekordbox exports, whose
/// copies are exactly what we are trying to look past.
fn search_roots() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    home.map(|home| Path::new(&home).join("Music"))
        .into_iter()
        .chain(
            devices::mount_points()
                .into_iter()
                .filter(|mount| !devices::has_export(mount)),
        )
        .collect()
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // XML 1.0 cannot carry these even escaped, and a tag can hold anything.
            '\u{FFFE}' | '\u{FFFF}' => {}
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Percent-encodes the bytes of the path as they are on disk, whatever their Unicode form.
fn location(path: &str) -> String {
    let path = if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.to_owned()
    };
    let path = path.trim_start_matches('/');
    let mut out = String::from("file://localhost/");
    let drive = cfg!(windows) && path.as_bytes().get(1) == Some(&b':');
    for (i, byte) in path.bytes().enumerate() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(byte as char)
            }
            b':' if drive && i == 1 => out.push(':'),
            _ => {
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

fn kind(path: &str) -> Option<&'static str> {
    let extension = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "mp3" => Some("MP3 File"),
        "wav" => Some("WAV File"),
        "flac" => Some("FLAC File"),
        "aif" | "aiff" => Some("AIFF File"),
        "m4a" | "aac" => Some("M4A File"),
        _ => None,
    }
}

fn key_name(key: MusicalKey) -> Option<&'static str> {
    let names = if key.minor { &MINOR_KEYS } else { &MAJOR_KEYS };
    names.get(key.tonic as usize).copied()
}

/// Attributes curado does not know are left out instead of written empty: an import replaces
/// fields, and rekordbox holds comments, genres and ratings curado never sees.
fn render_track(out: &mut String, id: usize, item: &Item) {
    let _ = write!(out, "    <TRACK TrackID=\"{id}\"");
    if !item.title.is_empty() {
        let _ = write!(out, " Name=\"{}\"", escape(&item.title));
    }
    if !item.artist.is_empty() && item.artist != UNKNOWN_ARTIST {
        let _ = write!(out, " Artist=\"{}\"", escape(&item.artist));
    }
    if let Some(kind) = kind(&item.path) {
        let _ = write!(out, " Kind=\"{kind}\"");
    }
    if let Some(size) = item.size {
        let _ = write!(out, " Size=\"{size}\"");
    }
    if item.duration >= 1. {
        let _ = write!(out, " TotalTime=\"{:.0}\"", item.duration.floor());
    }
    if let Some(grid) = &item.grid {
        let _ = write!(out, " AverageBpm=\"{:.2}\"", grid.bpm);
    }
    if item.sample_rate > 0 {
        let _ = write!(out, " SampleRate=\"{}\"", item.sample_rate);
    }
    if let Some(name) = item.key.and_then(key_name) {
        let _ = write!(out, " Tonality=\"{name}\"");
    }
    let _ = write!(out, " Location=\"{}\"", escape(&location(&item.path)));
    if item.grid.is_none() && item.marks.is_empty() {
        out.push_str("/>\n");
        return;
    }
    out.push_str(">\n");
    if let Some(grid) = &item.grid {
        let _ = writeln!(
            out,
            "      <TEMPO Inizio=\"{:.3}\" Bpm=\"{:.2}\" Metro=\"4/4\" Battito=\"{}\"/>",
            grid.start, grid.bpm, grid.beat_in_bar
        );
    }
    for mark in &item.marks {
        let _ = write!(
            out,
            "      <POSITION_MARK Name=\"{}\" Type=\"{}\" Start=\"{:.3}\"",
            escape(&mark.name),
            if mark.end.is_some() { 4 } else { 0 },
            mark.start
        );
        if let Some(end) = mark.end {
            let _ = write!(out, " End=\"{end:.3}\"");
        }
        let _ = write!(out, " Num=\"{}\"", mark.num);
        if let Some([red, green, blue]) = mark.color {
            let _ = write!(out, " Red=\"{red}\" Green=\"{green}\" Blue=\"{blue}\"");
        }
        out.push_str("/>\n");
    }
    out.push_str("    </TRACK>\n");
}

fn render_playlist(out: &mut String, name: &str, tracks: &[usize]) {
    let _ = writeln!(
        out,
        "        <NODE Type=\"1\" Name=\"{}\" KeyType=\"0\" Entries=\"{}\">",
        escape(name),
        tracks.len()
    );
    for track in tracks {
        let _ = writeln!(out, "          <TRACK Key=\"{track}\"/>");
    }
    out.push_str("        </NODE>\n");
}

/// Numbers are written with a period in any locale, and track ids are local to the file.
fn render(plan: &Plan, report: &mut ExportReport) -> String {
    let ids: HashMap<u64, usize> = plan
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| (item.id, i + 1))
        .collect();
    let changes: Vec<usize> = plan
        .items
        .iter()
        .filter(|item| item.changed)
        .map(|item| ids[&item.id])
        .collect();
    let mut lists: Vec<(&str, Vec<usize>)> = Vec::new();
    if !changes.is_empty() {
        lists.push((CHANGES_PLAYLIST, changes));
    }
    for (name, tracks) in &plan.playlists {
        let tracks = tracks
            .iter()
            .filter_map(|id| ids.get(id).copied())
            .collect();
        lists.push((name, tracks));
    }
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<DJ_PLAYLISTS Version=\"1.0.0\">\n");
    let _ = writeln!(
        out,
        "  <PRODUCT Name=\"curado\" Version=\"{}\" Company=\"curado\"/>",
        env!("CARGO_PKG_VERSION")
    );
    let _ = writeln!(out, "  <COLLECTION Entries=\"{}\">", plan.items.len());
    for (i, item) in plan.items.iter().enumerate() {
        render_track(&mut out, i + 1, item);
    }
    out.push_str("  </COLLECTION>\n  <PLAYLISTS>\n");
    // A folder of its own, so that no playlist lands beside one of the same name in rekordbox.
    if lists.is_empty() {
        out.push_str("    <NODE Type=\"0\" Name=\"ROOT\" Count=\"0\"/>\n");
    } else {
        out.push_str("    <NODE Type=\"0\" Name=\"ROOT\" Count=\"1\">\n");
        let _ = writeln!(
            out,
            "      <NODE Type=\"0\" Name=\"{FOLDER}\" Count=\"{}\">",
            lists.len()
        );
        for (name, tracks) in &lists {
            render_playlist(&mut out, name, tracks);
        }
        out.push_str("      </NODE>\n    </NODE>\n");
    }
    out.push_str("  </PLAYLISTS>\n</DJ_PLAYLISTS>\n");
    report.tracks = plan.items.len();
    report.changed = plan.items.iter().filter(|item| item.changed).count();
    report.playlists = lists.len();
    report.grids = plan.items.iter().filter(|item| item.grid.is_some()).count();
    for mark in plan.items.iter().flat_map(|item| &item.marks) {
        if mark.num < 0 {
            report.memory_cues += 1;
        } else {
            report.hot_cues += 1;
        }
    }
    out
}

/// The whole file appears at once or not at all, so a failure leaves any earlier export intact.
fn write_atomic(destination: &Path, text: &str) -> Result<(), String> {
    let mut temporary = destination.as_os_str().to_owned();
    temporary.push(".curado-tmp");
    let temporary = PathBuf::from(temporary);
    let written = fs::write(&temporary, text).and_then(|()| fs::rename(&temporary, destination));
    if let Err(error) = written {
        let _ = fs::remove_file(&temporary);
        return Err(format!("No se pudo guardar el XML: {error}"));
    }
    Ok(())
}

/// Runs outside any lock on the library: searching for originals can take a while.
pub fn export(mut plan: Plan, destination: &Path) -> Result<ExportReport, String> {
    let mut report = ExportReport::default();
    relocate(&mut plan, &mut report, &search_roots());
    let text = render(&plan, &mut report);
    write_atomic(destination, &text)?;
    report.saved = true;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::{CueStyle, MemoryCue};

    fn item(id: u64, path: &str) -> Item {
        Item {
            id,
            title: "T".into(),
            artist: "A".into(),
            path: path.into(),
            duration: 200.7,
            sample_rate: 44100,
            size: Some(10),
            marks: Vec::new(),
            grid: None,
            key: None,
            from_device: false,
            changed: true,
        }
    }
    fn database() -> Database {
        Database::open(Path::new(":memory:")).unwrap()
    }
    fn track(db: &Database, path: &str) -> u64 {
        db.register_reference(Path::new(path), "T", "A", 100.)
            .unwrap()
    }

    #[test]
    fn locations_encode_every_reserved_byte_of_each_segment() {
        let encoded = location("/Música/AC&DC #1/100% (mix)?.mp3");
        assert_eq!(
            encoded,
            "file://localhost/M%C3%BAsica/AC%26DC%20%231/100%25%20%28mix%29%3F.mp3"
        );
        assert_eq!(location("/a:b.mp3"), "file://localhost/a%3Ab.mp3");
        // The bytes stay as they are on disk: a decomposed accent is not silently composed.
        assert!(location("/Mu\u{301}sica").contains("Mu%CC%81sica"));
    }

    #[test]
    fn text_cannot_break_out_of_an_attribute() {
        assert_eq!(
            escape("<a href=\"x\">R&B's\u{1}\u{FFFF}"),
            "&lt;a href=&quot;x&quot;&gt;R&amp;B&apos;s "
        );
    }

    #[test]
    fn every_key_name_reads_back_as_the_same_key() {
        for tonic in 0..12 {
            for minor in [false, true] {
                let key = MusicalKey { tonic, minor };
                assert_eq!(
                    crate::rekordbox::parse_key(key_name(key).unwrap()),
                    Some(key)
                );
            }
        }
    }

    #[test]
    fn keys_use_classic_names() {
        let name = |tonic, minor| key_name(MusicalKey { tonic, minor }).unwrap();
        assert_eq!(name(9, true), "Am");
        assert_eq!(name(1, false), "Db");
        assert_eq!(name(1, true), "C#m");
        assert_eq!(
            key_name(MusicalKey {
                tonic: 12,
                minor: false
            }),
            None
        );
    }

    #[test]
    fn hot_cues_loops_and_memory_cues_become_position_marks() {
        let mut cues = [0.; HOT_CUES];
        let mut ends = [0.; HOT_CUES];
        cues[0] = 1.5;
        cues[2] = 10.;
        ends[2] = 12.;
        // An end before its start is not a loop, and an unset cue is not at the top of the track.
        cues[3] = 20.;
        ends[3] = 5.;
        let info = CueInfo {
            hot: vec![Some(CueStyle {
                color: Some([1, 2, 3]),
                label: Some("Drop".into()),
            })],
            memory: vec![
                MemoryCue {
                    start: 0.,
                    end: None,
                    color: None,
                    label: None,
                },
                MemoryCue {
                    start: 30.,
                    end: Some(31.),
                    color: None,
                    label: None,
                },
            ],
        };
        let marks = marks(&cues, &ends, &info);
        assert_eq!(
            marks.iter().map(|m| m.num).collect::<Vec<_>>(),
            [0, 2, 3, -1, -1]
        );
        assert_eq!(marks[0].name, "Drop");
        assert_eq!(marks[0].color, Some([1, 2, 3]));
        assert_eq!(marks[1].end, Some(12.));
        assert_eq!(marks[2].end, None);
        assert_eq!(marks[3].start, 0.);
        assert_eq!(marks[4].end, Some(31.));
    }

    #[test]
    fn the_grid_starts_on_its_first_beat_and_names_its_place_in_the_bar() {
        let music = MusicAnalysis {
            bpm: Some(120.),
            beat_offset: 2.25,
            downbeat: 1.75,
            manual: true,
            ..Default::default()
        };
        let grid = grid(&music).unwrap();
        assert_eq!(grid.bpm, 120.);
        assert!((grid.start - 0.25).abs() < 1e-9);
        // The first beat is three beats before a downbeat: the second of its bar.
        assert_eq!(grid.beat_in_bar, 2);
    }

    #[test]
    fn the_file_holds_marks_and_grid_but_nothing_curado_does_not_know() {
        let mut changed = item(7, "/a b.mp3");
        changed.marks = vec![
            Mark {
                name: "x".into(),
                start: 1.23456,
                end: None,
                num: 0,
                color: Some([1, 2, 3]),
            },
            Mark {
                name: String::new(),
                start: 2.,
                end: Some(4.),
                num: -1,
                color: None,
            },
        ];
        changed.grid = Some(Grid {
            bpm: 124.,
            start: 0.5,
            beat_in_bar: 3,
        });
        changed.key = Some(MusicalKey {
            tonic: 9,
            minor: true,
        });
        let mut listed = item(9, "/c.wav");
        listed.changed = false;
        listed.artist = UNKNOWN_ARTIST.into();
        let plan = Plan {
            items: vec![changed, listed],
            playlists: vec![("mía".into(), vec![9, 7, 404])],
        };
        let mut report = ExportReport::default();
        let text = render(&plan, &mut report);
        assert!(text.contains("<COLLECTION Entries=\"2\">"));
        assert!(text.contains(
            "<TRACK TrackID=\"1\" Name=\"T\" Artist=\"A\" Kind=\"MP3 File\" Size=\"10\" \
             TotalTime=\"200\" AverageBpm=\"124.00\" SampleRate=\"44100\" Tonality=\"Am\" \
             Location=\"file://localhost/a%20b.mp3\">"
        ));
        assert!(
            text.contains("<TEMPO Inizio=\"0.500\" Bpm=\"124.00\" Metro=\"4/4\" Battito=\"3\"/>")
        );
        assert!(text.contains(
            "<POSITION_MARK Name=\"x\" Type=\"0\" Start=\"1.235\" Num=\"0\" Red=\"1\" Green=\"2\" Blue=\"3\"/>"
        ));
        assert!(text.contains(
            "<POSITION_MARK Name=\"\" Type=\"4\" Start=\"2.000\" End=\"4.000\" Num=\"-1\"/>"
        ));
        // The placeholder artist is not a fact about the track, and neither are empty fields.
        assert!(!text.contains(UNKNOWN_ARTIST));
        assert!(!text.contains("Comments") && !text.contains("Genre") && !text.contains("Rating"));
        assert!(text.contains("<NODE Type=\"1\" Name=\"curado cues\" KeyType=\"0\" Entries=\"1\">"));
        // A playlist keeps its order and loses an entry that is not in the collection.
        assert!(text.contains(
            "<NODE Type=\"1\" Name=\"mía\" KeyType=\"0\" Entries=\"2\">\n          <TRACK Key=\"2\"/>\n          <TRACK Key=\"1\"/>"
        ));
        assert_eq!(
            report,
            ExportReport {
                tracks: 2,
                changed: 1,
                hot_cues: 1,
                memory_cues: 1,
                grids: 1,
                playlists: 2,
                ..Default::default()
            }
        );
    }

    #[test]
    fn an_empty_library_is_still_a_complete_file() {
        let mut report = ExportReport::default();
        let text = render(&Plan::default(), &mut report);
        assert!(text.contains("<COLLECTION Entries=\"0\">"));
        assert!(text.contains("<NODE Type=\"0\" Name=\"ROOT\" Count=\"0\"/>"));
        assert_eq!(report.playlists, 0);
    }

    #[test]
    fn only_tracks_with_something_to_send_or_a_playlist_are_included() {
        let db = database();
        let untouched = track(&db, "/untouched.mp3");
        let cued = track(&db, "/cued.mp3");
        let listed = track(&db, "/listed.mp3");
        let estimated = track(&db, "/estimated.mp3");
        let corrected = track(&db, "/corrected.mp3");
        let mut cues = vec![0.; HOT_CUES];
        cues[1] = 5.;
        db.save(&format!("cues:{cued}"), &cues).unwrap();
        db.edit(crate::library::LibraryEdit::CreatePlaylist { name: "p".into() })
            .unwrap();
        let playlist = db.library().unwrap().playlists[0].id;
        db.edit(crate::library::LibraryEdit::AddToPlaylist {
            playlist,
            track: listed,
        })
        .unwrap();
        let music = MusicAnalysis {
            version: curado_audio::music::ANALYSIS_VERSION,
            bpm: Some(120.),
            key: Some(MusicalKey {
                tonic: 0,
                minor: false,
            }),
            ..Default::default()
        };
        db.save(&format!("music:{estimated}"), &music).unwrap();
        db.save(
            &format!("music:{corrected}"),
            &MusicAnalysis {
                manual: true,
                ..music
            },
        )
        .unwrap();
        let plan = plan(&db).unwrap();
        let of = |id: u64| plan.items.iter().find(|item| item.id == id);
        assert!(of(untouched).is_none());
        assert!(
            of(estimated).is_none(),
            "an estimate must not replace rekordbox's analysis"
        );
        assert!(of(cued).unwrap().changed);
        assert!(!of(listed).unwrap().changed);
        let corrected = of(corrected).unwrap();
        assert!(corrected.grid.is_some() && corrected.key.is_some());
    }

    #[test]
    fn a_track_from_a_drive_counts_only_once_it_was_edited() {
        let db = database();
        let id = track(&db, "/Volumes/USB/Contents/a.mp3");
        let mut cues = vec![0.; HOT_CUES];
        cues[0] = 8.;
        let ends = vec![0.; HOT_CUES];
        db.save(&format!("cues:{id}"), &cues).unwrap();
        db.save(&format!("cue-ends:{id}"), &ends).unwrap();
        db.save(
            &format!("device-track:{id}"),
            &DeviceRecord {
                analysis_stamp: None,
                imported_cues: cues.clone(),
                imported_ends: ends.clone(),
            },
        )
        .unwrap();
        db.save(
            &format!("cue-info:{id}"),
            &CueInfo {
                hot: vec![Some(CueStyle {
                    color: Some([9, 9, 9]),
                    label: Some("Old".into()),
                })],
                memory: vec![MemoryCue {
                    start: 3.,
                    end: None,
                    color: None,
                    label: None,
                }],
            },
        )
        .unwrap();
        assert!(plan(&db).unwrap().items.is_empty());
        // Moved in curado: rekordbox's label described the old position, and the memory cues stay.
        cues[0] = 9.;
        cues[1] = 12.;
        db.save(&format!("cues:{id}"), &cues).unwrap();
        let plan = plan(&db).unwrap();
        let marks = &plan.items[0].marks;
        assert!(plan.items[0].from_device);
        assert_eq!(marks.len(), 3);
        assert_eq!(
            (marks[0].start, marks[0].name.as_str(), marks[0].color),
            (9., "", None)
        );
        assert_eq!(marks[2].num, -1);
    }

    #[cfg(unix)]
    fn folder(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("curado-xml-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[cfg(unix)]
    #[test]
    fn only_a_single_copy_with_the_same_name_and_size_is_an_original() {
        let root = folder("relocate");
        let device = root.join("usb");
        let music = root.join("music");
        for (path, bytes) in [
            ("usb/one.mp3", "xxx"),
            ("usb/two.mp3", "xxx"),
            ("usb/resized.mp3", "xxx"),
            ("usb/lost.mp3", "xxx"),
            ("music/deep/one.mp3", "yyy"),
            ("music/a/two.mp3", "yyy"),
            ("music/b/two.mp3", "yyy"),
            ("music/resized.mp3", "yyyy"),
            ("music/.hidden/lost.mp3", "yyy"),
        ] {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        // A link back to the drive must not make its copies look like originals, or loop.
        std::os::unix::fs::symlink(&device, music.join("link")).unwrap();
        std::os::unix::fs::symlink(&music, music.join("deep/loop")).unwrap();
        let mut plan = Plan::default();
        for (i, name) in ["one", "two", "resized", "lost"].iter().enumerate() {
            let mut item = item(
                i as u64,
                device.join(format!("{name}.mp3")).to_str().unwrap(),
            );
            item.size = Some(3);
            item.from_device = true;
            plan.items.push(item);
        }
        let local = item(9, "/not/a/device.mp3");
        plan.items.push(local);
        let mut report = ExportReport::default();
        relocate(&mut plan, &mut report, std::slice::from_ref(&music));
        assert_eq!(
            plan.items[0].path,
            music.join("deep/one.mp3").to_str().unwrap()
        );
        for i in 1..4 {
            assert!(plan.items[i].path.starts_with(device.to_str().unwrap()));
        }
        assert_eq!(plan.items[4].path, "/not/a/device.mp3");
        assert_eq!((report.relocated, report.on_device), (1, 3));
        assert!(report.search_complete);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_search_that_runs_out_of_budget_says_so() {
        let root = folder("budget");
        fs::write(root.join("a.mp3"), "x").unwrap();
        fs::write(root.join("b.mp3"), "x").unwrap();
        let wanted = HashSet::from([(OsString::from("a.mp3"), 1)]);
        let roots = [root.clone()];
        assert!(!search(&wanted, &roots, 1, MAX_SEARCH_TIME).1);
        assert!(!search(&wanted, &roots, 100, Duration::ZERO).1);
        let (found, complete) = search(&wanted, &roots, 100, MAX_SEARCH_TIME);
        assert!(complete);
        assert_eq!(found.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_write_keeps_the_previous_file_and_leaves_nothing_behind() {
        let root = folder("write");
        let file = root.join("out.xml");
        write_atomic(&file, "first").unwrap();
        write_atomic(&file, "second").unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "second");
        // Renaming over a folder fails after the temporary file was written.
        let taken = root.join("taken.xml");
        fs::create_dir(&taken).unwrap();
        assert!(write_atomic(&taken, "x").is_err());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
        fs::remove_dir_all(root).unwrap();
    }

    /// `CURADO_TEST_DEVICE=/Volumes/X CURADO_TEST_XML=out.xml cargo test --release -p curado
    /// real_xml -- --ignored --nocapture`. Reads the drive and writes only the given file.
    #[test]
    #[ignore = "needs a real rekordbox USB"]
    fn real_xml() {
        use crate::annotations::Annotations;
        use crate::devices::{DeviceInfo, Devices};
        let root = PathBuf::from(std::env::var("CURADO_TEST_DEVICE").unwrap());
        let output = PathBuf::from(std::env::var("CURADO_TEST_XML").unwrap());
        let name = root.file_name().unwrap().to_string_lossy().to_string();
        let devices = Devices::default();
        devices.remember(
            DeviceInfo {
                id: name.clone(),
                name: name.clone(),
                rekordbox: true,
            },
            root.clone(),
        );
        let drive = devices.export(&name).unwrap();
        let db = database();
        let mut annotations = Annotations::default();
        let mut edited = 0;
        for track in drive.tracks.iter().take(400) {
            let id = devices::import_track(&db, &mut annotations, &root, track).unwrap();
            // An edit in this in-memory library: the drive and the files stay as they are.
            if edited < 200 {
                let (mut cues, _) = db.saved_cues(id);
                cues[7] = 42.5;
                db.save(&format!("cues:{id}"), &cues.to_vec()).unwrap();
                edited += 1;
            }
        }
        let plan = plan(&db).unwrap();
        let started = Instant::now();
        let report = export(plan, &output).unwrap();
        println!("{report:?} in {:?}", started.elapsed());
    }
}
