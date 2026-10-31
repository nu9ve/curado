//! Connected drives: finding them, reading their rekordbox library and browsing their audio.
//!
//! Nothing here ever writes to a device. Its files are untrusted, and the interface never sends
//! or receives a path: it holds device names, rekordbox track ids and opaque folder entries, all
//! resolved here and checked to stay inside the drive.
use crate::{
    annotations::Annotations,
    library::{supported, Database},
    rekordbox::{self, Cue, DeviceTrack, Export, HOT_CUES},
};
use curado_audio::music::{MusicAnalysis, ANALYSIS_VERSION};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};

const EXPORT_FILE: &str = "PIONEER/rekordbox/export.pdb";
/// Far above any real library, and a bound on what a malicious drive can make us read.
const MAX_EXPORT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ANALYSIS_BYTES: u64 = 64 * 1024 * 1024;
const MAX_FOLDER_ENTRIES: usize = 5000;
const MAX_ENTRY_TOKENS: usize = 100_000;
/// How much of a folder's contents to look through to decide it holds audio.
const AUDIO_SEARCH_ENTRIES: usize = 2000;
const AUDIO_SEARCH_DEPTH: usize = 5;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub rekordbox: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct FolderEntry {
    pub entry: u64,
    pub name: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct DeviceFolder {
    pub name: String,
    pub parent: Option<u64>,
    pub folders: Vec<FolderEntry>,
    pub files: Vec<FolderEntry>,
    pub truncated: bool,
}
#[derive(Default)]
struct Entries {
    next: u64,
    paths: HashMap<u64, (String, PathBuf)>,
}
struct CachedExport {
    stamp: (u64, Option<SystemTime>),
    export: Arc<Export>,
}
#[derive(Default)]
pub struct Devices {
    known: Mutex<Vec<(DeviceInfo, PathBuf)>>,
    exports: Mutex<HashMap<String, CachedExport>>,
    entries: Mutex<Entries>,
}
/// Where operating systems mount external drives. The startup disk is not one of them.
pub fn mount_points() -> Vec<PathBuf> {
    let roots: Vec<PathBuf> = if cfg!(target_os = "macos") {
        vec!["/Volumes".into()]
    } else if let (true, Some(user)) = (cfg!(target_os = "linux"), std::env::var_os("USER")) {
        vec![
            Path::new("/media").join(&user),
            Path::new("/run/media").join(&user),
        ]
    } else {
        Vec::new()
    };
    let mut mounts = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            // macOS lists the startup disk here as a symbolic link to /.
            let is_directory = entry.file_type().is_ok_and(|kind| kind.is_dir());
            let hidden = entry.file_name().to_string_lossy().starts_with('.');
            if is_directory && !hidden {
                mounts.push(entry.path());
            }
        }
    }
    #[cfg(target_os = "windows")]
    for letter in 'D'..='Z' {
        let root = PathBuf::from(format!("{letter}:\\"));
        if root.exists() {
            mounts.push(root);
        }
    }
    mounts.sort();
    mounts
}
/// A drive prepared by rekordbox rather than one that just holds files.
pub fn has_export(mount: &Path) -> bool {
    mount.join(EXPORT_FILE).is_file()
}
impl Devices {
    /// Looks at every mount, outside any lock: a dead network drive can take long to answer.
    pub fn refresh(&self) {
        let found: Vec<(DeviceInfo, PathBuf)> = mount_points()
            .into_iter()
            .map(|path| {
                let name = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into(),
                );
                let info = DeviceInfo {
                    id: name.clone(),
                    name,
                    rekordbox: has_export(&path),
                };
                (info, path)
            })
            .collect();
        let mut known = self.known.lock().unwrap();
        if *known != found {
            // A drive that left may come back with different contents under the same name.
            self.exports
                .lock()
                .unwrap()
                .retain(|id, _| found.iter().any(|(info, _)| info.id == *id));
            *known = found;
        }
    }
    #[cfg(test)]
    pub fn remember(&self, info: DeviceInfo, root: PathBuf) {
        self.known.lock().unwrap().push((info, root));
    }
    pub fn list(&self) -> Vec<DeviceInfo> {
        let known = self.known.lock().unwrap();
        known.iter().map(|(info, _)| info.clone()).collect()
    }
    pub fn root(&self, device: &str) -> Result<PathBuf, String> {
        self.known
            .lock()
            .unwrap()
            .iter()
            .find(|(info, _)| info.id == device)
            .map(|(_, path)| path.clone())
            .ok_or_else(|| String::from("El dispositivo ya no está conectado"))
    }
    /// The drive's library, read once until the file changes.
    pub fn export(&self, device: &str) -> Result<Arc<Export>, String> {
        let path = self.root(device)?.join(EXPORT_FILE);
        let unreadable = |_| String::from("No se pudo leer la biblioteca del dispositivo");
        let metadata = fs::metadata(&path).map_err(unreadable)?;
        if metadata.len() > MAX_EXPORT_BYTES {
            return Err("La biblioteca del dispositivo es demasiado grande".into());
        }
        let stamp = (metadata.len(), metadata.modified().ok());
        if let Some(cached) = self.exports.lock().unwrap().get(device) {
            if cached.stamp == stamp {
                return Ok(cached.export.clone());
            }
        }
        let export = Arc::new(rekordbox::parse_export(
            &fs::read(&path).map_err(unreadable)?,
        )?);
        let cached = CachedExport {
            stamp,
            export: export.clone(),
        };
        self.exports.lock().unwrap().insert(device.into(), cached);
        Ok(export)
    }
    fn token(&self, device: &str, path: PathBuf) -> u64 {
        let mut entries = self.entries.lock().unwrap();
        // Bounded: an open folder listing simply asks to be reopened after this.
        if entries.paths.len() >= MAX_ENTRY_TOKENS {
            entries.paths.clear();
        }
        entries.next += 1;
        let token = entries.next;
        entries.paths.insert(token, (device.into(), path));
        token
    }
    /// A folder's subfolders with audio somewhere inside, and its audio files.
    pub fn browse(&self, device: &str, folder: Option<u64>) -> Result<DeviceFolder, String> {
        let root = self.root(device)?;
        let path = match folder {
            Some(entry) => self.entry(entry)?.1,
            None => root.clone(),
        };
        let path = inside(&root, &path)?;
        let top = inside(&root, &root)?;
        let parent = path.parent().filter(|_| path != top).map(Path::to_path_buf);
        let mut folders = Vec::new();
        let mut files = Vec::new();
        let mut truncated = false;
        let listing = fs::read_dir(&path).map_err(|_| "No se pudo abrir esta carpeta")?;
        for entry in listing.flatten() {
            if folders.len() + files.len() >= MAX_FOLDER_ENTRIES {
                truncated = true;
                break;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            // Never followed: a link could lead anywhere on this computer.
            if name.starts_with('.') || kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                let mut budget = AUDIO_SEARCH_ENTRIES;
                if !is_system_folder(&name)
                    && has_audio(&entry.path(), AUDIO_SEARCH_DEPTH, &mut budget)
                {
                    folders.push((name, entry.path()));
                }
            } else if kind.is_file() && supported(&entry.path()) {
                files.push((name, entry.path()));
            }
        }
        let tokens = |mut list: Vec<(String, PathBuf)>| -> Vec<FolderEntry> {
            list.sort_by_key(|(name, _)| name.to_lowercase());
            list.into_iter()
                .map(|(name, path)| FolderEntry {
                    entry: self.token(device, path),
                    name,
                })
                .collect()
        };
        Ok(DeviceFolder {
            name: path
                .file_name()
                .map_or_else(|| device.to_string(), |n| n.to_string_lossy().into()),
            parent: parent.map(|path| self.token(device, path)),
            folders: tokens(folders),
            files: tokens(files),
            truncated,
        })
    }
    fn entry(&self, entry: u64) -> Result<(String, PathBuf), String> {
        self.entries
            .lock()
            .unwrap()
            .paths
            .get(&entry)
            .cloned()
            .ok_or_else(|| String::from("Vuelve a abrir esta carpeta"))
    }
    /// A file from a listing, checked again: the drive may have changed since.
    pub fn file(&self, entry: u64) -> Result<PathBuf, String> {
        let (device, path) = self.entry(entry)?;
        resolve(&self.root(&device)?, &path)
    }
}
/// The rekordbox data and the operating system's own folders hold no music to browse.
fn is_system_folder(name: &str) -> bool {
    matches!(
        name,
        "PIONEER" | "System Volume Information" | "$RECYCLE.BIN"
    )
}
fn has_audio(folder: &Path, depth: usize, budget: &mut usize) -> bool {
    let Ok(entries) = fs::read_dir(folder) else {
        return false;
    };
    for entry in entries.flatten() {
        if *budget == 0 {
            // Too big to search: showing an empty folder beats hiding someone's music.
            return true;
        }
        *budget -= 1;
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_file() && supported(&entry.path()) {
            return true;
        }
        if kind.is_dir() && depth > 0 && has_audio(&entry.path(), depth - 1, budget) {
            return true;
        }
    }
    false
}
/// The real location of `path`, as long as it is inside `root` after following any links.
fn inside(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let outside = || String::from("La ruta queda fuera del dispositivo");
    let real_root = root.canonicalize().map_err(|_| outside())?;
    let real = path.canonicalize().map_err(|_| outside())?;
    real.starts_with(&real_root)
        .then_some(real)
        .ok_or_else(outside)
}
/// An audio file inside the drive, whatever the export or a listing claimed.
fn resolve(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let real = inside(root, path)?;
    if real.is_file() && supported(&real) {
        Ok(real)
    } else {
        Err("El archivo no es audio compatible".into())
    }
}
/// rekordbox writes paths from the root of the drive, with forward slashes.
fn exported(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    path.extend(relative.split('/').filter(|part| !part.is_empty()));
    path
}

/// What the last import of a track brought, to notice new exports and later edits.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRecord {
    pub analysis_stamp: Option<(u64, u64)>,
    pub imported_cues: Vec<f64>,
    pub imported_ends: Vec<f64>,
}
/// What the interface shows about a track's cues besides their positions.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CueInfo {
    pub hot: Vec<Option<CueStyle>>,
    pub memory: Vec<MemoryCue>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CueStyle {
    pub color: Option<[u8; 3]>,
    pub label: Option<String>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCue {
    pub start: f64,
    pub end: Option<f64>,
    pub color: Option<[u8; 3]>,
    pub label: Option<String>,
}
impl From<&Cue> for MemoryCue {
    fn from(cue: &Cue) -> Self {
        Self {
            start: cue.start,
            end: cue.end,
            color: cue.color,
            label: cue.label.clone(),
        }
    }
}
fn stamp(path: &Path) -> Option<(u64, u64)> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let seconds = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some((metadata.len(), seconds))
}
fn read_bounded(path: &Path) -> Option<Vec<u8>> {
    let size = fs::metadata(path).ok()?.len();
    (size <= MAX_ANALYSIS_BYTES)
        .then(|| fs::read(path).ok())
        .flatten()
}
/// Registers a rekordbox track and brings over its grid, key and cues, without touching the
/// drive. What the user changed in curado stays: a new export never overwrites it.
pub fn import_track(
    db: &Database,
    annotations: &mut Annotations,
    root: &Path,
    track: &DeviceTrack,
) -> Result<u64, String> {
    let path = resolve(root, &exported(root, &track.path))?;
    let artist = if track.artist.is_empty() {
        "Artista desconocido"
    } else {
        track.artist.as_str()
    };
    let id = db.register_reference(&path, &track.title, artist, track.duration as f64)?;
    let key = format!("device-track:{id}");
    let record: Option<DeviceRecord> = db.setting(&key);
    let analysis_path = inside(root, &exported(root, &track.analysis_path)).ok();
    let analysis_stamp = analysis_path.as_deref().and_then(stamp);
    if let Some(record) = &record {
        if record.analysis_stamp == analysis_stamp {
            return Ok(id);
        }
    }
    let mut record = record.unwrap_or_default();
    record.analysis_stamp = analysis_stamp;
    let dat = analysis_path.as_deref().and_then(read_bounded);
    let ext = analysis_path
        .as_deref()
        .and_then(|path| read_bounded(&path.with_extension("EXT")));
    let analysis = dat
        .as_deref()
        .map(|dat| rekordbox::parse_analysis(dat, ext.as_deref()))
        .unwrap_or_default();
    if let Some((bpm, beat_offset, downbeat)) = rekordbox::constant_grid(&analysis.beats) {
        let music = MusicAnalysis {
            version: ANALYSIS_VERSION,
            bpm: Some(bpm),
            beat_offset,
            bpm_confidence: 1.,
            key: track.key.as_deref().and_then(rekordbox::parse_key),
            key_confidence: 1.,
            manual: false,
            downbeat,
            first_sound: 0.,
            imported: true,
        };
        if music.valid() && !db.music(id).is_some_and(|saved| saved.manual) {
            db.save(&format!("music:{id}"), &music)?;
        }
    }
    let (saved_cues, saved_ends) = db.saved_cues(id);
    let unchanged = saved_cues.as_slice() == record.imported_cues.as_slice()
        && saved_ends.as_slice() == record.imported_ends.as_slice();
    let empty = saved_cues.iter().all(|cue| *cue == 0.);
    if empty || unchanged {
        let cues: [f64; HOT_CUES] =
            std::array::from_fn(|i| analysis.hot[i].as_ref().map_or(0., |cue| cue.start));
        let ends: [f64; HOT_CUES] = std::array::from_fn(|i| {
            analysis.hot[i]
                .as_ref()
                .and_then(|cue| cue.end)
                .unwrap_or(0.)
        });
        db.save(&format!("cues:{id}"), &cues)?;
        db.save(&format!("cue-ends:{id}"), &ends)?;
        annotations.replace(id, cues, ends);
        record.imported_cues = cues.to_vec();
        record.imported_ends = ends.to_vec();
        let info = CueInfo {
            hot: analysis
                .hot
                .iter()
                .map(|cue| {
                    cue.as_ref().map(|cue| CueStyle {
                        color: cue.color,
                        label: cue.label.clone(),
                    })
                })
                .collect(),
            memory: analysis.memory.iter().map(MemoryCue::from).collect(),
        };
        db.save(&format!("cue-info:{id}"), &info)?;
    }
    db.save(&key, &record)?;
    Ok(id)
}
/// Positions travel through JSON, so "the same" allows for less than a millisecond.
fn same_position(saved: &[f64], i: usize, now: f64) -> bool {
    (saved.get(i).copied().unwrap_or(0.) - now).abs() < 0.0005
}
/// Whether the cues are still the ones the drive brought.
pub fn unchanged_cues(
    record: &DeviceRecord,
    cues: &[f64; HOT_CUES],
    ends: &[f64; HOT_CUES],
) -> bool {
    (0..HOT_CUES).all(|i| {
        same_position(&record.imported_cues, i, cues[i])
            && same_position(&record.imported_ends, i, ends[i])
    })
}
/// A color and label describe the position rekordbox saved them for. Once curado moves or
/// replaces that cue they no longer belong to it.
pub fn current_cue_info(db: &Database, id: u64) -> CueInfo {
    let mut info: CueInfo = db.setting(&format!("cue-info:{id}")).unwrap_or_default();
    let record: DeviceRecord = db
        .setting(&format!("device-track:{id}"))
        .unwrap_or_default();
    let (cues, ends) = db.saved_cues(id);
    for (i, style) in info.hot.iter_mut().enumerate().take(HOT_CUES) {
        if !same_position(&record.imported_cues, i, cues[i])
            || !same_position(&record.imported_ends, i, ends[i])
        {
            *style = None;
        }
    }
    info
}
/// A file from a drive with no rekordbox data: curado analyzes it like any other.
pub fn register_file(db: &Database, path: &Path) -> Result<u64, String> {
    let title = path
        .file_stem()
        .map_or_else(String::new, |name| name.to_string_lossy().into());
    db.register_reference(path, &title, "Artista desconocido", 0.)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn temporary(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("curado-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path.canonicalize().unwrap()
    }
    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }
    fn device(root: &Path) -> Devices {
        let devices = Devices::default();
        let info = DeviceInfo {
            id: "USB".into(),
            name: "USB".into(),
            rekordbox: true,
        };
        *devices.known.lock().unwrap() = vec![(info, root.to_path_buf())];
        devices
    }

    #[test]
    fn exports_cannot_reach_outside_the_drive() {
        let root = temporary("escape");
        let secret = temporary("escape-secret");
        touch(&secret.join("secret.mp3"));
        touch(&root.join("Contents/ok.mp3"));
        touch(&root.join("Contents/notes.pdf"));
        symlink(secret.join("secret.mp3"), root.join("Contents/link.mp3")).unwrap();
        symlink(&secret, root.join("Contents/outside")).unwrap();
        let attempt = |relative: &str| resolve(&root, &exported(&root, relative));
        assert!(attempt("/Contents/ok.mp3").is_ok());
        assert!(attempt("Contents/ok.mp3").is_ok());
        assert!(attempt("/Contents/notes.pdf").is_err(), "not audio");
        assert!(attempt("/Contents/link.mp3").is_err(), "a link out");
        assert!(
            attempt("/Contents/outside/secret.mp3").is_err(),
            "through a link"
        );
        let traversal = format!(
            "/Contents/../../{}/secret.mp3",
            secret.file_name().unwrap().to_string_lossy()
        );
        assert!(attempt(&traversal).is_err(), "parent directories");
        assert!(attempt("/Contents/missing.mp3").is_err());
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(secret).unwrap();
    }

    #[test]
    fn browsing_shows_audio_and_the_folders_that_lead_to_it() {
        let root = temporary("browse");
        touch(&root.join("Contents/Artist/Album/one.mp3"));
        touch(&root.join("Mixes/two.flac"));
        touch(&root.join("loose.WAV"));
        touch(&root.join("Manual.pdf"));
        touch(&root.join("JUEGOS/game.exe"));
        touch(&root.join("PIONEER/rekordbox/export.pdb"));
        touch(&root.join("PIONEER/hidden.mp3"));
        touch(&root.join("._loose.wav"));
        touch(&root.join(".Trashes/deleted.mp3"));
        symlink("/", root.join("everything")).unwrap();
        let devices = device(&root);
        let top = devices.browse("USB", None).unwrap();
        let names = |list: &[FolderEntry]| list.iter().map(|e| e.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&top.folders), ["Contents", "Mixes"]);
        assert_eq!(names(&top.files), ["loose.WAV"]);
        assert_eq!(top.parent, None);
        // Only tokens cross to the interface, and they lead back here.
        let contents = devices.browse("USB", Some(top.folders[0].entry)).unwrap();
        assert_eq!(names(&contents.folders), ["Artist"]);
        let up = devices.browse("USB", contents.parent).unwrap();
        assert_eq!(names(&up.folders), ["Contents", "Mixes"]);
        assert_eq!(up.parent, None, "no way above the drive");
        let file = devices.file(top.files[0].entry).unwrap();
        assert!(file.ends_with("loose.WAV"));
        assert!(devices.file(999_999).is_err());
        assert!(devices.browse("Other", None).is_err());
        // A file that changes into a link after listing is rejected when used.
        let listed = top.files[0].entry;
        fs::remove_file(&file).unwrap();
        symlink("/etc/hosts", &file).unwrap();
        assert!(devices.file(listed).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    fn analysis_file(beats: &[(u16, u16, u32)], hot: &[(u32, u32)]) -> Vec<u8> {
        let mut file = b"PMAI".to_vec();
        file.extend_from_slice(&0x1c_u32.to_be_bytes());
        file.extend_from_slice(&[0; 20]);
        let mut grid = b"PQTZ".to_vec();
        grid.extend_from_slice(&0x18_u32.to_be_bytes());
        grid.extend_from_slice(&(0x18 + beats.len() as u32 * 8).to_be_bytes());
        grid.extend_from_slice(&[0; 8]);
        grid.extend_from_slice(&(beats.len() as u32).to_be_bytes());
        for (number, tempo, ms) in beats {
            grid.extend_from_slice(&number.to_be_bytes());
            grid.extend_from_slice(&tempo.to_be_bytes());
            grid.extend_from_slice(&ms.to_be_bytes());
        }
        file.extend(grid);
        let mut list = b"PCOB".to_vec();
        list.extend_from_slice(&0x18_u32.to_be_bytes());
        list.extend_from_slice(&(0x18 + hot.len() as u32 * 0x38).to_be_bytes());
        list.extend_from_slice(&1_u32.to_be_bytes());
        list.extend_from_slice(&[0; 2]);
        list.extend_from_slice(&(hot.len() as u16).to_be_bytes());
        list.extend_from_slice(&[0; 4]);
        for (index, ms) in hot {
            let mut entry = vec![0_u8; 0x38];
            entry[..4].copy_from_slice(b"PCPT");
            entry[8..12].copy_from_slice(&0x38_u32.to_be_bytes());
            entry[0x0c..0x10].copy_from_slice(&index.to_be_bytes());
            entry[0x1c] = 1;
            entry[0x20..0x24].copy_from_slice(&ms.to_be_bytes());
            list.extend(entry);
        }
        file.extend(list);
        file
    }
    fn exported_track(id: u32) -> DeviceTrack {
        DeviceTrack {
            id,
            title: "Pista".into(),
            artist: String::new(),
            album: String::new(),
            genre: String::new(),
            key: Some("8A".into()),
            bpm: 124.,
            duration: 200,
            rating: 0,
            path: "/Contents/one.mp3".into(),
            analysis_path: "/PIONEER/USBANLZ/P0/A/ANLZ0000.DAT".into(),
        }
    }
    fn write_analysis(root: &Path, hot: &[(u32, u32)]) {
        let path = root.join("PIONEER/USBANLZ/P0/A/ANLZ0000.DAT");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            analysis_file(&[(3, 12400, 250), (4, 12400, 734), (1, 12400, 1218)], hot),
        )
        .unwrap();
        // Coarse file times must not hide a new export from this test.
        let later = SystemTime::now() + std::time::Duration::from_secs(hot.len() as u64 * 10);
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(later)
            .unwrap();
    }

    #[test]
    fn importing_brings_grid_key_and_cues_and_keeps_later_edits() {
        let root = temporary("import");
        touch(&root.join("Contents/one.mp3"));
        write_analysis(&root, &[(1, 1218), (3, 5000)]);
        let db = Database::open(Path::new(":memory:")).unwrap();
        let mut annotations = Annotations::default();
        let track = exported_track(9);
        let id = import_track(&db, &mut annotations, &root, &track).unwrap();
        let music = db.music(id).unwrap();
        assert_eq!(music.bpm, Some(124.));
        assert_eq!(music.beat_offset, 0.25);
        assert_eq!(music.downbeat, 1.218);
        assert!(music.imported && !music.manual && music.is_current());
        assert_eq!(music.key.unwrap().tonic, 9);
        let (cues, _) = db.saved_cues(id);
        assert_eq!((cues[0], cues[1], cues[2]), (1.218, 0., 5.));
        let listed = db.library().unwrap();
        assert!(listed.tracks[0].device, "kept out of the local library");
        assert_eq!(listed.tracks[0].artist, "Artista desconocido");
        // Loading again finds the same track and repeats nothing.
        assert_eq!(
            import_track(&db, &mut annotations, &root, &track).unwrap(),
            id
        );
        // A new export with new cues updates a track that was not edited here.
        write_analysis(&root, &[(1, 2000), (2, 3000), (4, 4000)]);
        import_track(&db, &mut annotations, &root, &track).unwrap();
        assert_eq!(db.saved_cues(id).0[0], 2.);
        // Once edited here, the user's cue and grid win over the next export.
        let state = annotations.state(id, db.saved_cues(id).0, db.saved_cues(id).1);
        state.set(0, 7.5);
        annotations.flush(&db).unwrap();
        let mut corrected = db.music(id).unwrap();
        corrected.manual = true;
        corrected.bpm = Some(62.);
        db.save(&format!("music:{id}"), &corrected).unwrap();
        write_analysis(&root, &[(1, 111), (2, 222), (3, 333), (4, 444)]);
        import_track(&db, &mut annotations, &root, &track).unwrap();
        assert_eq!(db.saved_cues(id).0[0], 7.5, "the edited cue stays");
        assert_eq!(
            db.music(id).unwrap().bpm,
            Some(62.),
            "the corrected grid stays"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_damaged_analysis_still_lets_the_track_load() {
        let root = temporary("damaged");
        touch(&root.join("Contents/one.mp3"));
        let path = root.join("PIONEER/USBANLZ/P0/A/ANLZ0000.DAT");
        touch(&path);
        let db = Database::open(Path::new(":memory:")).unwrap();
        let mut annotations = Annotations::default();
        let id = import_track(&db, &mut annotations, &root, &exported_track(1)).unwrap();
        assert_eq!(db.music(id), None, "curado will analyze it when it loads");
        fs::remove_file(&path).unwrap();
        import_track(&db, &mut annotations, &root, &exported_track(1)).unwrap();
        let mut escaping = exported_track(2);
        escaping.path = "/../outside.mp3".into();
        assert!(import_track(&db, &mut annotations, &root, &escaping).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plain_files_register_and_stay_out_of_the_library_until_imported() {
        let root = temporary("plain");
        touch(&root.join("loose.wav"));
        let db = Database::open(Path::new(":memory:")).unwrap();
        let id = register_file(&db, &root.join("loose.wav")).unwrap();
        assert_eq!(register_file(&db, &root.join("loose.wav")).unwrap(), id);
        let listed = db.library().unwrap();
        assert_eq!(listed.tracks[0].title, "loose");
        assert!(listed.tracks[0].device);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_track_already_in_the_library_stays_there() {
        let root = temporary("library");
        touch(&root.join("Contents/one.mp3"));
        write_analysis(&root, &[(1, 1218)]);
        let db = Database::open(Path::new(":memory:")).unwrap();
        let clip = curado_audio::decode::Clip {
            loudness: None,
            id: 0,
            cues: Default::default(),
            samples: vec![[0.; 2]; 48000],
            sample_rate: 48000,
            waveform: vec![],
            title: Some("Mía".into()),
            artist: None,
            music: Default::default(),
        };
        db.insert(&root.join("Contents/one.mp3"), &clip).unwrap();
        let id = db.library().unwrap().tracks[0].id;
        db.save(&format!("cues:{id}"), &[33.5_f64, 0., 0.]).unwrap();
        let mut annotations = Annotations::default();
        let loaded = import_track(&db, &mut annotations, &root, &exported_track(1)).unwrap();
        assert_eq!(loaded, id, "the same track, not a copy");
        let listed = db.library().unwrap();
        assert_eq!(listed.tracks.len(), 1);
        assert!(!listed.tracks[0].device, "it is not hidden");
        assert_eq!(listed.tracks[0].title, "Mía");
        assert_eq!(db.saved_cues(id).0[0], 33.5, "and its cues are kept");
        // The other way: importing a file played from a drive makes it part of the library.
        let other = root.join("Contents/two.mp3");
        touch(&other);
        register_file(&db, &other).unwrap();
        assert!(db.library().unwrap().tracks.iter().any(|t| t.device));
        db.insert(&other, &clip).unwrap();
        assert!(db.library().unwrap().tracks.iter().all(|t| !t.device));
        fs::remove_dir_all(root).unwrap();
    }

    /// A person's hot cues are the only independent evidence of where bars start: they mostly
    /// sit on a bar's first beat. Compares curado's analysis with them, on tracks that have them.
    #[test]
    #[ignore = "needs a real rekordbox USB; decodes every track with cues"]
    fn real_bars_against_hot_cues() {
        let root = PathBuf::from(std::env::var("CURADO_TEST_DEVICE").unwrap());
        let data = fs::read(root.join(EXPORT_FILE)).unwrap();
        let export = rekordbox::parse_export(&data).unwrap();
        let (mut tracks, mut cues, mut reference_bar) = (0, 0, 0);
        let (mut ours_on_beat, mut ours_on_bar, mut same_tempo) = (0, 0, 0);
        let mut phase = Vec::new();
        for track in &export.tracks {
            let path = exported(&root, &track.analysis_path);
            let Ok(dat) = fs::read(&path) else { continue };
            let ext = fs::read(path.with_extension("EXT")).ok();
            let analysis = rekordbox::parse_analysis(&dat, ext.as_deref());
            let hot: Vec<f64> = analysis.hot.iter().flatten().map(|c| c.start).collect();
            let Some((_, reference_offset, _)) = rekordbox::constant_grid(&analysis.beats) else {
                continue;
            };
            if hot.is_empty() {
                continue;
            }
            let Ok(clip) = curado_audio::decode::decode_with_limit(
                &resolve(&root, &exported(&root, &track.path)).unwrap(),
                1,
                None,
                curado_audio::decode::MAX_CLIP_BYTES,
            ) else {
                continue;
            };
            tracks += 1;
            let Some(bpm) = clip.music.bpm else { continue };
            if (bpm - track.bpm).abs() > 0.1 {
                continue;
            }
            same_tempo += 1;
            let period = 60. / bpm;
            let error = (clip.music.beat_offset - reference_offset).rem_euclid(period);
            phase.push(error.min(period - error) * 1000.);
            for cue in hot {
                let beat = analysis
                    .beats
                    .iter()
                    .min_by_key(|b| (b.2 as i64 - (cue * 1000.) as i64).abs())
                    .unwrap();
                if (beat.2 as f64 / 1000. - cue).abs() > 0.025 {
                    continue;
                }
                cues += 1;
                reference_bar += (beat.0 == 1) as usize;
                let from_grid = (cue - clip.music.beat_offset) / period;
                if (from_grid - from_grid.round()).abs() * period < 0.03 {
                    ours_on_beat += 1;
                    let beats = ((cue - clip.music.downbeat) / period).round() as i64;
                    ours_on_bar += (beats.rem_euclid(4) == 0) as usize;
                }
            }
        }
        phase.sort_by(f64::total_cmp);
        println!(
            "{tracks} tracks with hot cues, {same_tempo} with our tempo within 0.1; {cues} cues on rekordbox's grid"
        );
        println!(
            "cues on rekordbox's beat 1: {reference_bar}; on our grid {ours_on_beat}; on our beat 1 {ours_on_bar}"
        );
        if !phase.is_empty() {
            println!(
                "our grid to rekordbox's, median {:.1} ms, under 5 ms {}, under 20 ms {} of {}",
                phase[phase.len() / 2],
                phase.iter().filter(|p| **p < 5.).count(),
                phase.iter().filter(|p| **p < 20.).count(),
                phase.len()
            );
        }
    }

    /// `CURADO_TEST_DEVICE=/Volumes/USB cargo test -p curado real_import -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a real rekordbox USB"]
    fn real_import() {
        let root = PathBuf::from(std::env::var("CURADO_TEST_DEVICE").unwrap());
        let devices = Devices::default();
        let name = root.file_name().unwrap().to_string_lossy().to_string();
        let info = DeviceInfo {
            id: name.clone(),
            name: name.clone(),
            rekordbox: true,
        };
        *devices.known.lock().unwrap() = vec![(info, root.clone())];
        let export = devices.export(&name).unwrap();
        let db = Database::open(Path::new(":memory:")).unwrap();
        let mut annotations = Annotations::default();
        let started = std::time::Instant::now();
        let (mut grids, mut keys, mut cues) = (0, 0, 0);
        for track in &export.tracks {
            let id = import_track(&db, &mut annotations, &root, track).unwrap();
            let music = db.music(id);
            grids += music.is_some() as usize;
            keys += music.and_then(|m| m.key).is_some() as usize;
            cues += db.saved_cues(id).0.iter().filter(|c| **c > 0.).count();
        }
        println!(
            "imported {} tracks in {:?}: {grids} grids, {keys} keys, {cues} hot cues",
            export.tracks.len(),
            started.elapsed()
        );
        let top = devices.browse(&name, None).unwrap();
        println!(
            "top level: {} folders with audio, {} files",
            top.folders.len(),
            top.files.len()
        );
        assert!(top.folders.iter().all(|f| f.name != "PIONEER"));
    }
}
