//! **What has been written since a speculation window opened** — the
//! validity test the creature pass's parallel sense rests on.
//!
//! **Read `Reports/evolution-lab-creature-parallelism-2026-09-13.md` first:
//! the mechanism this serves ships OFF, because it is correct and does not
//! pay on a four-core box. Everything below is still live — the hooks run
//! whenever the watch is armed, and it arms whenever the switch is on.**
//!
//! `scheduler::step` dispatches active sites serially, and a creature's
//! `sense` is a pure read of the world *as it stands when that creature's
//! turn comes*. Round 33 computes those senses ahead of time, several at
//! once, off one `&World` (`creature::speculate_window`). That is only
//! allowed to stand in for the serial read if **nothing the speculation
//! read has been written since** — otherwise the cached answer is a read of
//! a world that no longer exists, and the engine's one determinism surface
//! (`ActiveSite`'s `Ord`, which decides who reaches a contested cell first)
//! is silently gone.
//!
//! # The invariant
//!
//! **A cached sense is used only if no tile overlapping its recorded read
//! rectangle has been marked since the window opened.** Every write that
//! could change what `sense` returns marks. That claim is kept honest the
//! way `World::set`'s own doc keeps its sibling claims honest — *at the
//! seam, not at a list of callers*: the marks are made inside `World::set`,
//! `World::organism_mut`, `World::deposit_pheromone` and
//! `World::set_soil_moisture`, which every writer in the engine already
//! goes through, plus a whole-window invalidation (`mark_all`) on the rare
//! structural events (`push_organism`, `free_organism`, the
//! `set_organism_*` harness seams) where a spatial mark would need a
//! position the call does not have.
//!
//! **An over-mark is free and an under-mark is a bug**, which is the way
//! round the asymmetry has to be: missing a mark does not crash, it changes
//! behaviour invisibly. So the granularity is coarse (`TILE` cells square),
//! the rectangles are conservative, and anything ambiguous invalidates.
//! `PIXEL_PHYSICS_CREATURE_PAR=unchecked` skips the test entirely and is
//! **a control, never a setting**: it makes the cached read stale on
//! purpose, and `lab_cost`'s world and field hashes move under it. That is
//! how this gate is shown to be sensitive rather than merely green.
//! `=verify` is the stronger instrument — it recomputes every accepted
//! speculation and names the brain input that differs, which is how the
//! evaporation hole was found rather than guessed at.
//!
//! # Cost when it is off
//!
//! One `bool` test per write. `arm` is called once per world and allocates
//! one `u32` per `TILE`x`TILE` cells (1.3 MB on the shipped 8192x2560
//! world, 4 KB on the lab bed); `open_window` bumps a generation rather
//! than clearing, so a window costs nothing to start.

/// Cells per side of a watch tile. A power of two so the map index is a
/// shift.
///
/// **Chosen against the hit rate, not against memory.** A creature's read
/// rectangle is its body plus the widest sense radius — on the shipped ant
/// about 14x12 cells, or 3x3 tiles here — and a creature's writes are a
/// handful of cells. At a 32-site window that marks ~50 tiles of the lab
/// bed's 4,096, so nine independent tiles come clean about 90% of the time.
/// Halving this doubles the hit rate's headroom and quadruples the map;
/// doubling it would put a whole nest in one tile.
/// Sized from the hit rate rather than from memory, and **swept rather than
/// asserted**: `PIXEL_PHYSICS_CREATURE_PAR_TILE` sets the cells per side (a
/// power of two) so the choice below is a measurement.
/// `field::FIELD_SCALE` is 16, so four bits. Named here rather than imported
/// so this module stays free of the field's own types.
const FIELD_SHIFT: u32 = 4;

fn tile_shift() -> u32 {
    static S: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *S.get_or_init(|| {
        let cells: u32 = std::env::var("PIXEL_PHYSICS_CREATURE_PAR_TILE").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
        cells.max(1).next_power_of_two().trailing_zeros()
    })
}

/// A coarse map of "written since the window opened", plus the two escape
/// hatches (`off`, `all`).
/// `Clone` because `World` is cloneable; a clone starts its own life with
/// the map it was copied from, which is harmless -- the watch is closed
/// outside `scheduler::step` and `open_window` is what makes it meaningful.
#[derive(Clone, Default)]
pub struct WriteWatch {
    /// Armed only while `scheduler::step` is speculating. Off everywhere
    /// else, so every hook in `World` is one predictable branch.
    on: bool,
    /// Bumped per window instead of clearing `tiles`.
    gen: u32,
    /// Something was written that has no position to mark — the window is
    /// void wholesale.
    all: bool,
    /// Per tile, the generation a **cell** in it was last written in — the
    /// grid and the pheromone planes.
    tiles: Vec<u32>,
    /// Per tile, the generation an **organism whose cells lie in it** last
    /// had its state handed out mutably.
    ///
    /// **A second map rather than a second mark on the first, and it is
    /// worth the memory.** The two are read at very different reaches: a
    /// creature reads *cells* out to its crowding and curvature discs and
    /// its sight cast, and reads another creature's *state* only where a
    /// body cell touches its own ring (`is_living_kin`, `kin_deficit`) or
    /// where its eye resolves one. Pooling them made every ant that merely
    /// updated its own energy invalidate every neighbour within the wider
    /// radius — and a tick that writes no cell at all is the common case
    /// here, measured at 7.7 moves against 29.4 ticks a frame.
    otiles: Vec<u32>,
    /// Per **field** cell (`FIELD_SCALE` cells square), the generation it was
    /// last written in.
    ///
    /// **A third map, at the field's own resolution, and it is what makes
    /// evaporation affordable.** The moisture channel is read bilinearly, so
    /// a write to one field cell reaches cell-position reads up to two
    /// `FIELD_SCALE` blocks away. Marking that reach into the cell map above
    /// meant one evaporating surface dirtying a 67x67 cell square, and it
    /// halved the hit rate on a bed with water in it — measured 34.8% against
    /// 61%. At the field's own resolution the write is one entry and the read
    /// is the handful of field cells the samples actually interpolate.
    ftiles: Vec<u32>,
    fcols: i32,
    frows: i32,
    cols: i32,
    rows: i32,
    x0: i32,
    y0: i32,
    shift: u32,
    /// Per organism slot, the generation its cells were last marked in.
    /// **A dedup, not a second map**: `organism_mut` hands out `&mut` many
    /// times per tick and each hand-out would otherwise re-walk the whole
    /// organism, which for a grown tree is thousands of cells. Once per
    /// organism per window is enough, because everything that *moves* a cell
    /// afterwards goes through `World::set` and marks there.
    orgs: Vec<u32>,
}

impl WriteWatch {
    /// Size the map to the world. Idempotent, and a no-op for an unbounded
    /// world — which simply never speculates.
    pub fn fit(&mut self, x0: i32, y0: i32, w: i32, h: i32) {
        self.shift = tile_shift();
        let cols = (w >> self.shift) + 1;
        let rows = (h >> self.shift) + 1;
        if self.cols == cols && self.rows == rows && self.x0 == x0 && self.y0 == y0 {
            return;
        }
        self.x0 = x0;
        self.y0 = y0;
        self.cols = cols;
        self.rows = rows;
        self.tiles = vec![0; (cols.max(0) * rows.max(0)) as usize];
        self.otiles = vec![0; (cols.max(0) * rows.max(0)) as usize];
        self.fcols = (w >> FIELD_SHIFT) + 2;
        self.frows = (h >> FIELD_SHIFT) + 2;
        self.ftiles = vec![0; (self.fcols.max(0) * self.frows.max(0)) as usize];
        self.orgs.clear();
        self.gen = 0;
    }

    /// Is the map usable at all? False before `fit`, which is what disables
    /// speculation on an unbounded world.
    pub fn fitted(&self) -> bool {
        !self.tiles.is_empty()
    }

    /// Open a window: everything marked before this point stops counting.
    /// Leaves the watch **armed**.
    pub fn open_window(&mut self) {
        self.on = true;
        self.all = false;
        self.gen = self.gen.wrapping_add(1);
        if self.gen == 0 {
            // Wrapped, so a stale stamp could alias the live generation.
            // Once every four billion windows, and the alternative is a
            // wider counter for no reason.
            self.tiles.iter_mut().for_each(|t| *t = 0);
            self.otiles.iter_mut().for_each(|t| *t = 0);
            self.ftiles.iter_mut().for_each(|t| *t = 0);
            self.orgs.iter_mut().for_each(|t| *t = 0);
            self.gen = 1;
        }
    }

    /// Stop watching. Every hook below becomes one `bool` test.
    pub fn close(&mut self) {
        self.on = false;
        self.all = true;
    }

    #[inline]
    pub fn armed(&self) -> bool {
        self.on
    }

    /// Mark the tile holding `(x, y)`. Out-of-map writes invalidate the
    /// whole window rather than being dropped — a write the map cannot
    /// represent is exactly the case that must not be assumed harmless.
    #[inline]
    pub fn mark(&mut self, x: i32, y: i32) {
        if !self.on {
            return;
        }
        let (gen, cols, rows, shift, x0, y0) = (self.gen, self.cols, self.rows, self.shift, self.x0, self.y0);
        let tx = (x - x0) >> shift;
        let ty = (y - y0) >> shift;
        if tx < 0 || ty < 0 || tx >= cols || ty >= rows {
            self.all = true;
            return;
        }
        self.tiles[(ty * cols + tx) as usize] = gen;
    }

    /// Mark a cell of an organism whose **state** has just been handed out
    /// mutably. Separate map, narrower reach — see `otiles`.
    #[inline]
    pub fn mark_organism_cell(&mut self, x: i32, y: i32) {
        if !self.on {
            return;
        }
        let (gen, cols, rows, shift, x0, y0) = (self.gen, self.cols, self.rows, self.shift, self.x0, self.y0);
        let tx = (x - x0) >> shift;
        let ty = (y - y0) >> shift;
        if tx < 0 || ty < 0 || tx >= cols || ty >= rows {
            self.all = true;
            return;
        }
        self.otiles[(ty * cols + tx) as usize] = gen;
    }

    /// **Is this the first time this organism's slot has been handed out
    /// mutably in this window?** Returns true exactly once per slot per
    /// window, so the caller walks the organism's cells once.
    #[inline]
    pub fn first_touch(&mut self, slot: usize) -> bool {
        if !self.on {
            return false;
        }
        if self.orgs.len() <= slot {
            self.orgs.resize(slot + 1, 0);
        }
        if self.orgs[slot] == self.gen {
            return false;
        }
        self.orgs[slot] = self.gen;
        true
    }

    /// Mark the field cell holding world position `(x, y)`.
    #[inline]
    pub fn mark_field(&mut self, x: i32, y: i32) {
        if !self.on {
            return;
        }
        let fx = (x - self.x0).div_euclid(1 << FIELD_SHIFT);
        let fy = (y - self.y0).div_euclid(1 << FIELD_SHIFT);
        if fx < 0 || fy < 0 || fx >= self.fcols || fy >= self.frows {
            self.all = true;
            return;
        }
        self.ftiles[(fy * self.fcols + fx) as usize] = self.gen;
    }

    /// Has any field cell overlapping the inclusive **world** rectangle been
    /// written since the window opened? The rectangle is widened by one field
    /// cell on every side, which is exactly the reach of a bilinear sample.
    pub fn clean_field(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        if !self.on || self.all {
            return false;
        }
        let fx0 = (x0 - self.x0).div_euclid(1 << FIELD_SHIFT) - 1;
        let fy0 = (y0 - self.y0).div_euclid(1 << FIELD_SHIFT) - 1;
        let fx1 = (x1 - self.x0).div_euclid(1 << FIELD_SHIFT) + 1;
        let fy1 = (y1 - self.y0).div_euclid(1 << FIELD_SHIFT) + 1;
        if fx0 < 0 || fy0 < 0 || fx1 >= self.fcols || fy1 >= self.frows {
            return false;
        }
        for fy in fy0..=fy1 {
            let row = fy * self.fcols;
            for fx in fx0..=fx1 {
                if self.ftiles[(row + fx) as usize] == self.gen {
                    return false;
                }
            }
        }
        true
    }

    /// Void the whole window — for a write with no position to mark.
    #[inline]
    pub fn mark_all(&mut self) {
        if self.on {
            self.all = true;
        }
    }

    /// **The validity test.** True only if nothing in the inclusive
    /// rectangle has been written since the window opened.
    pub fn clean(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        self.clean_in(&self.tiles, x0, y0, x1, y1)
    }

    /// The same test over the organism-state map.
    pub fn clean_organisms(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        self.clean_in(&self.otiles, x0, y0, x1, y1)
    }

    fn clean_in(&self, map: &[u32], x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        if !self.on || self.all {
            return false;
        }
        let tx0 = (x0 - self.x0) >> self.shift;
        let ty0 = (y0 - self.y0) >> self.shift;
        let tx1 = (x1 - self.x0) >> self.shift;
        let ty1 = (y1 - self.y0) >> self.shift;
        // A rectangle reaching outside the map is not provably clean: the
        // part outside it was never watched. Same asymmetry as `mark`.
        if tx0 < 0 || ty0 < 0 || tx1 >= self.cols || ty1 >= self.rows {
            return false;
        }
        for ty in ty0..=ty1 {
            let row = ty * self.cols;
            for tx in tx0..=tx1 {
                if map[(row + tx) as usize] == self.gen {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn watch() -> WriteWatch {
        let mut w = WriteWatch::default();
        w.fit(0, 0, 255, 255);
        w.open_window();
        w
    }

    #[test]
    fn a_mark_dirties_its_own_tile_and_nothing_further_away() {
        let mut w = watch();
        w.mark(100, 100);
        assert!(!w.clean(100, 100, 100, 100), "the marked cell must read dirty");
        assert!(w.clean(60, 60, 63, 63), "a cell far away must stay clean");
    }

    #[test]
    fn opening_a_window_forgets_every_earlier_mark() {
        let mut w = watch();
        w.mark(40, 40);
        assert!(!w.clean(40, 40, 40, 40));
        w.open_window();
        assert!(w.clean(40, 40, 40, 40), "a new window starts clean");
    }

    /// The asymmetry the module doc states: everything ambiguous reads
    /// dirty. A closed watch, a voided window and a rectangle running off
    /// the map are all "not provably clean", never "clean".
    #[test]
    fn anything_unrepresentable_reads_dirty_rather_than_clean() {
        let mut w = watch();
        assert!(w.clean(10, 10, 20, 20));
        assert!(!w.clean(-4, 10, 20, 20), "a rectangle off the left edge is not provably clean");
        assert!(!w.clean(10, 10, 900, 20), "a rectangle off the right edge is not provably clean");
        w.mark(-40, -40);
        assert!(!w.clean(10, 10, 20, 20), "an out-of-map write voids the window");
        w.open_window();
        assert!(w.clean(10, 10, 20, 20));
        w.mark_all();
        assert!(!w.clean(10, 10, 20, 20), "mark_all voids the window");
        w.open_window();
        w.close();
        assert!(!w.clean(10, 10, 20, 20), "a closed watch answers no");
    }

    /// A mark made while the watch is closed must not be carried into the
    /// next window -- the hooks run on every write in the engine, and the
    /// watch is closed for all but the creature pass.
    #[test]
    fn marks_made_while_closed_are_not_remembered() {
        let mut w = watch();
        w.close();
        w.mark(50, 50);
        w.open_window();
        assert!(w.clean(50, 50, 50, 50), "a write outside any window cannot dirty one");
    }
}
