//! **The gas, on the fine grid.** One concentration per cell, 0 (clean air)
//! to 1 (nothing but gas), on the same 256x160 grid as the rock.
//!
//! Deliberately *not* the engine's air grid (`sim::field`): that works in
//! 16x16 blocks, so a gallery ten cells tall is half of one block and a door
//! three cells thick cannot stop it, and heat does not push it. The brief
//! (`prototypes/3-firedamp.md`) and the engine review both say the same:
//! keep the gas and its pushes on the fine grid, inside the game. Nothing in
//! here is read by any other binary.
//!
//! **What the gas does each tick**, in this order:
//! 1. the leak feeds it;
//! 2. fans carry it along their cones, and stir it (mix it with air);
//! 3. it sinks -- firedamp here is *heavier* than air, so it pools in the
//!    sump like slow water and climbs toward the faces as the pool rises.
//!    Rising gas is the obvious perk to try later;
//! 4. it spreads (diffusion, faster sideways than up and down so a pool
//!    levels off);
//! 5. it escapes at the top of the air shaft;
//! 6. flame runs through it (see [`Gas::step_flame`]).
//!
//! **The burn band is the whole game.** Below [`BURN_LO`] it is too thin to
//! burn; above [`BURN_HI`] it is too thick. Only the middle burns, and a
//! flame front stops where the band stops. The three are drawn in three fixed
//! colours, never blended, so a player can read the band off the screen.

pub const W: usize = 256;
pub const H: usize = 160;
pub const N: usize = W * H;

/// Under this, the cell reads as clean air and is not drawn.
pub const CLEAR: f32 = 0.04;
/// The bottom of the burn band: thinner than this will not carry a flame.
pub const BURN_LO: f32 = 0.20;
/// The top of the burn band: thicker than this will not burn either.
pub const BURN_HI: f32 = 0.55;

/// How fast gas sinks, as the share of a cell's gas that moves one cell down
/// per tick when the cell below is empty. Scaled by the room below, `1 - c`,
/// so a full cell takes nothing more and the pool stacks instead of crushing.
const SINK: f32 = 0.18;
/// Spreading sideways, per neighbour pair per tick.
const SPREAD_H: f32 = 0.20;
/// Spreading up and down. Smaller than sideways so a pool levels off rather
/// than smearing up the walls.
const SPREAD_V: f32 = 0.07;
/// The most a pair may exchange per tick once a fan's stir is added. Four
/// neighbours at this keep the explicit update positive.
const SPREAD_MAX: f32 = 0.24;
/// How far toward its cone's average a fully churned cell moves per tick.
const CHURN: f32 = 0.07;

/// Ticks a cell burns for once lit.
pub const BURN_TICKS: u8 = 7;
/// Gas eaten per burning tick.
const BURN_EAT: f32 = 0.09;
/// Ticks a burnt cell glows after burning. Since `EMBER_CATCH` is zero this
/// is the afterglow on screen, and the window in which a pending bang counts
/// its heart as reached.
pub const EMBER_TICKS: u8 = 24;
/// Chance per tick that a burning neighbour lights a burnable cell. Under
/// one, so the front is ragged; at this it crosses a 40-cell pocket in under
/// a second, faster than the gas around it moves.
const FLAME_CATCH: f32 = 0.85;
/// **A flame burns out after handing itself on this many times.** Long
/// enough to cross any pocket in the mine (the sump is ~45 cells across, and
/// a hand-on can go diagonally), so it does not stop a flame short of the
/// yellow's edge. What it stops is a
/// flame that never ends: the leak keeps making fresh burnable gas right
/// beside a flame sitting on it, so with no limit one match lit the crack as
/// a torch that burnt the whole leak off for the rest of the shift (seen
/// 2026-11-07, seed 1: one light, burning on every one of the next 3,000
/// ticks, both faces safe with nothing else done). At 160 a lit crack
/// still burnt for 25 s; at 64 it still flared for 19-21 s on two mines in four; at 48 for 13-14 s;
/// at 32 for 6-9 s, and the planned player still clears every pocket it
/// lights (12 mines: ore 194-198, the same as at 64).
pub const FLAME_REACH: u8 = 32;
/// Chance per tick that a hot (just burnt) neighbour lights a burnable cell.
///
/// **Zero, measured.** At 0.05 a fan churning the sump and one fuse made a
/// standing flame: the leak's gas was stirred into the band beside the
/// embers and relit as fast as it came, so the sump never filled, no pocket
/// ever went bang, and the faces were safe for the whole shift with nothing
/// else done (4 of 4 seeds, 2026-11-07). A flame that only runs through gas
/// that is burnable *now* keeps every light a decision.
const EMBER_CATCH: f32 = 0.0;

/// Which of the three readings a concentration gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    Clear,
    Thin,
    Burnable,
    Thick,
}

pub fn band(c: f32) -> Band {
    if c < CLEAR {
        Band::Clear
    } else if c < BURN_LO {
        Band::Thin
    } else if c <= BURN_HI {
        Band::Burnable
    } else {
        Band::Thick
    }
}

pub fn burnable(c: f32) -> bool {
    (BURN_LO..=BURN_HI).contains(&c)
}

#[inline]
pub fn idx(x: i32, y: i32) -> usize {
    y as usize * W + x as usize
}

#[inline]
pub fn inside(x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && (x as usize) < W && (y as usize) < H
}

/// A deterministic coin, so a seed replays exactly.
#[inline]
pub fn coin(seed: u64, tick: u64, i: usize) -> f32 {
    let mut z = seed ^ tick.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (i as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

pub struct Gas {
    pub c: Vec<f32>,
    scratch: Vec<f32>,
    /// Gas can be here: the rock is empty (or holds smoke) and no shut door
    /// stands in it. Rebuilt by the game every tick from the world.
    pub open: Vec<bool>,
    /// Ticks left burning; 0 is not burning.
    pub burn: Vec<u8>,
    /// Ticks left hot after burning.
    pub ember: Vec<u8>,
    /// How many more cells a flame here may hand itself on through. A light
    /// starts it at [`FLAME_REACH`]; each catch passes on one less.
    pub reach: Vec<u8>,
    /// Fan push, cells per tick, rebuilt when a fan or the rock changes.
    pub vx: Vec<f32>,
    pub vy: Vec<f32>,
    /// Fan stir, 0..1: how hard a fan is churning this cell. Holds the gas
    /// up against sinking and adds to its spreading.
    pub stir: Vec<f32>,
    /// Each fan's cone, as (cell, weight). Gas inside a cone is churned
    /// toward the cone's own average -- see [`Gas::churn`].
    pub cones: Vec<Vec<(usize, f32)>>,
    /// Cells at the top of the air shaft, where gas leaves the mine.
    pub vents: Vec<usize>,
    /// Gas that has left by the shaft, in cell-fulls. A counter, for the
    /// harness.
    pub vented: f32,
    /// Gas eaten by flame, in cell-fulls.
    pub burnt: f32,
    /// Cells lit this tick, for the sound and the harness.
    pub lit_this_tick: u32,
    /// Every cell lit since the game last drained this -- by a light, a
    /// catch or a fireball alike -- so the harness can stamp when it caught.
    pub just_lit: Vec<usize>,
}

impl Default for Gas {
    fn default() -> Self {
        Self::new()
    }
}

impl Gas {
    pub fn new() -> Self {
        Gas {
            c: vec![0.0; N],
            scratch: vec![0.0; N],
            open: vec![false; N],
            burn: vec![0; N],
            ember: vec![0; N],
            reach: vec![0; N],
            vx: vec![0.0; N],
            vy: vec![0.0; N],
            stir: vec![0.0; N],
            cones: Vec::new(),
            vents: Vec::new(),
            vented: 0.0,
            burnt: 0.0,
            lit_this_tick: 0,
            just_lit: Vec::new(),
        }
    }

    pub fn at(&self, x: i32, y: i32) -> f32 {
        if inside(x, y) {
            self.c[idx(x, y)]
        } else {
            0.0
        }
    }

    pub fn total(&self) -> f32 {
        self.c.iter().sum()
    }

    /// Add gas at a cell, up to full. Returns what fitted.
    pub fn add(&mut self, i: usize, amount: f32) -> f32 {
        if !self.open[i] {
            return 0.0;
        }
        let room = (1.0 - self.c[i]).max(0.0);
        let put = amount.min(room);
        self.c[i] += put;
        put
    }

    /// Drop gas from cells that have stopped being open (rubble landed, a
    /// door shut on it without a side to go to).
    pub fn clear_closed(&mut self) {
        for i in 0..N {
            if !self.open[i] {
                self.c[i] = 0.0;
                self.burn[i] = 0;
                self.ember[i] = 0;
            }
        }
    }

    /// Steps 2 to 5 of the module doc. The leak (step 1) is the caller's,
    /// because where it is belongs to the level.
    pub fn step_flow(&mut self) {
        // **Burning gas holds still.** While a cell burns, nothing flows into
        // or out of it, so the flame front is not starved by the burnt cells
        // behind it drawing the gas ahead of it down out of the band.
        //
        // Only *burning* cells. Holding the just-burnt (hot) cells too, and
        // pausing a fan's churn while fire is in its cone, was tried and
        // measured against this on 12 paired seeds of the planned player
        // (2026-11-07): it cut the share of burnt cells lying outside the lit
        // pocket from 50% to 35%, but the sump settled thick between bangs,
        // gas spilled up the climb, and the planned player lost ore on 12 of
        // 12 seeds (192-198 -> 133-190, sign test p = 0.0005) and a face on
        // 8 of 12. Inferred, not traced: most of that 50% is a bang's
        // fireball lighting the next pocket and the flame running on from
        // there, which a player sees as part of the bang.
        let burning: Vec<usize> = (0..N).filter(|&i| self.burn[i] > 0 && self.open[i]).collect();
        for &i in &burning {
            self.open[i] = false;
        }
        self.churn();
        self.advect();
        self.sink();
        self.spread();
        for &i in &burning {
            self.open[i] = true;
        }
        for &i in &self.vents {
            let gone = self.c[i] * 0.5;
            self.c[i] -= gone;
            self.vented += gone;
        }
    }

    /// **A fan churns what is in its cone.** Each cell moves part of the way
    /// toward the cone's weighted average, which mixes the gas with the air
    /// the fan is blowing through it -- the "thin it first" verb. It keeps
    /// every grain of gas (the weighted sum is unchanged), so a pocket too
    /// big for the cone's air stays too thick however long the fan runs,
    /// and has to be vented or burnt down first.
    ///
    /// Spreading alone could not do this: a fan's cone is ~40 cells long, and
    /// diffusion across 40 cells takes most of a minute. The churn does it in
    /// about a second, which is the half-second answer the hand needs.
    fn churn(&mut self) {
        for cone in &self.cones {
            let (mut mass, mut weight) = (0.0f32, 0.0f32);
            for &(i, w) in cone {
                if self.open[i] {
                    mass += w * self.c[i];
                    weight += w;
                }
            }
            if weight <= 0.0 {
                continue;
            }
            let mean = mass / weight;
            for &(i, w) in cone {
                if self.open[i] {
                    self.c[i] += CHURN * w * (mean - self.c[i]);
                }
            }
        }
    }

    /// Fans carry the gas. Donor-cell, one axis at a time, never more than a
    /// cell's own gas and never past full.
    fn advect(&mut self) {
        // x pass
        self.scratch.copy_from_slice(&self.c);
        for y in 0..H {
            for x in 0..W {
                let i = y * W + x;
                let v = self.vx[i];
                if v == 0.0 || self.c[i] <= 0.0 {
                    continue;
                }
                let tx = if v > 0.0 { x as i32 + 1 } else { x as i32 - 1 };
                if tx < 0 || tx >= W as i32 {
                    continue;
                }
                let j = y * W + tx as usize;
                if !self.open[j] {
                    continue;
                }
                let want = v.abs().min(0.9) * self.c[i];
                let room = (1.0 - self.scratch[j]).max(0.0);
                let m = want.min(room).min(self.scratch[i]);
                self.scratch[i] -= m;
                self.scratch[j] += m;
            }
        }
        std::mem::swap(&mut self.c, &mut self.scratch);
        // y pass
        self.scratch.copy_from_slice(&self.c);
        for y in 0..H {
            for x in 0..W {
                let i = y * W + x;
                let v = self.vy[i];
                if v == 0.0 || self.c[i] <= 0.0 {
                    continue;
                }
                let ty = if v > 0.0 { y as i32 + 1 } else { y as i32 - 1 };
                if ty < 0 || ty >= H as i32 {
                    continue;
                }
                let j = ty as usize * W + x;
                if !self.open[j] {
                    continue;
                }
                let want = v.abs().min(0.9) * self.c[i];
                let room = (1.0 - self.scratch[j]).max(0.0);
                let m = want.min(room).min(self.scratch[i]);
                self.scratch[i] -= m;
                self.scratch[j] += m;
            }
        }
        std::mem::swap(&mut self.c, &mut self.scratch);
    }

    /// Heavier than air: bottom row first, so a parcel falls at most one cell
    /// a tick.
    fn sink(&mut self) {
        for y in (0..H - 1).rev() {
            for x in 0..W {
                let i = y * W + x;
                let c = self.c[i];
                if c <= 0.0 {
                    continue;
                }
                let j = i + W;
                if !self.open[j] {
                    continue;
                }
                // A fan blowing upward holds the gas up; only sink what the
                // fan is not lifting.
                let lift = (-self.vy[i]).max(0.0);
                let held = (1.0 - 4.0 * lift).max(0.0) * (1.0 - 0.9 * self.stir[i].min(1.0));
                let m = SINK * held * c * (1.0 - self.c[j]).max(0.0);
                self.c[i] -= m;
                self.c[j] += m;
            }
        }
    }

    fn spread(&mut self) {
        self.scratch.copy_from_slice(&self.c);
        for y in 0..H {
            for x in 0..W {
                let i = y * W + x;
                if !self.open[i] {
                    continue;
                }
                // Right and down neighbours only: each pair once.
                if x + 1 < W {
                    let j = i + 1;
                    if self.open[j] {
                        let d = (SPREAD_H + 0.05 * (self.stir[i] + self.stir[j])).min(SPREAD_MAX);
                        let f = d * (self.c[i] - self.c[j]);
                        self.scratch[i] -= f;
                        self.scratch[j] += f;
                    }
                }
                if y + 1 < H {
                    let j = i + W;
                    if self.open[j] {
                        let d = (SPREAD_V + 0.08 * (self.stir[i] + self.stir[j])).min(SPREAD_MAX);
                        let f = d * (self.c[i] - self.c[j]);
                        self.scratch[i] -= f;
                        self.scratch[j] += f;
                    }
                }
            }
        }
        std::mem::swap(&mut self.c, &mut self.scratch);
        for v in self.c.iter_mut() {
            if *v < 1e-5 {
                *v = 0.0;
            } else if *v > 1.0 {
                *v = 1.0;
            }
        }
    }

    /// Light a cell if the gas there can burn. Returns whether it caught.
    pub fn light(&mut self, x: i32, y: i32) -> bool {
        if !inside(x, y) {
            return false;
        }
        let i = idx(x, y);
        if self.open[i] && self.burn[i] == 0 && burnable(self.c[i]) {
            self.burn[i] = BURN_TICKS;
            self.reach[i] = FLAME_REACH;
            self.lit_this_tick += 1;
            self.just_lit.push(i);
            true
        } else {
            false
        }
    }

    /// Whether any cell within `r` of (x, y) is burnable.
    pub fn burnable_near(&self, x: i32, y: i32, r: i32) -> Option<(i32, i32)> {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r {
                    continue;
                }
                let (px, py) = (x + dx, y + dy);
                if inside(px, py) {
                    let i = idx(px, py);
                    if self.open[i] && burnable(self.c[i]) {
                        return Some((px, py));
                    }
                }
            }
        }
        None
    }

    pub fn burning_near(&self, x: i32, y: i32, r: i32) -> bool {
        for dy in -r..=r {
            for dx in -r..=r {
                let (px, py) = (x + dx, y + dy);
                if inside(px, py) && self.burn[idx(px, py)] > 0 {
                    return true;
                }
            }
        }
        false
    }

    pub fn max_near(&self, x: i32, y: i32, r: i32) -> f32 {
        let mut m: f32 = 0.0;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r {
                    continue;
                }
                m = m.max(self.at(x + dx, y + dy));
            }
        }
        m
    }

    /// **The flame front.** A burnable cell catches from a burning
    /// neighbour (eight around), burns for [`BURN_TICKS`] eating its gas,
    /// and is then hot for [`EMBER_TICKS`]. It never enters a thin, thick,
    /// shut or solid cell, which is the rule the player reads off the
    /// colours: *the flame stops where the yellow stops.*
    pub fn step_flame(&mut self, seed: u64, tick: u64) {
        let mut catch: Vec<(usize, u8)> = Vec::new();
        for y in 1..H - 1 {
            for x in 1..W - 1 {
                let i = y * W + x;
                if self.burn[i] > 0 || !self.open[i] || !burnable(self.c[i]) {
                    continue;
                }
                let mut fire = 0;
                let mut hot = 0;
                let mut reach = 0u8;
                for (dx, dy) in NEIGHBOURS {
                    let j = (i as isize + dy * W as isize + dx) as usize;
                    if self.burn[j] > 0 && self.reach[j] > 1 {
                        fire += 1;
                        reach = reach.max(self.reach[j] - 1);
                    } else if self.ember[j] > 0 && self.reach[j] > 1 {
                        hot += 1;
                        reach = reach.max(self.reach[j] - 1);
                    }
                }
                if fire == 0 && hot == 0 {
                    continue;
                }
                let miss = (1.0 - FLAME_CATCH).powi(fire) * (1.0 - EMBER_CATCH).powi(hot);
                if coin(seed, tick, i) > miss {
                    catch.push((i, reach));
                }
            }
        }
        for i in 0..N {
            if self.burn[i] > 0 {
                let eat = BURN_EAT.min(self.c[i]);
                self.c[i] -= eat;
                self.burnt += eat;
                self.burn[i] -= 1;
                if self.burn[i] == 0 {
                    self.burnt += self.c[i];
                    self.c[i] = 0.0;
                    self.ember[i] = EMBER_TICKS;
                }
            } else if self.ember[i] > 0 {
                self.ember[i] -= 1;
            }
        }
        self.lit_this_tick += catch.len() as u32;
        for (i, reach) in catch {
            self.burn[i] = BURN_TICKS;
            self.reach[i] = reach;
            self.just_lit.push(i);
        }
    }

    pub fn burning_count(&self) -> u32 {
        self.burn.iter().filter(|&&b| b > 0).count() as u32
    }
}

pub const NEIGHBOURS: [(isize, isize); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// **The pocket a light at (x, y) would burn**: every burnable open cell
/// joined to it through burnable cells (four-way). Returns the cells, the
/// gas in them, and the gas in the *inside* of the pocket -- cells whose four
/// neighbours are all burnable too. A thin ribbon along the top of a pool
/// has almost no inside, and a pocket you thinned all the way through is
/// nearly all inside; only the inside decides whether it goes bang. That is
/// how a fringe flash and a real blast are told apart.
pub struct Pocket {
    pub cells: Vec<usize>,
    pub gas: f32,
    pub inner_gas: f32,
    pub heart: (i32, i32),
}

pub fn pocket(gas: &Gas, x: i32, y: i32, seen: &mut [bool]) -> Option<Pocket> {
    if !inside(x, y) {
        return None;
    }
    let start = idx(x, y);
    if !gas.open[start] || !burnable(gas.c[start]) || seen[start] {
        return None;
    }
    let mut stack = vec![start];
    seen[start] = true;
    let mut cells = Vec::new();
    let (mut total, mut inner, mut sx, mut sy) = (0.0f32, 0.0f32, 0.0f64, 0.0f64);
    while let Some(i) = stack.pop() {
        cells.push(i);
        let (cx, cy) = ((i % W) as i32, (i / W) as i32);
        let c = gas.c[i];
        total += c;
        let mut all = true;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (cx + dx, cy + dy);
            if !inside(nx, ny) {
                all = false;
                continue;
            }
            let j = idx(nx, ny);
            let ok = gas.open[j] && burnable(gas.c[j]);
            if !ok {
                all = false;
            } else if !seen[j] {
                seen[j] = true;
                stack.push(j);
            }
        }
        if all {
            inner += c;
            sx += cx as f64 * c as f64;
            sy += cy as f64 * c as f64;
        }
    }
    let heart = if inner > 0.0 {
        let (hx, hy) = ((sx / inner as f64) as i32, (sy / inner as f64) as i32);
        // Snap to the nearest pocket cell, so the bang is inside the gas.
        let mut best = (cells[0], i64::MAX);
        for &i in &cells {
            let d = ((i % W) as i64 - hx as i64).pow(2) + ((i / W) as i64 - hy as i64).pow(2);
            if d < best.1 {
                best = (i, d);
            }
        }
        ((best.0 % W) as i32, (best.0 / W) as i32)
    } else {
        (x, y)
    };
    Some(Pocket { cells, gas: total, inner_gas: inner, heart })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_box() -> Gas {
        let mut g = Gas::new();
        for y in 10..50 {
            for x in 10..100 {
                g.open[idx(x, y)] = true;
            }
        }
        g
    }

    #[test]
    fn the_band_has_three_readings_and_a_clear() {
        assert_eq!(band(0.0), Band::Clear);
        assert_eq!(band(0.1), Band::Thin);
        assert_eq!(band(0.4), Band::Burnable);
        assert_eq!(band(0.9), Band::Thick);
    }

    #[test]
    fn gas_sinks_and_is_conserved_in_a_closed_box() {
        let mut g = open_box();
        for x in 40..60 {
            g.c[idx(x, 15)] = 1.0;
        }
        let before = g.total();
        for _ in 0..600 {
            g.step_flow();
        }
        let after = g.total();
        assert!((before - after).abs() < 0.05 * before, "gas not conserved: {before} -> {after}");
        let top: f32 = (10..100).map(|x| g.c[idx(x, 11)]).sum();
        let bottom: f32 = (10..100).map(|x| g.c[idx(x, 49)]).sum();
        assert!(bottom > top * 5.0, "heavy gas did not pool: top {top} bottom {bottom}");
    }

    #[test]
    fn flame_runs_through_burnable_gas_and_stops_at_thick() {
        let mut g = open_box();
        // A burnable strip with a thick block in the middle of it.
        for x in 10..100 {
            let c = if (50..60).contains(&x) { 0.95 } else { 0.35 };
            for y in 20..24 {
                g.c[idx(x, y)] = c;
            }
        }
        assert!(g.light(12, 21));
        for t in 0..200 {
            g.step_flame(1, t);
        }
        // The near side burnt away; the thick block stopped it, so the far
        // side is untouched.
        assert!(g.c[idx(30, 21)] < 0.05, "near side did not burn");
        assert!(g.c[idx(55, 21)] > 0.9, "thick gas burnt");
        assert!(g.c[idx(80, 21)] > 0.3, "flame jumped the thick block");
    }

    #[test]
    fn a_ribbon_has_little_inside_and_a_block_has_a_lot() {
        let mut g = open_box();
        for x in 10..90 {
            g.c[idx(x, 30)] = 0.35;
        }
        let mut seen = vec![false; N];
        let ribbon = pocket(&g, 20, 30, &mut seen).unwrap();
        assert_eq!(ribbon.inner_gas, 0.0);
        let mut g = open_box();
        for y in 20..40 {
            for x in 20..40 {
                g.c[idx(x, y)] = 0.35;
            }
        }
        let mut seen = vec![false; N];
        let block = pocket(&g, 25, 25, &mut seen).unwrap();
        assert!(block.inner_gas > 0.8 * block.gas);
    }
}
