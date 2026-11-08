//! **Seep** (probe 4 of the 2026-11-07 new-game brainstorm):
//! `cargo run --release --bin seep`.
//!
//! A mine leaks gas toward the miners' lamps. You blow it and spark it, and
//! you grow a slime mould that eats it -- but a mould that eats a pocket too
//! thick to burn thins it until it *can* burn. The design is review-3 §4a in
//! `/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/`; the
//! level is its "Two Lamps", reworked after the 2026-11-08 playtest found the
//! mould was decoration: lamp B now hangs in a dripping stall whose seep no
//! spark can clear, so the mould has a job (see `level.rs` and the report's
//! Round 2, `Reports/seep-probe-2026-11-07.md`).
//!
//! **The question this probe exists to answer** is whether the mould is
//! something a player *uses* or decoration beside the gas. So the harness
//! (`examples/seepprobe.rs`) counts every Damp and Cut by the time it was
//! made, and the stop rule is "nobody Damps or Cuts after the first minute".
//!
//! Layers, and who drives each (review-3's rule: in any level the player
//! *pushes* one layer and *grows* one, and the rest only answer):
//! - the air (`gas.rs`) -- pushed, by Blow;
//! - the mould (`mould.rs`) -- grown, steered by Damp and Cut;
//! - fire and the shipped engine (water, rock, explosions) -- they answer.
//!
//! **Engine use is read-and-write through `World`'s public cell API only.**
//! No `src/sim/` file is touched: rock, water and the explosion handoff are
//! the shipped ones, and this game's gas, mould and flame live beside the
//! world in their own grids, the same way the lab's trail field lives beside
//! its cells.

pub mod draw;
pub mod gas;
pub mod level;
pub mod mould;
pub mod script;

use crate::sim::cell::Cell;
use crate::sim::chunk::Rect;
use crate::sim::explosion::Blasts;
use crate::sim::material::{self, MaterialKind};
use crate::sim::particle::ParticleSystem;
use crate::sim::world::World;
use gas::{band, idx, Air, Band, H, W};
use mould::{Ground, Mould, Rng};

pub const TICKS_PER_SECOND: u32 = 60;
pub const LEVEL_TICKS: u32 = 4 * 60 * TICKS_PER_SECOND;
/// Carts a working face hauls per second: two faces for four minutes is 96.
pub(crate) const CARTS_PER_SECOND: f32 = 1.0 / 5.0;
/// **Stars are carts** (2026-11-08). The first version scored stars by
/// faces surviving plus every crack capped, against a "30 carts" goal that
/// doing nothing beat (53 carts on seeds 1-2), so the number on screen meant
/// nothing and a choked face cost nothing. Now a face that is choked,
/// singed or lost stops hauling, and the stars are hauled carts.
pub const STAR_CARTS: [u32; 3] = [40, 65, 85];
/// Seconds before crack 2 opens that the rock starts to hiss there.
pub const HISS_LEAD: u32 = 30 * TICKS_PER_SECOND;
/// A burning tube this close to a lamp sets off the fuse alarm.
const FUSE_ALARM_RANGE: i32 = 32;
/// The stall seep's leak per tick, against crack 1's 0.22. Swept on 3
/// paired seeds (2026-11-08) on the *first* stall (seep in the floor under
/// it, x 101), for the plan (mould led there by a damp road), the same plan
/// with the mould left alone, and nothing. Kept for the dripping stall,
/// where 5 paired seeds give led 90 carts, unled 71-72, nothing 28-29:
/// - 0.03: the unled mould's mine still hauls 75-96 -- the stall barely bites;
/// - **0.05**: led 96/96/96 with lamp B never choked, unled 58-60 (lamp B
///   choked ~3 minutes; the mould finds the stall alone only at ~111 s);
/// - 0.07: led 90-91, unled 52-53;
/// - 0.10: even led loses 35-43 carts -- the pocket outlasts the cap.
const STALL_RATE: f32 = 0.05;
/// Mean gas round a safety lamp at which its face stops work: mid-pink.
const SAFETY_CHOKE: f32 = 0.08;
/// A singe darkens a face for this long.
const SINGE_TICKS: u32 = 10 * TICKS_PER_SECOND;
/// Gas burnt within `BURN_WINDOW` ticks round a lamp above which the face is
/// blown for good rather than singed.
const BLAST_MASS: f32 = 30.0;
/// ... and above which the burn hands over to the shipped explosion.
const EXPLODE_MASS: f32 = 70.0;
const BURN_WINDOW: usize = 45;
/// Ticks a face touched by flame stays at risk of the burn growing.
const BLAST_WATCH: u32 = 90;
/// Ticks a cell burns once lit, and how long before it can light again.
const BURN_TICKS: u8 = 7;
const REFRACTORY: u8 = 45;
/// Ticks for a flame to run one cell along a dry tube: about 10 cells a
/// second, against about 25 through burnable gas.
const FUSE_STEP: u32 = 6;
/// The blow's reach and push per tick: see `Game::blow`. Was 6 and 0.035.
const BLOW_RADIUS: f32 = 8.0;
const BLOW_PUSH: f32 = 0.055;
/// Ticks a wind streak lives.
pub const STREAK_LIFE: u8 = 36;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Blow,
    Spark,
    Damp,
    Cut,
    Fuse,
}

impl Tool {
    pub const ALL: [Tool; 5] = [Tool::Blow, Tool::Spark, Tool::Damp, Tool::Cut, Tool::Fuse];
    pub fn name(self) -> &'static str {
        match self {
            Tool::Blow => "BLOW",
            Tool::Spark => "SPARK",
            Tool::Damp => "DAMP",
            Tool::Cut => "CUT",
            Tool::Fuse => "FUSE",
        }
    }
    /// What the next press does, in the screen's words.
    pub fn hint(self) -> &'static str {
        match self {
            Tool::Blow => "DRAG: A GUST PUSHES THE GAS",
            Tool::Spark => "CLICK: LIGHT IT. ONLY PINK BURNS",
            Tool::Damp => "DRAG: WET ROCK. MOULD RACES ON IT",
            Tool::Cut => "DRAG: SNIP A DRY TUBE AHEAD OF FIRE",
            Tool::Fuse => "CLICK: LIGHTS WHEN GAS HERE CAN BURN",
        }
    }
    fn meter(self) -> Option<usize> {
        match self {
            Tool::Blow => Some(0),
            Tool::Spark => Some(1),
            Tool::Damp => Some(2),
            Tool::Cut => Some(3),
            Tool::Fuse => None,
        }
    }
}

/// One thing a player does this tick. The window and the scripted players
/// both speak only this.
#[derive(Clone, Copy, Debug)]
pub enum Act {
    Blow { x: f32, y: f32, dx: f32, dy: f32 },
    Spark { x: i32, y: i32 },
    Damp { x: i32, y: i32 },
    Cut { x: i32, y: i32 },
    Fuse { x: i32, y: i32 },
}

impl Act {
    pub fn tool(&self) -> Tool {
        match self {
            Act::Blow { .. } => Tool::Blow,
            Act::Spark { .. } => Tool::Spark,
            Act::Damp { .. } => Tool::Damp,
            Act::Cut { .. } => Tool::Cut,
            Act::Fuse { .. } => Tool::Fuse,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LampState {
    Working,
    /// Gas too thick at the face to work it.
    Choked,
    Singed,
    Blown,
}

pub struct Lamp {
    pub name: &'static str,
    pub x: i32,
    pub y: i32,
    pub state: LampState,
    pub dark_until: u32,
    pub carts: f32,
    pub worked_ticks: u32,
    pub singes: u32,
    pub blown_at: Option<u32>,
    /// When flame first reached this face in the current burn.
    pub burn_from: Option<u32>,
    /// **A safety lamp** (a gauze lamp, the miners' answer to firedamp)
    /// never lights the gas itself, but the face stops work once the gas
    /// at it is `SAFETY_CHOKE` -- burnable, not only too thick -- and a
    /// flame that reaches it still singes or blows it. Lamp B in the stall
    /// is one (2026-11-08): a naked lamp at the top of the stall lit the
    /// seep's plume the moment it arrived and was lost at 8 s, before
    /// anything could be done; a safety lamp turns the same pocket into a
    /// standing problem a spark cannot clear, because the flame reaches it.
    pub safety: bool,
    /// Ticks choked in a row, for "digging again" and the coach.
    pub choked_run: u32,
}

pub struct Crack {
    pub x0: i32,
    pub x1: i32,
    pub y: i32,
    pub opens_at: u32,
    pub rate: f32,
    pub emitted: f64,
    /// Gas the mould took at the mouth.
    pub capped: f64,
    /// Smoothed fraction capped, for "every crack capped".
    pub cap_ema: f32,
}

/// A wisp of moving air, drawn as a short pale line.
pub struct Streak {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: u8,
}

pub struct FusePlaced {
    pub x: i32,
    pub y: i32,
    pub lit: bool,
}

/// A word that floats up from where something happened, so the rule is
/// taught by the event ("TOO THICK", "SINGED") rather than by a manual.
pub struct Floater {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub until: u32,
    pub colour: [u8; 3],
}

#[derive(Default, Clone, Debug)]
pub struct Counts {
    pub emitted: f64,
    pub burnt: f64,
    pub sparks: u32,
    pub fizzles: u32,
    pub singes: u32,
    pub lamps_blown: u32,
    pub explosions: u32,
    pub fuses_lit: u32,
    /// Every use of each tool, as the tick it happened on -- the stop rule
    /// reads Damp and Cut after the first minute from these.
    pub uses: [Vec<u32>; 5],
    pub cuts_hit: u32,
    /// Cuts that landed on the core's own tubes and did nothing.
    pub cuts_guarded: u32,
    /// Times a burning dry tube came within `FUSE_ALARM_RANGE` of a lamp.
    pub fuse_alarms: u32,
    /// Ticks each lamp spent choked by thick gas.
    pub choked_ticks: [u32; 2],
    pub frames: u32,
}

pub struct Game {
    pub world: World,
    pub particles: ParticleSystem,
    pub blasts: Blasts,
    pub air: Air,
    pub mould: Mould,
    pub solid: Vec<bool>,
    pub water: Vec<bool>,
    /// Damp per cell. `wet_floor` is the level's own permanent damp (the
    /// sump, the drip line); the player's Damp decays back toward it.
    pub wet: Vec<f32>,
    wet_floor: Vec<f32>,
    pub fire: Vec<u8>,
    refractory: Vec<u8>,
    /// Tube cells waiting for a fuse to reach them: (tick, cell).
    fuse_queue: Vec<(u32, usize)>,
    /// Burning as part of a fuse: its flame takes the next dry tube for
    /// certain, not on a coin toss.
    fuse_cell: Vec<bool>,
    pub lamps: Vec<Lamp>,
    pub cracks: Vec<Crack>,
    pub fuses: Vec<FusePlaced>,
    pub fuses_left: u32,
    pub meters: [f32; 4],
    pub tick: u32,
    pub floaters: Vec<Floater>,
    pub streaks: Vec<Streak>,
    pub banner: Option<(String, u32)>,
    pub counts: Counts,
    burn_ring: [f32; BURN_WINDOW],
    rng: Rng,
    last_explosion: u32,
    drip: (i32, i32),
    told_thinning: bool,
    told_capped: bool,
    told_fuse: bool,
    told_cleared: bool,
    /// Coach tips already given, by `Coach` index.
    coached: [bool; 4],
    stars_shown: u32,
    /// Banners waiting their turn, with how many seconds each shows.
    queue: Vec<(String, u32)>,
    /// Rings pulsing round what a banner is talking about: (x, y, until).
    pub pointers: Vec<(i32, i32, u32)>,
    /// The tick the last star was won, for the bar's flash.
    pub star_won_at: Option<u32>,
    fuse_alarm_until: u32,
    pub seed: u64,
    /// False only for the probe's `nomould` arm: Firedamp alone, the mine
    /// with no mould in it at all.
    pub mould_on: bool,
}

fn ground<'a>(solid: &'a [bool], water: &'a [bool], wet: &'a [f32], floor: &'a [f32]) -> Ground<'a> {
    Ground { solid, water, wet, floor }
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut world = World::new(Rect::new(0, 0, W as i32 - 1, H as i32 - 1));
        let mut wet_floor = vec![0.0; W * H];
        build_two_lamps(&mut world, &mut wet_floor, seed);
        let mut g = Game {
            world,
            particles: ParticleSystem::new(),
            blasts: Blasts::new(),
            air: Air::new(),
            mould: Mould::new(32, 141, 70.0, seed),
            solid: vec![false; W * H],
            water: vec![false; W * H],
            wet: wet_floor.clone(),
            wet_floor,
            fire: vec![0; W * H],
            refractory: vec![0; W * H],
            fuse_queue: Vec::new(),
            fuse_cell: vec![false; W * H],
            lamps: vec![
                Lamp { name: "A", x: 234, y: 46, state: LampState::Working, dark_until: 0, carts: 0.0, worked_ticks: 0, singes: 0, blown_at: None, burn_from: None, safety: false, choked_run: 0 },
                Lamp { name: "B", x: level::STALL_X, y: 100, state: LampState::Working, dark_until: 0, carts: 0.0, worked_ticks: 0, singes: 0, blown_at: None, burn_from: None, safety: true, choked_run: 0 },
            ],
            cracks: vec![
                Crack { x0: 178, x1: 181, y: 139, opens_at: 0, rate: 0.22, emitted: 0.0, capped: 0.0, cap_ema: 0.0 },
                Crack { x0: 119, x1: 122, y: 63, opens_at: 90 * TICKS_PER_SECOND, rate: 0.14, emitted: 0.0, capped: 0.0, cap_ema: 0.0 },
                // The stall's seep, in its wet roof beside lamp B.
                Crack { x0: level::STALL_X + 5, x1: level::STALL_X + 7, y: 100, opens_at: 0, rate: STALL_RATE, emitted: 0.0, capped: 0.0, cap_ema: 0.0 },
            ],
            fuses: Vec::new(),
            fuses_left: 2,
            meters: [1.0; 4],
            tick: 0,
            floaters: Vec::new(),
            streaks: Vec::new(),
            banner: Some(("KEEP GAS OFF THE LAMPS".into(), 6 * TICKS_PER_SECOND)),
            counts: Counts::default(),
            burn_ring: [0.0; BURN_WINDOW],
            rng: Rng(seed.wrapping_mul(0xD1B5_4A32_D192_ED03) | 1),
            last_explosion: 0,
            drip: (149, 67),
            told_thinning: false,
            told_capped: false,
            told_fuse: false,
            told_cleared: false,
            coached: [false; 4],
            stars_shown: 0,
            queue: Vec::new(),
            pointers: Vec::new(),
            star_won_at: None,
            fuse_alarm_until: 0,
            seed,
            mould_on: true,
        };
        g.refresh_masks(true);
        g.air.sky_row = 36;
        // The stall seep sits in the stall's roof: the top open cell there.
        let sx = level::STALL_X + 6;
        if let Some(y) = (80..120).find(|&y| g.air.is_open(sx, y)) {
            g.cracks[2].y = y;
        }
        g
    }

    pub fn over(&self) -> bool {
        self.tick >= LEVEL_TICKS || self.lamps.iter().all(|l| l.state == LampState::Blown)
    }

    pub fn carts(&self) -> u32 {
        self.lamps.iter().map(|l| l.carts).sum::<f32>() as u32
    }

    pub fn face_seconds(&self) -> f32 {
        self.lamps.iter().map(|l| l.worked_ticks as f32).sum::<f32>() / TICKS_PER_SECOND as f32
    }

    pub fn cracks_open(&self) -> usize {
        self.cracks.iter().filter(|c| self.tick >= c.opens_at).count()
    }

    pub fn cracks_capped(&self) -> usize {
        self.cracks.iter().filter(|c| self.tick >= c.opens_at && c.cap_ema > 0.6).count()
    }

    /// One star for each of `STAR_CARTS` hauled.
    pub fn stars(&self) -> u32 {
        let c = self.carts();
        STAR_CARTS.iter().filter(|&&k| c >= k).count() as u32
    }

    pub fn meter(&self, t: Tool) -> f32 {
        match t.meter() {
            Some(k) => self.meters[k],
            None => self.fuses_left as f32 / 2.0,
        }
    }

    fn say(&mut self, text: &str, x: i32, y: i32, colour: [u8; 3]) {
        // One word at a place at a time, or a held Spark stacks them.
        if self.floaters.iter().any(|f| f.text == text && (f.x - x).abs() < 12 && (f.y - y).abs() < 12) {
            return;
        }
        self.floaters.push(Floater { text: text.into(), x, y, until: self.tick + 90, colour });
    }

    fn refresh_masks(&mut self, force: bool) {
        let mut changed = force;
        for y in 0..H {
            for x in 0..W {
                let i = idx(x, y);
                let c = self.world.get(x as i32, y as i32);
                let kind = self.world.materials.kind(c.material);
                let s = matches!(kind, MaterialKind::Solid | MaterialKind::Powder);
                let w = kind == MaterialKind::Liquid;
                if s != self.solid[i] || w != self.water[i] {
                    self.solid[i] = s;
                    self.water[i] = w;
                    changed = true;
                }
            }
        }
        if changed {
            let open: Vec<bool> = (0..W * H).map(|i| !self.solid[i] && !self.water[i]).collect();
            self.air.set_open(open);
            // A tube whose cells are now under water or rubble is gone.
            for k in 0..self.mould.edges.len() {
                if self.mould.edges[k].alive && self.mould.edges[k].cells.iter().any(|&(x, y)| self.solid[idx(x as usize, y as usize)]) {
                    self.mould.remove_edge(k);
                }
            }
        }
    }

    /// Apply one act. Returns false when the tool's meter was dry, so the
    /// window can flash it.
    pub fn act(&mut self, a: Act) -> bool {
        let t = a.tool();
        let cost = match a {
            Act::Blow { .. } => 0.5 / TICKS_PER_SECOND as f32,
            Act::Damp { .. } => 0.55 / TICKS_PER_SECOND as f32,
            Act::Spark { .. } => 0.25,
            Act::Cut { .. } => 0.08,
            Act::Fuse { .. } => 0.0,
        };
        if let Some(k) = t.meter() {
            if self.meters[k] < cost {
                return false;
            }
            self.meters[k] -= cost;
        } else if self.fuses_left == 0 {
            return false;
        }
        self.counts.uses[t as usize].push(self.tick);
        match a {
            Act::Blow { x, y, dx, dy } => self.blow(x, y, dx, dy),
            Act::Spark { x, y } => self.spark(x, y),
            Act::Damp { x, y } => self.damp(x, y),
            Act::Cut { x, y } => {
                let n = self.mould.cut(x, y, 2);
                self.counts.cuts_hit += n as u32;
                if n > 0 {
                    self.say("SNIP", x, y - 4, [240, 220, 120]);
                } else if self.mould.guarded(x, y) && self.mould.tube_at(x, y).is_some() {
                    self.counts.cuts_guarded += 1;
                    self.say("TOO TOUGH", x, y - 4, [240, 220, 120]);
                }
            }
            Act::Fuse { x, y } => {
                if self.air.is_open(x, y) {
                    self.fuses_left -= 1;
                    self.fuses.push(FusePlaced { x, y, lit: false });
                    self.say("FUSE SET", x, y - 5, [255, 170, 80]);
                } else {
                    self.counts.uses[t as usize].pop();
                }
            }
        }
        true
    }

    /// **A gust you can see** (owner playtest 2026-11-08: "blowing is not
    /// very satisfying"). The push used to be a radius-6 nudge centred on
    /// the cursor, and moving air was invisible: you saw the gas drift a
    /// little, if there was gas. Now the push is wider, harder and sits a
    /// little ahead of the hand, like a bellows, and every stroke throws
    /// out wind streaks that ride the air itself, so the gust is seen
    /// whether or not there is gas in it.
    fn blow(&mut self, x: f32, y: f32, dx: f32, dy: f32) {
        let l = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (nx, ny) = (dx / l, dy / l);
        self.air.blow(x + 2.0 * nx, y + 2.0 * ny, dx, dy, BLOW_RADIUS, BLOW_PUSH);
        // Placed by a hash of the tick, not the game's generator: streaks
        // are drawn only, and must not shift the dice the gas and the mould
        // roll, or a paired comparison would differ by more than the push.
        for k in 0..3u64 {
            let mut h = Rng((self.tick as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (k + 1).wrapping_mul(0xD1B5_4A32_D192_ED03) | 1);
            let a = h.f() * std::f32::consts::TAU;
            let r = h.f() * BLOW_RADIUS;
            let (sx, sy) = (x + r * a.cos(), y + r * a.sin());
            if self.air.is_open(sx as i32, sy as i32) && self.streaks.len() < 400 {
                self.streaks.push(Streak { x: sx, y: sy, vx: nx * 0.3, vy: ny * 0.3, life: STREAK_LIFE });
            }
        }
    }

    /// Wind streaks ride the air: see `blow`.
    fn move_streaks(&mut self) {
        let air = &self.air;
        self.streaks.retain_mut(|s| {
            let (cx, cy) = (s.x as i32, s.y as i32);
            if s.life == 0 || !air.is_open(cx, cy) {
                return false;
            }
            let (ux, uy) = (cx as usize, cy as usize);
            let u = 0.5 * (air.u[uy * (W + 1) + ux] + air.u[uy * (W + 1) + ux + 1]);
            let v = 0.5 * (air.v[uy * W + ux] + air.v[(uy + 1) * W + ux]);
            // A streak keeps a little of its own throw, so a stroke into
            // still air still reads as a gust.
            s.vx = 0.8 * s.vx + 0.2 * u * 1.6;
            s.vy = 0.8 * s.vy + 0.2 * v * 1.6;
            s.x += s.vx;
            s.y += s.vy;
            s.life -= 1;
            true
        });
    }

    fn spark(&mut self, x: i32, y: i32) {
        self.counts.sparks += 1;
        // A spark lights the burnable cell nearest the click within 2.
        let mut lit = false;
        let mut seen = Band::Clear;
        for r in 0..=2 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let (cx, cy) = (x + dx, y + dy);
                    if !self.air.is_open(cx, cy) {
                        continue;
                    }
                    let b = band(self.air.at(cx, cy));
                    if b as u8 > seen as u8 {
                        seen = b;
                    }
                    if !lit && self.ignitable(cx, cy) {
                        self.fire[idx(cx as usize, cy as usize)] = BURN_TICKS;
                        lit = true;
                    }
                }
            }
            if lit {
                break;
            }
        }
        self.spark_puff(x, y);
        if !lit {
            self.counts.fizzles += 1;
            let word = match seen {
                Band::Clear => "NO GAS HERE",
                Band::Thin => "TOO THIN",
                Band::Thick => "TOO THICK",
                Band::Burns => "TOO WET",
            };
            self.say(word, x, y - 6, [200, 200, 230]);
        }
    }

    fn spark_puff(&mut self, x: i32, y: i32) {
        let flame = self.world.materials.id_of("flame");
        if let Some(f) = flame {
            for _ in 0..6 {
                let vx = (self.rng.f() - 0.5) * 1.6;
                let vy = -self.rng.f() * 1.4;
                self.particles.spawn(x as f32, y as f32, vx, vy, f, 0);
            }
        }
    }

    fn damp(&mut self, x: i32, y: i32) {
        self.mould.lure_at(x, y, self.tick);
        let r = 4;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r {
                    continue;
                }
                let (cx, cy) = (x + dx, y + dy);
                if cx < 0 || cy < 0 || cx as usize >= W || cy as usize >= H {
                    continue;
                }
                let i = idx(cx as usize, cy as usize);
                self.wet[i] = 1.0;
            }
        }
        // A little real water runs off the brush: the shipped liquid, so it
        // drips, pools and blocks the gas where it lies.
        if self.tick.is_multiple_of(10) {
            let wx = x + (self.rng.f() * 5.0) as i32 - 2;
            if self.air.is_open(wx, y) && self.water_cells() < 900 {
                self.world.set(wx, y, Cell::new(material::WATER, 0));
            }
        }
    }

    fn water_cells(&self) -> usize {
        self.water.iter().filter(|&&w| w).count()
    }

    fn ignitable(&self, x: i32, y: i32) -> bool {
        if !self.air.is_open(x, y) {
            return false;
        }
        let i = idx(x as usize, y as usize);
        if self.fire[i] > 0 || self.refractory[i] > 0 {
            return false;
        }
        let g = ground(&self.solid, &self.water, &self.wet, &self.wet_floor);
        if let Some(e) = self.mould.tube_at(x, y) {
            // A dry tube burns whatever the gas; a wet one never does.
            return !self.mould.is_wet(e, &g);
        }
        if g.damp(x, y) > 0.5 {
            return false;
        }
        band(self.air.c[i]) == Band::Burns
    }

    /// One tick of everything.
    pub fn step(&mut self, acts: &[Act]) {
        for &a in acts {
            self.act(a);
        }
        // Meters refill.
        let refill = [0.22, 0.16, 0.12, 0.25];
        for (m, r) in self.meters.iter_mut().zip(refill) {
            *m = (*m + r / TICKS_PER_SECOND as f32).min(1.0);
        }
        // The shipped world: water drips and pools, a blast runs its course.
        if self.tick.is_multiple_of(70) && self.water_cells() < 900 {
            let (dx, dy) = self.drip;
            if self.world.get(dx, dy).material == material::EMPTY {
                self.world.set(dx, dy, Cell::new(material::WATER, 0));
            }
        }
        crate::sim::update::step(&mut self.world);
        self.blasts.step(&mut self.world, &mut self.particles);
        self.particles.step(&mut self.world);
        if self.tick.is_multiple_of(6) || !self.blasts.is_empty() {
            self.refresh_masks(false);
        }
        // Damp dries back toward the rock's own.
        for i in 0..W * H {
            if self.wet[i] > self.wet_floor[i] {
                self.wet[i] = (self.wet[i] - 0.00035).max(self.wet_floor[i]);
            }
        }
        self.leak();
        if self.mould_on {
            self.mould.eat(&mut self.air);
            let g = ground(&self.solid, &self.water, &self.wet, &self.wet_floor);
            self.mould.step(self.tick, &self.air, &g);
        }
        self.burn();
        self.air.step();
        self.move_streaks();
        self.fuses_and_lamps();
        self.floaters.retain(|f| f.until > self.tick);
        self.tell();
        self.tick += 1;
        self.counts.frames = self.tick;
    }

    fn leak(&mut self) {
        let t = self.tick as f32 / TICKS_PER_SECOND as f32;
        for k in 0..self.cracks.len() {
            let (opens_at, rate, cx0, cx1, cy) = {
                let c = &self.cracks[k];
                (c.opens_at, c.rate, c.x0, c.x1, c.y)
            };
            if self.tick < opens_at {
                // **A crack about to open hisses first** (playtest: "show
                // crack 2 before it opens", so leading the mould there is a
                // plan the player can see coming).
                let lead = opens_at.saturating_sub(self.tick);
                if lead <= HISS_LEAD && lead.is_multiple_of(3 * TICKS_PER_SECOND) {
                    let (x, y) = ((cx0 + cx1) / 2, cy);
                    self.say("HISS", x, y - 6, [230, 150, 190]);
                    if lead == HISS_LEAD {
                        self.urgent("THE ROCK IS HISSING. A CRACK WILL OPEN".into(), 5);
                    }
                }
                continue;
            }
            if self.tick == opens_at && opens_at > 0 {
                let (x, y) = ((cx0 + cx1) / 2, cy);
                self.say("A NEW CRACK!", x, y - 8, [255, 120, 160]);
                self.urgent("A NEW CRACK HAS OPENED".into(), 4);
            }
            // The crack breathes: a slow pulse and a little noise.
            let breath = 1.0 + 0.45 * (t * std::f32::consts::TAU / 5.0 + k as f32).sin();
            let total = rate * breath * (0.8 + 0.4 * self.rng.f());
            let cap = if self.mould_on { self.mould.cap_fraction(cx0, cx1, cy) } else { 0.0 };
            let to_mould = total * cap;
            let to_air = total - to_mould;
            let (x0, x1, y) = (cx0, cx1, cy);
            let n = (x1 - x0 + 1) as f32;
            // The leak comes up through whatever lies on the crack: if water
            // has pooled over it, the gas enters the first air above.
            for x in x0..=x1 {
                if let Some(yy) = (y - 12..=y).rev().find(|&yy| self.air.is_open(x, yy)) {
                    self.air.c[idx(x as usize, yy as usize)] += to_air / n;
                } else {
                    self.air.buried += (to_air / n) as f64;
                }
            }
            self.mould.credit_cap(x0, x1, y, to_mould);
            let c = &mut self.cracks[k];
            c.emitted += total as f64;
            c.capped += to_mould as f64;
            c.cap_ema = 0.995 * c.cap_ema + 0.005 * cap;
            self.counts.emitted += total as f64;
        }
    }

    fn burn(&mut self) {
        let mut burnt_now = 0.0;
        let mut lit = Vec::new();
        // A fuse reaches its next cell.
        let now = self.tick;
        let mut k = 0;
        while k < self.fuse_queue.len() {
            if self.fuse_queue[k].0 <= now {
                let (_, i) = self.fuse_queue.swap_remove(k);
                if self.fire[i] == 0 && self.refractory[i] == 0 && !self.solid[i] {
                    self.fire[i] = BURN_TICKS;
                    self.fuse_cell[i] = true;
                }
            } else {
                k += 1;
            }
        }
        let g = ground(&self.solid, &self.water, &self.wet, &self.wet_floor);
        let mut tubes_hit: Vec<(usize, usize)> = Vec::new();
        for y in 0..H {
            for x in 0..W {
                let i = idx(x, y);
                if self.refractory[i] > 0 {
                    self.refractory[i] -= 1;
                }
                if self.fire[i] == 0 {
                    continue;
                }
                let take = self.air.c[i].min(0.14);
                self.air.c[i] -= take;
                burnt_now += take;
                self.air.heat[i] += 0.35;
                self.air.expand[i] += take * 1.6 + 0.01;
                self.fire[i] -= 1;
                let fuse = self.fuse_cell[i];
                if self.fire[i] == 0 {
                    self.refractory[i] = REFRACTORY;
                    self.fuse_cell[i] = false;
                }
                if let Some(e) = self.mould.tube_at(x as i32, y as i32) {
                    tubes_hit.push((e, i));
                }
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    lit.push((nx, ny, dx != 0 && dy != 0, fuse));
                }
            }
        }
        let _ = &g;
        for (nx, ny, diag, fuse) in lit {
            if !self.ignitable(nx, ny) {
                continue;
            }
            let tube = self.mould.tube_at(nx, ny).is_some();
            // Through gas a flame runs at about 25 cells a second. A fuse
            // takes the next dry tube it touches for certain; any other
            // flame catches a dry tube on a coin toss.
            let p = match (tube, diag) {
                (true, _) if fuse => 1.0,
                (true, _) => 0.09,
                (false, false) => 0.42,
                (false, true) => 0.12,
            };
            if self.rng.f() < p {
                self.fire[idx(nx as usize, ny as usize)] = BURN_TICKS;
            }
        }
        tubes_hit.sort_unstable();
        tubes_hit.dedup_by_key(|h| h.0);
        // **The fuse alarm.** A dry tube burning toward a lit lamp is the
        // job Cut was missing (playtest: "Cut has no good use"): say so where
        // the fire is, so a snip ahead of it is a rescue.
        if self.tick >= self.fuse_alarm_until {
            let burning: Option<(i32, i32, usize)> = tubes_hit.iter().find_map(|&(e, _)| {
                let (fx, fy) = self.mould.edges[e].cells[0];
                self.lamps
                    .iter()
                    .enumerate()
                    .find(|(_, l)| l.state != LampState::Blown && (l.x - fx).abs() + (l.y - fy).abs() <= FUSE_ALARM_RANGE)
                    .map(|(k, _)| (fx, fy, k))
            });
            if let Some((fx, fy, k)) = burning {
                self.counts.fuse_alarms += 1;
                self.fuse_alarm_until = self.tick + 3 * TICKS_PER_SECOND;
                self.say("FUSE! CUT IT", fx, fy - 6, [255, 200, 90]);
                if !self.told_fuse {
                    self.told_fuse = true;
                    let name = self.lamps[k].name;
                    self.urgent(format!("A DRY TUBE IS BURNING TO LAMP {name}. CUT IT"), 5);
                }
            }
        }
        for (e, at) in tubes_hit {
            if self.mould.edges[e].alive {
                self.mould.remove_edge(e);
                self.mould.counts.tubes_burnt += 1;
                // **The flame runs along the tube from where it caught,
                // one cell every `FUSE_STEP` ticks, and takes the next dry
                // tube at the far end.** The first version lit every cell of
                // the tube at once on a random timer, and only the end cell
                // touched the next tube: when that cell's timer was short
                // the flame died at the junction. Traced 2026-11-08 on a
                // dry 8-tube line laid at a lamp: one tube burnt, the lamp
                // untouched -- "burns like a fuse" was never true past one
                // tube.
                let cells = self.mould.edges[e].cells.clone();
                let j = cells.iter().position(|&(x, y)| idx(x as usize, y as usize) == at).unwrap_or(0);
                for (k, &(x, y)) in cells.iter().enumerate() {
                    let d = (k as i64 - j as i64).unsigned_abs() as u32;
                    if d > 0 {
                        self.fuse_queue.push((self.tick + d * FUSE_STEP, idx(x as usize, y as usize)));
                    }
                }
                self.fuse_cell[at] = true;
            }
        }
        self.counts.burnt += burnt_now as f64;
        let slot = self.tick as usize % BURN_WINDOW;
        self.burn_ring[slot] = burnt_now;
        let window: f32 = self.burn_ring.iter().sum();
        if window > EXPLODE_MASS && self.tick > self.last_explosion + 5 * TICKS_PER_SECOND {
            // A big pocket going up hands over to the shipped explosion,
            // centred where the most is burning.
            let (mut sx, mut sy, mut n) = (0.0, 0.0, 0.0);
            for y in 0..H {
                for x in 0..W {
                    if self.fire[idx(x, y)] > 0 {
                        sx += x as f32;
                        sy += y as f32;
                        n += 1.0;
                    }
                }
            }
            if n > 0.0 {
                let (cx, cy) = ((sx / n) as i32, (sy / n) as i32);
                self.blasts.trigger_with(&mut self.world, &mut self.particles, cx, cy, 9, 90.0);
                self.counts.explosions += 1;
                self.last_explosion = self.tick;
                self.say("BOOM", cx, cy - 8, [255, 200, 90]);
            }
        }
    }

    fn fuses_and_lamps(&mut self) {
        for k in 0..self.fuses.len() {
            if self.fuses[k].lit {
                continue;
            }
            let (x, y) = (self.fuses[k].x, self.fuses[k].y);
            if self.ignitable(x, y) || [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| self.ignitable(x + dx, y + dy)) {
                self.fuses[k].lit = true;
                self.counts.fuses_lit += 1;
                for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                    if self.ignitable(x + dx, y + dy) {
                        self.fire[idx((x + dx) as usize, (y + dy) as usize)] = BURN_TICKS;
                    }
                }
                self.say("FUSE!", x, y - 6, [255, 170, 80]);
            }
        }
        self.fuses.retain(|f| !f.lit);
        let window: f32 = self.burn_ring.iter().sum();
        for k in 0..self.lamps.len() {
            let (x, y) = (self.lamps[k].x, self.lamps[k].y);
            if self.lamps[k].state == LampState::Blown {
                continue;
            }
            // A naked lamp lights any burnable gas that reaches its flame.
            if !self.lamps[k].safety && matches!(self.lamps[k].state, LampState::Working | LampState::Choked) {
                for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1), (0, -2), (2, 0), (-2, 0)] {
                    if self.ignitable(x + dx, y + dy) {
                        self.fire[idx((x + dx) as usize, (y + dy) as usize)] = BURN_TICKS;
                    }
                }
            }
            let mut fire_near = false;
            for dy in -5..=5 {
                for dx in -5..=5 {
                    let (cx, cy) = (x + dx, y + dy);
                    if self.air.is_open(cx, cy) && self.fire[idx(cx as usize, cy as usize)] > 0 {
                        fire_near = true;
                    }
                }
            }
            // **A burn is judged by how big it gets, not how big it was when
            // it reached the face.** The first version judged at the moment
            // of contact, and the first headless run (2026-11-07) showed why
            // that is wrong: the lamp lights the fringe, the fringe is
            // small, the face is "singed" -- and then the flame runs down
            // the shaft into the dome and the burn becomes a blast that
            // cracks rock, with the face already marked safe. So a face
            // touched by flame stays at risk for `BLAST_WATCH` ticks, and
            // is lost if the burn it is part of passes `BLAST_MASS`.
            if fire_near && self.lamps[k].burn_from.is_none() {
                self.lamps[k].burn_from = Some(self.tick);
                self.lamps[k].state = LampState::Singed;
                self.lamps[k].dark_until = self.tick + SINGE_TICKS;
                self.lamps[k].singes += 1;
                self.counts.singes += 1;
                self.say("SINGED", x, y - 8, [255, 160, 90]);
            }
            if let Some(t0) = self.lamps[k].burn_from {
                if self.tick - t0 < BLAST_WATCH && window > BLAST_MASS {
                    self.lamps[k].state = LampState::Blown;
                    self.lamps[k].blown_at = Some(self.tick);
                    self.counts.lamps_blown += 1;
                    let name = self.lamps[k].name;
                    self.say("BLOWN!", x, y - 8, [255, 90, 70]);
                    // Advice about a lamp that is gone is not shown late.
                    let about = format!("LAMP {name}");
                    self.queue.retain(|q| !q.0.contains(&about));
                    if self.banner.as_ref().is_some_and(|b| b.0.contains(&about)) {
                        self.banner = None;
                    }
                    self.urgent(format!("LAMP {name} IS LOST"), 5);
                    continue;
                }
                if self.tick - t0 >= BLAST_WATCH && !fire_near {
                    self.lamps[k].burn_from = None;
                }
            }
            match self.lamps[k].state {
                LampState::Singed if self.tick >= self.lamps[k].dark_until => self.lamps[k].state = LampState::Working,
                LampState::Working | LampState::Choked => {
                    let limit = if self.lamps[k].safety { SAFETY_CHOKE } else { gas::THICK };
                    let thick = self.air.mean_near(x, y, 3) > limit;
                    self.lamps[k].state = if thick { LampState::Choked } else { LampState::Working };
                }
                _ => {}
            }
            if self.lamps[k].state == LampState::Choked && k < 2 {
                self.counts.choked_ticks[k] += 1;
            }
            if self.lamps[k].state == LampState::Working && self.tick < LEVEL_TICKS {
                self.lamps[k].worked_ticks += 1;
                let before = self.lamps[k].carts as u32;
                self.lamps[k].carts += CARTS_PER_SECOND / TICKS_PER_SECOND as f32;
                // **Every cart pops out of its face** (owner playtest
                // 2026-11-08: "when you're doing well it needs to be
                // clearer"). A working lamp says so every five seconds.
                if self.lamps[k].carts as u32 > before {
                    // Beside the label: below a roof lamp, above a stall lamp.
                    let py = if y < 80 { y + 22 } else { y - 13 };
                    self.say("+1 CART", x, py, [150, 235, 120]);
                }
            }
            // A choked face that clears says so, the first time loudly.
            let now_working = self.lamps[k].state == LampState::Working;
            if now_working && self.lamps[k].choked_run > 3 * TICKS_PER_SECOND {
                self.say("DIGGING AGAIN", x, if y < 80 { y + 22 } else { y - 13 }, [150, 235, 120]);
                if !self.told_cleared {
                    self.told_cleared = true;
                    let name = self.lamps[k].name;
                    self.announce(format!("LAMP {name} IS CLEAR. MINERS DIGGING AGAIN"), 5);
                }
            }
            self.lamps[k].choked_run = if self.lamps[k].state == LampState::Choked { self.lamps[k].choked_run + 1 } else { 0 };
        }
    }

    /// Show a banner now; one it displaces goes back to the head of the
    /// queue, so a coach tip is not lost under "LAMP A IS LOST".
    fn urgent(&mut self, msg: String, secs: u32) {
        if let Some((old, until)) = self.banner.take() {
            let left = until.saturating_sub(self.tick) / TICKS_PER_SECOND;
            if left >= 1 && !old.starts_with("STAR") {
                self.queue.insert(0, (old, left.max(3)));
            }
        }
        self.banner = Some((msg, self.tick + secs * TICKS_PER_SECOND));
    }

    /// Show a banner now, or after the one showing.
    fn announce(&mut self, msg: String, secs: u32) {
        if self.banner.is_none() {
            self.banner = Some((msg, self.tick + secs * TICKS_PER_SECOND));
        } else if self.queue.len() < 4 {
            self.queue.push((msg, secs));
        }
    }

    fn point_at(&mut self, x: i32, y: i32, secs: u32) {
        self.pointers.push((x, y, self.tick + secs * TICKS_PER_SECOND));
    }

    /// **The coach** (owner playtest 2026-11-08: "the instructions need to
    /// be better"). Each tip is said once, when the thing it is about first
    /// happens, with a ring round that thing: the card at the start says
    /// what the tools are, and the coach says which one the mine is asking
    /// for now. The last tip names the mould's job outright, but only after
    /// lamp B has been choked for 25 s -- a player who works the stall out
    /// first never sees it.
    fn coach(&mut self) {
        if !self.tick.is_multiple_of(15) {
            return;
        }
        // 0: the first pink pocket. Find the biggest burnable patch's middle.
        if !self.coached[0] && self.tick > 4 * TICKS_PER_SECOND {
            let (mut sx, mut sy, mut n) = (0i64, 0i64, 0i64);
            for y in 0..H {
                for x in 0..W {
                    if band(self.air.c[idx(x, y)]) == Band::Burns {
                        sx += x as i64;
                        sy += y as i64;
                        n += 1;
                    }
                }
            }
            if n > 60 {
                self.coached[0] = true;
                let (x, y) = ((sx / n) as i32, (sy / n) as i32);
                self.point_at(x, y, 6);
                self.announce("PINK GAS BURNS. PRESS 2 AND CLICK IT".into(), 6);
            }
        }
        // 1: gas thickening at a naked lamp.
        if !self.coached[1] {
            if let Some(l) = self.lamps.iter().find(|l| !l.safety && l.state == LampState::Working && self.air.mean_near(l.x, l.y, 12) > gas::TRACE) {
                let (x, y) = (l.x, l.y);
                self.coached[1] = true;
                self.point_at(x, y, 6);
                self.announce("GAS IS REACHING LAMP A. BURN OR BLOW IT".into(), 6);
            }
        }
        // 2 and 3: lamp B choked.
        let b = self.lamps.iter().position(|l| l.safety);
        if let Some(k) = b {
            let (x, y, run) = (self.lamps[k].x, self.lamps[k].y, self.lamps[k].choked_run);
            if !self.coached[2] && run > 3 * TICKS_PER_SECOND {
                self.coached[2] = true;
                self.point_at(x, y, 6);
                self.announce("LAMP B IS CHOKED. NO SPARK LIGHTS HERE".into(), 6);
            }
            if !self.coached[3] && run > 25 * TICKS_PER_SECOND {
                self.coached[3] = true;
                let core = &self.mould.nodes[self.mould.core];
                let (cx, cy) = (core.x, core.y);
                self.point_at(cx, cy, 7);
                self.point_at(x, y, 7);
                self.announce("THE MOULD EATS GAS. DAMP A PATH TO LAMP B".into(), 7);
            }
        }
    }

    /// The level talks when something it has been waiting for happens.
    fn tell(&mut self) {
        self.coach();
        let stars = self.stars();
        if stars > self.stars_shown {
            self.stars_shown = stars;
            self.star_won_at = Some(self.tick);
            let msg = match stars {
                1 => format!("STAR 1! NEXT STAR AT {} CARTS", STAR_CARTS[1]),
                2 => format!("STAR 2! NEXT STAR AT {} CARTS", STAR_CARTS[2]),
                _ => "STAR 3! THE BEST SHIFT THIS MINE HAS".to_string(),
            };
            // A star jumps the queue.
            self.urgent(msg, 4);
        }
        self.pointers.retain(|p| p.2 > self.tick);
        if !self.told_capped && self.cracks_capped() > 0 {
            self.told_capped = true;
            self.urgent("YOUR MOULD HAS CAPPED A CRACK".into(), 5);
        }
        if !self.told_thinning && self.tick.is_multiple_of(30) {
            // A tube under a thick pocket with burnable gas between them.
            let thinning = self.mould.edges.iter().filter(|e| e.alive).any(|e| {
                let (x, y) = e.cells[e.cells.len() / 2];
                let b0 = band(self.air.at(x, y));
                let up = band(self.air.at(x, y - 3));
                b0 == Band::Burns && (up == Band::Thick || up == Band::Burns) && self.air.mean_near(x, y - 4, 3) > gas::THICK * 0.9
            });
            if thinning {
                self.told_thinning = true;
                self.urgent("YOUR MOULD IS THINNING THE POCKET".into(), 6);
            }
        }
        if let Some((_, until)) = &self.banner {
            if self.tick >= *until {
                self.banner = None;
            }
        }
        if self.banner.is_none() && !self.queue.is_empty() {
            let (msg, secs) = self.queue.remove(0);
            self.banner = Some((msg, self.tick + secs * TICKS_PER_SECOND));
        }
    }
}

/// **"Two Lamps"** into the shipped world: rock, bedrock round the edge,
/// and the sump's water. The shape itself is `level.rs`, kept free of the
/// engine so it can be looked at on its own.
fn build_two_lamps(world: &mut World, wet: &mut [f32], seed: u64) {
    let lay = level::two_lamps(seed);
    let mut rng = Rng(seed.wrapping_add(0x51_7CC1_B727_220A) | 1);
    for y in 0..H {
        for x in 0..W {
            let i = idx(x, y);
            let edge = x == 0 || x == W - 1 || y == H - 1;
            let m = if lay.open[i] {
                material::EMPTY
            } else if edge {
                material::BEDROCK
            } else {
                material::STONE
            };
            let shade = (rng.next_u64() % 4) as u8;
            world.set(x as i32, y as i32, Cell::new(m, shade));
        }
    }
    for &(x, y) in &lay.water {
        world.set(x, y, Cell::new(material::WATER, 0));
    }
    wet.copy_from_slice(&lay.wet);
}

#[cfg(test)]
mod tests;
