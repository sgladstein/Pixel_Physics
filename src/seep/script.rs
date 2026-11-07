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
//!   written as rules that read the screen: damp a road from the sump to the
//!   crack; blow burnable gas away from the upper lamp toward the air shaft;
//!   spark a burnable pocket only when the flame cannot reach a lamp, and
//!   snip any dry tube in its way first; fuses on the shaft; and later damp a
//!   road to the second crack.

use super::gas::{band, idx, Band, H, W};
use super::mould::Rng;
use super::{Act, Game, Tool, TICKS_PER_SECOND};

pub enum Player {
    Nothing,
    Random(RandomPlayer),
    Planned(Planned),
}

impl Player {
    pub fn name(&self) -> &'static str {
        match self {
            Player::Nothing => "nothing",
            Player::Random(_) => "random",
            Player::Planned(p) if p.no_mould => "gasonly",
            Player::Planned(_) => "planned",
        }
    }
    pub fn by_name(name: &str, seed: u64) -> Option<Player> {
        match name {
            "nothing" => Some(Player::Nothing),
            "random" => Some(Player::Random(RandomPlayer::new(seed))),
            "planned" => Some(Player::Planned(Planned::default())),
            "gasonly" | "nomould" => Some(Player::Planned(Planned { no_mould: true, ..Planned::default() })),
            _ => None,
        }
    }
    pub fn acts(&mut self, g: &Game) -> Vec<Act> {
        match self {
            Player::Nothing => Vec::new(),
            Player::Random(r) => r.acts(g),
            Player::Planned(p) => p.acts(g),
        }
    }
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
        // 1. Damp a road along the lower floor from the sump to crack 1.
        //    Paced by the meter: a stroke runs until it is nearly dry, then
        //    waits for it to come back.
        let road1: Vec<i32> = (44..=178).step_by(2).collect();
        if !self.no_mould && t > TICKS_PER_SECOND / 2 && self.road1 < road1.len() && g.meters[2] > 0.06 {
            let x = road1[self.road1];
            if let Some(y) = floor_y(g, x, 118, 145) {
                out.push(Act::Damp { x, y });
            }
            self.road1 += 1;
        }
        // 2. Fuses on the shaft, where overflow passes before it reaches the
        //    upper lamp.
        if t == 4 * TICKS_PER_SECOND && self.fuses_set == 0 {
            out.push(Act::Fuse { x: 211, y: 88 });
            self.fuses_set = 1;
        }
        if t == 5 * TICKS_PER_SECOND && self.fuses_set == 1 {
            out.push(Act::Fuse { x: 190, y: 47 });
            self.fuses_set = 2;
        }
        // 3. Blow burnable gas off the upper face: a stroke along the upper
        //    roof from the lamp end toward the air shaft.
        let danger = burnable_near(g, g.lamps[0].x - 14, g.lamps[0].y, 26);
        if self.blow_x.is_none() && danger > 30 && g.meters[0] > 0.5 {
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
        if t >= self.next_spark_check && g.meters[1] > 0.3 {
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
        // 5. Then lead the mould, the way a player herds it: damp a spot
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

/// Find a burnable pocket whose whole connected body stays more than 8
/// cells from every working lamp, and the cell in it farthest from the
/// lamps. Also returns the dry tube cells lying in it, to snip first.
/// A spark spot and the dry tube cells to snip before lighting it.
type SparkPlan = (i32, i32, Vec<(i32, i32)>);

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
            if body.len() < 60 {
                continue;
            }
            let near_lamp = body.iter().any(|&(x, y)| lamps.iter().any(|&(lx, ly)| (x - lx).abs() + (y - ly).abs() < 10));
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
