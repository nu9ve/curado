use crate::identity::Identity;
use curado_audio::{analysis, annotations::HOT_CUES, decode::Clip};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub path: String,
    pub duration: f64,
    pub sample_rate: u32,
    pub favorite: bool,
    pub music: Option<curado_audio::music::MusicAnalysis>,
    /// Played from a connected drive: kept for cues and analysis, not listed in the library.
    pub device: bool,
    /// `track`, `loop` or `stem`. A sample is any piece with the tag, not another kind.
    pub kind: String,
    pub tags: Vec<String>,
    /// The track a loop or a stem came from.
    pub parent: Option<u64>,
    pub stem: Option<String>,
}
#[derive(Serialize)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub tracks: Vec<u64>,
}
#[derive(Serialize)]
pub struct Library {
    pub tracks: Vec<Track>,
    pub playlists: Vec<Playlist>,
    pub collections: Vec<Collection>,
    /// Eight pads each, which may be empty.
    pub banks: Vec<Vec<Option<u64>>>,
}
/// A saved search: it holds no tracks, so it cannot go out of date or repeat them.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CollectionFilter {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub max_duration: Option<f64>,
    #[serde(default)]
    pub favorite: bool,
}
#[derive(Serialize)]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub filter: CollectionFilter,
}
/// Four banks are what fit a thumb's reach; more would be another list to browse.
pub const SAMPLE_BANKS: usize = 4;
pub const SAMPLE_PADS: usize = 8;
const MAX_TAG_BYTES: usize = 40;
#[derive(Serialize, Default)]
pub struct ImportReport {
    pub added: usize,
    pub skipped: usize,
    /// Already here under another path, so not listed twice.
    pub duplicates: usize,
    /// Pointed an entry whose file had gone missing at this copy.
    pub relinked: usize,
    pub errors: Vec<String>,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LibraryEdit {
    CreatePlaylist {
        name: String,
    },
    RenamePlaylist {
        id: i64,
        name: String,
    },
    DeletePlaylist {
        id: i64,
    },
    AddToPlaylist {
        playlist: i64,
        track: u64,
    },
    RemoveFromPlaylist {
        playlist: i64,
        track: u64,
    },
    Favorite {
        track: u64,
        value: bool,
    },
    RemoveTrack {
        track: u64,
    },
    AddTag {
        track: u64,
        tag: String,
    },
    RemoveTag {
        track: u64,
        tag: String,
    },
    SaveCollection {
        name: String,
        filter: CollectionFilter,
    },
    DeleteCollection {
        id: i64,
    },
    SaveBank {
        index: usize,
        tracks: Vec<Option<u64>>,
    },
}
const SCHEMA: u32 = 4;
const TRACK_COLUMNS: &str = "id INTEGER PRIMARY KEY AUTOINCREMENT, path TEXT NOT NULL, title TEXT NOT NULL, artist TEXT NOT NULL, duration REAL NOT NULL, sample_rate INTEGER NOT NULL, peaks TEXT NOT NULL, favorite INTEGER NOT NULL DEFAULT 0, waveform BLOB, kind TEXT NOT NULL DEFAULT 'track' CHECK(kind IN ('track','loop','stem')), parent INTEGER REFERENCES tracks(id) ON DELETE SET NULL, clip_start REAL, clip_end REAL, stem TEXT, stem_model TEXT, owned INTEGER NOT NULL DEFAULT 0, used_at INTEGER NOT NULL DEFAULT 0, file_size INTEGER, fingerprint TEXT, volume TEXT, relative TEXT";
pub struct PieceInfo {
    pub title: String,
    pub artist: String,
    pub duration: f64,
    pub kind: String,
    pub fingerprint: Option<String>,
}
pub struct Database {
    pub connection: Connection,
}
impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if version > SCHEMA {
            return Err(
                "La biblioteca pertenece a una versión más reciente de curado. No se modificó."
                    .into(),
            );
        }
        connection
            .execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| e.to_string())?;
        if version == 1 {
            // Stable identities: deleting the last row must never attach its cues to a new file.
            connection
                .execute_batch("PRAGMA foreign_keys=OFF;")
                .map_err(|e| e.to_string())?;
            let tx = connection
                .unchecked_transaction()
                .map_err(|e| e.to_string())?;
            tx.execute_batch("CREATE TABLE tracks_v2(id INTEGER PRIMARY KEY AUTOINCREMENT,path TEXT NOT NULL UNIQUE,title TEXT NOT NULL,artist TEXT NOT NULL,duration REAL NOT NULL,sample_rate INTEGER NOT NULL,peaks TEXT NOT NULL,favorite INTEGER NOT NULL DEFAULT 0);
            INSERT INTO tracks_v2 SELECT * FROM tracks;
            DROP TABLE tracks;
            ALTER TABLE tracks_v2 RENAME TO tracks;
            PRAGMA user_version=2;").map_err(|e|e.to_string())?;
            let broken: u64 = tx
                .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                    r.get(0)
                })
                .map_err(|e| e.to_string())?;
            if broken != 0 {
                return Err(
                    "La biblioteca tiene relaciones inválidas; la migración se revirtió".into(),
                );
            }
            tx.commit().map_err(|e| e.to_string())?;
        }
        if version == 1 || version == 2 {
            // v3: band waveform blob. Legacy rows keep NULL until their next decode backfills it.
            let tx = connection
                .unchecked_transaction()
                .map_err(|e| e.to_string())?;
            tx.execute_batch("ALTER TABLE tracks ADD COLUMN waveform BLOB; PRAGMA user_version=3;")
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        if (1..SCHEMA).contains(&version) {
            // v4: pieces. A loop shares its parent's file, so a path alone no longer means one
            // row: uniqueness moves to indexes that say what each kind may not repeat.
            connection
                .execute_batch("PRAGMA foreign_keys=OFF;")
                .map_err(|e| e.to_string())?;
            let tx = connection
                .unchecked_transaction()
                .map_err(|e| e.to_string())?;
            tx.execute_batch(&format!(
                "CREATE TABLE tracks_v4({TRACK_COLUMNS});
                INSERT INTO tracks_v4(id,path,title,artist,duration,sample_rate,peaks,favorite,waveform)
                    SELECT id,path,title,artist,duration,sample_rate,peaks,favorite,waveform FROM tracks;
                DELETE FROM sqlite_sequence WHERE name='tracks_v4';
                INSERT INTO sqlite_sequence(name,seq) SELECT 'tracks_v4',seq FROM sqlite_sequence WHERE name='tracks';
                DROP TABLE tracks;
                ALTER TABLE tracks_v4 RENAME TO tracks;"
            ))
            .map_err(|e| e.to_string())?;
            let broken: u64 = tx
                .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                    r.get(0)
                })
                .map_err(|e| e.to_string())?;
            if broken != 0 {
                return Err(
                    "La biblioteca tiene relaciones inválidas; la migración se revirtió".into(),
                );
            }
            tx.commit().map_err(|e| e.to_string())?;
        }
        connection.execute_batch(&format!("PRAGMA foreign_keys=ON; BEGIN IMMEDIATE;
        CREATE TABLE IF NOT EXISTS tracks ({TRACK_COLUMNS});
        CREATE UNIQUE INDEX IF NOT EXISTS tracks_file ON tracks(path) WHERE kind<>'loop';
        CREATE UNIQUE INDEX IF NOT EXISTS tracks_loop ON tracks(parent,clip_start,clip_end) WHERE kind='loop';
        CREATE UNIQUE INDEX IF NOT EXISTS tracks_stem ON tracks(parent,stem_model,stem) WHERE kind='stem';
        CREATE INDEX IF NOT EXISTS tracks_identity ON tracks(fingerprint) WHERE fingerprint IS NOT NULL;
        CREATE TABLE IF NOT EXISTS tags (track INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE, tag TEXT NOT NULL, PRIMARY KEY(track,tag)) WITHOUT ROWID;
        CREATE INDEX IF NOT EXISTS tags_by_name ON tags(tag);
        CREATE TABLE IF NOT EXISTS collections (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, filter TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS playlists (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS playlist_tracks (playlist INTEGER REFERENCES playlists(id) ON DELETE CASCADE, track INTEGER REFERENCES tracks(id) ON DELETE CASCADE, position INTEGER NOT NULL, PRIMARY KEY(playlist, track));
        CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL); PRAGMA user_version={SCHEMA}; COMMIT;")).map_err(|e| e.to_string())?;
        Ok(Self { connection })
    }
    pub fn library(&self) -> Result<Library, String> {
        // Waveforms are fetched per track on demand; the listing stays small for large libraries.
        let mut stmt = self.connection.prepare("SELECT id,path,title,artist,duration,sample_rate,favorite,(SELECT value FROM settings WHERE key='music:'||tracks.id),EXISTS(SELECT 1 FROM settings WHERE key='device-only:'||tracks.id),kind,parent,stem FROM tracks ORDER BY title COLLATE NOCASE").map_err(|e| e.to_string())?;
        let mut tracks = stmt
            .query_map([], |r| {
                Ok(Track {
                    id: r.get(0)?,
                    path: r.get(1)?,
                    title: r.get(2)?,
                    artist: r.get(3)?,
                    duration: r.get(4)?,
                    sample_rate: r.get(5)?,
                    favorite: r.get(6)?,
                    music: r
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| {
                            serde_json::from_str::<curado_audio::music::MusicAnalysis>(&s).ok()
                        })
                        .filter(|m| m.valid()),
                    device: r.get(8)?,
                    kind: r.get(9)?,
                    tags: Vec::new(),
                    parent: r.get(10)?,
                    stem: r.get(11)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let mut stmt = self
            .connection
            .prepare("SELECT id,name FROM playlists ORDER BY id")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        let mut playlists = Vec::new();
        for row in rows {
            let (id, name) = row.map_err(|e| e.to_string())?;
            let mut s = self
                .connection
                .prepare(
                    "SELECT track FROM playlist_tracks WHERE playlist=? ORDER BY position,track",
                )
                .map_err(|e| e.to_string())?;
            let tracks = s
                .query_map([id], |r| r.get(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            playlists.push(Playlist { id, name, tracks });
        }
        let mut tags: HashMap<u64, Vec<String>> = HashMap::new();
        let mut stmt = self
            .connection
            .prepare("SELECT track,tag FROM tags ORDER BY tag")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, u64>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (track, tag) = row.map_err(|e| e.to_string())?;
            tags.entry(track).or_default().push(tag);
        }
        for track in &mut tracks {
            track.tags = tags.remove(&track.id).unwrap_or_default();
        }
        let mut stmt = self
            .connection
            .prepare("SELECT id,name,filter FROM collections ORDER BY name COLLATE NOCASE")
            .map_err(|e| e.to_string())?;
        let collections = stmt
            .query_map([], |r| {
                Ok(Collection {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    filter: serde_json::from_str(&r.get::<_, String>(2)?).unwrap_or(
                        CollectionFilter {
                            query: String::new(),
                            kind: None,
                            tags: Vec::new(),
                            max_duration: None,
                            favorite: false,
                        },
                    ),
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let banks = self.sample_banks();
        Ok(Library {
            tracks,
            playlists,
            collections,
            banks,
        })
    }
    fn sample_banks(&self) -> Vec<Vec<Option<u64>>> {
        let mut banks: Vec<Vec<Option<u64>>> = self.setting("sample-banks").unwrap_or_default();
        banks.resize(SAMPLE_BANKS, vec![None; SAMPLE_PADS]);
        banks
    }
    /// A piece's stretch of its file, when it is only part of one.
    pub fn clip_range(&self, id: u64) -> Option<(f64, f64)> {
        self.connection
            .query_row(
                "SELECT clip_start,clip_end FROM tracks WHERE id=?",
                [id],
                |r| Ok((r.get::<_, Option<f64>>(0)?, r.get::<_, Option<f64>>(1)?)),
            )
            .ok()
            .and_then(|(start, end)| Some((start?, end?)))
    }
    pub fn mark_used(&self, id: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |time| time.as_secs());
        let _ = self.connection.execute(
            "UPDATE tracks SET used_at=? WHERE id=? AND owned=1",
            params![now, id],
        );
    }
    /// What a file is, kept apart from where it is.
    pub fn set_identity(&self, id: u64, identity: &Identity) -> Result<(), String> {
        self.connection
            .execute(
                "UPDATE tracks SET file_size=?,fingerprint=?,volume=?,relative=? WHERE id=?",
                params![
                    identity.size,
                    identity.fingerprint,
                    identity.volume,
                    identity.relative,
                    id
                ],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    /// Another entry for the same content, with where it says it is and whether it is only
    /// known from a drive.
    pub fn same_content(
        &self,
        identity: &Identity,
    ) -> Result<Option<(u64, PathBuf, bool)>, String> {
        let found = self.connection.query_row(
            "SELECT id,path,EXISTS(SELECT 1 FROM settings WHERE key='device-only:'||tracks.id) FROM tracks WHERE fingerprint=? AND file_size=? AND kind<>'loop' ORDER BY id LIMIT 1",
            params![identity.fingerprint, identity.size],
            |r| Ok((r.get::<_, u64>(0)?, PathBuf::from(r.get::<_, String>(1)?), r.get(2)?)),
        );
        match found {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }
    /// Points an entry at a file's new place, and the loops cut from it along with it. Its cues,
    /// tags and playlists stay, since they hang from the id.
    pub fn relink(&self, id: u64, path: &Path, identity: &Identity) -> Result<(), String> {
        let tx = self
            .connection
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE tracks SET path=? WHERE id=?",
            params![path.to_string_lossy(), id],
        )
        .map_err(|error| match error {
            rusqlite::Error::SqliteFailure(failure, _)
                if failure.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                "Ese archivo ya está en la biblioteca como otra pista".to_string()
            }
            other => other.to_string(),
        })?;
        tx.execute(
            "UPDATE tracks SET path=? WHERE parent=? AND kind='loop'",
            params![path.to_string_lossy(), id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE tracks SET file_size=?,fingerprint=?,volume=?,relative=? WHERE id=? OR (parent=? AND kind='loop')",
            params![identity.size, identity.fingerprint, identity.volume, identity.relative, id, id],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
    /// A stretch of a track as a piece of its own, which copies no audio. The same stretch twice
    /// is the same piece. Calling it plays the loop, through its first cue.
    pub fn save_loop(&self, source: u64, start: f64, end: f64) -> Result<u64, String> {
        let row = self
            .connection
            .query_row(
                "SELECT path,title,artist,sample_rate,kind,parent,clip_start,file_size,fingerprint,volume,relative,waveform FROM tracks WHERE id=?",
                [source],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, u32>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, Option<u64>>(5)?,
                        r.get::<_, Option<f64>>(6)?,
                        r.get::<_, Option<u64>>(7)?,
                        r.get::<_, Option<String>>(8)?,
                        r.get::<_, Option<String>>(9)?,
                        r.get::<_, Option<String>>(10)?,
                        r.get::<_, Option<Vec<u8>>>(11)?,
                    ))
                },
            )
            .map_err(|_| String::from("Pista no encontrada"))?;
        let (path, title, artist, rate, kind, parent, base, size, print, volume, relative, wave) =
            row;
        let (parent, base) = if kind == "loop" {
            (parent, base.unwrap_or(0.))
        } else {
            (Some(source), 0.)
        };
        // Tenths of a millisecond: the same loop read twice must be the same numbers.
        let exact = |seconds: f64| (seconds * 10_000.).round() / 10_000.;
        let (from, to) = (exact(base + start), exact(base + end));
        let span = to - from;
        if span.is_nan() || span < 0.01 || from < 0. {
            return Err("Ese loop es demasiado corto".into());
        }
        let source_music = self.music(source);
        let label = source_music
            .and_then(|music| music.bpm)
            .map(|bpm| {
                let beats = (to - from) * bpm / 60.;
                format!("{} beats", (beats * 100.).round() / 100.)
            })
            .unwrap_or_else(|| format!("{:.1} s", to - from));
        let added = self
            .connection
            .execute(
                "INSERT OR IGNORE INTO tracks(path,title,artist,duration,sample_rate,peaks,waveform,kind,parent,clip_start,clip_end,file_size,fingerprint,volume,relative) VALUES(?,?,?,?,?,'[]',?,'loop',?,?,?,?,?,?,?)",
                params![
                    path,
                    format!("{title} · {label}"),
                    artist,
                    to - from,
                    rate,
                    wave.as_deref().map(|blob| cut_waveform(blob, start, end)),
                    parent,
                    from,
                    to,
                    size,
                    print,
                    volume,
                    relative
                ],
            )
            .map_err(|e| e.to_string())?;
        let id: u64 = self
            .connection
            .query_row(
                "SELECT id FROM tracks WHERE parent IS ? AND clip_start=? AND clip_end=? AND kind='loop'",
                params![parent, from, to],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if added > 0 {
            let mut music = source_music.unwrap_or_default();
            music.version = curado_audio::music::ANALYSIS_VERSION;
            if let Some(bpm) = music.bpm {
                let beat = 60. / bpm;
                music.beat_offset = (music.beat_offset - start).rem_euclid(beat);
                music.downbeat = (music.downbeat - start).rem_euclid(beat * 4.);
            }
            music.first_sound = 0.;
            self.save(&format!("music:{id}"), &music)?;
            let mut cues = [0.; HOT_CUES];
            let mut ends = [0.; HOT_CUES];
            cues[0] = curado_audio::annotations::CUE_AT_START;
            ends[0] = to - from;
            self.save(&format!("cues:{id}"), &cues)?;
            self.save(&format!("cue-ends:{id}"), &ends)?;
        }
        Ok(id)
    }
    /// A generated stem, replacing an earlier one of the same track and model. It lines up with
    /// its track in time, so it keeps its grid, key and cues.
    #[allow(clippy::too_many_arguments)]
    pub fn save_stem(
        &self,
        parent: Option<u64>,
        title: &str,
        artist: &str,
        model: &str,
        stem: &str,
        path: &Path,
        duration: f64,
        rate: u32,
        waveform: Option<&[u8]>,
        identity: &Identity,
    ) -> Result<u64, String> {
        self.connection
            .execute(
                "INSERT INTO tracks(path,title,artist,duration,sample_rate,peaks,waveform,kind,parent,stem,stem_model,owned,file_size,fingerprint) VALUES(?,?,?,?,?,'[]',?,'stem',?,?,?,1,?,?)
                ON CONFLICT(parent,stem_model,stem) WHERE kind='stem' DO UPDATE SET path=excluded.path,duration=excluded.duration,sample_rate=excluded.sample_rate,waveform=excluded.waveform,file_size=excluded.file_size,fingerprint=excluded.fingerprint",
                params![
                    path.to_string_lossy(),
                    title,
                    artist,
                    duration,
                    rate,
                    waveform,
                    parent,
                    stem,
                    model,
                    identity.size,
                    identity.fingerprint
                ],
            )
            .map_err(|e| e.to_string())?;
        let id: u64 = self
            .connection
            .query_row(
                "SELECT id FROM tracks WHERE path=? AND kind='stem'",
                [path.to_string_lossy()],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if let Some(parent) = parent {
            if let Some(music) = self.music(parent) {
                self.save(&format!("music:{id}"), &music)?;
            }
            let (cues, ends) = self.saved_cues(parent);
            self.save(&format!("cues:{id}"), &cues)?;
            self.save(&format!("cue-ends:{id}"), &ends)?;
        }
        self.mark_used(id);
        Ok(id)
    }
    pub fn has_loops(&self, id: u64) -> bool {
        self.connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tracks WHERE parent=? AND kind='loop')",
                [id],
                |r| r.get(0),
            )
            .unwrap_or(false)
    }
    pub fn known_fingerprint(&self, id: u64) -> Result<Option<String>, String> {
        self.connection
            .query_row("SELECT fingerprint FROM tracks WHERE id=?", [id], |r| {
                r.get(0)
            })
            .map_err(|_| "Pista no encontrada".into())
    }
    pub fn piece_info(&self, id: u64) -> Result<PieceInfo, String> {
        self.connection
            .query_row(
                "SELECT title,artist,duration,kind,fingerprint FROM tracks WHERE id=?",
                [id],
                |r| {
                    Ok(PieceInfo {
                        title: r.get(0)?,
                        artist: r.get(1)?,
                        duration: r.get(2)?,
                        kind: r.get(3)?,
                        fingerprint: r.get(4)?,
                    })
                },
            )
            .map_err(|_| "Pista no encontrada".into())
    }
    /// Whether a separation already exists whole, so asking again costs nothing.
    pub fn stems_ready(&self, id: u64, model: &str, expected: usize) -> bool {
        let Ok(mut stmt) = self
            .connection
            .prepare("SELECT path FROM tracks WHERE parent=? AND stem_model=? AND kind='stem'")
        else {
            return false;
        };
        let Ok(rows) = stmt.query_map(params![id, model], |r| r.get::<_, String>(0)) else {
            return false;
        };
        let paths: Vec<String> = rows.flatten().collect();
        paths.len() == expected && paths.iter().all(|path| Path::new(path).exists())
    }
    /// The file curado made for a piece, which goes when the piece does. Never one of yours.
    pub fn owned_file(&self, id: u64) -> Option<PathBuf> {
        self.connection
            .query_row(
                "SELECT path FROM tracks WHERE id=? AND owned=1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .map(PathBuf::from)
    }
    pub fn insert(&self, path: &Path, clip: &Clip) -> Result<bool, String> {
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        let title = clip.title.clone().unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into()
        });
        // `peaks` is the v1/v2 overview column, kept only for schema compatibility.
        let inserted=self.connection.execute("INSERT OR IGNORE INTO tracks(path,title,artist,duration,sample_rate,peaks,waveform) VALUES(?,?,?,?,?,'[]',?)", params![path.to_string_lossy(), title, clip.artist.as_deref().unwrap_or("Artista desconocido"), clip.duration(), clip.sample_rate, analysis::encode(&clip.waveform)]).map(|n| n > 0).map_err(|e| e.to_string())?;
        let id: u64 = self
            .connection
            .query_row(
                "SELECT id FROM tracks WHERE path=? AND kind<>'loop'",
                [path.to_string_lossy()],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if self.music(id).is_none_or(|music| !music.is_current()) {
            self.save(&format!("music:{id}"), &clip.music)?;
        }
        // Importing it on purpose makes a file from a drive part of the library.
        self.connection
            .execute(
                "DELETE FROM settings WHERE key=?",
                [format!("device-only:{id}")],
            )
            .map_err(|e| e.to_string())?;
        Ok(inserted)
    }
    pub fn id_of_file(&self, path: &Path) -> Result<u64, String> {
        self.connection
            .query_row(
                "SELECT id FROM tracks WHERE path=? AND kind<>'loop'",
                [path.to_string_lossy()],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }
    /// A track from a drive, with details filled in by its first decode if needed. One already
    /// in the library stays there; a new one is kept out of the library's lists.
    pub fn register_reference(
        &self,
        path: &Path,
        title: &str,
        artist: &str,
        duration: f64,
    ) -> Result<u64, String> {
        let added = self
            .connection
            .execute(
                "INSERT OR IGNORE INTO tracks(path,title,artist,duration,sample_rate,peaks) VALUES(?,?,?,?,0,'[]')",
                params![path.to_string_lossy(), title, artist, duration],
            )
            .map_err(|e| e.to_string())?;
        let id: u64 = self
            .connection
            .query_row(
                "SELECT id FROM tracks WHERE path=? AND kind<>'loop'",
                [path.to_string_lossy()],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if added > 0 {
            self.save(&format!("device-only:{id}"), &true)?;
        }
        Ok(id)
    }
    /// Details only a decode knows. Runs once: a track with a sample rate is complete.
    pub fn complete_reference(&self, id: u64, clip: &Clip) -> Result<(), String> {
        self.connection
            .execute(
                "UPDATE tracks SET sample_rate=?,duration=?,title=COALESCE(?,title),artist=COALESCE(?,artist) WHERE id=? AND sample_rate=0",
                params![clip.sample_rate, clip.duration(), clip.title, clip.artist, id],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    /// Saved hot cues and loop ends. Older versions saved three cues and no ends.
    pub fn saved_cues(&self, id: u64) -> ([f64; HOT_CUES], [f64; HOT_CUES]) {
        let read = |key: String| -> [f64; HOT_CUES] {
            let saved = self.setting::<Vec<f64>>(&key).unwrap_or_default();
            std::array::from_fn(|i| saved.get(i).copied().unwrap_or(0.))
        };
        (read(format!("cues:{id}")), read(format!("cue-ends:{id}")))
    }
    pub fn music(&self, id: u64) -> Option<curado_audio::music::MusicAnalysis> {
        self.setting::<curado_audio::music::MusicAnalysis>(&format!("music:{id}"))
            .filter(|m| m.valid())
    }
    pub fn waveform(&self, id: u64) -> Result<Vec<u8>, String> {
        self.connection
            .query_row("SELECT waveform FROM tracks WHERE id=?", [id], |r| {
                r.get::<_, Option<Vec<u8>>>(0)
            })
            .map(Option::unwrap_or_default)
            .map_err(|_| "Pista no encontrada".into())
    }
    /// Backfills rows imported before v3. Never overwrites an existing analysis.
    pub fn fill_waveform(&self, id: u64, clip: &Clip) -> Result<bool, String> {
        if clip.waveform.is_empty() {
            return Ok(false);
        }
        self.connection
            .execute(
                "UPDATE tracks SET waveform=? WHERE id=? AND waveform IS NULL",
                params![analysis::encode(&clip.waveform), id],
            )
            .map(|n| n > 0)
            .map_err(|e| e.to_string())
    }
    pub fn path(&self, id: u64) -> Result<PathBuf, String> {
        self.connection
            .query_row("SELECT path FROM tracks WHERE id=?", [id], |r| {
                r.get::<_, String>(0)
            })
            .map(PathBuf::from)
            .map_err(|_| "Pista no encontrada".into())
    }
    pub fn edit(&self, edit: LibraryEdit) -> Result<(), String> {
        let name = |v: String| -> Result<String, String> {
            let s = v.trim();
            if s.is_empty() || s.len() > 100 {
                Err("El nombre debe tener entre 1 y 100 bytes".into())
            } else {
                Ok(s.into())
            }
        };
        let result = match edit {
            LibraryEdit::CreatePlaylist { name: n } => self.connection.execute("INSERT INTO playlists(name) VALUES(?)", [name(n)?]),
            LibraryEdit::RenamePlaylist { id, name: n } => self.connection.execute("UPDATE playlists SET name=? WHERE id=?", params![name(n)?, id]),
            LibraryEdit::DeletePlaylist { id } => self.connection.execute("DELETE FROM playlists WHERE id=?", [id]),
            LibraryEdit::AddToPlaylist { playlist, track } => self.connection.execute("INSERT OR IGNORE INTO playlist_tracks(playlist,track,position) VALUES(?,?,COALESCE((SELECT MAX(position)+1 FROM playlist_tracks WHERE playlist=?),0))", params![playlist, track, playlist]),
            LibraryEdit::RemoveFromPlaylist { playlist, track } => self.connection.execute("DELETE FROM playlist_tracks WHERE playlist=? AND track=?", params![playlist, track]),
            LibraryEdit::Favorite { track, value } => self.connection.execute("UPDATE tracks SET favorite=? WHERE id=?", params![value, track]),
            LibraryEdit::RemoveTrack { track } => self.connection.execute("DELETE FROM tracks WHERE id=?", [track]),
            LibraryEdit::AddTag { track, tag } => self.connection.execute("INSERT OR IGNORE INTO tags(track,tag) SELECT id,? FROM tracks WHERE id=?", params![clean_tag(&tag)?, track]),
            LibraryEdit::RemoveTag { track, tag } => self.connection.execute("DELETE FROM tags WHERE track=? AND tag=?", params![track, clean_tag(&tag)?]),
            LibraryEdit::SaveCollection { name: n, filter } => self.connection.execute("INSERT INTO collections(name,filter) VALUES(?,?) ON CONFLICT(name) DO UPDATE SET filter=excluded.filter", params![name(n)?, serde_json::to_string(&filter).map_err(|e| e.to_string())?]),
            LibraryEdit::DeleteCollection { id } => self.connection.execute("DELETE FROM collections WHERE id=?", [id]),
            LibraryEdit::SaveBank { index, tracks } => {
                if index >= SAMPLE_BANKS || tracks.len() != SAMPLE_PADS {
                    return Err("Banco de samples inválido".into());
                }
                let mut banks = self.sample_banks();
                banks[index] = tracks;
                return self.save("sample-banks", &banks);
            }
        };
        result.map(|_| ()).map_err(|e| e.to_string())
    }
    pub fn setting<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.connection
            .query_row("SELECT value FROM settings WHERE key=?", [key], |r| {
                r.get::<_, String>(0)
            })
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    }
    pub fn forget(&self, key: &str) -> Result<(), String> {
        self.connection
            .execute("DELETE FROM settings WHERE key=?", [key])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    /// Beats of a track whose tempo moves, from an analysis or a rekordbox grid.
    pub fn beats(&self, id: u64) -> Option<Vec<f64>> {
        self.setting::<Vec<f64>>(&format!("beats:{id}"))
            .filter(|beats| beats.len() >= 2 && beats.windows(2).all(|pair| pair[1] > pair[0]))
    }
    pub fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<(), String> {
        self.connection.execute("INSERT INTO settings(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, serde_json::to_string(value).map_err(|e| e.to_string())?]).map(|_| ()).map_err(|e| e.to_string())
    }
}
/// Tags are compared as typed once tidied, so one idea is not two spellings of case and spaces.
fn clean_tag(tag: &str) -> Result<String, String> {
    let clean = tag
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    if clean.is_empty() || clean.len() > MAX_TAG_BYTES {
        return Err("La etiqueta debe tener entre 1 y 40 bytes".into());
    }
    Ok(clean)
}
/// The part of a stored waveform for a stretch of its track.
fn cut_waveform(blob: &[u8], start: f64, end: f64) -> Vec<u8> {
    const HEADER: usize = 4;
    const BIN: usize = 4;
    if blob.len() < HEADER || blob[1] == 0 {
        return blob.to_vec();
    }
    let rate = blob[1] as f64;
    let bins = (blob.len() - HEADER) / BIN;
    let place = |seconds: f64| ((seconds.max(0.) * rate) as usize).min(bins);
    let mut out = blob[..HEADER].to_vec();
    out.extend_from_slice(&blob[HEADER + place(start) * BIN..HEADER + place(end) * BIN]);
    out
}
pub fn supported(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        [
            "mp3", "wav", "flac", "aif", "aiff", "m4a", "aac", "ogg", "caf",
        ]
        .contains(&e.to_ascii_lowercase().as_str())
    })
}
pub fn walk(root: PathBuf, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if files.len() >= 2000 {
        return Err("Máximo 2000 archivos por importación; selecciona una carpeta menor".into());
    }
    let metadata = std::fs::symlink_metadata(&root).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.is_dir() {
        for entry in std::fs::read_dir(root).map_err(|e| e.to_string())? {
            walk(entry.map_err(|e| e.to_string())?.path(), files)?;
        }
    } else if supported(&root) {
        files.push(root);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn music_metadata_is_optional_validated_and_preserves_manual_settings() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        db.connection.execute("INSERT INTO tracks(id,path,title,artist,duration,sample_rate,peaks) VALUES(1,'a.wav','A','B',30,48000,'[]')",[]).unwrap();
        assert!(db.library().unwrap().tracks[0].music.is_none());
        let music = curado_audio::music::MusicAnalysis {
            version: 1,
            bpm: Some(123.45),
            beat_offset: 0.123,
            key: Some(curado_audio::music::MusicalKey {
                tonic: 9,
                minor: true,
            }),
            manual: true,
            ..Default::default()
        };
        db.save("music:1", &music).unwrap();
        assert_eq!(db.music(1), Some(music));
        assert_eq!(db.library().unwrap().tracks[0].music, Some(music));
        db.save("music:1", &serde_json::json!({"bad":true}))
            .unwrap();
        assert!(db.library().unwrap().tracks[0].music.is_none());
    }
    #[test]
    fn analysis_saved_by_older_versions_stays_readable() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        db.connection.execute("INSERT INTO tracks(id,path,title,artist,duration,sample_rate,peaks) VALUES(1,'a.wav','A','B',30,48000,'[]')",[]).unwrap();
        // Exactly what version 1 wrote: no bars and no first sound.
        let stored = serde_json::json!({
            "version": 1,
            "bpm": 124.0,
            "beatOffset": 0.25,
            "bpmConfidence": 0.5,
            "key": null,
            "keyConfidence": 0.0,
            "manual": false
        });
        db.save("music:1", &stored).unwrap();
        let estimate = db.music(1).expect("still valid");
        assert_eq!(estimate.bpm, Some(124.));
        assert_eq!((estimate.downbeat, estimate.first_sound), (0., 0.));
        assert!(!estimate.is_current(), "an old estimate is recalculated");
        let mut corrected = stored;
        corrected["manual"] = true.into();
        db.save("music:1", &corrected).unwrap();
        assert!(
            db.music(1).unwrap().is_current(),
            "a manual correction is never replaced"
        );
    }
    #[test]
    fn playlists_persist_and_cascade_without_deleting_files() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        db.connection.execute("INSERT INTO tracks(id,path,title,artist,duration,sample_rate,peaks) VALUES(1,'/tmp/a.wav','A','B',1,48000,'[]')", []).unwrap();
        db.edit(LibraryEdit::CreatePlaylist {
            name: "Sesión".into(),
        })
        .unwrap();
        db.edit(LibraryEdit::AddToPlaylist {
            playlist: 1,
            track: 1,
        })
        .unwrap();
        db.edit(LibraryEdit::AddToPlaylist {
            playlist: 1,
            track: 1,
        })
        .unwrap();
        assert_eq!(db.library().unwrap().playlists[0].tracks, [1]);
        db.edit(LibraryEdit::RemoveTrack { track: 1 }).unwrap();
        assert!(db.library().unwrap().playlists[0].tracks.is_empty());
        db.save("test", &vec![1, 2]).unwrap();
        assert_eq!(db.setting::<Vec<i32>>("test").unwrap(), [1, 2]);
    }
    #[test]
    fn migration_preserves_playlists_and_never_recycles_track_ids() {
        let path =
            std::env::temp_dir().join(format!("curado-migration-{}.sqlite", std::process::id()));
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch("CREATE TABLE tracks(id INTEGER PRIMARY KEY,path TEXT NOT NULL UNIQUE,title TEXT NOT NULL,artist TEXT NOT NULL,duration REAL NOT NULL,sample_rate INTEGER NOT NULL,peaks TEXT NOT NULL,favorite INTEGER NOT NULL DEFAULT 0);CREATE TABLE playlists(id INTEGER PRIMARY KEY,name TEXT NOT NULL);CREATE TABLE playlist_tracks(playlist INTEGER REFERENCES playlists(id) ON DELETE CASCADE,track INTEGER REFERENCES tracks(id) ON DELETE CASCADE,position INTEGER NOT NULL,PRIMARY KEY(playlist,track));CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);INSERT INTO tracks VALUES(10,'old.wav','Old','Artist',1,48000,'[]',1);INSERT INTO playlists VALUES(1,'Set');INSERT INTO playlist_tracks VALUES(1,10,0);PRAGMA user_version=1;").unwrap();
        }
        let db = Database::open(&path).unwrap();
        let version: u32 = db
            .connection
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA);
        assert!(
            db.waveform(10).unwrap().is_empty(),
            "legacy rows wait for backfill"
        );
        let clip = Clip {
            loudness: None,
            beats: None,
            id: 10,
            cues: Default::default(),
            samples: vec![],
            sample_rate: 48000,
            waveform: vec![[9, 8, 7, 6]],
            title: None,
            artist: None,
            music: Default::default(),
        };
        assert!(db.fill_waveform(10, &clip).unwrap());
        assert!(!db.fill_waveform(10, &clip).unwrap(), "never overwrites");
        assert_eq!(db.waveform(10).unwrap()[4..], [9, 8, 7, 6]);
        assert_eq!(db.library().unwrap().playlists[0].tracks, vec![10]);
        assert!(db.library().unwrap().tracks[0].favorite);
        db.edit(LibraryEdit::RemoveTrack { track: 10 }).unwrap();
        db.connection.execute("INSERT INTO tracks(path,title,artist,duration,sample_rate,peaks) VALUES('new.wav','New','Artist',1,48000,'[]')",[]).unwrap();
        assert!(db.library().unwrap().tracks[0].id > 10);
        assert!(db.library().unwrap().playlists[0].tracks.is_empty());
        drop(db);
        let _ = std::fs::remove_file(path);
    }
    fn piece(db: &Database, id: u64, path: &str) {
        db.connection.execute("INSERT INTO tracks(id,path,title,artist,duration,sample_rate,peaks) VALUES(?,?,'A','B',100,48000,'[]')", params![id, path]).unwrap();
    }
    fn who(print: &str) -> Identity {
        Identity {
            size: 10,
            fingerprint: print.into(),
            volume: None,
            relative: None,
        }
    }
    #[test]
    fn a_v3_library_keeps_everything_and_its_ids_are_never_reused() {
        let path = std::env::temp_dir().join(format!("curado-v4-{}.sqlite", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch("CREATE TABLE tracks (id INTEGER PRIMARY KEY AUTOINCREMENT, path TEXT NOT NULL UNIQUE, title TEXT NOT NULL, artist TEXT NOT NULL, duration REAL NOT NULL, sample_rate INTEGER NOT NULL, peaks TEXT NOT NULL, favorite INTEGER NOT NULL DEFAULT 0, waveform BLOB);
            CREATE TABLE playlists (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
            CREATE TABLE playlist_tracks (playlist INTEGER REFERENCES playlists(id) ON DELETE CASCADE, track INTEGER REFERENCES tracks(id) ON DELETE CASCADE, position INTEGER NOT NULL, PRIMARY KEY(playlist, track));
            CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            INSERT INTO tracks(id,path,title,artist,duration,sample_rate,peaks,favorite,waveform) VALUES(3,'/m/a.mp3','A','B',10,44100,'[]',1,x'0102');
            INSERT INTO tracks(id,path,title,artist,duration,sample_rate,peaks) VALUES(9,'/m/z.mp3','Z','B',10,44100,'[]');
            DELETE FROM tracks WHERE id=9;
            INSERT INTO playlists VALUES(1,'p');
            INSERT INTO playlist_tracks VALUES(1,3,0);
            INSERT INTO settings VALUES('cues:3','[1.0]');
            PRAGMA user_version=3;").unwrap();
        }
        let db = Database::open(&path).unwrap();
        let library = db.library().unwrap();
        assert_eq!(library.tracks.len(), 1);
        let track = &library.tracks[0];
        assert_eq!(
            (track.id, track.favorite, track.kind.as_str()),
            (3, true, "track")
        );
        assert_eq!(library.playlists[0].tracks, [3]);
        assert_eq!(db.waveform(3).unwrap(), [1, 2]);
        assert_eq!(db.saved_cues(3).0[0], 1.);
        // Cues of the deleted 9 must not attach to a new file.
        db.connection.execute("INSERT INTO tracks(path,title,artist,duration,sample_rate,peaks) VALUES('/m/n.mp3','N','B',1,44100,'[]')", []).unwrap();
        assert_eq!(db.id_of_file(Path::new("/m/n.mp3")).unwrap(), 10);
        drop(db);
        // Opening again changes nothing, and a newer library is left alone.
        let db = Database::open(&path).unwrap();
        assert_eq!(db.library().unwrap().tracks.len(), 2);
        db.connection
            .execute_batch("PRAGMA user_version=5")
            .unwrap();
        drop(db);
        assert!(Database::open(&path).is_err());
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
    }
    #[test]
    fn the_same_file_cannot_be_listed_twice_but_loops_can_share_it() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        piece(&db, 1, "/m/a.wav");
        assert!(db.connection.execute("INSERT INTO tracks(path,title,artist,duration,sample_rate,peaks) VALUES('/m/a.wav','A','B',1,1,'[]')", []).is_err());
        let first = db.save_loop(1, 4., 6.).unwrap();
        assert_eq!(
            db.save_loop(1, 4., 6.).unwrap(),
            first,
            "the same stretch is one piece"
        );
        assert_ne!(db.save_loop(1, 4., 8.).unwrap(), first);
        assert_eq!(db.clip_range(first), Some((4., 6.)));
        assert_eq!(db.clip_range(1), None);
        assert_eq!(db.id_of_file(Path::new("/m/a.wav")).unwrap(), 1);
        assert!(db.save_loop(1, 6., 6.).is_err());
        assert!(db.save_loop(99, 1., 2.).is_err());
        assert_eq!(db.library().unwrap().tracks.len(), 3);
    }
    #[test]
    fn a_loop_of_a_loop_counts_from_the_track_and_calls_as_a_loop() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        piece(&db, 1, "/m/a.wav");
        db.save(
            "music:1",
            &curado_audio::music::MusicAnalysis {
                version: curado_audio::music::ANALYSIS_VERSION,
                bpm: Some(120.),
                beat_offset: 0.1,
                downbeat: 0.1,
                ..Default::default()
            },
        )
        .unwrap();
        let outer = db.save_loop(1, 10., 18.).unwrap();
        let inner = db.save_loop(outer, 2., 4.).unwrap();
        assert_eq!(db.clip_range(inner), Some((12., 14.)));
        let library = db.library().unwrap();
        let found = library.tracks.iter().find(|t| t.id == inner).unwrap();
        assert_eq!(found.parent, Some(1));
        assert!(found.title.contains("4 beats"), "{}", found.title);
        let (cues, ends) = db.saved_cues(inner);
        assert_eq!(cues[0], curado_audio::annotations::CUE_AT_START);
        assert_eq!(ends[0], 2.);
        // Its beats fall where they did: the grid moves with the stretch.
        let music = db.music(outer).unwrap();
        assert!(
            (music.beat_offset - 0.1).abs() < 1e-9,
            "{}",
            music.beat_offset
        );
    }
    #[test]
    fn tags_are_one_spelling_and_go_with_their_piece() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        piece(&db, 1, "/m/a.wav");
        let tag = |t: &str| LibraryEdit::AddTag {
            track: 1,
            tag: t.into(),
        };
        db.edit(tag("  Vocal   Chop ")).unwrap();
        db.edit(tag("vocal chop")).unwrap();
        db.edit(tag("sample")).unwrap();
        assert_eq!(
            db.library().unwrap().tracks[0].tags,
            ["sample", "vocal chop"]
        );
        assert!(db.edit(tag("   ")).is_err());
        assert!(db.edit(tag(&"x".repeat(41))).is_err());
        db.edit(LibraryEdit::AddTag {
            track: 99,
            tag: "x".into(),
        })
        .unwrap();
        db.edit(LibraryEdit::RemoveTag {
            track: 1,
            tag: "SAMPLE".into(),
        })
        .unwrap();
        assert_eq!(db.library().unwrap().tracks[0].tags, ["vocal chop"]);
        db.edit(LibraryEdit::RemoveTrack { track: 1 }).unwrap();
        let left: u32 = db
            .connection
            .query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left, 0);
    }
    #[test]
    fn collections_and_banks_keep_one_of_each() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        let filter = |query: &str| CollectionFilter {
            query: query.into(),
            kind: Some("stem".into()),
            tags: vec!["dub".into()],
            max_duration: Some(5.),
            favorite: true,
        };
        let save = |query: &str| LibraryEdit::SaveCollection {
            name: "drums".into(),
            filter: filter(query),
        };
        db.edit(save("a")).unwrap();
        db.edit(save("b")).unwrap();
        let library = db.library().unwrap();
        assert_eq!(library.collections.len(), 1);
        assert_eq!(library.collections[0].filter, filter("b"));
        assert_eq!(library.banks.len(), SAMPLE_BANKS);
        assert!(library.banks.iter().all(|bank| bank.len() == SAMPLE_PADS));
        let bank = vec![Some(7), None, None, None, None, None, None, Some(9)];
        db.edit(LibraryEdit::SaveBank {
            index: 2,
            tracks: bank.clone(),
        })
        .unwrap();
        assert_eq!(db.library().unwrap().banks[2], bank);
        assert!(db
            .edit(LibraryEdit::SaveBank {
                index: 4,
                tracks: bank
            })
            .is_err());
        assert!(db
            .edit(LibraryEdit::SaveBank {
                index: 0,
                tracks: vec![None]
            })
            .is_err());
    }
    #[test]
    fn relinking_moves_loops_along_and_refuses_a_file_already_listed() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        piece(&db, 1, "/old/a.wav");
        piece(&db, 2, "/old/b.wav");
        db.edit(LibraryEdit::Favorite {
            track: 1,
            value: true,
        })
        .unwrap();
        let cut = db.save_loop(1, 1., 2.).unwrap();
        db.relink(1, Path::new("/new/a.wav"), &who("abc")).unwrap();
        assert_eq!(db.path(1).unwrap(), PathBuf::from("/new/a.wav"));
        assert_eq!(db.path(cut).unwrap(), PathBuf::from("/new/a.wav"));
        assert!(
            db.library()
                .unwrap()
                .tracks
                .iter()
                .find(|t| t.id == 1)
                .unwrap()
                .favorite
        );
        let error = db
            .relink(2, Path::new("/new/a.wav"), &who("x"))
            .unwrap_err();
        assert!(error.contains("otra pista"), "{error}");
        assert_eq!(db.path(2).unwrap(), PathBuf::from("/old/b.wav"));
        assert_eq!(db.same_content(&who("abc")).unwrap().map(|f| f.0), Some(1));
        assert_eq!(db.same_content(&who("nope")).unwrap(), None);
    }
    #[test]
    fn only_what_curado_made_counts_as_its_own_file() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        piece(&db, 1, "/m/yours.wav");
        assert_eq!(db.owned_file(1), None);
    }
    #[test]
    fn a_stored_waveform_is_cut_with_its_stretch() {
        let mut blob = vec![1, 10, 0, 0];
        for bin in 0..100_u8 {
            blob.extend([bin; 4]);
        }
        let cut = cut_waveform(&blob, 1., 2.);
        assert_eq!(cut.len(), 4 + 10 * 4);
        assert_eq!(&cut[..4], &[1, 10, 0, 0]);
        assert_eq!(cut[4], 10);
        assert_eq!(cut_waveform(&blob, 50., 99.).len(), 4);
        assert_eq!(cut_waveform(&[1], 0., 1.), [1]);
    }
    /// `CURADO_TEST_DB=/copy/of/curado.sqlite cargo test -p curado --lib real_library -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a copy of a real library"]
    fn real_library() {
        let path = PathBuf::from(std::env::var("CURADO_TEST_DB").unwrap());
        let before: (u32, u32) = {
            let c = Connection::open(&path).unwrap();
            let count = |table: &str| -> u32 {
                c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                    .unwrap()
            };
            (count("tracks"), count("playlist_tracks"))
        };
        let started = std::time::Instant::now();
        let db = Database::open(&path).unwrap();
        println!("migrated in {:?}", started.elapsed());
        let started = std::time::Instant::now();
        let library = db.library().unwrap();
        println!(
            "{} tracks, {} playlists, listed in {:?}",
            library.tracks.len(),
            library.playlists.len(),
            started.elapsed()
        );
        let entries: u32 = library
            .playlists
            .iter()
            .map(|p| p.tracks.len() as u32)
            .sum();
        assert_eq!((library.tracks.len() as u32, entries), before);
        let broken: u32 = db
            .connection
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(broken, 0);
    }
}
