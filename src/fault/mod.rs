//! **Fault**: crack the ice wall before the lake breaks it.
//!
//! A probe, not a game (`/mnt/project-files/game-design/new-game-brainstorm-
//! 2026-11-07/prototypes/6-fault.md`). It asks one question about fun in the
//! hand: *is a running crack satisfying to cause, and can a player predict
//! roughly where it will go?* Everything here exists to let that be felt and
//! measured, and nothing more.
//!
//! # What is new and what is shipped
//!
//! - **New, and only in this module:** the tension field, the crack tips,
//!   the wetness that Meltwater leaves in a crack, the lake's clock and push,
//!   the score.
//! - **Shipped, and not touched:** what a crack *does*. A tip writes the
//!   engine's own edge cracks (`Cell::with_crack_right`/`_down`), schedules
//!   the structural checks a hammer blow would, and licenses the failure
//!   with `World::record_disturbance`. When a ring of cracks has cut a piece
//!   off every anchor, it is `structural.rs`/`load.rs` that notice and
//!   `rigid.rs` that throws it as a falling body; water then runs through
//!   the gap under the shipped liquid rules. There is no "slab" code here.
//!
//! # Why the tips walk the ice's own grain
//!
//! `Reports/dead-ends.md` records **four** rejections by the owner of crack
//! walkers drawn on rock (*"looks like a graphic, not physics"*, *"it
//! shouldn't look like a scribble"*), all for the same two missing
//! properties: straight edges and closed cells. So a tip here does not pick
//! a heading; it walks the joints of `fracture_field`'s Worley fabric -- the
//! same grain a blast and a hammer open -- and only chooses *which* joint to
//! take at a junction, by tension. The segments are straight by
//! construction, a crack that goes round a grain cell has enclosed it, and
//! the same tap on the same ice always runs the same way (the fabric is a
//! pure function of seed and position).
//!
//! The rejection was for *destruction* debris, where a line outnumbering
//! the breakage was the defect. Here the running crack is the verb itself,
//! so the line is the point -- but the grain keeps it from being a scribble.

pub mod level;
pub mod tension;
pub mod tips;
pub mod view;

use crate::sim::explosion::Blasts;
use crate::sim::frame;
use crate::sim::material::{self, MaterialId, MaterialKind};
use crate::sim::particle::ParticleSystem;
use crate::sim::player;
use crate::sim::world::World;

pub use level::{H, W};

/// Simulation frames per second of play time. The clock, the lake's rise
/// and every "within a second" claim below are counted in these.
pub const FPS: u32 = 60;

/// What the player can do. Plain world words, because they go on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    /// Strike the ice: a crack starts at the nearest joint and runs both ways.
    Tap,
    /// Bore a small hole. A crack that reaches it stops dead.
    Drill,
    /// Pour meltwater. It soaks along every open crack it touches.
    Meltwater,
    /// Freeze every wet crack at once: each one grows from both ends.
    Frost,
}

impl Tool {
    pub const ALL: [Tool; 4] = [Tool::Tap, Tool::Drill, Tool::Meltwater, Tool::Frost];

    pub fn label(self) -> &'static str {
        match self {
            Tool::Tap => "TAP",
            Tool::Drill => "DRILL",
            Tool::Meltwater => "MELTWATER",
            Tool::Frost => "FROST",
        }
    }

    /// The sentence the HUD shows: what your next click will do.
    pub fn says(self) -> &'static str {
        match self {
            Tool::Tap => "CLICK THE ICE TO START A CRACK",
            Tool::Drill => "CLICK TO BORE A HOLE THAT STOPS CRACKS",
            Tool::Meltwater => "HOLD TO POUR WATER INTO THE CRACKS",
            Tool::Frost => "CLICK TO FREEZE EVERY WET CRACK",
        }
    }
}

/// One thing the player did, at a world cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Action {
    pub tool: Tool,
    pub x: i32,
    pub y: i32,
}

/// Discrete event counts, printed beside every picture (`CLAUDE.md`: a
/// picture says what and where, only a count says whether it fired).
#[derive(Clone, Copy, Debug, Default)]
pub struct Counts {
    pub taps: u32,
    pub drills: u32,
    pub pours: u32,
    pub frosts: u32,
    /// Crack tips that ran at least one edge.
    pub cracks_run: u32,
    /// Edges of ice cracked, by tips (not by the shipped collapse).
    pub edges_cracked: u32,
    /// Tips that forked off another tip.
    pub forks: u32,
    /// Tips that stopped because they reached a drilled hole.
    pub stopped_by_hole: u32,
    /// Tips that stopped at a free face: air, water or the wall's edge.
    pub stopped_at_face: u32,
    /// Tips that ran into an existing crack and joined it.
    pub joined: u32,
    /// Tips that ran out of push before reaching anything.
    pub died: u32,
    /// Tips that Frost woke at the end of a wet crack.
    pub frost_tips: u32,
    /// Falling bodies the shipped collapse made out of the ice.
    pub slabs_freed: u32,
    /// Pieces a closed ring of cracks cut off and handed to the shipped
    /// break, and how many cells they held.
    pub pieces_cut: u32,
    pub piece_cells: u32,
    /// Severed pieces the lake (or a missing floor) set moving.
    pub pieces_released: u32,
    /// Grain blocks those pieces left as, thrown as bodies.
    pub blocks_thrown: u32,
    /// Cells of scraps too small to be a block, gone to slush.
    pub grit_cells: u32,
    /// Sliding pieces that jammed against something and stopped.
    pub jammed: u32,
    /// Lumps of ice found with nothing holding them up, and let fall.
    pub unheld: u32,
    /// Ice cells that hit the mill's box while moving, or mill cells lost.
    pub mill_hits: u32,
    /// Did the lake reach the top and burst the wall?
    pub burst: bool,
    /// Most water seen standing in the mill at once, in full cells.
    pub mill_flood: u32,
}

/// Graded outcome. Stars, never just win or lose.
#[derive(Clone, Copy, Debug, Default)]
pub struct Score {
    /// Highest spillway flow seen, in full water cells per second.
    pub peak_flow: f32,
    /// Share of the lake's peak volume drained by the end, 0..1.
    pub drained: f32,
    pub mill_hit: bool,
    pub burst: bool,
    pub stars: u32,
}

/// The spillway mark, in full water cells per second: about what the gorge
/// can carry away. It is only a readout -- what is graded is whether the
/// flood actually reached the mill, which is the world's own answer to a
/// spillway cut too big.
pub const FLOW_MARK: f32 = 120.0;
/// Holes a level gives you.
pub const DRILLS: u32 = 5;
/// Taps a level gives you. Unlimited, a random tapper carved 45 taps' worth
/// of cracks into the wall, lowered the crest by brute force and kept the
/// lake under it on 7 walls of 10 -- it played as well as a plan did. A
/// stock makes each tap a decision. A tap that starts no crack is free.
pub const TAPS: u32 = 10;

/// Water standing in the mill, in full cells, that counts as flooding it.
pub const MILL_FLOOD: u32 = 20;

/// How long the thaw lasts, in seconds. The lake rises until then; at the
/// end the score is read.
pub const THAW_SECONDS: u32 = 120;

/// The strongest tension on the ice at the start, in units a tip feels.
/// At 1.0 a crack pays its own way; so this says the deep upstream face is
/// already past self-running at the start, and the top of the wall is not.
pub const TENSION_SCALE: f32 = 1.6;

pub struct Game {
    pub world: World,
    pub particles: ParticleSystem,
    blasts: Blasts,
    pub ice: MaterialId,
    pub tension: tension::Field,
    pub tips: tips::Cracks,
    pub level: level::Level,
    pub counts: Counts,
    /// The shimmer: a faint tension tint on the ice. The A/B under test.
    pub shimmer: bool,
    /// Flow out through the wall, per frame, over the last second.
    flow_window: std::collections::VecDeque<u32>,
    pub peak_flow: f32,
    pub drained_total: u64,
    pub peak_volume: u64,
    /// Bodies already counted as freed, by the frame they appeared.
    seen_bodies: usize,
    /// Screen flashes for a fresh event, `(x, y, frames left, colour)`.
    pub flashes: Vec<(i32, i32, u32, [u8; 4])>,
    /// Frame until which the wet cracks Frost just froze are drawn flashing.
    pub frost_flash: u64,
    /// Holes left to drill. Few on purpose: with enough of them a row of
    /// holes is a saw cut and the cracks have no job.
    pub drills_left: u32,
    pub taps_left: u32,
    /// The tension a tip feels as "1": fixed at the start of the level so
    /// that the wall really does get more dangerous as the lake rises.
    unit: f32,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut world = level::build(seed);
        let ice = world.materials.id_of("ice").expect("ice is a shipped material");
        // **The one per-world change to a shipped material, and why it is
        // per-world.** Ice melts at 4 degrees against a 20-degree ambient,
        // by construction (`ice.ron`'s header): it exists only while the
        // weather keeps chilling it. This box has no weather, so the wall
        // would melt in seconds. The registry is owned by the world, so
        // raising the melting point here changes nothing for any other game.
        world.materials.get_mut(ice).melting_point = 400.0;
        // And recoloured, the same way and for the same reason: a white
        // crack does not read on the shipped pale-blue ice.
        world.materials.get_mut(ice).palette = view::ICE_PALETTE.to_vec();
        // Stone's fragment ladder rather than ice's own: the shipped ice is
        // a thin pond sheet that should craze into plates of 2-16 cells,
        // and a wall that did that would never shed a slab.
        world.materials.get_mut(ice).fragment_rungs = 6;
        // And the lake never freezes. A cold front out of `weather.rs`
        // skinned seed 3's lake with single ice cells, which
        // `tips::support_sweep` (rightly: they are hanging on nothing but
        // water) crumbled to snow -- 4,900 cells of it -- and the lake never
        // reached the crest.
        world.materials.get_mut(material::WATER).cooling_point = -1000.0;
        let level = level::Level::new(&world);
        let tension = tension::Field::new();
        let mut g = Game {
            world,
            particles: ParticleSystem::new(),
            blasts: Blasts::new(),
            ice,
            tension,
            tips: tips::Cracks::new(),
            level,
            counts: Counts::default(),
            shimmer: false,
            flow_window: std::collections::VecDeque::new(),
            peak_flow: 0.0,
            drained_total: 0,
            peak_volume: 0,
            seen_bodies: 0,
            flashes: Vec::new(),
            frost_flash: 0,
            drills_left: DRILLS,
            taps_left: TAPS,
            unit: 1.0,
        };
        g.settle();
        g
    }

    /// Settle the field and fix the unit. Called once, before play.
    fn settle(&mut self) {
        self.tension.update(&self.world, self.ice, &self.level, &self.tips);
        self.unit = (self.tension.max() / TENSION_SCALE).max(1e-6);
    }

    pub fn tension_unit(&self) -> f32 {
        self.unit
    }

    /// A copy to try a move on. The particles are not copied -- they are
    /// spray, and nothing a lookahead asks about reads them.
    pub fn trial(&self) -> Game {
        Game {
            world: self.world.clone(),
            particles: ParticleSystem::new(),
            blasts: Blasts::new(),
            ice: self.ice,
            tension: self.tension.clone(),
            tips: self.tips.clone(),
            level: self.level.clone(),
            counts: self.counts,
            shimmer: self.shimmer,
            flow_window: self.flow_window.clone(),
            peak_flow: self.peak_flow,
            drained_total: self.drained_total,
            peak_volume: self.peak_volume,
            seen_bodies: self.seen_bodies,
            flashes: Vec::new(),
            frost_flash: self.frost_flash,
            drills_left: self.drills_left,
            taps_left: self.taps_left,
            unit: self.unit,
        }
    }

    /// Run the cracks only -- no water, no bodies -- until every tip has
    /// stopped, then look for what they cut off. What a player does in
    /// their head before tapping, and what the harness's planned player
    /// does on a `trial` copy.
    pub fn settle_cracks(&mut self) {
        for _ in 0..200 {
            if self.tips.tips.is_empty() {
                break;
            }
            tips::advance(self);
        }
        tips::find_severed(self);
    }

    pub fn seconds(&self) -> f32 {
        self.world.frame as f32 / FPS as f32
    }

    pub fn over(&self) -> bool {
        self.world.frame >= (THAW_SECONDS * FPS) as u64
    }

    pub fn is_ice(&self, x: i32, y: i32) -> bool {
        self.world.in_bounds(x, y) && self.world.get(x, y).material == self.ice
    }

    /// Do one thing. Returns whether anything happened -- a tap on air, for
    /// instance, does nothing and says so.
    pub fn act(&mut self, a: Action) -> bool {
        match a.tool {
            Tool::Tap => {
                if self.taps_left == 0 {
                    return false;
                }
                self.counts.taps += 1;
                let n = tips::tap(self, a.x, a.y);
                if n > 0 {
                    self.taps_left -= 1;
                    self.flashes.push((a.x, a.y, 10, [255, 255, 255, 255]));
                }
                n > 0
            }
            Tool::Drill => {
                if self.drills_left == 0 {
                    return false;
                }
                let did = self.drill(a.x, a.y);
                if did {
                    self.counts.drills += 1;
                    self.drills_left -= 1;
                }
                did
            }
            Tool::Meltwater => {
                self.counts.pours += 1;
                self.pour(a.x, a.y)
            }
            Tool::Frost => {
                self.counts.frosts += 1;
                let n = tips::frost(self);
                n > 0
            }
        }
    }

    /// Bore a hole of radius `DRILL_R`. Real cells come out, through the
    /// engine's own paint so the structural checks it raises are the
    /// shipped ones.
    fn drill(&mut self, x: i32, y: i32) -> bool {
        let mut any = false;
        for dy in -tips::DRILL_R..=tips::DRILL_R {
            for dx in -tips::DRILL_R..=tips::DRILL_R {
                if dx * dx + dy * dy > tips::DRILL_R * tips::DRILL_R + 1 {
                    continue;
                }
                let (cx, cy) = (x + dx, y + dy);
                if self.is_ice(cx, cy) {
                    any = true;
                    self.tips.mark_hole(cx, cy);
                }
            }
        }
        if any {
            self.world.paint_circle(x, y, tips::DRILL_R, material::EMPTY);
            self.world.record_disturbance(x, y, tips::DRILL_R);
            self.flashes.push((x, y, 8, [255, 210, 120, 255]));
        }
        any
    }

    /// Pour a splash of meltwater: real water cells, which run wherever the
    /// shipped liquid rules take them. What makes it *meltwater* is
    /// `tips::soak`, which wets any crack the water touches.
    fn pour(&mut self, x: i32, y: i32) -> bool {
        let water = material::WATER;
        let mut n = 0;
        for dy in -2..=0 {
            for dx in -2..=2 {
                let (cx, cy) = (x + dx, y + dy);
                if self.world.in_bounds(cx, cy) && self.world.get(cx, cy).material == material::EMPTY {
                    self.world.set(cx, cy, crate::sim::cell::Cell::new(water, 0).with_aux(material::LIQUID_FULL));
                    n += 1;
                }
            }
        }
        n > 0
    }

    pub fn step(&mut self) {
        self.level.rise(&mut self.world, &mut self.counts);
        self.tension.update(&self.world, self.ice, &self.level, &self.tips);
        tips::soak(self);
        tips::advance(self);
        tips::find_severed(self);
        tips::release_ready(self);
        tips::slide(self);
        if self.world.frame.is_multiple_of(tips::SUPPORT_EVERY) {
            tips::support_sweep(self);
        }
        if self.counts.burst && !self.level.burst_started {
            self.level.burst_started = true;
            tips::burst(self);
        }
        self.level.push_bodies(&mut self.world);
        frame::step(&mut self.world, &mut self.particles, &mut self.blasts, player::PlayerInput::default(), &player::Tuning::default());
        self.census();
        for f in &mut self.flashes {
            f.2 = f.2.saturating_sub(1);
        }
        self.flashes.retain(|f| f.2 > 0);
    }

    fn census(&mut self) {
        // Bodies: every falling piece made of ice is a slab freed. The
        // shipped collapse decides they exist; this only counts them.
        let n = self.world.chunk_bodies.len();
        if n > self.seen_bodies {
            for b in &self.world.chunk_bodies[self.seen_bodies..] {
                if b.cells.iter().any(|c| c.material == self.ice) {
                    self.counts.slabs_freed += 1;
                }
            }
        }
        self.seen_bodies = n;
        // Mill: a moving body overlapping the mill's box.
        let m = self.level.mill;
        for b in &self.world.chunk_bodies {
            let (bx, by) = (b.x as i32, b.y as i32);
            if b.cells.iter().any(|c| {
                let (x, y) = (bx + c.dx, by + c.dy);
                x >= m.0 && x <= m.2 && y >= m.1 && y <= m.3
            }) && (b.vx.abs() + b.vy.abs()) > 0.3
            {
                self.counts.mill_hits += 1;
            }
        }
        self.counts.mill_flood = self.counts.mill_flood.max(self.level.mill_water(&self.world));
        let drained = self.level.drain(&mut self.world);
        self.drained_total += drained as u64;
        self.flow_window.push_back(drained);
        if self.flow_window.len() > FPS as usize {
            self.flow_window.pop_front();
        }
        let per_sec: u32 = self.flow_window.iter().sum();
        let per_sec = per_sec as f32 / material::LIQUID_FULL as f32;
        self.peak_flow = self.peak_flow.max(per_sec);
        let v = self.level.lake_volume(&self.world);
        self.peak_volume = self.peak_volume.max(v);
    }

    pub fn flow_now(&self) -> f32 {
        let s: u32 = self.flow_window.iter().sum();
        s as f32 / material::LIQUID_FULL as f32
    }

    pub fn score(&self) -> Score {
        let v = self.level.lake_volume(&self.world) as f32;
        let drained = if self.peak_volume > 0 { (1.0 - v / self.peak_volume as f32).clamp(0.0, 1.0) } else { 0.0 };
        let mill_hit = self.counts.mill_hits > 0 || self.counts.mill_flood >= MILL_FLOOD;
        let burst = self.counts.burst;
        let mut stars = 0;
        if !burst {
            stars += 1;
            if drained >= 0.5 {
                stars += 1;
            }
            if !mill_hit && drained > 0.05 {
                stars += 1;
            }
        }
        Score { peak_flow: self.peak_flow, drained, mill_hit, burst, stars }
    }
}

/// Whether `m` is something a crack can run through.
pub fn is_body(world: &World, m: MaterialId) -> bool {
    world.materials.kind(m) == MaterialKind::Solid
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cut_edges(g: &Game) -> Vec<(i32, i32, bool)> {
        let mut v = Vec::new();
        for y in 0..level::H {
            for x in 0..level::W {
                let c = g.world.get(x, y);
                if g.is_ice(x, y) && c.crack_right() {
                    v.push((x, y, true));
                }
                if g.is_ice(x, y) && c.crack_down() {
                    v.push((x, y, false));
                }
            }
        }
        v
    }

    fn tapped(seed: u64) -> Game {
        let mut g = Game::new(seed);
        assert!(g.act(Action { tool: Tool::Tap, x: 112, y: 60 }), "a tap on the wall's face starts no crack");
        g.settle_cracks();
        g
    }

    /// A tap runs a crack of a real length, spends one tap from the stock,
    /// and the same tap on the same wall runs the same crack edge for edge
    /// -- the repeatability the probe is asking a player to rely on.
    #[test]
    fn a_tap_runs_the_same_crack_on_the_same_wall() {
        let a = tapped(1);
        let b = tapped(1);
        let edges = cut_edges(&a);
        assert!(edges.len() >= 20, "a tap ran only {} edges", edges.len());
        assert_eq!(a.taps_left, TAPS - 1);
        assert_eq!(edges, cut_edges(&b));
    }

    /// Positive control for the stock: with no taps left a tap does nothing.
    #[test]
    fn no_taps_left_means_no_crack() {
        let mut g = Game::new(1);
        g.taps_left = 0;
        assert!(!g.act(Action { tool: Tool::Tap, x: 112, y: 60 }));
        g.settle_cracks();
        assert!(cut_edges(&g).is_empty());
    }

    /// A closed ring of cracks cuts a piece free.
    #[test]
    fn a_closed_ring_is_cut_free() {
        let mut g = Game::new(1);
        // Against the crest, so the piece has an open face: a ring wholly
        // inside the wall cuts nothing that could ever leave.
        tips::cut_ring(&mut g, 104, 40, 120, 52);
        tips::find_severed(&mut g);
        assert_eq!(g.counts.pieces_cut, 1, "the ring cut nothing free");
        assert!(g.counts.piece_cells >= 150, "the piece is {} cells", g.counts.piece_cells);
    }
}
