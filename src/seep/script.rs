//! **Scripted players**, so the probe can say "doing nothing loses" and
//! "a plan beats random play" with counts rather than impressions. They act
//! only through [`Act`], the same door the window uses.
//!
//! - [`Player::Nothing`]: the control. Touches nothing.
//! - [`Player::Random`]: a random tapper. Picks a tool, a place and a
//!   direction at random, uses it for a moment, pauses, repeats. The meters
//!   are what is meant to stop this working (review-3: "a random tapper runs
//!   dry when it matters").
//! - [`Player::Planned`]: review-3 §4a's intended solve for "Two Lamps",
//!   written as rules that read the screen: damp a road up the sump wall
//!   and along the roof into the stall (2026-11-08), then one along the
//!   floor from the sump to crack 1; blow burnable gas away from the upper lamp toward the air shaft;
//!   spark a burnable pocket only when the flame cannot reach a lamp, and
//!   snip any dry tube in its way first; fuses on the shaft; and later damp a
//!   road to the second crack.
//!
//! The 2026-11-08 playtest's strategies, kept so a later level can be
//! replayed against them (review: `/mnt/project-files/game-design/
//! new-game-brainstorm-2026-11-07/playtest-2026-11-08/seep-review.md`):
//! - [`Player::Fresh`] `fresh`: a first-time player. Sparks the biggest pink
//!   pocket every 2 s wherever it is, and tries each other tool once --
//!   including one curious Cut on the first tube by the sump at 20 s.
//!   `freshdamp` is the same player without the snip, who after the first
//!   minute damps just ahead of the mould toward the nearest open crack.
//! - `mouldonly`: the plan with Blow, Spark and Fuse removed.
//! - `ring`: Damp rings round the lamps and nothing else (the firebreak
//!   cheese).

use super::gas::{band, idx, Band, H, W};
use super::mould::Rng;
use super::{Act, Game, Tool, TICKS_PER_SECOND};

pub enum Player {
    Nothing,
    Random(RandomPlayer),
    Planned(Planned),
    Fresh(Fresh),
    Ring,
}

impl Player {
    pub fn name(&self) -> &'static str {
        match self {
            Player::Nothing => "nothing",
            Player::Random(_) => "random",
            Player::Planned(p) if p.no_mould => "gasonly",
            Player::Planned(p) if p.mould_only => "mouldonly",
            Player::Planned(_) => "planned",
            Player::Fresh(f) if f.damps => "freshdamp",
            Player::Fresh(_) => "fresh",
            Player::Ring => "ring",
        }
    }
    pub fn by_name(name: &str, seed: u64) -> Option<Player> {
        match name {
            "nothing" => Some(Player::Nothing),
            "random" => Some(Player::Random(RandomPlayer::new(seed))),
            "planned" => Some(Player::Planned(Planned::default())),
            "gasonly" | "nomould" => Some(Player::Planned(Planned { no_mould: true, ..Planned::default() })),
            "mouldonly" => Some(Player::Planned(Planned { mould_only: true, ..Planned::default() })),
            "fresh" => Some(Player::Fresh(Fresh { damps: false, next_spark: 2 * TICKS_PER_SECOND })),
            "freshdamp" => Some(Player::Fresh(Fresh { damps: true, next_spark: 2 * TICKS_PER_SECOND })),
            "ring" => Some(Player::Ring),
            _ => None,
        }
    }
    pub fn acts(&mut self, g: &Game) -> Vec<Act> {
        match self {
            Player::Nothing => Vec::new(),
            Player::Random(r) => r.acts(g),
            Player::Planned(p) => p.acts(g),
            Player::Fresh(f) => f.acts(g),
            Player::Ring => ring(g),
        }
    }
}

/// A first-time player: see the module doc.
pub struct Fresh {
    damps: bool,
    next_spark: u32,
}

impl Fresh {
    fn acts(&mut self, g: &Game) -> Vec<Act> {
        let t = g.tick;
        let s = TICKS_PER_SECOND;
        let mut out = Vec::new();
        // One try of each tool, early on, the way a new player pokes at a bar.
        if (5 * s..5 * s + 40).contains(&t) {
            let x = 120.0 - (t - 5 * s) as f32;
            out.push(Act::Blow { x, y: 118.0, dx: -1.0, dy: 0.0 });
        }
        if (10 * s..10 * s + 40).contains(&t) {
            let x = 50 + (t - 10 * s) as i32 / 2;
            if let Some(y) = floor_y(g, x, 118, 145) {
                out.push(Act::Damp { x, y });
            }
        }
        if t == 14 * s {
            out.push(Act::Fuse { x: 160, y: 104 });
        }
        if t == 20 * s && !self.damps {
            // One curious snip on the tube nearest the sump.
            let core = &g.mould.nodes[g.mould.core];
            let near = g.mould.edges.iter().filter(|e| e.alive).flat_map(|e| e.cells.iter().copied()).min_by_key(|&(x, y)| {
                let (dx, dy) = (x - core.x, y - core.y);
                // Not on the core itself: a few cells out along a tube.
                ((dx * dx + dy * dy) - 36).abs()
            });
            if let Some((x, y)) = near {
                out.push(Act::Cut { x, y });
            }
        }
        // Spark the biggest pink pocket, wherever it is.
        if t >= self.next_spark {
            self.next_spark = t + 2 * s;
            if let Some((x, y)) = biggest_pink(g) {
                out.push(Act::Spark { x, y });
            }
        }
        // After the first minute, `freshdamp` keeps damping just ahead of
        // the mould toward the nearest open crack.
        if self.damps && t >= 60 * s && t.is_multiple_of(6) && g.meters[2] > 0.06 {
            let target = g
                .cracks
                .iter()
                .filter(|c| t >= c.opens_at && c.cap_ema < 0.6)
                .map(|c| ((c.x0 + c.x1) / 2, c.y))
                .min_by_key(|&(x, y)| {
                    g.mould.tips.iter().filter(|tp| tp.alive).map(|tp| (tp.x - x).abs() + (tp.y - y).abs()).min().unwrap_or(i32::MAX)
                });
            if let Some(target) = target {
                if let Some((x, y)) = lead_spot(g, target) {
                    out.push(Act::Damp { x, y });
                }
            }
        }
        out
    }
}

/// Damp rings round every lamp still alight, and nothing else.
fn ring(g: &Game) -> Vec<Act> {
    let t = g.tick;
    if !t.is_multiple_of(4) || g.meters[2] < 0.06 {
        return Vec::new();
    }
    let lamps: Vec<&super::Lamp> = g.lamps.iter().filter(|l| l.state != super::LampState::Blown).collect();
    if lamps.is_empty() {
        return Vec::new();
    }
    let l = lamps[(t / 4) as usize / 12 % lamps.len()];
    let a = ((t / 4) % 12) as f32 * std::f32::consts::TAU / 12.0;
    let (x, y) = (l.x + (6.0 * a.cos()).round() as i32, l.y + (6.0 * a.sin()).round() as i32);
    vec![Act::Damp { x, y }]
}

/// The biggest connected body of burnable gas, and one cell in it.
fn biggest_pink(g: &Game) -> Option<(i32, i32)> {
    let mut seen = vec![false; W * H];
    let mut best: Option<((i32, i32), usize)> = None;
    for y0 in 0..H as i32 {
        for x0 in 0..W as i32 {
            let i0 = idx(x0 as usize, y0 as usize);
            if seen[i0] || !g.air.is_open(x0, y0) || band(g.air.c[i0]) != Band::Burns {
                continue;
            }
            seen[i0] = true;
            let mut stack = vec![(x0, y0)];
            let mut n = 0;
            let (mut sx, mut sy) = (0i64, 0i64);
            let mut cells = Vec::new();
            while let Some((x, y)) = stack.pop() {
                n += 1;
                sx += x as i64;
                sy += y as i64;
                cells.push((x, y));
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if !g.air.is_open(nx, ny) {
                        continue;
                    }
                    let j = idx(nx as usize, ny as usize);
                    if !seen[j] && band(g.air.c[j]) == Band::Burns {
                        seen[j] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            if best.as_ref().is_none_or(|b| n > b.1) {
                // Click the body's cell nearest its middle.
                let (mx, my) = ((sx / n as i64) as i32, (sy / n as i64) as i32);
                let c = *cells.iter().min_by_key(|&&(x, y)| (x - mx).abs() + (y - my).abs()).unwrap();
                best = Some((c, n));
            }
        }
    }
    best.map(|(c, _)| c)
}

pub struct RandomPlayer {
    rng: Rng,
    gesture: Option<(Tool, f32, f32, f32, f32, u32)>,
    wait: u32,
}

impl RandomPlayer {
    pub fn new(seed: u64) -> Self {
        RandomPlayer { rng: Rng(seed.wrapping_mul(0xA24B_AED4_963E_E407) | 1), gesture: None, wait: 30 }
    }

    fn acts(&mut self, g: &Game) -> Vec<Act> {
        if self.wait > 0 {
            self.wait -= 1;
            return Vec::new();
        }
        if self.gesture.is_none() {
            let tool = Tool::ALL[(self.rng.next_u64() % 5) as usize];
            // A random open cell.
            let (mut x, mut y) = (0, 0);
            for _ in 0..200 {
                x = (self.rng.next_u64() % W as u64) as i32;
                y = (self.rng.next_u64() % H as u64) as i32;
                if g.air.is_open(x, y) {
                    break;
                }
            }
            let a = self.rng.f() * std::f32::consts::TAU;
            let len = 10 + (self.rng.next_u64() % 80) as u32;
            self.gesture = Some((tool, x as f32, y as f32, a.cos(), a.sin(), len));
        }
        let (tool, x, y, dx, dy, left) = self.gesture.unwrap();
        let act = match tool {
            Tool::Blow => Act::Blow { x, y, dx, dy },
            Tool::Spark => Act::Spark { x: x as i32, y: y as i32 },
            Tool::Damp => Act::Damp { x: x as i32, y: y as i32 },
            Tool::Cut => Act::Cut { x: x as i32, y: y as i32 },
            Tool::Fuse => Act::Fuse { x: x as i32, y: y as i32 },
        };
        let tap = matches!(tool, Tool::Spark | Tool::Fuse);
        if tap || left <= 1 {
            self.gesture = None;
            self.wait = 30 + (self.rng.next_u64() % 150) as u32;
        } else {
            // Drags move along their direction a cell a tick.
            let (nx, ny) = (x + dx, y + dy);
            self.gesture = Some((tool, nx, ny, dx, dy, left - 1));
        }
        vec![act]
    }
}

/// Where a scripted stroke is in its road.
#[derive(Default)]
pub struct Planned {
    /// **The control for the probe's question.** The same plan with the
    /// mould left alone -- no Damp, no Cut -- which is Firedamp alone. If
    /// this scores as well as `planned`, the mould is decoration.
    pub no_mould: bool,
    /// The playtest's "clever" arm: the plan with Blow, Spark and Fuse
    /// taken away, so only the mould can save a lamp.
    pub mould_only: bool,
    road0: usize,
    road1: usize,
    waypoint: usize,
    fuses_set: u32,
    blow_x: Option<f32>,
    next_spark_check: u32,
    pub sparks_planned: u32,
    pub cuts_planned: u32,
}

/// The floor cell (the lowest open cell) of a column between two heights.
fn floor_y(g: &Game, x: i32, y0: i32, y1: i32) -> Option<i32> {
    (y0..=y1).rev().find(|&y| g.air.is_open(x, y))
}

fn roof_y(g: &Game, x: i32, y0: i32, y1: i32) -> Option<i32> {
    (y0..=y1).find(|&y| g.air.is_open(x, y))
}

impl Planned {
    fn acts(&mut self, g: &Game) -> Vec<Act> {
        let t = g.tick;
        let mut out = Vec::new();
        // 0. First the stall: a damp road up the sump's wall and along the
        //    roof into the dripping stall, where the mould runs on its own.
        if !self.no_mould && t > TICKS_PER_SECOND / 2 && g.meters[2] > 0.06 {
            let road0 = stall_road(g);
            if self.road0 < road0.len() {
                let (x, y) = road0[self.road0];
                out.push(Act::Damp { x, y });
                self.road0 += 1;
            }
        }
        // 1. Damp a road along the lower floor from the sump to crack 1.
        //    Paced by the meter: a stroke runs until it is nearly dry, then
        //    waits for it to come back.
        let road1: Vec<i32> = (44..=178).step_by(2).collect();
        if !self.no_mould && self.road0 >= stall_road(g).len() && self.road1 < road1.len() && g.meters[2] > 0.06 {
            let x = road1[self.road1];
            if let Some(y) = floor_y(g, x, 118, 145) {
                out.push(Act::Damp { x, y });
            }
            self.road1 += 1;
        }
        // 2. Fuses on the shaft, where overflow passes before it reaches the
        //    upper lamp.
        if !self.mould_only && t == 4 * TICKS_PER_SECOND && self.fuses_set == 0 {
            out.push(Act::Fuse { x: 211, y: 88 });
            self.fuses_set = 1;
        }
        if !self.mould_only && t == 5 * TICKS_PER_SECOND && self.fuses_set == 1 {
            out.push(Act::Fuse { x: 190, y: 47 });
            self.fuses_set = 2;
        }
        // 3. Blow burnable gas off the upper face: a stroke along the upper
        //    roof from the lamp end toward the air shaft.
        let danger = burnable_near(g, g.lamps[0].x - 14, g.lamps[0].y, 26);
        if !self.mould_only && self.blow_x.is_none() && danger > 30 && g.meters[0] > 0.5 {
            self.blow_x = Some(232.0);
        }
        if let Some(x) = self.blow_x {
            if g.meters[0] > 0.02 && x > 40.0 {
                if let Some(ry) = roof_y(g, x as i32, 30, 64) {
                    out.push(Act::Blow { x, y: (ry + 4) as f32, dx: -1.0, dy: 0.0 });
                }
                self.blow_x = Some(x - 1.5);
            } else {
                self.blow_x = None;
            }
        }
        // 4. Spark a burnable pocket when its flame cannot reach a lamp.
        if !self.mould_only && t >= self.next_spark_check && g.meters[1] > 0.3 {
            self.next_spark_check = t + TICKS_PER_SECOND / 2;
            if let Some((sx, sy, dry)) = safe_spark(g) {
                for (cx, cy) in dry.into_iter().take(if self.no_mould { 0 } else { 3 }) {
                    out.push(Act::Cut { x: cx, y: cy });
                    self.cuts_planned += 1;
                }
                out.push(Act::Spark { x: sx, y: sy });
                self.sparks_planned += 1;
            }
        }
        // 5. Snip a fuse: a dry tube with fire beside it, within reach of a
        //    lamp still alight, is cut at the fire's front.
        if !self.no_mould && !self.mould_only && t.is_multiple_of(3) && g.meters[3] > 0.1 {
            if let Some((x, y)) = fuse_front(g) {
                out.push(Act::Cut { x, y });
                self.cuts_planned += 1;
            }
        }
        // 6. Then lead the mould, the way a player herds it: damp a spot
        //    just ahead of the tip nearest the next waypoint, on the side
        //    toward it -- the drip chimney's foot, its top, then crack 2.
        //    (A fixed road laid along the upper floor at 60 s was tried
        //    first: it dried before the mould arrived, and the tips went for
        //    the richer gas under the roof instead.)
        let waypoints = [(149, 113), (149, 66), (121, 61)];
        if !self.no_mould && self.road1 >= road1.len() && t >= 40 * TICKS_PER_SECOND && g.cracks[1].cap_ema < 0.6 {
            while self.waypoint < waypoints.len() {
                let (wx, wy) = waypoints[self.waypoint];
                let reached = (-4..=4).any(|dy| (-4..=4).any(|dx| g.mould.tube_at(wx + dx, wy + dy).is_some()));
                if reached {
                    self.waypoint += 1;
                } else {
                    break;
                }
            }
            let target = waypoints.get(self.waypoint).copied().unwrap_or((121, 61));
            if t.is_multiple_of(6) && g.meters[2] > 0.06 {
                if let Some((x, y)) = lead_spot(g, target) {
                    out.push(Act::Damp { x, y });
                }
            }
        }
        out
    }
}

/// The stall road: up the lower gallery's left wall from the sump, then
/// along the roof to the stall's lip, every other cell.
pub(crate) fn stall_road(g: &Game) -> Vec<(i32, i32)> {
    let mut road = Vec::new();
    for y in (114..=134).rev().step_by(2) {
        if let Some(x) = (18..60).find(|&x| g.air.is_open(x, y)) {
            road.push((x, y));
        }
    }
    let lip = super::level::STALL_X - super::level::STALL_HALF;
    for x in (22..=lip).step_by(2) {
        if let Some(y) = roof_y(g, x, 100, 120) {
            road.push((x, y));
        }
    }
    road
}

/// Where to damp to lead the mould toward `target`: among the free surface
/// cells within 6 of the live tip nearest the target, the one nearest the
/// target.
fn lead_spot(g: &Game, target: (i32, i32)) -> Option<(i32, i32)> {
    let d2 = |x: i32, y: i32| (x - target.0) * (x - target.0) + (y - target.1) * (y - target.1);
    let tip = g.mould.tips.iter().filter(|t| t.alive).min_by_key(|t| d2(t.x, t.y))?;
    let gr = super::ground(&g.solid, &g.water, &g.wet);
    let mut best: Option<(i32, i32)> = None;
    for dy in -6..=6 {
        for dx in -6..=6 {
            let (x, y) = (tip.x + dx, tip.y + dy);
            if gr.free(x, y) && gr.surface(x, y) && best.is_none_or(|(bx, by)| d2(x, y) < d2(bx, by)) {
                best = Some((x, y));
            }
        }
    }
    best
}

/// A dry tube cell with fire beside it and a lamp still alight within 40:
/// where a fuse is about to run on.
fn fuse_front(g: &Game) -> Option<(i32, i32)> {
    let gr = super::ground(&g.solid, &g.water, &g.wet);
    for e in g.mould.edges.iter().filter(|e| e.alive) {
        for &(x, y) in &e.cells {
            let burning = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                let (nx, ny) = (x + dx, y + dy);
                g.air.is_open(nx, ny) && g.fire[idx(nx as usize, ny as usize)] > 0
            });
            if !burning || g.mould.guarded(x, y) {
                continue;
            }
            let k = g.mould.tube_at(x, y).unwrap();
            if g.mould.is_wet(k, &gr) {
                continue;
            }
            if g.lamps.iter().any(|l| l.state != super::LampState::Blown && (l.x - x).abs() + (l.y - y).abs() <= 40) {
                return Some((x, y));
            }
        }
    }
    None
}

fn burnable_near(g: &Game, x: i32, y: i32, r: i32) -> usize {
    let mut n = 0;
    for dy in -r..=r {
        for dx in -r..=r {
            if g.air.is_open(x + dx, y + dy) && band(g.air.at(x + dx, y + dy)) == Band::Burns {
                n += 1;
            }
        }
    }
    n
}

const SAFE_MARGIN: i32 = 18;

/// A spark spot and the dry tube cells to snip before lighting it.
type SparkPlan = (i32, i32, Vec<(i32, i32)>);

/// Find the biggest burnable pocket whose whole connected body stays at
/// least `SAFE_MARGIN` cells (Manhattan) from every lamp still alight, and the cell in
/// it farthest from the lamps. Also returns the dry tube cells lying in it,
/// to snip first.
fn safe_spark(g: &Game) -> Option<SparkPlan> {
    let mut seen = vec![false; W * H];
    let lamps: Vec<(i32, i32)> = g.lamps.iter().filter(|l| l.state != super::LampState::Blown).map(|l| (l.x, l.y)).collect();
    let mut best: Option<(SparkPlan, usize)> = None;
    for y0 in 0..H as i32 {
        for x0 in 0..W as i32 {
            let i0 = idx(x0 as usize, y0 as usize);
            if seen[i0] || !g.air.is_open(x0, y0) || band(g.air.c[i0]) != Band::Burns {
                continue;
            }
            let mut stack = vec![(x0, y0)];
            seen[i0] = true;
            let mut body = Vec::new();
            while let Some((x, y)) = stack.pop() {
                body.push((x, y));
                // Eight neighbours: flame crosses diagonals, so a lobe touching
                // the stall's pocket only at a corner is part of it.
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if !g.air.is_open(nx, ny) {
                        continue;
                    }
                    let j = idx(nx as usize, ny as usize);
                    if !seen[j] && band(g.air.c[j]) == Band::Burns {
                        seen[j] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            if body.len() < 60 {
                continue;
            }
            // 18, not the first version's 10: once the stall was added
            // (2026-11-08), a plume body 26 cells from lamp B burned big
            // enough to blow it.
            let near_lamp = body.iter().any(|&(x, y)| lamps.iter().any(|&(lx, ly)| (x - lx).abs() + (y - ly).abs() < SAFE_MARGIN));
            if near_lamp {
                continue;
            }
            let far = *body
                .iter()
                .max_by_key(|&&(x, y)| lamps.iter().map(|&(lx, ly)| (x - lx).abs() + (y - ly).abs()).min().unwrap_or(0))
                .unwrap();
            let gr = super::ground(&g.solid, &g.water, &g.wet);
            let mut dry = Vec::new();
            for &(x, y) in &body {
                if let Some(e) = g.mould.tube_at(x, y) {
                    if !g.mould.is_wet(e, &gr) && !dry.iter().any(|&(dx, dy): &(i32, i32)| (dx - x).abs() < 4 && (dy - y).abs() < 4) {
                        dry.push((x, y));
                    }
                }
            }
            if best.as_ref().is_none_or(|b| body.len() > b.1) {
                best = Some(((far.0, far.1, dry), body.len()));
            }
        }
    }
    best.map(|(plan, _)| plan)
}
