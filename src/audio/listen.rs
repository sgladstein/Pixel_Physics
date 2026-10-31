//! **The listener**: what in the world should be heard, and how loud.
//!
//! Reads `&World` and nothing mutable of the simulation -- the borrow checker
//! is the first guard that sound cannot change the game, and `tests/sound.rs`
//! is the second. It keeps its own memory of the last frame it saw (each
//! ant's head, the colony counters, the run log's per-kind totals) and turns
//! the difference into [`SoundCmd`]s. It has no random numbers at all; the
//! voice bank owns those.
//!
//! **Called once per drawn frame** (`Lab::paint`), with however many ticks ran
//! since the last call. That count is the delivered speed, and it decides the
//! mode (design §3d) -- never the requested preset, because a box asked for
//! 1024x that manages 4x should sound like 4x.
//!
//! What stage 1 listens to, and where each number comes from:
//!
//! | Sound | Source | |
//! |---|---|---|
//! | walking texture | each live animal's head vs. where it was last frame, weighted by where it is relative to the view | positional |
//! | carrying patter | the same ants, `OrganismState::crop` set | positional |
//! | digging texture | `CreatureStats::digs + gnaws`, diffed per tick; placed where the ants below ground are | colony-wide count, positional *approximation* |
//! | line ended | `RunLog::pushed(LineEnded)` diffed, each new event read from `recent()`, animal lines only, weighted by the line's peak share of the colony | graded alert |
//! | colony ended | live animals going from some to none | alert |
//! | your body's steps | `World::steered`'s head moving, real time only | grain |
//!
//! **A rebuild replaces the world** and every cumulative counter restarts at
//! zero (`World::new`); the listener notices the frame or a counter going
//! backwards and re-baselines instead of reading a negative or enormous diff.

use super::{AlertKind, GrainKind, SoundCmd, TextureKind};
use crate::sim::cell::OrganismId;
use crate::sim::fxhash::FxHashMap;
use crate::sim::material::{self, MaterialKind};
use crate::sim::world::{LogKind, World};

/// The ticks-per-drawn-frame below which a displayed frame still reads as
/// motion -- the engine's own crossover, reused so the sound switches where the
/// picture does (`lab::time::MOTION_TICKS_PER_FRAME`).
pub const REAL_TIME_BELOW: u64 = crate::lab::time::MOTION_TICKS_PER_FRAME as u64;
/// Past this many ticks per drawn frame, textures blur into an overview.
/// A first guess (design §3d), to be tuned by ear.
pub const OVERVIEW_FROM: u64 = 100;
/// Wall-clock gap between two alerts, unless the second is heavy. The same
/// spacing as the clock's `REACTION_COOLDOWN` (`lab/time.rs`, private), for the
/// same reason: at scale a line can end every few frames.
pub const ALERT_SPACING: f32 = 4.0;
/// An alert this heavy (the colony's last lines) plays through the spacing.
pub const ALERT_OVERRIDE: f32 = 0.6;
/// An alert this light is not played at all: one line of many ending is
/// ordinary turnover, and stage 3's colony-health bed is where it belongs.
pub const ALERT_FLOOR: f32 = 0.04;
/// Grains a moving ant contributes per second to the walking texture (before
/// the voice bank's rate cap).
const STEPS_PER_ANT: f32 = 6.0;
/// The exponent of the loudness law: amplitude follows `count^LOUDNESS_LAW`.
/// 0.4 makes a hundredfold colony about 16 dB louder rather than 40
/// (design §3b); the sketch "naive vs designed" is why.
pub const LOUDNESS_LAW: f32 = 0.4;
/// Ants outside the view but within one view-width of it still count, at
/// this weight and duller, so a busy nest just off-screen is heard.
const OFFSCREEN_WEIGHT: f32 = 0.35;
/// Soil depth at which a sound is fully muffled, in cells below the surface.
const MUFFLE_DEPTH: f32 = 48.0;

/// What part of the world the player is looking at, in world cells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub x0: i32,
    pub y0: i32,
    pub w: i32,
    pub h: i32,
    /// The ground row: sources below it are muffled by the soil above them.
    pub surface_y: i32,
}

impl View {
    /// -1 at the left edge of the view, 1 at the right; beyond the edges up to
    /// ±1.6, clamped by the caller's pan.
    fn pan_of(&self, x: i32) -> f32 {
        ((x - self.x0) as f32 + 0.5) / self.w.max(1) as f32 * 2.0 - 1.0
    }

    /// 1 inside the view, `OFFSCREEN_WEIGHT` within one view-size of it, 0
    /// beyond.
    fn weight_of(&self, (x, y): (i32, i32)) -> f32 {
        let dx = if x < self.x0 { self.x0 - x } else if x >= self.x0 + self.w { x - (self.x0 + self.w - 1) } else { 0 };
        let dy = if y < self.y0 { self.y0 - y } else if y >= self.y0 + self.h { y - (self.y0 + self.h - 1) } else { 0 };
        if dx == 0 && dy == 0 {
            1.0
        } else if dx <= self.w && dy <= self.h {
            OFFSCREEN_WEIGHT
        } else {
            0.0
        }
    }

    fn muffle_of(&self, y: i32) -> f32 {
        ((y - self.surface_y) as f32 / MUFFLE_DEPTH).clamp(0.0, 1.0) * 0.85
    }
}

/// A player verb the listener should answer (design §4). The lab maps its
/// tools onto these; anything without a sound of its own yet is a plain `Tick`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Tick,
    Water,
    Soil,
    Dig,
    Erase,
    Food,
    Fling,
    Fire,
}

/// The mode a drawn frame's delivered speed puts the sound in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Hush,
    RealTime,
    TimeLapse,
    Overview,
}

impl Mode {
    pub fn of(ticks: u64) -> Mode {
        if ticks == 0 {
            Mode::Hush
        } else if ticks < REAL_TIME_BELOW {
            Mode::RealTime
        } else if ticks < OVERVIEW_FROM {
            Mode::TimeLapse
        } else {
            Mode::Overview
        }
    }
}

/// `(rate, level)` for a texture driven by `count` sources each worth
/// `per_source` grains a second, `reference` being the count that plays at
/// `level_at_reference`. Rate follows the count up to the voice bank's cap;
/// amplitude follows `count^LOUDNESS_LAW`, divided by the square root of the
/// rate actually played so overlapping grains do not add up to more.
pub fn density(count: f32, per_source: f32, reference: f32, level_at_reference: f32) -> (f32, f32) {
    if count <= 0.0 {
        return (0.0, 0.0);
    }
    let rate = (count * per_source).min(super::synth::TEXTURE_RATE_CAP);
    let amplitude = level_at_reference * (count / reference).powf(LOUDNESS_LAW);
    // Normalised so that at the reference count the per-grain gain is
    // `level_at_reference` scaled for its own rate against a 50/s reference.
    let level = (amplitude / (rate / 50.0).max(0.02).sqrt()).min(1.0);
    (rate, level)
}

/// Counters for "did it fire" -- printed beside every soundtrack.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ListenStats {
    pub observes: u64,
    pub rebaselines: u64,
    pub ants: u32,
    pub moving_heard: f32,
    pub digs_per_sec: f32,
    pub alerts_sent: u64,
    pub alerts_held: u64,
    pub alerts_too_light: u64,
    pub verbs: u64,
    pub body_steps: u64,
    pub mode: Option<Mode>,
    /// Wall-clock seconds spent in `observe`, summed, and the worst single
    /// call -- the listener's whole cost on the game thread.
    pub observe_secs: f64,
    pub observe_worst: f64,
}

#[derive(Clone, Copy, Debug)]
struct Baseline {
    frame: u64,
    digs: u64,
    line_ended: u64,
    animals: usize,
}

/// One weighted-position accumulator: mean and spread of pan, mean muffle.
#[derive(Default)]
struct Spot {
    w: f32,
    pan: f32,
    pan2: f32,
    muffle: f32,
}

impl Spot {
    fn add(&mut self, w: f32, pan: f32, muffle: f32) {
        self.w += w;
        self.pan += w * pan;
        self.pan2 += w * pan * pan;
        self.muffle += w * muffle;
    }
    fn mean(&self) -> (f32, f32, f32) {
        if self.w <= 0.0 {
            return (0.0, 0.3, 0.0);
        }
        let m = self.pan / self.w;
        let var = (self.pan2 / self.w - m * m).max(0.0);
        (m.clamp(-1.0, 1.0), var.sqrt().clamp(0.05, 0.9), (self.muffle / self.w).clamp(0.0, 1.0))
    }
}

pub struct Listener {
    heads: FxHashMap<OrganismId, (i32, i32)>,
    next_heads: FxHashMap<OrganismId, (i32, i32)>,
    base: Option<Baseline>,
    since_alert: f32,
    hushed: bool,
    /// Sim seconds per wall second at the last observe; 0 while paused. Verbs
    /// use it to time a landing.
    speed: f32,
    /// Wall seconds since each verb's last burst, so a drag does not stack a
    /// burst per mouse move.
    since_verb: [f32; 8],
    body: Option<(OrganismId, (i32, i32))>,
    out: Vec<SoundCmd>,
    pub stats: ListenStats,
}

impl Default for Listener {
    fn default() -> Self {
        Self::new()
    }
}

impl Listener {
    pub fn new() -> Self {
        Listener {
            heads: FxHashMap::default(),
            next_heads: FxHashMap::default(),
            base: None,
            since_alert: ALERT_SPACING,
            hushed: false,
            speed: 1.0,
            since_verb: [1.0; 8],
            body: None,
            out: Vec::with_capacity(32),
            stats: ListenStats::default(),
        }
    }

    /// The commands produced since the last drain, oldest first.
    pub fn drain(&mut self) -> std::vec::Drain<'_, SoundCmd> {
        self.out.drain(..)
    }

    /// Queue a command from outside the listener (the lab's SOUND row).
    pub fn push(&mut self, cmd: SoundCmd) {
        self.out.push(cmd);
    }

    pub fn pending(&self) -> &[SoundCmd] {
        &self.out
    }

    /// Look at the world once, after a drawn frame. `wall_dt` is real seconds
    /// since the last call (the offline render passes its nominal frame time,
    /// so it stays deterministic).
    pub fn observe(&mut self, world: &World, view: View, wall_dt: f32) {
        // Timed here because the design's budget (0.25 ms a drawn frame, §9)
        // is this call's, and `examples/soundtrack.rs` prints it. The clock
        // reaches only the stats, never a command, so the render stays a pure
        // function of the world.
        let t0 = std::time::Instant::now();
        self.observe_inner(world, view, wall_dt);
        let secs = t0.elapsed().as_secs_f64();
        self.stats.observe_secs += secs;
        self.stats.observe_worst = self.stats.observe_worst.max(secs);
    }

    fn observe_inner(&mut self, world: &World, view: View, wall_dt: f32) {
        self.stats.observes += 1;
        let wall_dt = wall_dt.clamp(0.0, 1.0);
        self.since_alert += wall_dt;
        for s in self.since_verb.iter_mut() {
            *s += wall_dt;
        }

        // ---- the colony: one pass over live animals ----
        let mut walk = Spot::default();
        let mut carry = Spot::default();
        let mut below = Spot::default();
        let (mut walk_n, mut carry_n) = (0.0f32, 0.0f32);
        let (mut animals, mut below_total, mut below_seen) = (0usize, 0.0f32, 0.0f32);
        self.next_heads.clear();
        for (id, state) in world.live_organisms() {
            if world.species.get(state.species).creature.is_none() {
                continue;
            }
            let Some(&head) = state.chain.first() else { continue };
            animals += 1;
            self.next_heads.insert(id, head);
            let underground = head.1 > view.surface_y;
            let w = view.weight_of(head);
            if underground {
                below_total += 1.0;
                below_seen += w;
            }
            if w <= 0.0 {
                continue;
            }
            let pan = view.pan_of(head.0);
            let muffle = (view.muffle_of(head.1) + if w < 1.0 { 0.4 } else { 0.0 }).min(1.0);
            if underground {
                below.add(w, pan, muffle);
            }
            let moved = self.heads.get(&id).is_some_and(|&was| was != head);
            if moved {
                walk.add(w, pan, muffle);
                walk_n += w;
                if state.crop.is_some() {
                    carry.add(w, pan, muffle);
                    carry_n += w;
                }
            }
        }
        std::mem::swap(&mut self.heads, &mut self.next_heads);
        self.stats.ants = animals as u32;

        let digs = world.creature_stats.digs + world.creature_stats.gnaws;
        let line_ended = world.run_log.pushed(LogKind::LineEnded);

        // ---- baseline, and the rebuild check ----
        let reset = match self.base {
            None => true,
            Some(b) => world.frame < b.frame || digs < b.digs || line_ended < b.line_ended,
        };
        if reset {
            if self.base.is_some() {
                self.stats.rebaselines += 1;
            }
            self.base = Some(Baseline { frame: world.frame, digs, line_ended, animals });
            self.heads.clear();
            for (id, state) in world.live_organisms() {
                if let Some(&head) = state.chain.first() {
                    self.heads.insert(id, head);
                }
            }
            return;
        }
        let base = self.base.expect("set above");
        let ticks = world.frame - base.frame;
        let mode = Mode::of(ticks);
        self.stats.mode = Some(mode);
        self.speed = if wall_dt > 0.0 { ticks as f32 / crate::lab::time::TICKS_PER_SECOND as f32 / wall_dt } else { 0.0 };

        if mode == Mode::Hush {
            if !self.hushed {
                self.out.push(SoundCmd::Hush(true));
                self.hushed = true;
            }
            self.base = Some(Baseline { animals, ..base });
            return;
        }
        if self.hushed {
            self.out.push(SoundCmd::Hush(false));
            self.hushed = false;
        }

        // Mode shaping: time-lapse is brighter; overview is quieter and duller.
        let (level_mul, muffle_add, bright) = match mode {
            Mode::TimeLapse => (0.9, 0.0, 1.4),
            Mode::Overview => (0.5, 0.35, 1.0),
            _ => (1.0, 0.0, 1.0),
        };

        // Walking: positional, a state count, so independent of the dial.
        let (rate, level) = density(walk_n, STEPS_PER_ANT, 50.0, 0.35);
        let (pan, spread, muffle) = walk.mean();
        self.out.push(SoundCmd::Texture { kind: TextureKind::Walk, rate, level: level * level_mul, pan, spread, muffle: (muffle + muffle_add).min(1.0), bright });
        self.stats.moving_heard = walk_n;

        // Carrying: softer patter laid over the walk.
        let (rate, level) = density(carry_n, 4.0, 20.0, 0.22);
        let (pan, spread, muffle) = carry.mean();
        self.out.push(SoundCmd::Texture { kind: TextureKind::Carry, rate, level: level * level_mul, pan, spread, muffle: (muffle + muffle_add).min(1.0), bright });

        // Digging: the colony counter, per simulated second (so the dial does
        // not make a nest dig louder), heard where the ants below ground are
        // and as much as they are in view.
        let per_sec = (digs - base.digs) as f32 / ticks as f32 * crate::lab::time::TICKS_PER_SECOND as f32;
        let seen = if below_total > 0.0 { (below_seen / below_total).clamp(0.0, 1.0) } else { 0.0 };
        let (rate, level) = density(per_sec * seen, 1.0, 15.0, 0.4);
        let (pan, spread, muffle) = below.mean();
        self.out.push(SoundCmd::Texture { kind: TextureKind::Dig, rate, level: level * level_mul, pan, spread, muffle: (muffle + muffle_add).min(1.0), bright });
        self.stats.digs_per_sec = per_sec;

        // Your body, close-miked, in real time only.
        if let Some(steered) = world.steered.as_ref() {
            let head = world.organism_state(steered.id).and_then(|s| s.chain.first().copied());
            if let Some(head) = head {
                if mode == Mode::RealTime && self.body.is_some_and(|(id, was)| id == steered.id && was != head) {
                    let muffle = view.muffle_of(head.1) * 0.5;
                    self.out.push(SoundCmd::Grain { kind: GrainKind::Click, pan: view.pan_of(head.0).clamp(-1.0, 1.0) * 0.7, gain: 0.45, muffle, delay: 0.0 });
                    self.stats.body_steps += 1;
                }
                self.body = Some((steered.id, head));
            }
        } else {
            self.body = None;
        }

        // ---- alerts ----
        let new_lines = line_ended - base.line_ended;
        if new_lines > 0 {
            let mut heaviest: Option<(f32, f32)> = None; // (weight, pan)
            for e in world.run_log.recent().filter(|e| e.kind == LogKind::LineEnded).take(new_lines.min(64) as usize) {
                if world.species.get(e.species).creature.is_none() {
                    continue;
                }
                let peak = world.line_stats.get(&e.lineage).map_or(1, |l| l.peak_living.max(1)) as f32;
                let weight = peak / (peak + animals as f32);
                let pan = world.graveyard.recent().find(|g| g.id == e.id && g.born_frame == e.born_frame).map_or(0.0, |g| view.pan_of(g.at.0).clamp(-1.0, 1.0));
                if heaviest.is_none_or(|(w, _)| weight > w) {
                    heaviest = Some((weight, pan));
                }
            }
            if let Some((weight, pan)) = heaviest {
                self.alert(AlertKind::LineEnded, weight, pan);
            }
        }
        if base.animals > 0 && animals == 0 {
            self.alert(AlertKind::ColonyEnded, 1.0, 0.0);
        }

        self.base = Some(Baseline { frame: world.frame, digs, line_ended, animals });
    }

    fn alert(&mut self, kind: AlertKind, weight: f32, pan: f32) {
        if weight < ALERT_FLOOR {
            self.stats.alerts_too_light += 1;
            return;
        }
        if self.since_alert < ALERT_SPACING && weight < ALERT_OVERRIDE {
            self.stats.alerts_held += 1;
            return;
        }
        self.since_alert = 0.0;
        self.stats.alerts_sent += 1;
        self.out.push(SoundCmd::Alert { kind, weight, pan });
    }

    /// The player did something at `at` (world cells) that touched `cells`
    /// cells. `first` is the press that starts a gesture, which gets the
    /// interface tick; drag segments after it do not.
    ///
    /// **Where the world can answer, it does**: water's sound depends on what
    /// it will land on and arrives after the fall, at the speed the box is
    /// running; while paused nothing falls, so only the tick plays.
    pub fn verb(&mut self, world: &World, view: View, verb: Verb, at: (i32, i32), cells: u32, first: bool) {
        self.stats.verbs += 1;
        let pan = view.pan_of(at.0).clamp(-1.0, 1.0);
        let muffle = view.muffle_of(at.1) * 0.5;
        if first {
            self.out.push(SoundCmd::Grain { kind: GrainKind::Tick, pan, gain: 0.35, muffle: 0.0, delay: 0.0 });
        }
        let slot = verb as usize;
        if self.since_verb[slot] < 0.15 || cells == 0 && !matches!(verb, Verb::Fling | Verb::Fire) {
            return;
        }
        let running = self.speed > 0.0;
        let size = (cells.max(1) as f32 / 20.0).sqrt().clamp(0.4, 2.5);
        // Fall: how far below `at` the first thing that is not air is, and what
        // it is. Bounded scan; a fall past it is heard as "far".
        let (fall, landing) = {
            let mut d = 0;
            let mut hit = material::EMPTY;
            for y in at.1..at.1 + 160 {
                let c = world.get(at.0, y);
                if y > at.1 && c.material != material::EMPTY && c.material != material::WATER {
                    hit = c.material;
                    break;
                }
                if y > at.1 && c.material == material::WATER {
                    hit = material::WATER;
                    break;
                }
                d += 1;
            }
            (d, hit)
        };
        let speed = self.speed.max(0.25);
        let delay = (fall as f32 / crate::lab::time::TICKS_PER_SECOND as f32 / speed).min(1.5);
        let emitted = match verb {
            Verb::Tick => false,
            Verb::Water if running => {
                match world.materials.kind(landing) {
                    MaterialKind::Powder | MaterialKind::Plant => {
                        let c = world.get(at.0, at.1 + fall);
                        let dry = crate::sim::update::soil_moisture(c) < material::SOIL_FIELD_CAPACITY / 2;
                        let (hiss, drips) = if dry { (140.0, 5.0) } else { (40.0, 12.0) };
                        self.burst(GrainKind::Hiss, hiss * size, 0.9, pan, 0.3 * size.min(1.5), muffle, delay);
                        self.burst(GrainKind::Drip, drips * size, 0.6, pan, 0.3, muffle, delay);
                    }
                    MaterialKind::Liquid => self.burst(GrainKind::Drip, 14.0 * size, 0.8, pan, 0.35, muffle, delay),
                    _ => {
                        self.burst(GrainKind::Splash, 10.0 * size, 0.6, pan, 0.35, muffle, delay);
                        self.burst(GrainKind::Drip, 10.0 * size, 1.0, pan, 0.3, muffle, delay);
                    }
                }
                true
            }
            Verb::Soil if running => {
                let secs = 0.35 + (fall as f32 / crate::lab::time::TICKS_PER_SECOND as f32 / speed).min(1.2);
                self.burst(GrainKind::Hiss, 180.0 * size, secs, pan, 0.22, muffle, 0.0);
                self.burst(GrainKind::Grit, 20.0 * size, secs, pan, 0.25, muffle, delay);
                true
            }
            Verb::Soil | Verb::Dig | Verb::Erase => {
                self.burst(GrainKind::Grit, 30.0 * size, 0.25, pan, 0.3, muffle, 0.0);
                true
            }
            Verb::Food => {
                self.out.push(SoundCmd::Grain { kind: GrainKind::Thunk, pan, gain: 0.7, muffle, delay: if running { delay } else { 0.0 } });
                true
            }
            Verb::Fling => {
                self.out.push(SoundCmd::Grain { kind: GrainKind::Whoosh, pan, gain: 0.4, muffle: 0.0, delay: 0.0 });
                if running {
                    // The animal's landing, timed from the hop's typical
                    // flight -- an approximation until stage 2 hears the
                    // landing itself (`CreatureStats::falls`).
                    self.out.push(SoundCmd::Grain { kind: GrainKind::Knock, pan, gain: 0.35, muffle, delay: (0.5 / speed).min(1.5) });
                }
                true
            }
            Verb::Fire => {
                self.burst(GrainKind::Crackle, 40.0, 1.5, pan, 0.3, muffle, 0.0);
                true
            }
            Verb::Water => false,
        };
        if emitted {
            self.since_verb[slot] = 0.0;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn burst(&mut self, kind: GrainKind, rate: f32, secs: f32, pan: f32, gain: f32, muffle: f32, delay: f32) {
        self.out.push(SoundCmd::Burst { kind, rate, secs, pan, spread: 0.25, gain, muffle, delay });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_rises_with_the_count_but_far_slower_than_it() {
        let (r10, l10) = density(10.0, STEPS_PER_ANT, 50.0, 0.35);
        let (r1000, l1000) = density(1000.0, STEPS_PER_ANT, 50.0, 0.35);
        assert!(r1000 <= super::super::synth::TEXTURE_RATE_CAP);
        let amp = |r: f32, l: f32| l * (r / 50.0).sqrt();
        let db = 20.0 * (amp(r1000, l1000) / amp(r10, l10)).log10();
        assert!(db > 10.0 && db < 20.0, "a hundredfold colony is {db:.1} dB louder");
        assert_eq!(density(0.0, 6.0, 50.0, 0.35), (0.0, 0.0));
    }

    #[test]
    fn the_mode_follows_delivered_ticks() {
        assert_eq!(Mode::of(0), Mode::Hush);
        assert_eq!(Mode::of(1), Mode::RealTime);
        assert_eq!(Mode::of(REAL_TIME_BELOW), Mode::TimeLapse);
        assert_eq!(Mode::of(OVERVIEW_FROM), Mode::Overview);
    }

    #[test]
    fn the_view_weights_and_pans() {
        let v = View { x0: 100, y0: 0, w: 200, h: 100, surface_y: 50 };
        assert_eq!(v.weight_of((150, 10)), 1.0);
        assert_eq!(v.weight_of((50, 10)), OFFSCREEN_WEIGHT);
        assert_eq!(v.weight_of((900, 10)), 0.0);
        assert!(v.pan_of(100) < -0.95 && v.pan_of(299) > 0.95);
        assert_eq!(v.muffle_of(10), 0.0);
        assert!(v.muffle_of(50 + 48) > 0.8);
    }

    #[test]
    fn a_light_alert_is_dropped_and_a_crowded_one_held() {
        let mut l = Listener::new();
        l.alert(AlertKind::LineEnded, 0.01, 0.0);
        assert_eq!(l.stats.alerts_too_light, 1);
        l.alert(AlertKind::LineEnded, 0.2, 0.0);
        l.alert(AlertKind::LineEnded, 0.2, 0.0);
        assert_eq!((l.stats.alerts_sent, l.stats.alerts_held), (1, 1));
        l.alert(AlertKind::LineEnded, 0.9, 0.0);
        assert_eq!(l.stats.alerts_sent, 2, "a heavy alert plays through the spacing");
    }
}
