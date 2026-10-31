//! Sound Color FX and Beat FX. Every buffer exists before the first callback; `process` never
//! allocates and its work per frame is fixed. Effects that ring on (delay, reverb, dub echo)
//! keep their tails when they stop being fed.
use crate::dsp::{clean, preallocated, Svf, SvfCoef};

const BUTTERWORTH: f32 = std::f32::consts::FRAC_1_SQRT_2;
const COEF_INTERVAL: u32 = 16;
const MAX_ECHO_SECONDS: f32 = 8.;
/// Fade around the seam of a repeated slice.
const ROLL_SEAM_SECONDS: f32 = 0.002;

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn sweep(from: f32, to: f32, t: f32) -> f32 {
    from * (to / from).powf(t)
}

struct Comb {
    buffer: Vec<f32>,
    head: usize,
    store: f32,
}
impl Comb {
    fn new(frames: usize) -> Self {
        Self {
            buffer: preallocated(frames.max(1)),
            head: 0,
            store: 0.,
        }
    }
    #[inline]
    fn tick(&mut self, input: f32, room: f32, damp: f32) -> f32 {
        let out = self.buffer[self.head];
        self.store = clean(out * (1. - damp) + self.store * damp);
        self.buffer[self.head] = clean(input + self.store * room);
        self.head = (self.head + 1) % self.buffer.len();
        out
    }
}
struct Allpass {
    buffer: Vec<f32>,
    head: usize,
}
impl Allpass {
    fn new(frames: usize) -> Self {
        Self {
            buffer: preallocated(frames.max(1)),
            head: 0,
        }
    }
    #[inline]
    fn tick(&mut self, input: f32) -> f32 {
        let stored = self.buffer[self.head];
        self.buffer[self.head] = clean(input + stored * 0.5);
        self.head = (self.head + 1) % self.buffer.len();
        stored - input
    }
}
/// Schroeder-Moorer reverb with the delay lengths published with Freeverb, scaled to the rate.
struct Reverb {
    combs: [Vec<Comb>; 2],
    allpasses: [Vec<Allpass>; 2],
}
impl Reverb {
    fn new(sr: u32) -> Self {
        let scale = sr as f32 / 44100.;
        let frames = |tuning: usize, spread: usize| ((tuning + spread) as f32 * scale) as usize;
        let side = |spread: usize| -> (Vec<Comb>, Vec<Allpass>) {
            (
                [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617]
                    .map(|tuning| Comb::new(frames(tuning, spread)))
                    .into(),
                [556, 441, 341, 225]
                    .map(|tuning| Allpass::new(frames(tuning, spread)))
                    .into(),
            )
        };
        let (left_combs, left_passes) = side(0);
        let (right_combs, right_passes) = side(23);
        Self {
            combs: [left_combs, right_combs],
            allpasses: [left_passes, right_passes],
        }
    }
    /// Size is fixed, so this bounded clear can run on a selection in the callback.
    fn clear(&mut self) {
        for comb in self.combs.iter_mut().flatten() {
            comb.buffer.fill(0.);
            comb.store = 0.;
        }
        for pass in self.allpasses.iter_mut().flatten() {
            pass.buffer.fill(0.);
        }
    }
    fn tick(&mut self, input: [f32; 2], room: f32, damp: f32) -> [f32; 2] {
        let fed = (input[0] + input[1]) * 0.015;
        let mut out = [0.; 2];
        for (c, value) in out.iter_mut().enumerate() {
            let mut sum = 0.;
            for comb in &mut self.combs[c] {
                sum += comb.tick(fed, room, damp);
            }
            for pass in &mut self.allpasses[c] {
                sum = pass.tick(sum);
            }
            *value = sum * 3.;
        }
        out
    }
}

/// Reads a stereo delay line behind `head`, between two frames.
#[inline]
pub fn read_delay(buffer: &[[f32; 2]], head: usize, frames: f32) -> [f32; 2] {
    let len = buffer.len();
    let at = (head as f32 - frames).rem_euclid(len as f32);
    // A tiny negative position rounds up to exactly the buffer length in f32.
    let index = at as usize % len;
    let next = (index + 1) % len;
    let frac = at.fract();
    [
        buffer[index][0] + (buffer[next][0] - buffer[index][0]) * frac,
        buffer[index][1] + (buffer[next][1] - buffer[index][1]) * frac,
    ]
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ColorKind {
    #[default]
    Off,
    Space,
    DubEcho,
    Sweep,
    Noise,
    Crush,
    Filter,
}
impl ColorKind {
    pub const COUNT: u8 = 7;
    pub fn from_index(index: u8) -> Self {
        match index {
            1 => Self::Space,
            2 => Self::DubEcho,
            3 => Self::Sweep,
            4 => Self::Noise,
            5 => Self::Crush,
            6 => Self::Filter,
            _ => Self::Off,
        }
    }
    pub fn index(self) -> u8 {
        self as u8
    }
}
/// One channel's color effect. The knob is bipolar and does nothing at its center.
pub struct ColorFx {
    sr: f32,
    kind: ColorKind,
    next: ColorKind,
    switch: f32,
    amount: f32,
    amount_target: f32,
    param: f32,
    param_target: f32,
    smoothing: f32,
    frames_per_beat: f32,
    counter: u32,
    /// Which side of center the knob was last on: a tail keeps its color back at center.
    low_side: bool,
    coef: SvfCoef,
    coef_second: SvfCoef,
    filters: [[Svf; 2]; 2],
    rng: u32,
    hold: [f32; 2],
    hold_left: u32,
    envelope: f32,
    gate: f32,
    delay: Vec<[f32; 2]>,
    head: usize,
    delay_frames: f32,
    reverb: Reverb,
}
impl ColorFx {
    pub fn new(sr: u32) -> Self {
        let rate = sr as f32;
        Self {
            sr: rate,
            kind: ColorKind::Off,
            next: ColorKind::Off,
            switch: 1.,
            amount: 0.,
            amount_target: 0.,
            param: 0.5,
            param_target: 0.5,
            smoothing: 1. - (-1. / (rate * 0.005)).exp(),
            frames_per_beat: rate * 0.5,
            counter: 0,
            low_side: true,
            coef: SvfCoef::new(1000., BUTTERWORTH, rate),
            coef_second: SvfCoef::new(1000., BUTTERWORTH, rate),
            filters: Default::default(),
            rng: 0x9E37_79B9,
            hold: [0.; 2],
            hold_left: 0,
            envelope: 0.,
            gate: 1.,
            delay: preallocated((rate * 4.) as usize),
            head: 0,
            delay_frames: rate * 0.25,
            reverb: Reverb::new(sr),
        }
    }
    pub fn set_kind(&mut self, kind: ColorKind) {
        self.next = kind;
    }
    pub fn set_amount(&mut self, amount: f32) {
        self.amount_target = amount;
    }
    pub fn set_param(&mut self, param: f32) {
        self.param_target = param;
    }
    pub fn set_tempo(&mut self, frames_per_beat: f32) {
        self.frames_per_beat = frames_per_beat;
    }
    pub fn selected(&self) -> ColorKind {
        self.next
    }
    pub fn knob(&self) -> f32 {
        self.amount_target
    }
    pub fn parameter(&self) -> f32 {
        self.param_target
    }
    fn noise(&mut self) -> f32 {
        // xorshift: no allocation, no lock, and no need for anything better in an effect.
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng as f32 / u32::MAX as f32 * 2. - 1.
    }
    fn refresh(&mut self) {
        let depth = self.amount.abs();
        let q = 0.7 + self.param * 5.;
        let cut = |from: f32, to: f32| sweep(from, to, depth);
        let (first, second) = match self.kind {
            ColorKind::Filter if self.low_side => {
                let hz = cut(18000., 60.);
                (
                    SvfCoef::new(hz, q, self.sr),
                    SvfCoef::new(hz, BUTTERWORTH, self.sr),
                )
            }
            ColorKind::Filter => {
                let hz = cut(30., 10000.);
                (
                    SvfCoef::new(hz, q, self.sr),
                    SvfCoef::new(hz, BUTTERWORTH, self.sr),
                )
            }
            ColorKind::Sweep if self.low_side => {
                let hz = cut(18000., 800.);
                (
                    SvfCoef::new(hz, BUTTERWORTH, self.sr),
                    SvfCoef::new(hz, BUTTERWORTH, self.sr),
                )
            }
            ColorKind::Sweep => {
                let hz = cut(300., 6000.);
                let narrow = 1. + 6. * depth;
                (
                    SvfCoef::new(hz, narrow, self.sr),
                    SvfCoef::new(hz, narrow, self.sr),
                )
            }
            ColorKind::Crush | ColorKind::Noise if self.low_side => {
                let hz = cut(18000., 1000.);
                (
                    SvfCoef::new(hz, BUTTERWORTH, self.sr),
                    SvfCoef::new(hz, BUTTERWORTH, self.sr),
                )
            }
            ColorKind::Crush | ColorKind::Noise => {
                let hz = cut(40., 6000.);
                (
                    SvfCoef::new(hz, BUTTERWORTH, self.sr),
                    SvfCoef::new(hz, BUTTERWORTH, self.sr),
                )
            }
            _ if self.low_side => {
                let hz = cut(9000., 700.);
                (
                    SvfCoef::new(hz, BUTTERWORTH, self.sr),
                    SvfCoef::new(hz, BUTTERWORTH, self.sr),
                )
            }
            _ => {
                let hz = cut(80., 2500.);
                (
                    SvfCoef::new(hz, BUTTERWORTH, self.sr),
                    SvfCoef::new(hz, BUTTERWORTH, self.sr),
                )
            }
        };
        self.coef = first;
        self.coef_second = second;
    }
    #[inline]
    fn filtered(&mut self, c: usize, input: f32) -> f32 {
        // The narrowing side of Sweep is a band-pass, scaled so its peak stays at unity.
        let band = self.kind == ColorKind::Sweep && !self.low_side;
        let (coef, second) = (self.coef, self.coef_second);
        let pick = |(low, mid, high): (f32, f32, f32), k: f32| {
            if band {
                mid * k
            } else if self.low_side {
                low
            } else {
                high
            }
        };
        let first = pick(self.filters[0][c].tick(&coef, input), coef.k);
        pick(self.filters[1][c].tick(&second, first), second.k)
    }
    pub fn process(&mut self, input: [f32; 2]) -> [f32; 2] {
        if self.kind != self.next {
            self.switch -= self.switch * self.smoothing;
            if self.switch < 0.001 {
                self.kind = self.next;
                self.filters = Default::default();
                if self.kind == ColorKind::Space {
                    self.reverb.clear();
                }
            }
        } else {
            self.switch += (1. - self.switch) * self.smoothing;
        }
        self.amount += (self.amount_target - self.amount) * self.smoothing;
        self.param += (self.param_target - self.param) * self.smoothing;
        let depth = self.amount.abs();
        if depth > 0.02 {
            self.low_side = self.amount < 0.;
        }
        // A short ramp past center instead of a jump where the effect begins.
        let engage = ((depth - 0.01) / 0.04).clamp(0., 1.) * self.switch;
        if self.counter == 0 {
            self.refresh();
        }
        self.counter = (self.counter + 1) % COEF_INTERVAL;
        // The dub echo's line always runs, so a tail decays even once the effect is left.
        let delay_target = (self.frames_per_beat * 0.75).clamp(1., self.delay.len() as f32 - 2.);
        self.delay_frames += (delay_target - self.delay_frames) * self.smoothing * 0.1;
        let delayed = read_delay(&self.delay, self.head, self.delay_frames);
        let dub = self.kind == ColorKind::DubEcho;
        let feedback = if dub { 0.45 + 0.45 * self.param } else { 0. };
        let mut out = input;
        let mut written = [0.; 2];
        for c in 0..2 {
            let wet = if dub {
                self.filtered(c, delayed[c])
            } else {
                delayed[c]
            };
            let send = if dub { input[c] * depth } else { 0. };
            let fed = send + wet * feedback;
            // Keeps a long feedback from running away, unlike a clamp, without a hard edge.
            written[c] = clean(fed / (1. + 0.25 * fed.abs()));
            if dub {
                out[c] = input[c] + wet * self.switch;
            }
        }
        self.delay[self.head] = written;
        self.head = (self.head + 1) % self.delay.len();
        match self.kind {
            ColorKind::Off | ColorKind::DubEcho => {}
            ColorKind::Space => {
                let room = 0.7 + 0.28 * self.param;
                let wet = self.reverb.tick(input.map(|v| v * depth), room, 0.3);
                for c in 0..2 {
                    out[c] = input[c] + self.filtered(c, wet[c]) * self.switch;
                }
            }
            ColorKind::Filter => {
                for c in 0..2 {
                    let wet = self.filtered(c, input[c]);
                    out[c] = mix(input[c], wet, engage);
                }
            }
            ColorKind::Sweep => {
                if self.low_side {
                    // Everything under a threshold that rises with the knob is cut.
                    let level = input[0].abs().max(input[1].abs());
                    let attack = 1. - (-1. / (self.sr * 0.001)).exp();
                    let release = 1. - (-1. / (self.sr * 0.05)).exp();
                    let rate = if level > self.envelope {
                        attack
                    } else {
                        release
                    };
                    self.envelope = clean(self.envelope + (level - self.envelope) * rate);
                    let open = self.envelope > depth * depth * 0.5;
                    self.gate += (f32::from(open) - self.gate) * attack;
                    for c in 0..2 {
                        let wet = self.filtered(c, input[c]) * self.gate;
                        out[c] = mix(input[c], wet, engage);
                    }
                } else {
                    for c in 0..2 {
                        let wet = self.filtered(c, input[c]);
                        out[c] = mix(input[c], wet, engage);
                    }
                }
            }
            ColorKind::Noise => {
                let level = depth * (0.04 + 0.2 * self.param);
                let noise = [self.noise(), self.noise()];
                for c in 0..2 {
                    let wet = self.filtered(c, noise[c]) * level;
                    out[c] = input[c] + wet * engage;
                }
            }
            ColorKind::Crush => {
                if self.hold_left == 0 {
                    self.hold = input;
                    self.hold_left = (depth * depth * 24. * (0.5 + self.param)) as u32;
                } else {
                    self.hold_left -= 1;
                }
                let levels = 2_f32.powf(15. - 12. * depth);
                for c in 0..2 {
                    let crushed = (self.hold[c] * levels).round() / levels;
                    let wet = self.filtered(c, crushed);
                    out[c] = mix(input[c], wet, engage);
                }
            }
        }
        out
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum BeatKind {
    Delay,
    #[default]
    Echo,
    PingPong,
    Reverb,
    Flanger,
    Phaser,
    Filter,
    Trans,
    Roll,
}
impl BeatKind {
    pub const COUNT: u8 = 9;
    pub fn from_index(index: u8) -> Self {
        match index {
            0 => Self::Delay,
            2 => Self::PingPong,
            3 => Self::Reverb,
            4 => Self::Flanger,
            5 => Self::Phaser,
            6 => Self::Filter,
            7 => Self::Trans,
            8 => Self::Roll,
            _ => Self::Echo,
        }
    }
    pub fn index(self) -> u8 {
        self as u8
    }
    fn delays(self) -> bool {
        matches!(self, Self::Delay | Self::Echo | Self::PingPong)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeatSettings {
    pub kind: BeatKind,
    pub on: bool,
    pub beats: f32,
    pub depth: f32,
    /// A tapped tempo. Zero follows the track.
    pub bpm: f32,
}
impl Default for BeatSettings {
    fn default() -> Self {
        Self {
            kind: BeatKind::Echo,
            on: false,
            beats: 1.,
            depth: 0.5,
            bpm: 0.,
        }
    }
}
/// One Beat FX unit. A channel and the master each own one, so they can differ.
pub struct BeatFx {
    sr: f32,
    settings: BeatSettings,
    kind: BeatKind,
    switch: f32,
    smoothing: f32,
    frames_per_beat: f32,
    phase: f64,
    send: f32,
    wet: f32,
    amount: f32,
    counter: u32,
    delay: Vec<[f32; 2]>,
    head: usize,
    delay_frames: f32,
    damping: [Svf; 2],
    damping_coef: SvfCoef,
    reverb: Reverb,
    flanger: Vec<[f32; 2]>,
    flanger_head: usize,
    flanger_frames: f32,
    stages: [[f32; 6]; 2],
    phaser_feedback: [f32; 2],
    phaser_a: f32,
    filter: [Svf; 2],
    filter_coef: SvfCoef,
    gate: f32,
    gate_rate: f32,
    roll: Vec<[f32; 2]>,
    rolling: bool,
    roll_filled: usize,
    roll_length: usize,
    roll_read: usize,
    roll_seam: f32,
}
impl BeatFx {
    pub fn new(sr: u32) -> Self {
        let rate = sr as f32;
        let frames = (rate * MAX_ECHO_SECONDS) as usize;
        Self {
            sr: rate,
            settings: BeatSettings::default(),
            kind: BeatKind::default(),
            switch: 1.,
            smoothing: 1. - (-1. / (rate * 0.005)).exp(),
            frames_per_beat: rate * 0.5,
            phase: 0.,
            send: 0.,
            wet: 0.5,
            amount: 0.,
            counter: 0,
            delay: preallocated(frames),
            head: 0,
            delay_frames: rate * 0.5,
            damping: Default::default(),
            damping_coef: SvfCoef::new(5000., BUTTERWORTH, rate),
            reverb: Reverb::new(sr),
            flanger: preallocated((rate * 0.02) as usize + 2),
            flanger_head: 0,
            flanger_frames: rate * 0.003,
            stages: [[0.; 6]; 2],
            phaser_feedback: [0.; 2],
            phaser_a: 0.,
            filter: Default::default(),
            filter_coef: SvfCoef::new(1000., 1.5, rate),
            gate: 1.,
            gate_rate: 1. - (-1. / (rate * 0.002)).exp(),
            roll: preallocated(frames),
            rolling: false,
            roll_filled: 0,
            roll_length: 1,
            roll_read: 0,
            roll_seam: (rate * ROLL_SEAM_SECONDS).max(1.),
        }
    }
    pub fn settings(&self) -> BeatSettings {
        self.settings
    }
    pub fn set(&mut self, settings: BeatSettings) {
        self.settings = settings;
    }
    /// Called per block with the deck's tempo and, on a grid, its position in beats. That keeps
    /// gates and sweeps on the music; without a grid they run freely at the stand-in tempo.
    pub fn set_clock(&mut self, track_frames_per_beat: f32, beats: Option<f64>) {
        self.frames_per_beat = if self.settings.bpm > 0. {
            60. / self.settings.bpm * self.sr
        } else {
            track_frames_per_beat
        };
        if let Some(beats) = beats {
            self.phase = beats;
        }
    }
    fn lfo(&self) -> f32 {
        let cycle = (self.phase / self.settings.beats as f64).rem_euclid(1.) as f32;
        0.5 - 0.5 * (std::f32::consts::TAU * cycle).cos()
    }
    fn refresh(&mut self) {
        let lfo = self.lfo();
        match self.kind {
            BeatKind::Filter => {
                self.filter_coef = SvfCoef::new(sweep(250., 12000., lfo), 1.5, self.sr);
            }
            BeatKind::Phaser => {
                let t = (std::f32::consts::PI * sweep(200., 3000., lfo) / self.sr).tan();
                self.phaser_a = (t - 1.) / (t + 1.);
            }
            _ => {}
        }
    }
    pub fn process(&mut self, input: [f32; 2]) -> [f32; 2] {
        let target = self.settings.kind;
        if self.kind != target {
            self.switch -= self.switch * self.smoothing;
            if self.switch < 0.001 {
                self.kind = target;
                self.rolling = false;
                if self.kind == BeatKind::Reverb {
                    self.reverb.clear();
                }
            }
        } else {
            self.switch += (1. - self.switch) * self.smoothing;
        }
        let on = self.settings.on && self.kind == target;
        let depth = self.settings.depth;
        self.send += (f32::from(on) - self.send) * self.smoothing;
        self.wet += (depth - self.wet) * self.smoothing;
        let inline_target = if on { depth } else { 0. };
        self.amount += (inline_target - self.amount) * self.smoothing;
        self.phase += 1. / self.frames_per_beat.max(1.) as f64;
        if self.counter == 0 {
            self.refresh();
        }
        self.counter = (self.counter + 1) % COEF_INTERVAL;
        let cycle = self.settings.beats * self.frames_per_beat;
        // The echo line always runs, so a tail decays even once another effect is selected.
        let target_frames = cycle.clamp(1., self.delay.len() as f32 - 2.);
        self.delay_frames += (target_frames - self.delay_frames) * self.smoothing * 0.1;
        let delayed = read_delay(&self.delay, self.head, self.delay_frames);
        let feedback = match self.kind {
            BeatKind::Delay => 0.25,
            BeatKind::Echo => 0.62,
            BeatKind::PingPong => 0.6,
            _ => 0.,
        };
        let send = if self.kind.delays() { self.send } else { 0. };
        let mut written = [0.; 2];
        if self.kind == BeatKind::PingPong {
            let fed = (input[0] + input[1]) * 0.5 * send;
            written[0] = clean(fed + delayed[1] * feedback);
            written[1] = clean(delayed[0] * feedback);
        } else {
            for (c, value) in written.iter_mut().enumerate() {
                let (damped, _, _) = self.damping[c].tick(&self.damping_coef, delayed[c]);
                *value = clean(input[c] * send + damped * feedback);
            }
        }
        self.delay[self.head] = written;
        self.head = (self.head + 1) % self.delay.len();
        let inline = self.amount * self.switch;
        match self.kind {
            BeatKind::Delay | BeatKind::Echo | BeatKind::PingPong => {
                let wet = self.wet * self.switch;
                [input[0] + delayed[0] * wet, input[1] + delayed[1] * wet]
            }
            BeatKind::Reverb => {
                // Longer divisions make a longer tail, which is what TIME means here.
                let room = (0.72 + 0.06 * self.settings.beats.log2().clamp(-2., 4.)).min(0.97);
                let tail = self
                    .reverb
                    .tick(input.map(|value| value * self.send), room, 0.25);
                let wet = self.wet * self.switch;
                [input[0] + tail[0] * wet, input[1] + tail[1] * wet]
            }
            BeatKind::Flanger => {
                let target = self.sr * (0.0008 + 0.006 * self.lfo());
                self.flanger_frames += (target - self.flanger_frames) * self.smoothing;
                let read = read_delay(&self.flanger, self.flanger_head, self.flanger_frames);
                let mut out = input;
                let mut written = [0.; 2];
                for c in 0..2 {
                    written[c] = clean(input[c] + read[c] * 0.55);
                    let wet = (input[c] + read[c]) * 0.6;
                    out[c] = mix(input[c], wet, inline);
                }
                self.flanger[self.flanger_head] = written;
                self.flanger_head = (self.flanger_head + 1) % self.flanger.len();
                out
            }
            BeatKind::Phaser => {
                let a = self.phaser_a;
                let mut out = input;
                for c in 0..2 {
                    let mut x = input[c] + self.phaser_feedback[c] * 0.45;
                    for stage in &mut self.stages[c] {
                        let y = a * x + *stage;
                        *stage = clean(x - a * y);
                        x = y;
                    }
                    self.phaser_feedback[c] = clean(x);
                    let wet = (input[c] + x) * 0.6;
                    out[c] = mix(input[c], wet, inline);
                }
                out
            }
            BeatKind::Filter => {
                let mut out = input;
                for c in 0..2 {
                    let (low, _, _) = self.filter[c].tick(&self.filter_coef, input[c]);
                    out[c] = mix(input[c], low, inline);
                }
                out
            }
            BeatKind::Trans => {
                let cycle = (self.phase / self.settings.beats as f64).rem_euclid(1.);
                let open = if cycle < 0.5 { 1. } else { 0. };
                self.gate += (open - self.gate) * self.gate_rate;
                let wet = self.gate * inline + (1. - inline);
                [input[0] * wet, input[1] * wet]
            }
            BeatKind::Roll => {
                let length = (cycle as usize).clamp(2, self.roll.len());
                if !on {
                    self.rolling = false;
                } else if !self.rolling {
                    self.rolling = true;
                    self.roll_filled = 0;
                    self.roll_read = 0;
                    self.roll_length = length;
                }
                if self.rolling {
                    // A shorter division takes effect at once, within what is already held.
                    if length <= self.roll_filled {
                        self.roll_length = length;
                        if self.roll_read >= length {
                            self.roll_read = 0;
                        }
                    }
                    if self.roll_filled < self.roll_length {
                        self.roll[self.roll_filled] = input;
                        self.roll_filled += 1;
                    }
                }
                let held = self.roll[self.roll_read.min(self.roll.len() - 1)];
                let from_end = (self.roll_length - self.roll_read.min(self.roll_length)) as f32;
                let repeating = self.roll_filled >= self.roll_length;
                let seam = if repeating {
                    (from_end / self.roll_seam)
                        .min((self.roll_read as f32 + 1.) / self.roll_seam)
                        .min(1.)
                } else {
                    1.
                };
                if self.rolling {
                    self.roll_read += 1;
                    if self.roll_read >= self.roll_length {
                        self.roll_read = 0;
                    }
                }
                [
                    mix(input[0], held[0] * seam, inline),
                    mix(input[1], held[1] * seam, inline),
                ]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SR: u32 = 48000;
    const ALL_COLOR: [ColorKind; 6] = [
        ColorKind::Space,
        ColorKind::DubEcho,
        ColorKind::Sweep,
        ColorKind::Noise,
        ColorKind::Crush,
        ColorKind::Filter,
    ];
    const ALL_BEAT: [BeatKind; 9] = [
        BeatKind::Delay,
        BeatKind::Echo,
        BeatKind::PingPong,
        BeatKind::Reverb,
        BeatKind::Flanger,
        BeatKind::Phaser,
        BeatKind::Filter,
        BeatKind::Trans,
        BeatKind::Roll,
    ];
    fn sine(i: usize, hz: f32) -> f32 {
        (std::f32::consts::TAU * hz * i as f32 / SR as f32).sin() * 0.5
    }
    /// Lets the 5 ms parameter smoothing and any effect switch settle.
    const SETTLE: usize = SR as usize / 5;

    fn unit(kind: BeatKind, beats: f32) -> BeatFx {
        let mut fx = BeatFx::new(SR);
        fx.set(BeatSettings {
            kind,
            on: true,
            beats,
            depth: 1.,
            bpm: 0.,
        });
        fx
    }
    fn power(values: impl Iterator<Item = f32>) -> f32 {
        let (sum, n) = values.fold((0., 0), |(s, n), v| (s + v * v, n + 1));
        (sum / n.max(1) as f32).sqrt()
    }

    #[test]
    fn indexes_round_trip_and_stay_in_range() {
        for i in 0..ColorKind::COUNT {
            assert_eq!(ColorKind::from_index(i).index(), i);
        }
        for i in 0..BeatKind::COUNT {
            assert_eq!(BeatKind::from_index(i).index(), i);
        }
    }

    #[test]
    fn a_color_effect_at_center_leaves_the_audio_alone() {
        for kind in ALL_COLOR {
            let mut fx = ColorFx::new(SR);
            fx.set_kind(kind);
            for i in 0..SETTLE {
                fx.process([sine(i, 440.); 2]);
            }
            for i in SETTLE..SETTLE + 2000 {
                let input = sine(i, 440.);
                let out = fx.process([input; 2]);
                assert!((out[0] - input).abs() < 1e-5, "{kind:?}");
            }
        }
    }

    #[test]
    fn every_effect_stays_finite_and_bounded_at_its_extremes() {
        for kind in ALL_COLOR {
            for amount in [-1., 1.] {
                for param in [0., 1.] {
                    let mut fx = ColorFx::new(SR);
                    fx.set_kind(kind);
                    fx.set_amount(amount);
                    fx.set_param(param);
                    for i in 0..SR as usize * 6 {
                        let out = fx.process([sine(i, 80. + i as f32 * 0.01); 2]);
                        assert!(
                            out.iter().all(|v| v.is_finite() && v.abs() < 8.),
                            "{kind:?} {amount} {param}: {out:?}"
                        );
                    }
                }
            }
        }
        for kind in ALL_BEAT {
            for beats in [0.0625, 1., 16.] {
                let mut fx = unit(kind, beats);
                for i in 0..SR as usize * 6 {
                    let out = fx.process([sine(i, 80. + i as f32 * 0.01); 2]);
                    assert!(
                        out.iter().all(|v| v.is_finite() && v.abs() < 8.),
                        "{kind:?} {beats}: {out:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_color_filter_cuts_the_side_it_is_turned_to() {
        let measure = |amount: f32, hz: f32| {
            let mut fx = ColorFx::new(SR);
            fx.set_kind(ColorKind::Filter);
            fx.set_param(0.);
            fx.set_amount(amount);
            for i in 0..SETTLE {
                fx.process([sine(i, hz); 2]);
            }
            power((SETTLE..SETTLE + 9600).map(|i| fx.process([sine(i, hz); 2])[0]))
        };
        assert!(measure(-0.8, 8000.) < measure(-0.8, 60.) * 0.01);
        assert!(measure(0.8, 60.) < measure(0.8, 8000.) * 0.01);
    }

    #[test]
    fn the_noise_effect_adds_noise_only_off_center() {
        let mut fx = ColorFx::new(SR);
        fx.set_kind(ColorKind::Noise);
        fx.set_amount(0.9);
        fx.set_param(1.);
        for _ in 0..SETTLE {
            fx.process([0.; 2]);
        }
        assert!(power((0..9600).map(|_| fx.process([0.; 2])[0])) > 0.005);
    }

    #[test]
    fn dub_echo_and_space_ring_on_after_the_knob_returns() {
        for kind in [ColorKind::DubEcho, ColorKind::Space] {
            let mut fx = ColorFx::new(SR);
            fx.set_kind(kind);
            fx.set_tempo(SR as f32 / 4.);
            for _ in 0..SETTLE {
                fx.process([0.; 2]);
            }
            fx.set_amount(1.);
            for i in 0..SR as usize {
                fx.process([sine(i, 300.); 2]);
            }
            fx.set_amount(0.);
            for _ in 0..SETTLE / 2 {
                fx.process([0.; 2]);
            }
            let tail = power((0..SR as usize / 2).map(|_| fx.process([0.; 2])[0]));
            assert!(tail > 0.003, "{kind:?} left {tail}");
        }
    }

    #[test]
    fn beat_time_follows_the_tempo_and_division() {
        let mut fx = unit(BeatKind::Delay, 0.5);
        fx.set_clock(SR as f32 / 2., Some(0.));
        for _ in 0..SR as usize * 2 {
            fx.process([0.; 2]);
        }
        let mut loudest = (0, 0_f32);
        for i in 0..SR as usize / 2 {
            let out = fx.process(if i == 0 { [0.8; 2] } else { [0.; 2] });
            if i > 50 && out[0].abs() > loudest.1 {
                loudest = (i, out[0].abs());
            }
        }
        // Half a beat at 120 BPM is 250 ms; the damping filter adds a few samples.
        assert!((loudest.0 as i32 - 12000).abs() < 12, "{loudest:?}");
        // A tapped tempo replaces the track's.
        let mut tapped = BeatFx::new(SR);
        tapped.set(BeatSettings {
            bpm: 60.,
            ..Default::default()
        });
        tapped.set_clock(SR as f32 / 2., Some(0.));
        assert_eq!(tapped.frames_per_beat, SR as f32);
    }

    #[test]
    fn a_delay_tail_outlives_the_switch_and_an_inline_effect_does_not() {
        let tail = |kind: BeatKind| {
            let mut fx = unit(kind, 0.25);
            fx.set_clock(SR as f32 / 2., Some(0.));
            for i in 0..SR as usize {
                fx.process([sine(i, 300.); 2]);
            }
            let mut settings = fx.settings();
            settings.on = false;
            fx.set(settings);
            for _ in 0..SETTLE / 4 {
                fx.process([0.; 2]);
            }
            power((0..SR as usize / 2).map(|_| fx.process([0.; 2])[0]))
        };
        assert!(tail(BeatKind::Echo) > 0.01);
        assert!(tail(BeatKind::Reverb) > 0.003);
        for kind in [
            BeatKind::Trans,
            BeatKind::Filter,
            BeatKind::Phaser,
            BeatKind::Roll,
        ] {
            assert!(tail(kind) < 0.002, "{kind:?}");
        }
    }

    #[test]
    fn an_effect_that_is_off_passes_audio_exactly() {
        for kind in ALL_BEAT {
            let mut fx = BeatFx::new(SR);
            fx.set(BeatSettings {
                kind,
                on: false,
                depth: 1.,
                beats: 1.,
                bpm: 0.,
            });
            for i in 0..SETTLE {
                fx.process([sine(i, 500.); 2]);
            }
            for i in SETTLE..SETTLE + 2000 {
                let input = sine(i, 500.);
                let out = fx.process([input; 2]);
                assert!((out[0] - input).abs() < 1e-4, "{kind:?} {}", out[0] - input);
            }
        }
    }

    #[test]
    fn trans_gates_on_the_division() {
        let mut fx = unit(BeatKind::Trans, 1.);
        let beat = SR as usize / 2;
        fx.set_clock(beat as f32, Some(0.));
        for _ in 0..SETTLE {
            fx.process([0.; 2]);
        }
        fx.set_clock(beat as f32, Some(0.));
        let mut open = 0.;
        for i in 0..beat / 2 {
            let out = fx.process([0.5; 2]);
            if i > beat / 4 {
                open = out[0];
            }
        }
        assert!(open > 0.49, "{open}");
        let mut closed = 1.;
        for i in 0..beat / 2 {
            let out = fx.process([0.5; 2]);
            if i > beat / 4 {
                closed = out[0];
            }
        }
        assert!(closed < 0.01, "{closed}");
    }

    #[test]
    fn roll_repeats_a_slice_and_never_runs_past_its_buffer() {
        let mut fx = unit(BeatKind::Roll, 0.25);
        fx.set_clock(4800., Some(0.));
        // 1200 frames per repeat. The input is a ramp, so a repeat is easy to recognize.
        let mut outputs = Vec::new();
        for _ in 0..SETTLE {
            fx.process([0.; 2]);
        }
        let mut settings = fx.settings();
        settings.on = false;
        fx.set(settings);
        for _ in 0..SETTLE {
            fx.process([0.; 2]);
        }
        settings.on = true;
        fx.set(settings);
        for i in 0..6000 {
            outputs.push(fx.process([(i as f32 / 6000.) * 0.5 + 0.1; 2])[0]);
        }
        // Well after the first slice the output matches one held earlier, not the live ramp.
        let live = (4000. / 6000.) * 0.5 + 0.1;
        assert!((outputs[4000] - live).abs() > 0.1, "{}", outputs[4000]);
        assert!((outputs[3100] - outputs[3100 + 1200]).abs() < 0.005);
        let mut fx = unit(BeatKind::Roll, 16.);
        fx.set_clock(SR as f32, Some(0.));
        for i in 0..SR as usize * 20 {
            assert!(fx.process([sine(i, 100.); 2])[0].is_finite());
        }
    }

    #[test]
    fn changing_effect_does_not_jump() {
        let mut fx = unit(BeatKind::Echo, 1.);
        let mut last = 0_f32;
        let mut largest = 0_f32;
        for i in 0..SR as usize {
            if i == SR as usize / 2 {
                let mut settings = fx.settings();
                settings.kind = BeatKind::Filter;
                fx.set(settings);
            }
            let out = fx.process([sine(i, 100.); 2])[0];
            if i > SETTLE {
                largest = largest.max((out - last).abs());
            }
            last = out;
        }
        assert!(largest < 0.1, "{largest}");
    }
}
