//! **Sound.** The design, and the reasons for every choice below, are
//! `Reports/sound-design-2026-10-31.md`; this module is its stage 1 -- one lab
//! box you can hear.
//!
//! Three pieces, and the split is the point:
//!
//! - [`listen`] reads the world and decides what should be heard. It takes
//!   `&World` and nothing else of the simulation, so the borrow checker proves
//!   it cannot change a cell, a counter or an RNG draw. It has its own random
//!   numbers. It runs on the game thread, once per *drawn* frame, beside the
//!   renderer (`Lab::paint`), and emits [`SoundCmd`]s.
//! - [`synth`] turns commands into samples: a fixed pool of short procedural
//!   grains, one granular texture per kind of activity, a few tones for alerts,
//!   a room tone, a small reverb and a soft limiter. Pure Rust, no allocation
//!   after start, a pure function of its command stream and seed -- so the
//!   offline render (`examples/soundtrack.rs`) is the same code as the live one.
//! - `device` is the only file that touches an audio API (`cpal`), and the
//!   only file that is platform-gated: it exists on macOS and Windows, and on
//!   Linux only with `--features audio-alsa` (see `Cargo.toml` for why).
//!
//! **Never one voice per ant or per cell.** Thousands of sources become a
//! handful of textures whose grain rate follows the count up to a ceiling and
//! whose level follows its square root (`listen::density`); that is the whole
//! answer to "how do a thousand ants not sound like static", and the sketch
//! that demonstrates it is on the design's listening page.
//!
//! **Training never reaches any of this.** Nothing outside `bin/lab.rs`,
//! `Lab::paint` (behind `Lab::sound_on`, false unless a binary turns it
//! on) and `examples/soundtrack.rs` constructs a listener; the guards are in
//! `tests/sound.rs`.

pub mod listen;
pub mod synth;
pub mod wav;

#[cfg(any(target_os = "macos", target_os = "windows", all(target_os = "linux", feature = "audio-alsa")))]
pub mod device;

/// Whether this build can open a sound device at all. `false` on a default
/// Linux build (CI, cloud sessions, the training VM), where the device file is
/// not compiled -- the lab then runs silently, exactly as before sound existed.
pub const DEVICE_COMPILED: bool =
    cfg!(any(target_os = "macos", target_os = "windows", all(target_os = "linux", feature = "audio-alsa")));

/// **The speaker, or nothing.** What `bin/lab.rs` holds, so that binary needs
/// no `cfg` of its own: on a build without a device this is an empty shell
/// whose `send` drops the commands, and the lab runs exactly as it did before
/// sound existed.
pub struct Output {
    #[cfg(any(target_os = "macos", target_os = "windows", all(target_os = "linux", feature = "audio-alsa")))]
    device: device::Device,
}

impl Output {
    /// Open the default output, or say why not -- no device compiled, no
    /// device present (a VM, a container, a headless screenshot run), or the
    /// host refused the stream. The caller runs silent on any `Err`.
    pub fn open(seed: u64, style: Style) -> Result<Output, String> {
        #[cfg(any(target_os = "macos", target_os = "windows", all(target_os = "linux", feature = "audio-alsa")))]
        {
            device::Device::open(seed, style).map(|device| Output { device })
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", all(target_os = "linux", feature = "audio-alsa"))))]
        {
            let _ = (seed, style);
            Err("this build has no sound device (a Linux build without --features audio-alsa)".to_string())
        }
    }

    pub fn send(&mut self, cmds: impl IntoIterator<Item = SoundCmd>) {
        #[cfg(any(target_os = "macos", target_os = "windows", all(target_os = "linux", feature = "audio-alsa")))]
        self.device.send(cmds);
        #[cfg(not(any(target_os = "macos", target_os = "windows", all(target_os = "linux", feature = "audio-alsa"))))]
        drop(cmds.into_iter());
    }

    /// Commands dropped because the device fell behind -- should stay 0.
    pub fn dropped(&self) -> u64 {
        #[cfg(any(target_os = "macos", target_os = "windows", all(target_os = "linux", feature = "audio-alsa")))]
        return self.device.dropped;
        #[cfg(not(any(target_os = "macos", target_os = "windows", all(target_os = "linux", feature = "audio-alsa"))))]
        0
    }
}

/// Kinds of continuous activity, one granular texture voice each.
///
/// Stage 1 is the colony: walking, digging, and the softer patter of ants
/// carrying something. The world textures (sand, water, fire) are stage 2 and
/// need the strided snapshot diff the design's §3b describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureKind {
    Walk,
    Dig,
    Carry,
}

impl TextureKind {
    pub const ALL: [TextureKind; 3] = [TextureKind::Walk, TextureKind::Dig, TextureKind::Carry];

    pub fn index(self) -> usize {
        match self {
            TextureKind::Walk => 0,
            TextureKind::Dig => 1,
            TextureKind::Carry => 2,
        }
    }

    /// The grain one of this texture's events is made of.
    pub fn grain(self) -> GrainKind {
        match self {
            TextureKind::Walk => GrainKind::Click,
            TextureKind::Dig => GrainKind::Grit,
            TextureKind::Carry => GrainKind::Patter,
        }
    }
}

/// One short procedural sound. Each has a recipe in `synth::recipe`; every
/// instance is jittered in pitch, length and filter so no two are identical
/// (the machine-gun effect is the commonest reason game audio sounds cheap).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrainKind {
    /// An ant's step: a 3-8 ms bright noise tick.
    Click,
    /// A dig: a rough 20-45 ms scrape.
    Grit,
    /// A laden step: softer and lower than `Click`.
    Patter,
    /// One grain of a sand trickle or of water soaking into dry ground.
    Hiss,
    /// A water drop: a short rising sine.
    Drip,
    /// Water striking something hard.
    Splash,
    /// A heavy, low impact.
    Thud,
    /// A small hard impact.
    Knock,
    /// The interface's own wooden tick on a tool press.
    Tick,
    /// Something soft set down (food).
    Thunk,
    /// One pop of a fire.
    Crackle,
    /// Air moving fast: a rising band of noise (a fling).
    Whoosh,
}

/// Something that matters, graded by how much of the colony it cost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertKind {
    /// A founding line ran out. `weight` is the share of the colony's lines it
    /// was (one of many: soft; one of the last few: the full fall).
    LineEnded,
    /// The last animal of the colony died.
    ColonyEnded,
}

/// Which family of recipes the voice bank plays. The listener and the mix are
/// identical across all three; only grains and alert tones change. The owner
/// picks one from the blind sketches on the design page; until then
/// [`Style::Hybrid`] is the recommendation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Style {
    /// Natural sounds for the world, tuned tones for alerts.
    #[default]
    Hybrid,
    /// Natural sounds everywhere, alerts included (hollow wood knocks).
    Natural,
    /// The colony's steps as soft pentatonic plucks, tuned alerts.
    Musical,
}

impl Style {
    pub fn parse(s: &str) -> Option<Style> {
        match s.trim().to_ascii_lowercase().as_str() {
            "hybrid" => Some(Style::Hybrid),
            "natural" => Some(Style::Natural),
            "musical" => Some(Style::Musical),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Style::Hybrid => "HYBRID",
            Style::Natural => "NATURAL",
            Style::Musical => "MUSICAL",
        }
    }
}

/// What the game thread tells the voice bank. **Textures carry absolute
/// targets**, re-sent every drawn frame, so a dropped command costs one frame
/// of staleness and never a stuck sound.
///
/// Positions are already reduced to what the ear uses: `pan` in -1 (left of
/// the view) ..= 1 (right), `muffle` 0 (open air) ..= 1 (deep in the soil, or
/// off-screen), `delay` in seconds of wall clock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SoundCmd {
    Texture { kind: TextureKind, rate: f32, level: f32, pan: f32, spread: f32, muffle: f32, bright: f32 },
    Grain { kind: GrainKind, pan: f32, gain: f32, muffle: f32, delay: f32 },
    /// A short-lived texture whose rate decays over `secs` -- water soaking
    /// in, a soil pour settling, a fire's first crackles.
    Burst { kind: GrainKind, rate: f32, secs: f32, pan: f32, spread: f32, gain: f32, muffle: f32, delay: f32 },
    Alert { kind: AlertKind, weight: f32, pan: f32 },
    /// The box is paused: world textures fade out, the room tone stays, and
    /// verb sounds still answer.
    Hush(bool),
    /// Master level, 0 ..= 1 (perceptual-ish; squared inside).
    Master(f32),
    Style(Style),
}
