//! **The voice bank**: commands in, stereo samples out.
//!
//! Everything here runs on the audio thread in the live game and on the main
//! thread in the offline render, and must behave identically in both, so it
//! is a pure function of `(seed, sample rate, command stream)`: no clock, no
//! allocation after [`VoiceBank::new`], no locks. The device file owns the
//! thread; this file owns the sound.
//!
//! **Shape** (design §3b): a fixed pool of [`GRAIN_POOL`] procedural grains
//! that every sound in the game is made of; one granular texture per
//! [`TextureKind`] that spawns grains at a smoothed rate; a few short-lived
//! bursts; a few tones for alerts; a room tone so a quiet box never sounds
//! broken; a small reverb, "inside a box"; a soft limiter at the end.
//!
//! **A grain is filtered noise plus a sine, each with its own decay**, and the
//! twelve recipes in [`recipe`] are just settings of that one shape. That is
//! deliberately small: the design's case for procedural audio is that the
//! sources here are small, numerous and granular, which is exactly where one
//! cheap shape played thousands of times with jitter is enough.

use super::{AlertKind, GrainKind, SoundCmd, Style, TextureKind};

/// Grains that can sound at once, across every texture, burst and verb. Over
/// this the quietest playing grain is stolen (design §3e). 64 is far above
/// what the textures' rate caps produce at steady state (a 6 ms click at 200 a
/// second is about one grain alive at a time); the headroom is for bursts
/// landing on top of a busy colony.
pub const GRAIN_POOL: usize = 64;
/// The most grains of one kind fired by *verbs* that may sound together;
/// past it the oldest of that kind is replaced. Textures are bounded by their
/// own rate caps instead.
pub const PER_KIND_CAP: usize = 4;
const BURSTS: usize = 6;
const TONES: usize = 8;
/// The first-run master level. Starts well below full so the first launch is
/// never loud (design §3e); the bar's SOUND button steps it.
pub const DEFAULT_MASTER: f32 = 0.5;
/// Room tone level -- the lamp's hum and the air of the box -- about -40 dB
/// under a full-scale grain. Enough that a settled box reads as quiet rather
/// than as broken; low enough to disappear under any activity.
const ROOM_LEVEL: f32 = 0.010;
/// **Make-up gain after the master**, so `DEFAULT_MASTER` (whose square is
/// the applied gain, 0.25) plays the mix at unity rather than 12 dB down. The
/// first offline render without it measured -41 dB RMS for a busy colony and
/// a 0.15 peak for a water pour: inaudible on a phone, which is where the
/// owner listens. With it the colony murmur sits near -29 dB and a verb peaks
/// near half scale; `HIGH` lands on the soft limiter, which is what it is for.
const MAKEUP: f32 = 4.0;
/// Highest grain rate any texture plays, per second. Past it grains overlap
/// and the ear stops hearing them as separate, so only level grows
/// (`listen::density`). The musical style's plucks are tonal and turn to mush
/// far sooner, hence their own cap.
pub const TEXTURE_RATE_CAP: f32 = 220.0;
const MUSICAL_RATE_CAP: f32 = 30.0;

/// A small, fast, deterministic noise source. The audio thread's own, never
/// the simulation's: sound must not draw from `World::rng` (design §6d).
#[derive(Clone)]
struct Xs(u64);

impl Xs {
    #[inline]
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in 0 ..= 1.
    #[inline]
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    #[inline]
    fn bipolar(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
    /// Roughly normal (sum of three uniforms), mean 0, sd ~1.
    fn gauss(&mut self) -> f32 {
        (self.bipolar() + self.bipolar() + self.bipolar()) * 1.0
    }
}

/// RBJ constant-peak band-pass.
#[derive(Clone, Copy, Default)]
struct Biquad {
    b0: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    /// Divides out the filter's noise bandwidth so a recipe's `noise` level
    /// means the same loudness whatever its centre and Q.
    norm: f32,
}

impl Biquad {
    fn set(&mut self, f0: f32, q: f32, sr: f32) {
        let f0 = f0.clamp(40.0, sr * 0.45);
        let w0 = std::f32::consts::TAU * f0 / sr;
        let alpha = w0.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        self.b0 = alpha / a0;
        self.b2 = -alpha / a0;
        self.a1 = -2.0 * w0.cos() / a0;
        self.a2 = (1.0 - alpha) / a0;
        self.norm = 1.0 / (std::f32::consts::PI * f0 / (q * sr)).sqrt().max(1e-3);
    }
    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y * self.norm
    }
}

/// One grain recipe: a noise band and a sine, each with its own decay.
#[derive(Clone, Copy, Debug, Default)]
pub struct Spec {
    pub len: f32,
    pub attack: f32,
    pub noise: f32,
    pub noise_tau: f32,
    pub band: f32,
    pub q: f32,
    /// Band centre at the end of the grain, as a multiple of `band` (1 = still).
    pub band_end: f32,
    /// Amplitude modulation depth by slow random noise -- the scrape in `Grit`.
    pub rough: f32,
    pub sine: f32,
    pub sine_tau: f32,
    pub freq: f32,
    /// Frequency at the end of the grain as a multiple of `freq` -- a drop's rise.
    pub freq_end: f32,
    /// Extra pitch at the onset that falls away in ~20 ms -- an impact's knock.
    pub drop: f32,
}

/// **The recipes.** Every number is a first guess tuned by ear against the
/// listening sketches on the design page; jitter is drawn here so two grains of
/// one kind are never identical.
fn recipe(kind: GrainKind, style: Style, rng: &mut Xs) -> Spec {
    let base = Spec { band_end: 1.0, freq_end: 1.0, q: 1.4, ..Spec::default() };
    match kind {
        GrainKind::Click if style == Style::Musical => {
            const PENTA: [f32; 5] = [0.0, 2.0, 4.0, 7.0, 9.0];
            let semi = PENTA[(rng.next() % 5) as usize] + 12.0 * (rng.next() % 2) as f32;
            Spec { len: 0.12, attack: 0.002, sine: 0.35, sine_tau: 0.025, freq: 523.25 * 2f32.powf(semi / 12.0), ..base }
        }
        GrainKind::Click => {
            let c = rng.range(2800.0, 5500.0);
            Spec { len: 0.008, attack: 0.0004, noise: 0.5, noise_tau: rng.range(0.0006, 0.0014), band: c, q: 1.6, ..base }
        }
        GrainKind::Grit => Spec {
            len: rng.range(0.02, 0.045),
            attack: 0.002,
            noise: 0.45,
            noise_tau: rng.range(0.008, 0.016),
            band: rng.range(1100.0, 2200.0),
            q: 0.7,
            rough: 0.9,
            ..base
        },
        GrainKind::Patter => {
            let c = rng.range(900.0, 1800.0);
            Spec { len: 0.012, attack: 0.0006, noise: 0.4, noise_tau: 0.0018, band: c, q: 1.6, ..base }
        }
        GrainKind::Hiss => Spec {
            len: rng.range(0.008, 0.02),
            attack: 0.001,
            noise: 0.35,
            noise_tau: rng.range(0.003, 0.008),
            band: rng.range(5000.0, 7500.0),
            q: 0.8,
            ..base
        },
        GrainKind::Drip => {
            let len = rng.range(0.02, 0.05);
            Spec { len, attack: 0.0008, sine: 0.5, sine_tau: len / 3.5, freq: rng.range(500.0, 1300.0), freq_end: rng.range(2.2, 4.0), ..base }
        }
        GrainKind::Splash => Spec {
            len: 0.12,
            attack: 0.002,
            noise: 0.45,
            noise_tau: 0.018,
            band: rng.range(3000.0, 5000.0),
            q: 0.6,
            sine: 0.2,
            sine_tau: 0.012,
            freq: rng.range(900.0, 1700.0),
            freq_end: 2.5,
            ..base
        },
        GrainKind::Thud => {
            let f = rng.range(55.0, 95.0);
            Spec { len: 0.25, attack: 0.001, noise: 0.35, noise_tau: 0.03, band: 300.0, q: 0.7, sine: 0.9, sine_tau: 0.08, freq: f, drop: 1.5, ..base }
        }
        GrainKind::Knock => {
            let f = rng.range(250.0, 700.0);
            Spec { len: 0.07, attack: 0.0005, noise: 0.45, noise_tau: 0.008, band: f * 2.0, q: 1.0, sine: 0.5, sine_tau: 0.015, freq: f, ..base }
        }
        GrainKind::Tick => Spec {
            len: 0.03,
            attack: 0.0003,
            noise: 0.3,
            noise_tau: 0.0015,
            band: 3000.0,
            q: 1.2,
            sine: 0.4,
            sine_tau: 0.004,
            freq: rng.range(1850.0, 1950.0),
            ..base
        },
        GrainKind::Thunk => Spec {
            len: 0.12,
            attack: 0.001,
            noise: 0.25,
            noise_tau: 0.01,
            band: 600.0,
            q: 0.9,
            sine: 0.8,
            sine_tau: 0.035,
            freq: rng.range(160.0, 185.0),
            drop: 0.6,
            ..base
        },
        GrainKind::Crackle => Spec {
            len: 0.012,
            attack: 0.0002,
            noise: 0.4 * (1.0 + 2.0 * rng.unit() * rng.unit()),
            noise_tau: 0.0012,
            band: rng.range(1800.0, 4500.0),
            q: 1.1,
            ..base
        },
        GrainKind::Whoosh => Spec {
            len: 0.45,
            attack: 0.18,
            noise: 0.35,
            noise_tau: 0.12,
            band: 500.0,
            q: 0.9,
            band_end: 6.0,
            ..base
        },
    }
}

#[derive(Clone, Copy, Default)]
struct Grain {
    active: bool,
    kind_tag: u8,
    from_verb: bool,
    wait: u32,
    t: u32,
    len: u32,
    attack: u32,
    noise: f32,
    noise_env: f32,
    noise_decay: f32,
    band: Biquad,
    band0: f32,
    band_end: f32,
    q: f32,
    rough: f32,
    rough_lp: f32,
    sine: f32,
    sine_env: f32,
    sine_decay: f32,
    freq: f32,
    freq_end: f32,
    drop: f32,
    phase: f32,
    lp: [f32; 2],
    lp_c: f32,
    gl: f32,
    gr: f32,
}

#[derive(Clone, Copy, Default)]
struct Smoothed {
    value: f32,
    target: f32,
}

impl Smoothed {
    #[inline]
    fn step(&mut self, up: f32, down: f32) -> f32 {
        let k = if self.target > self.value { up } else { down };
        self.value += (self.target - self.value) * k;
        self.value
    }
}

#[derive(Clone, Copy, Default)]
struct Texture {
    rate: Smoothed,
    level: Smoothed,
    pan: Smoothed,
    spread: Smoothed,
    muffle: Smoothed,
    bright: Smoothed,
    /// **Habituation** (design §3e): eases a steady texture down a few dB after
    /// half a minute and snaps back on change, so the sound reports change and a
    /// constant murmur does not become wallpaper over an hour.
    habit: f32,
    steady: f32,
    last_rate: f32,
    last_level: f32,
}

#[derive(Clone, Copy)]
struct Burst {
    active: bool,
    kind: GrainKind,
    rate0: f32,
    secs: f32,
    t: f32,
    wait: f32,
    pan: f32,
    spread: f32,
    gain: f32,
    muffle: f32,
}

impl Default for Burst {
    fn default() -> Self {
        Burst { active: false, kind: GrainKind::Hiss, rate0: 0.0, secs: 0.0, t: 0.0, wait: 0.0, pan: 0.0, spread: 0.0, gain: 0.0, muffle: 0.0 }
    }
}

#[derive(Clone, Copy, Default)]
struct Tone {
    active: bool,
    wait: u32,
    t: u32,
    len: u32,
    f0: f32,
    partials: [(f32, f32, f32); 5],
    env: [f32; 5],
    phase: [f32; 5],
    gl: f32,
    gr: f32,
}

/// Freeverb-lite: four damped combs and two all-passes per side. Small and
/// short on purpose -- the box is a glass tank on a bench, not a hall.
struct Reverb {
    combs: [[Vec<f32>; 4]; 2],
    comb_pos: [[usize; 4]; 2],
    comb_lp: [[f32; 4]; 2],
    aps: [[Vec<f32>; 2]; 2],
    ap_pos: [[usize; 2]; 2],
}

impl Reverb {
    fn new(sr: f32) -> Self {
        let scale = sr / 44_100.0;
        let len = |n: usize| ((n as f32 * scale) as usize).max(8);
        let comb_n = [1116, 1188, 1277, 1356];
        let ap_n = [556, 441];
        let mk_c = |spread: usize| [0, 1, 2, 3].map(|i| vec![0.0f32; len(comb_n[i] + spread)]);
        let mk_a = |spread: usize| [0, 1].map(|i| vec![0.0f32; len(ap_n[i] + spread)]);
        Reverb { combs: [mk_c(0), mk_c(23)], comb_pos: [[0; 4]; 2], comb_lp: [[0.0; 4]; 2], aps: [mk_a(0), mk_a(23)], ap_pos: [[0; 2]; 2] }
    }

    #[inline]
    fn run(&mut self, input: f32) -> [f32; 2] {
        const FEEDBACK: f32 = 0.78;
        const DAMP: f32 = 0.3;
        let mut out = [0.0f32; 2];
        for (ch, o) in out.iter_mut().enumerate() {
            let mut acc = 0.0;
            for i in 0..4 {
                let buf = &mut self.combs[ch][i];
                let p = self.comb_pos[ch][i];
                let y = buf[p];
                self.comb_lp[ch][i] = y * (1.0 - DAMP) + self.comb_lp[ch][i] * DAMP;
                buf[p] = input + self.comb_lp[ch][i] * FEEDBACK;
                self.comb_pos[ch][i] = (p + 1) % buf.len();
                acc += y;
            }
            for i in 0..2 {
                let buf = &mut self.aps[ch][i];
                let p = self.ap_pos[ch][i];
                let b = buf[p];
                buf[p] = acc + b * 0.5;
                acc = b - acc;
                self.ap_pos[ch][i] = (p + 1) % buf.len();
            }
            *o = acc * 0.25;
        }
        out
    }
}

/// What the voice bank did -- the counters a page puts beside a soundtrack,
/// because "did it fire" needs a number, not a recording (`CLAUDE.md`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SynthStats {
    pub grains: u64,
    pub grains_by_texture: [u64; 3],
    pub verb_grains: u64,
    pub bursts: u64,
    pub alerts: u64,
    pub steals: u64,
    pub samples: u64,
}

pub struct VoiceBank {
    sr: f32,
    rng: Xs,
    style: Style,
    grains: [Grain; GRAIN_POOL],
    textures: [Texture; 3],
    bursts: [Burst; BURSTS],
    tones: [Tone; TONES],
    reverb: Reverb,
    room_phase: f32,
    room_lp: f32,
    room_am: f32,
    hush: Smoothed,
    master: Smoothed,
    /// Per-sample smoothing coefficients, precomputed from time constants.
    k_fast: f32,
    k_slow: f32,
    k_mid: f32,
    k_hush: f32,
    block: u32,
    pub stats: SynthStats,
}

fn coef(seconds: f32, sr: f32) -> f32 {
    1.0 - (-1.0 / (seconds * sr)).exp()
}

/// Equal-power pan gains.
fn pan_gains(pan: f32) -> (f32, f32) {
    let th = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    (th.cos(), th.sin())
}

/// Soft limiter: linear to 0.6, then a tanh knee that never passes 1.
#[inline]
fn limit(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.6 {
        x
    } else {
        x.signum() * (0.6 + 0.4 * ((a - 0.6) / 0.4).tanh())
    }
}

impl VoiceBank {
    pub fn new(sample_rate: u32, seed: u64) -> Self {
        let sr = sample_rate as f32;
        let mut t = Texture { habit: 1.0, ..Texture::default() };
        t.bright.value = 1.0;
        t.bright.target = 1.0;
        VoiceBank {
            sr,
            rng: Xs(seed | 1),
            style: Style::default(),
            grains: [Grain::default(); GRAIN_POOL],
            textures: [t; 3],
            bursts: [Burst::default(); BURSTS],
            tones: [Tone::default(); TONES],
            reverb: Reverb::new(sr),
            room_phase: 0.0,
            room_lp: 0.0,
            room_am: 0.0,
            hush: Smoothed { value: 1.0, target: 1.0 },
            master: Smoothed { value: DEFAULT_MASTER * DEFAULT_MASTER, target: DEFAULT_MASTER * DEFAULT_MASTER },
            k_fast: coef(0.05, sr),
            k_slow: coef(0.4, sr),
            k_mid: coef(0.2, sr),
            k_hush: coef(0.1, sr),
            block: 0,
            stats: SynthStats::default(),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sr as u32
    }

    pub fn style(&self) -> Style {
        self.style
    }

    pub fn apply(&mut self, cmd: SoundCmd) {
        match cmd {
            SoundCmd::Texture { kind, rate, level, pan, spread, muffle, bright } => {
                let t = &mut self.textures[kind.index()];
                t.rate.target = rate.max(0.0);
                t.level.target = level.max(0.0);
                t.pan.target = pan;
                t.spread.target = spread;
                t.muffle.target = muffle.clamp(0.0, 1.0);
                t.bright.target = bright.max(0.2);
            }
            SoundCmd::Grain { kind, pan, gain, muffle, delay } => {
                self.spawn(kind, pan, gain, muffle, (delay.max(0.0) * self.sr) as u32, true, 255);
            }
            SoundCmd::Burst { kind, rate, secs, pan, spread, gain, muffle, delay } => {
                let slot = self.bursts.iter().position(|b| !b.active).or_else(|| {
                    // Full: replace the one nearest its end.
                    self.bursts.iter().enumerate().min_by(|a, b| (a.1.secs * 4.0 - a.1.t).total_cmp(&(b.1.secs * 4.0 - b.1.t))).map(|(i, _)| i)
                });
                if let Some(i) = slot {
                    self.bursts[i] = Burst { active: true, kind, rate0: rate, secs: secs.max(0.05), t: 0.0, wait: delay.max(0.0), pan, spread, gain, muffle };
                    self.stats.bursts += 1;
                }
            }
            SoundCmd::Alert { kind, weight, pan } => self.alert(kind, weight, pan),
            SoundCmd::Hush(on) => self.hush.target = if on { 0.0 } else { 1.0 },
            SoundCmd::Master(m) => {
                let m = m.clamp(0.0, 1.0);
                self.master.target = m * m;
            }
            SoundCmd::Style(s) => self.style = s,
        }
    }

    /// Start a grain, stealing the quietest if the pool is full. `tag` is the
    /// texture index (255 = not a texture), kept for the per-texture counter.
    #[allow(clippy::too_many_arguments)]
    fn spawn(&mut self, kind: GrainKind, pan: f32, gain: f32, muffle: f32, wait: u32, from_verb: bool, tag: u8) {
        if gain <= 1e-5 {
            return;
        }
        let kind_tag = kind as u8;
        let slot = if from_verb && self.grains.iter().filter(|g| g.active && g.from_verb && g.kind_tag == kind_tag).count() >= PER_KIND_CAP {
            self.grains.iter().enumerate().filter(|(_, g)| g.active && g.from_verb && g.kind_tag == kind_tag).max_by_key(|(_, g)| g.t).map(|(i, _)| i)
        } else {
            self.grains.iter().position(|g| !g.active)
        };
        let i = match slot {
            Some(i) => i,
            None => {
                self.stats.steals += 1;
                let loud = |g: &Grain| g.noise * g.noise_env + g.sine * g.sine_env;
                self.grains.iter().enumerate().min_by(|a, b| loud(a.1).total_cmp(&loud(b.1))).map(|(i, _)| i).unwrap_or(0)
            }
        };
        let s = recipe(kind, self.style, &mut self.rng);
        let sr = self.sr;
        let (gl, gr) = pan_gains(pan);
        let mut band = Biquad::default();
        band.set(s.band.max(40.0), s.q, sr);
        // Muffle: a one-pole low-pass from ~18 kHz (open) down to ~500 Hz (deep).
        let cutoff = 18_000.0 * (500.0f32 / 18_000.0).powf(muffle.clamp(0.0, 1.0));
        let lp_c = 1.0 - (-std::f32::consts::TAU * cutoff / sr).exp();
        let decay = |tau: f32| if tau > 0.0 { (-1.0 / (tau * sr)).exp() } else { 0.0 };
        self.grains[i] = Grain {
            active: true,
            kind_tag,
            from_verb,
            wait,
            t: 0,
            len: (s.len * sr).max(1.0) as u32,
            attack: (s.attack * sr).max(1.0) as u32,
            noise: s.noise * gain,
            noise_env: 1.0,
            noise_decay: decay(s.noise_tau),
            band,
            band0: s.band,
            band_end: s.band_end,
            q: s.q,
            rough: s.rough,
            rough_lp: 0.5,
            sine: s.sine * gain,
            sine_env: 1.0,
            sine_decay: decay(s.sine_tau),
            freq: s.freq,
            freq_end: s.freq_end,
            drop: s.drop,
            phase: 0.0,
            lp: [0.0; 2],
            lp_c,
            gl,
            gr,
        };
        self.stats.grains += 1;
        if from_verb {
            self.stats.verb_grains += 1;
        }
        if (tag as usize) < 3 {
            self.stats.grains_by_texture[tag as usize] += 1;
        }
    }

    fn alert(&mut self, kind: AlertKind, weight: f32, pan: f32) {
        let w = weight.clamp(0.0, 1.0);
        let bell = [(1.0, 1.0, 0.9), (2.0, 0.5, 0.6), (2.76, 0.35, 0.4), (5.4, 0.15, 0.2), (0.5, 0.3, 1.2)];
        let marimba = [(1.0, 1.0, 0.35), (3.93, 0.25, 0.08), (9.2, 0.08, 0.03), (0.0, 0.0, 0.1), (0.0, 0.0, 0.1)];
        let knock = [(1.0, 1.0, 0.09), (2.4, 0.4, 0.04), (5.3, 0.15, 0.01), (0.0, 0.0, 0.1), (0.0, 0.0, 0.1)];
        // (frequency, delay seconds, gain, partial set)
        let mut notes: [(f32, f32, f32, u8); 3] = [(0.0, 0.0, 0.0, 0); 3];
        let natural = self.style == Style::Natural;
        match kind {
            AlertKind::LineEnded if w < 0.34 => {
                notes[0] = if natural { (300.0, 0.0, 0.12 + 0.3 * w, 2) } else { (659.0, 0.0, 0.08 + 0.2 * w, 1) };
            }
            AlertKind::LineEnded => {
                let g = 0.12 + 0.18 * w;
                if natural {
                    notes[0] = (170.0, 0.0, g * 1.6, 2);
                    notes[1] = (120.0, 0.4, g * 1.8, 2);
                } else {
                    notes[0] = (164.8, 0.0, g, 0);
                    notes[1] = (110.0, 0.45, g * 1.1, 0);
                }
            }
            AlertKind::ColonyEnded => {
                let set = if natural { 2 } else { 0 };
                let g = if natural { 0.5 } else { 0.3 };
                notes[0] = (196.0, 0.0, g, set);
                notes[1] = (146.8, 0.5, g, set);
                notes[2] = (98.0, 1.0, g * 1.2, set);
            }
        }
        let (gl, gr) = pan_gains(pan * 0.6);
        for &(f, delay, g, set) in notes.iter().filter(|n| n.2 > 0.0) {
            let partials = match set {
                0 => bell,
                1 => marimba,
                _ => knock,
            };
            let longest = partials.iter().map(|p| p.2).fold(0.0f32, f32::max);
            if let Some(slot) = self.tones.iter_mut().find(|t| !t.active) {
                *slot = Tone {
                    active: true,
                    wait: (delay * self.sr) as u32,
                    t: 0,
                    len: (longest * 6.0 * self.sr) as u32,
                    f0: f,
                    partials: partials.map(|(r, a, tau)| (r, a * g, (-1.0 / (tau.max(1e-3) * self.sr)).exp())),
                    env: [1.0; 5],
                    phase: [0.0; 5],
                    gl,
                    gr,
                };
            }
        }
        self.stats.alerts += 1;
    }

    /// Run the per-block bookkeeping: habituation. Every 512 samples.
    fn block_update(&mut self) {
        let dt = 512.0 / self.sr;
        for t in self.textures.iter_mut() {
            let (r, l) = (t.rate.target, t.level.target);
            let rel = |a: f32, b: f32| (a - b).abs() / b.abs().max(1e-3);
            if rel(r, t.last_rate) < 0.15 && rel(l, t.last_level) < 0.15 {
                t.steady += dt;
            } else {
                t.steady = 0.0;
            }
            t.last_rate = r;
            t.last_level = l;
            let goal = if t.steady > 30.0 { 0.63 } else { 1.0 };
            let k = if goal < t.habit { dt / 6.0 } else { dt / 0.25 };
            t.habit += (goal - t.habit) * k.min(1.0);
        }
    }

    /// Fill `out` with interleaved samples for `channels` channels (1 or 2
    /// are mixed properly; more get the stereo pair in the first two and
    /// silence after).
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        let channels = channels.max(1);
        let sr = self.sr;
        let cap = if self.style == Style::Musical { MUSICAL_RATE_CAP } else { TEXTURE_RATE_CAP };
        for frame in out.chunks_mut(channels) {
            self.block += 1;
            if self.block >= 512 {
                self.block = 0;
                self.block_update();
            }
            let hush = self.hush.step(self.k_hush, self.k_hush * 0.33);

            // Textures: smooth the targets, then spawn grains Poisson-wise.
            for ti in 0..3 {
                let (rate, level, pan, spread, muffle, bright, habit) = {
                    let t = &mut self.textures[ti];
                    (
                        t.rate.step(self.k_fast, self.k_slow),
                        t.level.step(self.k_fast, self.k_slow),
                        t.pan.step(self.k_mid, self.k_mid),
                        t.spread.step(self.k_mid, self.k_mid),
                        t.muffle.step(self.k_mid, self.k_mid),
                        t.bright.step(self.k_mid, self.k_mid),
                        t.habit,
                    )
                };
                let rate = rate.min(cap);
                if rate > 0.01 && hush > 0.05 && level > 1e-4 && self.rng.unit() < rate / sr {
                    let p = pan + spread * self.rng.gauss() * 0.5;
                    let g = level * hush * habit * self.rng.range(0.6, 1.0);
                    let kind = TextureKind::ALL[ti].grain();
                    // `bright` > 1 (time-lapse) opens the muffle a little and
                    // shortens nothing else; kept simple on purpose for stage 1.
                    let m = (muffle - (bright - 1.0) * 0.3).clamp(0.0, 1.0);
                    self.spawn(kind, p, g, m, 0, false, ti as u8);
                }
            }

            // Bursts.
            for bi in 0..BURSTS {
                let b = self.bursts[bi];
                if !b.active {
                    continue;
                }
                let dt = 1.0 / sr;
                if b.wait > 0.0 {
                    self.bursts[bi].wait -= dt;
                    continue;
                }
                let rate = b.rate0 * (-b.t / b.secs).exp();
                self.bursts[bi].t += dt;
                if b.t > b.secs * 4.0 {
                    self.bursts[bi].active = false;
                    continue;
                }
                if self.rng.unit() < rate / sr {
                    let p = b.pan + b.spread * self.rng.gauss() * 0.5;
                    let g = b.gain * self.rng.range(0.6, 1.0);
                    self.spawn(b.kind, p, g, b.muffle, 0, false, 255);
                }
            }

            // Grains.
            let (mut l, mut r) = (0.0f32, 0.0f32);
            for g in self.grains.iter_mut() {
                if !g.active {
                    continue;
                }
                if g.wait > 0 {
                    g.wait -= 1;
                    continue;
                }
                let tf = g.t as f32;
                let life = tf / g.len as f32;
                let att = (tf / g.attack as f32).min(1.0);
                let mut y = 0.0;
                if g.noise > 0.0 {
                    if g.band_end != 1.0 && g.t % 64 == 0 {
                        let c = g.band0 * (1.0 + (g.band_end - 1.0) * life);
                        let q = g.q;
                        g.band.set(c, q, sr);
                    }
                    let mut n = g.band.run(self.rng.bipolar() * 1.73);
                    if g.rough > 0.0 {
                        g.rough_lp += (self.rng.unit() - g.rough_lp) * 0.02;
                        n *= 1.0 - g.rough + g.rough * 2.0 * g.rough_lp;
                    }
                    y += n * g.noise * g.noise_env;
                    g.noise_env *= g.noise_decay;
                }
                if g.sine > 0.0 {
                    let f = g.freq * (1.0 + (g.freq_end - 1.0) * life) * (1.0 + g.drop * (-tf / (0.02 * sr)).exp());
                    y += g.phase.sin() * g.sine * g.sine_env;
                    g.phase += std::f32::consts::TAU * f / sr;
                    if g.phase > std::f32::consts::TAU {
                        g.phase -= std::f32::consts::TAU;
                    }
                    g.sine_env *= g.sine_decay;
                }
                y *= att;
                g.lp[0] += (y - g.lp[0]) * g.lp_c;
                g.lp[1] += (g.lp[0] - g.lp[1]) * g.lp_c;
                l += g.lp[1] * g.gl;
                r += g.lp[1] * g.gr;
                g.t += 1;
                if g.t >= g.len || (g.noise_env + g.sine_env < 1e-4 && g.t > g.attack) {
                    g.active = false;
                }
            }

            // Tones (alerts).
            for t in self.tones.iter_mut() {
                if !t.active {
                    continue;
                }
                if t.wait > 0 {
                    t.wait -= 1;
                    continue;
                }
                let att = (t.t as f32 / (0.003 * sr)).min(1.0);
                let mut y = 0.0;
                for k in 0..5 {
                    let (ratio, amp, decay) = t.partials[k];
                    if amp <= 0.0 || ratio <= 0.0 {
                        continue;
                    }
                    y += t.phase[k].sin() * amp * t.env[k];
                    t.phase[k] = (t.phase[k] + std::f32::consts::TAU * t.f0 * ratio / sr) % std::f32::consts::TAU;
                    t.env[k] *= decay;
                }
                y *= att;
                l += y * t.gl;
                r += y * t.gr;
                t.t += 1;
                if t.t >= t.len {
                    t.active = false;
                }
            }

            // Room tone: the lamp's 120 Hz hum and a little low air.
            self.room_phase = (self.room_phase + std::f32::consts::TAU * 120.0 / sr) % std::f32::consts::TAU;
            self.room_am = (self.room_am + std::f32::consts::TAU * 0.3 / sr) % std::f32::consts::TAU;
            let hum = (self.room_phase.sin() + 0.4 * (2.0 * self.room_phase).sin() + 0.15 * (3.0 * self.room_phase).sin()) * (1.0 + 0.1 * self.room_am.sin());
            self.room_lp += (self.rng.bipolar() - self.room_lp) * 0.02;
            let room = (hum * 0.6 + self.room_lp * 2.0) * ROOM_LEVEL;

            let wet = self.reverb.run((l + r) * 0.5 * 0.35);
            let master = self.master.step(self.k_mid, self.k_mid) * MAKEUP;
            let lo = limit((l + wet[0] + room) * master);
            let ro = limit((r + wet[1] + room) * master);
            match channels {
                1 => frame[0] = (lo + ro) * 0.5,
                _ => {
                    frame[0] = lo;
                    frame[1] = ro;
                    for s in frame.iter_mut().skip(2) {
                        *s = 0.0;
                    }
                }
            }
            self.stats.samples += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(bank: &mut VoiceBank, secs: f32) -> Vec<f32> {
        let mut out = vec![0.0; (secs * bank.sr) as usize * 2];
        bank.render(&mut out, 2);
        out
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn walk(rate: f32, level: f32) -> SoundCmd {
        SoundCmd::Texture { kind: TextureKind::Walk, rate, level, pan: 0.0, spread: 0.3, muffle: 0.0, bright: 1.0 }
    }

    #[test]
    fn the_same_commands_and_seed_render_the_same_samples() {
        let run = || {
            let mut b = VoiceBank::new(48_000, 7);
            b.apply(walk(150.0, 0.5));
            b.apply(SoundCmd::Alert { kind: AlertKind::LineEnded, weight: 0.8, pan: 0.0 });
            render(&mut b, 1.0)
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn a_silent_bank_plays_only_the_room_tone() {
        let mut b = VoiceBank::new(48_000, 1);
        let x = render(&mut b, 1.0);
        let level = rms(&x);
        assert!(level > 0.0, "the room tone must be there: a dead-silent box reads as broken");
        assert!(level < 0.02, "room tone {level} is louder than a whisper");
    }

    /// The positive control for the test above and the one below: a busy
    /// texture must be much louder than the room, or "quiet" and "busy" would
    /// be indistinguishable and both tests would pass on a dead bank.
    #[test]
    fn a_texture_is_heard_over_the_room_and_stays_under_full_scale() {
        let mut quiet = VoiceBank::new(48_000, 1);
        let room = rms(&render(&mut quiet, 1.0));
        let mut busy = VoiceBank::new(48_000, 1);
        busy.apply(walk(200.0, 0.8));
        let x = render(&mut busy, 1.0);
        assert!(rms(&x) > room * 3.0, "texture {} vs room {room}", rms(&x));
        assert!(x.iter().all(|v| v.abs() <= 1.0), "the limiter let a sample past full scale");
        assert!(busy.stats.grains_by_texture[0] > 100, "200/s for a second spawned {}", busy.stats.grains_by_texture[0]);
    }

    #[test]
    fn hush_fades_the_textures_but_not_verbs() {
        let mut b = VoiceBank::new(48_000, 3);
        b.apply(walk(200.0, 0.8));
        render(&mut b, 0.3);
        b.apply(SoundCmd::Hush(true));
        render(&mut b, 1.0);
        let before = b.stats.grains_by_texture[0];
        render(&mut b, 0.5);
        assert!(b.stats.grains_by_texture[0] - before < 5, "a hushed texture kept spawning");
        b.apply(SoundCmd::Grain { kind: GrainKind::Tick, pan: 0.0, gain: 0.5, muffle: 0.0, delay: 0.0 });
        render(&mut b, 0.1);
        assert_eq!(b.stats.verb_grains, 1, "a verb must still answer while paused");
    }

    #[test]
    fn the_pool_steals_instead_of_growing() {
        let mut b = VoiceBank::new(48_000, 5);
        for i in 0..(GRAIN_POOL + 10) {
            let kind = if i % 2 == 0 { GrainKind::Thud } else { GrainKind::Knock };
            b.spawn(kind, 0.0, 0.5, 0.0, 0, false, 255);
        }
        assert_eq!(b.stats.steals, 10);
        assert!(b.grains.iter().all(|g| g.active));
    }

    #[test]
    fn verb_grains_of_one_kind_are_capped() {
        let mut b = VoiceBank::new(48_000, 5);
        for _ in 0..10 {
            b.apply(SoundCmd::Grain { kind: GrainKind::Thunk, pan: 0.0, gain: 0.5, muffle: 0.0, delay: 0.0 });
        }
        let live = b.grains.iter().filter(|g| g.active && g.kind_tag == GrainKind::Thunk as u8).count();
        assert_eq!(live, PER_KIND_CAP);
    }

    #[test]
    fn a_steady_texture_habituates_and_a_change_wakes_it() {
        let mut b = VoiceBank::new(8_000, 9);
        b.apply(walk(100.0, 0.5));
        render(&mut b, 40.0);
        let settled = b.textures[0].habit;
        assert!(settled < 0.8, "after 40 s of no change habit is {settled}");
        b.apply(walk(300.0, 0.9));
        render(&mut b, 1.0);
        assert!(b.textures[0].habit > 0.95, "a change should wake it, habit {}", b.textures[0].habit);
    }

    #[test]
    fn every_recipe_makes_a_finite_bounded_sound() {
        let kinds = [
            GrainKind::Click,
            GrainKind::Grit,
            GrainKind::Patter,
            GrainKind::Hiss,
            GrainKind::Drip,
            GrainKind::Splash,
            GrainKind::Thud,
            GrainKind::Knock,
            GrainKind::Tick,
            GrainKind::Thunk,
            GrainKind::Crackle,
            GrainKind::Whoosh,
        ];
        for style in [Style::Hybrid, Style::Natural, Style::Musical] {
            for kind in kinds {
                let mut b = VoiceBank::new(48_000, 11);
                b.apply(SoundCmd::Style(style));
                b.apply(SoundCmd::Grain { kind, pan: 0.0, gain: 1.0, muffle: 0.0, delay: 0.0 });
                let x = render(&mut b, 0.6);
                assert!(x.iter().all(|v| v.is_finite() && v.abs() <= 1.0), "{kind:?} {style:?}");
                assert!(rms(&x) > 0.0005, "{kind:?} {style:?} made no sound");
            }
        }
    }
}
