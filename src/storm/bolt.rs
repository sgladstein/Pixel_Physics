//! **How a bolt finds its way down** -- the growing tip, without a charge
//! solve.
//!
//! The probe brief (`prototypes/1-lightning.md`, 2026-11-07) says skip the
//! field solve and grow a tip that weighs each neighbour by four things: a
//! downward pull, how well the cell conducts, a pull toward the nearest
//! conductor, and fresh ions. The weight is raised to a *straightness* power
//! and sampled, so a light bolt wanders and forks and a heavy one runs true.
//!
//! Everything here reads a [`Medium`], a snapshot of the world taken by the
//! game once per tick while a bolt is being called. Nothing in this file
//! touches a `World`, which is what makes it testable on a hand-built grid
//! and cheap to call from the headless probe.

use crate::sim::rng::Rng;

/// What one cell is, as far as a bolt cares.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stuff {
    /// Open air: the bolt travels through it.
    Air,
    /// Glass: a bolt can never enter it, and no conductor pulls through it.
    /// This is what makes old misses into a maze.
    Glass,
    /// Anything else. Touching it ends the air leg of the bolt.
    Ground,
}

/// A snapshot of the world for one tick of bolt growth.
pub struct Medium {
    pub w: i32,
    pub h: i32,
    pub stuff: Vec<Stuff>,
    /// How well each cell conducts, 0 (glass) to 1 (iron).
    pub cond: Vec<f32>,
    /// Steps through air to the nearest good conductor, weighted so a
    /// better conductor counts as nearer. `u16::MAX` where none can be
    /// reached (sealed by glass).
    pub reach: Vec<u16>,
    /// Fresh ions from earlier bolts, 0..1, fading over about 3 seconds.
    pub ions: Vec<f32>,
}

/// A conductor this good or better pulls bolts toward it.
pub const PULL_FROM: f32 = 0.45;

impl Medium {
    #[inline]
    pub fn idx(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.w && y < self.h).then(|| (y * self.w + x) as usize)
    }

    /// Rebuild [`Self::reach`]: a bucket-queue Dijkstra from every good
    /// conductor that touches air, outward through air only. A better
    /// conductor starts with a smaller cost, so iron out-pulls wet soil at
    /// the same distance. `only` restricts the sources to a set of cells (the
    /// cloud's own choice when it strikes by itself).
    pub fn build_reach(&mut self, only: Option<&[usize]>) {
        let n = (self.w * self.h) as usize;
        self.reach.clear();
        self.reach.resize(n, u16::MAX);
        const MAXC: usize = 1024;
        let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); MAXC];
        let seed = |i: usize, reach: &mut Vec<u16>, buckets: &mut Vec<Vec<u32>>| {
            let c = self.cond[i];
            let start = (((1.0 - c.min(1.0)) * 24.0) as u16).min(40);
            if start < reach[i] {
                reach[i] = start;
                buckets[start as usize].push(i as u32);
            }
        };
        match only {
            Some(list) => {
                for &i in list {
                    seed(i, &mut self.reach, &mut buckets);
                }
            }
            None => {
                for i in 0..n {
                    if self.stuff[i] == Stuff::Ground && self.cond[i] >= PULL_FROM {
                        seed(i, &mut self.reach, &mut buckets);
                    }
                }
            }
        }
        let (w, h) = (self.w, self.h);
        for b in 0..MAXC {
            let mut k = 0;
            while k < buckets[b].len() {
                let i = buckets[b][k] as usize;
                k += 1;
                if self.reach[i] as usize != b {
                    continue;
                }
                let (x, y) = ((i as i32) % w, (i as i32) / w);
                for (dx, dy, cost) in [(1, 0, 2), (-1, 0, 2), (0, 1, 2), (0, -1, 2), (1, 1, 3), (-1, 1, 3), (1, -1, 3), (-1, -1, 3)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    let j = (ny * w + nx) as usize;
                    if self.stuff[j] != Stuff::Air {
                        continue;
                    }
                    let nb = b + cost;
                    if nb < MAXC && (nb as u16) < self.reach[j] {
                        self.reach[j] = nb as u16;
                        buckets[nb].push(j as u32);
                    }
                }
            }
        }
    }
}

/// How the cloud is being asked to strike.
#[derive(Clone, Copy, Debug)]
pub struct Push {
    /// 0 (a tap) to 1 (a full hold). Straighter, fewer forks, deeper scar.
    pub heavy: f32,
    /// Extra weight on the conductor pull. The cloud striking by itself uses
    /// a large value with `reach` built from the tower alone.
    pub pull: f32,
    /// How far the pull reaches, in reach units (two per cell). A called
    /// bolt's is short -- see [`CALLED_RANGE`] -- and the cloud's own strike
    /// onto the bell reaches across the whole valley.
    pub range: f32,
}

/// **The conductor pull on a called bolt fades over about seven cells.**
/// Measured with `stormprobe mode=aim`, ten seeds per cell of the table:
/// with a long-range pull (a constant term plus a 35-cell fade) the field
/// lamp took **every** bolt aimed up to 20 cells beside it at every hold,
/// 10/10 in all twelve cells -- a magnet, not an aim -- and bolts aimed at
/// open sand 35 cells away landed on the lamp too. Raised to the straightness
/// power, even a small pull wins every step of a 90-step fall. At this range
/// the lamp takes 9-10/10 aimed at it, 5/10 from 12 cells with a tap and
/// 9/10 with a hold, and over open sand a tap lands a median 19 cells off
/// against 6 for a medium hold: hits fall with distance and rise with hold.
pub const CALLED_RANGE: f32 = 14.0;

impl Push {
    /// The straightness power the weights are raised to.
    pub fn power(self) -> f32 {
        1.0 + 5.5 * self.heavy.clamp(0.0, 1.0)
    }
}

/// The eight neighbours a tip may step to. Upward steps are allowed but
/// weighted near nothing, which lets a bolt hook *up* under a ledge only when
/// a conductor is right there.
const STEPS: [(i32, i32); 8] = [(0, 1), (-1, 1), (1, 1), (-1, 0), (1, 0), (-1, -1), (1, -1), (0, -1)];

/// The score of stepping from `from` to `to` in direction `d`, before the
/// power is applied. `last` is the previous step, for a little momentum so
/// a bolt runs in jagged segments instead of a uniform drizzle.
fn score(m: &Medium, from: usize, to: usize, d: (i32, i32), last: (i32, i32), push: Push) -> f32 {
    let down = match d {
        (0, 1) => 1.0,
        (_, 1) => 0.75,
        (_, 0) => 0.22,
        _ => 0.02,
    };
    let rf = m.reach[from];
    let rt = m.reach[to];
    // The conductor pull is strong near a conductor and fades with distance,
    // which is what lets a bolt fall straight from high up and then *bend*
    // in its last stretch -- the readable shape.
    let pull = if rf == u16::MAX || rt == u16::MAX {
        0.0
    } else {
        let gain = (rf as f32 - rt as f32).clamp(-3.0, 3.0) / 3.0;
        let near = (-(rf as f32) / push.range).exp();
        gain * 1.8 * near * push.pull
    };
    let ion = m.ions[to] * 0.9;
    // Light bolts keep their heading longer, so they run in long crooked
    // segments; heavy ones lean on the power instead and run true.
    //
    // **Momentum must lose to straight-down at full weight.** At 0.18 for a
    // heavy bolt, a diagonal step plus momentum (0.41 + 0.18) out-scored
    // straight down (0.55), so one diagonal step became a diagonal run:
    // full-weight bolts over open sand landed 16, 19 and 35 cells off their
    // aim in ten seeds. At 0.08 a heavy bolt's diagonal-plus-momentum is
    // 0.49 and it rights itself; a light bolt's is 0.84 and it still runs.
    let keep = if d == last { 0.08 + 0.35 * (1.0 - push.heavy.clamp(0.0, 1.0)) } else { 0.0 };
    let touch = if m.stuff[to] == Stuff::Ground { 0.25 + m.cond[to] } else { 0.0 };
    (0.55 * down + pull + ion + keep + touch).max(0.0) + 0.01
}

/// Choose the next step from `tip`, or `None` if boxed in.
pub fn choose(m: &Medium, tip: (i32, i32), last: (i32, i32), push: Push, rng: &mut Rng, greedy: bool) -> Option<(i32, i32)> {
    let from = m.idx(tip.0, tip.1)?;
    let p = push.power();
    let mut best = None;
    let mut best_w = 0.0f32;
    let mut cand: [(f32, (i32, i32)); 8] = [(0.0, (0, 0)); 8];
    let mut total = 0.0;
    for (k, &d) in STEPS.iter().enumerate() {
        let (nx, ny) = (tip.0 + d.0, tip.1 + d.1);
        let Some(to) = m.idx(nx, ny) else { continue };
        if m.stuff[to] == Stuff::Glass {
            continue;
        }
        let w = score(m, from, to, d, last, push).powf(p);
        cand[k] = (w, d);
        total += w;
        if w > best_w {
            best_w = w;
            best = Some(d);
        }
    }
    if total <= 0.0 {
        return None;
    }
    if greedy {
        return best;
    }
    let mut r = rng.unit_f32() * total;
    for (w, d) in cand {
        if w <= 0.0 {
            continue;
        }
        if r <= w {
            return Some(d);
        }
        r -= w;
    }
    best
}

/// A path through air, cell by cell, and where it ended.
#[derive(Clone, Debug, Default)]
pub struct Path {
    pub cells: Vec<(i32, i32)>,
    /// The ground cell it touched, if it touched one.
    pub hit: Option<(i32, i32)>,
}

/// Grow from `start` until the path touches ground or runs out of steps.
/// `last` seeds the momentum. When `forks` is given, side branches are
/// grown off the way down and pushed onto it -- they are drawn, and where
/// they touch ground they leave a small mark, but only the main path
/// carries the strike.
pub fn grow(m: &Medium, start: (i32, i32), last: (i32, i32), push: Push, rng: &mut Rng, max_steps: usize, mut forks: Option<&mut Vec<Path>>) -> Path {
    let mut path = Path::default();
    let mut tip = start;
    let mut d = last;
    let fork_p = 0.035 * (1.0 - push.heavy.clamp(0.0, 1.0)) + 0.008;
    for _ in 0..max_steps {
        let Some(step) = choose(m, tip, d, push, rng, false) else { break };
        let next = (tip.0 + step.0, tip.1 + step.1);
        let i = m.idx(next.0, next.1).expect("choose only returns in-bounds steps");
        if m.stuff[i] == Stuff::Ground {
            path.hit = Some(next);
            break;
        }
        path.cells.push(next);
        tip = next;
        d = step;
        if let Some(f) = forks.as_deref_mut() {
            if f.len() < 10 && rng.chance(fork_p) {
                let light = Push { heavy: push.heavy * 0.3, pull: push.pull * 0.6, range: push.range };
                let side = if rng.flip() { (-1, 1) } else { (1, 1) };
                let len = 10 + rng.below(40) as usize;
                let b = grow(m, tip, side, light, rng, len, None);
                if !b.cells.is_empty() {
                    f.push(b);
                }
            }
        }
    }
    path
}

/// The most likely way down from `tip`: the greedy path, shown dotted while
/// the player holds. Stops at ground or after `max_steps`.
pub fn likely(m: &Medium, tip: (i32, i32), _last: (i32, i32), push: Push, max_steps: usize) -> Path {
    let mut rng = Rng::new(1);
    let mut path = Path::default();
    let mut at = tip;
    let mut seen = std::collections::HashSet::new();
    // **No momentum in the preview.** With it, the greedy path of a light
    // hold took whichever diagonal it started on and kept it, drawing a long
    // zig-zag no real bolt follows; the measured bolts at that weight fall
    // within a few cells of straight down. `(9, 9)` is no real step, so the
    // momentum term never fires, and `_last` is taken only so a caller does
    // not have to know that.
    for _ in 0..max_steps {
        let Some(step) = choose(m, at, (9, 9), push, &mut rng, true) else { break };
        let next = (at.0 + step.0, at.1 + step.1);
        if !seen.insert(next) {
            break;
        }
        let i = m.idx(next.0, next.1).expect("in bounds");
        if m.stuff[i] == Stuff::Ground {
            path.hit = Some(next);
            break;
        }
        path.cells.push(next);
        at = next;
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat floor at `floor`, air above, one iron post at `post_x`.
    fn medium(w: i32, h: i32, floor: i32, post_x: Option<i32>) -> Medium {
        let n = (w * h) as usize;
        let mut m = Medium { w, h, stuff: vec![Stuff::Air; n], cond: vec![0.0; n], reach: Vec::new(), ions: vec![0.0; n] };
        for y in floor..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                m.stuff[i] = Stuff::Ground;
                m.cond[i] = 0.2;
            }
        }
        if let Some(px) = post_x {
            for y in (floor - 20)..floor {
                let i = (y * w + px) as usize;
                m.stuff[i] = Stuff::Ground;
                m.cond[i] = 1.0;
            }
        }
        m.build_reach(None);
        m
    }

    #[test]
    fn a_bolt_reaches_the_ground() {
        let m = medium(80, 60, 50, None);
        let mut rng = Rng::new(7);
        let p = grow(&m, (40, 0), (0, 1), Push { heavy: 0.5, pull: 1.0, range: CALLED_RANGE }, &mut rng, 400, None);
        assert!(p.hit.is_some(), "a bolt in open air must find the floor");
    }

    /// The question the probe asks: does a heavier bolt land closer to where
    /// it was called? Mean landing offset over 40 seeds, light against heavy.
    #[test]
    fn a_heavy_bolt_lands_nearer_its_aim() {
        let m = medium(160, 100, 90, None);
        let spread = |heavy: f32| {
            let mut sum = 0.0;
            for s in 0..40 {
                let mut rng = Rng::new(100 + s);
                let p = grow(&m, (80, 0), (0, 1), Push { heavy, pull: 1.0, range: CALLED_RANGE }, &mut rng, 800, None);
                sum += (p.hit.expect("lands").0 - 80).abs() as f32;
            }
            sum / 40.0
        };
        let (light, heavy) = (spread(0.0), spread(1.0));
        assert!(heavy < light * 0.7, "heavy {heavy:.1} should land much nearer the aim than light {light:.1}");
    }

    /// A post near the aim pulls the bolt onto it; the same aim with no post
    /// lands on the floor. Guards the conductor pull being connected at all.
    #[test]
    fn an_iron_post_draws_the_bolt() {
        let with = medium(160, 100, 90, Some(92));
        let mut hits = 0;
        for s in 0..20 {
            let mut rng = Rng::new(300 + s);
            let p = grow(&with, (80, 0), (0, 1), Push { heavy: 0.5, pull: 1.0, range: CALLED_RANGE }, &mut rng, 800, None);
            if p.hit.is_some_and(|(x, _)| x == 92) {
                hits += 1;
            }
        }
        assert!(hits >= 14, "the post 12 cells off the aim should take most bolts, took {hits}/20");
    }

    /// Glass is never entered and nothing pulls through it.
    #[test]
    fn glass_is_never_entered() {
        let mut m = medium(80, 60, 50, None);
        for x in 30..50 {
            let i = (30 * 80 + x) as usize;
            m.stuff[i] = Stuff::Glass;
        }
        m.build_reach(None);
        for s in 0..20 {
            let mut rng = Rng::new(s);
            let p = grow(&m, (40, 0), (0, 1), Push { heavy: 0.3, pull: 1.0, range: CALLED_RANGE }, &mut rng, 800, None);
            assert!(p.cells.iter().all(|&(x, y)| !(y == 30 && (30..50).contains(&x))));
        }
    }
}
