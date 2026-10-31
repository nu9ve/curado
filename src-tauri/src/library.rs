use curado_audio::{analysis, decode::Clip};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
}
#[derive(Serialize, Default)]
pub struct ImportReport {
    pub added: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LibraryEdit {
    CreatePlaylist { name: String },
    RenamePlaylist { id: i64, name: String },
    DeletePlaylist { id: i64 },
    AddToPlaylist { playlist: i64, track: u64 },
    RemoveFromPlaylist { playlist: i64, track: u64 },
    Favorite { track: u64, value: bool },
    RemoveTrack { track: u64 },
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
        if version > 3 {
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
        connection.execute_batch("PRAGMA foreign_keys=ON; BEGIN IMMEDIATE;
        CREATE TABLE IF NOT EXISTS tracks (id INTEGER PRIMARY KEY AUTOINCREMENT, path TEXT NOT NULL UNIQUE, title TEXT NOT NULL, artist TEXT NOT NULL, duration REAL NOT NULL, sample_rate INTEGER NOT NULL, peaks TEXT NOT NULL, favorite INTEGER NOT NULL DEFAULT 0, waveform BLOB);
        CREATE TABLE IF NOT EXISTS playlists (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS playlist_tracks (playlist INTEGER REFERENCES playlists(id) ON DELETE CASCADE, track INTEGER REFERENCES tracks(id) ON DELETE CASCADE, position INTEGER NOT NULL, PRIMARY KEY(playlist, track));
        CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL); PRAGMA user_version=3; COMMIT;").map_err(|e| e.to_string())?;
        Ok(Self { connection })
    }
    pub fn library(&self) -> Result<Library, String> {
        // Waveforms are fetched per track on demand; the listing stays small for large libraries.
        let mut stmt = self.connection.prepare("SELECT id,path,title,artist,duration,sample_rate,favorite,(SELECT value FROM settings WHERE key='music:'||tracks.id) FROM tracks ORDER BY title COLLATE NOCASE").map_err(|e| e.to_string())?;
        let tracks = stmt
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
        Ok(Library { tracks, playlists })
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
                "SELECT id FROM tracks WHERE path=?",
                [path.to_string_lossy()],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if self.music(id).is_none_or(|music| !music.is_current()) {
            self.save(&format!("music:{id}"), &clip.music)?;
        }
        Ok(inserted)
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
    pub fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<(), String> {
        self.connection.execute("INSERT INTO settings(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, serde_json::to_string(value).map_err(|e| e.to_string())?]).map(|_| ()).map_err(|e| e.to_string())
    }
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
        assert_eq!(version, 3);
        assert!(
            db.waveform(10).unwrap().is_empty(),
            "legacy rows wait for backfill"
        );
        let clip = Clip {
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
}
