//! Scripted players, for the probe (`examples/mouldprobe.rs`): the
//! do-nothing control, a random hand, and a simple plan. None of them is
//! meant to be good; the plan exists to answer *can a plan beat doing
//! nothing at all*, which is the owner's bar ("doing nothing must lose, and
//! random play must lose to a plan"). A person should beat the plan.

use super::{Dish, Phase, Rng, Tool, LAMP_R};

pub trait Player {
    fn name(&self) -> &'static str;
    /// Called once per drawn frame, before the dish steps.
    fn act(&mut self, dish: &mut Dish);
}

/// Touches nothing.
pub struct Idle;

impl Player for Idle {
    fn name(&self) -> &'static str {
        "do nothing"
    }
    fn act(&mut self, _dish: &mut Dish) {}
}

/// Uses every tool, at random places and times: oats every few seconds until
/// they run out, the lamp held somewhere random for a couple of seconds now
/// and then, and the snips on random strokes.
pub struct Random {
    rng: Rng,
    lamp_until: u64,
}

impl Random {
    pub fn new(seed: u64) -> Self {
        Random {
            rng: Rng::new(seed ^ 0xA5A5_5A5A),
            lamp_until: 0,
        }
    }
    fn spot(&mut self, d: &Dish) -> (f32, f32) {
        loop {
            let (x, y) = (self.rng.f32() * super::W as f32, self.rng.f32() * super::H as f32);
            if d.ground[y as usize * super::W + x as usize] == super::Ground::Agar {
                return (x, y);
            }
        }
    }
}

impl Player for Random {
    fn name(&self) -> &'static str {
        "random hand"
    }
    fn act(&mut self, d: &mut Dish) {
        if d.phase != Phase::Growing {
            return;
        }
        let frames_per_s = 60.0;
        if d.t >= self.lamp_until {
            d.set_lamp(None);
        }
        let r = self.rng.f32() * frames_per_s;
        if r < 0.25 && d.oats_left > 0 {
            let (x, y) = self.spot(d);
            d.place_oats(x, y);
        } else if r < 0.5 && d.lamp.is_none() {
            let p = self.spot(d);
            d.set_lamp(Some(p));
            self.lamp_until = d.t + 2 * super::STEPS_PER_SECOND as u64;
        } else if r < 0.6 && d.snips_left > 0 {
            let (x, y) = self.spot(d);
            let a = self.rng.f32() * std::f32::consts::TAU;
            if d.begin_stroke(Tool::Snip, x, y) {
                d.drag_stroke(x + a.cos() * 20.0, y + a.sin() * 20.0);
                d.end_stroke();
            }
        }
    }
}

/// A plain plan, the kind a first-time player is told: lead the mould from
/// food to food with oats as stepping stones, nearest unjoined pile first,
/// and keep the lamp on whatever part of it is growing away from every pile.
pub struct Plan {
    next_think: u64,
}

impl Plan {
    pub fn new() -> Self {
        Plan { next_think: 0 }
    }
}

impl Default for Plan {
    fn default() -> Self {
        Self::new()
    }
}

/// Distance from point to segment.
fn seg_dist(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let len2 = vx * vx + vy * vy;
    let t = if len2 > 0.0 {
        (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (cx, cy) = (a.0 + vx * t, a.1 + vy * t);
    ((p.0 - cx).powi(2) + (p.1 - cy).powi(2)).sqrt()
}

impl Player for Plan {
    fn name(&self) -> &'static str {
        "simple plan"
    }
    fn act(&mut self, d: &mut Dish) {
        if d.phase != Phase::Growing || d.t < self.next_think {
            return;
        }
        self.next_think = d.t + super::STEPS_PER_SECOND as u64 / 2;
        let piles: Vec<(f32, f32, f32, bool)> = d.dish_piles().map(|p| (p.x, p.y, p.r, p.reached())).collect();
        // The corridors a good network would use: a tree over every pile.
        let mut tree: Vec<((f32, f32), (f32, f32))> = Vec::new();
        {
            let mut inside = vec![false; piles.len()];
            inside[0] = true;
            for _ in 1..piles.len() {
                let mut best = (f32::INFINITY, 0, 0);
                for a in 0..piles.len() {
                    if !inside[a] {
                        continue;
                    }
                    for b in 0..piles.len() {
                        if inside[b] {
                            continue;
                        }
                        let dd = (piles[a].0 - piles[b].0).hypot(piles[a].1 - piles[b].1);
                        if dd < best.0 {
                            best = (dd, a, b);
                        }
                    }
                }
                inside[best.2] = true;
                tree.push(((piles[best.1].0, piles[best.1].1), (piles[best.2].0, piles[best.2].1)));
            }
        }
        // Oats: if an unjoined pile is out of reach of the nearest mould, put
        // oats part-way along the line from that mould to it.
        let live_oats = d.piles.iter().filter(|p| p.oats).count();
        if d.oats_left > 0 && live_oats == 0 && piles.iter().all(|p| p.3) {
            // Everything joined: spend what is left on a loop. Oats halfway
            // between the closest two piles with no tube straight between
            // them, which is what survives a storm.
            let mut best: Option<(f32, (f32, f32))> = None;
            for a in 0..piles.len() {
                for b in a + 1..piles.len() {
                    let (pa, pb) = (piles[a], piles[b]);
                    let len = (pa.0 - pb.0).hypot(pa.1 - pb.1);
                    let n = (len as usize).max(1);
                    let mut lit = 0;
                    for k in 0..=n {
                        let f = k as f32 / n as f32;
                        let (x, y) = (pa.0 + (pb.0 - pa.0) * f, pa.1 + (pb.1 - pa.1) * f);
                        if d.trail[y as usize * super::W + x as usize] > d.params.tube {
                            lit += 1;
                        }
                    }
                    let linked = lit * 10 >= (n + 1) * 8;
                    if !linked && best.is_none_or(|bb| len < bb.0) {
                        best = Some((len, ((pa.0 + pb.0) / 2.0, (pa.1 + pb.1) / 2.0)));
                    }
                }
            }
            if let Some((_, (x, y))) = best {
                d.place_oats(x, y);
            }
        } else if d.oats_left > 0 && live_oats == 0 {
            // (gap, nearest mould, pile)
            type Lead = (f32, (f32, f32), (f32, f32));
            let mut best: Option<Lead> = None;
            for &(px, py, pr, reached) in &piles {
                if reached {
                    continue;
                }
                for w in d.walkers.iter().step_by(7) {
                    let dd = (w.x - px).hypot(w.y - py) - pr;
                    if best.is_none_or(|b| dd < b.0) {
                        best = Some((dd, (w.x, w.y), (px, py)));
                    }
                }
            }
            if let Some((dd, (wx, wy), (px, py))) = best {
                if dd > 22.0 {
                    let f = 22.0 / (dd + 6.0);
                    d.place_oats(wx + (px - wx) * f, wy + (py - wy) * f);
                }
            }
        }
        // Lamp: on the mould farthest from every corridor, if any strays --
        // but never where the light would reach a pile or a corridor, which
        // in the first census cut the plan's own trunk (seed 2: joined 3
        // dropped to 2 under its own lamp).
        let clear = LAMP_R + 4.0;
        let mut far: Option<(f32, (f32, f32))> = None;
        for w in d.walkers.iter().step_by(5) {
            let dd = tree
                .iter()
                .map(|&(a, b)| seg_dist((w.x, w.y), a, b))
                .fold(f32::INFINITY, f32::min);
            let dp = piles
                .iter()
                .map(|p| (p.0 - w.x).hypot(p.1 - w.y) - p.2)
                .fold(f32::INFINITY, f32::min);
            if dd > clear && dp > clear && far.is_none_or(|f| dd > f.0) {
                far = Some((dd, (w.x, w.y)));
            }
        }
        d.set_lamp(far.map(|f| f.1));
    }
}

/// One unjoined pile as a lead: (pile x, y, radius), the nearest walker,
/// and the gap from that walker to the pile's rim.
type Lead = ((f32, f32, f32), (f32, f32), f32);

/// Unjoined dish piles, nearest to the mould first.
fn leads(d: &Dish) -> Vec<Lead> {
    let mut out = Vec::new();
    for p in d.dish_piles().filter(|p| !p.reached()) {
        let mut best: Option<((f32, f32), f32)> = None;
        for w in d.walkers.iter().step_by(5) {
            let g = (w.x - p.x).hypot(w.y - p.y) - p.r;
            if best.is_none_or(|b| g < b.1) {
                best = Some(((w.x, w.y), g));
            }
        }
        if let Some((at, g)) = best {
            out.push(((p.x, p.y, p.r), at, g));
        }
    }
    out.sort_by(|a, b| a.2.total_cmp(&b.2));
    out
}

/// Oats `ahead` cells from the mould toward the nearest unjoined pile.
fn oats_toward_nearest(d: &mut Dish, ahead: f32) -> bool {
    let Some(((px, py, _), (wx, wy), g)) = leads(d).first().copied() else {
        return false;
    };
    let f = (ahead / (g + 6.0)).min(1.0);
    d.place_oats(wx + (px - wx) * f, wy + (py - wy) * f)
}

/// The playtest's fresh player (2026-11-08 review): lamp on the blob for the
/// first 4 s, five oats in a line toward the far side by 18 s, a salt line,
/// and the three snips on strays. Oats that come back later go toward the
/// nearest unjoined pile, as anyone would spend them.
pub struct Fresh {
    line: Vec<(f32, f32)>,
}

impl Fresh {
    pub fn new() -> Self {
        Fresh { line: Vec::new() }
    }
}

impl Default for Fresh {
    fn default() -> Self {
        Self::new()
    }
}

impl Player for Fresh {
    fn name(&self) -> &'static str {
        "fresh player"
    }
    fn act(&mut self, d: &mut Dish) {
        if d.phase != Phase::Growing {
            return;
        }
        let s = super::STEPS_PER_SECOND as u64;
        let (hx, hy) = (d.piles[0].x, d.piles[0].y);
        if d.t < 4 * s {
            d.set_lamp(Some((hx + 4.0, hy)));
            return;
        }
        d.set_lamp(None);
        if self.line.is_empty() {
            // Toward the farthest pile on the dish.
            let far = d
                .dish_piles()
                .map(|p| (p.x, p.y))
                .max_by(|a, b| (a.0 - hx).hypot(a.1 - hy).total_cmp(&(b.0 - hx).hypot(b.1 - hy)))
                .unwrap();
            self.line = (1..=5)
                .map(|k| k as f32 / 6.0)
                .map(|f| (hx + (far.0 - hx) * f, hy + (far.1 - hy) * f))
                .collect();
        }
        // One oat every 2.8 s from 4 s: all five down by 18 s.
        let placed = d.counts.oats_placed as usize;
        if placed < 5 && d.t >= 4 * s + placed as u64 * (28 * s / 10) {
            let (x, y) = self.line[placed];
            d.place_oats(x, y);
        } else if placed >= 5 && d.oats_left > 0 && d.t.is_multiple_of(s) {
            oats_toward_nearest(d, 22.0);
        }
        // A salt line at 20 s, behind home where nothing is wanted.
        if d.t == 20 * s && d.salt_left > 0 && d.begin_stroke(super::Tool::Salt, hx - 22.0, hy - 25.0) {
            d.drag_stroke(hx - 22.0, hy + 25.0);
            d.end_stroke();
        }
        // Snips on strays at 25, 35 and 45 s: the walker farthest from every pile.
        if [25, 35, 45].iter().any(|&k| d.t == k * s) && d.snips_left > 0 {
            let piles: Vec<(f32, f32)> = d.dish_piles().map(|p| (p.x, p.y)).collect();
            let stray = d.walkers.iter().map(|w| (w.x, w.y)).max_by(|a, b| {
                let da = piles
                    .iter()
                    .map(|p| (p.0 - a.0).hypot(p.1 - a.1))
                    .fold(f32::INFINITY, f32::min);
                let db = piles
                    .iter()
                    .map(|p| (p.0 - b.0).hypot(p.1 - b.1))
                    .fold(f32::INFINITY, f32::min);
                da.total_cmp(&db)
            });
            if let Some((x, y)) = stray {
                if d.begin_stroke(super::Tool::Snip, x - 8.0, y - 8.0) {
                    d.drag_stroke(x + 8.0, y + 8.0);
                    d.end_stroke();
                }
            }
        }
    }
}

/// The playtest's clever player: lamp only, lighting strays beside the front
/// so the growth heads for the nearest unjoined pile.
pub struct Shepherd;

impl Player for Shepherd {
    fn name(&self) -> &'static str {
        "lamp shepherd"
    }
    fn act(&mut self, d: &mut Dish) {
        if d.phase != Phase::Growing || !d.t.is_multiple_of(super::STEPS_PER_SECOND as u64 / 2) {
            return;
        }
        let Some(((px, py, _), tip, _)) = leads(d).first().copied() else {
            d.set_lamp(None);
            return;
        };
        // The source: the reached pile nearest the tip.
        let src = d
            .dish_piles()
            .filter(|p| p.reached())
            .map(|p| (p.x, p.y))
            .min_by(|a, b| {
                (a.0 - tip.0)
                    .hypot(a.1 - tip.1)
                    .total_cmp(&(b.0 - tip.0).hypot(b.1 - tip.1))
            })
            .unwrap_or(tip);
        let mut stray: Option<(f32, (f32, f32))> = None;
        for w in d.walkers.iter().step_by(3) {
            if (w.x - tip.0).hypot(w.y - tip.1) > 50.0 {
                continue;
            }
            let off = seg_dist((w.x, w.y), src, (px, py));
            if off > 12.0 && stray.is_none_or(|s| off > s.0) {
                stray = Some((off, (w.x, w.y)));
            }
        }
        d.set_lamp(stray.map(|s| s.1));
    }
}

/// The builder's plan with its oats taken away: the lamp logic alone.
pub struct LampOnly(Plan);

impl LampOnly {
    pub fn new() -> Self {
        LampOnly(Plan::new())
    }
}

impl Default for LampOnly {
    fn default() -> Self {
        Self::new()
    }
}

impl Player for LampOnly {
    fn name(&self) -> &'static str {
        "plan's lamp, no oats"
    }
    fn act(&mut self, d: &mut Dish) {
        d.oats_left = 0;
        self.0.act(d);
    }
}

/// Cheese: every oat dropped the moment it is in hand, a short step from
/// home toward each pile in turn.
pub struct Dump;

impl Player for Dump {
    fn name(&self) -> &'static str {
        "dump the oats"
    }
    fn act(&mut self, d: &mut Dish) {
        if d.phase != Phase::Growing || d.t < super::STEPS_PER_SECOND as u64 || d.oats_left == 0 {
            return;
        }
        let (hx, hy) = (d.piles[0].x, d.piles[0].y);
        let piles: Vec<(f32, f32)> = d.dish_piles().skip(1).map(|p| (p.x, p.y)).collect();
        while d.oats_left > 0 {
            let k = d.counts.oats_placed as usize % piles.len().max(1);
            let (px, py) = piles[k];
            let l = (px - hx).hypot(py - hy).max(1.0);
            if !d.place_oats(hx + (px - hx) / l * 25.0, hy + (py - hy) / l * 25.0) {
                break;
            }
        }
    }
}
