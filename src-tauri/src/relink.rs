//! Finding files again. A drive that is not connected is not lost, and a moved file is only
//! taken back when it can be shown to be the same one: nothing is relinked on a guess.
use crate::{
    devices,
    identity::{self, Identity},
    library::{supported, Database},
};
use rusqlite::params;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

/// What the library's files look like right now.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    /// Its drive is connected or it was on this computer, and the file is not there.
    pub missing: Vec<u64>,
    /// On a drive that is not connected: nothing to fix, only to plug in.
    pub offline: Vec<u64>,
    /// Entries with the same content as an earlier one.
    pub duplicates: Vec<u64>,
    /// Found by itself, on the same drive under another mount.
    pub healed: usize,
}
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchReport {
    pub found: usize,
    /// More than one copy, or nothing to compare with: left for you to choose.
    pub unsure: usize,
    pub not_found: usize,
    /// The search ran out of time before looking everywhere.
    pub partial: bool,
}
struct Entry {
    id: u64,
    path: PathBuf,
    kind: String,
    parent: Option<u64>,
    size: Option<u64>,
    fingerprint: Option<String>,
    volume: Option<String>,
    relative: Option<String>,
}
fn entries(db: &Database) -> Result<Vec<Entry>, String> {
    let mut stmt = db
        .connection
        .prepare(
            "SELECT id,path,kind,parent,file_size,fingerprint,volume,relative FROM tracks
        WHERE NOT EXISTS(SELECT 1 FROM settings WHERE key='device-only:'||tracks.id)",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Entry {
                id: r.get(0)?,
                path: PathBuf::from(r.get::<_, String>(1)?),
                kind: r.get(2)?,
                parent: r.get(3)?,
                size: r.get(4)?,
                fingerprint: r.get(5)?,
                volume: r.get(6)?,
                relative: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());
    rows
}
fn duplicates(db: &Database) -> Result<Vec<u64>, String> {
    let mut stmt = db.connection.prepare(
        "SELECT id FROM tracks a WHERE kind='track' AND fingerprint IS NOT NULL
        AND NOT EXISTS(SELECT 1 FROM settings WHERE key='device-only:'||a.id)
        AND EXISTS(SELECT 1 FROM tracks b WHERE b.fingerprint=a.fingerprint AND b.file_size=a.file_size AND b.kind='track' AND b.id<a.id
            AND NOT EXISTS(SELECT 1 FROM settings WHERE key='device-only:'||b.id))",
    ).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| r.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<u64>, _>>()
        .map_err(|e| e.to_string());
    rows
}
/// `mounted` maps a drive's id to where it is now. Touches the disk only outside the lock.
pub fn health(db: &Mutex<Database>, mounted: &HashMap<String, PathBuf>) -> Result<Health, String> {
    let list = entries(&db.lock().unwrap())?;
    let mut report = Health::default();
    let connected: HashSet<&Path> = mounted.values().map(PathBuf::as_path).collect();
    // A loop's file is its parent's: once is enough, and healing the parent moves the loop.
    let mut healed: HashMap<u64, PathBuf> = HashMap::new();
    for entry in list
        .iter()
        .filter(|entry| entry.kind != "loop" || entry.parent.is_none())
    {
        if entry.path.exists() {
            continue;
        }
        if let Some(candidate) = same_drive(entry, mounted) {
            if let Ok(found) = identity::identify(&candidate) {
                if entry
                    .fingerprint
                    .as_deref()
                    .is_none_or(|known| known == found.fingerprint)
                    && db
                        .lock()
                        .unwrap()
                        .relink(entry.id, &candidate, &found)
                        .is_ok()
                {
                    report.healed += 1;
                    healed.insert(entry.id, candidate);
                }
            }
        }
    }
    for entry in &list {
        let through = entry
            .parent
            .filter(|_| entry.kind == "loop")
            .unwrap_or(entry.id);
        if healed.contains_key(&through) || entry.path.exists() {
            continue;
        }
        let drive_gone = match (&entry.volume, identity::volume_root(&entry.path)) {
            (Some(volume), _) => !mounted.contains_key(volume),
            (None, Some(root)) => !connected.contains(root.as_path()) && !root.exists(),
            (None, None) => false,
        };
        if drive_gone {
            report.offline.push(entry.id);
        } else {
            report.missing.push(entry.id);
        }
    }
    report.duplicates = duplicates(&db.lock().unwrap())?;
    Ok(report)
}
fn same_drive(entry: &Entry, mounted: &HashMap<String, PathBuf>) -> Option<PathBuf> {
    let root = mounted.get(entry.volume.as_ref()?)?;
    let candidate = root.join(entry.relative.as_ref()?);
    (fs::metadata(&candidate).ok()?.len() == entry.size?).then_some(candidate)
}
/// Connected drives by their own id.
pub fn mounted_drives() -> HashMap<String, PathBuf> {
    identity::forget_volumes();
    devices::mount_points()
        .into_iter()
        .filter_map(|root| Some((identity::volume_id(&root)?, root)))
        .collect()
}
const MAX_ENTRIES: usize = 400_000;
const MAX_TIME: Duration = Duration::from_secs(60);
/// Files with these names under the roots. Links are not followed, so a loop on a drive cannot
/// trap it, and the limits keep a slow drive from holding it up.
fn files_named(
    names: &HashSet<std::ffi::OsString>,
    roots: &[PathBuf],
    max_entries: usize,
    max_time: Duration,
) -> (HashMap<std::ffi::OsString, Vec<PathBuf>>, bool) {
    let mut found: HashMap<_, Vec<PathBuf>> = HashMap::new();
    let started = Instant::now();
    let mut pending: Vec<PathBuf> = roots.to_vec();
    let mut seen = HashSet::new();
    let mut entries = 0;
    while let Some(folder) = pending.pop() {
        if !seen.insert(folder.clone()) {
            continue;
        }
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
                found.entry(name).or_default().push(entry.path());
            }
        }
    }
    (found, true)
}
/// Where to look: music folders, drives and the folders the library's other files live in.
pub fn search_roots(db: &Mutex<Database>) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = devices::mount_points();
    if let Some(home) = std::env::var_os("HOME") {
        for folder in ["Music", "Downloads", "Desktop", "Documents"] {
            roots.push(Path::new(&home).join(folder));
        }
    }
    if let Ok(list) = entries(&db.lock().unwrap()) {
        let mut near: HashSet<PathBuf> = HashSet::new();
        for entry in &list {
            if entry.path.exists() {
                near.extend(entry.path.parent().map(Path::to_path_buf));
            }
        }
        roots.extend(near);
    }
    roots
}
/// Takes back a file only with one candidate, and only if it is shown to be the same. What was
/// never fingerprinted has nothing to compare, so it is left for you to choose by hand.
pub fn search(db: &Mutex<Database>, roots: &[PathBuf]) -> Result<SearchReport, String> {
    search_within(db, roots, MAX_ENTRIES, MAX_TIME)
}
fn search_within(
    db: &Mutex<Database>,
    roots: &[PathBuf],
    max_entries: usize,
    max_time: Duration,
) -> Result<SearchReport, String> {
    let lost: Vec<Entry> = entries(&db.lock().unwrap())?
        .into_iter()
        .filter(|entry| (entry.kind != "loop" || entry.parent.is_none()) && !entry.path.exists())
        .collect();
    let mut report = SearchReport::default();
    if lost.is_empty() {
        return Ok(report);
    }
    let names: HashSet<_> = lost
        .iter()
        .filter_map(|entry| entry.path.file_name().map(ToOwned::to_owned))
        .collect();
    let (found, complete) = files_named(&names, roots, max_entries, max_time);
    report.partial = !complete;
    for entry in lost {
        let candidates = entry.path.file_name().and_then(|name| found.get(name));
        let Some(candidates) = candidates else {
            report.not_found += 1;
            continue;
        };
        let Some(known) = entry.fingerprint.as_deref() else {
            report.unsure += 1;
            continue;
        };
        let same: Vec<(PathBuf, Identity)> = candidates
            .iter()
            .filter(|path| fs::metadata(path).is_ok_and(|file| Some(file.len()) == entry.size))
            .filter_map(|path| Some((path.clone(), identity::identify(path).ok()?)))
            .filter(|(_, found)| found.fingerprint == known)
            .collect();
        match same.as_slice() {
            [(path, found)] if db.lock().unwrap().relink(entry.id, path, found).is_ok() => {
                report.found += 1;
            }
            [] => report.not_found += 1,
            _ => report.unsure += 1,
        }
    }
    Ok(report)
}
/// Fingerprints what has none yet, a little at a time, so that a moved file can be recognized
/// later. Files that cannot be read now are tried again next launch.
pub fn backfill_identities(db: &Mutex<Database>, keep_going: impl Fn() -> bool) {
    let mut skipped: HashSet<u64> = HashSet::new();
    while keep_going() {
        let next: Vec<(u64, PathBuf)> = {
            let db = db.lock().unwrap();
            let Ok(mut stmt) = db.connection.prepare(
                "SELECT id,path FROM tracks WHERE fingerprint IS NULL AND kind<>'loop'
                AND NOT EXISTS(SELECT 1 FROM settings WHERE key='device-only:'||tracks.id) ORDER BY id",
            ) else {
                return;
            };
            let Ok(rows) = stmt.query_map([], |r| {
                Ok((r.get::<_, u64>(0)?, PathBuf::from(r.get::<_, String>(1)?)))
            }) else {
                return;
            };
            rows.flatten()
                .filter(|(id, _)| !skipped.contains(id))
                .take(16)
                .collect()
        };
        if next.is_empty() {
            return;
        }
        for (id, path) in next {
            if !keep_going() {
                return;
            }
            match identity::identify(&path) {
                Ok(found) => {
                    let db = db.lock().unwrap();
                    let _ = db.set_identity(id, &found);
                    let _ = db.connection.execute(
                        "UPDATE tracks SET file_size=?,fingerprint=?,volume=?,relative=? WHERE parent=? AND kind='loop'",
                        params![found.size, found.fingerprint, found.volume, found.relative, id],
                    );
                }
                Err(_) => {
                    skipped.insert(id);
                }
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}
/// Whether a path can be an entry's file at all.
pub fn usable(path: &Path) -> bool {
    path.is_file() && supported(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Database;
    fn folder(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("curado-relink-{name}"));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path.canonicalize().unwrap()
    }
    fn write(path: &Path, seed: u8) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![seed; 4096]).unwrap();
    }
    fn database() -> Mutex<Database> {
        Mutex::new(Database::open(Path::new(":memory:")).unwrap())
    }
    fn add(db: &Mutex<Database>, id: u64, path: &Path, known: bool) {
        let db = db.lock().unwrap();
        db.connection.execute("INSERT INTO tracks(id,path,title,artist,duration,sample_rate,peaks) VALUES(?,?,'A','B',100,48000,'[]')", params![id, path.to_string_lossy()]).unwrap();
        if known {
            db.set_identity(id, &identity::identify(path).unwrap())
                .unwrap();
        }
    }
    #[test]
    fn a_moved_file_comes_back_with_its_entry_and_its_loops() {
        let (old, new) = (folder("old"), folder("new"));
        let (before, after) = (old.join("a.wav"), new.join("deep/a.wav"));
        write(&before, 1);
        let db = database();
        add(&db, 1, &before, true);
        let cut = db.lock().unwrap().save_loop(1, 1., 2.).unwrap();
        fs::rename(&before, {
            fs::create_dir_all(after.parent().unwrap()).unwrap();
            &after
        })
        .unwrap();
        let drives = HashMap::new();
        let health_now = health(&db, &drives).unwrap();
        assert_eq!(health_now.missing, [1, cut]);
        let report = search(&db, std::slice::from_ref(&new)).unwrap();
        assert_eq!((report.found, report.unsure, report.not_found), (1, 0, 0));
        assert_eq!(db.lock().unwrap().path(1).unwrap(), after);
        assert_eq!(db.lock().unwrap().path(cut).unwrap(), after);
        assert!(health(&db, &drives).unwrap().missing.is_empty());
    }
    #[test]
    fn a_same_named_file_with_other_content_is_not_taken() {
        let (old, new) = (folder("wrong-old"), folder("wrong-new"));
        let before = old.join("a.wav");
        write(&before, 1);
        let db = database();
        add(&db, 1, &before, true);
        fs::remove_file(&before).unwrap();
        write(&new.join("a.wav"), 2);
        let report = search(&db, &[new]).unwrap();
        assert_eq!((report.found, report.not_found), (0, 1));
        assert_eq!(db.lock().unwrap().path(1).unwrap(), before);
    }
    #[test]
    fn two_identical_copies_are_left_for_you_to_choose() {
        let (old, new) = (folder("two-old"), folder("two-new"));
        let before = old.join("a.wav");
        write(&before, 1);
        let db = database();
        add(&db, 1, &before, true);
        fs::remove_file(&before).unwrap();
        write(&new.join("x/a.wav"), 1);
        write(&new.join("y/a.wav"), 1);
        let report = search(&db, &[new]).unwrap();
        assert_eq!((report.found, report.unsure), (0, 1));
    }
    #[test]
    fn without_a_fingerprint_a_name_is_not_enough() {
        let (old, new) = (folder("bare-old"), folder("bare-new"));
        let before = old.join("a.wav");
        let db = database();
        write(&before, 1);
        add(&db, 1, &before, false);
        fs::remove_file(&before).unwrap();
        write(&new.join("a.wav"), 1);
        let report = search(&db, &[new]).unwrap();
        assert_eq!((report.found, report.unsure), (0, 1));
    }
    #[test]
    fn the_same_drive_under_another_mount_heals_by_itself() {
        let (old, mount) = (folder("mount-old"), folder("mount-new"));
        let before = old.join("music/a.wav");
        write(&before, 3);
        let db = database();
        add(&db, 1, &before, true);
        {
            let db = db.lock().unwrap();
            db.connection
                .execute(
                    "UPDATE tracks SET volume='DRIVE',relative='music/a.wav' WHERE id=1",
                    [],
                )
                .unwrap();
        }
        fs::remove_file(&before).unwrap();
        // Not connected: offline, not lost.
        let away = health(&db, &HashMap::new()).unwrap();
        assert_eq!((away.offline.as_slice(), away.missing.len()), (&[1][..], 0));
        let after = mount.join("music/a.wav");
        write(&after, 3);
        let drives = HashMap::from([("DRIVE".to_string(), mount)]);
        let back = health(&db, &drives).unwrap();
        assert_eq!(back.healed, 1);
        assert!(back.missing.is_empty() && back.offline.is_empty());
        assert_eq!(db.lock().unwrap().path(1).unwrap(), after);
    }
    #[test]
    fn duplicates_name_the_later_copy_and_identities_fill_in() {
        let root = folder("dup");
        let (a, b, c) = (root.join("a.wav"), root.join("b.wav"), root.join("c.wav"));
        write(&a, 1);
        write(&b, 1);
        write(&c, 2);
        let db = database();
        add(&db, 1, &a, false);
        add(&db, 2, &b, false);
        add(&db, 3, &c, false);
        add(&db, 4, &root.join("gone.wav"), false);
        backfill_identities(&db, || true);
        assert_eq!(health(&db, &HashMap::new()).unwrap().duplicates, [2]);
        let bare: u32 = db
            .lock()
            .unwrap()
            .connection
            .query_row(
                "SELECT COUNT(*) FROM tracks WHERE fingerprint IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(bare, 1, "what cannot be read waits");
        // Stops when told to.
        let db2 = database();
        add(&db2, 1, &a, false);
        backfill_identities(&db2, || false);
        let bare: u32 = db2
            .lock()
            .unwrap()
            .connection
            .query_row(
                "SELECT COUNT(*) FROM tracks WHERE fingerprint IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(bare, 1);
    }
    #[test]
    fn a_search_that_runs_out_says_so() {
        let (old, new) = (folder("limit-old"), folder("limit-new"));
        let before = old.join("a.wav");
        write(&before, 1);
        let db = database();
        add(&db, 1, &before, true);
        fs::remove_file(&before).unwrap();
        for i in 0..20 {
            write(&new.join(format!("{i}.wav")), 9);
        }
        let report = search_within(&db, &[new], 3, MAX_TIME).unwrap();
        assert!(report.partial);
    }
}
