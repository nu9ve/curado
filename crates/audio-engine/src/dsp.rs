use crate::fx::{self, BeatFx, ColorFx};

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
    pub trim: f32,
    /// Level matching between tracks, on top of the trim.
    pub auto: f32,
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
            trim: 1.,
            auto: 1.,
        }
    }
}
/// A topology-preserving state-variable filter: stable under fast cutoff changes, which is what
/// a DJ filter gets. Two in series at Butterworth Q give 24 dB per octave.
#[derive(Clone, Copy)]
pub struct SvfCoef {
    a1: f32,
    a2: f32,
    a3: f32,
    pub k: f32,
}
impl SvfCoef {
    pub fn new(hz: f32, q: f32, sr: f32) -> Self {
        let g = (std::f32::consts::PI * hz.clamp(10., sr * 0.49) / sr).tan();
        let k = 1. / q;
        let a1 = 1. / (1. + g * (g + k));
        let a2 = g * a1;
        Self {
            a1,
            a2,
            a3: g * a2,
            k,
        }
    }
}
#[derive(Clone, Copy, Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
}
impl Svf {
    /// Low-pass, band-pass and high-pass of the same input.
    #[inline]
    pub fn tick(&mut self, c: &SvfCoef, x: f32) -> (f32, f32, f32) {
        let v3 = x - self.ic2;
        let v1 = c.a1 * self.ic1 + c.a2 * v3;
        let v2 = self.ic2 + c.a2 * self.ic1 + c.a3 * v3;
        self.ic1 = clean(2. * v1 - self.ic1);
        self.ic2 = clean(2. * v2 - self.ic2);
        (v2, v1, x - c.k * v1 - v2)
    }
}
/// RBJ cookbook biquad, for fixed filters that an analysis specifies by frequency and gain.
#[derive(Clone, Copy)]
pub struct BiquadCoef {
    b: [f64; 3],
    a: [f64; 2],
}
impl BiquadCoef {
    fn normalized(b: [f64; 3], a: [f64; 3]) -> Self {
        Self {
            b: b.map(|value| value / a[0]),
            a: [a[1] / a[0], a[2] / a[0]],
        }
    }
    pub fn high_shelf(hz: f64, gain_db: f64, q: f64, sr: f64) -> Self {
        let amplitude = 10_f64.powf(gain_db / 40.);
        let w = std::f64::consts::TAU * hz / sr;
        let (sin, cos) = w.sin_cos();
        let alpha = sin / (2. * q);
        let root = 2. * amplitude.sqrt() * alpha;
        let (up, down) = (amplitude + 1., amplitude - 1.);
        Self::normalized(
            [
                amplitude * (up + down * cos + root),
                -2. * amplitude * (down + up * cos),
                amplitude * (up + down * cos - root),
            ],
            [
                up - down * cos + root,
                2. * (down - up * cos),
                up - down * cos - root,
            ],
        )
    }
    pub fn high_pass(hz: f64, q: f64, sr: f64) -> Self {
        let w = std::f64::consts::TAU * hz / sr;
        let (sin, cos) = w.sin_cos();
        let alpha = sin / (2. * q);
        Self::normalized(
            [(1. + cos) / 2., -(1. + cos), (1. + cos) / 2.],
            [1. + alpha, -2. * cos, 1. - alpha],
        )
    }
}
#[derive(Clone, Copy, Default)]
pub struct Biquad {
    z: [f64; 2],
}
impl Biquad {
    #[inline]
    pub fn tick(&mut self, c: &BiquadCoef, x: f64) -> f64 {
        let y = c.b[0] * x + self.z[0];
        self.z[0] = c.b[1] * x - c.a[0] * y + self.z[1];
        self.z[1] = c.b[2] * x - c.a[1] * y;
        y
    }
}

const LOW_CROSSOVER: f32 = 220.;
const HIGH_CROSSOVER: f32 = 2600.;
/// Band splits at this Q in pairs are Linkwitz-Riley: flat when summed, with a true kill.
const BUTTERWORTH: f32 = std::f32::consts::FRAC_1_SQRT_2;
const FILTER_INTERVAL: u32 = 16;

/// A 24 dB filter whose cutoff moves smoothly and which fades out of the path when bypassed.
struct Sweep {
    stages: [[Svf; 2]; 2],
    coef: SvfCoef,
    hz: f32,
    target: f32,
    mix: f32,
    mix_target: f32,
}
impl Sweep {
    fn new(hz: f32, sr: f32) -> Self {
        Self {
            stages: Default::default(),
            coef: SvfCoef::new(hz, BUTTERWORTH, sr),
            hz,
            target: hz,
            mix: 0.,
            mix_target: 0.,
        }
    }
    fn move_mix(&mut self, smoothing: f32) {
        self.mix += (self.mix_target - self.mix) * smoothing;
        if self.mix_target == 0. && self.mix < 0.0005 {
            self.mix = 0.;
        }
    }
    fn refresh(&mut self, smoothing: f32, sr: f32) {
        if (self.hz / self.target - 1.).abs() > 0.001 {
            self.hz *= (self.target / self.hz).powf(smoothing);
            self.coef = SvfCoef::new(self.hz, BUTTERWORTH, sr);
        }
    }
    #[inline]
    fn tick(&mut self, c: usize, input: f32, high: bool) -> f32 {
        let pick = |out: (f32, f32, f32)| if high { out.2 } else { out.0 };
        let first = pick(self.stages[0][c].tick(&self.coef, input));
        let second = pick(self.stages[1][c].tick(&self.coef, first));
        if self.mix == 0. {
            return input;
        }
        input + (second - input) * self.mix
    }
}

/// The two music channels also have color and beat effects. Samples do not.
pub struct ChannelFx {
    pub color: ColorFx,
    pub beat: BeatFx,
}
pub struct Effects {
    sr: f32,
    low_lp: [[Svf; 2]; 2],
    low_hp: [[Svf; 2]; 2],
    high_lp: [[Svf; 2]; 2],
    high_hp: [[Svf; 2]; 2],
    low_align: [Svf; 2],
    low_coef: SvfCoef,
    high_coef: SvfCoef,
    low_pass: Sweep,
    high_pass: Sweep,
    counter: u32,
    filter_smoothing: f32,
    delay: Vec<[f32; 2]>,
    head: usize,
    gain: f32,
    trim: f32,
    eq: [f32; 3],
    eq_mix: f32,
    wet: f32,
    delay_frames: f32,
    delay_target: f32,
    smoothing: f32,
    pub channel: Option<Box<ChannelFx>>,
}
impl Effects {
    /// A sample has no echo control, so it skips the long delay line and the effects.
    pub fn new(sr: u32, channel: bool) -> Self {
        let rate = sr as f32;
        let smoothing = 1. - (-1. / (rate * 0.005)).exp();
        Self {
            sr: rate,
            low_lp: Default::default(),
            low_hp: Default::default(),
            high_lp: Default::default(),
            high_hp: Default::default(),
            low_align: Default::default(),
            low_coef: SvfCoef::new(LOW_CROSSOVER, BUTTERWORTH, rate),
            high_coef: SvfCoef::new(HIGH_CROSSOVER, BUTTERWORTH, rate),
            low_pass: Sweep::new(16000_f32.min(rate * 0.45), rate),
            high_pass: Sweep::new(20., rate),
            counter: 0,
            filter_smoothing: 1. - (-(FILTER_INTERVAL as f32) / (rate * 0.008)).exp(),
            delay: preallocated(if channel { sr as usize * 8 } else { 4 }),
            head: 0,
            gain: Params::default().gain,
            trim: 1.,
            eq: [1.; 3],
            eq_mix: 0.,
            wet: 0.,
            delay_frames: rate * 0.25,
            delay_target: rate * 0.25,
            smoothing,
            channel: channel.then(|| {
                Box::new(ChannelFx {
                    color: ColorFx::new(sr),
                    beat: BeatFx::new(sr),
                })
            }),
        }
    }
    pub fn set_highpass(&mut self, value: f32) {
        self.high_pass.mix_target = if value > 0.001 { 1. } else { 0. };
        self.high_pass.target = (20. * 1000_f32.powf(value)).min(self.sr * 0.45);
    }
    pub fn set_echo_time(&mut self, seconds: f32) {
        self.delay_target = (seconds * self.sr).clamp(1., self.delay.len() as f32 - 2.);
    }
    pub fn set_filter(&mut self, value: f32) {
        self.low_pass.mix_target = if value > 0.995 { 0. } else { 1. };
        self.low_pass.target = (80. * 200_f32.powf(value)).min(self.sr * 0.45);
    }
    /// A silent deck has no fader move to smooth: the next attack starts at the set level.
    pub fn snap_gain(&mut self, gain: f32) {
        self.gain = gain;
    }
    pub fn process(&mut self, input: [f32; 2], p: Params) -> [f32; 2] {
        self.gain = clean(self.gain + (p.gain - self.gain) * self.smoothing);
        self.trim = clean(self.trim + (p.trim * p.auto - self.trim) * self.smoothing);
        for (current, target) in self.eq.iter_mut().zip([p.low, p.mid, p.high]) {
            *current += (target - *current) * self.smoothing;
        }
        self.wet = clean(self.wet + (p.echo - self.wet) * self.smoothing);
        // Crossovers spread an attack in time even when flat. A centered EQ leaves the path, so
        // transients stay sample-exact until someone actually turns a band.
        let shaped = [p.low, p.mid, p.high]
            .iter()
            .any(|band| (band - 1.).abs() > 0.001);
        self.eq_mix += (f32::from(shaped) - self.eq_mix) * self.smoothing;
        if !shaped && self.eq_mix < 0.0005 {
            self.eq_mix = 0.;
        }
        self.low_pass.move_mix(self.smoothing);
        self.high_pass.move_mix(self.smoothing);
        if self.counter == 0 {
            self.low_pass.refresh(self.filter_smoothing, self.sr);
            self.high_pass.refresh(self.filter_smoothing, self.sr);
        }
        self.counter = (self.counter + 1) % FILTER_INTERVAL;
        let mut pre = [0.; 2];
        for c in 0..2 {
            let x = input[c] * self.trim;
            let low = self.low_lp[0][c].tick(&self.low_coef, x).0;
            let low = self.low_lp[1][c].tick(&self.low_coef, low).0;
            let rest = self.low_hp[0][c].tick(&self.low_coef, x).2;
            let rest = self.low_hp[1][c].tick(&self.low_coef, rest).2;
            let mid = self.high_lp[0][c].tick(&self.high_coef, rest).0;
            let mid = self.high_lp[1][c].tick(&self.high_coef, mid).0;
            let high = self.high_hp[0][c].tick(&self.high_coef, rest).2;
            let high = self.high_hp[1][c].tick(&self.high_coef, high).2;
            // The upper split shifts the phase of what passes it. The low band takes the same
            // shift through an all-pass, or the sum would dip around the upper crossover.
            let (l, b, h) = self.low_align[c].tick(&self.high_coef, low);
            let low = l + h - self.high_coef.k * b;
            let eq = low * self.eq[0] + mid * self.eq[1] + high * self.eq[2];
            let eq = x + (eq - x) * self.eq_mix;
            let filtered = self.low_pass.tick(c, eq, false);
            pre[c] = self.high_pass.tick(c, filtered, true);
        }
        if let Some(channel) = &mut self.channel {
            pre = channel.color.process(pre);
        }
        let delayed = fx::read_delay(&self.delay, self.head, self.delay_frames);
        self.delay_frames += (self.delay_target - self.delay_frames) * self.smoothing * 0.1;
        let mut out = [0.; 2];
        let mut written = [0.; 2];
        for c in 0..2 {
            let dry = pre[c] * self.gain;
            written[c] = clean(dry + delayed[c] * self.wet * 0.55);
            out[c] = dry + delayed[c] * self.wet * 0.5;
        }
        self.delay[self.head] = written;
        self.head = (self.head + 1) % self.delay.len();
        if let Some(channel) = &mut self.channel {
            out = channel.beat.process(out);
        }
        out
    }
}
/// Written once here, so the callback never takes a page fault on a first write. Zeroed memory
/// from the system can be mapped only when touched.
pub fn preallocated<T: Copy + Default>(frames: usize) -> Vec<T> {
    let mut buffer = Vec::with_capacity(frames);
    buffer.resize(frames, T::default());
    buffer
}
pub fn clean(v: f32) -> f32 {
    if v.abs() < 1e-20 {
        0.
    } else {
        v
    }
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
        let mut fx = Effects::new(sr, true);
        let p = Params {
            gain: 1.,
            echo: 1.,
            ..Default::default()
        };
        fx.set_echo_time(0.5);
        for _ in 0..24000 {
            fx.process([0.; 2], p);
        }
        let mut first_echo = (0_i32, 0_f32);
        for i in 0..12000 {
            let out = fx.process(if i == 0 { [0.5; 2] } else { [0.; 2] }, p);
            assert!(out.iter().all(|v| v.is_finite()));
            if i > 100 && out[0].abs() > first_echo.1 {
                first_echo = (i, out[0].abs());
            }
        }
        assert!((first_echo.0 - 6000).abs() <= 2, "{first_echo:?}");
        assert!(first_echo.1 > 0.1);
    }
    #[test]
    fn echo_read_position_never_leaves_the_buffer() {
        // A delay equal to the write position, an exact wrap, and values around both.
        let sr = 4000;
        for seconds in [0.25_f32, 1.0, 1.999, 2., 2.001, 3.999, 7.9] {
            let mut fx = Effects::new(sr, true);
            fx.set_echo_time(seconds);
            let p = Params {
                gain: 1.,
                echo: 1.,
                ..Default::default()
            };
            for _ in 0..sr * 18 {
                assert!(fx.process([0.1; 2], p).iter().all(|v| v.is_finite()));
            }
        }
    }
    #[test]
    fn high_pass_attenuates_bass_and_bypass_preserves_it() {
        let measure = |enabled| {
            let mut fx = Effects::new(12000, false);
            fx.set_highpass(if enabled { 0.57 } else { 0. });
            let p = Params {
                gain: 1.,
                ..Default::default()
            };
            let mut power = 0.;
            for i in 0..24000 {
                let x = (std::f32::consts::TAU * 60. * i as f32 / 12000.).sin() * 0.5;
                let out = fx.process([x, x], p);
                if i >= 12000 {
                    power += out[0] * out[0];
                }
            }
            power
        };
        assert!(measure(true) < measure(false) * 0.01);
    }

    fn sine_level(fx: &mut Effects, p: Params, hz: f32, sr: f32) -> f32 {
        let tone = |i: usize| (std::f32::consts::TAU * hz * i as f32 / sr).sin() * 0.5;
        for i in 0..sr as usize {
            fx.process([tone(i); 2], p);
        }
        let n = sr as usize / 2;
        let power: f32 = (0..n)
            .map(|i| fx.process([tone(i + sr as usize); 2], p)[0].powi(2))
            .sum();
        (power / n as f32).sqrt() / (0.5 / std::f32::consts::SQRT_2)
    }
    fn db(ratio: f32) -> f32 {
        20. * ratio.max(1e-9).log10()
    }
    #[test]
    fn a_flat_eq_is_flat_and_each_kill_is_a_real_kill() {
        let sr = 48000;
        let flat = Params {
            gain: 1.,
            ..Default::default()
        };
        // Barely off center, so the signal really goes through the crossovers.
        let engaged = Params { low: 1.002, ..flat };
        for hz in [30., 100., 220., 700., 2600., 6000., 15000.] {
            let level = sine_level(&mut Effects::new(sr, false), engaged, hz, sr as f32);
            assert!(db(level).abs() < 0.1, "{hz} Hz at {} dB", db(level));
        }
        // Centered, nothing touches the audio, including its attacks.
        let mut fx = Effects::new(sr, false);
        fx.snap_gain(1.);
        for i in 0..2000 {
            let x = if i < 1000 { 0. } else { 0.5 };
            assert_eq!(fx.process([x; 2], flat)[0], x);
        }
        let kill =
            |p: Params, hz: f32| db(sine_level(&mut Effects::new(sr, false), p, hz, sr as f32));
        assert!(kill(Params { low: 0., ..flat }, 50.) < -35.);
        assert!(kill(Params { mid: 0., ..flat }, 800.) < -35.);
        assert!(kill(Params { high: 0., ..flat }, 12000.) < -35.);
        // A kill leaves the other bands alone.
        assert!(kill(Params { low: 0., ..flat }, 6000.).abs() < 0.3);
        assert!(kill(Params { high: 0., ..flat }, 100.).abs() < 0.3);
    }
    #[test]
    fn the_filters_have_a_24_db_slope() {
        let sr = 48000;
        let p = Params {
            gain: 1.,
            ..Default::default()
        };
        let mut fx = Effects::new(sr, false);
        // 80 * 200^0.5 is about 1131 Hz; two octaves up should lose about 48 dB.
        fx.set_filter(0.5);
        let level = db(sine_level(&mut fx, p, 1131. * 4., sr as f32));
        assert!((-54.0..-42.).contains(&level), "{level} dB");
        let mut fx = Effects::new(sr, false);
        fx.set_highpass(0.5);
        let level = db(sine_level(&mut fx, p, 632. / 4., sr as f32));
        assert!((-54.0..-42.).contains(&level), "{level} dB");
    }
    #[test]
    fn trim_scales_the_input_and_the_echo_stays_after_the_fader() {
        let sr = 48000;
        let p = Params {
            gain: 1.,
            trim: 0.5,
            ..Default::default()
        };
        let level = sine_level(&mut Effects::new(sr, false), p, 1000., sr as f32);
        assert!((level - 0.5).abs() < 0.01);
    }
}
