use crate::AppState;
use curado_audio::{decode::Clip, Control, Snapshot};
use serde::Serialize;
use std::sync::Arc;

/// Decode the next track this many seconds before the current one ends.
pub const PREFETCH_SECONDS: f64 = 15.;

#[derive(Default)]
pub struct Queue {
    pub ids: Vec<u64>,
    pub index: usize,
    pub repeat: String,
    pub enabled: bool,
    armed: bool,
    pub error: Option<String>,
    /// At most one decoded clip waiting for the transition (≤96 MiB of PCM).
    prefetched: Option<(u64, Arc<Clip>)>,
    revision: u64,
    failed_prefetch: Option<u64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrefetchRequest {
    revision: u64,
    id: u64,
}
#[derive(Clone, Serialize)]
pub struct Status {
    pub count: usize,
    pub index: usize,
    pub active: bool,
    pub repeat: String,
    pub error: Option<String>,
}
/// What the playback worker must do after observing a snapshot.
#[derive(Debug, PartialEq)]
pub enum Step {
    /// Load and start this track (prefetched clip when available).
    Next(u64),
    /// Repeat-one: the same clip is still loaded; seek to zero instead of decoding again.
    Restart,
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
        let revision = self.revision.wrapping_add(1);
        *self = Self {
            ids,
            repeat,
            enabled: true,
            revision,
            ..Default::default()
        };
    }
    fn upcoming(&self) -> Option<u64> {
        if self.repeat == "one" {
            return None;
        }
        match self.ids.get(self.index + 1) {
            Some(id) => Some(*id),
            None if self.repeat == "all" => self.ids.first().copied(),
            None => None,
        }
    }
    /// The track worth decoding now, if the current one is close to its end.
    pub fn wants_prefetch(&self, snap: &Snapshot) -> Option<PrefetchRequest> {
        let a = &snap.slots[0];
        let next = self.upcoming()?;
        let near_end = a.playing && a.duration > 0. && a.duration - a.position < PREFETCH_SECONDS;
        let current = self.ids.get(self.index).copied() == Some(a.id);
        let pending = self.prefetched.as_ref().map(|(id, _)| *id);
        (self.enabled
            && current
            && near_end
            && pending != Some(next)
            && self.failed_prefetch != Some(next))
        .then_some(PrefetchRequest {
            revision: self.revision,
            id: next,
        })
    }
    pub fn finish_prefetch(&mut self, request: PrefetchRequest, result: Result<Arc<Clip>, String>) {
        if self.enabled && self.revision == request.revision && self.upcoming() == Some(request.id)
        {
            match result {
                Ok(clip) => {
                    self.prefetched = Some((request.id, clip));
                    self.error = None;
                }
                Err(error) => {
                    // Do not repeatedly decode a missing/corrupt file at every 50 ms tick.
                    // A new queue session resets this; advance still gets one final attempt.
                    self.failed_prefetch = Some(request.id);
                    self.error = Some(error);
                }
            }
        }
    }
    pub fn take_prefetch(&mut self, id: u64) -> Option<Arc<Clip>> {
        match self.prefetched.take() {
            Some((pending, clip)) if pending == id => Some(clip),
            _ => None,
        }
    }
    /// Disabled queues release their decoded clip on the worker, never in the callback.
    pub fn release_if_idle(&mut self) -> Option<Arc<Clip>> {
        if self.enabled {
            None
        } else {
            self.prefetched.take().map(|(_, clip)| clip)
        }
    }
    pub fn finished(&mut self, snap: &Snapshot) -> Option<Step> {
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
            if self.repeat == "one" {
                return Some(Step::Restart);
            }
            self.index += 1;
            if self.index >= self.ids.len() {
                if self.repeat == "all" {
                    self.index = 0;
                } else {
                    self.enabled = false;
                    return None;
                }
            }
            return self.ids.get(self.index).copied().map(Step::Next);
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
    let Some(snapshot) = snapshot else {
        return;
    };
    let (revision, step, prefetch, idle) = {
        let mut queue = app.queue.lock().unwrap();
        let step = queue.finished(&snapshot);
        let prefetch = queue.wants_prefetch(&snapshot);
        (queue.revision, step, prefetch, queue.release_if_idle())
    };
    drop(idle);
    let result = match step {
        Some(Step::Restart) => (|| {
            let queue = app.queue.lock().unwrap();
            if !queue.enabled || queue.revision != revision {
                return Ok(());
            }
            let output = app.output.lock().unwrap();
            let handle = &output.as_ref().ok_or("Motor no disponible")?.handle;
            handle.control(0, Control::Trigger)
        })(),
        Some(Step::Next(id)) => advance(app, revision, id),
        None => {
            if let Some(request) = prefetch {
                // Decode without holding queue/output; a manual load may cancel the queue meanwhile.
                let decoded = {
                    let _decode = app.decode_gate.lock().unwrap();
                    {
                        let queue = app.queue.lock().unwrap();
                        if !queue.enabled
                            || queue.revision != request.revision
                            || queue.upcoming() != Some(request.id)
                        {
                            return;
                        }
                    }
                    app.decode_track(request.id)
                };
                app.queue.lock().unwrap().finish_prefetch(request, decoded);
            }
            Ok(())
        }
    };
    if let Err(e) = result {
        let mut q = app.queue.lock().unwrap();
        if q.revision == revision && q.enabled {
            q.error = Some(e);
            q.enabled = false;
        }
    }
}
fn advance(app: &AppState, revision: u64, id: u64) -> Result<(), String> {
    let ready = {
        let mut queue = app.queue.lock().unwrap();
        if !queue.enabled || queue.revision != revision {
            return Ok(());
        }
        queue.take_prefetch(id)
    };
    let _decode = app.decode_gate.lock().unwrap();
    {
        let queue = app.queue.lock().unwrap();
        if !queue.enabled || queue.revision != revision {
            return Ok(());
        }
    }
    let clip = match ready {
        Some(clip) => clip,
        None => app.decode_track(id)?,
    };
    // Manual loads cancel the queue while the background decode is running.
    let queue = app.queue.lock().unwrap();
    if !queue.enabled
        || queue.revision != revision
        || queue.ids.get(queue.index).copied() != Some(id)
    {
        return Ok(());
    }
    let output = app.output.lock().unwrap();
    let handle = &output.as_ref().ok_or("Motor no disponible")?.handle;
    handle.load_and_play(0, clip)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn clip(id: u64) -> Arc<Clip> {
        Arc::new(Clip {
            id,
            cues: Default::default(),
            samples: vec![],
            sample_rate: 48000,
            waveform: vec![],
            title: None,
            artist: None,
        })
    }
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
        assert_eq!(q.finished(&snap), Some(Step::Next(20)));
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
            ("one", Some(Step::Restart), true),
            ("all", Some(Step::Next(1)), true),
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
    #[test]
    fn prefetches_next_track_near_the_end_and_releases_when_stopped() {
        let (h, _) = curado_audio::engine(48000);
        let mut snap = h.snapshot();
        let mut q = Queue::default();
        q.set(vec![1, 2], "off".into());
        snap.slots[0].id = 1;
        snap.slots[0].duration = 100.;
        snap.slots[0].playing = true;
        snap.slots[0].position = 10.;
        assert_eq!(q.wants_prefetch(&snap), None, "too early");
        snap.slots[0].position = 90.;
        let request = q.wants_prefetch(&snap).unwrap();
        assert_eq!(request.id, 2);
        q.finish_prefetch(PrefetchRequest { id: 7, ..request }, Ok(clip(7)));
        assert_eq!(q.wants_prefetch(&snap), Some(request), "ignores stale ids");
        q.finish_prefetch(request, Ok(clip(2)));
        assert_eq!(q.wants_prefetch(&snap), None, "already decoded");
        assert!(q.take_prefetch(3).is_none(), "mismatch drops the clip");
        q.finish_prefetch(request, Ok(clip(2)));
        assert_eq!(q.take_prefetch(2).map(|c| c.id), Some(2));
        q.finish_prefetch(request, Ok(clip(2)));
        q.enabled = false;
        assert!(q.release_if_idle().is_some());
        let mut one = Queue::default();
        one.set(vec![1], "one".into());
        assert_eq!(
            one.wants_prefetch(&snap),
            None,
            "repeat one reuses the loaded clip"
        );
    }
    #[test]
    fn failed_prefetch_is_bounded_and_old_results_cannot_modify_a_new_queue() {
        let (h, _) = curado_audio::engine(48000);
        let mut snap = h.snapshot();
        snap.slots[0].id = 1;
        snap.slots[0].duration = 100.;
        snap.slots[0].position = 90.;
        snap.slots[0].playing = true;
        let mut q = Queue::default();
        q.set(vec![1, 2], "off".into());
        let old = q.wants_prefetch(&snap).unwrap();
        q.finish_prefetch(old, Err("missing file".into()));
        assert_eq!(q.error.as_deref(), Some("missing file"));
        for _ in 0..100 {
            assert!(q.wants_prefetch(&snap).is_none());
        }
        assert!(q.enabled, "the current track can still finish");
        q.set(vec![1, 2], "off".into());
        let current = q.wants_prefetch(&snap).unwrap();
        assert_ne!(
            current, old,
            "same track IDs do not imply the same queue session"
        );
        q.finish_prefetch(old, Err("late error".into()));
        q.finish_prefetch(old, Ok(clip(2)));
        assert!(q.error.is_none());
        assert!(q.prefetched.is_none());
        q.finish_prefetch(current, Ok(clip(2)));
        assert_eq!(q.take_prefetch(2).unwrap().id, 2);
        q.enabled = false;
        q.finish_prefetch(current, Err("cancelled".into()));
        assert!(q.error.is_none());
    }
}
