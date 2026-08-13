//! Liquid heightfield bodies (`Reports/liquid-heightfield-design.md`).
//!
//! Step 1 of that design's build order (§11): the ownership substrate and
//! the promote/demote round trip, with no solver and no absorption yet. A
//! promoted body is a [`LiquidBody`] — it holds per-column heights (`h`) on
//! the same `material::LIQUID_FULL` scale a `Liquid` cell's own `aux`
//! already uses, and in this step never changes on its own: promotion marks
//! the existing cells `FLAG_MANAGED` without moving any mass, and a body
//! only ever leaves the world via demotion (also mass-free — see
//! `World::demote_body`).
//!
//! The grid never lies (§2a): a promoted body's cells stay exactly correct
//! in `World`'s ordinary storage at all times. What promotion changes is
//! *who is allowed to write them* — `update::update_cell` skips a managed
//! cell outright, and any write into one from outside the body's own
//! rasterizer (`World::set_owned`) demotes it (`World::set`'s own doc).

use std::collections::{BTreeMap, HashSet};

use super::cell::Cell;
use super::material::{self, MaterialId, MaterialKind};
use super::world::World;

/// Below this many columns, ordinary CA diffusion levels a puddle fast
/// enough that promotion is pure overhead — design doc §3b.4. The design's
/// own empirical anchor (`a_wide_deep_water_column_levels_out_instead_of_
/// only_eroding_at_the_edges`'s test comment) puts the real line somewhere
/// between 40 (still levels) and 100 (does not) columns wide at reach 1;
/// starting here rather than at either end. **Untuned, flagged as such.**
pub const MIN_BODY_COLUMNS: usize = 32;

/// The flood fill's cap — design doc §3b.5. A refusal by cap is "not this
/// frame," not an error; a body this large is vanishingly unlikely in
/// ordinary play and the cap exists only to bound worst-case promotion cost.
pub const MAX_BODY_CELLS: usize = 20_000;

/// One promoted liquid body — design doc §2b/§3a/§9a. `h[i]` and the column
/// bounds are all that exist in step 1; there is no flux array and no
/// solver yet (`Reports/liquid-heightfield-design.md` §11 step 3 adds
/// those). A body is always exactly one material (`Reports/liquid-
/// heightfield-design.md` §3b.1) and always exactly one contiguous run of
/// x-columns — 4-connectivity guarantees no gaps (`label_body`'s own doc).
pub struct LiquidBody {
    pub material: MaterialId,
    pub x0: i32,
    /// Per column (indexed from `x0`): topmost liquid cell, inclusive.
    pub top_y: Vec<i32>,
    /// Per column: first non-body row below the column's liquid, exclusive
    /// — i.e. the column's own cells span `top_y[i]..bed_y[i]`.
    pub bed_y: Vec<i32>,
    /// Per column: total fill in units on the `material::LIQUID_FULL`
    /// scale, summed over the column's own cells. Computed *from* the
    /// cells at promotion (`World::promote_liquid_body`) and never
    /// otherwise — this is what makes promotion mass-free.
    pub h: Vec<u32>,
}

impl LiquidBody {
    pub fn columns(&self) -> usize {
        self.top_y.len()
    }

    /// Every cell this body owns and, once a solver exists, moves —
    /// design doc §3c. Not the same set as `container_positions` below,
    /// which the body depends on but never moves or counts as its own mass.
    pub fn managed_positions(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        (0..self.columns()).flat_map(move |i| {
            let x = self.x0 + i as i32;
            (self.top_y[i]..self.bed_y[i]).map(move |y| (x, y))
        })
    }

    /// The bed and walls immediately outside the body's own cells — design
    /// doc §3c. Flagged `FLAG_MANAGED` exactly like the body's own cells
    /// (so disturbing them also demotes, through the identical one-bit
    /// test) but never counted in `h` and never moved by the body. Derived
    /// from `managed_positions` rather than stored separately: for every
    /// body cell, its left/right/below neighbour is a container cell
    /// unless that neighbour is itself a body cell (an interior column
    /// boundary). Deliberately excludes "above" — that is the free surface
    /// §3b.3 requires stay open, and is where a later step's absorption
    /// inflow lands.
    pub fn container_positions(&self) -> Vec<(i32, i32)> {
        let body: HashSet<(i32, i32)> = self.managed_positions().collect();
        let mut container = HashSet::new();
        for &(x, y) in &body {
            for (dx, dy) in [(-1, 0), (1, 0), (0, 1)] {
                let p = (x + dx, y + dy);
                if !body.contains(&p) {
                    container.insert(p);
                }
            }
        }
        container.into_iter().collect()
    }

    pub fn total_fill(&self) -> u64 {
        self.h.iter().map(|&fill| fill as u64).sum()
    }

    /// Whether `(x, y)` is a cell this body owns — its own liquid cells or
    /// its bed. A fast column-range check first (the common case, since
    /// most disturbances land in the body's own bulk), falling back to the
    /// full `container_positions` set for the remaining case: a step wall
    /// where an interior column boundary is exposed because a neighbouring
    /// column is shorter. Used by `World::find_body_at` to resolve which of
    /// a chunk's few candidate bodies a disturbed position belongs to.
    pub fn owns(&self, x: i32, y: i32) -> bool {
        let i = x - self.x0;
        if i >= 0 && (i as usize) < self.columns() {
            let i = i as usize;
            if (y >= self.top_y[i] && y < self.bed_y[i]) || y == self.bed_y[i] {
                return true;
            }
        }
        self.container_positions().contains(&(x, y))
    }

    /// Rasterize column `i` back into cells if its whole-cell count has
    /// changed since it was last written — design doc §7e. Called after
    /// `h[i]` changes (today, only `World::absorb_liquid`'s credit; a later
    /// step's solver calls it too). A no-op, deliberately, when the count
    /// hasn't changed: nothing is written, so nothing dirties, so a quiet
    /// column keeps letting its chunk sleep.
    ///
    /// **Step 2 only ever grows a column** (absorption adds mass; nothing
    /// yet removes it — the solver, §11 step 3, is what makes shrinking a
    /// real case). Growing means claiming new cells upward from the current
    /// `top_y[i]` and flagging their own new left/right/below neighbours as
    /// container cells, exactly mirroring what `World::promote_liquid_body`
    /// did for the body's initial footprint — just incrementally, for the
    /// rows that are actually new. Only the newly claimed rows plus the
    /// previously-topmost row (whose fill may have flipped from partial to
    /// full) are ever rewritten; every cell below stays byte-identical.
    pub(crate) fn rasterize_column(&mut self, world: &mut World, i: usize) {
        let whole = self.h[i] / material::LIQUID_FULL as u32;
        let partial = (self.h[i] % material::LIQUID_FULL as u32) as u16;
        let new_count = whole as i32 + i32::from(partial > 0);
        let old_count = self.bed_y[i] - self.top_y[i];
        if new_count == old_count {
            return;
        }
        debug_assert!(
            new_count > old_count,
            "a liquid body column shrank -- step 2 has no solver to remove mass, only absorption to add it"
        );

        // Snapshotted before growing so the container-flagging pass below
        // can tell exactly which positions are newly exposed, rather than
        // re-flagging (harmlessly, but wastefully) ones that were already
        // correct.
        let old_container: HashSet<(i32, i32)> = self.container_positions().into_iter().collect();

        let old_top = self.top_y[i];
        let new_top = old_top - (new_count - old_count);
        self.top_y[i] = new_top;

        let x = self.x0 + i as i32;
        let shades = world.materials.get(self.material).palette.len().max(1) as u32;
        for y in new_top..=old_top {
            let fill = if y == new_top && partial > 0 { partial } else { material::LIQUID_FULL };
            // `aux == 0` is `liquid_fill`'s own "treat as full" sentinel
            // (`update::liquid_fill`'s doc), not "empty" -- a genuinely
            // full cell is written as 0, matching every other liquid write
            // in the engine.
            let aux = if fill >= material::LIQUID_FULL { 0 } else { fill };
            let shade = world.rng.below(shades) as u8;
            let cell = Cell::new(self.material, shade).with_managed(true).with_aux(aux);
            world.set_owned(x, y, cell);
        }

        for (cx, cy) in self.container_positions() {
            if !old_container.contains(&(cx, cy)) {
                let cell = world.get(cx, cy);
                world.set_owned(cx, cy, cell.with_managed(true));
            }
        }
    }
}

/// A validated candidate for promotion — design doc §3a's `BodyScan`.
/// Returned by `label_body` only once every one of §3b's structural checks
/// already holds; there is no separate "scan then validate" caller step.
pub struct BodyScan {
    pub material: MaterialId,
    pub x0: i32,
    pub top_y: Vec<i32>,
    pub bed_y: Vec<i32>,
    pub fill: Vec<u32>,
}

/// Bounded 4-connected flood fill over same-material `Liquid` cells
/// starting at `(x, y)`, validated against design doc §3b. `None` if
/// `(x, y)` isn't `Liquid`, any cell in the component is already
/// `FLAG_MANAGED` (already claimed by a different promoted body — checked
/// per visited cell, not just at `(x, y)`, so a caller can never end up
/// with two live `BodyId`s over overlapping cells; found by independent
/// review: promoting an already-promoted pool a second time used to
/// silently succeed, leaving a ghost `BodyId` in `body_index` that nothing
/// could ever demote, since its own cells could never again read as
/// disturbed once the *first* body's demotion had already cleared their
/// flag), the component fails any structural check, or it exceeds
/// `MAX_BODY_CELLS`.
///
/// Deliberately a new function rather than a generalization of `rigid::
/// label_component` — design doc §3a: the predicate (`Solid`-and-not-
/// `BEDROCK` there, same-`MaterialId`-`Liquid` here) and the return shape
/// (positions only there; per-column extents and fill here) both differ
/// enough that sharing would mean generics over both, which is more
/// machinery than two short functions.
pub fn label_body(world: &World, x: i32, y: i32) -> Option<BodyScan> {
    let start = world.get(x, y);
    if world.materials.kind(start.material) != MaterialKind::Liquid || start.managed() {
        return None;
    }
    let material = start.material;

    let mut visited: HashSet<(i32, i32)> = HashSet::new();
    let mut stack = vec![(x, y)];
    visited.insert((x, y));
    // `BTreeMap` rather than `HashMap`: iterated below to build `top_y`/
    // `bed_y`/`fill` in ascending-x order, and this module has no reason to
    // reintroduce issue #7's hazard (`scheduler.rs`'s own doc) for a
    // per-body scan that has no cross-run ordering requirement of its own,
    // but also no reason not to be deterministic for free.
    let mut columns: BTreeMap<i32, Vec<i32>> = BTreeMap::new();

    while let Some((cx, cy)) = stack.pop() {
        columns.entry(cx).or_default().push(cy);
        if visited.len() > MAX_BODY_CELLS {
            return None;
        }
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let (nx, ny) = (cx + dx, cy + dy);
            if visited.contains(&(nx, ny)) {
                continue;
            }
            let neighbour = world.get(nx, ny);
            if neighbour.material == material {
                // Already claimed by a different live body -- refuse the
                // whole scan rather than silently accepting it. See this
                // function's own doc for the exact ghost-`BodyId` bug this
                // guards against.
                if neighbour.managed() {
                    return None;
                }
                visited.insert((nx, ny));
                stack.push((nx, ny));
            }
        }
    }

    if columns.len() < MIN_BODY_COLUMNS {
        return None;
    }

    let x0 = *columns.keys().next().unwrap();
    let x1 = *columns.keys().next_back().unwrap();
    let width = (x1 - x0 + 1) as usize;
    // 4-connectivity guarantees the x-values present have no gaps (design
    // doc §3a): reaching a liquid cell in column x+2 from one in column x
    // requires passing through *some* liquid cell in column x+1, so a
    // BTreeMap with fewer keys than the span it covers would mean that
    // guarantee broke somewhere upstream, not a case to handle gracefully.
    if columns.len() != width {
        return None;
    }

    let mut top_y = vec![0i32; width];
    let mut bed_y = vec![0i32; width];
    let mut fill = vec![0u32; width];

    for (col_x, ys) in &columns {
        let i = (col_x - x0) as usize;
        let min_y = *ys.iter().min().unwrap();
        let max_y = *ys.iter().max().unwrap();
        // §3b.2: single vertical span per column -- the column's own cell
        // count must equal its y-range, or this column has a gap (two
        // separate bodies stacked in one column, connected to each other
        // only around some other column). Refused, stays CA.
        if ys.len() != (max_y - min_y + 1) as usize {
            return None;
        }
        // §3b.3: a free surface directly above the column's top cell.
        // Refuses sealed/ceiling-capped bodies -- not this design's
        // problem, see `Reports/liquid-heightfield-design.md` §14.
        let above = world.get(*col_x, min_y - 1);
        if !above.is_empty() && world.materials.kind(above.material) != MaterialKind::Gas {
            return None;
        }
        top_y[i] = min_y;
        bed_y[i] = max_y + 1;
        fill[i] = ys.iter().map(|&cy| super::update::liquid_fill(world.get(*col_x, cy)) as u32).sum();
    }

    Some(BodyScan { material, x0, top_y, bed_y, fill })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell::Cell;
    use crate::sim::chunk::Rect;
    use crate::sim::explosion;
    use crate::sim::material;
    use crate::sim::parallel;
    use crate::sim::particle::ParticleSystem;
    use crate::sim::update;

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 99, 99))
    }

    const POOL_X0: i32 = 10;
    const POOL_WIDTH: i32 = 40; // >= MIN_BODY_COLUMNS (32)
    const FLOOR_Y: i32 = 55;
    const WATER_Y: i32 = 54;

    /// A flat, `MIN_BODY_COLUMNS`-plus pool on a stone floor with open sky
    /// above — the minimal shape `label_body` will actually promote.
    fn build_pool(w: &mut World) {
        for x in (POOL_X0 - 1)..=(POOL_X0 + POOL_WIDTH) {
            w.set(x, FLOOR_Y, Cell::new(material::STONE, 0));
        }
        for x in POOL_X0..POOL_X0 + POOL_WIDTH {
            w.set(x, WATER_Y, Cell::new(material::WATER, 0));
        }
    }

    fn pool_positions() -> Vec<(i32, i32)> {
        (POOL_X0..POOL_X0 + POOL_WIDTH).map(|x| (x, WATER_Y)).collect()
    }

    /// Same material and fill (`aux`) -- the actual "mass" claim -- ignoring
    /// `flags`, which *correctly* differs once one is demoted (`managed`
    /// clears) and the other was snapshotted while still promoted.
    fn same_content(a: Cell, b: Cell) -> bool {
        a.material == b.material && a.aux() == b.aux()
    }

    /// Sum of `liquid_fill` over every `Liquid`-kind cell in the world --
    /// the actual conserved quantity (not cell count, which absorption's
    /// whole point is to change).
    fn total_liquid_fill(w: &World) -> u64 {
        let bounds = w.bounds().unwrap();
        let mut sum = 0u64;
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                let c = w.get(x, y);
                if w.materials.kind(c.material) == MaterialKind::Liquid {
                    sum += update::liquid_fill(c) as u64;
                }
            }
        }
        sum
    }

    #[test]
    fn promoting_a_pool_flags_every_cell_and_moves_no_mass() {
        let mut w = test_world();
        build_pool(&mut w);
        let before: Vec<Cell> = pool_positions().iter().map(|&(x, y)| w.get(x, y)).collect();

        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("a wide flat pool should promote");

        for (&(x, y), &original) in pool_positions().iter().zip(&before) {
            let cell = w.get(x, y);
            assert!(cell.managed(), "promoted cell at ({x},{y}) was not flagged managed");
            assert_eq!(cell.material, original.material, "promotion changed a cell's material");
            assert_eq!(cell.aux(), original.aux(), "promotion moved mass -- aux changed at ({x},{y})");
        }

        // B-1's other half: cell_count == sum(ceil(h[i] / LIQUID_FULL)).
        let body = w.body(id).unwrap();
        let cell_count = body.managed_positions().count();
        let expected: usize = body.h.iter().map(|&h| h.div_ceil(material::LIQUID_FULL as u32) as usize).sum();
        assert_eq!(cell_count, expected, "cell_count != sum(ceil(h[i]/LIQUID_FULL)) invariant broke");
    }

    #[test]
    fn promoting_an_already_managed_pool_a_second_time_fails() {
        // Independent review's finding: `label_body` used to accept a
        // component regardless of whether its cells were already `FLAG_
        // MANAGED` by a different live body. Promoting the same pool twice
        // produced two live `BodyId`s over identical cells -- demoting the
        // first cleared `FLAG_MANAGED` everywhere, silently orphaning the
        // second (`body_index`/`bodies` still held it, but nothing could
        // ever disturb its now-unmanaged cells to trigger its own
        // demotion; a permanent ghost registration).
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");

        let second = w.promote_liquid_body(POOL_X0, WATER_Y);

        assert!(second.is_none(), "promoting an already-managed pool a second time should refuse, not create a second BodyId");
        assert_eq!(w.body_count(), 1, "exactly one live body should exist after the refused second attempt");
    }

    #[test]
    fn a_promoted_body_does_not_move_for_2000_frames() {
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        // Snapshotted *after* promotion -- promotion itself flips `managed`,
        // so the claim under test is "nothing changes across 2000 frames",
        // not "promotion changed nothing" (a different, already-covered
        // claim, see `promoting_a_pool_flags_every_cell_and_moves_no_mass`).
        let before: Vec<Cell> = pool_positions().iter().map(|&(x, y)| w.get(x, y)).collect();

        for _ in 0..2000 {
            parallel::step(&mut w);
        }

        for (&(x, y), &original) in pool_positions().iter().zip(&before) {
            assert_eq!(w.get(x, y), original, "a promoted cell moved or changed at ({x},{y})");
        }
    }

    #[test]
    fn promote_run_then_demote_round_trip_is_mass_exact() {
        let mut w = test_world();
        build_pool(&mut w);
        let before: Vec<Cell> = pool_positions().iter().map(|&(x, y)| w.get(x, y)).collect();
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");

        for _ in 0..2000 {
            parallel::step(&mut w);
        }
        w.demote_body(id);

        for (&(x, y), &original) in pool_positions().iter().zip(&before) {
            let cell = w.get(x, y);
            assert!(!cell.managed(), "cell at ({x},{y}) is still flagged managed after demotion");
            assert_eq!(cell, original, "promote -> 2000 frames -> demote was not bit identical at ({x},{y})");
        }
        assert_eq!(w.body_count(), 0, "the body's slot should be freed after demotion");
    }

    // --- Every disturbance step 1's own verify list enumerates: paint,
    // erase, an explosion, fire (a reaction writing into a neighbour), a
    // falling grain of sand, and digging out the bed. Each demotes, and
    // demotion itself adds no further change beyond the disturbance's own
    // write -- checked by comparing every *other* cell of the body against
    // its pre-disturbance state.

    #[test]
    fn painting_over_a_managed_cell_demotes_the_body() {
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).unwrap();
        let target = (POOL_X0 + 5, WATER_Y);
        let rest: Vec<(i32, i32)> = pool_positions().into_iter().filter(|&p| p != target).collect();
        let rest_before: Vec<Cell> = rest.iter().map(|&(x, y)| w.get(x, y)).collect();

        w.set(target.0, target.1, Cell::new(material::SAND, 0));

        assert_eq!(w.body_count(), 0, "painting a managed cell should have demoted its body");
        assert_eq!(w.get(target.0, target.1).material, material::SAND, "the paint itself should have taken effect");
        for (&(x, y), &original) in rest.iter().zip(&rest_before) {
            let cell = w.get(x, y);
            assert!(!cell.managed(), "cell at ({x},{y}) is still flagged managed after its body was demoted");
            assert!(same_content(cell, original), "demotion disturbed an untouched cell's content at ({x},{y})");
        }
    }

    #[test]
    fn erasing_a_managed_cell_demotes_the_body() {
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).unwrap();
        let target = (POOL_X0 + 5, WATER_Y);
        let rest: Vec<(i32, i32)> = pool_positions().into_iter().filter(|&p| p != target).collect();
        let rest_before: Vec<Cell> = rest.iter().map(|&(x, y)| w.get(x, y)).collect();

        w.set(target.0, target.1, Cell::EMPTY);

        assert_eq!(w.body_count(), 0, "erasing a managed cell should have demoted its body");
        assert!(w.get(target.0, target.1).is_empty());
        for (&(x, y), &original) in rest.iter().zip(&rest_before) {
            let cell = w.get(x, y);
            assert!(!cell.managed(), "cell at ({x},{y}) is still flagged managed after its body was demoted");
            assert!(same_content(cell, original), "demotion disturbed an untouched cell's content at ({x},{y})");
        }
    }

    #[test]
    fn digging_out_the_bed_demotes_the_body() {
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).unwrap();
        let bed = (POOL_X0 + 5, FLOOR_Y);
        assert!(w.get(bed.0, bed.1).managed(), "test setup: the bed cell should have been flagged as a container cell");
        let before: Vec<Cell> = pool_positions().iter().map(|&(x, y)| w.get(x, y)).collect();

        w.set(bed.0, bed.1, Cell::EMPTY);

        assert_eq!(w.body_count(), 0, "digging out the bed should have demoted the body");
        for (&(x, y), &original) in pool_positions().iter().zip(&before) {
            let cell = w.get(x, y);
            assert!(!cell.managed(), "cell at ({x},{y}) is still flagged managed after its body was demoted");
            assert!(same_content(cell, original), "demotion disturbed the body's own cells' content at ({x},{y})");
        }
    }

    #[test]
    fn an_explosion_touching_the_body_demotes_it() {
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).unwrap();
        let mut particles = ParticleSystem::new();

        explosion::trigger(&mut w, &mut particles, POOL_X0 + 5, WATER_Y, 3, 150.0);

        assert_eq!(w.body_count(), 0, "an explosion touching the body should have demoted it");
        for x in POOL_X0 + 20..POOL_X0 + POOL_WIDTH {
            assert!(!w.get(x, WATER_Y).managed(), "a far cell is still flagged managed after demotion");
        }
    }

    #[test]
    fn a_falling_grain_of_sand_displacing_into_the_body_demotes_it() {
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).unwrap();
        let target_x = POOL_X0 + 5;
        // Directly above the body's own free surface -- an unmanaged
        // position, so placing the grain here is not itself a disturbance.
        w.set(target_x, WATER_Y - 1, Cell::new(material::SAND, 0));

        for _ in 0..10 {
            parallel::step(&mut w);
        }

        assert_eq!(w.body_count(), 0, "a grain of sand displacing into the body should have demoted it");
        assert_eq!(w.get(target_x, WATER_Y).material, material::SAND, "the grain should have sunk into the managed cell");
    }

    #[test]
    fn a_reaction_writing_into_the_body_demotes_it() {
        // Mirrors `fire.rs`'s own synthetic-reaction test pattern
        // (`react_liquid_a`/`b`/`c`) -- no shipped material reacts with
        // anything today, so a real "fire disturbs a neighbour" scenario
        // needs one built for the test, the same way that suite does.
        let dir = std::env::temp_dir().join("pixel-physics-liquid-body-reaction-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("spark.ron"),
            "(name: \"spark\", kind: Solid, density: 1.0, colors: [(200, 0, 0)], \
             reactions: [(with: \"water\", produces: (\"spark_ash\", \"steam\"), chance: 1.0)])",
        )
        .unwrap();
        std::fs::write(dir.join("spark_ash.ron"), "(name: \"spark_ash\", kind: Solid, density: 1.0, colors: [(80, 80, 80)])").unwrap();
        std::fs::write(dir.join("steam.ron"), "(name: \"steam\", kind: Gas, density: 0.01, colors: [(220, 220, 220)])").unwrap();

        let mut w = test_world();
        w.materials.reload(&dir).unwrap();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).unwrap();

        let spark = w.materials.id_of("spark").unwrap();
        let target_x = POOL_X0 + 5;
        // Directly above the free surface, same reasoning as the falling-
        // sand test: unmanaged, so placing it is not itself a disturbance,
        // and its own neighbour scan (`fire::update`'s `try_react`) reaches
        // straight down into the managed cell below it.
        w.set(target_x, WATER_Y - 1, Cell::new(spark, 0));

        update::step(&mut w);

        assert_eq!(w.body_count(), 0, "a reaction writing into the body should have demoted it");
        let steam = w.materials.id_of("steam").unwrap();
        assert_eq!(w.get(target_x, WATER_Y).material, steam, "the reaction should have written steam into the managed cell");

        std::fs::remove_dir_all(&dir).ok();
    }

    // --- Step 2 (design doc §11): absorption and lazy rasterization.
    // Deliberately still no solver -- a column that absorbs mass grows
    // taller in place ("the informative failure" the design doc itself
    // predicts) rather than spreading sideways.

    #[test]
    fn absorbing_a_falling_cell_grows_the_body_and_conserves_mass() {
        let mut w = test_world();
        build_pool(&mut w);
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        let col = 5usize;
        let x = POOL_X0 + col as i32;
        // A full, unmanaged water cell directly above the free surface --
        // `transfer_liquid_vertical` will find the managed cell below it
        // on the very next CA step.
        w.set(x, WATER_Y - 1, Cell::new(material::WATER, 0));

        let h_before = w.body(id).unwrap().h[col];
        let total_before = total_liquid_fill(&w);

        update::step(&mut w);

        let body = w.body(id).expect("the body should still be alive -- absorption is not a disturbance");
        assert_eq!(
            body.h[col],
            h_before + material::LIQUID_FULL as u32,
            "the body's own column should have gained exactly the source cell's fill"
        );
        // Rasterization writes the absorbed mass straight back into the
        // same position the source cell fell from -- the column grew by
        // exactly one cell, and that new cell is the topmost.
        let top = w.get(x, WATER_Y - 1);
        assert!(top.managed(), "the newly rasterized cell should be flagged managed");
        assert_eq!(top.material, material::WATER);
        assert_eq!(top.aux(), 0, "a full rasterized cell should carry the LIQUID_FULL sentinel (aux == 0), not a raw 1000");
        assert_eq!(total_liquid_fill(&w), total_before, "absorption must not create or destroy mass");
    }

    #[test]
    fn repeated_pours_onto_one_column_pile_up_into_a_visible_spike() {
        // The design doc's own predicted step-2 behaviour, made concrete:
        // without a solver to spread it, repeatedly pouring onto one
        // column grows *that* column while its neighbours stay untouched.
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        let spike_x = POOL_X0 + 5;
        let quiet_x = POOL_X0 + 25;

        // Dropped from well above the free surface, not repeatedly at a
        // fixed row just above it -- once the first pour rasterizes into
        // that exact row, it is itself managed, and painting a *second*
        // cell directly there would be a disturbance (`World::set`'s own
        // check), demoting the body instead of growing it. Free-falling
        // from further up lands correctly on the current top regardless of
        // how tall the column has grown by that point.
        for _ in 0..4 {
            w.set(spike_x, WATER_Y - 10, Cell::new(material::WATER, 0));
            for _ in 0..12 {
                update::step(&mut w);
            }
        }

        let column_height = |w: &World, x: i32| -> i32 {
            let mut y = WATER_Y;
            let mut h = 0;
            while w.get(x, y).managed() && w.materials.kind(w.get(x, y).material) == MaterialKind::Liquid {
                h += 1;
                y -= 1;
            }
            h
        };
        let spike_height = column_height(&w, spike_x);
        let quiet_height = column_height(&w, quiet_x);
        assert!(spike_height > quiet_height, "repeated pours onto one column should pile up higher than an untouched column: spike={spike_height}, quiet={quiet_height}");
        assert_eq!(quiet_height, 1, "an untouched column should be exactly as tall as it started");
    }

    #[test]
    fn demoting_a_grown_body_clears_every_newly_claimed_cell_too() {
        let mut w = test_world();
        build_pool(&mut w);
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        let x = POOL_X0 + 5;
        w.set(x, WATER_Y - 1, Cell::new(material::WATER, 0));
        update::step(&mut w);
        assert!(w.get(x, WATER_Y - 1).managed(), "test setup: the pour should have grown the column by one managed cell");
        let grown_cell_before = w.get(x, WATER_Y - 1);

        w.demote_body(id);

        assert_eq!(w.body_count(), 0);
        let grown_cell_after = w.get(x, WATER_Y - 1);
        assert!(!grown_cell_after.managed(), "the newly-grown cell should be unmanaged after demotion");
        assert!(same_content(grown_cell_after, grown_cell_before), "demotion should not change the grown cell's own content");
    }

    #[test]
    fn digging_beside_a_newly_grown_cell_demotes_the_body() {
        // The container-cell bookkeeping must extend as the body grows, not
        // just cover its original footprint at promotion time.
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        let x = POOL_X0 + 5;
        w.set(x, WATER_Y - 1, Cell::new(material::WATER, 0));
        update::step(&mut w);
        assert!(w.get(x, WATER_Y - 1).managed(), "test setup: the pour should have grown the column by one managed cell");
        let wall = (x - 1, WATER_Y - 1);
        assert!(w.get(wall.0, wall.1).managed(), "the newly grown row's own wall should have been flagged managed too");

        w.set(wall.0, wall.1, Cell::EMPTY);

        assert_eq!(w.body_count(), 0, "digging beside a newly grown cell should have demoted the body");
    }

    #[test]
    fn absorption_works_under_the_parallel_driver_too() {
        let mut w = test_world();
        build_pool(&mut w);
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        let col = 5usize;
        let x = POOL_X0 + col as i32;
        w.set(x, WATER_Y - 1, Cell::new(material::WATER, 0));
        let total_before = total_liquid_fill(&w);

        parallel::step(&mut w);

        let body = w.body(id).expect("the body should still be alive under the parallel driver too");
        assert_eq!(body.h[col], material::LIQUID_FULL as u32 * 2, "absorption should credit the body the same way under the parallel driver");
        assert_eq!(total_liquid_fill(&w), total_before, "absorption must not create or destroy mass under the parallel driver either");
    }

    #[test]
    fn a_column_growing_across_a_chunk_boundary_stays_findable() {
        // Independent review's finding: `promote_liquid_body` only ever
        // registers a body's *initial* footprint in `body_index`, but
        // `rasterize_column`'s growth can claim cells in a chunk the body
        // never touched at promotion time. Grown directly via
        // `World::absorb_liquid` with a large fill, rather than physically
        // simulating tens of thousands of frames of falling cells, to cross
        // a real `CHUNK_SIZE` (64-row) boundary in one call.
        let mut w = World::new(Rect::new(0, 0, 99, 199));
        let floor_y = 151;
        let water_y = 150;
        for x in (POOL_X0 - 1)..=(POOL_X0 + POOL_WIDTH) {
            w.set(x, floor_y, Cell::new(material::STONE, 0));
        }
        for x in POOL_X0..POOL_X0 + POOL_WIDTH {
            w.set(x, water_y, Cell::new(material::WATER, 0));
        }
        let id = w.promote_liquid_body(POOL_X0, water_y).expect("test setup: pool should promote");
        let col = 5usize;
        let x = POOL_X0 + col as i32;

        w.absorb_liquid(x, water_y, 70_000);

        let new_top = w.body(id).unwrap().top_y[col];
        assert!(water_y - new_top >= 64, "test setup: growth should have crossed at least one chunk boundary, only reached {}", water_y - new_top);

        // Absorption into the same column, now resolved entirely inside
        // the newly-crossed chunk, must still find the body -- silently
        // lost mass is exactly the bug independent review found.
        let h_before = w.body(id).unwrap().h[col];
        w.absorb_liquid(x, new_top, 500);
        assert_eq!(w.body(id).unwrap().h[col], h_before + 500, "absorption into the newly-crossed chunk was silently lost");

        // A disturbance in the newly-crossed chunk must also still demote
        // the body -- the write-seam's whole invariant otherwise silently
        // stops holding for that region.
        w.set(x, new_top, Cell::new(material::SAND, 0));
        assert_eq!(w.body_count(), 0, "a disturbance in the newly-crossed chunk should have demoted the body");
    }
}
