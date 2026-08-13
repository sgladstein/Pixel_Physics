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

use super::material::{MaterialId, MaterialKind};
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
}
