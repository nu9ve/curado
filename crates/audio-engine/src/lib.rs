pub mod analysis;
pub mod annotations;
pub mod decode;
mod dsp;
pub mod music;
pub mod output;
pub mod tuning;
use crossbeam_queue::ArrayQueue;
use decode::Clip;
use dsp::{Effects, Params};
use music::MusicAnalysis;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering::Relaxed},
    Arc,
};
use tuning::{MusicMeter, TunedClip, Tuning, TuningMeter};

pub const SLOTS: usize = 6;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub slot: usize,
    pub kind: String,
    #[serde(default)]
    pub value: f64,
}
#[derive(Clone, Copy, Debug)]
pub enum Control {
    Play,
    Pause,
    Toggle,
    Seek(f64),
    Nudge(f64),
    Cue(usize),
    SetCue(usize),
    Loop,
    LoopIn,
    LoopOut,
    Trigger,
    PlayBoth,
    PauseBoth,
    CueBoth(usize),
    Quantize(bool),
    LoopBeats(f64),
    Sync(bool),
    MasterDeck,
    Tempo(f64),
    Transpose(i8),
    TransposeStep(i8),
    KeyLock(bool),
    KeySync,
    RetryTuning,
    HighPass(f32),
    EchoBeats(f32),
    Gain(f32),
    Rate(f32),
    Low(f32),
    Mid(f32),
    High(f32),
    Filter(f32),
    Echo(f32),
    Crossfader(f32),
    Master(f32),
    Panic,
    SetMode(bool),
}
impl Action {
    pub fn validate(&self) -> Result<Control, String> {
        if self.slot >= SLOTS || !self.value.is_finite() {
            return Err("Control o valor inválido".into());
        }
        let v = self.value as f32;
        Ok(match self.kind.as_str() {
            "play" => Control::Play,
            "pause" => Control::Pause,
            "toggle" => Control::Toggle,
            "seek" if self.value >= 0. => Control::Seek(self.value),
            "nudge" if self.value.abs() <= 1. => Control::Nudge(self.value),
            "cue" if (0.0..3.0).contains(&self.value) && self.value.fract() == 0. => {
                Control::Cue(self.value as usize)
            }
            "setCue" if (0.0..3.0).contains(&self.value) && self.value.fract() == 0. => {
                Control::SetCue(self.value as usize)
            }
            "loop" => Control::Loop,
            "loopIn" => Control::LoopIn,
            "loopOut" => Control::LoopOut,
            "trigger" => Control::Trigger,
            "playBoth" => Control::PlayBoth,
            "pauseBoth" => Control::PauseBoth,
            "cueBoth" if (0.0..3.0).contains(&self.value) && self.value.fract() == 0. => {
                Control::CueBoth(self.value as usize)
            }
            "quantize" if matches!(self.value, 0.0 | 1.0) => Control::Quantize(self.value == 1.),
            "loopBeats" if [1., 2., 4., 8., 16.].contains(&self.value) => {
                Control::LoopBeats(self.value)
            }
            "sync" if self.slot < 2 && matches!(self.value, 0.0 | 1.0) => {
                Control::Sync(self.value == 1.)
            }
            "masterDeck" if self.slot < 2 => Control::MasterDeck,
            "tempo" if self.slot < 2 && (40.0..=300.0).contains(&self.value) => {
                Control::Tempo(self.value)
            }
            "transpose"
                if self.slot < 2
                    && (-12.0..=12.0).contains(&self.value)
                    && self.value.fract() == 0. =>
            {
                Control::Transpose(self.value as i8)
            }
            "transposeStep" if self.slot < 2 && matches!(self.value, -1.0 | 1.0) => {
                Control::TransposeStep(self.value as i8)
            }
            "keyLock" if self.slot < 2 && matches!(self.value, 0.0 | 1.0) => {
                Control::KeyLock(self.value == 1.)
            }
            "keySync" if self.slot < 2 => Control::KeySync,
            "retryTuning" if self.slot < 2 => Control::RetryTuning,
            "gain" if (0.0..=1.5).contains(&v) => Control::Gain(v),
            "rate" if (0.5..=1.5).contains(&v) => Control::Rate(v),
            "low" if (0.0..=2.0).contains(&v) => Control::Low(v),
            "mid" if (0.0..=2.0).contains(&v) => Control::Mid(v),
            "high" if (0.0..=2.0).contains(&v) => Control::High(v),
            "filter" if (0.0..=1.0).contains(&v) => Control::Filter(v),
            "echo" if (0.0..=1.0).contains(&v) => Control::Echo(v),
            "highPass" if (0.0..=1.0).contains(&v) => Control::HighPass(v),
            "echoBeats" if [0.25, 0.5, 0.75, 1., 2.].contains(&v) => Control::EchoBeats(v),
            "crossfader" if (0.0..=1.0).contains(&v) => Control::Crossfader(v),
            "master" if (0.0..=1.0).contains(&v) => Control::Master(v),
            "panic" => Control::Panic,
            _ => return Err(format!("Control fuera de rango: {}", self.kind)),
        })
    }
}
pub enum Command {
    Load(usize, Arc<Clip>, u64, bool),
    Control(usize, Control),
    TuneBatch(u64, [Option<TunedClip>; 2]),
    Music(u64, MusicAnalysis),
}
#[derive(Default)]
struct Meter {
    id: AtomicU64,
    load_ticket: AtomicU64,
    position: AtomicU64,
    duration: AtomicU64,
    playing: AtomicBool,
    looping: AtomicBool,
    peak: AtomicU32,
    gain: AtomicU32,
    rate: AtomicU32,
    low: AtomicU32,
    mid: AtomicU32,
    high: AtomicU32,
    filter: AtomicU32,
    echo: AtomicU32,
    high_pass: AtomicU32,
    echo_beats: AtomicU32,
    cues: [AtomicU64; 3],
    loop_in: AtomicU64,
    loop_out: AtomicU64,
    music: MusicMeter,
    tuning: TuningMeter,
    applied_tuning: TuningMeter,
    tune_revision: AtomicU64,
    tuned_revision: AtomicU64,
    quantize: AtomicBool,
    sync: AtomicBool,
    loop_beats: AtomicU64,
    queued: AtomicBool,
}
pub struct Shared {
    commands: ArrayQueue<Command>,
    retired: ArrayQueue<Arc<Clip>>,
    meters: [Meter; SLOTS],
    pending_loads: AtomicUsize,
    next_load: AtomicU64,
    pub stream_error: AtomicBool,
    pub callbacks: AtomicU64,
    pub clipped: AtomicU64,
    peak: AtomicU32,
    rms: AtomicU32,
    waveform: [AtomicU32; 64],
    crossfader: AtomicU32,
    master: AtomicU32,
    master_deck: AtomicUsize,
    next_tuning: AtomicU64,
    tuning_ack: AtomicU64,
}
#[derive(Clone)]
pub struct EngineHandle(Arc<Shared>);
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotSnapshot {
    pub id: u64,
    pub position: f64,
    pub duration: f64,
    pub playing: bool,
    pub looping: bool,
    pub peak: f32,
    pub gain: f32,
    pub rate: f32,
    pub low: f32,
    pub mid: f32,
    pub high: f32,
    pub filter: f32,
    pub echo: f32,
    pub high_pass: f32,
    pub echo_beats: f32,
    pub cues: [f64; 3],
    pub loop_in: f64,
    pub loop_out: f64,
    pub load_ticket: u64,
    pub music: MusicAnalysis,
    pub tuning: Tuning,
    pub applied_tuning: Tuning,
    pub tune_revision: u64,
    pub tuned_revision: u64,
    pub quantize: bool,
    pub sync: bool,
    pub loop_beats: f64,
    pub queued: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub slots: Vec<SlotSnapshot>,
    pub peak: f32,
    pub rms: f32,
    pub waveform: Vec<f32>,
    pub crossfader: f32,
    pub master: f32,
    pub stream_error: bool,
    pub callbacks: u64,
    pub clipped: u64,
    pub master_deck: usize,
}
impl EngineHandle {
    pub fn action(&self, action: Action) -> Result<(), String> {
        let c = action.validate()?;
        if let Control::Rate(rate) = c {
            let s = self.snapshot();
            if action.slot == s.master_deck {
                if let Some(base) = s.slots[action.slot].music.bpm {
                    for d in &s.slots[..2] {
                        if d.sync
                            && d.music
                                .bpm
                                .is_some_and(|b| !(0.5..=1.5).contains(&(base * rate as f64 / b)))
                        {
                            return Err("Velocidad fuera de rango para el deck sincronizado".into());
                        }
                    }
                }
            }
        }
        if let Control::LoopBeats(beats) = c {
            let s = self.snapshot();
            let d = &s.slots[action.slot];
            if d.quantize
                && d.music
                    .bpm
                    .is_some_and(|b| d.duration + 1e-6 < d.music.beat_offset + beats * 60. / b)
            {
                return Err(
                    "La pista no tiene tantos beats completos; elige un loop más corto".into(),
                );
            }
        }
        if matches!(
            c,
            Control::Sync(true) | Control::Tempo(_) | Control::KeySync
        ) {
            let s = self.snapshot();
            let target = &s.slots[action.slot];
            let master = &s.slots[s.master_deck];
            match c {
                Control::Sync(true) if action.slot==s.master_deck || target.music.bpm.is_none() || master.music.bpm.is_none() => return Err("Elige otro deck master y corrige el BPM de ambas pistas antes de sincronizar".into()),
                Control::Sync(true) => {
                    let rate=master.music.bpm.unwrap()*master.tuning.rate as f64/target.music.bpm.unwrap();
                    if !(0.5..=1.5).contains(&rate) { return Err("El sync necesita una velocidad fuera de ±50 %. Corrige BPM ×2/÷2 o elige otra pista".into()); }
                }
                Control::Tempo(bpm) => {
                    if target.sync{return Err("Cambia el tempo desde el deck master o desactiva SYNC".into());}
                    let base=target.music.bpm.ok_or("Corrige o analiza el BPM de esta pista")?;
                    if !(0.5..=1.5).contains(&(bpm/base)) { return Err("Tempo fuera del rango de velocidad ±50 %".into()); }
                    for slot in &s.slots[..2] {
                        if slot.sync && slot.music.bpm.is_some_and(|b| !(0.5..=1.5).contains(&(bpm/b))) { return Err("El tempo dejaría un deck sincronizado fuera de rango".into()); }
                    }
                }
                Control::KeySync if action.slot==s.master_deck || target.music.key.is_none() || master.music.key.is_none() => return Err("Elige otro deck master y corrige la tonalidad de ambas pistas".into()),
                Control::KeySync if !master.tuning.key_lock => return Err("Activa key lock en el master antes de igualar tonalidades".into()),
                _=>{}
            }
        }
        self.control(action.slot, c)
    }
    pub fn set_music(&self, id: u64, music: MusicAnalysis) -> Result<(), String> {
        if !music.valid() {
            return Err("Análisis musical inválido".into());
        }
        self.0
            .commands
            .push(Command::Music(id, music))
            .map_err(|_| "Cola de audio llena".into())
    }
    pub fn tuning_acknowledged(&self, ticket: u64) -> bool {
        self.0.tuning_ack.load(std::sync::atomic::Ordering::Acquire) >= ticket
    }
    pub fn apply_tuning(&self, batch: [Option<TunedClip>; 2]) -> Result<u64, String> {
        let count = batch.iter().flatten().count();
        if count == 0 {
            return Ok(0);
        }
        let mut pending = self.0.pending_loads.load(Relaxed);
        loop {
            if pending + count > 2 {
                return Err("El motor todavía prepara otra carga".into());
            }
            match self.0.pending_loads.compare_exchange_weak(
                pending,
                pending + count,
                Relaxed,
                Relaxed,
            ) {
                Ok(_) => break,
                Err(p) => pending = p,
            }
        }
        let ticket = self.0.next_tuning.fetch_add(1, Relaxed) + 1;
        if self
            .0
            .commands
            .push(Command::TuneBatch(ticket, batch))
            .is_err()
        {
            self.0.pending_loads.fetch_sub(count, Relaxed);
            return Err("Cola de audio llena".into());
        }
        Ok(ticket)
    }
    pub fn control(&self, slot: usize, control: Control) -> Result<(), String> {
        if slot >= SLOTS
            || matches!(control, Control::Cue(i)|Control::SetCue(i)|Control::CueBoth(i) if i>=3)
            || (slot >= 2
                && matches!(
                    control,
                    Control::MasterDeck
                        | Control::Sync(_)
                        | Control::Tempo(_)
                        | Control::Transpose(_)
                        | Control::TransposeStep(_)
                        | Control::KeyLock(_)
                        | Control::KeySync
                        | Control::RetryTuning
                ))
        {
            return Err("Deck o cue inválido".into());
        }
        self.0
            .commands
            .push(Command::Control(slot, control))
            .map_err(|_| "Cola de audio llena; vuelve a intentar".into())
    }
    pub fn tuning_current(&self, slot: usize, ticket: u64, revision: u64, tuning: Tuning) -> bool {
        let m = &self.0.meters[slot];
        !self.0.stream_error.load(Relaxed)
            && m.load_ticket.load(std::sync::atomic::Ordering::Acquire) == ticket
            && m.tune_revision.load(Relaxed) == revision
            && m.tuning.read() == tuning
    }
    pub fn load(&self, slot: usize, clip: Arc<Clip>) -> Result<u64, String> {
        self.queue_load(slot, clip, false)
    }
    /// Load and start in one queue entry: no partially accepted Load + Play pair.
    pub fn load_and_play(&self, slot: usize, clip: Arc<Clip>) -> Result<u64, String> {
        self.queue_load(slot, clip, true)
    }
    fn queue_load(&self, slot: usize, clip: Arc<Clip>, play: bool) -> Result<u64, String> {
        if slot >= SLOTS {
            return Err("Deck inválido".into());
        }
        self.collect();
        if self.0.stream_error.load(Relaxed) {
            return Err("Recupera el dispositivo de audio antes de cargar otra pista".into());
        }
        let mut pending = self.0.pending_loads.load(Relaxed);
        loop {
            if pending >= 2 {
                return Err("Hay dos cargas pendientes. Espera a que el motor las procese".into());
            }
            match self
                .0
                .pending_loads
                .compare_exchange_weak(pending, pending + 1, Relaxed, Relaxed)
            {
                Ok(_) => break,
                Err(actual) => pending = actual,
            }
        }
        let ticket = self.0.next_load.fetch_add(1, Relaxed) + 1;
        if self
            .0
            .commands
            .push(Command::Load(slot, clip, ticket, play))
            .is_err()
        {
            self.0.pending_loads.fetch_sub(1, Relaxed);
            return Err("Cola de audio llena".into());
        }
        Ok(ticket)
    }
    pub fn is_loaded(&self, slot: usize, ticket: u64) -> bool {
        self.0
            .meters
            .get(slot)
            .is_some_and(|m| m.load_ticket.load(std::sync::atomic::Ordering::Acquire) == ticket)
    }
    /// Call from a non-audio worker; last references are never destroyed in the callback.
    pub fn collect(&self) {
        while self.0.retired.pop().is_some() {}
    }
    pub fn snapshot(&self) -> Snapshot {
        let f = |v: &AtomicU32| f32::from_bits(v.load(Relaxed));
        Snapshot {
            slots: self
                .0
                .meters
                .iter()
                .map(|m| SlotSnapshot {
                    id: m.id.load(Relaxed),
                    position: f64::from_bits(m.position.load(Relaxed)),
                    duration: f64::from_bits(m.duration.load(Relaxed)),
                    playing: m.playing.load(Relaxed),
                    looping: m.looping.load(Relaxed),
                    peak: f(&m.peak),
                    gain: f(&m.gain),
                    rate: f(&m.rate),
                    low: f(&m.low),
                    mid: f(&m.mid),
                    high: f(&m.high),
                    filter: f(&m.filter),
                    echo: f(&m.echo),
                    high_pass: f(&m.high_pass),
                    echo_beats: f(&m.echo_beats),
                    cues: std::array::from_fn(|i| f64::from_bits(m.cues[i].load(Relaxed))),
                    loop_in: f64::from_bits(m.loop_in.load(Relaxed)),
                    loop_out: f64::from_bits(m.loop_out.load(Relaxed)),
                    load_ticket: m.load_ticket.load(std::sync::atomic::Ordering::Acquire),
                    music: m.music.read(),
                    tuning: m.tuning.read(),
                    applied_tuning: m.applied_tuning.read(),
                    tune_revision: m.tune_revision.load(Relaxed),
                    tuned_revision: m.tuned_revision.load(Relaxed),
                    quantize: m.quantize.load(Relaxed),
                    sync: m.sync.load(Relaxed),
                    loop_beats: f64::from_bits(m.loop_beats.load(Relaxed)),
                    queued: m.queued.load(Relaxed),
                })
                .collect(),
            peak: f(&self.0.peak),
            rms: f(&self.0.rms),
            waveform: self.0.waveform.iter().map(f).collect(),
            crossfader: f(&self.0.crossfader),
            master: f(&self.0.master),
            stream_error: self.0.stream_error.load(Relaxed),
            callbacks: self.0.callbacks.load(Relaxed),
            clipped: self.0.clipped.load(Relaxed),
            master_deck: self.0.master_deck.load(Relaxed),
        }
    }
    pub fn fail(&self) {
        self.0.stream_error.store(true, Relaxed);
    }
}
struct Deck {
    clip: Option<Arc<Clip>>,
    load_ticket: u64,
    position: f64,
    playing: bool,
    params: Params,
    effects: Effects,
    loop_in: f64,
    loop_out: f64,
    looping: bool,
    peak: f32,
    last_output: [f32; 2],
    transition_from: [f32; 2],
    transition: u32,
    music: MusicAnalysis,
    tuning: Tuning,
    applied_tuning: Tuning,
    tune_revision: u64,
    tuned_revision: u64,
    quantize: bool,
    sync: bool,
    loop_beats: f64,
    echo_beats: f32,
    scheduled: Option<(Control, i64)>,
}
impl Deck {
    fn make_loop(&mut self, beats: f64, len: f64, rate: f64) {
        let span = self.grid().map_or(rate * beats * 0.5, |(_, b)| b * beats);
        let mut start = self.snap(self.position, false).min(len);
        if self.quantize && self.grid().is_some() && start + span > len {
            start = self.snap((len - span).max(0.), false);
        }
        if self.quantize && self.grid().is_some() && start + span > len + 1e-6 {
            self.looping = false;
            return;
        }
        self.loop_in = start;
        self.loop_out = (start + span).min(len);
        self.looping = self.loop_out > self.loop_in;
    }
    fn grid(&self) -> Option<(f64, f64)> {
        let rate = self.clip.as_ref()?.sample_rate as f64;
        Some((self.music.beat_offset * rate, 60. / self.music.bpm? * rate))
    }
    fn snap(&self, position: f64, ceil: bool) -> f64 {
        if !self.quantize {
            return position;
        }
        self.grid().map_or(position, |(offset, beat)| {
            (offset
                + if ceil {
                    ((position - offset) / beat).ceil()
                } else {
                    ((position - offset) / beat).floor()
                } * beat)
                .max(offset)
        })
    }
    fn request_tuning(&mut self, tuning: Tuning) {
        if self.tuning != tuning {
            self.tuning = tuning;
            self.tune_revision = self.tune_revision.wrapping_add(1);
        }
        // Reuse the current PCM whenever the required pitch factor is unchanged.
        if (tuning.factor() - self.applied_tuning.factor()).abs() < 1e-6 {
            self.applied_tuning = tuning;
            self.params.rate = tuning.rate;
            self.tuned_revision = self.tune_revision;
        }
    }
    /// Explicit jumps may leave a loop. Do not snap a cue back into the old range.
    fn jump(&mut self, position: f64) {
        self.position = position;
        if position < self.loop_in || position >= self.loop_out {
            self.looping = false;
        }
    }
    fn new(sr: u32) -> Self {
        Self {
            clip: None,
            load_ticket: 0,
            position: 0.,
            playing: false,
            params: Params::default(),
            effects: Effects::new(sr),
            loop_in: 0.,
            loop_out: 0.,
            looping: false,
            peak: 0.,
            last_output: [0.; 2],
            transition_from: [0.; 2],
            transition: 0,
            music: Default::default(),
            tuning: Default::default(),
            applied_tuning: Default::default(),
            tune_revision: 0,
            tuned_revision: 0,
            quantize: true,
            sync: false,
            loop_beats: 4.,
            echo_beats: 0.5,
            scheduled: None,
        }
    }
    fn frame(&mut self, sr: u32) -> [f32; 2] {
        if self.clip.is_none() {
            return [0.; 2];
        }
        let mut input = [0.; 2];
        if let Some(clip) = &self.clip {
            if self.playing && !clip.samples.is_empty() {
                let frame = self.position as usize;
                if frame < clip.samples.len() {
                    let a = clip.samples[frame];
                    let b = clip.samples[(frame + 1).min(clip.samples.len() - 1)];
                    let frac = self.position.fract() as f32;
                    input = [a[0] + (b[0] - a[0]) * frac, a[1] + (b[1] - a[1]) * frac];
                    self.position += clip.sample_rate as f64 / sr as f64 * self.params.rate as f64;
                    if self.looping
                        && self.loop_out > self.loop_in
                        && self.position >= self.loop_out
                    {
                        self.position = self.loop_in
                            + (self.position - self.loop_out) % (self.loop_out - self.loop_in);
                    }
                    if self.position >= clip.samples.len() as f64 {
                        self.position = clip.samples.len() as f64;
                        self.playing = false;
                    }
                } else {
                    self.playing = false;
                }
            }
        }
        let mut out = self
            .effects
            .process(input, self.params, if self.playing { 1. } else { 0. });
        if self.transition > 0 {
            let blend = self.transition as f32 / 128.;
            for (c, value) in out.iter_mut().enumerate() {
                *value = *value * (1. - blend) + self.transition_from[c] * blend;
            }
            self.transition -= 1;
        }
        self.last_output = out;
        self.peak = self.peak.max(out[0].abs()).max(out[1].abs());
        out
    }
    fn control(&mut self, c: Control, sr: u32) {
        let len = self.clip.as_ref().map_or(0., |c| c.samples.len() as f64);
        let rate = self.clip.as_ref().map_or(sr, |c| c.sample_rate) as f64;
        if matches!(
            c,
            Control::Play
                | Control::Pause
                | Control::Toggle
                | Control::Seek(_)
                | Control::Nudge(_)
                | Control::Cue(_)
                | Control::Trigger
        ) {
            self.transition_from = self.last_output;
            self.transition = 128;
        }
        match c {
            Control::Play => {
                if self.position >= len {
                    self.position = 0.;
                }
                self.playing = self.clip.is_some();
            }
            Control::Pause => self.playing = false,
            Control::Toggle => {
                if self.playing {
                    self.playing = false;
                } else {
                    self.control(Control::Play, sr);
                }
            }
            Control::Seek(t) => self.jump((t * rate).clamp(0., len)),
            Control::Nudge(t) => self.jump((self.position + t * rate).clamp(0., len)),
            Control::SetCue(i) => {
                if let Some(clip) = &self.clip {
                    clip.cues
                        .set(i, self.snap(self.position, false).min(len) / rate);
                }
            }
            Control::Cue(i) => {
                if let Some(clip) = &self.clip {
                    let position = (clip.cues.get(i) * rate).clamp(0., len);
                    self.jump(self.snap(position, false).min(len));
                    // A hot cue is a single jump-and-play command, even after EOF.
                    // It never toggles or changes another deck's state.
                    self.playing = self.position < len;
                }
            }
            Control::Trigger => {
                self.jump(0.);
                self.playing = self.clip.is_some();
            }
            Control::Loop => {
                if self.loop_out <= self.loop_in {
                    self.make_loop(
                        if self.grid().is_some() {
                            self.loop_beats
                        } else {
                            8.
                        },
                        len,
                        rate,
                    );
                    return;
                }
                self.looping = !self.looping && self.loop_out > self.loop_in;
            }
            Control::LoopIn => {
                self.loop_in = self.snap(self.position, false);
                self.looping = false;
            }
            Control::LoopOut => {
                self.loop_out = if self.quantize && self.grid().is_some() {
                    self.snap(self.position, true).min(self.snap(len, false))
                } else {
                    self.position.min(len)
                };
                self.looping = self.loop_out > self.loop_in;
            }
            Control::Gain(v) => self.params.gain = v,
            Control::Rate(v) => self.params.rate = v,
            Control::Quantize(q) => self.quantize = q,
            Control::LoopBeats(beats) => {
                self.loop_beats = beats;
                self.make_loop(beats, len, rate);
            }
            Control::Low(v) => self.params.low = v,
            Control::Mid(v) => self.params.mid = v,
            Control::High(v) => self.params.high = v,
            Control::Filter(v) => {
                self.params.filter = v;
                self.effects.set_filter(v, sr);
            }
            Control::Echo(v) => self.params.echo = v,
            Control::HighPass(v) => {
                self.params.high_pass = v;
                self.effects.set_highpass(v, sr);
            }
            Control::EchoBeats(v) => self.echo_beats = v,
            _ => {}
        }
    }
}
pub struct Renderer {
    shared: Arc<Shared>,
    decks: [Deck; SLOTS],
    sample_rate: u32,
    crossfader: f32,
    mix: [f32; 2],
    master: f32,
    master_target: f32,
    scope: [f32; 64],
    scope_head: usize,
    master_deck: usize,
}
pub fn engine(sample_rate: u32) -> (EngineHandle, Renderer) {
    let shared = Arc::new(Shared {
        commands: ArrayQueue::new(256),
        retired: ArrayQueue::new(256),
        meters: std::array::from_fn(|_| Meter::default()),
        pending_loads: AtomicUsize::new(0),
        next_load: AtomicU64::new(0),
        stream_error: AtomicBool::new(false),
        callbacks: AtomicU64::new(0),
        clipped: AtomicU64::new(0),
        peak: AtomicU32::new(0),
        rms: AtomicU32::new(0),
        waveform: std::array::from_fn(|_| AtomicU32::new(0)),
        crossfader: AtomicU32::new(0.5_f32.to_bits()),
        master: AtomicU32::new(0.75_f32.to_bits()),
        master_deck: AtomicUsize::new(0),
        next_tuning: AtomicU64::new(0),
        tuning_ack: AtomicU64::new(0),
    });
    (
        EngineHandle(shared.clone()),
        Renderer {
            shared,
            decks: std::array::from_fn(|_| Deck::new(sample_rate)),
            sample_rate,
            crossfader: 0.5,
            mix: dsp::crossfade(0.5),
            master: 0.,
            master_target: 0.75,
            scope: [0.; 64],
            scope_head: 0,
            master_deck: 0,
        },
    )
}
impl Renderer {
    fn refresh_sync(&mut self) {
        let master = self.master_deck;
        let other = 1 - master;
        if !self.decks[other].sync {
            return;
        }
        let (Some(a), Some(b)) = (self.decks[master].music.bpm, self.decks[other].music.bpm) else {
            self.decks[other].sync = false;
            return;
        };
        let rate = (a * self.decks[master].tuning.rate as f64 / b) as f32;
        if !(0.5..=1.5).contains(&rate) {
            self.decks[other].sync = false;
            return;
        }
        let tuning = Tuning {
            rate,
            ..self.decks[other].tuning
        };
        self.decks[other].request_tuning(tuning);
    }
    fn align_sync(&mut self) {
        let master = self.master_deck;
        let other = 1 - master;
        if !self.decks[other].sync || !self.decks[master].playing {
            return;
        }
        let (Some((ma, mb)), Some((fa, fb))) =
            (self.decks[master].grid(), self.decks[other].grid())
        else {
            return;
        };
        let phase = ((self.decks[master].position - ma) / mb).rem_euclid(1.);
        let d = &mut self.decks[other];
        let beat = ((d.position - fa) / fb).floor();
        let mut position = (fa + (beat + phase) * fb).max(0.);
        if d.looping && d.loop_out > d.loop_in {
            position = d.loop_in + (position - d.loop_in).rem_euclid(d.loop_out - d.loop_in);
        }
        let len = d.clip.as_ref().map_or(0., |c| c.samples.len() as f64);
        d.transition_from = d.last_output;
        d.transition = 128;
        d.position = position.min((len - 1.).max(0.));
    }
    pub fn render<T: cpal::Sample + cpal::FromSample<f32>>(
        &mut self,
        output: &mut [T],
        channels: usize,
    ) {
        // Bounded work: at most 64 control events per callback. Do not pop a Load if reclamation is full.
        let mut align = false;
        let mut tuning_ack = 0;
        for _ in 0..64 {
            if self.shared.retired.capacity() - self.shared.retired.len() < 2 {
                break;
            }
            let Some(command) = self.shared.commands.pop() else {
                break;
            };
            match command {
                Command::Load(slot, clip, ticket, play) => {
                    if slot < 2 && slot == self.master_deck {
                        self.decks[1 - slot].sync = false;
                        self.decks[1 - slot].scheduled = None;
                    }
                    self.shared.pending_loads.fetch_sub(1, Relaxed);
                    let d = &mut self.decks[slot];
                    if let Some(old) = d.clip.replace(clip) {
                        let _ = self.shared.retired.push(old);
                    }
                    d.load_ticket = ticket;
                    d.music = d.clip.as_ref().unwrap().music;
                    d.tuning = Default::default();
                    d.applied_tuning = Default::default();
                    d.params.rate = 1.;
                    d.tune_revision = d.tune_revision.wrapping_add(1);
                    d.tuned_revision = d.tune_revision;
                    d.sync = false;
                    d.scheduled = None;
                    d.transition_from = d.last_output;
                    d.transition = 128;
                    d.position = 0.;
                    d.playing = play;
                    d.looping = false;
                    d.loop_in = 0.;
                    d.loop_out = 0.;
                }
                Command::TuneBatch(ticket, batch) => {
                    tuning_ack = ticket;
                    let valid = batch.iter().enumerate().all(|(i, job)| {
                        job.as_ref().is_none_or(|j| {
                            self.decks[i].load_ticket == j.load_ticket
                                && self.decks[i].tune_revision == j.revision
                                && self.decks[i].tuning == j.tuning
                                && self.decks[i].clip.as_ref().is_some_and(|c| {
                                    c.id == j.clip.id
                                        && c.sample_rate == j.clip.sample_rate
                                        && c.samples.len() == j.clip.samples.len()
                                })
                        })
                    });
                    for (i, job) in batch.into_iter().enumerate() {
                        if let Some(job) = job {
                            self.shared.pending_loads.fetch_sub(1, Relaxed);
                            if valid {
                                let d = &mut self.decks[i];
                                if let Some(old) = d.clip.replace(job.clip) {
                                    let _ = self.shared.retired.push(old);
                                }
                                d.applied_tuning = job.tuning;
                                d.params.rate = job.tuning.rate;
                                d.tuned_revision = job.revision;
                                d.transition_from = d.last_output;
                                d.transition = 128;
                                align = true;
                            } else {
                                let _ = self.shared.retired.push(job.clip);
                            }
                        }
                    }
                }
                Command::Music(id, music) => {
                    align = true;
                    for d in &mut self.decks {
                        d.scheduled = None;
                        if d.clip.as_ref().is_some_and(|c| c.id == id) {
                            d.music = music;
                        }
                    }
                }
                Command::Control(slot, Control::MasterDeck) => {
                    for d in &mut self.decks[..2] {
                        d.scheduled = None;
                    }
                    self.master_deck = slot;
                    self.decks[slot].sync = false;
                    align = true;
                }
                Command::Control(slot, Control::Sync(enabled)) => {
                    self.decks[slot].scheduled = None;
                    self.decks[slot].sync = enabled && slot != self.master_deck;
                    align = enabled;
                }
                Command::Control(slot, Control::Tempo(bpm)) => {
                    if let Some(base) = self.decks[slot].music.bpm {
                        let rate = (bpm / base) as f32;
                        if (0.5..=1.5).contains(&rate) {
                            let t = Tuning {
                                rate,
                                ..self.decks[slot].tuning
                            };
                            self.decks[slot].request_tuning(t);
                        }
                    }
                }
                Command::Control(slot, Control::Rate(rate)) if slot < 2 => {
                    self.decks[slot].scheduled = None;
                    let t = Tuning {
                        rate,
                        ..self.decks[slot].tuning
                    };
                    self.decks[slot].sync = false;
                    self.decks[slot].request_tuning(t);
                }
                Command::Control(slot, Control::TransposeStep(step)) => {
                    let t = Tuning {
                        semitones: (self.decks[slot].tuning.semitones + step).clamp(-12, 12),
                        ..self.decks[slot].tuning
                    };
                    self.decks[slot].request_tuning(t);
                }
                Command::Control(slot, Control::RetryTuning) => {
                    self.decks[slot].tune_revision = self.decks[slot].tune_revision.wrapping_add(1);
                    let tuning = self.decks[slot].tuning;
                    self.decks[slot].request_tuning(tuning);
                }
                Command::Control(slot, Control::Transpose(semitones)) => {
                    let t = Tuning {
                        semitones,
                        ..self.decks[slot].tuning
                    };
                    self.decks[slot].request_tuning(t);
                }
                Command::Control(slot, Control::KeyLock(key_lock)) => {
                    let t = Tuning {
                        key_lock,
                        ..self.decks[slot].tuning
                    };
                    self.decks[slot].request_tuning(t);
                }
                Command::Control(slot, Control::KeySync) => {
                    let master = &self.decks[self.master_deck];
                    let Some(m) = master.music.key else {
                        continue;
                    };
                    let Some(k) = self.decks[slot].music.key else {
                        continue;
                    };
                    let target = m.tonic as i32
                        + master.tuning.semitones as i32
                        + if k.minor != m.minor {
                            if k.minor {
                                -3
                            } else {
                                3
                            }
                        } else {
                            0
                        };
                    let semitones = ((target - k.tonic as i32 + 6).rem_euclid(12) - 6) as i8;
                    let t = Tuning {
                        semitones,
                        key_lock: true,
                        ..self.decks[slot].tuning
                    };
                    self.decks[slot].request_tuning(t);
                }
                Command::Control(_, Control::Crossfader(v)) => {
                    self.crossfader = v;
                }
                Command::Control(_, Control::Master(v)) => self.master_target = v,
                Command::Control(
                    _,
                    c @ (Control::PlayBoth | Control::PauseBoth | Control::CueBoth(_)),
                ) => {
                    let control = match c {
                        Control::PlayBoth => Control::Play,
                        Control::PauseBoth => Control::Pause,
                        Control::CueBoth(i) => Control::Cue(i),
                        _ => unreachable!(),
                    };
                    // One queue entry: both decks change at the same audio boundary,
                    // even if the command budget is exhausted immediately afterwards.
                    for deck in &mut self.decks[..2] {
                        deck.scheduled = None;
                        deck.control(control, self.sample_rate);
                    }
                    align = true;
                }
                Command::Control(_, Control::SetMode(mix)) => {
                    self.crossfader = if mix { 0.5 } else { 0. };
                    if !mix {
                        self.master_deck = 0;
                        for d in &mut self.decks[..2] {
                            d.sync = false;
                            d.scheduled = None;
                        }
                        for deck in &mut self.decks[1..] {
                            deck.control(Control::Pause, self.sample_rate);
                        }
                    }
                }
                Command::Control(_, Control::Panic) => {
                    for d in &mut self.decks {
                        d.playing = false;
                        d.scheduled = None;
                    }
                    self.master_target = 0.;
                }
                Command::Control(slot, c) => {
                    if matches!(c, Control::Quantize(false)) {
                        self.decks[slot].scheduled = None;
                    }
                    if slot < 2
                        && self.decks[slot].sync
                        && self.decks[slot].quantize
                        && matches!(c, Control::Cue(_))
                        && self.decks[self.master_deck].playing
                    {
                        if let Some((offset, beat)) = self.decks[self.master_deck].grid() {
                            let current = (self.decks[self.master_deck].position - offset) / beat;
                            if current.rem_euclid(1.) > 1. / beat {
                                self.decks[slot].scheduled = Some((c, current.floor() as i64));
                                continue;
                            }
                        }
                    }
                    if matches!(
                        c,
                        Control::Pause
                            | Control::Toggle
                            | Control::Cue(_)
                            | Control::Seek(_)
                            | Control::Nudge(_)
                            | Control::Trigger
                            | Control::Play
                    ) {
                        self.decks[slot].scheduled = None;
                        if slot < 2 {
                            align = true;
                        }
                        if slot == self.master_deck
                            && matches!(c, Control::Cue(_) | Control::Seek(_) | Control::Trigger)
                        {
                            self.decks[1 - slot].scheduled = None;
                        }
                    }
                    self.decks[slot].control(c, self.sample_rate);
                }
            }
        }
        self.refresh_sync();
        if align
            && self.decks[..2]
                .iter()
                .all(|d| d.tune_revision == d.tuned_revision)
        {
            self.align_sync();
        }
        let target_mix = dsp::crossfade(self.crossfader);
        let mut peak = 0.0_f32;
        let mut squares = 0.0_f64;
        let mut clipped = 0;
        for d in &mut self.decks {
            d.peak = 0.;
            d.effects.set_echo_time(
                60. / (d.music.bpm.unwrap_or(120.) as f32 * d.params.rate) * d.echo_beats,
                self.sample_rate,
            );
        }
        for frame in output.chunks_mut(channels.max(1)) {
            let follower = 1 - self.master_deck;
            if let Some((control, queued_beat)) = self.decks[follower].scheduled {
                let master = &self.decks[self.master_deck];
                if !master.playing {
                    self.decks[follower].scheduled = None;
                }
                let master = &self.decks[self.master_deck];
                if master.playing
                    && master.grid().is_some_and(|(offset, beat)| {
                        ((master.position - offset) / beat).floor() as i64 != queued_beat
                    })
                {
                    self.decks[follower].scheduled = None;
                    self.decks[follower].control(control, self.sample_rate);
                }
            }
            self.master += (self.master_target - self.master) * 0.004;
            for (i, target) in target_mix.iter().enumerate() {
                self.mix[i] += (target - self.mix[i]) * 0.004;
            }
            let mut stereo = [0.; 2];
            for (i, deck) in self.decks.iter_mut().enumerate() {
                let signal = deck.frame(self.sample_rate);
                let weight = if i < 2 { self.mix[i] } else { 0.5 };
                for c in 0..2 {
                    stereo[c] += signal[c] * weight;
                }
            }
            for x in &mut stereo {
                *x *= self.master;
                if x.abs() > 0.98 {
                    clipped += 1;
                }
                *x = x.clamp(-0.98, 0.98);
                peak = peak.max(x.abs());
                squares += (*x as f64).powi(2);
            }
            self.scope[self.scope_head % 64] = (stereo[0] + stereo[1]) * 0.5;
            self.scope_head = (self.scope_head + 1) % 64;
            let mono = (stereo[0] + stereo[1]) * 0.5;
            for (c, sample) in frame.iter_mut().enumerate() {
                *sample = T::from_sample(if channels == 1 {
                    mono
                } else if c < 2 {
                    stereo[c]
                } else {
                    0.
                });
            }
        }
        for (i, d) in self.decks.iter().enumerate() {
            let m = &self.shared.meters[i];
            let sr = d.clip.as_ref().map_or(1, |c| c.sample_rate) as f64;
            m.id.store(d.clip.as_ref().map_or(0, |c| c.id), Relaxed);
            m.position.store((d.position / sr).to_bits(), Relaxed);
            m.duration.store(
                d.clip.as_ref().map_or(0., |c| c.duration()).to_bits(),
                Relaxed,
            );
            m.playing.store(d.playing, Relaxed);
            m.looping.store(d.looping, Relaxed);
            m.peak.store(d.peak.to_bits(), Relaxed);
            for (atomic, value) in [
                (&m.gain, d.params.gain),
                (&m.rate, d.params.rate),
                (&m.low, d.params.low),
                (&m.mid, d.params.mid),
                (&m.high, d.params.high),
                (&m.filter, d.params.filter),
                (&m.echo, d.params.echo),
                (&m.high_pass, d.params.high_pass),
                (&m.echo_beats, d.echo_beats),
            ] {
                atomic.store(value.to_bits(), Relaxed);
            }
            for j in 0..3 {
                m.cues[j].store(
                    d.clip.as_ref().map_or(0., |c| c.cues.get(j)).to_bits(),
                    Relaxed,
                );
            }
            m.load_ticket
                .store(d.load_ticket, std::sync::atomic::Ordering::Release);
            m.loop_in.store((d.loop_in / sr).to_bits(), Relaxed);
            m.loop_out.store((d.loop_out / sr).to_bits(), Relaxed);
            m.music.store(d.music);
            m.tuning.store(d.tuning);
            m.applied_tuning.store(d.applied_tuning);
            m.tune_revision.store(d.tune_revision, Relaxed);
            m.tuned_revision.store(d.tuned_revision, Relaxed);
            m.quantize.store(d.quantize, Relaxed);
            m.sync.store(d.sync, Relaxed);
            m.loop_beats.store(d.loop_beats.to_bits(), Relaxed);
            m.queued.store(d.scheduled.is_some(), Relaxed);
        }
        self.shared
            .crossfader
            .store(self.crossfader.to_bits(), Relaxed);
        self.shared
            .master
            .store(self.master_target.to_bits(), Relaxed);
        self.shared.peak.store(peak.to_bits(), Relaxed);
        // f32 wire format (f64::to_bits cannot be truncated).
        self.shared.rms.store(
            ((squares / (output.len() / channels.max(1) * 2).max(1) as f64).sqrt() as f32)
                .to_bits(),
            Relaxed,
        );
        for i in 0..64 {
            self.shared.waveform[i]
                .store(self.scope[(self.scope_head + i) % 64].to_bits(), Relaxed);
        }
        self.shared.callbacks.fetch_add(1, Relaxed);
        if tuning_ack != 0 {
            self.shared
                .tuning_ack
                .store(tuning_ack, std::sync::atomic::Ordering::Release);
        }
        self.shared.master_deck.store(self.master_deck, Relaxed);
        self.shared.clipped.fetch_add(clipped, Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn clip(id: u64) -> Arc<Clip> {
        Arc::new(Clip {
            id,
            samples: vec![[0.25, 0.25]; 48000],
            sample_rate: 48000,
            waveform: vec![],
            title: None,
            artist: None,
            music: Default::default(),
            cues: Arc::new(crate::annotations::CueState::default()),
        })
    }
    fn render(r: &mut Renderer, frames: usize) -> Vec<f32> {
        let mut out = vec![0.; frames * 2];
        r.render(&mut out, 2);
        out
    }
    #[test]
    fn playback_seek_eof_and_reload() {
        let (h, mut r) = engine(48000);
        let ticket = h.load(0, clip(7)).unwrap();
        assert!(!h.is_loaded(0, ticket));
        h.control(0, Control::Play).unwrap();
        assert!(render(&mut r, 4800).iter().any(|v| *v > 0.01));
        assert!(h.is_loaded(0, ticket));
        assert!((h.snapshot().slots[0].position - 0.1).abs() < 0.001);
        h.control(0, Control::Seek(0.99)).unwrap();
        render(&mut r, 1000);
        assert!(!h.snapshot().slots[0].playing);
        h.load(0, clip(8)).unwrap();
        render(&mut r, 1);
        h.collect();
        assert_eq!(h.snapshot().slots[0].id, 8);
    }
    #[test]
    fn loop_wraps_and_panic_mutes() {
        let (h, mut r) = engine(48000);
        h.load(0, clip(1)).unwrap();
        h.control(0, Control::Play).unwrap();
        h.control(0, Control::LoopIn).unwrap();
        h.control(0, Control::Seek(0.2)).unwrap();
        h.control(0, Control::LoopOut).unwrap();
        render(&mut r, 48000);
        assert!(h.snapshot().slots[0].position < 0.201);
        h.control(0, Control::Panic).unwrap();
        let out = render(&mut r, 10000);
        assert!(out.last().unwrap().abs() < 0.0001);
    }
    #[test]
    fn crossfader_and_output_are_bounded() {
        let (h, mut r) = engine(48000);
        h.load(0, clip(1)).unwrap();
        h.control(0, Control::Play).unwrap();
        h.control(0, Control::Crossfader(1.)).unwrap();
        let out = render(&mut r, 10000);
        assert!(out.last().unwrap().abs() < 0.0001);
        assert!(out.iter().all(|v| v.is_finite() && v.abs() <= 0.98));
        let weights = dsp::crossfade(0.5);
        assert!((weights[0].powi(2) + weights[1].powi(2) - 1.).abs() < 0.00001);
    }
    #[test]
    fn mode_switch_is_one_audio_command_and_cues_share_by_track() {
        let (h, mut r) = engine(48000);
        let shared = clip(20);
        h.load(0, shared.clone()).unwrap();
        h.load(1, shared).unwrap();
        h.control(0, Control::Play).unwrap();
        h.control(1, Control::Play).unwrap();
        render(&mut r, 4800);
        h.control(0, Control::SetCue(1)).unwrap();
        h.control(0, Control::SetMode(false)).unwrap();
        render(&mut r, 1);
        let s = h.snapshot();
        assert!(s.slots[0].playing);
        assert!(!s.slots[1].playing);
        assert_eq!(s.crossfader, 0.);
        assert!((s.slots[1].cues[1] - 0.1).abs() < 0.001);
        h.control(1, Control::Cue(1)).unwrap();
        render(&mut r, 1);
        assert!((h.snapshot().slots[1].position - 0.1).abs() < 0.001);
        assert!(h.snapshot().slots[1].playing);
        h.control(1, Control::Pause).unwrap();
        h.control(0, Control::SetMode(true)).unwrap();
        render(&mut r, 1);
        assert_eq!(h.snapshot().crossfader, 0.5);
        assert!(!h.snapshot().slots[1].playing);
    }
    #[test]
    fn validation_and_backpressure() {
        let (h, _) = engine(48000);
        assert!(h
            .action(Action {
                slot: 9,
                kind: "play".into(),
                value: 0.
            })
            .is_err());
        assert!(h
            .action(Action {
                slot: 0,
                kind: "gain".into(),
                value: f64::NAN
            })
            .is_err());
        for _ in 0..256 {
            h.control(0, Control::Toggle).unwrap();
        }
        assert!(h.control(0, Control::Toggle).is_err());
    }
    #[test]
    fn resampling_position_respects_source_rate() {
        let (h, mut r) = engine(44100);
        h.load(0, clip(1)).unwrap();
        h.control(0, Control::Play).unwrap();
        render(&mut r, 4410);
        assert!((h.snapshot().slots[0].position - 0.1).abs() < 0.00001);
    }
    #[test]
    fn hot_cue_restarts_an_ended_deck_without_interrupting_the_other() {
        let (h, mut r) = engine(48000);
        h.load(0, clip(1)).unwrap();
        h.load(1, clip(2)).unwrap();
        h.control(0, Control::Play).unwrap();
        h.control(1, Control::Seek(0.99)).unwrap();
        h.control(1, Control::Play).unwrap();
        render(&mut r, 960);
        assert!(!h.snapshot().slots[1].playing);
        h.control(1, Control::Cue(0)).unwrap();
        render(&mut r, 480);
        let s = h.snapshot();
        assert!(s.slots[0].playing);
        assert!(s.slots[1].playing, "hot cue must also start an ended deck");
        assert!((s.slots[0].position - 0.03).abs() < 0.00001);
        assert!((s.slots[1].position - 0.01).abs() < 0.00001);
        assert!(s.slots[1].peak > 0.01);
    }
    #[test]
    fn load_and_play_is_one_entry_even_when_only_one_queue_slot_remains() {
        let (h, mut r) = engine(48000);
        for _ in 0..255 {
            h.control(0, Control::Gain(0.8)).unwrap();
        }
        let ticket = h.load_and_play(0, clip(5)).unwrap();
        assert!(!h.is_loaded(0, ticket));
        assert!(
            h.control(0, Control::Play).is_err(),
            "no second queue entry was available"
        );
        for _ in 0..3 {
            render(&mut r, 256);
        }
        assert!(!h.is_loaded(0, ticket));
        let out = render(&mut r, 256);
        assert!(h.is_loaded(0, ticket));
        assert!(h.snapshot().slots[0].playing);
        assert!(out.last().unwrap().abs() > 0.01);
        assert_eq!(h.snapshot().slots[0].id, 5);
    }
    #[test]
    fn shared_clip_has_independent_positions_and_gains() {
        let (h, mut r) = engine(48000);
        let shared = clip(1);
        shared.cues.set(0, 0.2);
        h.load_and_play(0, shared.clone()).unwrap();
        h.load_and_play(1, shared).unwrap();
        render(&mut r, 480);
        h.control(1, Control::Cue(0)).unwrap();
        h.control(1, Control::Gain(0.)).unwrap();
        render(&mut r, 480);
        let s = h.snapshot();
        assert!((s.slots[0].position - 0.02).abs() < 0.00001);
        assert!((s.slots[1].position - 0.21).abs() < 0.00001);
        assert_eq!(s.slots[0].gain, 0.8);
        assert_eq!(s.slots[1].gain, 0.);
        assert!(s.slots[0].playing && s.slots[1].playing);
    }
    #[test]
    fn linked_transport_is_atomic_at_the_command_budget_boundary() {
        let (h, mut r) = engine(48000);
        h.load(0, clip(1)).unwrap();
        h.load(1, clip(2)).unwrap();
        render(&mut r, 1);
        for _ in 0..63 {
            h.control(0, Control::Gain(0.8)).unwrap();
        }
        h.control(0, Control::CueBoth(0)).unwrap();
        h.control(0, Control::PauseBoth).unwrap();
        render(&mut r, 256);
        let s = h.snapshot();
        assert!(s.slots[0].playing && s.slots[1].playing);
        assert_eq!(s.slots[0].position, s.slots[1].position);
        render(&mut r, 256);
        let s = h.snapshot();
        assert!(!s.slots[0].playing && !s.slots[1].playing);
        h.control(0, Control::PlayBoth).unwrap();
        render(&mut r, 256);
        assert!(h.snapshot().slots[..2].iter().all(|slot| slot.playing));
        assert!(h.control(0, Control::CueBoth(3)).is_err());
        h.control(0, Control::Panic).unwrap();
        h.control(0, Control::CueBoth(0)).unwrap();
        let out = render(&mut r, 4096);
        assert!(
            out.last().unwrap().abs() < 0.00001,
            "linked cue must not undo panic's master mute"
        );
    }
    #[test]
    fn cue_leaves_an_unrelated_loop_but_seek_keeps_pause() {
        let (h, mut r) = engine(48000);
        h.load(0, clip(1)).unwrap();
        h.control(0, Control::Seek(0.2)).unwrap();
        h.control(0, Control::LoopIn).unwrap();
        h.control(0, Control::Seek(0.4)).unwrap();
        h.control(0, Control::LoopOut).unwrap();
        h.control(0, Control::Cue(0)).unwrap();
        render(&mut r, 256);
        let s = h.snapshot();
        assert!(!s.slots[0].looping);
        assert!(s.slots[0].playing);
        assert!(s.slots[0].position < 0.01);
        h.control(0, Control::Pause).unwrap();
        h.control(0, Control::Seek(0.3)).unwrap();
        render(&mut r, 256);
        assert!(!h.snapshot().slots[0].playing);
        assert!((h.snapshot().slots[0].position - 0.3).abs() < 0.00001);
        h.control(1, Control::Cue(0)).unwrap();
        render(&mut r, 256);
        assert!(!h.snapshot().slots[1].playing, "empty decks stay stopped");
    }
    #[test]
    fn rapid_hot_cues_mix_both_decks_and_leave_samples_running() {
        for sample_rate in [44100, 48000, 96000] {
            let (h, mut r) = engine(sample_rate);
            let mut a = clip(1);
            Arc::get_mut(&mut a).unwrap().samples.fill([0.25, 0.]);
            let mut b = clip(2);
            Arc::get_mut(&mut b).unwrap().samples.fill([0., 0.25]);
            h.load(0, a).unwrap();
            h.load(1, b).unwrap();
            render(&mut r, 1);
            h.load(2, clip(3)).unwrap();
            h.control(2, Control::Trigger).unwrap();
            h.control(2, Control::Gain(0.)).unwrap();
            render(&mut r, 1024);
            let initial_sample_position = h.snapshot().slots[2].position;
            for _ in 0..20 {
                // Both events are consumed before rendering the next audio block.
                h.control(0, Control::Cue(0)).unwrap();
                h.control(1, Control::Cue(0)).unwrap();
                let out = render(&mut r, 256);
                let s = h.snapshot();
                assert!(s.slots[0].playing && s.slots[1].playing);
                assert_eq!(s.slots[0].position, s.slots[1].position);
                assert!(s.slots[0].peak > 0.01 && s.slots[1].peak > 0.01);
                assert!(out[out.len() - 2] > 0.01, "A contributes to left output");
                assert!(out[out.len() - 1] > 0.01, "B contributes to right output");
                assert!((out[out.len() - 2] - out[out.len() - 1]).abs() < 0.00001);
            }
            let s = h.snapshot();
            assert!(s.slots[2].playing);
            assert!(
                (s.slots[2].position - initial_sample_position - 5120. / sample_rate as f64).abs()
                    < 0.00001
            );
            h.control(1, Control::Pause).unwrap();
            let out = render(&mut r, 4096);
            assert!(out[out.len() - 2] > 0.01);
            assert!(out[out.len() - 1].abs() < 0.00001);
            assert!(h.snapshot().slots[0].playing);
        }
    }
}

#[cfg(test)]
mod allocation_tests;
#[cfg(test)]
mod musical_tests;
