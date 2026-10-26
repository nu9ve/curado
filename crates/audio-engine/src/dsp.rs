#[derive(Clone, Copy)]
pub struct Params {
    pub gain: f32,
    pub rate: f32,
    pub low: f32,
    pub mid: f32,
    pub high: f32,
    pub filter: f32,
    pub echo: f32,
}
impl Default for Params {
    fn default() -> Self {
        Self {
            gain: 0.8,
            rate: 1.0,
            low: 1.0,
            mid: 1.0,
            high: 1.0,
            filter: 1.0,
            echo: 0.0,
        }
    }
}
pub struct Effects {
    low: [f32; 2],
    high_split: [f32; 2],
    filtered: [f32; 2],
    delay: Vec<[f32; 2]>,
    head: usize,
    low_a: f32,
    high_a: f32,
    filter_a: f32,
    gain: f32,
    eq: [f32; 3],
    wet: f32,
    filter_target: f32,
}
impl Effects {
    pub fn new(sr: u32) -> Self {
        Self {
            low: [0.; 2],
            high_split: [0.; 2],
            filtered: [0.; 2],
            delay: vec![[0.; 2]; (sr as f32 * 0.375) as usize],
            head: 0,
            low_a: coeff(220., sr),
            high_a: coeff(2600., sr),
            filter_a: 1.,
            gain: 0.,
            eq: [1.; 3],
            wet: 0.,
            filter_target: 1.,
        }
    }
    pub fn set_filter(&mut self, value: f32, sr: u32) {
        self.filter_target = if value > 0.995 {
            1.
        } else {
            coeff(80. * (200.0_f32).powf(value), sr)
        };
    }
    pub fn process(&mut self, input: [f32; 2], p: Params, envelope: f32) -> [f32; 2] {
        self.gain += (p.gain * envelope - self.gain) * 0.004;
        for (current, target) in self.eq.iter_mut().zip([p.low, p.mid, p.high]) {
            *current += (target - *current) * 0.004;
        }
        self.wet += (p.echo - self.wet) * 0.004;
        self.filter_a += (self.filter_target - self.filter_a) * 0.004;
        let mut out = [0.; 2];
        for c in 0..2 {
            self.low[c] += self.low_a * (input[c] - self.low[c]);
            self.high_split[c] += self.high_a * (input[c] - self.high_split[c]);
            let eq = self.low[c] * self.eq[0]
                + (self.high_split[c] - self.low[c]) * self.eq[1]
                + (input[c] - self.high_split[c]) * self.eq[2];
            self.filtered[c] += self.filter_a * (eq - self.filtered[c]);
            let dry = self.filtered[c] * self.gain;
            let delayed = self.delay[self.head][c];
            self.delay[self.head][c] = dry + delayed * self.wet * 0.55;
            out[c] = dry + delayed * self.wet * 0.5;
        }
        self.head = (self.head + 1) % self.delay.len();
        out
    }
}
fn coeff(hz: f32, sr: u32) -> f32 {
    1. - (-std::f32::consts::TAU * hz / sr as f32).exp()
}
pub fn crossfade(x: f32) -> [f32; 2] {
    let t = x.clamp(0., 1.) * std::f32::consts::FRAC_PI_2;
    [t.cos(), t.sin()]
}
