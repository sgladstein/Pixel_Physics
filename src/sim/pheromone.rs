//! Two world-sized scalar planes at **CA resolution**, for stigmergy: the
//! signal creatures leave behind for other creatures to read.
//!
//! # Why not a sixth `FieldCell` channel
//!
//! Because it was measured, not assumed. `field.rs` runs at `FIELD_SCALE =
//! 8`, and a sixth channel there would have been much less new code — so
//! `creature::tests::pheromone_resolution_experiment_offset8_tracking` was
//! written first to find out whether it would work. Its verdict:
//!
//! * A Jones follower at sensor offset 8 does find a smeared trail and
//!   stay in its band (0.988 of steps within 2 cells, against a no-trail
//!   control's 0.023). Taken alone this reads as "field scale is fine."
//! * It cannot *travel* the trail. Along-trail progress was 0.052 against
//!   the random walk's 0.262 — headway five times worse than chance,
//!   because it sits oscillating at the block edge. An ant that cannot
//!   commute a trail has not followed it.
//! * **Two trails four cells apart produce a bit-identical field.**
//!   Largest difference across the 17 rows spanning them: 0.0000. Not hard
//!   to resolve — *impossible* to resolve, at any sensor offset, by any
//!   brain. And choosing between competing routes is the entire mechanism
//!   (`Reports/stigmergy-research.md` §2).
//!
//! Trails are the one signal in this engine that genuinely needs fine
//! resolution, which `Reports/emergent-world-architecture.md` §6b predicted
//! and asked to have decided before ants rather than after.
//!
//! # Two channels, and they mean nothing
//!
//! `Channel::A` and `Channel::B` carry **no semantics**. One channel gets
//! milling; two get commuting, because nest-scent plus food-trail is the
//! minimal published configuration that produces real out-and-back
//! foraging. Which is which lives in a species' instinct weights, never
//! here — that is what lets a second species reuse, or parasitize, the same
//! planes. Resist a third until a concrete consumer exists
//! (`stigmergy-research.md` §8's standing rule).
//!
//! # Known limitation
//!
//! World-sized planes are the wrong shape for M10 streaming, where they
//! will need to become per-chunk like `FieldTile`. That is a mechanical
//! migration and is deliberately **not** built speculatively.

use super::chunk::Rect;

/// Frames between diffusion/decay passes. Trails are slow signals — a
/// deposit that took an ant a second to lay does not need resolving at CA
/// granularity, and quartering the pass rate quarters its cost.
pub const PHEROMONE_INTERVAL: u64 = 4;

/// Blend toward the 3x3 mean per pass.
///
/// **Set from measurement, and it is not the 0.1 the design report
/// assumed** (`Reports/creature-direction.md` §5b). At 0.1 on a u8 plane
/// the spread simply does not happen: a continuously re-laid single cell
/// reaches its immediate neighbour and dies, because the blended value at
/// distance 2 rounds to zero before it can propagate. The report's figure
/// was reasoned about in the continuum; quantization is what breaks it.
/// This was caught by the one line in the seam test that asked whether the
/// spread had *crossed* the seam at all — its symmetry assertion was
/// happily comparing two columns of zeroes.
///
/// `trail_following_sweep` measures what the value is actually for — a
/// Jones follower tracking a bent trail, mean over 6 seeds:
///
/// | diffuse | on-trail | traversed | peak of a shared trail |
/// |---|---|---|---|
/// | 0.10 | 0.623 | 0.707 | 216 |
/// | **0.25** | **0.817** | **0.961** | **153** |
/// | 0.50 | 0.820 | 0.975 | 106 |
/// | 1.00 | 0.853 | 0.985 | 63 |
///
/// Tracking keeps improving to 1.0, and 0.25 is still the right pick,
/// because the two ends buy different things (`CLAUDE.md`: when several
/// knobs move the same number, check what each one trades). A full mean
/// filter flattens a shared trail's peak from 153 to 63 — and the
/// *height* of a well-used trail against a lightly-used one is
/// differential reinforcement, which is the entire path-selection
/// algorithm. 0.25 takes 96% of the available tracking for 40% of the
/// peak loss.
pub const DIFFUSE: f32 = 0.25;

/// Evaporation per pass. The literature band is 0.1–0.5 and **this is the
/// parameter the whole mechanism balances on**: too slow and the world
/// ossifies on the first path found, too fast and no trail survives long
/// enough to be reinforced.
///
/// Measured at the bottom of the band and staying there for now — the same
/// sweep puts on-trail at 0.817 for rho 0.1 against 0.613 at 0.25 and
/// 0.633 at 0.40, because a trail evaporating faster than a single ant
/// re-lays it never accumulates. Expect this to want raising once a *whole
/// colony* is depositing on it, which is the condition ossification
/// actually appears under; there is no point tuning it against one
/// follower.
pub const DECAY_RHO: f32 = 0.1;

/// Deposit per successful move, of 255. A trail a dozen ants share should
/// sit well below saturation — differential reinforcement *is* the
/// path-selection algorithm, and it clips flat at the ceiling. P-14: if
/// trails pin at 255, halve this before touching anything else.
pub const DEPOSIT: u8 = 40;

/// Sleep granularity, in cells. Equal to `CHUNK_SIZE` by choice rather than
/// by coupling — nothing here indexes chunks, and a plane that outlived the
/// chunk size would still be correct.
const TILE: usize = 64;

/// Which plane. **Meaning-free by construction** — see the module doc.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Channel {
    A = 0,
    B = 1,
}

/// `decay_lut[v] < v` for every `v > 0`, so evaporation provably reaches
/// zero.
///
/// **A plain multiply cannot promise that on a u8.** `(v as f32 * 0.9) as
/// u8` has a fixed point at small values — 1 x 0.9 = 0.9, truncating to 0,
/// fine; but at `rho` small enough, or with rounding instead of truncation,
/// a low value maps to itself and stays there forever, and the world slowly
/// fills with permanent ghost trails that no amount of waiting clears.
///
/// This is not hypothetical: the canopy-density scalar packed into 4 bits
/// had exactly this failure, measured at 0.800 → 0.533 → 0.267 → 0.267 →
/// 0.267, a fixed point at tick 3, giving every cell that ever received a
/// deposit a permanent floor (`organism::CANOPY_DENSITY_SCALE`'s own doc).
/// It survived because its test asserted about a cell at *full* scale, far
/// away from the floor at the bottom of the range.
///
/// So the strict decrease is forced (`min(v - 1)`) and asserted at
/// construction rather than reasoned about (P-13).
fn build_decay_lut(rho: f32) -> [u8; 256] {
    let mut lut = [0u8; 256];
    for (v, slot) in lut.iter_mut().enumerate().skip(1) {
        let decayed = ((v as f32) * (1.0 - rho)) as u8;
        *slot = decayed.min(v as u8 - 1);
    }
    lut
}

/// One channel's world-sized u8 plane, double-buffered for the pass.
///
/// 512x320 x 1 byte x 2 buffers is ~320 KB per channel, ~640 KB for both.
pub struct PheromonePlane {
    w: usize,
    h: usize,
    /// World coordinate of plane index 0 — the planes cover the world's
    /// bounds `Rect`, which does not have to start at the origin.
    origin: (i32, i32),
    front: Vec<u8>,
    back: Vec<u8>,
    /// Tiles across and down.
    tw: usize,
    th: usize,
    /// Highest value in each tile as of the last pass. A tile at 0 with no
    /// deposits since is skipped entirely — this is "pheromone sleep", the
    /// same law field sleeping exists for.
    tile_max: Vec<u8>,
    /// Set by `deposit`, cleared when the tile is next processed. A tile
    /// that was empty and has just been written to must not be skipped
    /// because its *stale* max still reads 0.
    deposited: Vec<bool>,
    decay_lut: [u8; 256],
    /// Held per plane rather than read from the const inside `step`, so a
    /// sweep can vary it — see `diffusion_spread_profile_sweep`, which is
    /// how `DIFFUSE`'s value was chosen rather than guessed.
    diffuse: f32,
}

impl PheromonePlane {
    fn new(bounds: Rect) -> Self {
        Self::with_params(bounds, DIFFUSE, DECAY_RHO)
    }

    fn with_params(bounds: Rect, diffuse: f32, rho: f32) -> Self {
        let w = (bounds.max_x - bounds.min_x + 1).max(1) as usize;
        let h = (bounds.max_y - bounds.min_y + 1).max(1) as usize;
        let (tw, th) = (w.div_ceil(TILE), h.div_ceil(TILE));
        let lut = build_decay_lut(rho);
        debug_assert!((1..256).all(|v| (lut[v] as usize) < v), "decay LUT must strictly decrease, or evaporation never reaches zero");
        Self {
            w,
            h,
            origin: (bounds.min_x, bounds.min_y),
            front: vec![0; w * h],
            back: vec![0; w * h],
            tw,
            th,
            tile_max: vec![0; tw * th],
            deposited: vec![false; tw * th],
            decay_lut: lut,
            diffuse,
        }
    }

    /// Plane index for a world position, or `None` outside the plane.
    #[inline]
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        let lx = x - self.origin.0;
        let ly = y - self.origin.1;
        if lx < 0 || ly < 0 || lx as usize >= self.w || ly as usize >= self.h {
            return None;
        }
        Some(ly as usize * self.w + lx as usize)
    }

    /// Nearest-cell read. **No interpolation, deliberately**: the plane is
    /// already at CA resolution, which is the entire reason it exists
    /// instead of a field channel. Out-of-plane reads 0, so a creature at
    /// the world edge senses nothing rather than sampling garbage.
    #[inline]
    pub fn sample(&self, x: i32, y: i32) -> u8 {
        self.index(x, y).map_or(0, |i| self.front[i])
    }

    /// Add to a cell, saturating. Out-of-plane deposits are dropped
    /// silently — an ant walking off the edge of the world is not an error.
    fn deposit(&mut self, x: i32, y: i32, amount: u8) -> bool {
        let Some(i) = self.index(x, y) else {
            return false;
        };
        self.front[i] = self.front[i].saturating_add(amount);
        let (tx, ty) = ((x - self.origin.0) as usize / TILE, (y - self.origin.1) as usize / TILE);
        self.deposited[ty * self.tw + tx] = true;
        true
    }

    /// Should this tile be processed at all?
    ///
    /// The 8-neighbour term is the load-bearing part. Without it a trail
    /// stops dead at a tile seam: the tile it is spreading *into* is still
    /// at max 0 with no deposits of its own, so it sleeps, and the trail
    /// develops a hard edge on the grid. Chunk-boundary artifacts are this
    /// codebase's most-repeated root cause (`CLAUDE.md`) and this is the
    /// same shape of mistake, pre-empted.
    fn tile_awake(&self, tx: usize, ty: usize) -> bool {
        let i = ty * self.tw + tx;
        if self.tile_max[i] > 0 || self.deposited[i] {
            return true;
        }
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let (nx, ny) = (tx as i32 + dx, ty as i32 + dy);
                if nx < 0 || ny < 0 || nx as usize >= self.tw || ny as usize >= self.th {
                    continue;
                }
                let n = ny as usize * self.tw + nx as usize;
                if self.tile_max[n] > 0 || self.deposited[n] {
                    return true;
                }
            }
        }
        false
    }

    /// One diffusion + decay pass over the awake tiles. Returns how many
    /// tiles were processed — the "did it fire" number, and the direct
    /// readout on sleeping.
    ///
    /// **Jacobi, front-canonical, two-phase — do not "optimize" this into
    /// an in-place sweep.** Reading and writing the same buffer makes the
    /// result depend on visit order, which costs determinism (required,
    /// `PLAN.md`) and reintroduces exactly the chunk-decomposition
    /// asymmetry that `field.rs` is shaped this way to avoid.
    ///
    /// Skipped tiles are never copied between buffers. That *is* the sleep:
    /// a settled plane touches no memory at all beyond the tile scan.
    fn step(&mut self) -> usize {
        let mut processed = 0;
        // Phase 1: compute into `back`, from `front` only.
        let mut new_max = self.tile_max.clone();
        for ty in 0..self.th {
            for tx in 0..self.tw {
                if !self.tile_awake(tx, ty) {
                    continue;
                }
                processed += 1;
                let mut tile_peak = 0u8;
                let x0 = tx * TILE;
                let y0 = ty * TILE;
                for ly in y0..(y0 + TILE).min(self.h) {
                    for lx in x0..(x0 + TILE).min(self.w) {
                        let here = self.front[ly * self.w + lx] as f32;
                        // 3x3 mean, missing neighbours reading 0 — the
                        // world edge is a sink, matching `sample`.
                        let mut sum = 0.0;
                        for dy in -1i32..=1 {
                            for dx in -1i32..=1 {
                                let (nx, ny) = (lx as i32 + dx, ly as i32 + dy);
                                if nx < 0 || ny < 0 || nx as usize >= self.w || ny as usize >= self.h {
                                    continue;
                                }
                                sum += self.front[ny as usize * self.w + nx as usize] as f32;
                            }
                        }
                        let mean = sum / 9.0;
                        let blended = (here + (mean - here) * self.diffuse).round().clamp(0.0, 255.0) as u8;
                        let out = self.decay_lut[blended as usize];
                        self.back[ly * self.w + lx] = out;
                        tile_peak = tile_peak.max(out);
                    }
                }
                new_max[ty * self.tw + tx] = tile_peak;
            }
        }
        // Phase 2: blit the processed tiles back, and only those.
        for ty in 0..self.th {
            for tx in 0..self.tw {
                let i = ty * self.tw + tx;
                if new_max[i] == self.tile_max[i] && !self.deposited[i] && self.tile_max[i] == 0 {
                    // Untouched sleeping tile: nothing to copy.
                    continue;
                }
                if !self.tile_awake(tx, ty) {
                    continue;
                }
                let x0 = tx * TILE;
                let y0 = ty * TILE;
                for ly in y0..(y0 + TILE).min(self.h) {
                    let row = ly * self.w;
                    let x1 = (x0 + TILE).min(self.w);
                    self.front[row + x0..row + x1].copy_from_slice(&self.back[row + x0..row + x1]);
                }
                self.deposited[i] = false;
            }
        }
        self.tile_max = new_max;
        processed
    }

    /// Highest value anywhere in the plane. For scenes and tests; walks the
    /// whole plane, so not for the hot path.
    pub fn max(&self) -> u8 {
        self.front.iter().copied().max().unwrap_or(0)
    }
}

/// "Did it fire" counters, in the style of `FailureCounts`.
#[derive(Default, Clone, Copy, Debug)]
pub struct PheromoneStats {
    pub deposits_a: u64,
    pub deposits_b: u64,
    pub passes: u64,
    /// Tiles actually processed across every pass. **Zero on a settled
    /// world is the whole design goal**, and a counter is the only thing
    /// that can say so — a timing can be fast for unrelated reasons.
    pub tiles_processed: u64,
}

pub struct Pheromones {
    planes: [PheromonePlane; 2],
    pub stats: PheromoneStats,
}

impl Pheromones {
    pub fn new(bounds: Rect) -> Self {
        Self { planes: [PheromonePlane::new(bounds), PheromonePlane::new(bounds)], stats: PheromoneStats::default() }
    }

    /// Both planes with non-default diffusion/decay — for sweeps only.
    #[cfg(test)]
    fn with_params(bounds: Rect, diffuse: f32, rho: f32) -> Self {
        Self {
            planes: [PheromonePlane::with_params(bounds, diffuse, rho), PheromonePlane::with_params(bounds, diffuse, rho)],
            stats: PheromoneStats::default(),
        }
    }

    #[inline]
    pub fn plane(&self, channel: Channel) -> &PheromonePlane {
        &self.planes[channel as usize]
    }

    #[inline]
    pub fn sample(&self, channel: Channel, x: i32, y: i32) -> u8 {
        self.planes[channel as usize].sample(x, y)
    }

    pub fn deposit(&mut self, channel: Channel, x: i32, y: i32, amount: u8) {
        if amount == 0 {
            return;
        }
        if self.planes[channel as usize].deposit(x, y, amount) {
            match channel {
                Channel::A => self.stats.deposits_a += 1,
                Channel::B => self.stats.deposits_b += 1,
            }
        }
    }

    /// Run a pass on both planes. Callers call this every frame; the
    /// interval gate lives here so no caller has to know about it —
    /// the same shape `World::step_fields` already uses.
    pub fn step(&mut self, frame: u64) {
        if !frame.is_multiple_of(PHEROMONE_INTERVAL) {
            return;
        }
        self.stats.passes += 1;
        for plane in &mut self.planes {
            self.stats.tiles_processed += plane.step() as u64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane_world() -> Pheromones {
        Pheromones::new(Rect::new(0, 0, 255, 191))
    }

    /// Run `n` passes, stepping the frame counter so the interval gate
    /// actually lets each one through.
    fn passes(p: &mut Pheromones, n: usize) {
        for i in 0..n {
            p.step(i as u64 * PHEROMONE_INTERVAL);
        }
    }

    /// The `trail_follow` geometry, shared by the sweep below and by
    /// `examples/ascii.rs`'s scene: a horizontal run with a bend in it, so
    /// tracking has to survive a turn rather than a straight line a
    /// follower could hold by doing nothing at all.
    pub fn trail_y_at(x: i32) -> Option<i32> {
        match x {
            30..=127 => Some(80),
            128..=219 => Some(80 + (x - 128) * 30 / 92),
            _ => None,
        }
    }

    /// One Jones follower over a continuously re-laid trail. Returns the
    /// fraction of steps within 2 cells of it, and the fraction of the
    /// trail's length actually traversed.
    ///
    /// **Both numbers, because proximity alone lies.** The Stage-0
    /// resolution experiment scored 0.988 on proximity while advancing 21
    /// cells in 400 steps — pinned, not commuting. Anything that measures
    /// trail following here reports travel too.
    pub fn run_follower(diffuse: f32, rho: f32, so: i32, steps: usize, seed: u64) -> (f32, f32) {
        run_follower_mode(diffuse, rho, so, steps, seed, true, true)
    }

    /// `normalize`: rescale the three sensor readings across the candidate
    /// set before weighting, so the best scores 1.0 and the worst 0.0.
    /// `trail`: lay one at all (false = the control).
    pub fn run_follower_mode(diffuse: f32, rho: f32, so: i32, steps: usize, seed: u64, normalize: bool, trail: bool) -> (f32, f32) {
        use crate::sim::creature::{choose_weighted, DIRS, CHOICE_EXPLORATION_K};

        let mut p = Pheromones::with_params(Rect::new(0, 0, 255, 159), diffuse, rho);
        // Lay the trail in before the follower starts, so it is not chasing
        // a signal that is still forming.
        for i in 0..30 {
            if trail {
                for x in 30..220 {
                    if let Some(y) = trail_y_at(x) {
                        p.deposit(Channel::B, x, y, DEPOSIT);
                    }
                }
            }
            p.step(i as u64 * PHEROMONE_INTERVAL);
        }

        let (mut px, mut py) = (34i32, 80i32);
        let mut heading: u8 = 0;
        let (mut on_trail, mut furthest) = (0usize, 34i32);
        for step in 0..steps {
            let sense = |h: u8, px: i32, py: i32| {
                let (dx, dy) = DIRS[h as usize % 8];
                p.sample(Channel::B, px + dx * so, py + dy * so) as f32 / 255.0
            };
            let mut scores = [sense((heading + 1) % 8, px, py), sense(heading, px, py), sense((heading + 7) % 8, px, py)];
            if normalize {
                // **The discrimination has to come from the difference, not
                // the level.** On a trail all three sensors read strongly,
                // so raw `(k + s)^2` weights sit within a few percent of
                // each other and the follower random-walks along a signal
                // it can plainly see. Rescaling across the candidate set is
                // what `creature::worm_tick`'s thermotaxis already does,
                // for exactly this reason, and it costs nothing: a flat
                // read still scores all-zero and still falls through to
                // uniform exploration.
                let hi = scores.iter().copied().fold(f32::MIN, f32::max);
                let lo = scores.iter().copied().fold(f32::MAX, f32::min);
                for v in &mut scores {
                    *v = (*v - lo) / (hi - lo + 1e-6);
                }
            }
            let mut rng = crate::sim::rng::stream(seed, 1, step as u64, 0);
            heading = match choose_weighted(&scores, CHOICE_EXPLORATION_K, rng.unit_f32()) {
                0 => (heading + 1) % 8,
                2 => (heading + 7) % 8,
                _ => heading,
            };
            let (dx, dy) = DIRS[heading as usize];
            px = (px + dx).clamp(1, 254);
            py = (py + dy).clamp(1, 158);

            if let Some(ty) = trail_y_at(px) {
                if (py - ty).abs() <= 2 {
                    on_trail += 1;
                    furthest = furthest.max(px);
                }
            }
            // Keep the trail alive under the follower, as ants would.
            if trail {
                for x in 30..220 {
                    if let Some(y) = trail_y_at(x) {
                        p.deposit(Channel::B, x, y, DEPOSIT);
                    }
                }
            }
            p.step(step as u64 * PHEROMONE_INTERVAL);
        }
        (on_trail as f32 / steps as f32, (furthest - 34) as f32 / (219.0 - 34.0))
    }

    /// **How `DIFFUSE` was chosen.** A measurement, not a guard.
    ///
    /// `cargo test --lib diffusion_spread_profile_sweep -- --ignored --nocapture`
    #[test]
    #[ignore = "a measurement, not a guard -- prints profiles, asserts nothing"]
    fn diffusion_spread_profile_sweep() {
        println!("value at distance d from a single deposit of 200, after 12 passes (rho = {DECAY_RHO}):");
        print!("  {:<10}", "diffuse");
        for d in 0..10 {
            print!(" d{d:<4}");
        }
        println!();
        for diffuse in [0.1f32, 0.25, 0.5, 0.75, 1.0] {
            let mut p = Pheromones::with_params(Rect::new(0, 0, 255, 191), diffuse, DECAY_RHO);
            p.deposit(Channel::A, 128, 96, 200);
            for i in 0..12 {
                p.step(i as u64 * PHEROMONE_INTERVAL);
            }
            print!("  {diffuse:<10.2}");
            for d in 0..10 {
                print!(" {:<5}", p.sample(Channel::A, 128 + d, 96));
            }
            println!();
        }
        println!();
        println!("and the same, for a continuously re-laid one-cell trail (deposit {DEPOSIT}/pass along y = 96), profile across it:");
        for diffuse in [0.1f32, 0.25, 0.5, 0.75, 1.0] {
            let mut p = Pheromones::with_params(Rect::new(0, 0, 255, 191), diffuse, DECAY_RHO);
            for i in 0..40 {
                for x in 40..220 {
                    p.deposit(Channel::A, x, 96, DEPOSIT);
                }
                p.step(i as u64 * PHEROMONE_INTERVAL);
            }
            print!("  {diffuse:<10.2}");
            for d in 0..10 {
                print!(" {:<5}", p.sample(Channel::A, 128, 96 + d));
            }
            println!();
        }
    }

    /// The sweep that actually decides it: tracking a bent trail, which is
    /// what the constant is *for*. The profile sweep above says how wide
    /// the signal is; this says whether a follower can use it.
    ///
    /// `cargo test --lib trail_following_sweep -- --ignored --nocapture`
    #[test]
    #[ignore = "a measurement, not a guard -- prints a sweep, asserts nothing"]
    fn trail_following_sweep() {
        println!("Jones follower on a bent trail: mean (on-trail fraction, trail traversed) over 6 seeds");
        println!("  {:<8} {:<6} {:<16} {:<16}", "diffuse", "rho", "on-trail", "traversed");
        for &(diffuse, rho) in &[(0.1f32, 0.1f32), (0.25, 0.1), (0.5, 0.1), (0.75, 0.1), (1.0, 0.1), (0.5, 0.25), (0.5, 0.4)] {
            let runs: Vec<(f32, f32)> = (0..6).map(|s| run_follower(diffuse, rho, 6, 400, 0xA11 + s)).collect();
            let on = runs.iter().map(|r| r.0).sum::<f32>() / runs.len() as f32;
            let tv = runs.iter().map(|r| r.1).sum::<f32>() / runs.len() as f32;
            let worst = runs.iter().map(|r| r.0).fold(f32::MAX, f32::min);
            println!("  {diffuse:<8.2} {rho:<6.2} {on:<6.3} (worst {worst:.3})  {tv:<6.3}");
        }
        println!();
        println!("raw sensor levels vs rescaled-across-candidates, and the no-trail control:");
        for &(label, norm, trail) in &[("raw", false, true), ("normalized", true, true), ("CONTROL no trail", true, false)] {
            let runs: Vec<(f32, f32)> = (0..6).map(|s| run_follower_mode(0.25, 0.1, 6, 400, 0xA11 + s, norm, trail)).collect();
            let on = runs.iter().map(|r| r.0).sum::<f32>() / runs.len() as f32;
            let tv = runs.iter().map(|r| r.1).sum::<f32>() / runs.len() as f32;
            let worst = runs.iter().map(|r| r.0).fold(f32::MAX, f32::min);
            println!("  {label:<18} on-trail {on:.3} (worst {worst:.3})  traversed {tv:.3}");
        }
        println!();
        println!("sensor offset sweep, normalized, diffuse 0.25 / rho 0.1:");
        for so in [2, 3, 4, 6, 8, 10] {
            let runs: Vec<(f32, f32)> = (0..6).map(|s| run_follower(0.25, 0.1, so, 400, 0xA11 + s)).collect();
            let on = runs.iter().map(|r| r.0).sum::<f32>() / runs.len() as f32;
            let tv = runs.iter().map(|r| r.1).sum::<f32>() / runs.len() as f32;
            let worst = runs.iter().map(|r| r.0).fold(f32::MAX, f32::min);
            println!("  SO {so:<3} on-trail {on:.3} (worst {worst:.3})  traversed {tv:.3}");
        }
    }

    #[test]
    fn decay_lut_strictly_decreases_every_nonzero_value() {
        // P-13, asserted over the whole domain rather than sampled: the
        // canopy-density fixed point lived at the *bottom* of its range and
        // its test looked at the top.
        for rho in [0.05f32, 0.1, 0.3, 0.5, 0.9] {
            let lut = build_decay_lut(rho);
            assert_eq!(lut[0], 0);
            for (v, &out) in lut.iter().enumerate().skip(1) {
                assert!((out as usize) < v, "rho={rho}: lut[{v}] = {out} did not decrease");
            }
        }
    }

    #[test]
    fn a_deposit_decays_to_exactly_zero_and_never_increases() {
        let mut p = plane_world();
        p.deposit(Channel::A, 100, 100, 200);
        let mut previous = p.plane(Channel::A).max();
        assert_eq!(previous, 200);

        // 255 forced decrements is the worst case even if diffusion did
        // nothing at all, so this bound cannot be the thing that fails.
        for pass in 0..400 {
            p.step(pass * PHEROMONE_INTERVAL);
            let now = p.plane(Channel::A).max();
            assert!(now <= previous, "pass {pass}: plane max rose from {previous} to {now}");
            previous = now;
        }
        assert_eq!(p.plane(Channel::A).max(), 0, "evaporation must reach exactly zero, or the world fills with ghost trails");
    }

    #[test]
    fn deposits_saturate_instead_of_wrapping() {
        // A wrap here is worse than a clip: a heavily-used trail would read
        // as *empty*, which is the strongest possible signal pointing the
        // opposite way.
        let mut p = plane_world();
        for _ in 0..10 {
            p.deposit(Channel::A, 50, 50, 200);
        }
        assert_eq!(p.sample(Channel::A, 50, 50), 255);
    }

    #[test]
    fn a_settled_plane_processes_zero_tiles() {
        // **The sleep guard, as a counter rather than a timing.** A timing
        // can be fast because the machine was idle; a tile count cannot be
        // zero unless the work genuinely did not happen (`CLAUDE.md`: "did
        // it fire at all" needs a counter).
        let mut p = plane_world();
        passes(&mut p, 4);
        assert_eq!(p.stats.tiles_processed, 0, "an untouched plane must not process a single tile");

        p.deposit(Channel::A, 100, 100, 200);
        passes(&mut p, 1);
        assert!(p.stats.tiles_processed > 0, "a plane with a deposit in it must process something");

        // Let it drain completely, then confirm it goes back to sleep
        // rather than staying awake forever once disturbed.
        for pass in 0..600 {
            p.step(pass * PHEROMONE_INTERVAL);
        }
        assert_eq!(p.plane(Channel::A).max(), 0);
        let before = p.stats.tiles_processed;
        passes(&mut p, 10);
        assert_eq!(p.stats.tiles_processed, before, "a drained plane must return to processing zero tiles");
    }

    #[test]
    fn a_tile_seam_does_not_block_or_bias_diffusion() {
        // Chunk/tile decomposition is this codebase's most-repeated root
        // cause. Two deposits mirrored across the seam at x = 64 must
        // spread identically; if the wake rule ignored neighbouring tiles,
        // the one at x = 63 would have a hard edge on its right.
        // **A re-laid *line*, not a single blob.** Two earlier versions of
        // this test were vacuous and only the "did it actually cross" line
        // caught them. A one-shot deposit evaporates before it spreads at
        // all; a single re-laid *cell* has too little mass to push a
        // rounded u8 past its immediate neighbour. A trail in play is a
        // continuously re-laid line, which is both the realistic case and
        // the only one with a spread that could be biased in the first
        // place. This is `CLAUDE.md`'s "ask what a metric counts when
        // nothing is wrong", twice over.
        let mut p = plane_world();
        for i in 0..24 {
            for y in 80..120 {
                p.deposit(Channel::A, 63, y, DEPOSIT);
                p.deposit(Channel::B, 64, y, DEPOSIT);
            }
            p.step(i as u64 * PHEROMONE_INTERVAL);
        }

        for d in 0..8 {
            let left = p.sample(Channel::A, 63 - d, 100);
            let right = p.sample(Channel::B, 64 + d, 100);
            assert_eq!(left, right, "spread at distance {d} differs across the seam: {left} vs {right}");
        }
        // And the spread genuinely crossed the seam, or every assertion
        // above was comparing two columns of zeroes.
        assert!(p.sample(Channel::A, 65, 100) > 0, "channel A's line at x=63 never spread past the seam at x=64 -- the symmetry check above was vacuous");
        assert!(p.sample(Channel::B, 62, 100) > 0, "channel B's line at x=64 never spread back past the seam");
    }

    #[test]
    fn a_follower_tracks_a_bent_trail_far_better_than_it_tracks_nothing() {
        // **Gated on the mean over six seeds, not on a run.** Outcome
        // spread here is enormous — the same configuration that means 0.817
        // has a worst single seed of 0.447 — so a bar set from one run is a
        // sample from a wide distribution and flakes in whichever direction
        // that run landed (`CLAUDE.md`). The mean is stable; a single seed
        // is not, and a per-seed baseline would get rubber-stamped.
        //
        // Bar 0.70, from a measured 0.817 with headroom. That happens to
        // equal the figure `Reports/creature-direction.md` §9a asks for,
        // which is a coincidence worth stating rather than a target that
        // was aimed at: the report's 0.70 was written before anything was
        // built, and the honest version of this bar is "below what the
        // engine measures", which it is.
        //
        // The control is what makes the number mean anything. The same
        // follower on an empty plane scores 0.050 — so this is 16x, not a
        // fraction that merely sounds high.
        let mean = |norm: bool, trail: bool| {
            let runs: Vec<(f32, f32)> = (0..6).map(|s| run_follower_mode(DIFFUSE, DECAY_RHO, 6, 400, 0xA11 + s, norm, trail)).collect();
            (runs.iter().map(|r| r.0).sum::<f32>() / 6.0, runs.iter().map(|r| r.1).sum::<f32>() / 6.0)
        };
        let (on_trail, traversed) = mean(true, true);
        let (control, _) = mean(true, false);

        assert!(on_trail >= 0.70, "follower stayed within 2 cells of the trail on only {on_trail:.3} of steps (measured 0.817 when this bar was set)");
        assert!(
            traversed >= 0.70,
            "follower reached only {traversed:.3} of the trail's length (measured 0.961). Proximity without travel is the Stage-0 failure: pinned at the start, scoring well, going nowhere"
        );
        assert!(on_trail > control * 5.0, "tracking {on_trail:.3} is not meaningfully better than the no-trail control {control:.3}");
    }

    #[test]
    fn two_identical_runs_are_byte_identical() {
        // Determinism is required (same build, `PLAN.md`), and a Jacobi
        // pass is the easy case — this guards against someone later making
        // it in-place, where visit order would start to matter.
        let run = || {
            let mut p = plane_world();
            for i in 0..20 {
                p.deposit(Channel::A, 40 + i * 3, 90 + (i % 7), 90);
                p.deposit(Channel::B, 200 - i * 2, 120, 70);
                p.step(i as u64 * PHEROMONE_INTERVAL);
            }
            let a: Vec<u8> = (0..256).map(|x| p.sample(Channel::A, x, 90)).collect();
            let b: Vec<u8> = (0..256).map(|x| p.sample(Channel::B, x, 120)).collect();
            (a, b)
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn out_of_world_samples_read_zero_and_out_of_world_deposits_are_dropped() {
        let mut p = plane_world();
        p.deposit(Channel::A, -5, 100, 200);
        p.deposit(Channel::A, 1000, 100, 200);
        assert_eq!(p.sample(Channel::A, -5, 100), 0);
        assert_eq!(p.sample(Channel::A, 1000, 100), 0);
        assert_eq!(p.sample(Channel::A, 100, -1), 0);
        assert_eq!(p.plane(Channel::A).max(), 0, "an out-of-plane deposit must not land anywhere");
        assert_eq!(p.stats.deposits_a, 0, "and must not be counted as one");
    }

    #[test]
    fn the_two_channels_are_independent() {
        // They are meaning-free, but they are not the same plane: a design
        // that accidentally aliased them would still pass every test above.
        let mut p = plane_world();
        p.deposit(Channel::A, 100, 100, 200);
        passes(&mut p, 3);
        assert!(p.sample(Channel::A, 100, 100) > 0);
        assert_eq!(p.sample(Channel::B, 100, 100), 0);
    }

    #[test]
    fn the_interval_gate_actually_gates() {
        // The `include_str!` lesson in a different costume: a knob that is
        // never connected produces identical output at every setting, and
        // reads as "the mechanism is fine".
        let mut p = plane_world();
        p.deposit(Channel::A, 100, 100, 200);
        for frame in 1..PHEROMONE_INTERVAL {
            p.step(frame);
        }
        assert_eq!(p.stats.passes, 0, "off-interval frames must not run a pass");
        p.step(PHEROMONE_INTERVAL);
        assert_eq!(p.stats.passes, 1);
    }
}
