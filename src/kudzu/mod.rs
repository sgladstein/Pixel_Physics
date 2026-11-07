//! **Kudzu: a vine is eating the village.** `cargo run --release --bin kudzu`
//!
//! A prototype, not a game (`/mnt/project-files/game-design/new-game-
//! brainstorm-2026-11-07/prototypes/7-kudzu.md`). It asks one question about
//! fun in the hand: **is a cut that drops a whole sheet of vine satisfying,
//! and can a player read which stem feeds which arm well enough that cutting
//! is a deduction rather than a guess?** The hover preview -- everything that
//! would fall lights up -- is the whole bet.
//!
//! It runs on the shipped plant engine and changes it for this world only:
//!
//! - `World::plant_speedup` runs every plant here [`SPEEDUP`] times faster
//!   (the organism clock is shared with the lab, so the constant is not
//!   touched);
//! - `World::climber` makes the one vine species climb: it hugs stone and
//!   soil, and is held up by its roots and by stems lying on soil, never by
//!   stone, so a cut where an arm leaves the ground drops the arm
//!   (`plant::Climber`);
//! - `World::plant_load_failure` is off, so a living vine does not snap
//!   under its own weight (cuts still fall);
//! - the vine is its own species, `kudzu.ron` beside this file, registered
//!   into this world at runtime and never into the shipped registry.
//!
//! **What the game does on top, every frame, and why each was needed** (the
//! measurements are on each item's own doc):
//!
//! - `vigour`: new runners and roots from the vine's spare food, because the
//!   engine's own frontier is buds, and a vine that has wrapped one face has
//!   no bud facing the next;
//! - hunger is held one tick short of the starvation death, so debt thins
//!   the vine from its tips instead of killing it whole;
//! - `let_go`: what a cut disconnects turns to leaf litter and dead wood and
//!   pours down, because the engine's own fall left a cut sheet standing;
//! - `water_all`: the garden is kept at field capacity.
//!
//! The cut preview is `plant::severed_if_cut`, the engine's support walk run
//! read-only; `tests::preview_lights_what_the_engine_drops` makes a cut with
//! the game's own fall left out and checks the engine drops what was lit.
//! Regrowth from the stump and going pale when hungry are the engine's own.
//!
//! The binary (`src/bin/kudzu.rs`) is the window; this module is the game,
//! headless, so `examples/kudzuprobe.rs` plays exactly what a player plays.

pub mod hud;
pub mod level;

use crate::sim::cell::{Cell, OrganismId};
use crate::sim::explosion::Blasts;
use crate::sim::fxhash::PosSet;
use crate::sim::material::MaterialKind;
use crate::sim::organism::{self, CellType, SpeciesId};
use crate::sim::particle::ParticleSystem;
use crate::sim::world::World;
use crate::sim::{frame, plant, player};

pub use level::{Layout, Level};

/// The world, fixed for the probe: the size that reads on a phone.
pub const WIDTH: i32 = 256;
pub const HEIGHT: i32 = 160;

/// How much faster than baseline the vine grows. The brief asks for about
/// 6x; at 45 frames a tick that is a tick every 7 frames.
pub const SPEEDUP: u32 = 6;

/// `plant::Climber::cling`: score added per surface cell around a candidate.
pub const CLING: f32 = 1.5;

/// The simulation rate the window runs at, the same 60 Hz every binary uses.
pub const TICKS_PER_SECOND: u32 = 60;

/// Snip: charges held at most, and frames to earn one back.
pub const SNIP_CHARGES: u32 = 3;
pub const SNIP_RECHARGE: u32 = 8 * TICKS_PER_SECOND;
/// Radius of the blade, in cells: a snip takes a disc this wide out of the
/// vine, so a stem 1-2 cells thick is always cut through.
pub const SNIP_RADIUS: i32 = 2;
/// Dig out a crown: frames the button must be held.
pub const DIG_HOLD: u32 = 3 * TICKS_PER_SECOND;
/// How far round the pointer the dig reaches for the vine's root crown.
pub const DIG_RADIUS: i32 = 6;

/// **The vine's vigour**: every this many frames, each vine with fewer than
/// [`RUNNER_TARGET`] growing tips puts out one new runner from the edge of
/// its mat (`plant::sprout_climber`), if it can pay for one. Without it the
/// engine's vine wraps one wall face and stops, because every tip retires
/// and no bud faces the next wall.
pub const RUNNER_EVERY: u64 = 6;
pub const RUNNER_TARGET: usize = 24;
/// The vine keeps at least one root cell per this many shoot cells, and at
/// most [`ROOT_TIPS`] roots growing at once.
pub const SHOOT_PER_ROOT: u32 = 6;
pub const ROOT_TIPS: usize = 6;
/// A runner is put out only while income is at least this many times
/// upkeep.
pub const RUNNER_MARGIN: f32 = 1.2;

/// Wall clear at dawn for one, two and three stars.
pub const STAR_BARS: [f32; 3] = [0.5, 0.75, 0.95];

/// **A tuning knob read from the environment**, `default` when unset: the
/// vine's settings can be swept without a rebuild per point. The window and
/// the probe both read the same names, so a setting found by sweeping is
/// the one a player gets by exporting it.
pub fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// A list of cell positions.
pub type Cells = Vec<(i32, i32)>;

/// What the player has in hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Snip,
    Dig,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Snip => "SHEARS",
            Tool::Dig => "DIG OUT ROOTS",
        }
    }
}

/// The counts that go beside every picture.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    /// Snips made.
    pub cuts: u32,
    /// Vine cells a snip disconnected, summed over every cut (what the
    /// preview promised).
    pub predicted: u32,
    /// Of those, how many actually came away from the vine within
    /// [`FALL_WINDOW`] frames. Equal to `predicted` by construction while
    /// `let_go` turns every lit cell to litter; the preview's truth against
    /// the engine is the test's job, not this count's.
    pub fell: u32,
    /// Vine cells that came down, by any route: every cell that stopped
    /// being part of a vine organism other than by being cut or dug out.
    pub green_dropped: u32,
    /// Vine cells grown since play began.
    pub regrown: u32,
    /// Crowns dug out.
    pub dug: u32,
}

/// Frames after a cut within which its predicted cells must have left the
/// vine to count as `fell`.
pub const FALL_WINDOW: u64 = 600;

#[derive(Clone, Debug)]
struct PendingCut {
    at: u64,
    cells: Vec<(i32, i32)>,
    organism: OrganismId,
}

/// The game, headless.
pub struct Kudzu {
    pub world: World,
    pub particles: ParticleSystem,
    blasts: Blasts,
    tuning: player::Tuning,
    pub layout: Layout,
    pub vine: SpeciesId,
    pub tool: Tool,
    pub snips: u32,
    /// Frames until the next snip charge comes back.
    pub snip_timer: u32,
    pub digs: u32,
    /// The dig in progress: where, and frames held so far.
    pub digging: Option<((i32, i32), u32)>,
    /// Frames played since the level opened (after the overnight growth).
    pub played: u64,
    pub stats: Stats,
    pending: Vec<PendingCut>,
    /// Runners the vine has put out (`vigour`), overnight included.
    pub runners: u32,
    /// Vine cells last frame, for the dropped and regrown counts.
    last_cells: PosSet,
    /// Cells removed by the player this frame (cut or dug): not counted as
    /// dropped.
    removed_by_hand: PosSet,
}

impl Kudzu {
    /// A fresh level, grown overnight: the vine has already had
    /// `layout.overnight` frames when play starts.
    pub fn new(level: Level, seed: u64) -> Self {
        let mut world = World::new(crate::sim::chunk::Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
        world.seed = seed;
        // `KUDZU_SPECIES=path` reads the species from a file instead, so a
        // tuning sweep does not need a rebuild per point -- `include_str!`
        // compiles the file in, and an edit to it is invisible until the
        // next build (`.claude/rules/assets.md`).
        let source = match std::env::var("KUDZU_SPECIES") {
            Ok(path) => std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("KUDZU_SPECIES={path}: {e}")),
            Err(_) => include_str!("kudzu.ron").to_string(),
        };
        let vine = world.species.register_ron(&source).expect("the kudzu species parses");
        world.plant_speedup = env_or("KUDZU_SPEEDUP", SPEEDUP);
        // **A living vine does not snap under its own weight here.** The
        // load model carries a plant's weight down its own tissue to where
        // it anchors, and a climber anchors only at its roots, so a
        // 1,700-cell sheet hung entirely from three collar cells, which
        // broke into deadwood and dropped the lot in one frame (seed 1,
        // frame 175 of play; on half the seeds the vine was gone by dawn).
        // On a real wall the pads carry it. The lab's COLLAPSE UNDER LOAD
        // switch, off; a cut still falls (`plant_cut_falls` stays on).
        world.plant_load_failure = false;
        world.climber = Some(plant::Climber { species: vine, cling: CLING });
        // A clear day held at noon: the probe is about cutting, and a night
        // that stops the vine earning, or a rain that waters one arm, would
        // be noise between two paired runs.
        world.weather_override = crate::sim::weather::Pin::Clear.weather();
        world.clock.set_rates(0, |c| c.sky_hold = crate::sim::clock::SkyPin::Noon.hold());
        let layout = level::build(level, &mut world);
        let digs = layout.digs;
        let mut game = Kudzu {
            world,
            particles: ParticleSystem::new(),
            blasts: Blasts::new(),
            tuning: player::Tuning::default(),
            layout,
            vine,
            tool: Tool::Snip,
            snips: SNIP_CHARGES,
            snip_timer: 0,
            digs,
            digging: None,
            played: 0,
            stats: Stats::default(),
            pending: Vec::new(),
            runners: 0,
            last_cells: PosSet::default(),
            removed_by_hand: PosSet::default(),
        };
        for _ in 0..game.layout.overnight {
            if game.layout.watered {
                game.water_all();
            }
            game.step_world();
            game.vigour();
        }
        game.last_cells = game.vine_cells();
        game
    }

    fn step_world(&mut self) {
        frame::step(&mut self.world, &mut self.particles, &mut self.blasts, player::PlayerInput::default(), &self.tuning);
    }

    /// One frame of the vine's push: see [`RUNNER_EVERY`].
    fn vigour(&mut self) {
        if !self.world.frame.is_multiple_of(env_or("KUDZU_RUNNER_EVERY", RUNNER_EVERY)) {
            return;
        }
        for id in self.vines() {
            // **Hunger thins the vine; it never kills it outright.** The
            // engine's count marks a plant senescent after
            // `STARVATION_DEATH_TICKS` hungry ticks in a row, and a
            // senescent vine rots away whole -- a binary end (`CLAUDE.md`,
            // law 1), and on half the seeds it came mid-game with nobody
            // touching it: the overnight vine reached 2,800 cells, shaded
            // itself into debt and was gone by frame 5,500. Held one tick
            // short of the mark, the vine keeps the engine's own graded
            // answer to debt -- dieback from the tips inward -- and the only
            // way to kill it outright is to dig out its crown.
            // A vine with no roots left (dug out) is not held.
            if let Some(s) = self.world.organism_mut(id).filter(|s| s.root_cells > 0) {
                s.starving_ticks = s.starving_ticks.min(plant::STARVATION_DEATH_TICKS - 2);
            }
            // **Roots first, and whatever the books say.** Only roots drink
            // once the vine is off the ground (`plant::sprout_climber_root`),
            // so a dry vine that stopped rooting until it was rich again
            // would never be rich again. One root cell for every
            // [`SHOOT_PER_ROOT`] of shoot.
            let (shoot, root) = self.world.organism(id).map_or((0, 0), |s| (s.shoot_cells, s.root_cells));
            if root * SHOOT_PER_ROOT < shoot && self.world.organism_active_tip_count(id, CellType::RootTip) < ROOT_TIPS {
                let mut rng = crate::sim::rng::stream(self.world.seed ^ 0x526F_6F74, id as u64, 0, self.world.frame);
                plant::sprout_climber_root(&mut self.world, id, |n| rng.below(n as u32) as usize);
            }
            if self.world.organism_active_tip_count(id, CellType::GrowingTip) >= env_or("KUDZU_RUNNER_TARGET", RUNNER_TARGET) {
                continue;
            }
            // **Only out of surplus.** A vine that put out runners whatever
            // its books said outgrew its income and starved to death inside
            // a minute (measured: 1,997 cells -> 273 between frames 4,000
            // and 8,000, senescent). The engine's own tips already stop on
            // carbon; this push has to stop on the budget too.
            if plant::day_budget_ratio(&self.world, id).is_none_or(|r| r < env_or("KUDZU_RUNNER_MARGIN", RUNNER_MARGIN)) {
                continue;
            }
            let mut rng = crate::sim::rng::stream(self.world.seed ^ 0x4B75_647A_7521, id as u64, 0, self.world.frame);
            if plant::sprout_climber(&mut self.world, id, |n| rng.below(n as u32) as usize).is_some() {
                self.runners += 1;
            }
        }
    }

    /// **The garden is watered**: every soil cell under the ground line is
    /// held at field capacity. Roots here drink once a plant tick, six
    /// times as often as baseline, while the soil's own water moves per real
    /// frame; left alone the bed dried to a water status of 0.13 in the
    /// overnight run and the vine stalled. A stand-in for the village's
    /// spring, gutter and well, which are the level's real water.
    fn water_all(&mut self) {
        let soil = self.world.materials.id_of("soil").expect("soil");
        for x in 0..WIDTH {
            for y in self.layout.ground_y..HEIGHT - 1 {
                let c = self.world.get(x, y);
                if c.material == soil && c.aux() < crate::sim::material::SOIL_FIELD_CAPACITY {
                    self.world.set(x, y, c.with_aux(crate::sim::material::SOIL_FIELD_CAPACITY));
                }
            }
        }
    }

    /// Every living vine organism.
    pub fn vines(&self) -> Vec<OrganismId> {
        self.world
            .live_organism_ids()
            .into_iter()
            .filter(|&id| self.world.organism(id).is_some_and(|s| s.species == self.vine && !s.cells.is_empty()))
            .collect()
    }

    /// Every cell of every living vine.
    pub fn vine_cells(&self) -> PosSet {
        let mut out = PosSet::default();
        for id in self.vines() {
            if let Some(s) = self.world.organism(id) {
                out.extend(s.cells.keys().copied());
            }
        }
        out
    }

    /// One frame of play. Does nothing once dawn has come.
    pub fn tick(&mut self) {
        if self.over() {
            return;
        }
        if self.layout.watered {
            self.water_all();
        }
        self.step_world();
        self.vigour();
        self.played += 1;
        if self.snips < SNIP_CHARGES {
            self.snip_timer = self.snip_timer.saturating_sub(1);
            if self.snip_timer == 0 {
                self.snips += 1;
                if self.snips < SNIP_CHARGES {
                    self.snip_timer = SNIP_RECHARGE;
                }
            }
        }
        // Dropped and regrown, by difference against last frame.
        let now = self.vine_cells();
        for p in self.last_cells.difference(&now) {
            if !self.removed_by_hand.contains(p) {
                self.stats.green_dropped += 1;
            }
        }
        self.stats.regrown += now.difference(&self.last_cells).count() as u32;
        self.removed_by_hand.clear();
        // Did each cut's prediction come true?
        let frame_now = self.world.frame;
        let mut i = 0;
        while i < self.pending.len() {
            let p = &self.pending[i];
            let gone = p.cells.iter().filter(|&&(x, y)| self.world.get(x, y).organism_id() != p.organism).count();
            if gone == p.cells.len() || frame_now >= p.at + FALL_WINDOW {
                self.stats.fell += gone as u32;
                self.pending.swap_remove(i);
            } else {
                i += 1;
            }
        }
        self.last_cells = now;
    }

    /// The disc a snip at `(x, y)` would take: vine cells within
    /// [`SNIP_RADIUS`].
    pub fn blade(&self, x: i32, y: i32) -> Vec<(i32, i32, OrganismId)> {
        let mut out = Vec::new();
        for dy in -SNIP_RADIUS..=SNIP_RADIUS {
            for dx in -SNIP_RADIUS..=SNIP_RADIUS {
                if dx * dx + dy * dy > SNIP_RADIUS * SNIP_RADIUS + 1 {
                    continue;
                }
                let (cx, cy) = (x + dx, y + dy);
                let c = self.world.get(cx, cy);
                let id = c.organism_id();
                if id != 0 && self.world.organism(id).is_some_and(|s| s.species == self.vine) {
                    out.push((cx, cy, id));
                }
            }
        }
        out
    }

    /// **The hover preview**: every vine cell a snip at `(x, y)` would drop,
    /// and the blade's own cells. Empty if the blade touches no vine.
    pub fn preview(&self, x: i32, y: i32) -> (Cells, Cells) {
        let blade = self.blade(x, y);
        let mut drop = Vec::new();
        let mut ids: Vec<OrganismId> = blade.iter().map(|b| b.2).collect();
        ids.sort_unstable();
        ids.dedup();
        for id in ids {
            let removed: PosSet = blade.iter().filter(|b| b.2 == id).map(|b| (b.0, b.1)).collect();
            drop.extend(plant::severed_if_cut(&self.world, id, &removed));
        }
        (drop, blade.iter().map(|b| (b.0, b.1)).collect())
    }

    /// Snip at `(x, y)`. Returns how many cells the preview said would fall,
    /// or `None` if there was no charge or no vine under the blade.
    pub fn snip(&mut self, x: i32, y: i32) -> Option<usize> {
        if self.snips == 0 {
            return None;
        }
        let blade = self.blade(x, y);
        if blade.is_empty() {
            return None;
        }
        let (drop, _) = self.preview(x, y);
        let organism = blade[0].2;
        for &(cx, cy, _) in &blade {
            self.world.set(cx, cy, Cell::EMPTY);
            self.world.record_disturbance(cx, cy, 1);
            self.removed_by_hand.insert((cx, cy));
        }
        if self.snips == SNIP_CHARGES {
            self.snip_timer = SNIP_RECHARGE;
        }
        self.snips -= 1;
        self.stats.cuts += 1;
        self.stats.predicted += drop.len() as u32;
        let n = drop.len();
        self.let_go(&drop);
        self.pending.push(PendingCut { at: self.world.frame, cells: drop, organism });
        self.world.log_player_action(format!("SNIPPED AT {x},{y} -- {n} CELLS LET GO"));
        Some(n)
    }

    /// **Everything a cut disconnected comes down now, loose.** Leaves
    /// become `deadleaf` (green, clumping, flammable, rots to litter) and
    /// stem becomes `deadwood`, both powders, so the sheet pours off the
    /// wall and heaps at its foot.
    ///
    /// Not the engine's own fall, and that was measured rather than chosen:
    /// left to `structural`, a sheet cut from a wall face became one tall
    /// thin piece standing on its own foot against the stone, with its
    /// leaves clinging to it (`deadleaf`'s `clings_to_wood`), and did not
    /// move -- 871 cells let go and the picture did not change. Physically
    /// fair (it was resting on the ground) and no use to a player: the brief
    /// asks for what falls to come down as clumps.
    fn let_go(&mut self, cells: &[(i32, i32)]) {
        let leaf = self.world.materials.id_of("deadleaf");
        let wood = self.world.materials.id_of("deadwood");
        for &(x, y) in cells {
            let c = self.world.get(x, y);
            let is_leaf = organism::cell_type(c.aux()) == Some(CellType::Leaf);
            let Some(m) = (if is_leaf { leaf } else { wood }) else { continue };
            let shades = self.world.materials.get(m).palette.len().max(1) as u32;
            let shade = (crate::sim::rng::jitter(x, y) * shades as f32) as u8 % shades as u8;
            self.world.set(x, y, Cell::new(m, shade));
            self.world.record_disturbance(x, y, 1);
        }
    }

    /// The root crown nearest `(x, y)` within [`DIG_RADIUS`]: the vine cells
    /// in soil, which hold it up and hold its store.
    pub fn crown_near(&self, x: i32, y: i32) -> Option<OrganismId> {
        let mut best: Option<(i32, OrganismId)> = None;
        for dy in -DIG_RADIUS..=DIG_RADIUS {
            for dx in -DIG_RADIUS..=DIG_RADIUS {
                let c = self.world.get(x + dx, y + dy);
                let id = c.organism_id();
                if id == 0 || !self.world.organism(id).is_some_and(|s| s.species == self.vine) {
                    continue;
                }
                let root = self.world.materials.get(c.material).reinforces_powder
                    || organism::cell_type(c.aux()) == Some(CellType::RootTip);
                if !root {
                    continue;
                }
                let d = dx * dx + dy * dy;
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, id));
                }
            }
        }
        best.map(|b| b.1)
    }

    /// Hold the dig at `(x, y)` for one frame. Returns true on the frame the
    /// crown comes out.
    pub fn hold_dig(&mut self, x: i32, y: i32) -> bool {
        if self.digs == 0 || self.crown_near(x, y).is_none() {
            self.digging = None;
            return false;
        }
        let held = match self.digging {
            Some((at, n)) if (at.0 - x).abs() <= 2 && (at.1 - y).abs() <= 2 => n + 1,
            _ => 1,
        };
        self.digging = Some(((x, y), held));
        if held < DIG_HOLD {
            return false;
        }
        self.digging = None;
        self.dig_out(x, y)
    }

    /// Pull the crown out: every root cell of the nearest vine is replaced
    /// by the soil it grew through, and the plant's store goes with it.
    fn dig_out(&mut self, x: i32, y: i32) -> bool {
        let Some(id) = self.crown_near(x, y) else { return false };
        let soil = self.world.materials.id_of("soil").expect("soil is compiled in");
        let cells: Vec<(i32, i32)> = self.world.organism(id).map(|s| s.cells.keys().copied().collect()).unwrap_or_default();
        let mut n = 0u32;
        for (cx, cy) in cells {
            let c = self.world.get(cx, cy);
            let m = self.world.materials.get(c.material);
            let root = m.reinforces_powder || organism::cell_type(c.aux()) == Some(CellType::RootTip);
            if root {
                self.world.set(cx, cy, Cell::new(soil, 0).with_aux(crate::sim::material::SOIL_FIELD_CAPACITY));
                self.world.record_disturbance(cx, cy, 1);
                self.removed_by_hand.insert((cx, cy));
                n += 1;
            } else if self.world.materials.kind(c.material) != MaterialKind::Empty {
                // The shoot at the collar, too: a stem resting on soil
                // anchors (`is_structural_anchor`), so a dug crown whose
                // runners still lay on the ground would stay up.
                let on_soil = matches!(self.world.materials.kind(self.world.get(cx, cy + 1).material), MaterialKind::Powder);
                if on_soil && (cx - x).abs() <= DIG_RADIUS && (cy - y).abs() <= DIG_RADIUS {
                    self.world.set(cx, cy, Cell::EMPTY);
                    self.world.record_disturbance(cx, cy, 1);
                    self.removed_by_hand.insert((cx, cy));
                    n += 1;
                }
            }
        }
        self.digs -= 1;
        self.stats.dug += 1;
        self.world.log_player_action(format!("DUG OUT A CROWN AT {x},{y} -- {n} CELLS"));
        true
    }

    /// Whether dawn has come: the level is over and scored.
    pub fn over(&self) -> bool {
        self.played >= self.layout.length
    }

    /// Frames left until dawn.
    pub fn left(&self) -> u64 {
        self.layout.length.saturating_sub(self.played)
    }

    /// Stars for a wall-clear fraction: 50 / 75 / 95%.
    pub fn stars_for(clear: f32) -> u32 {
        STAR_BARS.iter().filter(|&&b| clear >= b).count() as u32
    }

    /// The vine's books against the engine's starvation test
    /// (`plant::day_budget_ratio`), the hungriest vine's: below 1.0 it is
    /// starving. `None` when no vine is left.
    pub fn budget(&self) -> Option<f32> {
        self.vines().iter().filter_map(|&id| plant::day_budget_ratio(&self.world, id)).reduce(f32::min)
    }

    /// Fraction of the wall faces' cells (the strips beside each wall that a
    /// vine would grow in) with no vine on them, 0..1.
    pub fn wall_clear(&self) -> f32 {
        let mut total = 0u32;
        let mut clear = 0u32;
        for &(x, y) in &self.layout.faces {
            total += 1;
            let id = self.world.get(x, y).organism_id();
            if id == 0 || !self.world.organism(id).is_some_and(|s| s.species == self.vine) {
                clear += 1;
            }
        }
        if total == 0 {
            1.0
        } else {
            clear as f32 / total as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The biggest-dropping blade position among every seventh vine cell.
    fn big_cut(game: &Kudzu) -> ((i32, i32), Vec<(i32, i32)>) {
        let mut cells: Vec<(i32, i32)> = game.vine_cells().into_iter().filter(|&(_, y)| y < game.layout.ground_y).collect();
        cells.sort_unstable();
        let mut best = ((0, 0), Vec::new());
        for &(x, y) in cells.iter().step_by(7) {
            let drop = game.preview(x, y).0;
            if drop.len() > best.1.len() {
                best = ((x, y), drop);
            }
        }
        best
    }

    /// **The preview is the engine's own answer.** The game turns what a cut
    /// drops into litter itself (`let_go`), so "promised" against "fell" in
    /// the probe agrees by construction and checks nothing. This test makes
    /// the cut *without* `let_go` and lets the engine's own support walk
    /// decide: every cell the preview lit must leave the vine, and almost
    /// nothing it did not light may leave with it.
    ///
    /// Positive control, run by hand 2026-11-07: with the climber's anchor
    /// rule in `is_structural_anchor` switched off (the shipped rule, where a
    /// stem resting on stone holds), no cut lit a sheet -- the walls held
    /// every arm -- and this failed on its first assert.
    #[test]
    fn preview_lights_what_the_engine_drops() {
        let mut game = Kudzu::new(Level::Walls, 1);
        let ((x, y), drop) = big_cut(&game);
        assert!(drop.len() > 100, "a sheet worth cutting: best cut lights {} cells", drop.len());
        let id = game.blade(x, y)[0].2;
        let lit: PosSet = drop.iter().copied().collect();
        let blade: Vec<(i32, i32)> = game.blade(x, y).iter().map(|b| (b.0, b.1)).collect();
        let before: PosSet = game.vine_cells();
        for &(cx, cy) in &blade {
            game.world.set(cx, cy, Cell::EMPTY);
            game.world.record_disturbance(cx, cy, 1);
        }
        for _ in 0..30 {
            game.step_world();
        }
        let stayed = drop.iter().filter(|&&(cx, cy)| game.world.get(cx, cy).organism_id() == id).count();
        assert_eq!(stayed, 0, "{stayed} of {} lit cells are still on the vine", drop.len());
        let blade_set: PosSet = blade.iter().copied().collect();
        let unlit_lost = before
            .iter()
            .filter(|p| !lit.contains(p) && !blade_set.contains(p))
            .filter(|&&(cx, cy)| game.world.get(cx, cy).organism_id() != id)
            .count();
        assert!(unlit_lost * 10 <= drop.len(), "{unlit_lost} cells the preview did not light left the vine (lit {})", drop.len());
    }

    /// Every switch the game turns on is off in a world it did not make.
    #[test]
    fn the_game_switches_are_off_by_default() {
        let world = World::new(crate::sim::chunk::Rect::new(0, 0, 63, 63));
        assert_eq!(world.plant_speedup, 1);
        assert!(world.climber.is_none());
        assert_eq!(world.organism_interval(45), world.clock.organism_interval(45));
    }
}
