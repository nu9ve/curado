#[derive(Clone, Copy)]
pub struct Params {
    pub gain: f32,
    pub rate: f32,
    pub low: f32,
    pub mid: f32,
    pub high: f32,
    pub filter: f32,
    pub echo: f32,
    pub high_pass: f32,
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
            high_pass: 0.,
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
    hp_state: [f32; 2],
    hp_a: f32,
    hp_target: f32,
    hp_mix: f32,
    hp_enabled: f32,
    delay_frames: f32,
    delay_target: f32,
    smoothing: f32,
}
impl Effects {
    pub fn new(sr: u32) -> Self {
        Self {
            low: [0.; 2],
            high_split: [0.; 2],
            filtered: [0.; 2],
            delay: vec![[0.; 2]; sr as usize * 8],
            head: 0,
            low_a: coeff(220., sr),
            high_a: coeff(2600., sr),
            filter_a: 1.,
            gain: 0.,
            eq: [1.; 3],
            wet: 0.,
            filter_target: 1.,
            hp_state: [0.; 2],
            hp_a: coeff(20., sr),
            hp_target: coeff(20., sr),
            hp_mix: 0.,
            hp_enabled: 0.,
            delay_frames: sr as f32 * 0.25,
            delay_target: sr as f32 * 0.25,
            smoothing: 1. - (-1. / (sr as f32 * 0.005)).exp(),
        }
    }
    pub fn set_highpass(&mut self, value: f32, sr: u32) {
        self.hp_enabled = if value > 0.001 { 1. } else { 0. };
        self.hp_target = coeff((20. * 1000_f32.powf(value)).min(sr as f32 * 0.45), sr);
    }
    pub fn set_echo_time(&mut self, seconds: f32, sr: u32) {
        self.delay_target = (seconds * sr as f32).clamp(1., self.delay.len() as f32 - 2.);
    }
    pub fn set_filter(&mut self, value: f32, sr: u32) {
        self.filter_target = if value > 0.995 {
            1.
        } else {
            coeff(80. * (200.0_f32).powf(value), sr)
        };
    }
    pub fn process(&mut self, input: [f32; 2], p: Params, envelope: f32) -> [f32; 2] {
        self.gain += (p.gain * envelope - self.gain) * self.smoothing;
        for (current, target) in self.eq.iter_mut().zip([p.low, p.mid, p.high]) {
            *current += (target - *current) * self.smoothing;
        }
        self.wet += (p.echo - self.wet) * self.smoothing;
        self.gain = clean(self.gain);
        self.wet = clean(self.wet);
        self.filter_a += (self.filter_target - self.filter_a) * self.smoothing;
        self.hp_a += (self.hp_target - self.hp_a) * self.smoothing;
        self.hp_mix += (self.hp_enabled - self.hp_mix) * self.smoothing;
        self.delay_frames += (self.delay_target - self.delay_frames) * self.smoothing * 0.1;
        let read = (self.head as f32 - self.delay_frames).rem_euclid(self.delay.len() as f32);
        let index = read as usize;
        let next = (index + 1) % self.delay.len();
        let frac = read.fract();
        let mut out = [0.; 2];
        for c in 0..2 {
            self.low[c] += self.low_a * (input[c] - self.low[c]);
            self.high_split[c] += self.high_a * (input[c] - self.high_split[c]);
            let eq = self.low[c] * self.eq[0]
                + (self.high_split[c] - self.low[c]) * self.eq[1]
                + (input[c] - self.high_split[c]) * self.eq[2];
            self.filtered[c] += self.filter_a * (eq - self.filtered[c]);
            self.hp_state[c] += self.hp_a * (self.filtered[c] - self.hp_state[c]);
            let dry = (self.filtered[c] - self.hp_state[c] * self.hp_mix) * self.gain;
            let delayed =
                self.delay[index][c] + (self.delay[next][c] - self.delay[index][c]) * frac;
            self.delay[self.head][c] = clean(dry + delayed * self.wet * 0.55);
            out[c] = dry + delayed * self.wet * 0.5;
            self.low[c] = clean(self.low[c]);
            self.high_split[c] = clean(self.high_split[c]);
            self.filtered[c] = clean(self.filtered[c]);
            self.hp_state[c] = clean(self.hp_state[c]);
        }
        self.head = (self.head + 1) % self.delay.len();
        out
    }
}
fn clean(v: f32) -> f32 {
    if v.abs() < 1e-20 {
        0.
    } else {
        v
    }
}
fn coeff(hz: f32, sr: u32) -> f32 {
    1. - (-std::f32::consts::TAU * hz / sr as f32).exp()
}
pub fn crossfade(x: f32) -> [f32; 2] {
    let t = x.clamp(0., 1.) * std::f32::consts::FRAC_PI_2;
    [t.cos(), t.sin()]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn echo_delay_follows_beat_time_and_remains_finite() {
        let sr = 12000;
        let mut fx = Effects::new(sr);
        let p = Params {
            gain: 1.,
            echo: 1.,
            ..Default::default()
        };
        fx.set_echo_time(0.5, sr);
        for _ in 0..24000 {
            fx.process([0.; 2], p, 1.);
        }
        let mut first_echo = (0_i32, 0_f32);
        for i in 0..12000 {
            let out = fx.process(if i == 0 { [0.5; 2] } else { [0.; 2] }, p, 1.);
            assert!(out.iter().all(|v| v.is_finite()));
            if i > 100 && out[0].abs() > first_echo.1 {
                first_echo = (i, out[0].abs());
            }
        }
        assert!((first_echo.0 - 6000).abs() <= 2, "{first_echo:?}");
        assert!(first_echo.1 > 0.1);
    }
    #[test]
    fn high_pass_attenuates_bass_and_bypass_preserves_it() {
        let measure = |enabled| {
            let mut fx = Effects::new(12000);
            fx.set_highpass(if enabled { 0.57 } else { 0. }, 12000);
            let p = Params {
                gain: 1.,
                ..Default::default()
            };
            let mut power = 0.;
            for i in 0..24000 {
                let x = (std::f32::consts::TAU * 60. * i as f32 / 12000.).sin() * 0.5;
                let out = fx.process([x, x], p, 1.);
                if i >= 12000 {
                    power += out[0] * out[0];
                }
            }
            power
        };
        assert!(measure(true) < measure(false) * 0.01);
    }
}
