use crate::AppState;
use crossbeam_queue::ArrayQueue;
use curado_audio::Action;
use midir::{Ignore, MidiInput, MidiInputConnection};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicU64, Ordering::Relaxed},
    Arc, Mutex,
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    pub device: String,
    pub status: u8,
    pub channel: u8,
    pub number: u8,
    pub action: Action,
}
#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MidiStatus {
    pub connected: Option<String>,
    pub bindings: Vec<Binding>,
    pub learning: Option<Action>,
    pub last_message: Option<String>,
    pub error: Option<String>,
    pub dropped: u64,
}
#[derive(Clone, Copy)]
struct Raw {
    status: u8,
    number: u8,
    value: u8,
}
pub struct Midi {
    pub state: Arc<Mutex<MidiStatus>>,
    queue: Arc<ArrayQueue<Raw>>,
    dropped: Arc<AtomicU64>,
    connection: Option<MidiInputConnection<()>>,
}
/// On/off controls: the engine takes the state to set, and a button only knows it was pressed.
fn is_toggle(kind: &str) -> bool {
    matches!(kind, "slip" | "reverse" | "beatFxOn")
}
/// The value that flips a toggle from what the engine reports now.
pub fn flipped(snapshot: &curado_audio::Snapshot, action: &Action) -> Option<f64> {
    let on = match action.kind.as_str() {
        "slip" => snapshot.slots.get(action.slot)?.slip,
        "reverse" => snapshot.slots.get(action.slot)?.reverse,
        "beatFxOn" => snapshot.beat_fx.get(action.slot)?.on,
        _ => return None,
    };
    Some(if on { 0. } else { 1. })
}
impl Midi {
    pub fn new(bindings: Vec<Binding>) -> Self {
        Self {
            state: Arc::new(Mutex::new(MidiStatus {
                bindings,
                ..Default::default()
            })),
            queue: Arc::new(ArrayQueue::new(256)),
            dropped: Arc::new(AtomicU64::new(0)),
            connection: None,
        }
    }
    pub fn ports() -> Result<Vec<String>, String> {
        let midi = MidiInput::new("curado").map_err(|e| e.to_string())?;
        midi.ports()
            .iter()
            .map(|p| midi.port_name(p).map_err(|e| e.to_string()))
            .collect()
    }
    pub fn connect(&mut self, name: Option<String>) -> Result<(), String> {
        self.connection = None;
        self.state.lock().unwrap().connected = None;
        while self.queue.pop().is_some() {}
        let Some(name) = name else {
            return Ok(());
        };
        let mut midi = MidiInput::new("curado").map_err(|e| e.to_string())?;
        midi.ignore(Ignore::All);
        let port = midi
            .ports()
            .into_iter()
            .find(|p| midi.port_name(p).ok().as_ref() == Some(&name))
            .ok_or("Puerto MIDI no disponible")?;
        let queue = self.queue.clone();
        let dropped = self.dropped.clone();
        self.connection = Some(
            midi.connect(
                &port,
                "curado input",
                move |_, data, _| {
                    if data.len() >= 3
                        && matches!(data[0] & 0xf0, 0x90 | 0xb0)
                        && queue
                            .push(Raw {
                                status: data[0],
                                number: data[1],
                                value: data[2],
                            })
                            .is_err()
                    {
                        dropped.fetch_add(1, Relaxed);
                    }
                },
                (),
            )
            .map_err(|e| e.to_string())?,
        );
        self.state.lock().unwrap().connected = Some(name);
        Ok(())
    }
    pub fn tick(&self, app: &AppState) {
        self.state.lock().unwrap().dropped = self.dropped.load(Relaxed);
        for _ in 0..64 {
            let Some(raw) = self.queue.pop() else {
                break;
            };
            let mut state = self.state.lock().unwrap();
            let kind = raw.status & 0xf0;
            let channel = raw.status & 0xf;
            let device = state.connected.clone().unwrap_or_default();
            if kind == 0x80 || (kind == 0x90 && raw.value == 0) {
                // A release only matters to what is held, like a roll pad.
                let held = state.bindings.iter().find(|b| {
                    b.device == device
                        && b.status == 0x90
                        && b.channel == channel
                        && b.number == raw.number
                        && b.action.kind == "roll"
                });
                if let Some(slot) = held.map(|b| b.action.slot) {
                    let release = Action {
                        slot,
                        kind: "rollEnd".into(),
                        value: 0.,
                    };
                    if let Err(e) = app.action(release) {
                        state.error = Some(e);
                    }
                }
                continue;
            }
            state.last_message = Some(format!(
                "CH {} · {} {} · {}",
                channel + 1,
                if kind == 0xb0 { "CC" } else { "NOTE" },
                raw.number,
                raw.value
            ));
            if let Some(action) = state.learning.take() {
                state.bindings.retain(|b| {
                    !(b.device == device
                        && b.status == kind
                        && b.channel == channel
                        && b.number == raw.number)
                });
                state.bindings.push(Binding {
                    device,
                    status: kind,
                    channel,
                    number: raw.number,
                    action,
                });
                if let Err(e) = app.db.lock().unwrap().save("midi", &state.bindings) {
                    state.error = Some(e);
                }
                continue;
            }
            for binding in &state.bindings.clone() {
                if binding.device == device
                    && binding.status == kind
                    && binding.channel == channel
                    && binding.number == raw.number
                {
                    let mut action = binding.action.clone();
                    if is_toggle(&action.kind) {
                        // One press flips it, whether the control sends a note or a CC.
                        if kind == 0xb0 && raw.value < 64 {
                            continue;
                        }
                        let Some(value) = app.toggled(&action) else {
                            continue;
                        };
                        action.value = value;
                    } else if kind == 0xb0 {
                        action.value = match action.kind.as_str() {
                            "gain" => raw.value as f64 / 127. * 1.5,
                            // EQ range is 0..2 with unity at the centre detent (CC 64).
                            "low" | "mid" | "high" => raw.value as f64 / 64.,
                            "rate" => 0.8 + raw.value as f64 / 127. * 0.4,
                            "master" | "crossfader" | "filter" | "echo" | "highPass"
                            | "colorParam" | "beatFxDepth" => raw.value as f64 / 127.,
                            "sampler" => raw.value as f64 / 127. * 1.5,
                            "trim" => raw.value as f64 / 64.,
                            // Bipolar with its center at the detent (CC 64).
                            "color" => ((raw.value as f64 - 64.) / 63.).clamp(-1., 1.),
                            _ => {
                                if raw.value < 64 {
                                    continue;
                                }
                                action.value
                            }
                        };
                    }
                    if let Err(e) = app.action(action) {
                        state.error = Some(e);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curado_audio::{engine, Control};

    #[test]
    fn a_button_flips_what_the_engine_reports() {
        let (handle, mut renderer) = engine(48000);
        let action = |slot, kind: &str| Action {
            slot,
            kind: kind.into(),
            value: 0.,
        };
        let mut output = vec![0_f32; 512];
        let snapshot = |renderer: &mut curado_audio::Renderer, output: &mut [f32]| {
            renderer.render(output, 2);
            handle.snapshot()
        };
        let before = snapshot(&mut renderer, &mut output);
        for (slot, kind) in [(0, "slip"), (1, "reverse"), (2, "beatFxOn")] {
            assert!(is_toggle(kind));
            assert_eq!(flipped(&before, &action(slot, kind)), Some(1.));
        }
        handle.control(0, Control::Slip(true)).unwrap();
        handle.control(1, Control::Reverse(true)).unwrap();
        handle.control(2, Control::BeatFxOn(true)).unwrap();
        let after = snapshot(&mut renderer, &mut output);
        for (slot, kind) in [(0, "slip"), (1, "reverse"), (2, "beatFxOn")] {
            assert_eq!(flipped(&after, &action(slot, kind)), Some(0.));
        }
        assert_eq!(flipped(&after, &action(0, "gain")), None);
        assert!(!is_toggle("gain"));
    }
}
