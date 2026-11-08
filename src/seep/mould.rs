//! **The mould, as a graph.** Junction nodes, with tubes as edges drawn into
//! cells -- review-3 §4a's instruction, "built as a graph from the start",
//! and the deliberate opposite of probe 2's agent swarm, so the two moulds
//! are a comparison rather than a duplicate.
//!
//! What it does, in the world's words:
//! - **Tips creep along rock**, walls and roofs included, and climb toward
//!   richer gas. They are fast on damp rock and slow on dry.
//! - **Where a tube touches gas it eats it.** The food flows down the tubes
//!   to the core, and growing tips draw reserve back out along them.
//! - **Tubes that carry flow thicken; idle ones thin and pull back.** This is
//!   the Physarum rule (Tero et al. 2010): solve the flow through the network
//!   as a resistor circuit, then let each tube's thickness relax toward a
//!   function of the flow it carried. Nobody writes "retreat when the job is
//!   done"; a pocket that is eaten away stops feeding its tubes and they go.
//! - **Tips sense gas over a disc, not with forward sensors.** The Jones
//!   (2010) agent's three forward sensors are on `Reports/dead-ends.md`
//!   for anything walking a surface in side view: both lateral sensors land
//!   in open air and their difference is identically zero
//!   (`creature-direction.md` §13b). A tip here scores each free surface
//!   cell beside it by the mean gas in a radius-6 disc round that cell, so
//!   a tip on a wall still feels a pocket above it.
//! - **Fed tubes are wet and stop flame; starving tubes dry and burn like a
//!   fuse.** The fire rule itself lives in `mod.rs`; this file only says
//!   which tube cells are wet and which are dry.

use super::gas::{idx, Air, H, W};

/// Cells a tip lays before it puts down a junction.
const SEGMENT: usize = 5;
/// Reserve spent per cell of growth.
const GROW_COST: f32 = 0.12;
/// Tip speed, cells per tick, on damp and on dry rock.
const FAST: f32 = 0.07;
const SLOW: f32 = 0.008;
/// **Fresh water makes it race** (owner playtest 2026-11-08: "I just kind
/// of click water along the edge and then it kind of slowly grows there").
/// On rock the player has just wetted -- damp above the rock's own -- a tip
/// runs at 12 cells a second, so a stroke is answered while the hand is
/// still on it, and slows to `FAST` as the stroke dries over ~20 s.
const RACE: f32 = 0.2;
/// Fresh damp above which a tip races.
const RACE_FRESH: f32 = 0.35;
/// **The mould reaches for the brush.** For this long after a Damp stroke
/// the tips within `LURE_RANGE` lean toward it, and if none is near it a
/// new tip sets out for it from the nearest rooted junction -- the stroke
/// gets an answer on screen at once instead of waiting for a tip to wander
/// onto the wet.
const LURE_TICKS: u32 = 240;
const LURE_RANGE: i32 = 48;
const LURE_PULL: f32 = 1.4;
const LURE_SPROUT_EVERY: u32 = 40;
pub const MAX_TIPS: usize = 18;
/// The most of a crack's leak a mould can take at the mouth. Less than all
/// of it on purpose: a capped crack still feeds the pocket above it slowly.
const CAP_MAX: f32 = 0.75;
const SPROUT_RESERVE: f32 = 40.0;
const SPROUT_BELOW_TIPS: usize = 5;
/// Per tick: about one sprout every two seconds while it applies.
const SPROUT_CHANCE: f32 = 1.0 / 120.0;
const SPROUT_COST: f32 = 4.0;
/// Fraction of a cell's gas a tube of thickness 1 eats per tick.
const EAT: f32 = 0.012;
/// Reserve gained per unit of gas eaten.
const FOOD_VALUE: f32 = 1.6;
/// The flow solve runs every this many ticks.
pub const SOLVE_EVERY: u32 = 4;
/// Thickness below which an idle tube is gone.
const PRUNE: f32 = 0.12;
/// A new tube cannot thin below this until it is this many ticks old: its
/// tip has not had time to find anything yet.
const YOUNG_TICKS: u32 = 420;
const YOUNG_FLOOR: f32 = 0.55;
/// **The core is never snipped and never gives up** (playtest 2026-11-08:
/// one curious Cut beside the sump at 20 s killed the mould for the whole
/// level on 5 of 5 seeds, and nothing on screen said so). Tubes with a cell
/// this close to the core cannot be cut...
pub const CORE_GUARD: i32 = 6;
/// ...the sump feeds the core this much reserve a tick whatever happens...
const CORE_INCOME: f32 = 0.005;
/// ...and a core with fewer than this many tips sprouts one when it can
/// pay for a few cells, about once a second and a half.
const REGROW_BELOW_TIPS: usize = 2;
const REGROW_CHANCE: f32 = 1.0 / 90.0;
/// Ticks a dead tube's grey husk stays on screen.
pub const HUSK_TICKS: u8 = 240;
/// Flow at which a tube is "fed" and holds water.
pub const FED_WET: f32 = 0.004;
/// Thickness at which a tube holds water whatever it is carrying.
pub const THICK_WET: f32 = 1.5;

pub struct Node {
    pub x: i32,
    pub y: i32,
    pub alive: bool,
    p: f32,
    inject: f32,
}

pub struct Edge {
    pub a: usize,
    pub b: usize,
    pub cells: Vec<(i32, i32)>,
    pub d: f32,
    pub q: f32,
    /// Smoothed |flow|, the "is this tube fed" reading.
    pub fed: f32,
    pub alive: bool,
    pub age: u32,
    /// Connected to the core this solve.
    pub rooted: bool,
}

pub struct Tip {
    pub node: usize,
    pub x: i32,
    pub y: i32,
    dir: (f32, f32),
    pub path: Vec<(i32, i32)>,
    progress: f32,
    pub alive: bool,
}

/// What the mould needs to know about the rock. Built by the game each tick.
pub struct Ground<'a> {
    pub solid: &'a [bool],
    pub water: &'a [bool],
    /// Damp level of each cell, 0..1, rock and air alike.
    pub wet: &'a [f32],
    /// The level's own damp, which the player's dries back to.
    pub floor: &'a [f32],
}

impl Ground<'_> {
    fn inside(x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < W && (y as usize) < H
    }
    pub fn free(&self, x: i32, y: i32) -> bool {
        Self::inside(x, y) && {
            let i = idx(x as usize, y as usize);
            !self.solid[i] && !self.water[i]
        }
    }
    /// Touching rock or water on a side: the mould is a film, it needs
    /// something to lie on.
    pub fn surface(&self, x: i32, y: i32) -> bool {
        [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            !Self::inside(nx, ny) || {
                let i = idx(nx as usize, ny as usize);
                self.solid[i] || self.water[i]
            }
        })
    }
    /// How damp the ground under a cell is.
    /// How much fresher than the rock's own the damp here is: water the
    /// player put down. Read like `damp`, the most of the cell and its four
    /// neighbours.
    pub fn fresh(&self, x: i32, y: i32) -> f32 {
        let mut m: f32 = 0.0;
        for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if Self::inside(nx, ny) {
                let i = idx(nx as usize, ny as usize);
                m = m.max(self.wet[i] - self.floor[i]);
            }
        }
        m
    }
    pub fn damp(&self, x: i32, y: i32) -> f32 {
        let mut m: f32 = 0.0;
        for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if Self::inside(nx, ny) {
                let i = idx(nx as usize, ny as usize);
                m = m.max(self.wet[i]);
                if self.water[i] {
                    m = 1.0;
                }
            }
        }
        m
    }
}

pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct MouldCounts {
    pub eaten: f64,
    pub grown_cells: u64,
    pub tubes_burnt: u64,
    pub tubes_cut: u64,
    pub tubes_faded: u64,
    pub loops: u64,
    /// Tips sent toward a Damp stroke.
    pub lure_sprouts: u64,
}

pub struct Mould {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub tips: Vec<Tip>,
    pub core: usize,
    pub reserve: f32,
    /// Edge id + 1 on each tube cell, 0 elsewhere.
    pub occ: Vec<u32>,
    occ_dirty: bool,
    /// Ticks left on the grey husk a dead tube leaves on each cell: drawn
    /// only, so a cut, burnt or starved tube is seen to die rather than
    /// vanish.
    pub husk: Vec<u8>,
    rng: Rng,
    food_since_solve: f32,
    pub counts: MouldCounts,
    /// The last Damp stroke: where, and the tick it was laid.
    pub lure: Option<(i32, i32, u32)>,
    now: u32,
    last_lure_sprout: u32,
}

impl Mould {
    pub fn new(x: i32, y: i32, reserve: f32, seed: u64) -> Self {
        let mut m = Mould {
            nodes: vec![Node { x, y, alive: true, p: 0.0, inject: 0.0 }],
            edges: Vec::new(),
            tips: Vec::new(),
            core: 0,
            reserve,
            occ: vec![0; W * H],
            occ_dirty: true,
            husk: vec![0; W * H],
            rng: Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1),
            food_since_solve: 0.0,
            counts: MouldCounts::default(),
            lure: None,
            now: 0,
            last_lure_sprout: 0,
        };
        for k in 0..4 {
            let a = k as f32 * std::f32::consts::FRAC_PI_2 + 0.4;
            m.tips.push(Tip { node: 0, x, y, dir: (a.cos(), a.sin()), path: Vec::new(), progress: 0.0, alive: true });
        }
        m
    }

    pub fn tube_at(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x as usize >= W || y as usize >= H {
            return None;
        }
        let o = self.occ[idx(x as usize, y as usize)];
        if o == 0 {
            None
        } else {
            Some(o as usize - 1)
        }
    }

    /// Fed or thick: holds water, and flame will not cross it.
    pub fn is_wet(&self, e: usize, ground: &Ground) -> bool {
        let ed = &self.edges[e];
        if ed.fed > FED_WET || ed.d > THICK_WET {
            return true;
        }
        let (x, y) = ed.cells[ed.cells.len() / 2];
        ground.damp(x, y) > 0.4
    }

    pub fn live_tips(&self) -> usize {
        self.tips.iter().filter(|t| t.alive).count()
    }

    pub fn live_tubes(&self) -> usize {
        self.edges.iter().filter(|e| e.alive).count()
    }

    pub fn tube_cells(&self) -> usize {
        self.edges.iter().filter(|e| e.alive).map(|e| e.cells.len()).sum()
    }

    fn add_node(&mut self, x: i32, y: i32) -> usize {
        self.nodes.push(Node { x, y, alive: true, p: 0.0, inject: 0.0 });
        self.nodes.len() - 1
    }

    fn add_edge(&mut self, a: usize, b: usize, cells: Vec<(i32, i32)>) -> usize {
        self.edges.push(Edge { a, b, cells, d: 0.8, q: 0.0, fed: 0.0, alive: true, age: 0, rooted: true });
        self.occ_dirty = true;
        self.edges.len() - 1
    }

    pub fn remove_edge(&mut self, e: usize) {
        if self.edges[e].alive {
            self.edges[e].alive = false;
            self.occ_dirty = true;
            self.leave_husk(e);
        }
    }

    fn leave_husk(&mut self, e: usize) {
        for &(x, y) in &self.edges[e].cells {
            self.husk[idx(x as usize, y as usize)] = HUSK_TICKS;
        }
    }

    /// Too near the core to snip.
    pub fn guarded(&self, x: i32, y: i32) -> bool {
        let c = &self.nodes[self.core];
        (x - c.x).abs() <= CORE_GUARD && (y - c.y).abs() <= CORE_GUARD
    }

    /// A tube laid by hand from the core, for tests.
    #[cfg(test)]
    pub fn test_tube(&mut self, cells: Vec<(i32, i32)>) -> usize {
        let (x, y) = *cells.last().unwrap();
        let n = self.add_node(x, y);
        let e = self.add_edge(self.core, n, cells);
        self.rebuild_occ();
        e
    }

    fn rebuild_occ(&mut self) {
        if !self.occ_dirty {
            return;
        }
        self.occ.iter_mut().for_each(|o| *o = 0);
        for (k, e) in self.edges.iter().enumerate() {
            if !e.alive {
                continue;
            }
            for &(x, y) in &e.cells {
                self.occ[idx(x as usize, y as usize)] = k as u32 + 1;
            }
        }
        // Nodes with no live tube and no tip are gone (the core never is).
        let mut used = vec![false; self.nodes.len()];
        used[self.core] = true;
        for e in self.edges.iter().filter(|e| e.alive) {
            used[e.a] = true;
            used[e.b] = true;
        }
        for t in self.tips.iter().filter(|t| t.alive) {
            used[t.node] = true;
        }
        for (n, u) in self.nodes.iter_mut().zip(used) {
            n.alive = u;
        }
        self.occ_dirty = false;
    }

    /// **Cut**: snip every tube within `r` of the point, except those that
    /// touch the core (`CORE_GUARD`). Returns how many.
    pub fn cut(&mut self, x: i32, y: i32, r: i32) -> usize {
        let mut hit = Vec::new();
        for (k, e) in self.edges.iter().enumerate() {
            if e.alive && e.cells.iter().any(|&(cx, cy)| (cx - x).abs() <= r && (cy - y).abs() <= r) && !e.cells.iter().any(|&(cx, cy)| self.guarded(cx, cy)) {
                hit.push(k);
            }
        }
        for &k in &hit {
            self.remove_edge(k);
        }
        // A tip whose path runs through the cut dies with it.
        for t in self.tips.iter_mut().filter(|t| t.alive) {
            if (t.x - x).abs() <= r && (t.y - y).abs() <= r {
                t.alive = false;
            }
        }
        self.counts.tubes_cut += hit.len() as u64;
        self.rebuild_occ();
        hit.len()
    }

    /// Eat gas off every tube cell. Returns the gas eaten this tick.
    pub fn eat(&mut self, air: &mut Air) -> f32 {
        let mut total = 0.0;
        for k in 0..self.edges.len() {
            if !self.edges[k].alive {
                continue;
            }
            let rate = EAT * (0.4 + self.edges[k].d.min(2.5));
            let mut got = 0.0;
            for &(x, y) in &self.edges[k].cells {
                let i = idx(x as usize, y as usize);
                let take = air.c[i] * rate;
                air.c[i] -= take;
                got += take;
            }
            let (a, b) = (self.edges[k].a, self.edges[k].b);
            self.nodes[a].inject += 0.5 * got;
            self.nodes[b].inject += 0.5 * got;
            total += got;
        }
        // The tips eat too: they are where the mould meets new gas.
        for t in self.tips.iter().filter(|t| t.alive) {
            let i = idx(t.x as usize, t.y as usize);
            let take = air.c[i] * EAT * 2.0;
            air.c[i] -= take;
            self.nodes[t.node].inject += take;
            total += take;
        }
        self.reserve += total * FOOD_VALUE;
        self.food_since_solve += total;
        self.counts.eaten += total as f64;
        total
    }

    /// Gas taken straight from a crack's mouth before it reaches the air:
    /// a tube sitting on a crack caps it. Returns the fraction intercepted.
    pub fn cap_fraction(&self, x0: i32, x1: i32, y: i32) -> f32 {
        let mut s: f32 = 0.0;
        let mut seen = Vec::new();
        for yy in (y - 3)..=(y + 1) {
            for xx in (x0 - 2)..=(x1 + 2) {
                if let Some(e) = self.tube_at(xx, yy) {
                    if !seen.contains(&e) {
                        seen.push(e);
                        s += 0.25 * self.edges[e].d.min(2.0);
                    }
                }
            }
        }
        s.min(CAP_MAX)
    }

    /// Credit gas the mould took at a crack's mouth.
    pub fn credit_cap(&mut self, x0: i32, x1: i32, y: i32, amount: f32) {
        let mut found = None;
        'o: for yy in (y - 3)..=(y + 1) {
            for xx in (x0 - 2)..=(x1 + 2) {
                if let Some(e) = self.tube_at(xx, yy) {
                    found = Some(e);
                    break 'o;
                }
            }
        }
        if let Some(e) = found {
            let a = self.edges[e].a;
            self.nodes[a].inject += amount;
            self.reserve += amount * FOOD_VALUE;
            self.food_since_solve += amount;
            self.counts.eaten += amount as f64;
        }
    }

    /// Grow, then every `SOLVE_EVERY` ticks solve the flow and adapt.
    /// A Damp stroke at (x, y) this tick: see `LURE_TICKS`.
    pub fn lure_at(&mut self, x: i32, y: i32, tick: u32) {
        self.lure = Some((x, y, tick));
    }

    fn live_lure(&self) -> Option<(i32, i32)> {
        self.lure.filter(|&(_, _, t0)| self.now.saturating_sub(t0) < LURE_TICKS).map(|(x, y, _)| (x, y))
    }

    pub fn step(&mut self, tick: u32, air: &Air, ground: &Ground) {
        self.now = tick;
        self.reserve += CORE_INCOME;
        if tick.is_multiple_of(2) {
            for h in self.husk.iter_mut() {
                *h = h.saturating_sub(1);
            }
        }
        self.grow(air, ground);
        if tick.is_multiple_of(SOLVE_EVERY) {
            self.solve_and_adapt();
        }
        self.rebuild_occ();
    }

    fn grow(&mut self, air: &Air, ground: &Ground) {
        let lure = self.live_lure();
        let n_tips = self.tips.len();
        let mut new_tips = Vec::new();
        for t in 0..n_tips {
            if !self.tips[t].alive {
                continue;
            }
            let (tx, ty) = (self.tips[t].x, self.tips[t].y);
            // A tip the brush is calling moves at damp-rock speed even over
            // dry rock: the first try left a tip 10 cells from a stroke
            // crawling at `SLOW`, and it blocked the new tip that would
            // have answered (3 cells in 6 s).
            let lured = lure.is_some_and(|(lx, ly)| (lx - tx).abs().max((ly - ty).abs()) <= LURE_RANGE);
            let speed = if ground.fresh(tx, ty) > RACE_FRESH {
                RACE
            } else if lured || ground.damp(tx, ty) > 0.3 {
                FAST
            } else {
                SLOW
            };
            if self.reserve < GROW_COST {
                continue;
            }
            self.tips[t].progress += speed;
            while self.tips[t].progress >= 1.0 && self.tips[t].alive {
                self.tips[t].progress -= 1.0;
                self.step_tip(t, air, ground, &mut new_tips);
            }
        }
        // **A well-fed mould keeps exploring.** Tips die when they hit a dead
        // end or close a loop, and the first look at a damp road (2026-11-07)
        // showed the network capping the crack and then going still with one
        // tip left and a reserve of 1,500 it had no way to spend. So while
        // there is reserve and few tips, a new one sprouts from a fed
        // junction -- which is where the mould has been finding food.
        if self.reserve > SPROUT_RESERVE && self.live_tips() < SPROUT_BELOW_TIPS && self.rng.f() < SPROUT_CHANCE {
            let fed: Vec<usize> = self
                .edges
                .iter()
                .filter(|e| e.alive && e.rooted && e.fed > FED_WET)
                .map(|e| if self.rng_peek_bool(e.a) { e.a } else { e.b })
                .collect();
            let pool: Vec<usize> = if fed.is_empty() { vec![self.core] } else { fed };
            let n = pool[(self.rng.next_u64() % pool.len() as u64) as usize];
            let a = self.rng.f() * std::f32::consts::TAU;
            let (x, y) = (self.nodes[n].x, self.nodes[n].y);
            new_tips.push(Tip { node: n, x, y, dir: (a.cos(), a.sin()), path: Vec::new(), progress: 0.0, alive: true });
            self.reserve -= SPROUT_COST;
        }
        // A fresh stroke with no tip near it: send one from the nearest
        // rooted junction in range.
        if let Some((lx, ly)) = self.live_lure() {
            // Any live tip in range is already running to it (see `lured`
            // in the speed rule), so only a stroke no tip can answer sends
            // one. The first rule ("none within 12") sent seven tips in six
            // seconds at a roof stroke the floor tips could not reach yet,
            // 28 reserve for nothing.
            let near = self.tips.iter().chain(new_tips.iter()).any(|t| t.alive && (t.x - lx).abs().max((t.y - ly).abs()) <= LURE_RANGE);
            if !near && self.now >= self.last_lure_sprout + LURE_SPROUT_EVERY && self.reserve > SPROUT_COST + 10.0 * GROW_COST {
                let mut best: Option<(usize, i32)> = None;
                for e in self.edges.iter().filter(|e| e.alive && e.rooted) {
                    for n in [e.a, e.b] {
                        let d = (self.nodes[n].x - lx).abs().max((self.nodes[n].y - ly).abs());
                        if d <= LURE_RANGE && best.is_none_or(|(_, bd)| d < bd) {
                            best = Some((n, d));
                        }
                    }
                }
                if best.is_none() && (self.nodes[self.core].x - lx).abs().max((self.nodes[self.core].y - ly).abs()) <= LURE_RANGE {
                    best = Some((self.core, 0));
                }
                if let Some((n, _)) = best {
                    let (x, y) = (self.nodes[n].x, self.nodes[n].y);
                    let (dx, dy) = ((lx - x) as f32, (ly - y) as f32);
                    let l = (dx * dx + dy * dy).sqrt().max(1.0);
                    new_tips.push(Tip { node: n, x, y, dir: (dx / l, dy / l), path: Vec::new(), progress: 0.0, alive: true });
                    self.reserve -= SPROUT_COST;
                    self.last_lure_sprout = self.now;
                    self.counts.lure_sprouts += 1;
                }
            }
        }
        // The core grows back out after a snip or a burn has taken
        // everything: see `CORE_GUARD`.
        // It sprouts from a free end of what is still rooted, not from the
        // core: the core sits in the sump boxed in by its own tubes and the
        // water, and a tip put there died on its first step, every time
        // (first try, 2026-11-08: 40 s after a mould was cut back to the
        // core it had regrown one tube, its reserve spent on dead sprouts).
        if self.live_tips() + new_tips.len() < REGROW_BELOW_TIPS && self.reserve > SPROUT_COST + 20.0 * GROW_COST && self.rng.f() < REGROW_CHANCE {
            let mut degree = vec![0u32; self.nodes.len()];
            for e in self.edges.iter().filter(|e| e.alive && e.rooted) {
                degree[e.a] += 1;
                degree[e.b] += 1;
            }
            let ends: Vec<usize> = (0..self.nodes.len()).filter(|&n| n != self.core && degree[n] == 1).collect();
            let n = if ends.is_empty() { self.core } else { ends[(self.rng.next_u64() % ends.len() as u64) as usize] };
            let a = self.rng.f() * std::f32::consts::TAU;
            let (x, y) = (self.nodes[n].x, self.nodes[n].y);
            new_tips.push(Tip { node: n, x, y, dir: (a.cos(), a.sin()), path: Vec::new(), progress: 0.0, alive: true });
            self.reserve -= SPROUT_COST;
        }
        let live = self.live_tips();
        for nt in new_tips.into_iter().take(MAX_TIPS.saturating_sub(live)) {
            self.tips.push(nt);
        }
        // Compact the dead.
        if self.tips.len() > 64 {
            self.tips.retain(|t| t.alive);
        }
    }

    /// A deterministic coin per node, so picking an end of a fed tube does
    /// not consume the shared generator inside an iterator.
    fn rng_peek_bool(&self, n: usize) -> bool {
        (n as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(self.rng.0) >> 63 == 1
    }

    fn step_tip(&mut self, t: usize, air: &Air, ground: &Ground, new_tips: &mut Vec<Tip>) {
        let (x, y) = (self.tips[t].x, self.tips[t].y);
        let here = air.mean_near(x, y, 6);
        let dir = self.tips[t].dir;
        let mut best: Option<((i32, i32), f32)> = None;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if !ground.free(nx, ny) || !ground.surface(nx, ny) {
                continue;
            }
            if self.tips[t].path.iter().rev().take(8).any(|&p| p == (nx, ny)) {
                continue;
            }
            let own_node = self.tips[t].node;
            // **A tip cannot join a tube within 3 cells of the junction it
            // left.** Traced 2026-11-08: a tip climbing the sump wall
            // branched at the roof corner, the two tips stood side by side,
            // and each "met another tube" -- its twin's -- on the next step,
            // closed a one-cell loop and died. Every led mould stopped dead
            // at that corner.
            let (ox, oy) = (self.nodes[own_node].x, self.nodes[own_node].y);
            let near_home = (nx - ox).abs() <= 3 && (ny - oy).abs() <= 3;
            let other = self.tube_at(nx, ny).filter(|&e| self.edges[e].a != own_node && self.edges[e].b != own_node && !near_home);
            if self.tube_at(nx, ny).is_some() && other.is_none() {
                continue;
            }
            let len = ((dx * dx + dy * dy) as f32).sqrt();
            let gas = (air.mean_near(nx, ny, 6) - here) * 900.0 + air.mean_near(nx, ny, 6) * 6.0;
            let persist = 0.9 * (dir.0 * dx as f32 + dir.1 * dy as f32) / len;
            let damp = 1.6 * ground.damp(nx, ny);
            let lure = match self.live_lure() {
                Some((lx, ly)) if (lx - x).abs().max((ly - y).abs()) <= LURE_RANGE && (lx, ly) != (x, y) => {
                    let (ux, uy) = ((lx - x) as f32, (ly - y) as f32);
                    let ul = (ux * ux + uy * uy).sqrt();
                    LURE_PULL * (ux * dx as f32 + uy * dy as f32) / (ul * len)
                }
                _ => 0.0,
            };
            // Crowding: avoid running beside its own tubes, which is what
            // turns a mould into a fan instead of a smear.
            let mut crowd = 0.0;
            for oy in -2..=2 {
                for ox in -2..=2 {
                    if self.tube_at(nx + ox, ny + oy).is_some() {
                        crowd += 0.12;
                    }
                }
            }
            let noise = 0.7 * self.rng.f();
            let mut score = gas + persist + damp + lure - crowd + noise;
            if other.is_some() {
                // Meeting another tube closes a loop. Worth it, but not so
                // much the tips do nothing else.
                score += 0.4;
            }
            if best.is_none_or(|(_, s)| score > s) {
                best = Some(((nx, ny), score));
            }
        }
        let Some(((nx, ny), _)) = best else {
            self.finish_tip(t);
            return;
        };
        if let Some(e) = self.tube_at(nx, ny) {
            // Anastomosis: join the tube's nearer end, and the tip is done.
            let (a, b) = (self.edges[e].a, self.edges[e].b);
            let da = (self.nodes[a].x - nx).abs() + (self.nodes[a].y - ny).abs();
            let db = (self.nodes[b].x - nx).abs() + (self.nodes[b].y - ny).abs();
            let target = if da <= db { a } else { b };
            let mut path = std::mem::take(&mut self.tips[t].path);
            if !path.is_empty() && target != self.tips[t].node {
                path.retain(|&(px, py)| self.tube_at(px, py).is_none());
                if !path.is_empty() {
                    let from = self.tips[t].node;
                    self.add_edge(from, target, path);
                    self.counts.loops += 1;
                }
            }
            self.tips[t].alive = false;
            return;
        }
        self.reserve -= GROW_COST;
        self.counts.grown_cells += 1;
        let tip = &mut self.tips[t];
        let nd = ((nx - x) as f32, (ny - y) as f32);
        let l = (nd.0 * nd.0 + nd.1 * nd.1).sqrt();
        tip.dir = (0.6 * tip.dir.0 + 0.4 * nd.0 / l, 0.6 * tip.dir.1 + 0.4 * nd.1 / l);
        let dl = (tip.dir.0 * tip.dir.0 + tip.dir.1 * tip.dir.1).sqrt().max(1e-3);
        tip.dir = (tip.dir.0 / dl, tip.dir.1 / dl);
        tip.x = nx;
        tip.y = ny;
        tip.path.push((nx, ny));
        if tip.path.len() >= SEGMENT {
            let path = std::mem::take(&mut self.tips[t].path);
            let from = self.tips[t].node;
            let n = self.add_node(nx, ny);
            self.add_edge(from, n, path);
            self.rebuild_occ();
            self.tips[t].node = n;
            let wet = ground.damp(nx, ny) > 0.3;
            let rich = air.mean_near(nx, ny, 6) > 0.05;
            let p_branch = if wet || rich { 0.3 } else { 0.08 };
            if self.rng.f() < p_branch {
                let s = if self.rng.f() < 0.5 { 1.0 } else { -1.0 };
                let (c, sn) = (1.1f32.cos(), s * 1.1f32.sin());
                let d = self.tips[t].dir;
                new_tips.push(Tip {
                    node: n,
                    x: nx,
                    y: ny,
                    dir: (d.0 * c - d.1 * sn, d.0 * sn + d.1 * c),
                    path: Vec::new(),
                    progress: 0.0,
                    alive: true,
                });
            }
        }
    }

    fn finish_tip(&mut self, t: usize) {
        let path = std::mem::take(&mut self.tips[t].path);
        if path.len() >= 2 {
            let (x, y) = *path.last().unwrap();
            let from = self.tips[t].node;
            let n = self.add_node(x, y);
            self.add_edge(from, n, path);
        }
        self.tips[t].alive = false;
    }

    /// The Physarum step: Kirchhoff flow through the tubes, then each tube
    /// thickens toward the flow it carried and thins without it.
    fn solve_and_adapt(&mut self) {
        // Growing tips draw reserve out to themselves.
        for t in self.tips.iter().filter(|t| t.alive) {
            self.nodes[t.node].inject -= GROW_COST * FAST * SOLVE_EVERY as f32 * 0.5;
        }
        // Which nodes reach the core.
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); self.nodes.len()];
        for (k, e) in self.edges.iter().enumerate() {
            if e.alive {
                adj[e.a].push(k);
                adj[e.b].push(k);
            }
        }
        let mut rooted = vec![false; self.nodes.len()];
        let mut stack = vec![self.core];
        rooted[self.core] = true;
        while let Some(n) = stack.pop() {
            for &k in &adj[n] {
                let m = if self.edges[k].a == n { self.edges[k].b } else { self.edges[k].a };
                if !rooted[m] {
                    rooted[m] = true;
                    stack.push(m);
                }
            }
        }
        let cond = |e: &Edge| e.d / e.cells.len().max(1) as f32;
        for _ in 0..40 {
            for n in 0..self.nodes.len() {
                if n == self.core || !rooted[n] || adj[n].is_empty() {
                    continue;
                }
                let (mut sc, mut sp) = (0.0, 0.0);
                for &k in &adj[n] {
                    let e = &self.edges[k];
                    let m = if e.a == n { e.b } else { e.a };
                    let c = cond(e);
                    sc += c;
                    sp += c * self.nodes[m].p;
                }
                if sc > 0.0 {
                    self.nodes[n].p = (self.nodes[n].inject + sp) / sc;
                }
            }
        }
        self.nodes[self.core].p = 0.0;
        let mut faded = 0;
        for e in self.edges.iter_mut() {
            if !e.alive {
                continue;
            }
            e.age += SOLVE_EVERY;
            e.rooted = rooted[e.a];
            let c = e.d / e.cells.len().max(1) as f32;
            e.q = if e.rooted { c * (self.nodes[e.a].p - self.nodes[e.b].p) } else { 0.0 };
            e.fed = 0.85 * e.fed + 0.15 * e.q.abs();
            let qn = e.q.abs() / 0.004;
            let f = 2.8 * qn.powf(1.4) / (1.0 + qn.powf(1.4));
            let rate = if e.rooted { 0.012 } else { 0.03 };
            e.d += rate * (f - e.d);
            if e.age < YOUNG_TICKS && e.rooted {
                e.d = e.d.max(YOUNG_FLOOR);
            }
            if e.d < PRUNE {
                e.alive = false;
                faded += 1;
                for &(x, y) in &e.cells {
                    self.husk[idx(x as usize, y as usize)] = HUSK_TICKS;
                }
            }
        }
        if faded > 0 {
            self.counts.tubes_faded += faded;
            self.occ_dirty = true;
        }
        for n in self.nodes.iter_mut() {
            n.inject = 0.0;
        }
        self.food_since_solve = 0.0;
    }
}
