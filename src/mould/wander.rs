//! **The Wanderer: lead the mould somewhere, instead of scoring a dish.**
//! `cargo run --release --bin mould -- wander`
//!
//! Prototype for the 2026-11-08 brainstorm (`theme.md` section 1; brief
//! `prototypes/wanderer.md`). The question is whether *leading the mould
//! somewhere* feels better than *scoring a dish*, and this is the cheapest
//! test of it: the scored dish with its clock, storm and stars taken out, and
//! in their place
//!
//! * **an exit**, a glowing crack on the right edge of the dish (`in_exit`);
//! * **Go** (`Dish::go`), allowed only while some fed mould stands in the
//!   crack;
//! * **"MABEL ARRIVED WITH N OF M"** in place of the stars, where M is her
//!   peak this dish, and **"MABEL DIDN'T MAKE IT"** if every walker starves;
//! * **carry-over**: the next dish starts with the N that arrived, placed at
//!   its left edge (`Dish::wanderer`).
//!
//! There is no clock. The food under the mould drains (a 20,000 pile lasts
//! about 2 min 20 s at a full table) and its scent fades as it goes, which
//! is the reason to move on.
//!
//! # Who arrives: the pour
//!
//! **On Go, every fed walker joined to the crack by tube sets off down its
//! tubes toward the crack, one walker to a cell, and every one that gets
//! through the rim has arrived.** The pour is off food, so a walker in the
//! line starves (`POUR_STARVE`, about 7% of the line a second). So the cost
//! of leaving is set by the route the player built: a long route, or a thin
//! bridge that makes the line
//! queue, loses mould on the way; a thick short trunk brings most of her.
//! That is the theme's own line, "the thicker the trunk to the exit, the
//! more of it makes it through", and it is graded walker by walker. N is
//! counted as they come through, so the player watches it climb, and the
//! grey that stays behind is what the route cost.
//!
//! From Go on, nothing left in the dish counts as fed: the mould that stays
//! behind cannot divide and starves at the dish's ordinary unfed rate, and
//! only the cells beside a walker in the line are drawn yellow, so the
//! yellow is seen to drain out of the network toward the crack. (On the
//! first try the route out stayed fed; the thirty-odd walkers left on it
//! bred back to 1,760 during the pour and the network never went grey.)
//!
//! M is her **peak** this dish, not her size at Go: the number answers "how
//! much of her, at her best, made it", so mould lost to the food running out
//! while the player dithered counts against it too.
//!
//! **Tried first and dropped (2026-11-08): N fixed at Go as the fed walkers
//! within 70 cells of tube from the crack** (the food's own reach). The
//! mould in this model does not migrate, it extends, and each pile feeds a
//! fixed number of walkers, so that rule only ever took the one or two piles
//! nearest the crack: on seeds 1 and 2 the leading hand arrived with 203 of
//! 1,894 and 203 of 1,902, and the hand that pressed Go the instant it could
//! with 173 of 1,705 and 209 of 1,867 -- 11% each, leading worth nothing.
//!
//! **Independent of the store.** Who sets off reads `fed` and the tube; the
//! line's starving is a rate applied here, not the store; so it means
//! the same with one shared store or with the per-piece food the thread owner
//! is building (2026-11-08). Lifting the pourers out of `Dish::walkers` means
//! `economy()` never sees them, and it is unchanged.

use std::f32::consts::TAU;

use super::{cell, idx, Dish, Ground, Phase, Walker, H, STEPS_PER_SECOND, W};

/// The crack: open dish at or right of this column, within `EXIT_HALF`
/// rows of `EXIT_Y`. About 12 cells deep and 45 tall on the dish's right
/// edge, the opposite end from home (40, 82).
pub const EXIT_X: usize = 238;
pub const EXIT_Y: f32 = 80.0;
pub const EXIT_HALF: f32 = 22.0;
/// Chance per step that a walker in the pour moves one cell on, if the cell
/// ahead is free: every step, 120 cells a second with nobody in its way, so
/// how long the pour takes is set by queueing at the narrowest tube, not by
/// distance. At 0.6 (and `POUR_STARVE` 1/2400) the led pour on seed 1 took
/// 26 s to watch; at 1.0 and 1/1600 it takes 15 s and the leading hand's
/// share over seeds 1-4 moved from 47-55% to 52-59%.
pub const FLOW_CHANCE: f32 = 1.0;
/// Chance per step that a walker in the line starves: about 7% of the line a
/// second. **About a third of the dish's unfed rate** (`Params::starve`,
/// about 18% a second), and the reason is measured: at the unfed rate the
/// leading hand, its whole body joined to the crack, lost 77% of it in the
/// line on seeds 1-4 (1,456 of 1,893 on seed 1), so a good route and a bad
/// one both read as "most of her died". The fiction: on the move, she lives
/// off her own body for a while.
pub const POUR_STARVE: f32 = 1.0 / 1600.0;
/// The pour lasts at least this long, so the grey left behind can be seen
/// starting, and at most `POUR_MAX_SECONDS` (by then about 3% of the line
/// is left).
pub const POUR_MIN_SECONDS: u32 = 2;
pub const POUR_MAX_SECONDS: u32 = 45;
/// Her name. The theme names her on the first screen; the prototype fixes
/// it.
pub const NAME: &str = "MABEL";
/// Walkers in the first dish's inoculum (`Dish::build`'s blob).
pub const FIRST_WALKERS: u32 = 400;

/// How a stretch ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// `n` came through the crack, of `of` at her peak this dish.
    Arrived { n: u32, of: u32 },
    /// Every walker starved, at this step.
    Lost { at: u64 },
}

/// The Wanderer's state on a dish.
#[derive(Clone, Debug)]
pub struct Wander {
    /// Which dish of the journey this is, from 1.
    pub stretch: u32,
    /// Walkers she started this dish with.
    pub came_in: u32,
    /// Most walkers she has had this dish (M).
    pub peak: u32,
    /// Cells of tube from the crack (0 in it; `u16::MAX` off tube or not
    /// joined to it).
    pub exit_dist: Vec<u16>,
    /// Fed walkers joined to the crack by tube: the most that could set off
    /// if Go were pressed now.
    pub joined: u32,
    /// Some fed walker stands in the crack.
    pub touching: bool,
    pub go_at: Option<u64>,
    /// Walkers that set off when Go was pressed.
    pub set_off: u32,
    /// Fraction of the dish's food left when Go was pressed.
    pub food_at_go: f32,
    /// Through the crack so far.
    pub through: u32,
    /// Starved in the line.
    pub lost_on_way: u32,
    pub outcome: Option<Outcome>,
    /// Walkers in the line, lifted out of `Dish::walkers`.
    pub pouring: Vec<(f32, f32)>,
    /// One walker to a cell in the line.
    pour_occ: Vec<bool>,
}

/// Whether a cell is in the crack (ground aside).
pub fn in_exit(x: usize, y: usize) -> bool {
    x >= EXIT_X && (y as f32 + 0.5 - EXIT_Y).abs() <= EXIT_HALF
}

impl Dish {
    /// One stretch of the Wanderer's journey: dish `seed`, the `stretch`-th
    /// of the run, with `came_in` walkers. The first stretch starts as a
    /// scored dish does, a 400-walker blob on home; a later one places what
    /// arrived at the dish's left edge, around home, as if it had just come
    /// in through the crack of the dish before.
    ///
    /// **The store is not scaled to the walkers carried in** (the thread
    /// owner's per-piece food replaces it, 2026-11-08), so a large mould
    /// arrives on one pile's worth of food and has seconds to reach more
    /// before the excess starves. That is a cost of the journey, measured in
    /// the probe, not designed.
    pub fn wanderer(seed: u64, stretch: u32, came_in: u32) -> Self {
        let mut d = Self::build(seed, super::Params::default(), true);
        // No clock: no storm, no score.
        d.storm_at = u64::MAX;
        if stretch > 1 || came_in != FIRST_WALKERS {
            for w in d.walkers.drain(..) {
                if let Some(i) = cell(w.x, w.y) {
                    d.occ[i] = 0;
                }
            }
            d.place_arrivals(came_in);
        }
        let n = d.walkers.len() as u32;
        d.wander = Some(Wander {
            stretch,
            came_in: n,
            peak: n,
            exit_dist: vec![u16::MAX; W * H],
            joined: 0,
            touching: false,
            go_at: None,
            set_off: 0,
            food_at_go: 1.0,
            through: 0,
            lost_on_way: 0,
            outcome: None,
            pouring: Vec::new(),
            pour_occ: vec![false; W * H],
        });
        d.census();
        d
    }

    /// `n` walkers in a blob at the left edge, around home, each heading
    /// into the dish and laying a little trail, so she arrives as a body
    /// already joined to the home pile rather than as dust.
    fn place_arrivals(&mut self, n: u32) {
        let (hx, hy) = (self.piles[0].x, self.piles[0].y);
        let (cx, cy) = (hx - 14.0, hy);
        let mut r = (n as f32 / std::f32::consts::PI / 0.7).sqrt().max(10.0);
        let (mut placed, mut misses) = (0, 0);
        while placed < n && r < 120.0 {
            let a = self.rng.f32() * TAU;
            let d = self.rng.f32().sqrt() * r;
            let (x, y) = (cx + a.cos() * d, cy + a.sin() * d);
            match cell(x, y) {
                Some(i) if self.ground[i] == Ground::Agar && self.occ[i] == 0 => {
                    self.occ[i] = 1;
                    // Headed roughly right, into the dish.
                    let heading = (self.rng.f32() - 0.5) * 2.4;
                    self.walkers.push(Walker { x, y, heading });
                    self.trail[i] += self.params.deposit * 2.0;
                    placed += 1;
                }
                _ => {
                    misses += 1;
                    if misses % 400 == 0 {
                        r += 2.0;
                    }
                }
            }
        }
    }

    /// Food left on the dish's own piles, as a fraction of what they started
    /// with (piles still to come left out).
    pub fn food_left(&self) -> f32 {
        let (left, full) = self
            .dish_piles()
            .fold((0.0, 0.0), |(l, f), p| (l + p.food.max(0.0), f + p.full));
        if full > 0.0 {
            left / full
        } else {
            0.0
        }
    }

    /// Go is open: a Wanderer dish, still growing, fed mould in the crack.
    pub fn can_go(&self) -> bool {
        matches!(&self.wander, Some(w) if self.phase == Phase::Growing && w.touching && w.joined > 0)
    }

    /// **Go: she leaves through the crack.** From a fresh feeding search,
    /// every fed walker joined to the crack by tube is lifted out to pour;
    /// everything else is marked unfed and stays. False if Go is not open.
    pub fn go(&mut self) -> bool {
        if !self.can_go() {
            return false;
        }
        self.census();
        self.exit_bfs();
        let Some(mut wd) = self.wander.take() else { return false };
        let mut pouring = Vec::new();
        let mut k = 0;
        while k < self.walkers.len() {
            let w = self.walkers[k];
            let i = cell(w.x, w.y).expect("walker inside the dish");
            if self.fed[i] != u16::MAX && wd.exit_dist[i] != u16::MAX {
                self.occ[i] = 0;
                wd.pour_occ[i] = true;
                self.walkers.swap_remove(k);
                pouring.push((w.x, w.y));
            } else {
                k += 1;
            }
        }
        if pouring.is_empty() {
            // A stale `joined`: nothing fed is joined after all. Nobody was
            // lifted; stay.
            self.wander = Some(wd);
            return false;
        }
        // Nothing left in the dish is fed from here on: what stays cannot
        // divide, and goes grey (`flow_out` draws the line itself yellow).
        self.fed.fill(u16::MAX);
        wd.peak = wd.peak.max(pouring.len() as u32 + self.walkers.len() as u32);
        wd.set_off = pouring.len() as u32;
        wd.go_at = Some(self.t);
        wd.food_at_go = self.food_left();
        wd.pouring = pouring;
        self.wander = Some(wd);
        self.lamp = None;
        self.end_stroke();
        self.update_light();
        self.stamp_fed();
        self.phase = Phase::Crossing(self.t);
        true
    }

    /// After Go, who is fed is settled and the feeding search stops.
    pub(super) fn wander_frozen(&self) -> bool {
        self.wander.as_ref().is_some_and(|w| w.go_at.is_some())
    }

    /// Cells of tube from the crack, over the whole piece joined to it.
    fn exit_bfs(&mut self) {
        let Some(wd) = self.wander.as_mut() else { return };
        let dist = &mut wd.exit_dist;
        dist.fill(u16::MAX);
        let mut queue = std::collections::VecDeque::new();
        for y in 0..H {
            for x in EXIT_X..W {
                let i = idx(x, y);
                if in_exit(x, y) && self.ground[i] == Ground::Agar {
                    dist[i] = 0;
                    queue.push_back(i);
                }
            }
        }
        let tube = self.params.tube;
        while let Some(i) = queue.pop_front() {
            let d = dist[i];
            let (x, y) = (i % W, i / W);
            for (ok, j) in [
                (x > 0, i.wrapping_sub(1)),
                (x + 1 < W, i + 1),
                (y > 0, i.wrapping_sub(W)),
                (y + 1 < H, i + W),
            ] {
                if ok && dist[j] == u16::MAX && self.ground[j] == Ground::Agar && self.trail[j] > tube {
                    dist[j] = d.saturating_add(1);
                    queue.push_back(j);
                }
            }
        }
    }

    /// Every feeding census, while growing: the crack's distance field,
    /// whether fed mould is in it, and how much fed mould is joined to it.
    pub(super) fn wander_census(&mut self) {
        if self.wander.is_none() || self.phase != Phase::Growing {
            return;
        }
        self.exit_bfs();
        let Some(wd) = self.wander.as_mut() else { return };
        let (mut touching, mut joined) = (false, 0);
        for w in &self.walkers {
            let (x, y) = (w.x as usize, w.y as usize);
            let i = idx(x, y);
            if self.fed[i] == u16::MAX {
                continue;
            }
            if in_exit(x, y) {
                touching = true;
            }
            if wd.exit_dist[i] != u16::MAX {
                joined += 1;
            }
        }
        wd.touching = touching;
        wd.joined = joined;
    }

    /// The pour, one step: each walker in the line may starve (it is off
    /// food), else may step down the crack's distance field into a free cell,
    /// laying trail; in the crack it runs right, and through the rim it has
    /// arrived.
    pub(super) fn flow_out(&mut self) {
        if !matches!(self.phase, Phase::Crossing(_)) {
            return;
        }
        let Some(mut wd) = self.wander.take() else { return };
        let dep = self.params.deposit * 2.0;
        let starve = POUR_STARVE;
        // A fresh order every step, so no part of the line always goes first.
        let n = wd.pouring.len();
        for k in (1..n).rev() {
            let j = self.rng.below(k as u32 + 1) as usize;
            wd.pouring.swap(k, j);
        }
        let mut k = 0;
        while k < wd.pouring.len() {
            let (x, y) = wd.pouring[k];
            let (ix, iy) = (x as usize, y as usize);
            let i = idx(ix, iy);
            let roll = self.rng.f32();
            if roll < starve {
                wd.pour_occ[i] = false;
                wd.pouring.swap_remove(k);
                wd.lost_on_way += 1;
                continue;
            }
            if roll >= FLOW_CHANCE {
                k += 1;
                continue;
            }
            let here = wd.exit_dist[i];
            let nb: &[(i32, i32)] = if here == 0 {
                // In the crack: out through the right.
                &[(1, 0), (1, -1), (1, 1)]
            } else {
                &[(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)]
            };
            // The free neighbour nearest the crack; ties broken from a random
            // start, so the line spreads over the tube's width.
            let start = self.rng.below(nb.len() as u32) as usize;
            let mut best: Option<(u16, usize)> = None;
            let mut out = false;
            for o in 0..nb.len() {
                let (dx, dy) = nb[(start + o) % nb.len()];
                let (nx, ny) = (ix as i32 + dx, iy as i32 + dy);
                if nx < 0 || ny < 0 || ny as usize >= H {
                    continue;
                }
                if nx as usize >= W || self.ground[idx(nx as usize, ny as usize)] != Ground::Agar {
                    if here == 0 {
                        out = true;
                    }
                    continue;
                }
                let j = idx(nx as usize, ny as usize);
                let dj = wd.exit_dist[j];
                let closer = if here == 0 { dj == 0 } else { dj < here };
                if closer && !wd.pour_occ[j] && best.is_none_or(|b| dj < b.0) {
                    best = Some((dj, j));
                }
            }
            if out {
                // Through the rim: gone on to the next dish.
                wd.pour_occ[i] = false;
                wd.pouring.swap_remove(k);
                wd.through += 1;
                continue;
            }
            if let Some((_, j)) = best {
                wd.pour_occ[i] = false;
                wd.pour_occ[j] = true;
                self.trail[j] += dep;
                wd.pouring[k] = ((j % W) as f32 + 0.5, (j / W) as f32 + 0.5);
            }
            k += 1;
        }
        // The line is the only fed thing on the dish: the cells beside each
        // walker in it are drawn yellow (`looks_fed`).
        let t = self.t;
        for &(x, y) in &wd.pouring {
            let (x, y) = (x as usize, y as usize);
            for yy in y.saturating_sub(1)..(y + 2).min(H) {
                for xx in x.saturating_sub(1)..(x + 2).min(W) {
                    self.fed_at[idx(xx, yy)] = t;
                }
            }
        }
        self.wander = Some(wd);
    }

    /// Her peak, the starving end, and the end of the pour.
    pub(super) fn wander_tick(&mut self) {
        let t = self.t;
        let alive = self.walkers.len() as u32;
        let phase = self.phase;
        let Some(wd) = self.wander.as_mut() else { return };
        match phase {
            Phase::Growing => {
                wd.peak = wd.peak.max(alive);
                if alive == 0 {
                    wd.outcome = Some(Outcome::Lost { at: t });
                    self.phase = Phase::Over;
                }
            }
            Phase::Crossing(at) => {
                let sps = STEPS_PER_SECOND as u64;
                let done = wd.pouring.is_empty() && t >= at + POUR_MIN_SECONDS as u64 * sps;
                if done || t >= at + POUR_MAX_SECONDS as u64 * sps {
                    // Anyone still in the line at the cap stays behind.
                    wd.lost_on_way += wd.pouring.len() as u32;
                    for &(x, y) in &wd.pouring {
                        wd.pour_occ[idx(x as usize, y as usize)] = false;
                    }
                    wd.pouring.clear();
                    wd.outcome = Some(if wd.through > 0 {
                        Outcome::Arrived {
                            n: wd.through,
                            of: wd.peak,
                        }
                    } else {
                        Outcome::Lost { at: t }
                    });
                    self.phase = Phase::Over;
                }
            }
            _ => {}
        }
    }

    /// The Wanderer's state, if this is a Wanderer dish.
    pub fn wander_state(&self) -> Option<&Wander> {
        self.wander.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Wanderer dish with, laid by hand, a fed body of mould on a pile 30
    /// cells from the crack (every cell of a 16-cell disc), joined to the
    /// crack by a tube `width` + 1 cells wide with mould along it.
    fn bridged(seed: u64, width: usize) -> Dish {
        let mut d = Dish::wanderer(seed, 1, FIRST_WALKERS);
        for _ in 0..STEPS_PER_SECOND {
            d.step();
        }
        let (px, y0) = (205usize, EXIT_Y as usize);
        d.piles[0].x = px as f32 + 0.5;
        d.piles[0].y = EXIT_Y;
        let lay = |d: &mut Dish, x: usize, y: usize| {
            let i = idx(x, y);
            if d.ground[i] == Ground::Agar {
                d.trail[i] = 20.0;
                if d.occ[i] == 0 {
                    d.occ[i] = 1;
                    d.walkers.push(Walker {
                        x: x as f32 + 0.5,
                        y: y as f32 + 0.5,
                        heading: 0.0,
                    });
                }
            }
        };
        for y in y0 - 16..=y0 + 16 {
            for x in px - 16..=px + 16 {
                if (x as f32 - px as f32).hypot(y as f32 - y0 as f32) <= 16.0 {
                    lay(&mut d, x, y);
                }
            }
        }
        for y in y0 - width / 2..=y0 + width / 2 {
            for x in px..252 {
                lay(&mut d, x, y);
            }
        }
        d.census();
        d.wander_census();
        d
    }

    /// Go must be refused until fed mould is in the crack, and must deliver:
    /// the walkers it lifts leave the dish and pour, and the dish ends on
    /// "arrived with N of M", N no more than set off.
    #[test]
    fn go_is_refused_until_the_crack_is_reached_then_delivers() {
        let mut d = Dish::wanderer(1, 1, FIRST_WALKERS);
        for _ in 0..STEPS_PER_SECOND {
            d.step();
        }
        assert!(!d.can_go(), "mould starts at home, far from the crack");
        assert!(!d.go());
        let mut d = bridged(1, 8);
        assert!(d.can_go(), "fed mould in the crack: Go is open");
        let before = d.walkers.len();
        assert!(d.go());
        let set_off = d.wander.as_ref().unwrap().set_off;
        assert_eq!(before - d.walkers.len(), set_off as usize, "the line left the dish");
        while d.phase != Phase::Over {
            d.step();
        }
        let w = d.wander.as_ref().unwrap();
        let Some(Outcome::Arrived { n, of }) = w.outcome else {
            panic!("a short fed bridge arrives: {:?}", w.outcome)
        };
        assert!(n > 0 && n <= set_off && n <= of);
        assert_eq!(n + w.lost_on_way, set_off, "every walker in the line is accounted for");
        assert!(w.pouring.is_empty());
    }

    /// The thicker the trunk, the more comes through: the same bridge one
    /// cell wide loses more of its line than eleven wide. The positive
    /// control for the pour's whole point.
    #[test]
    fn a_thin_bridge_loses_more_on_the_way() {
        let share = |width| {
            let mut d = bridged(5, width);
            assert!(d.go());
            while d.phase != Phase::Over {
                d.step();
            }
            let w = d.wander.as_ref().unwrap();
            w.through as f32 / w.set_off as f32
        };
        let (thin, thick) = (share(0), share(10));
        assert!(thin + 0.1 < thick, "thin {thin:.2} vs thick {thick:.2}");
    }

    /// Nobody touching anything: the food runs out and she dies, and the
    /// dish says so.
    #[test]
    fn a_mould_left_alone_does_not_make_it() {
        let mut d = Dish::wanderer(2, 1, FIRST_WALKERS);
        let cap = 12 * 60 * STEPS_PER_SECOND as u64;
        while d.phase == Phase::Growing && d.t < cap {
            d.step();
        }
        assert!(
            matches!(d.wander.as_ref().unwrap().outcome, Some(Outcome::Lost { .. })),
            "left alone she starves: {} walkers at {} s",
            d.walkers.len(),
            d.t / STEPS_PER_SECOND as u64
        );
    }

    #[test]
    fn carried_walkers_arrive_at_the_left_edge() {
        let d = Dish::wanderer(3, 2, 900);
        assert_eq!(d.walkers.len(), 900);
        assert!(d.walkers.iter().all(|w| w.x < 100.0));
        assert_eq!(d.wander.as_ref().unwrap().came_in, 900);
    }

    /// The sixth pile's late spot at (232, 80) sits in the crack; a
    /// Wanderer dish never puts food there, so the crack always needs a
    /// bridge.
    #[test]
    fn no_food_turns_up_in_the_crack() {
        for seed in 1..40 {
            let d = Dish::wanderer(seed, 1, FIRST_WALKERS);
            for p in d.piles.iter().chain(d.pending.iter().map(|(_, p)| p)) {
                assert!(p.x < 230.0, "seed {seed}: a pile at ({}, {})", p.x, p.y);
            }
        }
    }
}
