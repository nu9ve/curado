//! MIDI controllers: connecting, reading, learning and presets. What a message means lives in
//! `curado-midi` (crates/midi); this module turns it into engine actions. See docs/MIDI.md.

use crate::AppState;
use crossbeam_queue::ArrayQueue;
use curado_audio::Action;
use curado_midi::detect::{Detected, Explorer, Seen, Want};
use curado_midi::input::{normalize, Gate, Mode};
use curado_midi::message::{Key, Kind, Message};
use curado_midi::preset::{self, Preset};
use midir::{Ignore, MidiInput, MidiInputConnection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering::Relaxed},
    Arc, Mutex,
};
use std::time::Instant;
use tauri::Emitter;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    pub device: String,
    /// 0x90 note, 0xB0 CC, 0xE0 pitch bend.
    pub status: u8,
    pub channel: u8,
    pub number: u8,
    pub action: Action,
    /// The CC with the low 7 bits of a 14-bit control.
    #[serde(default)]
    pub lsb: Option<u8>,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub invert: bool,
    /// Relative: what one step is worth. Seconds for `jog`, rows for `listMove`, else 0–1.
    #[serde(default)]
    pub scale: Option<f64>,
    /// Absolute: how far a lone message has to move (of 0–1) to count.
    #[serde(default)]
    pub deadband: Option<f64>,
    /// Tempo: ± around 1.
    #[serde(default)]
    pub range: Option<f64>,
    /// The control's name on the hardware, from a preset.
    #[serde(default)]
    pub label: Option<String>,
    /// The preset this came from; `None` when learned.
    #[serde(default)]
    pub preset: Option<String>,
}
impl Binding {
    fn key(&self) -> Option<Key> {
        Some(Key {
            kind: Kind::from_status(self.status)?,
            channel: self.channel,
            number: self.number,
        })
    }
    fn from_preset(device: &str, preset: &Preset) -> Vec<Self> {
        preset
            .mapped()
            .map(|(control, action)| {
                let key = control.key();
                Binding {
                    device: device.into(),
                    status: key.kind.status(),
                    channel: key.channel,
                    number: key.number,
                    action: Action {
                        slot: action.slot,
                        kind: action.kind.clone(),
                        value: action.value,
                    },
                    lsb: control.lsb,
                    mode: control.mode,
                    invert: control.invert,
                    scale: control.scale,
                    deadband: control.deadband,
                    range: control.range,
                    label: Some(control.label.clone()),
                    preset: Some(preset.id.clone()),
                }
            })
            .collect()
    }
    fn learned(device: &str, d: &Detected, action: Action) -> Self {
        let scale = match d.mode {
            Mode::Relative { .. } => Some(match action.kind.as_str() {
                "jog" => JOG_SECONDS,
                "listMove" => 1.,
                _ => 1. / 128.,
            }),
            _ => None,
        };
        Binding {
            device: device.into(),
            status: d.key.kind.status(),
            channel: d.key.channel,
            number: d.key.number,
            action,
            lsb: d.lsb,
            mode: d.mode,
            invert: false,
            scale,
            deadband: None,
            range: None,
            label: None,
            preset: None,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetInfo {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub controls: usize,
    pub mapped: usize,
    pub notes: Vec<String>,
}
impl From<&Preset> for PresetInfo {
    fn from(p: &Preset) -> Self {
        Self {
            id: p.id.clone(),
            name: p.name.clone(),
            vendor: p.vendor.clone(),
            controls: p.controls.len(),
            mapped: p.mapped().count(),
            notes: p.notes.clone(),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiPort {
    pub name: String,
    pub preset: Option<PresetInfo>,
}

/// What the interface sees 20 times a second. Bindings and the explored controls are read on
/// request (`midi_bindings`, `midi_seen`): a preset has a hundred of them.
#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MidiStatus {
    pub connected: Option<String>,
    /// Connected again whenever it is plugged in.
    pub remembered: Option<String>,
    #[serde(skip_serializing)]
    pub bindings: Vec<Binding>,
    pub bindings_version: u64,
    /// Bindings for the connected device.
    pub mapped: usize,
    /// The bundled preset for the connected device, if there is one.
    pub preset: Option<PresetInfo>,
    pub learning: Option<Action>,
    /// What the last learn found and bound.
    pub learned: Option<Detected>,
    /// The last gesture seen while not learning.
    pub detected: Option<Detected>,
    /// While listening for controls that move by themselves.
    pub baseline_left_ms: Option<u64>,
    /// Controls found moving by themselves.
    pub noisy: Vec<Key>,
    pub last_message: Option<String>,
    pub error: Option<String>,
    pub dropped: u64,
}
impl MidiStatus {
    pub(crate) fn changed_bindings(&mut self) {
        self.bindings_version += 1;
        self.mapped = match &self.connected {
            Some(device) => self.bindings.iter().filter(|b| &b.device == device).count(),
            None => 0,
        };
    }
}

#[derive(Clone, Copy)]
struct Raw {
    status: u8,
    number: u8,
    value: u8,
}

/// A local action for the interface: the list belongs to it, not to the engine.
#[derive(Clone, Serialize)]
struct Local {
    slot: usize,
    kind: &'static str,
    value: f64,
}

/// One jog step, in seconds of music, until a controller says otherwise: 720 steps a turn of
/// a record at 33⅓.
const JOG_SECONDS: f64 = 1.8 / 720.;
/// A jog turned without touching the top bends the tempo: this much bend per second of music
/// the step is worth, so 0.04 for one DDJ-SB step.
const BEND_PER_SECOND: f64 = 16.;
/// The bend lets go when the jog stops sending.
const BEND_HOLD_MS: u64 = 60;
/// How often curado looks for a remembered controller, or notices it gone.
const SCAN_MS: u64 = 2000;
const BASELINE_MS: u64 = 3000;

pub struct Midi {
    pub state: Arc<Mutex<MidiStatus>>,
    queue: Arc<ArrayQueue<Raw>>,
    dropped: Arc<AtomicU64>,
    connection: Option<MidiInputConnection<()>>,
    /// For list actions; set once the app runs.
    pub app: Option<tauri::AppHandle>,
    clock: Instant,
    explorer: Explorer,
    /// The high half of each 14-bit control, waiting for its low half.
    msb: HashMap<(u8, u8), u16>,
    gates: HashMap<(u8, u8, u8), Gate>,
    /// Position of a relative encoder bound to a parameter with ends.
    positions: HashMap<(u8, u8, u8), f64>,
    touched: [bool; 2],
    bend_until: [Option<u64>; 2],
    next_scan: u64,
    pending: Vec<Action>,
}

/// On/off controls: the engine takes the state to set, and a button only knows it was pressed.
fn is_toggle(kind: &str) -> bool {
    matches!(
        kind,
        "slip"
            | "reverse"
            | "beatFxOn"
            | "sync"
            | "keyLock"
            | "channelCue"
            | "quantize"
            | "masterCue"
            | "autoGain"
    )
}
/// The value that flips a toggle from what the engine reports now.
pub fn flipped(snapshot: &curado_audio::Snapshot, action: &Action) -> Option<f64> {
    let slot = || snapshot.slots.get(action.slot);
    let on = match action.kind.as_str() {
        "slip" => slot()?.slip,
        "reverse" => slot()?.reverse,
        "beatFxOn" => snapshot.beat_fx.get(action.slot)?.on,
        "sync" => slot()?.sync,
        "keyLock" => slot()?.tuning.key_lock,
        "channelCue" => slot()?.cue,
        "quantize" => slot()?.quantize,
        "masterCue" => snapshot.cue_master,
        "autoGain" => snapshot.auto_gain,
        _ => return None,
    };
    Some(if on { 0. } else { 1. })
}
/// The engine value for a continuous action at `x` of its travel, or `None` for one that is not.
pub fn continuous(kind: &str, x: f64, range: Option<f64>) -> Option<f64> {
    Some(match kind {
        "gain" | "sampler" => x * 1.5,
        // Unity at the centre detent.
        "low" | "mid" | "high" | "trim" => x * 2.,
        "rate" => 1. + (x * 2. - 1.) * range.unwrap_or(0.2).clamp(0.01, 0.5),
        "color" => (x * 2. - 1.).clamp(-1., 1.),
        "master" | "crossfader" | "filter" | "echo" | "highPass" | "colorParam" | "beatFxDepth"
        | "cueMix" | "cueLevel" => x,
        _ => return None,
    })
}
/// What a control may be learned for: an engine action, or one of MIDI's own (the jog, which
/// scratches or bends depending on touch, and the list, which belongs to the interface).
pub fn validate_target(action: &Action) -> Result<(), String> {
    match action.kind.as_str() {
        "jog" if action.slot < 2 => Ok(()),
        "listMove" if action.value != 0. && action.value.abs() <= 64. => Ok(()),
        "listLoad" if action.slot < 2 => Ok(()),
        "jog" | "listMove" | "listLoad" => Err("Control o valor inválido".into()),
        _ => action.validate().map(|_| ()),
    }
}
/// Which kind of control learning should prefer for an action.
fn wanted(kind: &str) -> Want {
    if continuous(kind, 0., None).is_some() || matches!(kind, "jog" | "listMove") {
        Want::Continuous
    } else {
        Want::Button
    }
}

impl Midi {
    pub fn new(bindings: Vec<Binding>, remembered: Option<String>) -> Self {
        Self {
            state: Arc::new(Mutex::new(MidiStatus {
                bindings,
                remembered,
                ..Default::default()
            })),
            queue: Arc::new(ArrayQueue::new(256)),
            dropped: Arc::new(AtomicU64::new(0)),
            connection: None,
            app: None,
            clock: Instant::now(),
            explorer: Explorer::default(),
            msb: HashMap::new(),
            gates: HashMap::new(),
            positions: HashMap::new(),
            touched: [false; 2],
            bend_until: [None; 2],
            next_scan: 0,
            pending: Vec::new(),
        }
    }
    fn now(&self) -> u64 {
        self.clock.elapsed().as_millis() as u64
    }
    pub fn ports() -> Result<Vec<String>, String> {
        let midi = MidiInput::new("curado").map_err(|e| e.to_string())?;
        midi.ports()
            .iter()
            .map(|p| midi.port_name(p).map_err(|e| e.to_string()))
            .collect()
    }
    pub fn ports_with_presets() -> Result<Vec<MidiPort>, String> {
        Ok(Self::ports()?
            .into_iter()
            .map(|name| MidiPort {
                preset: preset::for_port(&name).as_ref().map(PresetInfo::from),
                name,
            })
            .collect())
    }
    /// Opens a port, or closes the current one with `None`. A controller with a bundled preset
    /// and nothing mapped yet gets the preset. Saving is the caller's: `remember`.
    pub fn connect(&mut self, name: Option<String>, app: &AppState) -> Result<(), String> {
        self.connection = None;
        while self.queue.pop().is_some() {}
        self.explorer.reset();
        self.msb.clear();
        self.gates.clear();
        self.touched = [false; 2];
        let mut state = self.state.lock().unwrap();
        state.connected = None;
        state.preset = None;
        state.changed_bindings();
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
                    // Copy three bytes and leave: notes, CCs and pitch bend only.
                    if data.len() >= 3
                        && matches!(data[0] & 0xf0, 0x80 | 0x90 | 0xb0 | 0xe0)
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
            .map_err(|e| format!("{e}. Cierra otro programa DJ que lo tenga abierto."))?,
        );
        state.connected = Some(name.clone());
        state.error = None;
        if let Some(found) = preset::for_port(&name) {
            state.preset = Some(PresetInfo::from(&found));
            if !state.bindings.iter().any(|b| b.device == name) {
                state.bindings.extend(Binding::from_preset(&name, &found));
                app.db.lock().unwrap().save("midi", &state.bindings)?;
            }
            self.pending = found
                .init
                .iter()
                .map(|a| Action {
                    slot: a.slot,
                    kind: a.kind.clone(),
                    value: a.value,
                })
                .collect();
        }
        state.changed_bindings();
        Ok(())
    }
    /// Replaces this device's mappings with its bundled preset.
    pub fn apply_preset(&mut self, app: &AppState) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        let device = state
            .connected
            .clone()
            .ok_or("Conecta primero un dispositivo MIDI")?;
        let found = preset::for_port(&device).ok_or("No hay un preset para este equipo")?;
        state.bindings.retain(|b| b.device != device);
        state.bindings.extend(Binding::from_preset(&device, &found));
        self.gates.clear();
        self.positions.clear();
        state.changed_bindings();
        app.db.lock().unwrap().save("midi", &state.bindings)
    }
    /// Forgets every mapping of the connected device.
    pub fn clear_device(&mut self, app: &AppState) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        let device = state
            .connected
            .clone()
            .ok_or("Conecta primero un dispositivo MIDI")?;
        state.bindings.retain(|b| b.device != device);
        state.changed_bindings();
        app.db.lock().unwrap().save("midi", &state.bindings)
    }
    pub fn start_baseline(&mut self) {
        let now = self.now();
        self.explorer.start_baseline(now, BASELINE_MS);
        let mut state = self.state.lock().unwrap();
        state.baseline_left_ms = Some(BASELINE_MS);
        state.detected = None;
    }
    pub fn seen(&self) -> Vec<Seen> {
        self.explorer.seen()
    }
    /// A new learn starts from nothing, so a gesture already under way is not bound.
    pub fn learn(&mut self, action: Option<Action>) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if action.is_some() && state.connected.is_none() {
            return Err("Conecta primero un dispositivo MIDI".into());
        }
        self.explorer.reset();
        state.learning = action;
        state.learned = None;
        Ok(())
    }

    pub fn tick(&mut self, app: &AppState) {
        let now = self.now();
        if now >= self.next_scan {
            self.next_scan = now + SCAN_MS;
            self.scan(app);
        }
        for action in std::mem::take(&mut self.pending) {
            // Without an engine the init waits; with one, a refusal is shown.
            if let Err(e) = app.action(action) {
                self.state.lock().unwrap().error = Some(e);
            }
        }
        let shared = self.state.clone();
        let mut state = shared.lock().unwrap();
        state.dropped = self.dropped.load(Relaxed);
        let baseline = self.explorer.baseline_left_ms(now);
        state.baseline_left_ms = baseline;
        for _ in 0..64 {
            let Some(raw) = self.queue.pop() else {
                break;
            };
            let Some(message) = Message::parse(&[raw.status, raw.number, raw.value]) else {
                continue;
            };
            state.last_message = Some(format!("{} · {}", message.key.label(), message.value));
            self.explorer.feed(now, message);
            // While learning or listening for noise, the controller drives nothing.
            if state.learning.is_some() || baseline.is_some() {
                continue;
            }
            self.dispatch(message, now, app, &mut state);
        }
        if baseline.is_none() {
            let noisy: Vec<Key> = self.explorer.noisy().iter().copied().collect();
            if noisy != state.noisy {
                state.noisy = noisy;
            }
        }
        match state.learning.clone() {
            Some(action) => {
                if let Some(d) = self.explorer.poll(now, Some(wanted(&action.kind))) {
                    self.bind(&mut state, d, action, app);
                }
            }
            None => {
                if let Some(d) = self.explorer.poll(now, None) {
                    state.detected = Some(d);
                }
            }
        }
        for slot in 0..2 {
            if self.bend_until[slot].is_some_and(|until| now >= until) {
                self.bend_until[slot] = None;
                self.send(app, &mut state, slot, "jogBend", 0.);
            }
        }
    }

    /// Notices a remembered controller plugged in or pulled out.
    fn scan(&mut self, app: &AppState) {
        let (remembered, connected) = {
            let state = self.state.lock().unwrap();
            (state.remembered.clone(), state.connected.clone())
        };
        let Some(wanted) = remembered else {
            return;
        };
        let Ok(ports) = Self::ports() else {
            return;
        };
        let present = ports.contains(&wanted);
        if connected.is_some() && !present {
            self.connection = None;
            let mut state = self.state.lock().unwrap();
            state.connected = None;
            state.preset = None;
            state.changed_bindings();
            state.error = Some(format!(
                "{wanted} se desconectó. curado lo conectará de nuevo al volver."
            ));
        } else if connected.is_none() && present {
            if let Err(e) = self.connect(Some(wanted), app) {
                self.state.lock().unwrap().error = Some(e);
            }
        }
    }

    fn bind(&mut self, state: &mut MidiStatus, d: Detected, action: Action, app: &AppState) {
        let Some(device) = state.connected.clone() else {
            return;
        };
        let lsb_key = d.lsb.map(|lsb| (d.key.channel, lsb));
        // A control does one thing: learning it again replaces what it did, here or as an LSB.
        state.bindings.retain(|b| {
            let same = b.device == device
                && b.channel == d.key.channel
                && b.status == d.key.kind.status()
                && b.number == d.key.number;
            let overlaps = b.device == device
                && b.status == 0xb0
                && d.key.kind == Kind::Cc
                && (lsb_key == Some((b.channel, b.number))
                    || b.lsb
                        .is_some_and(|l| (b.channel, l) == (d.key.channel, d.key.number)));
            !(same || overlaps)
        });
        state.bindings.push(Binding::learned(&device, &d, action));
        state.learning = None;
        state.learned = Some(d);
        state.changed_bindings();
        if let Err(e) = app.db.lock().unwrap().save("midi", &state.bindings) {
            state.error = Some(e);
        }
    }

    fn dispatch(&mut self, message: Message, now: u64, app: &AppState, state: &mut MidiStatus) {
        let Some(device) = state.connected.clone() else {
            return;
        };
        let key = message.key;
        let mut hits: Vec<(Binding, u16, u8)> = Vec::new();
        for b in state
            .bindings
            .iter()
            .filter(|b| b.device == device && b.channel == key.channel)
        {
            if key.kind == Kind::Cc && b.status == 0xb0 && b.lsb == Some(key.number) {
                // The low half completes the value the high half started.
                let high = self.msb.get(&(b.channel, b.number)).copied().unwrap_or(0);
                hits.push((b.clone(), high << 7 | message.value, 14));
            } else if b.key() == Some(key) {
                if b.lsb.is_some() {
                    self.msb.insert((b.channel, b.number), message.value);
                    continue;
                }
                let bits = if key.kind == Kind::PitchBend { 14 } else { 7 };
                hits.push((b.clone(), message.value, bits));
            }
        }
        for (binding, value, bits) in hits {
            let mode = match binding.mode {
                Mode::Auto if key.kind == Kind::Note => Mode::Button,
                Mode::Auto if continuous(&binding.action.kind, 0., None).is_some() => {
                    Mode::Absolute
                }
                Mode::Auto => Mode::Button,
                mode => mode,
            };
            match mode {
                Mode::Button => {
                    let pressed = match key.kind {
                        Kind::Note => message.value > 0,
                        _ => value >= 1 << (bits - 1),
                    };
                    self.button(&binding, pressed, app, state);
                }
                Mode::Absolute => self.absolute(&binding, value, bits, now, app, state),
                Mode::Relative { encoding } => {
                    let delta = encoding.delta(value);
                    if delta != 0 {
                        self.relative(&binding, delta, now, app, state);
                    }
                }
                Mode::Auto => {}
            }
        }
    }

    fn button(&mut self, b: &Binding, pressed: bool, app: &AppState, state: &mut MidiStatus) {
        let slot = b.action.slot;
        let kind = b.action.kind.as_str();
        // Held controls: what happens on release matters.
        let release = match kind {
            "roll" => Some("rollEnd"),
            "releaseFx" => Some("releaseEnd"),
            "jogTouch" => Some("jogTouch"),
            _ => None,
        };
        if kind == "jogTouch" && slot < 2 {
            self.touched[slot] = pressed;
            self.bend_until[slot] = None;
            return self.send(app, state, slot, "jogTouch", if pressed { 1. } else { 0. });
        }
        if !pressed {
            if let Some(release) = release {
                self.send(app, state, slot, release, 0.);
            }
            return;
        }
        match kind {
            "listMove" => self.local(slot, "listMove", b.action.value),
            "listLoad" => self.local(slot, "listLoad", 0.),
            _ if is_toggle(kind) => {
                if let Some(value) = app.toggled(&b.action) {
                    self.send(app, state, slot, kind, value);
                }
            }
            _ => {
                if let Err(e) = app.action(b.action.clone()) {
                    state.error = Some(e);
                }
            }
        }
    }

    fn absolute(
        &mut self,
        b: &Binding,
        raw: u16,
        bits: u8,
        now: u64,
        app: &AppState,
        state: &mut MidiStatus,
    ) {
        let mut x = normalize(raw, bits);
        if b.invert {
            x = 1. - x;
        }
        let kind = b.action.kind.as_str();
        let Some(value) = continuous(kind, x, b.range) else {
            // A button-like action on a fader: it acts as it crosses the middle.
            if x >= 0.5 {
                self.button(b, true, app, state);
            }
            return;
        };
        // A light dead band by default on 14 bits, where a resting fader still whispers.
        let deadband = b.deadband.unwrap_or(if bits == 14 { 0.002 } else { 0. });
        if deadband > 0. {
            let gate = self
                .gates
                .entry((b.status, b.channel, b.number))
                .or_default();
            if !gate.pass(x, now, deadband) {
                return;
            }
        }
        self.send(app, state, b.action.slot, kind, value);
    }

    fn relative(
        &mut self,
        b: &Binding,
        delta: i32,
        now: u64,
        app: &AppState,
        state: &mut MidiStatus,
    ) {
        let slot = b.action.slot;
        let kind = b.action.kind.as_str();
        match kind {
            "jog" if slot < 2 => {
                let seconds = delta as f64 * b.scale.unwrap_or(JOG_SECONDS);
                if self.touched[slot] {
                    self.send(app, state, slot, "jogMove", seconds.clamp(-8., 8.));
                } else {
                    let bend = (seconds * BEND_PER_SECOND).clamp(-0.5, 0.5);
                    self.bend_until[slot] = Some(now + BEND_HOLD_MS);
                    self.send(app, state, slot, "jogBend", bend);
                }
            }
            "listMove" => {
                let rows = (delta as f64 * b.scale.unwrap_or(1.)).round();
                if rows != 0. {
                    self.local(slot, "listMove", rows);
                }
            }
            _ => {
                // An endless knob on a parameter with ends: curado keeps where it is.
                let at = self
                    .positions
                    .entry((b.status, b.channel, b.number))
                    .or_insert(0.5);
                *at = (*at + delta as f64 * b.scale.unwrap_or(1. / 128.)).clamp(0., 1.);
                if let Some(value) = continuous(kind, *at, b.range) {
                    self.send(app, state, slot, kind, value);
                }
            }
        }
    }

    fn send(&self, app: &AppState, state: &mut MidiStatus, slot: usize, kind: &str, value: f64) {
        let action = Action {
            slot,
            kind: kind.into(),
            value,
        };
        if let Err(e) = app.action(action) {
            state.error = Some(e);
        }
    }

    fn local(&self, slot: usize, kind: &'static str, value: f64) {
        if let Some(app) = &self.app {
            let _ = app.emit("midi:local", Local { slot, kind, value });
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
        for (slot, kind) in [(0, "slip"), (1, "reverse"), (2, "beatFxOn"), (0, "sync")] {
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
        // Key lock starts on.
        assert_eq!(flipped(&after, &action(0, "keyLock")), Some(0.));
        assert_eq!(flipped(&after, &action(0, "gain")), None);
        assert!(!is_toggle("gain"));
    }

    #[test]
    fn every_toggle_is_an_engine_action_with_both_states() {
        for kind in [
            "slip",
            "reverse",
            "beatFxOn",
            "sync",
            "keyLock",
            "channelCue",
            "quantize",
            "masterCue",
            "autoGain",
        ] {
            for value in [0., 1.] {
                let action = Action {
                    slot: 0,
                    kind: kind.into(),
                    value,
                };
                action.validate().unwrap_or_else(|e| panic!("{kind}: {e}"));
            }
        }
    }

    #[test]
    fn continuous_values_rest_on_their_detents_and_stay_in_range() {
        assert_eq!(continuous("low", 0.5, None), Some(1.));
        assert_eq!(continuous("color", 0.5, None), Some(0.));
        assert_eq!(continuous("rate", 0.5, Some(0.1)), Some(1.));
        assert_eq!(continuous("rate", 1., Some(0.1)), Some(1.1));
        assert_eq!(continuous("toggle", 1., None), None);
        for kind in [
            "gain",
            "sampler",
            "low",
            "mid",
            "high",
            "trim",
            "rate",
            "color",
            "master",
            "crossfader",
            "filter",
            "echo",
            "highPass",
            "colorParam",
            "beatFxDepth",
            "cueMix",
            "cueLevel",
        ] {
            for x in [0., 0.25, 0.5, 0.75, 1.] {
                let value = continuous(kind, x, None).unwrap();
                let slot = if kind == "beatFxDepth" { 2 } else { 0 };
                Action {
                    slot,
                    kind: kind.into(),
                    value,
                }
                .validate()
                .unwrap_or_else(|e| panic!("{kind} at {x}: {e}"));
            }
        }
    }

    #[test]
    fn every_preset_action_is_one_the_engine_or_the_list_takes() {
        for preset in preset::bundled() {
            let bindings = Binding::from_preset("x", &preset);
            assert_eq!(bindings.len(), preset.mapped().count());
            for b in bindings.iter().chain(
                preset
                    .init
                    .iter()
                    .map(|a| Binding {
                        action: Action {
                            slot: a.slot,
                            kind: a.kind.clone(),
                            value: a.value,
                        },
                        ..bindings[0].clone()
                    })
                    .collect::<Vec<_>>()
                    .iter(),
            ) {
                let kind = b.action.kind.as_str();
                if matches!(kind, "jog" | "listMove" | "listLoad") {
                    continue;
                }
                if kind == "jogTouch" || is_toggle(kind) {
                    for value in [0., 1.] {
                        Action {
                            value,
                            ..b.action.clone()
                        }
                        .validate()
                        .unwrap_or_else(|e| panic!("{}: {e}", b.label.clone().unwrap_or_default()));
                    }
                    continue;
                }
                let action = match continuous(kind, 0.5, b.range) {
                    Some(value) => Action {
                        value,
                        ..b.action.clone()
                    },
                    None => b.action.clone(),
                };
                action
                    .validate()
                    .unwrap_or_else(|e| panic!("{}: {e}", b.label.clone().unwrap_or_default()));
            }
        }
    }

    #[test]
    fn bindings_saved_before_modes_still_read() {
        let old = r#"[{"device":"X","status":176,"channel":0,"number":7,"action":{"slot":0,"kind":"low","value":0}}]"#;
        let bindings: Vec<Binding> = serde_json::from_str(old).unwrap();
        assert_eq!(bindings[0].mode, Mode::Auto);
        assert_eq!(bindings[0].lsb, None);
    }

    #[test]
    fn learning_asks_for_the_right_kind_of_control() {
        assert_eq!(wanted("toggle"), Want::Button);
        assert_eq!(wanted("jogTouch"), Want::Button);
        assert_eq!(wanted("low"), Want::Continuous);
        assert_eq!(wanted("jog"), Want::Continuous);
        assert_eq!(wanted("listMove"), Want::Continuous);
    }
}
