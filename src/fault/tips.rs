//! Crack tips: what runs when you tap.
//!
//! A tip sits on a **corner** of the cell grid and moves one cell-edge at a
//! time, writing the engine's own edge crack as it goes
//! (`Cell::with_crack_down` for an edge run sideways, `with_crack_right`
//! for one run up or down -- the edge cut is the one the tip travels
//! *along*, which is the orientation trap `rigid::score_cracks` records).
//!
//! # Where a tip goes
//!
//! Only along **joints** of the ice's grain (`fracture_field::domain`: an
//! edge between two cells of different Worley domains), and at a junction
//! it takes the joint with **the most tension** across it. Inside a grain
//! cell, where there is no joint, it runs straight until it meets one. So a
//! crack is a chain of straight segments that turns where the grain turns,
//! and leans toward the strained part of the wall -- which is the thing a
//! player can learn to predict, and the shimmer A/B tests whether they can
//! see it coming.
//!
//! # How far it goes
//!
//! A tip carries **push**. Every edge costs one; tension pays some back. In
//! slack ice a tap's push runs out after a short crack, and in strained ice
//! (deep, against the lake) the tension pays for the whole run and the
//! crack goes until it hits something. A tip **forks** at a junction when
//! the ice there is strained and it has push to share.
//!
//! # What stops it
//!
//! - **A drilled hole**: the tip reaches a corner touching one and stops
//!   dead. That is the real crack-arrest trick (a stop-drill), and the
//!   whole reason Drill exists.
//! - **A free face** -- air, water, the wall's edge, another gap.
//! - **An existing crack**: it joins it. This is how loops close.
//! - Running out of push. Such a tip is remembered: Frost can wake it if
//!   water has soaked down the crack to it.

use crate::sim::fracture_field;
use crate::sim::material;

use super::level::{self, H, W};
use super::Game;

/// Drill hole radius, in cells.
pub const DRILL_R: i32 = 2;
/// Grain size of the ice, in cells: the Worley pitch the joints are cut at.
/// Ten makes the wall (36 across) about four grains thick, so a slab is a
/// few grains -- big enough to read as a slab on a phone.
pub const GRAIN: f32 = 10.0;
/// Push a tap gives each of the two tips it starts.
pub const TAP_PUSH: f32 = 22.0;
/// Push Frost gives a woken tip, plus `FROST_PER_WET` for every wet edge in
/// the crack behind it -- more water, more ice, more wedge.
pub const FROST_PUSH: f32 = 14.0;
pub const FROST_PER_WET: f32 = 0.25;
/// Tension at which the push a tip gets back equals what an edge costs:
/// above it a crack runs on its own.
pub const SELF_RUNNING: f32 = 1.0;
/// Tension above which a tip at a junction forks.
pub const FORK_TENSION: f32 = 0.75;
/// Push a tip must still have to fork, so a dying crack does not split.
pub const FORK_MIN_PUSH: f32 = 8.0;
/// Edges a tip runs per frame. Three makes a 60-edge crack take a third of
/// a second: visibly running, and finished well inside the one second the
/// bar sets for a visible consequence.
pub const EDGES_PER_FRAME: u32 = 3;
/// What the burst gives every tip it starts: enough to cross the wall.
/// How deep the burst takes the wall's top off, in rows.
const BURST_ROWS: i32 = 28;
/// A tap more than this many rows under the lake's surface starts a deep
/// crack (`Tip::deep`). The playtest's ask (2026-11-08): random taps hit
/// every depth and a lucky deep cut drained 76-80% of the lake, so a deep
/// cut has to be what it is in a real dam -- the dangerous one.
pub const DEEP_ROWS: i32 = 3;
/// A hole draws in any tip within this many cells of its centre: the tip
/// leaves the grain and runs straight at it, and does not tire on the way.
/// "Drill two, tap between" is then a plan a player can make on purpose.
pub const HOLE_PULL: i32 = 7;
/// Fastest a block peels off a sliding slab, cells a frame.
const PEEL_VX: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    E,
    W,
    S,
    N,
}

impl Dir {
    const ALL: [Dir; 4] = [Dir::E, Dir::S, Dir::W, Dir::N];
    fn rev(self) -> Dir {
        match self {
            Dir::E => Dir::W,
            Dir::W => Dir::E,
            Dir::S => Dir::N,
            Dir::N => Dir::S,
        }
    }
    fn step(self, v: (i32, i32)) -> (i32, i32) {
        match self {
            Dir::E => (v.0 + 1, v.1),
            Dir::W => (v.0 - 1, v.1),
            Dir::S => (v.0, v.1 + 1),
            Dir::N => (v.0, v.1 - 1),
        }
    }
}

/// The edge leaving corner `v` in `d`: the two cells it separates, and
/// whether it is a horizontal edge (owned as the upper cell's `crack_down`)
/// or a vertical one (the left cell's `crack_right`). The first cell is
/// always the owner.
pub fn edge(v: (i32, i32), d: Dir) -> ((i32, i32), (i32, i32), bool) {
    let (x, y) = v;
    match d {
        Dir::E => ((x, y - 1), (x, y), true),
        Dir::W => ((x - 1, y - 1), (x - 1, y), true),
        Dir::S => ((x - 1, y), (x, y), false),
        Dir::N => ((x - 1, y - 1), (x, y - 1), false),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tip {
    pub v: (i32, i32),
    /// The direction it arrived in, so it does not turn straight back.
    pub heading: Option<Dir>,
    pub push: f32,
    /// Edges this tip has cut. A tip that cut none is not a crack run.
    pub run: u32,
    /// Started under the lake's surface: the water in the crack drives it,
    /// so it loses no push while it is below the surface (`DEEP_ROWS`).
    pub deep: bool,
    /// The hole this half of a tap was aimed at: it runs straight on at it
    /// until it gets there or can get no closer. Without it a half started
    /// eight cells off wandered back onto the grain before `HOLE_PULL`
    /// caught it, and the planner's stitch across wall 2 curled up short of
    /// the far hole.
    pub aim: Option<(i32, i32)>,
}

#[derive(Clone)]
pub struct Cracks {
    pub tips: Vec<Tip>,
    /// Corners where a tip stopped against something -- a face, a hole or
    /// another crack. Only there can a ring of cracks have just closed, so
    /// only there is the severance check run.
    pub closures: Vec<(i32, i32)>,
    /// Pieces cut off by a closed ring, waiting for the lake to reach them.
    pub severed: Vec<Piece>,
    /// Pieces the lake is shoving along the crack under them.
    pub slides: Vec<Slide>,
    /// Tips that ran out of push, where Frost can wake them.
    pub ends: Vec<(i32, i32, Option<Dir>)>,
    pub hole: Vec<bool>,
    /// Centres of the holes drilled, for the pull (`HOLE_PULL`).
    pub holes: Vec<(i32, i32)>,
    /// Wet edges, as the owner cell's right/down edge.
    pub wet_right: Vec<bool>,
    pub wet_down: Vec<bool>,
    /// Frame each tip-cut edge was cut, for the flash as it runs.
    pub cut_right: Vec<u64>,
    pub cut_down: Vec<u64>,
    /// Lumps `support_sweep` has handed to the body code, by the cells they
    /// filled, and the frame. One the body code wrote back where it was --
    /// it judged it standing, or floating on the pool -- is left alone for
    /// `RECHECK_FRAMES` rather than handed over again every sweep (seen:
    /// one ice floe re-dropped 177 times in a game).
    pub dropped: std::collections::HashMap<u64, u64>,
}

#[inline]
fn idx(x: i32, y: i32) -> usize {
    (y * W + x) as usize
}

fn inside(x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < W && y < H
}

impl Cracks {
    pub fn new() -> Self {
        let n = (W * H) as usize;
        Cracks {
            tips: Vec::new(),
            closures: Vec::new(),
            severed: Vec::new(),
            slides: Vec::new(),
            ends: Vec::new(),
            hole: vec![false; n],
            holes: Vec::new(),
            wet_right: vec![false; n],
            wet_down: vec![false; n],
            cut_right: vec![0; n],
            cut_down: vec![0; n],
            dropped: std::collections::HashMap::new(),
        }
    }

    pub fn mark_hole(&mut self, x: i32, y: i32) {
        if inside(x, y) {
            self.hole[idx(x, y)] = true;
        }
    }

    pub fn is_hole(&self, x: i32, y: i32) -> bool {
        inside(x, y) && self.hole[idx(x, y)]
    }

    fn wet(&self, owner: (i32, i32), horizontal: bool) -> bool {
        if !inside(owner.0, owner.1) {
            return false;
        }
        if horizontal {
            self.wet_down[idx(owner.0, owner.1)]
        } else {
            self.wet_right[idx(owner.0, owner.1)]
        }
    }

    fn set_wet(&mut self, owner: (i32, i32), horizontal: bool) -> bool {
        if !inside(owner.0, owner.1) {
            return false;
        }
        let slot = if horizontal { &mut self.wet_down[idx(owner.0, owner.1)] } else { &mut self.wet_right[idx(owner.0, owner.1)] };
        let fresh = !*slot;
        *slot = true;
        fresh
    }

    pub fn wet_count(&self) -> usize {
        self.wet_right.iter().filter(|&&w| w).count() + self.wet_down.iter().filter(|&&w| w).count()
    }
}

impl Default for Cracks {
    fn default() -> Self {
        Self::new()
    }
}

fn is_cracked(g: &Game, owner: (i32, i32), horizontal: bool) -> bool {
    let c = g.world.get(owner.0, owner.1);
    if horizontal {
        c.crack_down()
    } else {
        c.crack_right()
    }
}

/// Can a crack run along this edge at all: ice on both sides, not yet cut.
fn crackable(g: &Game, v: (i32, i32), d: Dir) -> bool {
    let (a, b, h) = edge(v, d);
    g.is_ice(a.0, a.1) && g.is_ice(b.0, b.1) && !is_cracked(g, a, h)
}

fn is_joint(g: &Game, v: (i32, i32), d: Dir) -> bool {
    let (a, b, _) = edge(v, d);
    fracture_field::domain(g.world.seed, a.0, a.1, GRAIN) != fracture_field::domain(g.world.seed, b.0, b.1, GRAIN)
}

/// The four cells round a corner.
fn around(v: (i32, i32)) -> [(i32, i32); 4] {
    let (x, y) = v;
    [(x - 1, y - 1), (x, y - 1), (x - 1, y), (x, y)]
}

/// Tension as a tip feels it, in units of `SELF_RUNNING`.
fn strain(g: &Game, v: (i32, i32), d: Dir) -> f32 {
    let (a, b, _) = edge(v, d);
    g.tension.across(a, b) / g.tension_unit()
}

/// The moves open to a tip at `v`, best first.
fn choices(g: &Game, v: (i32, i32), heading: Option<Dir>) -> Vec<(Dir, f32)> {
    let mut joints = Vec::new();
    for d in Dir::ALL {
        if Some(d.rev()) == heading || !crackable(g, v, d) || !is_joint(g, v, d) {
            continue;
        }
        // A small preference for going on straight breaks ties along a
        // staircase joint, so a diagonal grain boundary is followed rather
        // than bounced along.
        let straight = if Some(d) == heading { 0.02 } else { 0.0 };
        joints.push((d, strain(g, v, d) + straight));
    }
    if joints.is_empty() {
        // Inside a grain cell: straight on, or nowhere.
        if let Some(h) = heading {
            if crackable(g, v, h) {
                return vec![(h, strain(g, v, h))];
            }
        }
        return Vec::new();
    }
    joints.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    joints
}

fn cut(g: &mut Game, v: (i32, i32), d: Dir) {
    let (a, _, h) = edge(v, d);
    let c = g.world.get(a.0, a.1);
    let scored = if h { c.with_crack_down(true) } else { c.with_crack_right(true) };
    g.world.set(a.0, a.1, scored);
    let frame = g.world.frame.max(1);
    if h {
        g.tips.cut_down[idx(a.0, a.1)] = frame;
    } else {
        g.tips.cut_right[idx(a.0, a.1)] = frame;
    }
    // **No structural check is scheduled for a crack.** It was, the way a
    // hammer blow does it, and the load model then did with every closed
    // ring what it does with a cut-off piece of rock: judged it unsupported
    // and broke it up in place, where the bodies landed on the ice they came
    // off and welded back into a jagged lump with no cracks in it (seen at
    // seed 1: the notch the planned player cut was a lump on the crest
    // before the lake ever reached it, and the slide had nothing to shove).
    // What a crack does here is decided by `find_severed`/`release_ready`,
    // and the shipped code takes over at the release.
    g.counts.edges_cracked += 1;
}

/// The joint corner a tap at `(x, y)` would start from: the nearest one,
/// within six cells, with at least two joints out of it to run along.
pub fn tap_corner(g: &Game, x: i32, y: i32) -> Option<(i32, i32)> {
    let mut best: Option<((i32, i32), i32)> = None;
    for dy in -6..=6 {
        for dx in -6..=6 {
            let v = (x + dx, y + dy);
            let d2 = dx * dx + dy * dy;
            if d2 > 36 || best.is_some_and(|(_, b)| b <= d2) {
                continue;
            }
            if Dir::ALL.iter().filter(|&&d| crackable(g, v, d) && is_joint(g, v, d)).count() >= 2 {
                best = Some((v, d2));
            }
        }
    }
    best.map(|(v, _)| v)
}

/// The two ways a tap at corner `v` splits the ice: the two joints out of
/// it with the most tension, which are usually the two halves of one grain
/// boundary.
pub fn tap_dirs(g: &Game, v: (i32, i32)) -> Vec<Dir> {
    tap_aims(g, v).into_iter().map(|(d, _)| d).collect()
}

/// `tap_dirs`, with the hole each half is aimed at.
fn tap_aims(g: &Game, v: (i32, i32)) -> Vec<(Dir, Option<(i32, i32)>)> {
    // Holes in reach come first: one half of the crack towards each of the
    // two nearest, so a tap between two holes stitches them together.
    let mut out: Vec<(Dir, Option<(i32, i32)>)> = Vec::new();
    let mut near: Vec<(i32, (i32, i32))> = g.tips.holes.iter().map(|&h| (dist2(v, h), h)).filter(|&(d, _)| d <= (HOLE_PULL * 3 / 2).pow(2)).collect();
    near.sort();
    for (_, h) in near.into_iter().take(2) {
        if let Some(d) = towards(g, v, None, h) {
            if !out.iter().any(|o| o.0 == d) {
                out.push((d, Some(h)));
            }
        }
    }
    for (d, _) in choices(g, v, None) {
        if out.len() >= 2 {
            break;
        }
        if !out.iter().any(|o| o.0 == d) {
            out.push((d, None));
        }
    }
    out
}

fn dist2(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).pow(2) + (a.1 - b.1).pow(2)
}

/// The open edge out of `v` that most closes the distance to `h`.
fn towards(g: &Game, v: (i32, i32), heading: Option<Dir>, h: (i32, i32)) -> Option<Dir> {
    Dir::ALL
        .into_iter()
        .filter(|&d| Some(d.rev()) != heading && crackable(g, v, d))
        .map(|d| (dist2(d.step(v), h), d))
        .filter(|&(dd, _)| dd < dist2(v, h))
        .min_by_key(|&(dd, d)| (dd, d as u8))
        .map(|(_, d)| d)
}

/// A hole within `HOLE_PULL` of this corner, and the edge towards it.
fn pulled(g: &Game, v: (i32, i32), heading: Option<Dir>) -> Option<Dir> {
    let h = g.tips.holes.iter().copied().filter(|&h| dist2(v, h) <= HOLE_PULL * HOLE_PULL).min_by_key(|&h| dist2(v, h))?;
    towards(g, v, heading, h)
}

/// Start a crack at the joint corner nearest `(x, y)`. Returns tips started.
pub fn tap(g: &mut Game, x: i32, y: i32) -> u32 {
    let Some(v) = tap_corner(g, x, y) else { return 0 };
    let deep = v.1 > g.level.surface + DEEP_ROWS;
    if deep {
        g.counts.deep_taps += 1;
    }
    let mut n = 0;
    for (d, aim) in tap_aims(g, v) {
        // Each tip gets its first edge now, in its own direction, so the
        // two halves leave the corner rather than both taking the best one.
        cut(g, v, d);
        g.tips.tips.push(Tip { v: d.step(v), heading: Some(d), push: TAP_PUSH, run: 1, deep, aim });
        n += 1;
    }
    g.counts.cracks_run += n;
    n
}

/// Move every live tip. Called once a frame.
pub fn advance(g: &mut Game) {
    for _ in 0..EDGES_PER_FRAME {
        let mut live = std::mem::take(&mut g.tips.tips);
        let mut next = Vec::with_capacity(live.len());
        for mut tip in live.drain(..) {
            if let Some(stopped) = stop_reason(g, tip.v, tip.heading) {
                match stopped {
                    Stop::Hole => g.counts.stopped_by_hole += 1,
                    Stop::Face => g.counts.stopped_at_face += 1,
                    Stop::Joined => g.counts.joined += 1,
                }
                g.tips.closures.push(tip.v);
                continue;
            }
            if tip.push <= 0.0 {
                g.counts.died += 1;
                g.tips.ends.push((tip.v.0, tip.v.1, tip.heading));
                continue;
            }
            // Drawn into a hole: straight at it, no fork, no tiring.
            let aimed = tip.aim.and_then(|h| towards(g, tip.v, tip.heading, h));
            if aimed.is_none() {
                tip.aim = None;
            }
            if let Some(d) = aimed.or_else(|| pulled(g, tip.v, tip.heading)) {
                cut(g, tip.v, d);
                tip.v = d.step(tip.v);
                tip.heading = Some(d);
                tip.run += 1;
                next.push(tip);
                continue;
            }
            let c = choices(g, tip.v, tip.heading);
            let Some(&(d, s)) = c.first() else {
                g.counts.died += 1;
                g.tips.ends.push((tip.v.0, tip.v.1, tip.heading));
                continue;
            };
            // Fork: a strained junction with push to share.
            if c.len() >= 2 && s >= FORK_TENSION && tip.push >= FORK_MIN_PUSH {
                let (d2, _) = c[1];
                let share = tip.push * 0.5;
                tip.push = share;
                cut(g, tip.v, d2);
                g.counts.forks += 1;
                g.counts.cracks_run += 1;
                next.push(Tip { v: d2.step(tip.v), heading: Some(d2), push: share, run: 1, deep: tip.deep, aim: None });
            }
            cut(g, tip.v, d);
            let gain = s.min(3.0) / SELF_RUNNING - 1.0;
            // A deep crack under the water does not tire.
            tip.push += if tip.deep && tip.v.1 > g.level.surface { gain.max(0.0) } else { gain };
            tip.v = d.step(tip.v);
            tip.heading = Some(d);
            tip.run += 1;
            next.push(tip);
        }
        g.tips.tips = next;
        if g.tips.tips.is_empty() {
            break;
        }
    }
}

enum Stop {
    Hole,
    Face,
    Joined,
}

fn stop_reason(g: &Game, v: (i32, i32), heading: Option<Dir>) -> Option<Stop> {
    let cells = around(v);
    if cells.iter().any(|&(x, y)| g.tips.is_hole(x, y)) {
        return Some(Stop::Hole);
    }
    if cells.iter().any(|&(x, y)| !g.is_ice(x, y)) {
        return Some(Stop::Face);
    }
    // Any cracked edge at this corner other than the one just cut.
    for d in Dir::ALL {
        if Some(d.rev()) == heading {
            continue;
        }
        let (a, _, h) = edge(v, d);
        if is_cracked(g, a, h) {
            return Some(Stop::Joined);
        }
    }
    None
}

/// Meltwater soaks into cracks. Any cracked edge of an ice cell that has
/// water beside it gets wet, and wetness creeps one edge a frame along the
/// crack network -- so water poured at one end of a crack finds its way to
/// the far end, and into every hairline joined to it.
///
/// The lake itself does not soak the wall: only water on or past the
/// upstream face, or above the lake's surface, counts. Otherwise every crack
/// that touched the lake would be a Frost target whether or not the player
/// had poured anything, and Meltwater would have no job.
pub fn soak(g: &mut Game) {
    let x0 = level::WALL_X0 - 1;
    let x1 = (level::WALL_X1 + 12).min(W - 1);
    let mut fresh = Vec::new();
    for y in 0..level::FLOOR_Y {
        for x in x0..=x1 {
            if g.world.get(x, y).material != material::WATER {
                continue;
            }
            if x < level::WALL_X0 && y >= g.level.surface - 1 {
                continue;
            }
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let (ix, iy) = (x + dx, y + dy);
                if !g.is_ice(ix, iy) {
                    continue;
                }
                // The four edges of the ice cell.
                for (owner, h) in [((ix, iy), true), ((ix, iy - 1), true), ((ix, iy), false), ((ix - 1, iy), false)] {
                    if inside(owner.0, owner.1) && is_cracked(g, owner, h) {
                        fresh.push((owner, h));
                    }
                }
            }
        }
    }
    // Creep: every wet edge wets the cracked edges that share a corner.
    for y in level::WALL_TOP - 2..level::FLOOR_Y {
        for x in x0..=x1 {
            for h in [true, false] {
                if !g.tips.wet((x, y), h) {
                    continue;
                }
                // Corners of this edge.
                let corners = if h { [(x, y + 1), (x + 1, y + 1)] } else { [(x + 1, y), (x + 1, y + 1)] };
                for v in corners {
                    for d in Dir::ALL {
                        let (a, _, eh) = edge(v, d);
                        if inside(a.0, a.1) && is_cracked(g, a, eh) && !g.tips.wet(a, eh) {
                            fresh.push((a, eh));
                        }
                    }
                }
            }
        }
    }
    for (owner, h) in fresh {
        g.tips.set_wet(owner, h);
    }
}

/// Freeze every wet crack. Each tip that died at the end of a wet crack
/// wakes with push for the water behind it; the water is spent.
pub fn frost(g: &mut Game) -> u32 {
    let wet = g.tips.wet_count();
    if wet == 0 {
        return 0;
    }
    let ends = std::mem::take(&mut g.tips.ends);
    let mut kept = Vec::new();
    let mut woke = 0;
    for (x, y, heading) in ends {
        let v = (x, y);
        let wet_here = Dir::ALL.iter().any(|&d| {
            let (a, _, h) = edge(v, d);
            g.tips.wet(a, h)
        });
        if wet_here {
            let push = FROST_PUSH + FROST_PER_WET * wet as f32;
            g.tips.tips.push(Tip { v, heading, push, run: 0, deep: false, aim: None });
            g.flashes.push((x, y, 12, [200, 240, 255, 255]));
            woke += 1;
        } else {
            kept.push((x, y, heading));
        }
    }
    g.tips.ends = kept;
    // The wet edges flash as they freeze, then are dry.
    g.frost_flash = g.world.frame + 20;
    for w in g.tips.wet_right.iter_mut().chain(g.tips.wet_down.iter_mut()) {
        *w = false;
    }
    g.counts.frost_tips += woke;
    g.counts.cracks_run += woke;
    woke
}

/// The lake has reached the top: the wall fails all at once. Its top
/// `BURST_ROWS` rows break off along one line and the lake shoves them over
/// the downstream side as slabs, with the flood behind them.
///
/// It was cracks: strained tips from every grain corner of the upstream
/// face, left to cut what they would. On some walls they cut nothing that
/// could leave, and "THE WALL BURST" was the lake quietly overflowing with
/// nothing falling at all -- the worst outcome looked like the mildest
/// (2026-11-08 playtest).
pub fn burst(g: &mut Game) {
    let floor = level::WALL_TOP + BURST_ROWS;
    let taken: std::collections::HashSet<(i32, i32)> = g.tips.slides.iter().flat_map(|s| s.cells.iter().copied()).chain(g.tips.severed.iter().flat_map(|p| p.cells.iter().copied())).collect();
    g.tips.severed.clear();
    let mut cells = Vec::new();
    for y in 0..floor {
        for x in level::WALL_X0 - 4..=(level::WALL_X1 + 12).min(W - 1) {
            if g.is_ice(x, y) && !taken.contains(&(x, y)) {
                cells.push((x, y));
            }
        }
    }
    for x in level::WALL_X0 - 4..=(level::WALL_X1 + 12).min(W - 1) {
        if g.is_ice(x, floor - 1) && g.is_ice(x, floor) {
            cut(g, (x, floor), Dir::E);
        }
    }
    if cells.is_empty() {
        return;
    }
    g.flashes.push((level::WALL_X0, floor, 20, [255, 255, 255, 255]));
    g.counts.pieces_released += 1;
    g.tips.slides.push(Slide { cells, depth: BURST_ROWS, acc: 0.0, stuck: 0 });
}

/// Cut a closed rectangle of cracks round cells `x0..=x1, y0..=y1`, all at
/// once. Not a player verb: the harness's positive control for the hand-off
/// -- a ring that has certainly closed must free a piece, or nothing the
/// tips do can be expected to.
pub fn cut_ring(g: &mut Game, x0: i32, y0: i32, x1: i32, y1: i32) {
    for x in x0..=x1 {
        cut(g, (x, y0), Dir::E);
        cut(g, (x, y1 + 1), Dir::E);
    }
    for y in y0..=y1 {
        cut(g, (x0, y), Dir::S);
        cut(g, (x1 + 1, y), Dir::S);
    }
    g.tips.closures.push((x0, y0));
}

/// Bound on the severance flood, in cells: more than the whole wall
/// (~3,800 cells), so it bounds work and never decides anything -- a piece
/// is anchored because it touches the bedrock, not because it is big.
///
/// It was 1,600, which made it a gate (`CLAUDE.md`, *a size cap must bound
/// work*): a deep crack that split the wall low cut off everything above
/// it, too big to count, so the wall stood on a crack and a deep tap was
/// as safe as a shallow one.
pub const MAX_PIECE: usize = 5000;
/// Frames between `support_sweep`s.
pub const SUPPORT_EVERY: u64 = 6;
/// How long a lump the body code put back where it was is trusted to be
/// standing before it is checked again.
const RECHECK_FRAMES: u64 = 120;

/// **Has a ring just closed?** From every corner where a tip stopped,
/// flood the ice without crossing a crack. A flood that finishes under
/// `MAX_PIECE` without touching bedrock has found a piece cut off from
/// everything: it is remembered as **severed**, and stays exactly where it
/// is, ring and all, until something moves it (`release_ready`).
///
/// # Why severance is decided here and not left to the load model
///
/// Measured with `faultprobe ring=`: a closed ring cut *inside* the wall is
/// reported `confined` every five frames for as long as you watch (43
/// confined failures, 2,924 cells, nothing released, in 240 frames), and one
/// cut at the wall's top corner `crumbled` to snow in place. Both are the
/// load model being right -- a piece resting on ice is held, whatever the
/// crack under it says (`calve_free_blocks`'s doc records the same finding
/// for rock: "the trigger is severance, not unsupportedness").
///
/// # Why a severed piece waits
///
/// Released at once, a piece resting on the ice below it falls nowhere,
/// lands where it was and is written back into the grid **without its
/// cracks** -- welded, as if it had never been cut (measured: a 192-cell
/// corner piece released as 3 bodies, and 192 cells of uncut ice back in the
/// ring 30 frames later). So it waits, cut, until the lake reaches its back
/// or the ice under it is gone, which is what makes a cut made early into a
/// spillway that opens when the lake rises to it.
pub fn find_severed(g: &mut Game) {
    let corners = std::mem::take(&mut g.tips.closures);
    let mut seen = std::collections::HashSet::new();
    for v in corners {
        for start in around(v) {
            if seen.contains(&start)
                || !g.is_ice(start.0, start.1)
                || g.tips.severed.iter().any(|p| p.cells.contains(&start))
                || g.tips.slides.iter().any(|p| p.cells.contains(&start))
            {
                continue;
            }
            let Some(piece) = flood_piece(g, start, &mut seen) else { continue };
            if !has_open_face(g, &piece) {
                continue;
            }
            g.counts.pieces_cut += 1;
            g.counts.piece_cells += piece.len() as u32;
            g.flashes.push((v.0, v.1, 14, [255, 255, 255, 255]));
            g.tips.severed.push(Piece { cells: piece, at: v });
        }
    }
}

/// A piece the cracks have cut off from the wall, not yet moving.
#[derive(Clone, Debug)]
pub struct Piece {
    pub cells: Vec<(i32, i32)>,
    pub at: (i32, i32),
}

/// Set moving every severed piece that now can: **the lake is at its back**,
/// or **nothing is under it**.
///
/// A wet piece starts to **slide** (`slide`): the lake shoves it along the
/// crack under it until it goes over the crest. A hanging piece **drops**
/// at once. Either way it leaves as **its grain blocks** -- split along the
/// joints inside it, so a cut five grains big comes out as five blocks
/// rather than one rectangle -- through the shipped body code
/// (`rigid::throw_block`). Scraps too small to be a body go to the
/// material's own debris, which with the blocks and the landing's own
/// breakage is the spread the brief asks for: a few blocks, more cobbles,
/// some grit.
///
/// **Thrown, it did not work, at two strengths.** Heaved up and out at
/// 1.6/0.9 cells a frame, the blocks fell back on the stub and welded into a
/// lump that dammed the notch; at 2.2/1.3 they cleared it, but a block
/// falling ninety rows from the crest flies ~100 cells and every gentle
/// spillway landed its slab on the mill. A shove off the crest is what a
/// dam spillway's slab actually does.
pub fn release_ready(g: &mut Game) -> u32 {
    let surface = g.level.surface;
    let pieces = std::mem::take(&mut g.tips.severed);
    let mut kept = Vec::new();
    let mut released = 0;
    for p in pieces {
        // Something else took part of it (the shipped collapse, the burst,
        // a drill): what is left is no longer the piece that was found.
        let live: Vec<(i32, i32)> = p.cells.iter().copied().filter(|&(x, y)| g.is_ice(x, y)).collect();
        if live.len() * 10 < p.cells.len() * 9 {
            continue;
        }
        let wet_depth = live
            .iter()
            .filter(|&&(x, y)| y >= surface - 1 && g.world.get(x - 1, y).material == material::WATER)
            .map(|&(_, y)| y - surface)
            .max();
        let set: std::collections::HashSet<(i32, i32)> = live.iter().copied().collect();
        let resting = live.iter().any(|&(x, y)| g.is_ice(x, y + 1) && !set.contains(&(x, y + 1)));
        if wet_depth.is_none() && resting {
            kept.push(p);
            continue;
        }
        match wet_depth {
            // The lake shoves it along the crack it sits on (`slide`).
            Some(d) => g.tips.slides.push(Slide { cells: live, depth: d.max(0), acc: 0.0, stuck: 0 }),
            // Nothing under it: it drops, as its grain blocks.
            None => drop_blocks(g, &live, 0.0),
        }
        g.world.record_disturbance(p.at.0, p.at.1, 8);
        g.counts.pieces_released += 1;
        released += 1;
    }
    g.tips.severed = kept;
    released
}

/// The row the lake's surface must reach before it pushes this piece: the
/// lowest row where the piece faces the lake across the upstream face.
/// `None` if no part of it faces the lake.
pub fn push_row(g: &Game, p: &Piece) -> Option<i32> {
    p.cells.iter().filter(|&&(x, y)| x - 1 < level::WALL_X0 && !g.is_ice(x - 1, y)).map(|&(_, y)| y).max()
}

/// Split a piece into its grain blocks: 4-connected runs of cells in the
/// same Worley domain.
fn grain_blocks(g: &Game, cells: &[(i32, i32)]) -> Vec<Vec<(i32, i32)>> {
    let set: std::collections::HashSet<(i32, i32)> = cells.iter().copied().collect();
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for &c in cells {
        if !seen.insert(c) {
            continue;
        }
        let home = fracture_field::domain(g.world.seed, c.0, c.1, GRAIN);
        let mut block = vec![c];
        let mut i = 0;
        while i < block.len() {
            let (x, y) = block[i];
            i += 1;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = (x + dx, y + dy);
                if set.contains(&n) && !seen.contains(&n) && fracture_field::domain(g.world.seed, n.0, n.1, GRAIN) == home {
                    seen.insert(n);
                    block.push(n);
                }
            }
        }
        out.push(block);
    }
    out
}

fn flood_piece(g: &Game, start: (i32, i32), seen: &mut std::collections::HashSet<(i32, i32)>) -> Option<Vec<(i32, i32)>> {
    let mut piece = vec![start];
    let mut inside = std::collections::HashSet::new();
    inside.insert(start);
    let mut i = 0;
    let mut anchored = false;
    while i < piece.len() {
        let (x, y) = piece[i];
        i += 1;
        if piece.len() > MAX_PIECE {
            anchored = true;
            break;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if g.world.get(nx, ny).material == material::BEDROCK {
                anchored = true;
            }
            if inside.contains(&(nx, ny)) || !g.is_ice(nx, ny) {
                continue;
            }
            if crate::sim::structural::edge_is_cracked(&g.world, x, y, dx, dy) {
                continue;
            }
            inside.insert((nx, ny));
            piece.push((nx, ny));
        }
    }
    for c in &piece {
        seen.insert(*c);
    }
    (!anchored).then_some(piece)
}

/// Does any cell of the piece border something that is not ice -- air,
/// water, a hole -- so that it has somewhere to go?
fn has_open_face(g: &Game, piece: &[(i32, i32)]) -> bool {
    piece.iter().any(|&(x, y)| [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| !g.is_ice(x + dx, y + dy) && g.world.get(x + dx, y + dy).material != material::BEDROCK))
}

/// A severed piece the lake is shoving downstream along the crack under it.
#[derive(Clone, Debug)]
pub struct Slide {
    pub cells: Vec<(i32, i32)>,
    /// How far under the surface its back was when the lake reached it.
    pub depth: i32,
    acc: f32,
    stuck: u32,
}

/// Cells a frame a piece slides at, by how deep the water at its back is.
/// A notch the lake has only just reached creeps off the crest; one cut
/// free far below the surface is shot out.
fn slide_speed(depth: i32) -> f32 {
    (0.25 + 0.03 * depth as f32).min(1.5)
}

/// **The lake's push, the one force this game adds.** Water does not press
/// on solids in the shipped engine, and a body resting on ice cannot slide
/// (`rigid::throw_block`'s doc), so a cut piece is moved here: a whole cell
/// to the right at a time, into air or water only -- the water it displaces
/// goes into the space behind it, which is how the lake starts to pour
/// through. Its cracks move with it: the edges along its bottom and right
/// are owned by its own cells (`Cell::crack_down`/`crack_right`), so it stays
/// cut from the ice it rides on.
///
/// When more than half of its underside is over air, it has gone over the
/// crest: it is handed to the shipped body code as its grain blocks, moving
/// at the speed it slid, and falls down the downstream face. If it jams for
/// half a second it stays where it is, cut, and the lake holds it there.
pub fn slide(g: &mut Game) {
    let mut slides = std::mem::take(&mut g.tips.slides);
    let mut keep = Vec::new();
    for mut s in slides.drain(..) {
        s.cells.retain(|&(x, y)| g.is_ice(x, y));
        if s.cells.is_empty() {
            continue;
        }
        let speed = slide_speed(s.depth);
        s.acc += speed;
        while s.acc >= 1.0 {
            s.acc -= 1.0;
            if shift_right(g, &mut s.cells, s.stuck) {
                s.stuck = 0;
            } else {
                s.stuck += 1;
                break;
            }
        }
        // Blocks peel off the crest one at a time: any grain block of the
        // slab with no ice anywhere under it -- the crest, or another block
        // still riding on it -- is handed over and falls, and the rest keeps
        // coming. Dropped whole when half of it overhung, the upstream
        // half fell straight back onto the stub and welded into a lump; and
        // dropped whole once its centre passed the last cell of crest under
        // it, ten blocks fell together onto the crest they were still mostly
        // over and set there in a heap. The slab has to be shoved all the way
        // across, a block at a time.
        let blocks = grain_blocks(g, &s.cells);
        let mut falling: Vec<(i32, i32)> = Vec::new();
        for b in &blocks {
            let own: std::collections::HashSet<(i32, i32)> = b.iter().copied().collect();
            // Anything of ice under it in its own columns, all the way down:
            // a block that dropped two rows onto the slab it was part of
            // set there, and was left hanging in the air when the slab
            // slid on from under it.
            let held = b.iter().any(|&(x, y)| (y + 1..level::FLOOR_Y).any(|yy| !own.contains(&(x, yy)) && g.is_ice(x, yy)));
            if !held {
                falling.extend_from_slice(b);
            }
        }
        if !falling.is_empty() {
            // Thrown at no more than `PEEL_VX`: at the slide's own speed (up to
            // 1.5 a frame) a block off the crest flew seventy cells and
            // landed on the mill's roof from a notch that held the lake
            // safely (wall 4, 2026-11-08).
            drop_blocks(g, &falling, speed.min(PEEL_VX));
            let gone: std::collections::HashSet<(i32, i32)> = falling.iter().copied().collect();
            s.cells.retain(|c| !gone.contains(c));
            if s.cells.is_empty() {
                continue;
            }
        }
        if s.stuck > super::FPS / 2 {
            // Jammed: what is left stays where it is, as ice. Handing it all
            // to the body code at rest wrote blocks back into the grid on
            // top of a neighbouring slab that was still moving -- and when
            // that slab slid on, they were left hanging in the air off the
            // crest for the rest of the game. `support_sweep` now finds
            // anything left unheld and lets it fall.
            g.counts.jammed += 1;
            continue;
        }
        keep.push(s);
    }
    g.tips.slides = keep;
}

/// Move a piece one cell by `(dx, dy)`, if everything in front of it is air
/// or liquid -- or ice it can grind through: up to `grind` cells of ice in
/// its way are crushed to the material's debris first, which is what a slab
/// shoved along a stepped crack does to the corners of the step. Returns
/// whether it moved. What it displaces fills the cells it leaves, so water
/// in front ends up behind it rather than being destroyed.
fn shift(g: &mut Game, cells: &mut [(i32, i32)], (dx, dy): (i32, i32), grind: usize) -> bool {
    let set: std::collections::HashSet<(i32, i32)> = cells.iter().copied().collect();
    let mut fronts = Vec::new();
    let mut ground = Vec::new();
    let mut backs = Vec::new();
    for &(x, y) in cells.iter() {
        let to = (x + dx, y + dy);
        if !set.contains(&to) {
            if !g.world.in_bounds(to.0, to.1) {
                return false;
            }
            let m = g.world.get(to.0, to.1).material;
            let k = g.world.materials.kind(m);
            if m == g.ice {
                ground.push(to);
            } else if !(m == material::EMPTY || k == material::MaterialKind::Liquid || k == material::MaterialKind::Gas || k == material::MaterialKind::Powder) {
                return false;
            }
            fronts.push(to);
        }
        if !set.contains(&(x - dx, y - dy)) {
            backs.push((x, y));
        }
    }
    if ground.len() > grind {
        return false;
    }
    let into = g.world.materials.get(g.ice).breaks_into;
    for &(x, y) in &ground {
        if let Some(into) = into {
            g.world.set(x, y, crate::sim::cell::Cell::new(into, (x & 1) as u8));
        }
        g.counts.grit_cells += 1;
    }
    fronts.sort_by_key(|&(x, y)| (y, x));
    backs.sort_by_key(|&(x, y)| (y, x));
    let displaced: Vec<crate::sim::cell::Cell> = fronts.iter().map(|&(x, y)| g.world.get(x, y)).collect();
    let moved: Vec<crate::sim::cell::Cell> = cells.iter().map(|&(x, y)| g.world.get(x, y)).collect();
    for (i, &(x, y)) in backs.iter().enumerate() {
        g.world.set(x, y, displaced.get(i).copied().unwrap_or(crate::sim::cell::Cell::EMPTY));
    }
    for (c, &(x, y)) in moved.iter().zip(cells.iter()) {
        g.world.set(x + dx, y + dy, *c);
    }
    for c in cells.iter_mut() {
        c.0 += dx;
        c.1 += dy;
    }
    true
}

/// Right if it can, grinding off a few corners of the step it rides on; up
/// and right over a ledge too big to grind.
///
/// **The longer it is held, the more it grinds**: four cells at first, and
/// four more for every frame it has been stuck. A slab the lake has hold of
/// does not stop at a step in its crack, it shears it; capped at four, the
/// planned player's first notch on wall 2 jammed in its own gap, closed it,
/// and the wall burst behind it (2026-11-08).
fn shift_right(g: &mut Game, cells: &mut [(i32, i32)], stuck: u32) -> bool {
    shift(g, cells, (1, 0), 4 + 4 * stuck as usize) || shift(g, cells, (1, -1), 1 + stuck as usize)
}

/// Hand a piece to the shipped body code as its grain blocks, each moving
/// downstream at `vx`; scraps too small to be a body go to the material's
/// own debris. What happens next -- the fall, the landing, breaking up on
/// impact, coming to rest -- is all shipped.
fn drop_blocks(g: &mut Game, cells: &[(i32, i32)], vx: f32) {
    let mut blocks = grain_blocks(g, cells);
    // Lowest and furthest downstream first. Bodies step in the order they
    // were made, and one that cannot move for three frames is written back
    // into the grid where it is -- so a block promoted above one that has
    // not yet moved stalls on it and sets solid in mid-air (seen: a slab
    // that toppled as a cluster of blobs hanging off the crest).
    blocks.sort_by_key(|b| (-b.iter().map(|c| c.1).max().unwrap_or(0), -b.iter().map(|c| c.0).max().unwrap_or(0)));
    for b in blocks {
        if b.len() < crate::sim::rigid::MIN_BODY_CELLS {
            let into = g.world.materials.get(g.ice).breaks_into;
            for &(x, y) in &b {
                if let Some(into) = into {
                    g.world.set(x, y, crate::sim::cell::Cell::new(into, (x & 1) as u8));
                    g.world.schedule_structural_check_around(x, y);
                }
            }
            g.counts.grit_cells += b.len() as u32;
            continue;
        }
        let (ox, oy) = b[0];
        if vx > 0.0 {
            // `promote` sets velocity = (cell - origin) * force / dist^2:
            // put the origin ten cells upstream.
            crate::sim::rigid::throw_block(&mut g.world, &b, (ox - 10, oy), vx * 10.0);
        } else {
            crate::sim::rigid::throw_block(&mut g.world, &b, (ox, oy - 1), 0.0);
        }
        g.counts.blocks_thrown += 1;
    }
}

/// Find ice that nothing holds up and let it fall; ride ice resting on a
/// slab along with it.
///
/// **Held** is grown from ice touching anything solid that is not ice (the
/// bedrock under the wall, the mill), and from the pieces `find_severed` is
/// keeping in place. It spreads across any uncut joint, and **upward across
/// a crack** -- a block cut loose on top of the wall is still standing on
/// it -- but never sideways or downward across one: a block hanging off the
/// side of the wall by nothing but a crack face is not held.
///
/// Whatever is left over is either sitting on a slab the lake is shoving --
/// it joins that slab, so it rides off the crest with it instead of being
/// left in the air when the slab moves out from under it (seen: a ten-block
/// lump hanging off the crest for the rest of the game) -- or it falls,
/// as grain blocks, through the shipped body code.
///
/// Every `SUPPORT_EVERY` frames: a flood over the wall, ~0.2 ms.
pub fn support_sweep(g: &mut Game) {
    use std::collections::HashMap;
    let n = (W * H) as usize;
    let mut held = vec![false; n];
    let mut in_slide: HashMap<(i32, i32), usize> = HashMap::new();
    for (k, s) in g.tips.slides.iter().enumerate() {
        for &c in &s.cells {
            in_slide.insert(c, k);
        }
    }
    let mut stack = Vec::new();
    for p in &g.tips.severed {
        for &(x, y) in &p.cells {
            if inside(x, y) && !held[idx(x, y)] {
                held[idx(x, y)] = true;
                stack.push((x, y));
            }
        }
    }
    let ice = g.ice;
    let anchor = |g: &Game, x: i32, y: i32| -> bool {
        if y + 1 >= level::FLOOR_Y {
            return true;
        }
        [(0, 1), (-1, 0), (1, 0), (0, -1)].iter().any(|&(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            if !g.world.in_bounds(nx, ny) {
                return false;
            }
            let m = g.world.get(nx, ny).material;
            m != ice && g.world.materials.kind(m) == material::MaterialKind::Solid
        })
    };
    for y in 0..level::FLOOR_Y {
        for x in 0..W {
            if !inside(x, y) || held[idx(x, y)] || in_slide.contains_key(&(x, y)) || !g.is_ice(x, y) {
                continue;
            }
            if anchor(g, x, y) {
                held[idx(x, y)] = true;
                stack.push((x, y));
            }
        }
    }
    while let Some((x, y)) = stack.pop() {
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let (nx, ny) = (x + dx, y + dy);
            if !inside(nx, ny) || held[idx(nx, ny)] || in_slide.contains_key(&(nx, ny)) || !g.is_ice(nx, ny) {
                continue;
            }
            // Across a crack only upward: what is above rests on this.
            if dy != -1 && crate::sim::structural::edge_is_cracked(&g.world, x, y, dx, dy) {
                continue;
            }
            held[idx(nx, ny)] = true;
            stack.push((nx, ny));
        }
    }
    // What is left, in connected lumps.
    let mut seen = vec![false; n];
    for y in 0..level::FLOOR_Y {
        for x in 0..W {
            if !inside(x, y) || held[idx(x, y)] || seen[idx(x, y)] || in_slide.contains_key(&(x, y)) || !g.is_ice(x, y) {
                continue;
            }
            let mut lump = Vec::new();
            let mut riding: Option<usize> = None;
            seen[idx(x, y)] = true;
            stack.push((x, y));
            while let Some((cx, cy)) = stack.pop() {
                lump.push((cx, cy));
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if !inside(nx, ny) {
                        continue;
                    }
                    if let Some(&k) = in_slide.get(&(nx, ny)) {
                        if dy == 1 {
                            riding = Some(k);
                        }
                        continue;
                    }
                    if held[idx(nx, ny)] || seen[idx(nx, ny)] || !g.is_ice(nx, ny) {
                        continue;
                    }
                    seen[idx(nx, ny)] = true;
                    stack.push((nx, ny));
                }
            }
            match riding {
                Some(k) => g.tips.slides[k].cells.extend(lump),
                None => {
                    use std::hash::{Hash, Hasher};
                    if floating(g, &lump) {
                        continue;
                    }
                    lump.sort_unstable();
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    lump.hash(&mut h);
                    let key = h.finish();
                    let now = g.world.frame;
                    match g.tips.dropped.get(&key).copied() {
                        Some(f) if now < f + RECHECK_FRAMES => continue,
                        Some(_) => {
                            // Handed over before and written back exactly
                            // where it was, with air under it: it is wedged
                            // by its sides, which the body code cannot get
                            // out of (a step blocked on either axis ends the
                            // move -- seen: a 54-cell lump hanging off the
                            // crest for a whole game). Let it down by hand,
                            // until it lands or meets water to float on.
                            let mut moved = 0;
                            for _ in 0..3 {
                                // Free to drop: nothing solid under it, and
                                // not floating -- less than half its
                                // underside on water.
                                let own: std::collections::HashSet<(i32, i32)> = lump.iter().copied().collect();
                                let (mut bottoms, mut wet, mut blocked) = (0, 0, false);
                                for &(x, y) in &lump {
                                    if own.contains(&(x, y + 1)) {
                                        continue;
                                    }
                                    bottoms += 1;
                                    let m = g.world.get(x, y + 1).material;
                                    match g.world.materials.kind(m) {
                                        material::MaterialKind::Empty | material::MaterialKind::Gas => {}
                                        material::MaterialKind::Liquid => wet += 1,
                                        _ if m == material::EMPTY => {}
                                        _ => blocked = true,
                                    }
                                }
                                let free = !blocked && wet * 2 < bottoms;
                                if !free || !shift(g, &mut lump, (0, 1), 0) {
                                    break;
                                }
                                moved += 1;
                            }
                            if moved > 0 {
                                g.counts.unheld += 1;
                                lump.sort_unstable();
                                let mut h = std::collections::hash_map::DefaultHasher::new();
                                lump.hash(&mut h);
                                // Expired at once, so the next sweep lets it
                                // down again.
                                g.tips.dropped.insert(h.finish(), 0);
                            } else {
                                g.tips.dropped.insert(key, now);
                            }
                            continue;
                        }
                        None => {}
                    }
                    g.tips.dropped.insert(key, now);
                    g.counts.unheld += 1;
                    if std::env::var_os("FAULT_DEBUG_UNHELD").is_some() {
                        let (x0, x1) = (lump.iter().map(|c| c.0).min().unwrap(), lump.iter().map(|c| c.0).max().unwrap());
                        let (y0, y1) = (lump.iter().map(|c| c.1).min().unwrap(), lump.iter().map(|c| c.1).max().unwrap());
                        let below: Vec<String> = lump.iter().filter(|c| !lump.contains(&(c.0, c.1 + 1))).take(3).map(|c| format!("{:?}", g.world.materials.get(g.world.get(c.0, c.1 + 1).material).name)).collect();
                        eprintln!("unheld f={} n={} x={}..{} y={}..{} below={:?}", g.world.frame, lump.len(), x0, x1, y0, y1, below);
                    }
                    drop_blocks(g, &lump, 0.0);
                }
            }
        }
    }
}

/// Is this lump of ice floating -- at least half of its underside on water?
/// Ice floats; the sweep leaves a floe alone.
fn floating(g: &Game, lump: &[(i32, i32)]) -> bool {
    let own: std::collections::HashSet<(i32, i32)> = lump.iter().copied().collect();
    let (mut bottoms, mut wet) = (0, 0);
    for &(x, y) in lump {
        if own.contains(&(x, y + 1)) {
            continue;
        }
        bottoms += 1;
        if g.world.materials.kind(g.world.get(x, y + 1).material) == material::MaterialKind::Liquid {
            wet += 1;
        }
    }
    wet * 2 >= bottoms
}
