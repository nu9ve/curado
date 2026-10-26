use crate::AppState;
use curado_audio::{Control, Snapshot};
use serde::Serialize;
#[derive(Default)]
pub struct Queue {
    pub ids: Vec<u64>,
    pub index: usize,
    pub repeat: String,
    pub enabled: bool,
    armed: bool,
    pub error: Option<String>,
}
#[derive(Clone, Serialize)]
pub struct Status {
    pub count: usize,
    pub index: usize,
    pub active: bool,
    pub repeat: String,
    pub error: Option<String>,
}
impl Queue {
    pub fn status(&self) -> Status {
        Status {
            count: self.ids.len(),
            index: self.index,
            active: self.enabled,
            repeat: self.repeat.clone(),
            error: self.error.clone(),
        }
    }
    pub fn set(&mut self, ids: Vec<u64>, repeat: String) {
        *self = Self {
            ids,
            repeat,
            enabled: true,
            ..Default::default()
        };
    }
    pub fn finished(&mut self, snap: &Snapshot) -> Option<u64> {
        if !self.enabled {
            return None;
        }
        let a = &snap.slots[0];
        if self.ids.get(self.index).copied() != Some(a.id) {
            return None;
        }
        if a.playing {
            self.armed = true;
            return None;
        }
        if self.armed && a.duration > 0. && a.position >= a.duration - 0.0001 {
            self.armed = false;
            if self.repeat != "one" {
                self.index += 1;
            }
            if self.index >= self.ids.len() {
                if self.repeat == "all" {
                    self.index = 0;
                } else {
                    self.enabled = false;
                    return None;
                }
            }
            return self.ids.get(self.index).copied();
        }
        None
    }
}
pub fn tick(app: &AppState) {
    let snapshot = app
        .output
        .lock()
        .unwrap()
        .as_ref()
        .map(|o| o.handle.snapshot());
    let next = snapshot
        .as_ref()
        .and_then(|s| app.queue.lock().unwrap().finished(s));
    let Some(id) = next else {
        return;
    };
    let result = (|| {
        let _decode = app.decode_gate.lock().unwrap();
        let clip = app.decode_track(id)?;
        // Manual loads cancel the queue while the background decode is running.
        let queue = app.queue.lock().unwrap();
        if !queue.enabled || queue.ids.get(queue.index).copied() != Some(id) {
            return Ok(());
        }
        let output = app.output.lock().unwrap();
        let handle = &output.as_ref().ok_or("Motor no disponible")?.handle;
        handle.load(0, clip)?;
        handle.control(0, Control::Play)
    })();
    if let Err(e) = result {
        let mut q = app.queue.lock().unwrap();
        q.error = Some(e);
        q.enabled = false;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn advances_only_after_playback_reaches_end() {
        let (handle, _) = curado_audio::engine(48000);
        let mut snap = handle.snapshot();
        let mut q = Queue::default();
        q.set(vec![10, 20], "off".into());
        snap.slots[0].id = 10;
        snap.slots[0].duration = 5.;
        assert!(q.finished(&snap).is_none());
        snap.slots[0].playing = true;
        q.finished(&snap);
        snap.slots[0].playing = false;
        snap.slots[0].position = 2.;
        assert!(q.finished(&snap).is_none());
        snap.slots[0].position = 5.;
        assert_eq!(q.finished(&snap), Some(20));
        assert!(q.finished(&snap).is_none());
    }
    #[test]
    fn repeat_policies_do_not_advance_paused_tracks() {
        let (h, _) = curado_audio::engine(48000);
        let mut snap = h.snapshot();
        snap.slots[0].id = 1;
        snap.slots[0].duration = 1.;
        for (policy, expected, active) in [
            ("off", None, false),
            ("one", Some(1), true),
            ("all", Some(1), true),
        ] {
            let mut q = Queue::default();
            q.set(vec![1], policy.into());
            snap.slots[0].playing = true;
            q.finished(&snap);
            snap.slots[0].playing = false;
            snap.slots[0].position = 1.;
            assert_eq!(q.finished(&snap), expected);
            assert_eq!(q.enabled, active);
        }
    }
}
