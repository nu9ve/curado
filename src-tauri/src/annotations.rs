use curado_audio::annotations::{CueState, HOT_CUES};
use std::{collections::HashMap, sync::Arc};
pub struct SavedCue {
    pub state: Arc<CueState>,
    saved: u64,
}
#[derive(Default)]
pub struct Annotations {
    tracks: HashMap<u64, SavedCue>,
}
impl Annotations {
    pub fn state(
        &mut self,
        id: u64,
        cues: [f64; HOT_CUES],
        ends: [f64; HOT_CUES],
    ) -> Arc<CueState> {
        self.tracks
            .entry(id)
            .or_insert_with(|| SavedCue {
                state: Arc::new(CueState::new(cues, ends)),
                saved: 0,
            })
            .state
            .clone()
    }
    /// For cues just written to the database by a new export: a session that already knows
    /// this track must not write its older copy back over them.
    pub fn replace(&mut self, id: u64, cues: [f64; HOT_CUES], ends: [f64; HOT_CUES]) {
        if let Some(record) = self.tracks.get_mut(&id) {
            record.state.replace(cues, ends);
            record.saved = record.state.revision();
        }
    }
    pub fn flush(&mut self, db: &crate::library::Database) -> Result<(), String> {
        for (id, record) in &mut self.tracks {
            if record.state.revision() != record.saved {
                let (revision, cues, ends) = record.state.snapshot();
                db.save(&format!("cues:{id}"), &cues)?;
                db.save(&format!("cue-ends:{id}"), &ends)?;
                record.saved = revision;
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const NONE: [f64; HOT_CUES] = [0.; HOT_CUES];
    #[test]
    fn cues_share_across_decks_and_survive_database_reopen() {
        let path = std::env::temp_dir().join(format!("curado-cues-{}.sqlite", std::process::id()));
        {
            let db = crate::library::Database::open(&path).unwrap();
            let mut cues = Annotations::default();
            let a = cues.state(10, NONE, NONE);
            let b = cues.state(10, [1.; HOT_CUES], NONE);
            a.set(1, 14.25);
            a.set(7, 30.);
            assert_eq!(b.get(1), 14.25);
            cues.flush(&db).unwrap();
            let saved = db.saved_cues(10).0;
            assert_eq!((saved[0], saved[1], saved[7]), (0., 14.25, 30.));
        }
        {
            let db = crate::library::Database::open(&path).unwrap();
            assert_eq!(db.saved_cues(10).0[1], 14.25);
        }
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn cues_saved_by_older_versions_are_kept() {
        let db = crate::library::Database::open(std::path::Path::new(":memory:")).unwrap();
        db.save("cues:5", &[1.5_f64, 0., 9.]).unwrap();
        let (cues, ends) = db.saved_cues(5);
        assert_eq!(&cues[..4], [1.5, 0., 9., 0.]);
        assert_eq!(ends, NONE);
        assert_eq!(db.saved_cues(6), (NONE, NONE));
    }
    #[test]
    fn an_import_is_not_overwritten_by_the_copy_in_memory() {
        let db = crate::library::Database::open(std::path::Path::new(":memory:")).unwrap();
        let mut cues = Annotations::default();
        let state = cues.state(3, NONE, NONE);
        let mut imported = NONE;
        imported[0] = 4.;
        db.save("cues:3", &imported).unwrap();
        cues.replace(3, imported, NONE);
        assert_eq!(state.get(0), 4., "a loaded deck sees it");
        cues.flush(&db).unwrap();
        assert_eq!(db.saved_cues(3).0[0], 4.);
    }
}
