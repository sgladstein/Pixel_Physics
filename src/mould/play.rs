//! Scripted players, for the probe (`examples/mouldprobe.rs`): the
//! do-nothing control, a random hand, and a simple plan. None of them is
//! meant to be good; the plan exists to answer *can a plan beat doing
//! nothing at all*, which is the owner's bar ("doing nothing must lose, and
//! random play must lose to a plan"). A person should beat the plan.

use super::{Dish, Phase, Rng, Tool, LAMP_R, STEPS_PER_SECOND};

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
pub fn oats_toward_nearest(d: &mut Dish, ahead: f32) -> bool {
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

/// **The intended hand for a hand-made dish** (`level.rs`): what the
/// dish's designer would do, scripted, so the probe can show a dish pays
/// three stars to a player who reads it (the ship bar: do nothing 0,
/// random at most 1, this 3).
///
/// Oats are laid along a route, one at a time, a short step ahead of the
/// furthest point on it that home's mould has *tube* to, and the next only
/// once the tube has caught up with the last (or `WAIT_S` has passed). A
/// route is a list of legs, each a polyline, done in order: Behind the Wall
/// goes down to the gap and back up; Beat the Ruler goes home to the empty
/// middle, then out from the middle to each far food. Running Dry has no
/// fixed route: it `chase`s, leading a finger from the fed mould toward the
/// next food as soon as its dotted ring shows.
///
/// The front is fed mould in home's piece. The first version took any fed
/// cell, and a single walker wandering onto an oat 30 cells out made that
/// oat "fed": all five oats were down by 3.5 s and none of them pulled.
/// Oats never go on a dish pile (one did, on the far food, and was wasted).
pub struct Route {
    legs: Vec<Vec<(f32, f32)>>,
    leg: usize,
    chase: bool,
    next_think: u64,
    /// The last oat laid, and when.
    last: Option<((f32, f32), u64)>,
    /// Cells ahead of the front each oat goes.
    ahead: f32,
    /// Seconds to let home's mould grow out on its own before the first oat.
    start: u64,
    /// A meeting point to keep fed once the route is done (Beat the Ruler).
    tend: Option<(f32, f32)>,
    /// Chasing: lay oats only when the fed tube nearest the food is more
    /// than this many cells from it (Keep It Alive, where oats do not
    /// come back and the 22.5-degree mould reaches near food by itself).
    min_gap: f32,
}

/// Seconds the hand waits for the tube to reach its last oat before it lays
/// another.
const WAIT_S: f32 = 4.0;

impl Route {
    pub fn new(legs: Vec<Vec<(f32, f32)>>) -> Self {
        Route {
            legs,
            leg: 0,
            chase: false,
            next_think: 0,
            last: None,
            ahead: 22.0,
            start: 6,
            tend: None,
            min_gap: 0.0,
        }
    }

    /// The intended hand for `super::level::DISHES[k]`.
    pub fn for_level(k: usize) -> Self {
        let l = &super::level::DISHES[k];
        let p = |i: usize| (l.piles[i].x, l.piles[i].y);
        match l.key {
            // Down the wall to the gap, through it, and up to the food.
            "wall" => Route::new(vec![vec![
                p(0),
                (88.0, 114.0),
                (116.0, 136.0),
                (140.0, 136.0),
                (168.0, 114.0),
                p(1),
            ]]),
            // Home to the empty middle (the meeting point), then from the
            // middle -- not from home -- out to each far food.
            "ruler" => {
                let (m, _) = l.meeting_point();
                Route {
                    ahead: 26.0,
                    tend: Some(m),
                    ..Route::new(vec![vec![p(0), m], vec![m, p(1)], vec![m, p(2)]])
                }
            }
            "alive" => Route {
                chase: true,
                start: 0,
                min_gap: 35.0,
                ..Route::new(Vec::new())
            },
            _ => Route {
                chase: true,
                start: 0,
                ..Route::new(Vec::new())
            },
        }
    }

    /// Points every 2 cells along a polyline.
    fn samples(path: &[(f32, f32)]) -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        for w in path.windows(2) {
            let (a, b) = (w[0], w[1]);
            let n = ((b.0 - a.0).hypot(b.1 - a.1) / 2.0).ceil().max(1.0) as usize;
            for k in 0..n {
                let f = k as f32 / n as f32;
                out.push((a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f));
            }
        }
        if let Some(&last) = path.last() {
            out.push(last);
        }
        out
    }

    /// The piece the front must belong to: home's, or (chasing, where home
    /// runs dry) any.
    fn anchor(&self, d: &Dish) -> Option<usize> {
        if self.chase {
            return None;
        }
        let h = d.dish_piles().next()?;
        d.piece_of(h.y as usize * super::W + h.x as usize)
    }

    /// Whether fed tube of the anchor's piece lies within `r` cells.
    fn tube_near(d: &Dish, anchor: Option<usize>, (x, y): (f32, f32), r: f32) -> bool {
        let n = r.ceil() as i32;
        for dy in -n..=n {
            for dx in -n..=n {
                if (dx * dx + dy * dy) as f32 > r * r {
                    continue;
                }
                let (cx, cy) = (x as i32 + dx, y as i32 + dy);
                if cx < 0 || cy < 0 || cx as usize >= super::W || cy as usize >= super::H {
                    continue;
                }
                let i = cy as usize * super::W + cx as usize;
                let f = d.fed[i];
                if f != u16::MAX && (anchor.is_none() || d.piece_of(i) == anchor) {
                    return true;
                }
            }
        }
        false
    }

    /// Lay an oat `ahead` cells past the furthest tube on `path`. Returns
    /// true once the end of the path has tube.
    fn lead_along(&mut self, d: &mut Dish, path: &[(f32, f32)], anchor: Option<usize>) -> bool {
        let s = Self::samples(path);
        let front = (0..s.len()).rev().find(|&k| Self::tube_near(d, anchor, s[k], 4.0)).unwrap_or(0);
        if front + 3 >= s.len() {
            return true;
        }
        // Save up: an oat drains in about 3 s once eaten, so a relay must
        // be laid in one go. Between relays, wait until there are oats
        // enough to reach the end (the food's own scent pulls the last
        // 20 cells), or a full hand.
        let sps = STEPS_PER_SECOND as u64;
        let in_relay = self.last.is_some_and(|(_, when)| d.t < when + (WAIT_S * sps as f32) as u64);
        let left = (s.len() - 1 - front) as f32 * 2.0;
        let need = (((left - 20.0) / self.ahead).ceil().max(1.0) as u32).min(d.oats_stock);
        if !self.chase && !in_relay && d.oats_left < need {
            return false;
        }
        let want = (front + (self.ahead / 2.0) as usize).min(s.len() - 1);
        for k in (front + 1..=want).rev() {
            let on_pile = d.dish_piles().any(|p| (p.x - s[k].0).hypot(p.y - s[k].1) <= p.r + 3.0);
            if !on_pile && d.place_oats(s[k].0, s[k].1) {
                self.last = Some((s[k], d.t));
                break;
            }
        }
        false
    }

    /// Running Dry: the food to head for next -- the next pile whose ring
    /// is showing, else the newest pile with food not yet eaten.
    fn next_food(d: &Dish) -> Option<(f32, f32)> {
        let sps = STEPS_PER_SECOND as u64;
        let soon = d
            .pending
            .iter()
            .filter(|(at, _)| *at <= d.t + 5 * sps)
            .map(|(_, p)| (p.x, p.y))
            .next();
        soon.or_else(|| {
            d.dish_piles()
                .filter(|p| !p.reached() && p.food > 0.0)
                .map(|p| (p.x, p.y))
                .last()
        })
    }
}

impl Player for Route {
    fn name(&self) -> &'static str {
        "intended hand"
    }
    fn act(&mut self, d: &mut Dish) {
        let sps = STEPS_PER_SECOND as u64;
        if d.phase != Phase::Growing || d.t < self.next_think || d.t < self.start * sps {
            return;
        }
        self.next_think = d.t + sps / 2;
        let anchor = self.anchor(d);
        // One oat at a time: wait until the tube has reached the last one.
        if let Some((at, when)) = self.last {
            let caught = Self::tube_near(d, anchor, at, 6.0);
            if !caught && d.t < when + (WAIT_S * sps as f32) as u64 {
                return;
            }
        }
        if d.oats_left == 0 {
            return;
        }
        if self.chase {
            if let Some(target) = Self::next_food(d) {
                // From the fed tube nearest the target.
                let mut from: Option<((f32, f32), f32)> = None;
                for (i, &f) in d.fed.iter().enumerate() {
                    if f == u16::MAX || f == 0 || i % 3 != 0 {
                        continue;
                    }
                    let (x, y) = ((i % super::W) as f32 + 0.5, (i / super::W) as f32 + 0.5);
                    let g = (x - target.0).hypot(y - target.1);
                    if from.is_none_or(|b| g < b.1) {
                        from = Some(((x, y), g));
                    }
                }
                // Keep It Alive: only once the food being eaten is nearly
                // gone, since the mould reaches near food by itself and
                // the oats do not come back.
                let left: f32 = d.dish_piles().filter(|p| p.reached()).map(|p| p.food / p.full).sum();
                let urgent = self.min_gap <= 0.0 || left < 0.35;
                if let Some((from, g)) = from {
                    if urgent && g > self.min_gap {
                        self.lead_along(d, &[from, target], None);
                    }
                }
            }
        } else {
            while self.leg < self.legs.len() {
                let path = self.legs[self.leg].clone();
                if self.lead_along(d, &path, anchor) {
                    self.leg += 1;
                } else {
                    break;
                }
            }
            // Tending a meeting point: once every leg is done, a junction
            // left alone slides toward a side (the mould straightens any
            // bend it is not pulled into), and on 1 seed in 5 the intended
            // hand's 0.87x had become 0.99x by the bell. So while the join
            // reads worse than 0.90x, an oat goes on the meeting point,
            // one at a time.
            if let (true, Some(m)) = (self.leg >= self.legs.len(), self.tend) {
                let slack = d.join_now.is_none_or(|r| r > 0.90);
                let quiet = self.last.is_none_or(|(_, when)| d.t >= when + 6 * sps);
                let oat_there = d.piles.iter().any(|p| p.oats && (p.x - m.0).hypot(p.y - m.1) < 8.0);
                if slack && quiet && !oat_there && d.place_oats(m.0, m.1) {
                    self.last = Some((m, d.t));
                }
            }
        }
    }
}

// ------------------------------------------------------------ the Wanderer

/// Oats `ahead` cells from the walker nearest `to`, toward it (never past it).
pub fn oats_toward(d: &mut Dish, to: (f32, f32), ahead: f32) -> bool {
    let Some((wx, wy)) = d
        .walkers
        .iter()
        .step_by(3)
        .map(|w| (w.x, w.y))
        .min_by(|a, b| (a.0 - to.0).hypot(a.1 - to.1).total_cmp(&(b.0 - to.0).hypot(b.1 - to.1)))
    else {
        return false;
    };
    let g = (wx - to.0).hypot(wy - to.1).max(1.0);
    let f = (ahead / g).min(1.0);
    d.place_oats(wx + (to.0 - wx) * f, wy + (to.1 - wy) * f)
}

/// A point on the dish, in cells.
type Pt = (f32, f32);

/// The closest pair (stranded, joined): a fed walker in a piece not joined
/// to the crack, and a walker that is, if at least a tenth of her is
/// stranded. Sampled, every fourth walker of each.
fn bridge_gap(d: &Dish) -> Option<(Pt, Pt)> {
    let w = d.wander_state()?;
    let at = |x: f32, y: f32| y as usize * super::W + x as usize;
    let (mut joined, mut stranded) = (Vec::new(), Vec::new());
    for k in d.walkers.iter().step_by(4) {
        let i = at(k.x, k.y);
        if w.exit_dist[i] != u16::MAX {
            joined.push((k.x, k.y));
        } else if d.fed[i] != u16::MAX {
            stranded.push((k.x, k.y));
        }
    }
    if joined.is_empty() || stranded.len() * 10 < (joined.len() + stranded.len()) {
        return None;
    }
    let mut best: Option<(f32, Pt, Pt)> = None;
    for &a in &stranded {
        for &b in &joined {
            let g = (a.0 - b.0).hypot(a.1 - b.1);
            if best.is_none_or(|x| g < x.0) {
                best = Some((g, a, b));
            }
        }
    }
    best.map(|b| (b.1, b.2))
}

/// Does nothing, but presses Go the moment it is open: the control for the
/// Go rule itself (how much arrives when nobody leads).
pub struct IdleGo;

impl Player for IdleGo {
    fn name(&self) -> &'static str {
        "do nothing, Go when open"
    }
    fn act(&mut self, d: &mut Dish) {
        if d.can_go() {
            d.go();
        }
    }
}

/// **The Wanderer's leading hand**, lamp and oats. Oats one at a time, 22
/// cells ahead of the mould: first to the food on the crack's side of the
/// dish (x > 150), via the nearest food if none of that is out yet, then to
/// the crack, then kept at the crack's mouth so the bridge stays fed. The
/// lamp shepherds strays beside the front back toward the line it is being
/// led along (as `Shepherd`). Once in the crack, it waits for the trunk to
/// thicken: Go when the mould joined to the crack has peaked (a tenth below
/// its best, or no new best for 15 s), or the food is nearly gone.
///
/// `careless`: the same lead, but Go the first moment it is open.
pub struct Leader {
    careless: bool,
    best: u32,
    best_at: u64,
    next_think: u64,
}

impl Leader {
    pub fn new(careless: bool) -> Self {
        Leader {
            careless,
            best: 0,
            best_at: 0,
            next_think: 0,
        }
    }
}

impl Player for Leader {
    fn name(&self) -> &'static str {
        if self.careless {
            "careless lead, Go at once"
        } else {
            "lead, Go at the peak"
        }
    }
    fn act(&mut self, d: &mut Dish) {
        use super::wander::{in_exit, EXIT_X, EXIT_Y};
        if d.phase != Phase::Growing || d.t < self.next_think {
            return;
        }
        let s = super::STEPS_PER_SECOND as u64;
        self.next_think = d.t + s / 2;
        if d.can_go() {
            if self.careless {
                d.go();
                return;
            }
            // Wait until most of her is joined to the crack and the joined
            // count has stopped climbing for a few seconds (the trunk has
            // had time to thicken); give up waiting after 20 s without a new
            // best, or when the food is nearly gone. On the first census
            // this hand went at the first dip of the count and took only
            // the far pile's 203 walkers, its main body never joined.
            let wc = d.wander_state().map_or(0, |w| w.joined);
            if wc > self.best {
                self.best = wc;
                self.best_at = d.t;
            }
            let whole = wc as usize * 100 >= d.walkers.len() * 85;
            if (whole && d.t > self.best_at + 5 * s) || d.t > self.best_at + 20 * s || d.food_left() < 0.05 {
                d.go();
                return;
            }
        }
        let crack = (EXIT_X as f32 + 6.0, EXIT_Y);
        let touching = d.wander_state().is_some_and(|w| w.touching);
        let reached_east = d.dish_piles().any(|p| p.reached() && p.x > 150.0);
        let east: Vec<(f32, f32)> = d
            .dish_piles()
            .filter(|p| !p.reached() && p.x > 150.0)
            .map(|p| (p.x, p.y))
            .collect();
        let gap = |d: &Dish, p: (f32, f32)| {
            d.walkers
                .iter()
                .map(|w| (w.x - p.0).hypot(w.y - p.1))
                .fold(f32::INFINITY, f32::min)
        };
        // Where it is being led.
        let target = if touching {
            crack
        } else if let Some(&p) = east.iter().min_by(|a, b| gap(d, **a).total_cmp(&gap(d, **b))) {
            p
        } else {
            crack
        };
        let live_oats = d.piles.iter().filter(|p| p.oats).count();
        if d.oats_left > 0 && live_oats == 0 {
            if touching {
                if let Some((a, b)) = bridge_gap(d) {
                    // Part of her is not joined to the crack: oats from the
                    // stranded mould toward the nearest joined tube.
                    let g = (a.0 - b.0).hypot(a.1 - b.1).max(1.0);
                    let f = (22.0 / g).min(0.5);
                    d.place_oats(a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
                } else {
                    // All joined: keep the bridge fed and thick, oats at
                    // the crack's mouth level with the mould already in it.
                    let y = d
                        .walkers
                        .iter()
                        .find(|w| in_exit(w.x as usize, w.y as usize))
                        .map_or(EXIT_Y, |w| w.y);
                    d.place_oats(EXIT_X as f32 - 2.0, y);
                }
            } else if !reached_east && east.is_empty() {
                oats_toward_nearest(d, 22.0);
            } else {
                oats_toward(d, target, 22.0);
            }
        }
        // Lamp: a stray near the front, well off the line it is led along.
        let Some(tip) = d
            .walkers
            .iter()
            .map(|w| (w.x, w.y))
            .min_by(|a, b| (a.0 - target.0).hypot(a.1 - target.1).total_cmp(&(b.0 - target.0).hypot(b.1 - target.1)))
        else {
            d.set_lamp(None);
            return;
        };
        let src = d
            .dish_piles()
            .filter(|p| p.reached())
            .map(|p| (p.x, p.y))
            .min_by(|a, b| (a.0 - tip.0).hypot(a.1 - tip.1).total_cmp(&(b.0 - tip.0).hypot(b.1 - tip.1)))
            .unwrap_or(tip);
        let mut stray: Option<(f32, (f32, f32))> = None;
        for w in d.walkers.iter().step_by(3) {
            if (w.x - tip.0).hypot(w.y - tip.1) > 50.0 {
                continue;
            }
            let near_food = d.piles.iter().any(|p| (p.x - w.x).hypot(p.y - w.y) < p.r + LAMP_R + 2.0);
            let off = seg_dist((w.x, w.y), src, target);
            if off > 16.0 && !near_food && stray.is_none_or(|s| off > s.0) {
                stray = Some((off, (w.x, w.y)));
            }
        }
        d.set_lamp(stray.map(|s| s.1));
    }
}
