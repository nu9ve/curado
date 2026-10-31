pub mod analysis;
pub mod annotations;
mod beat;
pub mod decode;
mod dsp;
pub mod fx;
pub mod limiter;
pub mod loudness;
pub mod music;
pub mod output;
mod pitch;
pub mod resample;
pub mod tuning;
use annotations::CUE_AT_START;
use annotations::HOT_CUES;
use crossbeam_queue::ArrayQueue;
use decode::Clip;
use dsp::{Effects, Params};
use fx::{BeatFx, BeatKind, BeatSettings, ColorKind, BEAT_DIVISIONS};
use limiter::Limiter;
use music::MusicAnalysis;
use pitch::PitchVoice;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering::Relaxed},
    Arc,
};
use tuning::{MusicMeter, Tuning, TuningMeter};

pub const SLOTS: usize = 11;
/// Hears a track without it being on a deck: to the cue bus, which needs a 4-channel output.
pub const PREVIEW_SLOT: usize = 10;
/// A preview on an output with no cue channels can only go to the speakers, so it stays quieter.
const PREVIEW_LEVEL: f32 = 0.5;
/// Release FX, in pad order.
const RELEASE_KINDS: u8 = 8;
const BRAKE_SHORT: f32 = 0.5;
const BRAKE_LONG: f32 = 2.;
const BACKSPIN_SHORT: f64 = 0.5;
const BACKSPIN_LONG: f64 = 1.5;
const BACKSPIN_SPEED: f64 = 5.;
/// How long the echo keeps its own settings after Echo Out, so its tail is not cut short.
const ECHO_OUT_TAIL_SECONDS: f64 = 6.;
const BUILD_UP_BEATS: f32 = 16.;
/// Beat FX units: one per music channel, then the master's, addressed as slot 2.
pub const BEAT_UNITS: usize = 3;
pub const MASTER_UNIT: usize = 2;
/// Loudness that auto gain brings tracks to, and how far it may move them.
const AUTO_GAIN_TARGET: f32 = -14.;
const AUTO_GAIN_RANGE: f32 = 12.;
/// Revolutions of a 33⅓ record: a turn of the platter moves about this much audio.
const JOG_RESPONSE_SECONDS: f64 = 0.02;
const JOG_SMOOTHING_SECONDS: f64 = 0.005;
const JOG_MAX_SPEED: f64 = 8.;
/// How fast a held pitch bend relaxes if its release never arrives.
const BEND_RELAX_SECONDS: f32 = 0.15;
/// How fast an interrupted brake gets back up to speed.
const SPIN_UP_SECONDS: f64 = 0.08;
/// One bounded, on-demand master capture for the visual worker.
pub const VISUAL_SAMPLES: usize = 2048;
/// Fade of a transport change. A cue or play reaches its target at full gain when it ends,
/// so this is also the delay between the command and an attack at that target.
pub const JUMP_FADE_SECONDS: f64 = 0.003;
/// A tempo this close to the original is not worth processing: it shifts pitch under a cent.
const KEY_LOCK_MIN_RATE_CHANGE: f32 = 0.0005;
/// Both voices meet exactly at a loop boundary, so its fade adds no delay and can be long
/// enough for sustained bass.
const LOOP_FADE_SECONDS: f64 = 0.008;
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
    ClearCue(usize),
    CrossfaderCurve(u8),
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
    Trim(f32),
    Color(f32),
    ColorFx(u8),
    ColorParam(f32),
    BeatFx(u8),
    BeatFxOn(bool),
    BeatFxBeats(f32),
    BeatFxDepth(f32),
    BeatFxBpm(f32),
    Slip(bool),
    Reverse(bool),
    Roll(f64),
    RollEnd,
    BeatJump(f64),
    LoopScale(f64),
    JogTouch(bool),
    JogMove(f64),
    JogBend(f32),
    Brake(f32),
    Sampler(f32),
    AutoGain(bool),
    ToggleBoth,
    Release(u8),
    ReleaseEnd,
    Cue2(bool),
    CueMaster(bool),
    CueMix(f32),
    CueLevel(f32),
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
            "cue" if (0.0..HOT_CUES as f64).contains(&self.value) && self.value.fract() == 0. => {
                Control::Cue(self.value as usize)
            }
            "setCue"
                if (0.0..HOT_CUES as f64).contains(&self.value) && self.value.fract() == 0. =>
            {
                Control::SetCue(self.value as usize)
            }
            "clearCue"
                if (0.0..HOT_CUES as f64).contains(&self.value) && self.value.fract() == 0. =>
            {
                Control::ClearCue(self.value as usize)
            }
            "crossfaderCurve" if matches!(self.value, 0.0 | 1.0 | 2.0) => {
                Control::CrossfaderCurve(self.value as u8)
            }
            "loop" => Control::Loop,
            "loopIn" => Control::LoopIn,
            "loopOut" => Control::LoopOut,
            "trigger" => Control::Trigger,
            "playBoth" => Control::PlayBoth,
            "pauseBoth" => Control::PauseBoth,
            "cueBoth"
                if (0.0..HOT_CUES as f64).contains(&self.value) && self.value.fract() == 0. =>
            {
                Control::CueBoth(self.value as usize)
            }
            "quantize" if matches!(self.value, 0.0 | 1.0) => Control::Quantize(self.value == 1.),
            "loopBeats" if BEAT_DIVISIONS.contains(&self.value) && self.value != 0.75 => {
                Control::LoopBeats(self.value)
            }
            "roll" if self.slot < 2 && BEAT_DIVISIONS.contains(&self.value) => {
                Control::Roll(self.value)
            }
            "rollEnd" if self.slot < 2 => Control::RollEnd,
            "beatJump" if self.slot < 2 && self.value != 0. && self.value.abs() <= 64. => {
                Control::BeatJump(self.value)
            }
            "loopScale" if self.slot < 2 && matches!(self.value, 0.5 | 2.0) => {
                Control::LoopScale(self.value)
            }
            "slip" if self.slot < 2 && matches!(self.value, 0.0 | 1.0) => {
                Control::Slip(self.value == 1.)
            }
            "reverse" if self.slot < 2 && matches!(self.value, 0.0 | 1.0) => {
                Control::Reverse(self.value == 1.)
            }
            "jogTouch" if self.slot < 2 && matches!(self.value, 0.0 | 1.0) => {
                Control::JogTouch(self.value == 1.)
            }
            "jogMove" if self.slot < 2 && self.value.abs() <= 8. => Control::JogMove(self.value),
            "jogBend" if self.slot < 2 && v.abs() <= 0.5 => Control::JogBend(v),
            "brake" if self.slot < 2 && (0.0..=3.0).contains(&v) => Control::Brake(v),
            "trim" if self.slot < 2 && (0.0..=2.0).contains(&v) => Control::Trim(v),
            "color" if self.slot < 2 && (-1.0..=1.0).contains(&v) => Control::Color(v),
            "colorFx"
                if self.slot < 2
                    && self.value.fract() == 0.
                    && (0.0..ColorKind::COUNT as f64).contains(&self.value) =>
            {
                Control::ColorFx(self.value as u8)
            }
            "colorParam" if self.slot < 2 && (0.0..=1.0).contains(&v) => Control::ColorParam(v),
            "beatFx"
                if self.slot < BEAT_UNITS
                    && self.value.fract() == 0.
                    && (0.0..BeatKind::COUNT as f64).contains(&self.value) =>
            {
                Control::BeatFx(self.value as u8)
            }
            "beatFxOn" if self.slot < BEAT_UNITS && matches!(self.value, 0.0 | 1.0) => {
                Control::BeatFxOn(self.value == 1.)
            }
            "beatFxBeats" if self.slot < BEAT_UNITS && BEAT_DIVISIONS.contains(&self.value) => {
                Control::BeatFxBeats(v)
            }
            "beatFxDepth" if self.slot < BEAT_UNITS && (0.0..=1.0).contains(&v) => {
                Control::BeatFxDepth(v)
            }
            "beatFxBpm" if self.slot < BEAT_UNITS && (v == 0. || (40.0..=300.0).contains(&v)) => {
                Control::BeatFxBpm(v)
            }
            "toggleBoth" => Control::ToggleBoth,
            "releaseFx"
                if self.slot < 2
                    && self.value.fract() == 0.
                    && (0.0..RELEASE_KINDS as f64).contains(&self.value) =>
            {
                Control::Release(self.value as u8)
            }
            "releaseEnd" if self.slot < 2 => Control::ReleaseEnd,
            "channelCue" if self.slot < 2 && matches!(self.value, 0.0 | 1.0) => {
                Control::Cue2(self.value == 1.)
            }
            "masterCue" if matches!(self.value, 0.0 | 1.0) => Control::CueMaster(self.value == 1.),
            "cueMix" if (0.0..=1.0).contains(&v) => Control::CueMix(v),
            "cueLevel" if (0.0..=1.0).contains(&v) => Control::CueLevel(v),
            "sampler" if (0.0..=1.5).contains(&v) => Control::Sampler(v),
            "autoGain" if matches!(self.value, 0.0 | 1.0) => Control::AutoGain(self.value == 1.),
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
            "gain" if (0.0..=1.5).contains(&v) => Control::Gain(v),
            "rate" if (0.5..=1.5).contains(&v) => Control::Rate(v),
            "low" if (0.0..=2.0).contains(&v) => Control::Low(v),
            "mid" if (0.0..=2.0).contains(&v) => Control::Mid(v),
            "high" if (0.0..=2.0).contains(&v) => Control::High(v),
            "filter" if (0.0..=1.0).contains(&v) => Control::Filter(v),
            "echo" if self.slot < 2 && (0.0..=1.0).contains(&v) => Control::Echo(v),
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
    cues: [AtomicU64; HOT_CUES],
    cue_ends: [AtomicU64; HOT_CUES],
    loop_in: AtomicU64,
    loop_out: AtomicU64,
    music: MusicMeter,
    tuning: TuningMeter,
    quantize: AtomicBool,
    sync: AtomicBool,
    loop_beats: AtomicU64,
    queued: AtomicBool,
    trim: AtomicU32,
    color: AtomicU32,
    color_param: AtomicU32,
    color_fx: AtomicU32,
    brake: AtomicU32,
    slip: AtomicBool,
    reverse: AtomicBool,
    rolling: AtomicBool,
    cue: AtomicBool,
}
#[derive(Default)]
struct BeatMeter {
    kind: AtomicU32,
    on: AtomicBool,
    beats: AtomicU32,
    depth: AtomicU32,
    bpm: AtomicU32,
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
    limited: AtomicU64,
    limiter_db: AtomicU32,
    peak: AtomicU32,
    rms: AtomicU32,
    waveform: [AtomicU32; 64],
    visual_state: AtomicUsize,
    visual_next: AtomicUsize,
    visual_samples: [AtomicU32; VISUAL_SAMPLES],
    visual_rate: u32,
    crossfader: AtomicU32,
    master: AtomicU32,
    master_deck: AtomicUsize,
    beat_meters: [BeatMeter; BEAT_UNITS],
    sampler: AtomicU32,
    auto_gain: AtomicBool,
    crossfader_curve: AtomicU32,
    cue_master: AtomicBool,
    cue_mix: AtomicU32,
    cue_level: AtomicU32,
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
    pub cues: [f64; HOT_CUES],
    /// Where a hot cue's loop ends, or 0 for a plain cue.
    pub cue_ends: [f64; HOT_CUES],
    pub loop_in: f64,
    pub loop_out: f64,
    pub load_ticket: u64,
    pub music: MusicAnalysis,
    pub tuning: Tuning,
    pub quantize: bool,
    pub sync: bool,
    pub loop_beats: f64,
    pub queued: bool,
    pub trim: f32,
    pub color: f32,
    pub color_param: f32,
    pub color_fx: u8,
    pub brake: f32,
    pub slip: bool,
    pub reverse: bool,
    pub rolling: bool,
    pub cue: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatFxSnapshot {
    pub kind: u8,
    pub on: bool,
    pub beats: f32,
    pub depth: f32,
    pub bpm: f32,
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
    /// Samples the final safety clamp had to touch. The limiter should keep this at zero.
    pub clipped: u64,
    /// Output frames the limiter turned down.
    pub limited: u64,
    /// Recent gain reduction in dB, held long enough for 20 Hz telemetry to show it.
    pub limiter_db: f32,
    pub master_deck: usize,
    /// Channel A, channel B, master.
    pub beat_fx: Vec<BeatFxSnapshot>,
    pub sampler: f32,
    pub auto_gain: bool,
    pub crossfader_curve: u8,
    pub cue_master: bool,
    pub cue_mix: f32,
    pub cue_level: f32,
}
impl EngineHandle {
    /// Called by the visual worker only. Returns one complete mono master window and
    /// requests the next one; the callback never waits for the consumer.
    pub fn visual_samples(&self) -> Option<([f32; VISUAL_SAMPLES], u32)> {
        use std::sync::atomic::Ordering::{Acquire, Release};
        let state = self.0.visual_state.load(Acquire);
        if state != 0 && state & 1 == 0 {
            let samples =
                std::array::from_fn(|i| f32::from_bits(self.0.visual_samples[i].load(Relaxed)));
            let next = self.0.visual_next.fetch_add(2, Relaxed);
            self.0.visual_state.store(next, Release);
            Some((samples, self.0.visual_rate))
        } else {
            if state == 0 {
                let next = self.0.visual_next.fetch_add(2, Relaxed);
                let _ = self
                    .0
                    .visual_state
                    .compare_exchange(0, next, Release, Relaxed);
            }
            None
        }
    }
    pub fn stop_visual_capture(&self) {
        self.0
            .visual_state
            .store(0, std::sync::atomic::Ordering::Release);
    }
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
    pub fn control(&self, slot: usize, control: Control) -> Result<(), String> {
        if slot >= SLOTS
            || matches!(control, Control::Cue(i)|Control::SetCue(i)|Control::ClearCue(i)|Control::CueBoth(i) if i>=HOT_CUES)
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
                        | Control::Slip(_)
                        | Control::Reverse(_)
                        | Control::Roll(_)
                        | Control::RollEnd
                        | Control::BeatJump(_)
                        | Control::LoopScale(_)
                        | Control::JogTouch(_)
                        | Control::JogMove(_)
                        | Control::JogBend(_)
                        | Control::Brake(_)
                        | Control::Release(_)
                        | Control::ReleaseEnd
                        | Control::Cue2(_)
                        | Control::Trim(_)
                        | Control::Color(_)
                        | Control::ColorFx(_)
                        | Control::ColorParam(_)
                ))
            || (slot >= BEAT_UNITS
                && matches!(
                    control,
                    Control::BeatFx(_)
                        | Control::BeatFxOn(_)
                        | Control::BeatFxBeats(_)
                        | Control::BeatFxDepth(_)
                        | Control::BeatFxBpm(_)
                ))
        {
            return Err("Deck o cue inválido".into());
        }
        self.0
            .commands
            .push(Command::Control(slot, control))
            .map_err(|_| "Cola de audio llena; vuelve a intentar".into())
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
                    cue_ends: std::array::from_fn(|i| f64::from_bits(m.cue_ends[i].load(Relaxed))),
                    loop_in: f64::from_bits(m.loop_in.load(Relaxed)),
                    loop_out: f64::from_bits(m.loop_out.load(Relaxed)),
                    load_ticket: m.load_ticket.load(std::sync::atomic::Ordering::Acquire),
                    music: m.music.read(),
                    tuning: m.tuning.read(),
                    quantize: m.quantize.load(Relaxed),
                    sync: m.sync.load(Relaxed),
                    loop_beats: f64::from_bits(m.loop_beats.load(Relaxed)),
                    queued: m.queued.load(Relaxed),
                    trim: f(&m.trim),
                    color: f(&m.color),
                    color_param: f(&m.color_param),
                    color_fx: m.color_fx.load(Relaxed) as u8,
                    brake: f(&m.brake),
                    slip: m.slip.load(Relaxed),
                    reverse: m.reverse.load(Relaxed),
                    rolling: m.rolling.load(Relaxed),
                    cue: m.cue.load(Relaxed),
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
            limited: self.0.limited.load(Relaxed),
            limiter_db: f(&self.0.limiter_db),
            master_deck: self.0.master_deck.load(Relaxed),
            beat_fx: self
                .0
                .beat_meters
                .iter()
                .map(|m| BeatFxSnapshot {
                    kind: m.kind.load(Relaxed) as u8,
                    on: m.on.load(Relaxed),
                    beats: f(&m.beats),
                    depth: f(&m.depth),
                    bpm: f(&m.bpm),
                })
                .collect(),
            sampler: f(&self.0.sampler),
            auto_gain: self.0.auto_gain.load(Relaxed),
            crossfader_curve: self.0.crossfader_curve.load(Relaxed) as u8,
            cue_master: self.0.cue_master.load(Relaxed),
            cue_mix: f(&self.0.cue_mix),
            cue_level: f(&self.0.cue_level),
        }
    }
    /// Just the music decks' positions, or nothing while neither moves. Cheap enough to read far
    /// more often than a full snapshot.
    pub fn live_positions(&self) -> Option<[f64; 2]> {
        let decks = &self.0.meters[..2];
        decks
            .iter()
            .any(|m| m.playing.load(Relaxed))
            .then(|| std::array::from_fn(|i| f64::from_bits(decks[i].position.load(Relaxed))))
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
    /// Outgoing read head during a fade. It reads `tail_clip` only after the PCM was replaced.
    tail_position: f64,
    tail_step: f64,
    tail_gain: f32,
    tail_clip: Option<Arc<Clip>>,
    tail_reads_clip: bool,
    fade_left: u32,
    fade_total: u32,
    /// False when the main voice must sound at full level, like the first sample of a file.
    fade_main: bool,
    jump_frames: u32,
    loop_frames: u32,
    music: MusicAnalysis,
    tuning: Tuning,
    /// Two voices let a jump fade between old and new audio while both stay pitched. Music
    /// decks only; a sample never needs one.
    pitch: Option<Box<[PitchVoice; 2]>>,
    main_pitch: usize,
    main_pitched: bool,
    tail_pitched: bool,
    restart_pitch: bool,
    quantize: bool,
    sync: bool,
    loop_beats: f64,
    echo_beats: f32,
    scheduled: Option<(Control, i64)>,
    slip: bool,
    /// Where the track would be without the current detour, while one is in progress.
    shadow: f64,
    excursion: bool,
    reverse: bool,
    rolling: bool,
    saved_loop: (f64, f64, bool),
    jog_touch: bool,
    jog_target: f64,
    jog_speed: f64,
    jog_response: f64,
    jog_smoothing: f64,
    bend: f32,
    brake: f32,
    braking: bool,
    /// Speed relative to normal while braking or recovering from a brake.
    ramp: f64,
    brake_seconds: f32,
    /// Backspin: frames left of how many, and the deck stops when they end.
    spin_left: u32,
    spin_total: u32,
    /// Echo Out: frames until the deck stops, then until the unit gets its settings back.
    echo_out_left: u32,
    echo_restore_left: u32,
    echo_restore: Option<BeatSettings>,
    building: bool,
    build: f32,
    ducking: bool,
    cue: bool,
}
impl Deck {
    /// Output frames per beat at the current speed, with 120 BPM standing in for an unknown one.
    fn clock(&self, sr: u32) -> f32 {
        60. / (self.local_bpm().unwrap_or(120.) as f32 * self.params.rate.max(0.01)) * sr as f32
    }
    /// How many beats into the track the read head is, when it has a grid to count on.
    fn beats_position(&self) -> Option<f64> {
        let (offset, beat) = self.grid()?;
        let inside = (self.position - offset) / beat;
        Some(match self.beat_map() {
            Some(beats) => {
                let rate = self.clip.as_ref()?.sample_rate as f64;
                map_index(beats, self.position / rate) as f64 + inside
            }
            None => inside,
        })
    }
    fn make_loop(&mut self, beats: f64, len: f64, rate: f64) {
        let mut start = self.snap(self.position, f64::round).min(len);
        let span = self.span_from(start, beats, rate);
        if self.quantize && self.grid().is_some() && start + span > len {
            start = self.snap((len - span).max(0.), f64::floor);
        }
        if self.quantize && self.grid().is_some() && start + span > len + 1e-6 {
            self.looping = false;
            return;
        }
        self.loop_in = start;
        self.loop_out = (start + span).min(len);
        self.looping = self.loop_out > self.loop_in;
    }
    /// A correction by hand is one tempo, whatever the analysis found before it.
    fn beat_map(&self) -> Option<&[f64]> {
        self.clip
            .as_ref()?
            .beats
            .as_deref()
            .filter(|_| !self.music.manual)
            .filter(|beats| beats.len() >= 2)
    }
    /// The beat around the read head: where it starts and how long it lasts, in source frames.
    /// With one tempo those hold for the whole track; on a map they hold for this beat, which
    /// is why anything counting beats across a distance uses `span_from` or `beats_position`.
    fn grid(&self) -> Option<(f64, f64)> {
        let rate = self.clip.as_ref()?.sample_rate as f64;
        if let Some(beats) = self.beat_map() {
            let at = map_index(beats, self.position / rate);
            return Some((beats[at] * rate, (beats[at + 1] - beats[at]) * rate));
        }
        Some((self.music.beat_offset * rate, 60. / self.music.bpm? * rate))
    }
    /// The tempo under the read head.
    fn local_bpm(&self) -> Option<f64> {
        if let Some(beats) = self.beat_map() {
            let rate = self.clip.as_ref()?.sample_rate as f64;
            let at = map_index(beats, self.position / rate);
            return Some(60. / (beats[at + 1] - beats[at]));
        }
        self.music.bpm
    }
    /// Source frames from `start` to `beats` beats later (or earlier, when negative).
    fn span_from(&self, start: f64, beats: f64, rate: f64) -> f64 {
        let Some(map) = self.beat_map() else {
            return self
                .grid()
                .map_or(rate * beats * 0.5, |(_, beat)| beat * beats);
        };
        let at = map_index(map, start / rate);
        let index = at as f64 + (start / rate - map[at]) / (map[at + 1] - map[at]) + beats;
        let whole = (index.floor().max(0.) as usize).min(map.len() - 2);
        let target = map[whole] + (index - whole as f64) * (map[whole + 1] - map[whole]);
        target * rate - start
    }
    fn snap(&self, position: f64, to_beat: fn(f64) -> f64) -> f64 {
        if !self.quantize {
            return position;
        }
        self.grid().map_or(position, |(offset, beat)| {
            (offset + to_beat((position - offset) / beat) * beat).max(offset)
        })
    }
    /// Speed through the track at normal transport, which is what a slip's shadow moves at.
    fn forward_step(&self, sr: u32) -> f64 {
        self.clip.as_ref().map_or(1., |clip| {
            clip.sample_rate as f64 / sr as f64 * self.params.rate as f64
        })
    }
    fn step(&self, sr: u32) -> f64 {
        let direction = if self.reverse { -1. } else { 1. };
        self.forward_step(sr) * (1. + self.bend as f64) * self.ramp * direction
    }
    /// Anything that takes the read head away from where the track would be. With slip on, or
    /// during a roll, the track carries on silently underneath and resumes from there.
    fn sync_slip(&mut self, sr: u32) {
        if self.rolling {
            return;
        }
        let away = self.looping || self.reverse || self.jog_touch;
        if self.slip && away && !self.excursion {
            self.excursion = true;
            self.shadow = self.position;
        } else if self.excursion && (!self.slip || !away) {
            let resume = self.slip;
            self.excursion = false;
            if resume {
                self.return_to_shadow(sr);
            }
        }
    }
    fn return_to_shadow(&mut self, sr: u32) {
        let len = self.clip.as_ref().map_or(0., |c| c.samples.len() as f64);
        if self.playing {
            let step = self.step(sr);
            self.release_voice(self.jump_frames, true, step);
        }
        self.position = self.shadow.min(len);
    }
    fn start_roll(&mut self, beats: f64, len: f64, rate: f64) {
        if !self.playing || self.clip.is_none() {
            return;
        }
        if !self.rolling {
            self.rolling = true;
            self.saved_loop = (self.loop_in, self.loop_out, self.looping);
            self.shadow = if self.excursion {
                self.shadow
            } else {
                self.position
            };
            self.excursion = true;
        }
        let anchor = self.shadow;
        let span = self.span_from(anchor, beats, rate);
        let start = match self.grid().filter(|_| self.quantize) {
            Some((offset, _)) => (offset + ((anchor - offset) / span).floor() * span).max(0.),
            None => anchor,
        };
        self.loop_in = start;
        self.loop_out = (start + span).min(len);
        self.looping = self.loop_out > self.loop_in;
        self.loop_beats = beats;
    }
    fn end_roll(&mut self, sr: u32) {
        if !self.rolling {
            return;
        }
        self.rolling = false;
        self.excursion = false;
        (self.loop_in, self.loop_out, self.looping) = self.saved_loop;
        self.return_to_shadow(sr);
    }
    /// A brake and a scratch change speed continuously, which only plain playback can follow.
    fn refresh_pitch(&mut self, sr: u32) {
        let pitched = self.wants_pitch();
        if pitched != self.main_pitched {
            if self.playing {
                let step = self.step(sr);
                self.release_voice(self.jump_frames, true, step);
            }
            self.main_pitched = pitched;
            self.restart_pitch = true;
        }
    }
    /// Raised cosine: the gains sum to one, so a fade never adds level to correlated audio.
    fn fade_gain(&self) -> f32 {
        if self.fade_left == 0 {
            return 1.;
        }
        let progress = 1. - self.fade_left as f32 / self.fade_total as f32;
        0.5 - 0.5 * (std::f32::consts::PI * progress).cos()
    }
    /// Hands the read head to the fading voice. Only one outgoing voice exists: a retrigger
    /// during a fade keeps the louder one, so any step is at most the quieter voice.
    fn release_voice(&mut self, frames: u32, fade_main: bool, step: f64) {
        let gain = self.fade_gain();
        let tail = self.tail_gain * (1. - gain);
        let main = match (self.playing, self.fade_main) {
            (false, _) => 0.,
            (true, true) => gain,
            (true, false) => 1.,
        };
        if main > 0. && main >= tail {
            self.tail_position = self.position;
            self.tail_step = step;
            self.tail_gain = main;
            self.tail_reads_clip = false;
            self.tail_pitched = self.main_pitched;
            if self.main_pitched {
                self.main_pitch = 1 - self.main_pitch;
            }
        } else {
            self.tail_gain = tail;
        }
        self.restart_pitch = true;
        self.fade_left = frames;
        self.fade_total = frames.max(1);
        self.fade_main = fade_main;
    }
    /// Starts early enough to reach `target` at full gain, so an attack there stays intact.
    fn preroll_to(&mut self, target: f64, sr: u32) {
        let step = self.step(sr);
        let lead = self.jump_frames as f64 * step;
        if target >= lead {
            self.release_voice(self.jump_frames, true, step);
            self.position = target - lead;
        } else {
            // Nothing precedes the start of the file: its first sample plays at full level.
            self.release_voice(self.jump_frames, false, step);
            self.position = target;
        }
    }
    /// With a brake set the record winds down first. `stop_now` is for what must be silent.
    fn stop(&mut self, sr: u32) {
        if self.playing && self.brake > 0. && !self.jog_touch {
            self.braking = true;
            self.brake_seconds = self.brake;
        } else {
            self.stop_now(sr);
        }
    }
    fn stop_now(&mut self, sr: u32) {
        self.braking = false;
        self.ramp = 1.;
        self.spin_left = 0;
        self.echo_out_left = 0;
        if self.playing {
            let step = self.step(sr);
            self.release_voice(self.jump_frames, true, step);
            self.playing = false;
        }
    }
    /// Replaced PCM stays alive while the outgoing voice still reads it. Retires at most one
    /// clip; the renderer reserves room for that before taking the command.
    fn install_clip(&mut self, clip: Arc<Clip>, retired: &ArrayQueue<Arc<Clip>>, sr: u32) {
        if self.playing {
            let step = self.step(sr);
            self.release_voice(self.jump_frames, true, step);
        }
        let tail_reads_old = self.fade_left > 0 && !self.tail_reads_clip;
        let Some(old) = self.clip.replace(clip) else {
            return;
        };
        if tail_reads_old {
            self.tail_reads_clip = true;
            if let Some(previous) = self.tail_clip.replace(old) {
                let _ = retired.push(previous);
            }
        } else {
            let _ = retired.push(old);
        }
    }
    fn wrap_loop(&mut self, step: f64) {
        if !self.looping || self.loop_out <= self.loop_in || step == 0. {
            return;
        }
        let span = self.loop_out - self.loop_in;
        if step < 0. {
            if self.position <= self.loop_in {
                let overshoot = (self.loop_in - self.position) % span;
                let fade = (self.loop_frames as f64 * -step).min(span * 0.5);
                self.release_voice((fade / -step) as u32, true, step);
                self.position = self.loop_out - overshoot;
            }
            return;
        }
        let fade = (self.loop_frames as f64 * step).min(span * 0.5);
        // The old voice ends at OUT as the new one reaches IN at full gain. With no audio
        // before IN, the new voice starts there directly and the old one fades past OUT.
        let lead = if self.loop_in >= fade { fade } else { 0. };
        let boundary = self.loop_out - lead;
        if self.position >= boundary {
            let overshoot = (self.position - boundary) % span;
            self.release_voice((fade / step) as u32, lead > 0., step);
            self.position = self.loop_in - lead + overshoot;
        }
    }
    fn wants_pitch(&self) -> bool {
        self.pitch.is_some()
            && !self.reverse
            && !self.jog_touch
            && !self.braking
            && self.spin_left == 0
            && self.ramp >= 1.
            && (self.tuning.semitones != 0
                || (self.tuning.key_lock
                    && (self.params.rate - 1.).abs() >= KEY_LOCK_MIN_RATE_CHANGE))
    }
    /// Takes effect at once. Moving between plain playback and pitched audio crossfades, with
    /// both voices at the same position.
    fn apply_tuning(&mut self, tuning: Tuning, sr: u32) {
        if self.tuning == tuning && self.params.rate == tuning.rate {
            return;
        }
        let outgoing_step = self.step(sr);
        let was_pitched = self.main_pitched;
        self.tuning = tuning;
        self.params.rate = tuning.rate;
        let pitched = self.wants_pitch();
        if pitched != was_pitched {
            if self.playing {
                self.release_voice(self.jump_frames, true, outgoing_step);
            }
            self.main_pitched = pitched;
            self.restart_pitch = true;
        } else if pitched {
            if let Some(voices) = &mut self.pitch {
                voices[self.main_pitch].set_transpose(tuning.pitch_ratio());
            }
        }
    }
    /// Explicit jumps may leave a loop. Do not snap a cue back into the old range.
    fn jump(&mut self, target: f64, sr: u32) {
        if self.playing {
            let step = self.step(sr);
            self.release_voice(self.jump_frames, true, step);
        }
        self.position = target;
        self.leave_loop_if_outside(target);
    }
    fn leave_loop_if_outside(&mut self, target: f64) {
        if target < self.loop_in || target >= self.loop_out {
            self.looping = false;
        }
    }
    /// A saved cue plays exactly where it was stored; Q alone never moves it. `on_grid` keeps
    /// a synced deck in phase. An unsaved cue reads 0:00 and means the track's first sound.
    fn trigger_cue(&mut self, index: usize, on_grid: bool, sr: u32) {
        let Some(clip) = &self.clip else {
            return;
        };
        let len = clip.samples.len() as f64;
        let saved = clip.cues.get(index);
        let stored = (saved * clip.sample_rate as f64).clamp(0., len);
        let loop_end = (clip.cues.end(index) * clip.sample_rate as f64).clamp(0., len);
        let target = if saved <= 0. {
            let first = self.music.first_sound * clip.sample_rate as f64;
            self.snap(first.min(len), f64::round).min(len)
        } else if stored < 1. {
            // Saved at the very start of the file.
            0.
        } else if on_grid {
            self.snap(stored, f64::round).min(len)
        } else {
            stored
        };
        self.excursion = self.rolling;
        // A hot cue is a single jump-and-play command, even after EOF.
        // It never toggles or changes another deck's state.
        if target < len {
            self.preroll_to(target, sr);
            if loop_end > stored {
                // A saved loop plays as saved; a synced deck shifts it with the start.
                self.loop_in = target;
                self.loop_out = (loop_end + target - stored).min(len);
                self.looping = true;
            } else {
                self.leave_loop_if_outside(target);
            }
            self.playing = true;
        } else {
            self.jump(target, sr);
            self.playing = false;
        }
    }
    fn new(sr: u32, can_pitch: bool) -> Self {
        Self {
            clip: None,
            load_ticket: 0,
            position: 0.,
            playing: false,
            params: Params::default(),
            effects: Effects::new(sr, can_pitch),
            loop_in: 0.,
            loop_out: 0.,
            looping: false,
            peak: 0.,
            tail_position: 0.,
            tail_step: 1.,
            tail_gain: 0.,
            tail_clip: None,
            tail_reads_clip: false,
            fade_left: 0,
            fade_total: 1,
            fade_main: true,
            jump_frames: (sr as f64 * JUMP_FADE_SECONDS).round() as u32,
            loop_frames: (sr as f64 * LOOP_FADE_SECONDS).round() as u32,
            music: Default::default(),
            tuning: Default::default(),
            pitch: can_pitch.then(|| Box::new([PitchVoice::new(sr), PitchVoice::new(sr)])),
            main_pitch: 0,
            main_pitched: false,
            tail_pitched: false,
            restart_pitch: false,
            quantize: true,
            sync: false,
            loop_beats: 4.,
            echo_beats: 0.5,
            scheduled: None,
            slip: false,
            shadow: 0.,
            excursion: false,
            reverse: false,
            rolling: false,
            saved_loop: (0., 0., false),
            jog_touch: false,
            jog_target: 0.,
            jog_speed: 0.,
            jog_response: 1. / (sr as f64 * JOG_RESPONSE_SECONDS),
            jog_smoothing: 1. - (-1. / (sr as f64 * JOG_SMOOTHING_SECONDS)).exp(),
            bend: 0.,
            brake: 0.,
            braking: false,
            ramp: 1.,
            brake_seconds: 0.,
            spin_left: 0,
            spin_total: 1,
            echo_out_left: 0,
            echo_restore_left: 0,
            echo_restore: None,
            building: false,
            build: 0.,
            ducking: false,
            cue: false,
        }
    }
    /// Follows the hand: speed is eased so that the gaps between pointer events do not buzz.
    fn scratch(&mut self, len: f64) -> [f32; 2] {
        let wanted = ((self.jog_target - self.position) * self.jog_response)
            .clamp(-JOG_MAX_SPEED, JOG_MAX_SPEED);
        self.jog_speed += (wanted - self.jog_speed) * self.jog_smoothing;
        self.position = (self.position + self.jog_speed).clamp(0., (len - 1.).max(0.));
        // A record standing still is silent, not a held sample value.
        let level = (self.jog_speed.abs() / 0.02).min(1.) as f32;
        let Some(clip) = &self.clip else {
            return [0.; 2];
        };
        resample::read(&clip.samples, self.position, self.jog_speed.abs().max(1.))
            .map(|v| v * level)
    }
    fn frame(&mut self, sr: u32) -> [f32; 2] {
        let step = self.step(sr);
        let forward = self.forward_step(sr);
        let Some(len) = self.clip.as_ref().map(|clip| clip.samples.len() as f64) else {
            return [0.; 2];
        };
        let mut input = [0.; 2];
        if self.spin_left > 0 && self.playing {
            // A backspin throws the record backward and lets it die out.
            let left = self.spin_left as f64 / self.spin_total as f64;
            let speed = BACKSPIN_SPEED * left * left;
            self.position = (self.position - speed).max(0.);
            let level = (left * 12.).min(1.) as f32;
            if let Some(clip) = &self.clip {
                input =
                    resample::read(&clip.samples, self.position, speed.max(1.)).map(|v| v * level);
            }
            self.spin_left -= 1;
            if self.spin_left == 0 {
                self.stop_now(sr);
            }
        } else if self.jog_touch {
            input = self.scratch(len);
            if self.excursion && self.playing {
                self.shadow += forward;
            }
        } else if self.playing {
            if self.excursion {
                self.shadow += forward;
            }
            if let Some(clip) = self
                .clip
                .as_ref()
                .filter(|_| self.position < len && self.position >= 0.)
            {
                input = match &mut self.pitch {
                    Some(voices) if self.main_pitched => {
                        let voice = &mut voices[self.main_pitch];
                        if std::mem::take(&mut self.restart_pitch) {
                            voice.start(
                                &clip.samples,
                                self.position,
                                step,
                                clip.sample_rate as f64 / sr as f64,
                                self.tuning.pitch_ratio(),
                            );
                        }
                        voice.next(&clip.samples, self.position, step)
                    }
                    _ => resample::read(&clip.samples, self.position, step.abs()),
                };
                self.position += step;
                self.wrap_loop(step);
                if self.position >= len {
                    self.position = len;
                    self.playing = false;
                } else if step < 0. && self.position <= 0. {
                    self.position = 0.;
                    self.playing = false;
                }
            } else {
                self.playing = false;
            }
            if self.braking {
                self.ramp -= 1. / (self.brake_seconds.max(0.01) as f64 * sr as f64);
                if self.ramp <= 0. {
                    self.ramp = 1.;
                    self.braking = false;
                    self.stop_now(sr);
                }
            } else if self.ramp < 1. {
                self.ramp = (self.ramp + 1. / (SPIN_UP_SECONDS * sr as f64)).min(1.);
            }
        }
        if self.fade_left > 0 {
            let gain = self.fade_gain();
            if self.fade_main {
                input = input.map(|value| value * gain);
            }
            let tail = self.tail_gain * (1. - gain);
            let source = if self.tail_reads_clip {
                &self.tail_clip
            } else {
                &self.clip
            };
            if let (true, Some(source)) = (tail > 0., source) {
                let old = match &mut self.pitch {
                    Some(voices) if self.tail_pitched => voices[1 - self.main_pitch].next(
                        &source.samples,
                        self.tail_position,
                        self.tail_step,
                    ),
                    _ => resample::read(&source.samples, self.tail_position, self.tail_step.abs()),
                };
                input = [input[0] + old[0] * tail, input[1] + old[1] * tail];
            }
            self.tail_position += self.tail_step;
            self.fade_left -= 1;
            if self.fade_left == 0 {
                self.tail_gain = 0.;
                self.tail_reads_clip = false;
                self.tail_pitched = false;
                self.fade_main = true;
            }
        }
        if self.echo_out_left > 0 {
            self.echo_out_left -= 1;
            if self.echo_out_left == 0 {
                self.stop_now(sr);
                if let Some(channel) = &mut self.effects.channel {
                    let settings = channel.beat.settings();
                    channel.beat.set(BeatSettings {
                        on: false,
                        ..settings
                    });
                }
            }
        } else if self.echo_restore_left > 0 {
            self.echo_restore_left -= 1;
            if self.echo_restore_left == 0 {
                if let (Some(channel), Some(saved)) =
                    (&mut self.effects.channel, self.echo_restore.take())
                {
                    channel.beat.set(saved);
                }
            }
        }
        let out = self.effects.process(input, self.params);
        self.peak = self.peak.max(out[0].abs()).max(out[1].abs());
        out
    }
    /// Effects that end a track on the way out, on the pads. Each leaves the deck stopped except
    /// Build Up and Ducking, which last as long as the pad is held.
    fn release_fx(&mut self, kind: u8, sr: u32) {
        if !self.playing {
            return;
        }
        match kind {
            0 | 1 => {
                self.braking = true;
                self.brake_seconds = if kind == 0 { BRAKE_SHORT } else { BRAKE_LONG };
            }
            2 | 3 => {
                let seconds = if kind == 2 {
                    BACKSPIN_SHORT
                } else {
                    BACKSPIN_LONG
                };
                self.spin_total = (seconds * sr as f64) as u32;
                self.spin_left = self.spin_total;
                let step = self.step(sr);
                self.release_voice(self.jump_frames, true, step);
            }
            4 => {
                let frames_per_beat = self.clock(sr) as f64;
                if let Some(channel) = &mut self.effects.channel {
                    let saved = channel.beat.settings();
                    if self.echo_restore.is_none() {
                        self.echo_restore = Some(saved);
                    }
                    channel.beat.set(BeatSettings {
                        kind: BeatKind::Echo,
                        on: true,
                        beats: 0.5,
                        depth: 1.,
                        bpm: saved.bpm,
                    });
                    // Half a beat of feed, then the track goes and the repeats carry on.
                    self.echo_out_left = (frames_per_beat * 0.5) as u32;
                    self.echo_restore_left = (ECHO_OUT_TAIL_SECONDS * sr as f64) as u32;
                }
            }
            5 => self.building = true,
            6 => self.stop_now(sr),
            _ => self.ducking = true,
        }
    }
    /// Per block: what the held Release FX do to the channel.
    fn hold_release(&mut self, frames: f32, sr: u32) {
        if self.building {
            let beat = self.clock(sr).max(1.);
            self.build = (self.build + frames / (beat * BUILD_UP_BEATS)).min(1.);
            self.effects
                .set_highpass(self.params.high_pass.max(self.build * 0.6));
        }
        if self.ducking {
            let phase = self.beats_position().unwrap_or(0.).rem_euclid(1.) as f32;
            self.params.pump = 0.12 + 0.88 * (phase * 1.6).min(1.);
        }
    }
    fn control(&mut self, c: Control, sr: u32) {
        let len = self.clip.as_ref().map_or(0., |c| c.samples.len() as f64);
        let rate = self.clip.as_ref().map_or(sr, |c| c.sample_rate) as f64;
        match c {
            Control::Play => {
                if self.position >= len {
                    self.position = 0.;
                }
                if !self.playing && self.clip.is_some() {
                    self.preroll_to(self.position, sr);
                    self.playing = true;
                }
                // Pressing play during a brake catches the record again.
                self.braking = false;
            }
            Control::Pause => self.stop(sr),
            Control::Toggle => {
                if self.playing && !self.braking {
                    self.stop(sr);
                } else {
                    self.control(Control::Play, sr);
                }
            }
            Control::Seek(t) => {
                self.excursion = self.rolling;
                self.jump((t * rate).clamp(0., len), sr);
            }
            Control::Nudge(t) => {
                self.shadow += t * rate;
                self.jump((self.position + t * rate).clamp(0., len), sr);
            }
            Control::Cue2(on) => self.cue = on,
            Control::Release(kind) => self.release_fx(kind, sr),
            Control::ReleaseEnd => {
                if self.building {
                    self.building = false;
                    self.build = 0.;
                    self.effects.set_highpass(self.params.high_pass);
                }
                self.ducking = false;
                self.params.pump = 1.;
            }
            Control::Slip(on) => {
                self.slip = on;
                if !on && !self.rolling {
                    self.excursion = false;
                }
            }
            Control::Reverse(on) => {
                if on != self.reverse {
                    if self.playing {
                        let step = self.step(sr);
                        self.release_voice(self.jump_frames, true, step);
                    }
                    self.reverse = on;
                }
            }
            Control::Roll(beats) => self.start_roll(beats, len, rate),
            Control::RollEnd => self.end_roll(sr),
            Control::BeatJump(beats) => {
                let delta = self.span_from(self.position, beats, rate);
                let target = (self.position + delta).clamp(0., len);
                if self.looping && (self.loop_in + delta < 0. || self.loop_out + delta > len) {
                    self.looping = false;
                } else if self.loop_out > self.loop_in {
                    self.loop_in += delta;
                    self.loop_out += delta;
                }
                self.shadow += delta;
                if self.playing {
                    let step = self.step(sr);
                    self.release_voice(self.jump_frames, true, step);
                }
                self.position = target;
            }
            Control::LoopScale(factor) => {
                if self.loop_out > self.loop_in {
                    let span = (self.loop_out - self.loop_in) * factor;
                    // A slice shorter than its own fades would only be those fades.
                    if span >= self.loop_frames as f64 * 2. && self.loop_in + span <= len {
                        self.loop_out = self.loop_in + span;
                        self.loop_beats *= factor;
                    }
                }
            }
            Control::JogTouch(on) => {
                if on && !self.jog_touch {
                    self.jog_target = self.position;
                    self.jog_speed = if self.playing { self.step(sr) } else { 0. };
                    if self.playing {
                        let step = self.step(sr);
                        self.release_voice(self.jump_frames, true, step);
                    }
                    self.jog_touch = true;
                } else if !on && self.jog_touch {
                    self.jog_touch = false;
                    if self.playing {
                        self.release_voice(self.jump_frames, true, self.jog_speed);
                    }
                }
            }
            Control::JogMove(seconds) => {
                if self.jog_touch {
                    self.jog_target = (self.jog_target + seconds * rate).clamp(0., len);
                }
            }
            Control::JogBend(amount) => self.bend = amount,
            Control::Brake(seconds) => self.brake = seconds,
            Control::Trim(v) => self.params.trim = v,
            Control::Color(v) => {
                if let Some(channel) = &mut self.effects.channel {
                    channel.color.set_amount(v);
                }
            }
            Control::ColorFx(kind) => {
                if let Some(channel) = &mut self.effects.channel {
                    channel.color.set_kind(ColorKind::from_index(kind));
                }
            }
            Control::ColorParam(v) => {
                if let Some(channel) = &mut self.effects.channel {
                    channel.color.set_param(v);
                }
            }
            Control::SetCue(i) => {
                if let Some(clip) = &self.clip {
                    let at = self.snap(self.position, f64::round).min(len) / rate;
                    clip.cues.set(i, at.max(CUE_AT_START));
                }
            }
            Control::ClearCue(i) => {
                if let Some(clip) = &self.clip {
                    clip.cues.set(i, 0.);
                }
            }
            Control::Cue(i) => self.trigger_cue(i, false, sr),
            Control::Trigger => {
                self.preroll_to(0., sr);
                self.leave_loop_if_outside(0.);
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
                self.loop_in = self.snap(self.position, f64::round);
                self.looping = false;
            }
            Control::LoopOut => {
                self.loop_out = match self.grid().filter(|_| self.quantize) {
                    Some((_, beat)) => {
                        let nearest = self.snap(self.position, f64::round);
                        let out = if nearest > self.loop_in {
                            nearest
                        } else {
                            self.loop_in + beat
                        };
                        out.min(self.snap(len, f64::floor))
                    }
                    None => self.position.min(len),
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
                self.effects.set_filter(v);
            }
            Control::Echo(v) => self.params.echo = v,
            Control::HighPass(v) => {
                self.params.high_pass = v;
                self.effects.set_highpass(v);
            }
            Control::EchoBeats(v) => self.echo_beats = v,
            _ => {}
        }
        self.sync_slip(sr);
    }
}
/// The beat at or before a time, always leaving a next one to measure against.
fn map_index(beats: &[f64], seconds: f64) -> usize {
    beats
        .partition_point(|beat| *beat <= seconds)
        .saturating_sub(1)
        .min(beats.len() - 2)
}
pub struct Renderer {
    shared: Arc<Shared>,
    decks: [Deck; SLOTS],
    sample_rate: u32,
    crossfader: f32,
    mix: [f32; 2],
    master: f32,
    master_target: f32,
    level_smoothing: f32,
    limiter: Limiter,
    limiter_db: f32,
    limiter_hold: usize,
    scope: [f32; 64],
    scope_head: usize,
    visual_capture: [f32; VISUAL_SAMPLES],
    visual_head: usize,
    visual_request: usize,
    master_deck: usize,
    master_fx: Box<BeatFx>,
    sampler: f32,
    sampler_level: f32,
    auto_gain: bool,
    bend_relax: f32,
    crossfader_curve: u8,
    cue_master: bool,
    cue_mix: f32,
    cue_level: f32,
}
pub fn engine(sample_rate: u32) -> (EngineHandle, Renderer) {
    resample::prepare();
    let shared = Arc::new(Shared {
        commands: ArrayQueue::new(256),
        retired: ArrayQueue::new(256),
        meters: std::array::from_fn(|_| Meter::default()),
        pending_loads: AtomicUsize::new(0),
        next_load: AtomicU64::new(0),
        stream_error: AtomicBool::new(false),
        callbacks: AtomicU64::new(0),
        clipped: AtomicU64::new(0),
        limited: AtomicU64::new(0),
        limiter_db: AtomicU32::new(0),
        peak: AtomicU32::new(0),
        rms: AtomicU32::new(0),
        waveform: std::array::from_fn(|_| AtomicU32::new(0)),
        visual_state: AtomicUsize::new(0),
        visual_next: AtomicUsize::new(1),
        visual_samples: std::array::from_fn(|_| AtomicU32::new(0)),
        visual_rate: sample_rate,
        crossfader: AtomicU32::new(0.5_f32.to_bits()),
        master: AtomicU32::new(0.75_f32.to_bits()),
        master_deck: AtomicUsize::new(0),
        beat_meters: Default::default(),
        sampler: AtomicU32::new(1_f32.to_bits()),
        auto_gain: AtomicBool::new(false),
        crossfader_curve: AtomicU32::new(1),
        cue_master: AtomicBool::new(false),
        cue_mix: AtomicU32::new(0_f32.to_bits()),
        cue_level: AtomicU32::new(0.7_f32.to_bits()),
    });
    (
        EngineHandle(shared.clone()),
        Renderer {
            shared,
            decks: std::array::from_fn(|slot| Deck::new(sample_rate, slot < 2)),
            sample_rate,
            crossfader: 0.5,
            mix: dsp::crossfade(0.5, 1),
            crossfader_curve: 1,
            cue_master: false,
            cue_mix: 0.,
            cue_level: 0.7,
            master: 0.,
            master_target: 0.75,
            level_smoothing: 1. - (-1. / (sample_rate as f32 * 0.005)).exp(),
            limiter: Limiter::new(sample_rate),
            limiter_db: 0.,
            limiter_hold: 0,
            scope: [0.; 64],
            scope_head: 0,
            visual_capture: [0.; VISUAL_SAMPLES],
            visual_head: 0,
            visual_request: 0,
            master_deck: 0,
            master_fx: Box::new(BeatFx::new(sample_rate)),
            sampler: 1.,
            sampler_level: 1.,
            auto_gain: false,
            bend_relax: (-1. / (sample_rate as f32 * BEND_RELAX_SECONDS)).exp(),
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
        let (Some(a), Some(b)) = (
            self.decks[master].local_bpm(),
            self.decks[other].local_bpm(),
        ) else {
            self.decks[other].sync = false;
            return;
        };
        let mut rate = (a * self.decks[master].tuning.rate as f64 / b) as f32;
        // On a moving tempo, matching speeds alone would let the beats wander apart. A small
        // pull closes the gap without a jump, and a steady grid has no use for it.
        if self.decks[master].beat_map().is_some() || self.decks[other].beat_map().is_some() {
            if let (Some(lead), Some(follow)) = (
                self.decks[master].beats_position(),
                self.decks[other].beats_position(),
            ) {
                let gap = (follow - lead + 0.5).rem_euclid(1.) - 0.5;
                rate *= 1. - (gap as f32 * 0.05).clamp(-0.01, 0.01);
            }
        }
        if !(0.5..=1.5).contains(&rate) {
            self.decks[other].sync = false;
            return;
        }
        let tuning = Tuning {
            rate,
            ..self.decks[other].tuning
        };
        self.decks[other].apply_tuning(tuning, self.sample_rate);
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
        let sample_rate = self.sample_rate;
        let d = &mut self.decks[other];
        // A paused follower keeps the position it was given and aligns when it starts.
        if !d.playing {
            return;
        }
        // Nearest beat: a small phase error must never become a jump of almost a whole beat.
        let beat = ((d.position - fa) / fb - phase).round();
        let mut position = (fa + (beat + phase) * fb).max(0.);
        if d.looping && d.loop_out > d.loop_in {
            position = d.loop_in + (position - d.loop_in).rem_euclid(d.loop_out - d.loop_in);
        }
        let len = d.clip.as_ref().map_or(0., |c| c.samples.len() as f64);
        let position = position.min((len - 1.).max(0.));
        if (position - d.position).abs() > 1. {
            let step = d.step(sample_rate);
            d.release_voice(d.jump_frames, true, step);
        }
        d.position = position;
    }
    pub fn render<T: cpal::Sample + cpal::FromSample<f32>>(
        &mut self,
        output: &mut [T],
        channels: usize,
    ) {
        // Bounded work: at most 64 control events per callback. Do not pop a Load if reclamation is full.
        let mut align = false;
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
                    if !d.playing {
                        d.effects.snap_gain(d.params.gain);
                    }
                    d.install_clip(clip, &self.shared.retired, self.sample_rate);
                    d.load_ticket = ticket;
                    d.music = d.clip.as_ref().unwrap().music;
                    d.tuning = Default::default();
                    d.params.rate = 1.;
                    d.main_pitched = false;
                    d.sync = false;
                    d.scheduled = None;
                    d.position = 0.;
                    d.playing = play;
                    d.fade_main = false;
                    d.looping = false;
                    d.loop_in = 0.;
                    d.loop_out = 0.;
                    d.reverse = false;
                    d.rolling = false;
                    d.excursion = false;
                    d.jog_touch = false;
                    d.braking = false;
                    d.ramp = 1.;
                    d.bend = 0.;
                    d.spin_left = 0;
                    d.echo_out_left = 0;
                    d.building = false;
                    d.build = 0.;
                    d.ducking = false;
                    d.params.pump = 1.;
                    d.effects.set_highpass(d.params.high_pass);
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
                            self.decks[slot].apply_tuning(t, self.sample_rate);
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
                    self.decks[slot].apply_tuning(t, self.sample_rate);
                }
                Command::Control(slot, Control::TransposeStep(step)) => {
                    let t = Tuning {
                        semitones: (self.decks[slot].tuning.semitones + step).clamp(-12, 12),
                        ..self.decks[slot].tuning
                    };
                    self.decks[slot].apply_tuning(t, self.sample_rate);
                }
                Command::Control(slot, Control::Transpose(semitones)) => {
                    let t = Tuning {
                        semitones,
                        ..self.decks[slot].tuning
                    };
                    self.decks[slot].apply_tuning(t, self.sample_rate);
                }
                Command::Control(slot, Control::KeyLock(key_lock)) => {
                    let t = Tuning {
                        key_lock,
                        ..self.decks[slot].tuning
                    };
                    self.decks[slot].apply_tuning(t, self.sample_rate);
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
                    self.decks[slot].apply_tuning(t, self.sample_rate);
                }
                Command::Control(_, Control::Crossfader(v)) => {
                    self.crossfader = v;
                }
                Command::Control(_, Control::Master(v)) => self.master_target = v,
                Command::Control(_, Control::Sampler(v)) => self.sampler_level = v,
                Command::Control(_, Control::AutoGain(on)) => self.auto_gain = on,
                Command::Control(_, Control::CueMaster(on)) => self.cue_master = on,
                Command::Control(_, Control::CueMix(v)) => self.cue_mix = v,
                Command::Control(_, Control::CueLevel(v)) => self.cue_level = v,
                Command::Control(_, Control::ToggleBoth) => {
                    let control = if self.decks[..2].iter().any(|deck| deck.playing) {
                        Control::Pause
                    } else {
                        Control::Play
                    };
                    for deck in &mut self.decks[..2] {
                        deck.scheduled = None;
                        deck.control(control, self.sample_rate);
                    }
                    align = true;
                }
                Command::Control(_, Control::CrossfaderCurve(curve)) => {
                    self.crossfader_curve = curve;
                }
                Command::Control(
                    slot,
                    c @ (Control::BeatFx(_)
                    | Control::BeatFxOn(_)
                    | Control::BeatFxBeats(_)
                    | Control::BeatFxDepth(_)
                    | Control::BeatFxBpm(_)),
                ) => {
                    if let Some(unit) = self.beat_unit(slot) {
                        let mut settings = unit.settings();
                        match c {
                            Control::BeatFx(kind) => settings.kind = BeatKind::from_index(kind),
                            Control::BeatFxOn(on) => settings.on = on,
                            Control::BeatFxBeats(beats) => settings.beats = beats,
                            Control::BeatFxDepth(depth) => settings.depth = depth,
                            Control::BeatFxBpm(bpm) => settings.bpm = bpm,
                            _ => unreachable!(),
                        }
                        unit.set(settings);
                    }
                }
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
                            deck.stop_now(self.sample_rate);
                        }
                        // The player has no controls for these, so nothing may stay engaged.
                        for deck in &mut self.decks[..2] {
                            deck.end_roll(self.sample_rate);
                            deck.jog_touch = false;
                            deck.reverse = false;
                            deck.slip = false;
                            deck.excursion = false;
                            deck.bend = 0.;
                            if let Some(channel) = &mut deck.effects.channel {
                                channel.color.set_kind(ColorKind::Off);
                                channel.color.set_amount(0.);
                                let settings = channel.beat.settings();
                                channel.beat.set(BeatSettings {
                                    on: false,
                                    ..settings
                                });
                            }
                        }
                        let settings = self.master_fx.settings();
                        self.master_fx.set(BeatSettings {
                            on: false,
                            ..settings
                        });
                    }
                }
                Command::Control(_, Control::Panic) => {
                    for d in &mut self.decks {
                        d.stop_now(self.sample_rate);
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
                            let lead = self.scheduled_master_position(slot)
                                - self.decks[self.master_deck].position;
                            let current = self.decks[self.master_deck]
                                .beats_position()
                                .unwrap_or((self.decks[self.master_deck].position - offset) / beat)
                                + lead / beat;
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
                            | Control::BeatJump(_)
                            | Control::RollEnd
                    ) {
                        self.decks[slot].scheduled = None;
                        // A nudge corrects the relative phase by ear: aligning would undo it.
                        if slot < 2 && !matches!(c, Control::Nudge(_) | Control::Pause) {
                            align = true;
                        }
                        if slot == self.master_deck
                            && matches!(
                                c,
                                Control::Cue(_)
                                    | Control::Seek(_)
                                    | Control::Trigger
                                    | Control::BeatJump(_)
                            )
                        {
                            self.decks[1 - slot].scheduled = None;
                        }
                    }
                    self.decks[slot].control(c, self.sample_rate);
                }
            }
        }
        self.refresh_sync();
        if align {
            self.align_sync();
        }
        let target_mix = dsp::crossfade(self.crossfader, self.crossfader_curve);
        let mut peak = 0.0_f32;
        let mut squares = 0.0_f64;
        let mut clipped = 0;
        let mut lowest_gain = 1_f32;
        let mut limited = 0;
        let block = (output.len() / channels.max(1)) as i32;
        let relax = self.bend_relax.powi(block);
        for i in 0..BEAT_UNITS {
            let settings = self.beat_unit(i).map(|unit| unit.settings());
            let meter = &self.shared.beat_meters[i];
            if let Some(settings) = settings {
                meter.kind.store(settings.kind.index() as u32, Relaxed);
                meter.on.store(settings.on, Relaxed);
                meter.beats.store(settings.beats.to_bits(), Relaxed);
                meter.depth.store(settings.depth.to_bits(), Relaxed);
                meter.bpm.store(settings.bpm.to_bits(), Relaxed);
            }
        }
        let (master_beat, master_position) = {
            let lead = &self.decks[self.master_deck];
            (lead.clock(self.sample_rate), lead.beats_position())
        };
        self.master_fx.set_clock(master_beat, master_position);
        for (i, d) in self.decks.iter_mut().enumerate() {
            d.peak = 0.;
            d.bend *= relax;
            if d.bend.abs() < 1e-3 {
                d.bend = 0.;
            }
            d.effects.set_echo_time(
                60. / (d.local_bpm().unwrap_or(120.) as f32 * d.params.rate) * d.echo_beats,
            );
            d.refresh_pitch(self.sample_rate);
            d.hold_release(block as f32, self.sample_rate);
            d.params.auto = match (
                self.auto_gain && i < 2,
                d.clip.as_ref().and_then(|c| c.loudness),
            ) {
                (true, Some(loudness)) => {
                    let change =
                        (AUTO_GAIN_TARGET - loudness).clamp(-AUTO_GAIN_RANGE, AUTO_GAIN_RANGE);
                    10_f32.powf(change / 20.)
                }
                _ => 1.,
            };
            let clock = d.clock(self.sample_rate);
            let beats = d.beats_position();
            if let Some(channel) = &mut d.effects.channel {
                channel.color.set_tempo(clock);
                channel.beat.set_clock(clock, beats);
            }
            // Finished fades give their PCM back here, never dropping it in the callback. If
            // reclamation has no room, the clip simply waits for the next block.
            let finished = d.fade_left == 0 || !d.tail_reads_clip;
            if finished && self.shared.retired.capacity() - self.shared.retired.len() > 2 {
                if let Some(old) = d.tail_clip.take() {
                    let _ = self.shared.retired.push(old);
                }
            }
        }
        let request = self
            .shared
            .visual_state
            .load(std::sync::atomic::Ordering::Acquire);
        let capturing = request != 0 && request & 1 == 1;
        if request != self.visual_request {
            self.visual_head = 0;
            self.visual_request = request;
        }
        for frame in output.chunks_mut(channels.max(1)) {
            let follower = 1 - self.master_deck;
            if let Some((control, queued_beat)) = self.decks[follower].scheduled {
                let master = &self.decks[self.master_deck];
                if !master.playing {
                    self.decks[follower].scheduled = None;
                }
                let lead = self.scheduled_master_position(follower)
                    - self.decks[self.master_deck].position;
                let master = &self.decks[self.master_deck];
                if master.playing
                    && master.grid().is_some_and(|(_, beat)| {
                        let current = master.beats_position().unwrap_or(0.) + lead / beat;
                        current.floor() as i64 != queued_beat
                    })
                {
                    self.decks[follower].scheduled = None;
                    // The pre-roll started here makes the cue land exactly on the master beat.
                    if let Control::Cue(index) = control {
                        self.decks[follower].trigger_cue(index, true, self.sample_rate);
                    }
                }
            }
            self.master += (self.master_target - self.master) * self.level_smoothing;
            self.sampler += (self.sampler_level - self.sampler) * self.level_smoothing;
            for (i, target) in target_mix.iter().enumerate() {
                self.mix[i] += (target - self.mix[i]) * self.level_smoothing;
            }
            let mut stereo = [0.; 2];
            let mut cue = [0.; 2];
            let cue_channels = channels >= 4;
            for (i, deck) in self.decks.iter_mut().enumerate() {
                let signal = deck.frame(self.sample_rate);
                if i == PREVIEW_SLOT {
                    if cue_channels {
                        cue[0] += signal[0];
                        cue[1] += signal[1];
                    } else {
                        stereo[0] += signal[0] * PREVIEW_LEVEL;
                        stereo[1] += signal[1] * PREVIEW_LEVEL;
                    }
                    continue;
                }
                let weight = if i < 2 {
                    self.mix[i]
                } else {
                    0.5 * self.sampler
                };
                for c in 0..2 {
                    stereo[c] += signal[c] * weight;
                }
                if i < 2 && deck.cue {
                    cue[0] += deck.effects.pre_fader[0];
                    cue[1] += deck.effects.pre_fader[1];
                }
            }
            let stereo = self.master_fx.process(stereo);
            let (mut stereo, gain) = self
                .limiter
                .process(stereo.map(|value| value * self.master));
            if gain < 1. {
                limited += 1;
                lowest_gain = lowest_gain.min(gain);
            }
            for x in &mut stereo {
                if x.abs() > limiter::CEILING * 1.001 {
                    clipped += 1;
                }
                *x = x.clamp(-limiter::CEILING, limiter::CEILING);
                peak = peak.max(x.abs());
                squares += (*x as f64).powi(2);
            }
            self.scope[self.scope_head % 64] = (stereo[0] + stereo[1]) * 0.5;
            self.scope_head = (self.scope_head + 1) % 64;
            let mono = (stereo[0] + stereo[1]) * 0.5;
            if capturing && self.visual_head < VISUAL_SAMPLES {
                self.visual_capture[self.visual_head] = mono;
                self.visual_head += 1;
            }
            // With no cue selected the headphones still hear the mix, as on a mixer.
            let solo = if self.cue_master { 1. } else { self.cue_mix };
            let phones = [
                ((cue[0] * (1. - solo) + stereo[0] * solo) * self.cue_level).clamp(-1., 1.),
                ((cue[1] * (1. - solo) + stereo[1] * solo) * self.cue_level).clamp(-1., 1.),
            ];
            for (c, sample) in frame.iter_mut().enumerate() {
                *sample = T::from_sample(if channels == 1 {
                    mono
                } else if c < 2 {
                    stereo[c]
                } else if c < 4 {
                    phones[c - 2]
                } else {
                    0.
                });
            }
        }
        if capturing && self.visual_head == VISUAL_SAMPLES {
            for (dst, sample) in self.shared.visual_samples.iter().zip(self.visual_capture) {
                dst.store(sample.to_bits(), Relaxed);
            }
            let _ = self.shared.visual_state.compare_exchange(
                request,
                request + 1,
                std::sync::atomic::Ordering::Release,
                Relaxed,
            );
            self.visual_head = 0;
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
            for j in 0..HOT_CUES {
                let cues = d.clip.as_ref().map(|c| &c.cues);
                m.cues[j].store(cues.map_or(0., |c| c.get(j)).to_bits(), Relaxed);
                m.cue_ends[j].store(cues.map_or(0., |c| c.end(j)).to_bits(), Relaxed);
            }
            m.load_ticket
                .store(d.load_ticket, std::sync::atomic::Ordering::Release);
            m.loop_in.store((d.loop_in / sr).to_bits(), Relaxed);
            m.loop_out.store((d.loop_out / sr).to_bits(), Relaxed);
            m.music.store(d.music);
            m.tuning.store(d.tuning);
            m.quantize.store(d.quantize, Relaxed);
            m.sync.store(d.sync, Relaxed);
            m.loop_beats.store(d.loop_beats.to_bits(), Relaxed);
            m.queued.store(d.scheduled.is_some(), Relaxed);
            m.trim.store(d.params.trim.to_bits(), Relaxed);
            m.brake.store(d.brake.to_bits(), Relaxed);
            m.slip.store(d.slip, Relaxed);
            m.reverse.store(d.reverse, Relaxed);
            m.rolling.store(d.rolling, Relaxed);
            m.cue.store(d.cue, Relaxed);
            if let Some(channel) = &d.effects.channel {
                m.color.store(channel.color.knob().to_bits(), Relaxed);
                m.color_param
                    .store(channel.color.parameter().to_bits(), Relaxed);
                m.color_fx
                    .store(channel.color.selected().index() as u32, Relaxed);
            }
        }
        self.shared
            .sampler
            .store(self.sampler_level.to_bits(), Relaxed);
        self.shared.auto_gain.store(self.auto_gain, Relaxed);
        self.shared.cue_master.store(self.cue_master, Relaxed);
        self.shared.cue_mix.store(self.cue_mix.to_bits(), Relaxed);
        self.shared
            .cue_level
            .store(self.cue_level.to_bits(), Relaxed);
        self.shared
            .crossfader_curve
            .store(self.crossfader_curve as u32, Relaxed);
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
        self.shared.master_deck.store(self.master_deck, Relaxed);
        self.shared.clipped.fetch_add(clipped, Relaxed);
        self.shared.limited.fetch_add(limited, Relaxed);
        let frames = output.len() / channels.max(1);
        let reduction = if lowest_gain < 1. {
            -20. * lowest_gain.log10()
        } else {
            0.
        };
        if reduction >= self.limiter_db {
            self.limiter_db = reduction;
            self.limiter_hold = self.sample_rate as usize / 5;
        } else if self.limiter_hold > frames {
            self.limiter_hold -= frames;
        } else {
            self.limiter_hold = 0;
            let fall = frames as f32 / self.sample_rate as f32 * 20.;
            self.limiter_db = (self.limiter_db - fall).max(reduction);
        }
        self.shared
            .limiter_db
            .store(self.limiter_db.to_bits(), Relaxed);
    }
    fn beat_unit(&mut self, slot: usize) -> Option<&mut BeatFx> {
        if slot == MASTER_UNIT {
            return Some(&mut self.master_fx);
        }
        self.decks[slot]
            .effects
            .channel
            .as_mut()
            .map(|channel| &mut channel.beat)
    }
    /// Where the master will be when a cue pre-rolled now on `slot` reaches its target.
    fn scheduled_master_position(&self, slot: usize) -> f64 {
        let master = &self.decks[self.master_deck];
        master.position + self.decks[slot].jump_frames as f64 * master.step(self.sample_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn visual_capture_is_complete_bounded_and_cancelled_on_restart() {
        let (handle, mut renderer) = engine(48000);
        assert!(handle.visual_samples().is_none());
        render(&mut renderer, 1024);
        assert!(handle.visual_samples().is_none());
        handle.stop_visual_capture();
        assert!(handle.visual_samples().is_none());
        render(&mut renderer, 1024);
        assert!(handle.visual_samples().is_none());
        render(&mut renderer, 1024);
        let (samples, rate) = handle.visual_samples().unwrap();
        assert_eq!(rate, 48000);
        assert!(samples.iter().all(|sample| *sample == 0.));
        // No request can build an unbounded backlog when the reader is late.
        render(&mut renderer, 8192);
        assert!(handle.visual_samples().is_some());
        assert!(handle.visual_samples().is_none());
        handle.load(0, clip(9)).unwrap();
        handle.control(0, Control::Play).unwrap();
        let output = render(&mut renderer, VISUAL_SAMPLES);
        let (captured, _) = handle.visual_samples().unwrap();
        assert!(captured.iter().any(|sample| *sample > 0.01));
        for (i, sample) in captured.iter().enumerate() {
            assert_eq!(*sample, (output[i * 2] + output[i * 2 + 1]) * 0.5);
        }
        handle.stop_visual_capture();
    }
    fn clip(id: u64) -> Arc<Clip> {
        Arc::new(Clip {
            loudness: None,
            beats: None,
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
        let weights = dsp::crossfade(0.5, 1);
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
        let target = 0.1 - JUMP_FADE_SECONDS;
        assert!((h.snapshot().slots[1].position - target).abs() < 0.001);
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
                slot: SLOTS,
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
        assert!((s.slots[1].position - (0.21 - JUMP_FADE_SECONDS)).abs() < 0.00001);
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
        assert!(h.control(0, Control::CueBoth(HOT_CUES)).is_err());
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
#[cfg(test)]
mod transport_tests;
