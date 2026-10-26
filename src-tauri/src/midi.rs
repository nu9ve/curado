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
        let midi = MidiInput::new("Curado").map_err(|e| e.to_string())?;
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
        let mut midi = MidiInput::new("Curado").map_err(|e| e.to_string())?;
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
                "Curado input",
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
            if kind == 0x90 && raw.value == 0 {
                continue;
            }
            let device = state.connected.clone().unwrap_or_default();
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
                    if kind == 0xb0 {
                        action.value = match action.kind.as_str() {
                            "gain" | "low" | "mid" | "high" => raw.value as f64 / 127. * 1.5,
                            "rate" => 0.8 + raw.value as f64 / 127. * 0.4,
                            "master" | "crossfader" | "filter" | "echo" => raw.value as f64 / 127.,
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
