pub mod annotations;
pub mod decode;
mod dsp;
pub mod output;
use crossbeam_queue::ArrayQueue;
use decode::Clip;
use dsp::{Effects, Params};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering::Relaxed},
    Arc,
};

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
            "gain" if (0.0..=1.5).contains(&v) => Control::Gain(v),
            "rate" if (0.5..=1.5).contains(&v) => Control::Rate(v),
            "low" if (0.0..=2.0).contains(&v) => Control::Low(v),
            "mid" if (0.0..=2.0).contains(&v) => Control::Mid(v),
            "high" if (0.0..=2.0).contains(&v) => Control::High(v),
            "filter" if (0.0..=1.0).contains(&v) => Control::Filter(v),
            "echo" if (0.0..=1.0).contains(&v) => Control::Echo(v),
            "crossfader" if (0.0..=1.0).contains(&v) => Control::Crossfader(v),
            "master" if (0.0..=1.0).contains(&v) => Control::Master(v),
            "panic" => Control::Panic,
            _ => return Err(format!("Control fuera de rango: {}", self.kind)),
        })
    }
}
pub enum Command {
    Load(usize, Arc<Clip>, u64),
    Control(usize, Control),
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
    cues: [AtomicU64; 3],
    loop_in: AtomicU64,
    loop_out: AtomicU64,
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
    pub cues: [f64; 3],
    pub loop_in: f64,
    pub loop_out: f64,
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
}
impl EngineHandle {
    pub fn action(&self, action: Action) -> Result<(), String> {
        let c = action.validate()?;
        self.control(action.slot, c)
    }
    pub fn control(&self, slot: usize, control: Control) -> Result<(), String> {
        if slot >= SLOTS || matches!(control, Control::Cue(i)|Control::SetCue(i) if i>=3) {
            return Err("Deck o cue inválido".into());
        }
        self.0
            .commands
            .push(Command::Control(slot, control))
            .map_err(|_| "Cola de audio llena; vuelve a intentar".into())
    }
    pub fn load(&self, slot: usize, clip: Arc<Clip>) -> Result<u64, String> {
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
            .push(Command::Load(slot, clip, ticket))
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
                    cues: std::array::from_fn(|i| f64::from_bits(m.cues[i].load(Relaxed))),
                    loop_in: f64::from_bits(m.loop_in.load(Relaxed)),
                    loop_out: f64::from_bits(m.loop_out.load(Relaxed)),
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
}
impl Deck {
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
            Control::Seek(t) => self.position = (t * rate).clamp(0., len),
            Control::Nudge(t) => self.position = (self.position + t * rate).clamp(0., len),
            Control::SetCue(i) => {
                if let Some(clip) = &self.clip {
                    clip.cues.set(i, self.position / rate);
                }
            }
            Control::Cue(i) => {
                if let Some(clip) = &self.clip {
                    self.position = (clip.cues.get(i) * rate).clamp(0., len);
                }
            }
            Control::Trigger => {
                self.position = 0.;
                self.playing = self.clip.is_some();
            }
            Control::Loop => {
                if self.loop_out <= self.loop_in {
                    self.loop_in = self.position;
                    self.loop_out = (self.position + rate * 4.).min(len);
                }
                self.looping = !self.looping && self.loop_out > self.loop_in;
            }
            Control::LoopIn => {
                self.loop_in = self.position;
                self.looping = false;
            }
            Control::LoopOut => {
                self.loop_out = self.position;
                self.looping = self.loop_out > self.loop_in;
            }
            Control::Gain(v) => self.params.gain = v,
            Control::Rate(v) => self.params.rate = v,
            Control::Low(v) => self.params.low = v,
            Control::Mid(v) => self.params.mid = v,
            Control::High(v) => self.params.high = v,
            Control::Filter(v) => {
                self.params.filter = v;
                self.effects.set_filter(v, sr);
            }
            Control::Echo(v) => self.params.echo = v,
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
        },
    )
}
impl Renderer {
    pub fn render<T: cpal::Sample + cpal::FromSample<f32>>(
        &mut self,
        output: &mut [T],
        channels: usize,
    ) {
        // Bounded work: at most 64 control events per callback. Do not pop a Load if reclamation is full.
        for _ in 0..64 {
            if self.shared.retired.is_full() {
                break;
            }
            let Some(command) = self.shared.commands.pop() else {
                break;
            };
            match command {
                Command::Load(slot, clip, ticket) => {
                    self.shared.pending_loads.fetch_sub(1, Relaxed);
                    let d = &mut self.decks[slot];
                    if let Some(old) = d.clip.replace(clip) {
                        let _ = self.shared.retired.push(old);
                    }
                    d.load_ticket = ticket;
                    d.transition_from = d.last_output;
                    d.transition = 128;
                    d.position = 0.;
                    d.playing = false;
                    d.looping = false;
                    d.loop_in = 0.;
                    d.loop_out = 0.;
                }
                Command::Control(_, Control::Crossfader(v)) => {
                    self.crossfader = v;
                }
                Command::Control(_, Control::Master(v)) => self.master_target = v,
                Command::Control(_, Control::SetMode(mix)) => {
                    self.crossfader = if mix { 0.5 } else { 0. };
                    if !mix {
                        for deck in &mut self.decks[1..] {
                            deck.control(Control::Pause, self.sample_rate);
                        }
                    }
                }
                Command::Control(_, Control::Panic) => {
                    for d in &mut self.decks {
                        d.playing = false;
                    }
                    self.master_target = 0.;
                }
                Command::Control(slot, c) => self.decks[slot].control(c, self.sample_rate),
            }
        }
        let target_mix = dsp::crossfade(self.crossfader);
        let mut peak = 0.0_f32;
        let mut squares = 0.0_f64;
        let mut clipped = 0;
        for d in &mut self.decks {
            d.peak = 0.;
        }
        for frame in output.chunks_mut(channels.max(1)) {
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
            peaks: vec![0.25],
            title: None,
            artist: None,
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
}

#[cfg(test)]
mod allocation_tests;
