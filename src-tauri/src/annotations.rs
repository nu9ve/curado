use curado_audio::annotations::CueState;
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
    pub fn state(&mut self, id: u64, initial: [f64; 3]) -> Arc<CueState> {
        self.tracks
            .entry(id)
            .or_insert_with(|| SavedCue {
                state: Arc::new(CueState::new(initial)),
                saved: 0,
            })
            .state
            .clone()
    }
    pub fn flush(&mut self, db: &crate::library::Database) -> Result<(), String> {
        for (id, record) in &mut self.tracks {
            if record.state.revision() != record.saved {
                let (revision, values) = record.state.snapshot();
                db.save(&format!("cues:{id}"), &values)?;
                record.saved = revision;
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cues_share_across_decks_and_survive_database_reopen() {
        let path = std::env::temp_dir().join(format!("curado-cues-{}.sqlite", std::process::id()));
        {
            let db = crate::library::Database::open(&path).unwrap();
            let mut cues = Annotations::default();
            let a = cues.state(10, [0.; 3]);
            let b = cues.state(10, [1.; 3]);
            a.set(1, 14.25);
            assert_eq!(b.get(1), 14.25);
            cues.flush(&db).unwrap();
            assert_eq!(db.setting::<[f64; 3]>("cues:10").unwrap(), [0., 14.25, 0.]);
        }
        {
            let db = crate::library::Database::open(&path).unwrap();
            assert_eq!(db.setting::<[f64; 3]>("cues:10").unwrap()[1], 14.25);
        }
        let _ = std::fs::remove_file(path);
    }
}
