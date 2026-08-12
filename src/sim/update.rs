//! The cellular automaton step.
//!
//! Two ordering rules matter and are easy to get wrong:
//!
//! * **Bottom to top.** If rows were swept top-down, a falling cell would be
//!   re-examined at its new position on the same sweep and fall again, so a
//!   column of sand would reach the floor in one frame instead of falling.
//! * **Alternating horizontal direction.** Sweeping left-to-right every frame
//!   biases every symmetric decision the same way, and piles visibly drift.
//!
//! Movement rules here are deliberately simple; M3 replaces them with the
//! physically grounded model (friction angle, BTW toppling, hole propagation).
//!
//! # Liquids are a compressible volume, not a discrete occupied cell
//!
//! `update_liquid` does not use `flow_sideways`/`dispersion` the way
//! `update_gas` still does. A cell searching a fixed radius for a directly
//! reachable opening can never level a column wider than that radius — a
//! cell buried in the interior of a wide pile, with same-height liquid
//! neighbours on both sides past its search radius, has no destination to
//! find on any frame, no matter how long the simulation runs. Confirmed live
//! from a playtest screenshot: a tall water column eroded only from its
//! edges inward, never flattening.
//!
//! The fix is the standard falling-sand technique for this (see [Tom
//! Forsyth, "Cellular Automata for Physical
//! Modelling"](https://tomforsyth1000.github.io/papers/cellular_automata_for_physical_modelling.html)
//! and the [w-shadow.com
//! tutorial](https://w-shadow.com/blog/2009/09/29/falling-sand-style-water-simulation/)):
//! each `Liquid` cell holds a continuous fill amount (`Cell::aux`, on the
//! `material::LIQUID_FULL` = 1000 scale) instead of being simply occupied or
//! not, with a small allowed overfill (`material::LIQUID_MAX_COMPRESS`) when
//! the cell below is also full. Two same-height cells in columns of
//! different depth end up with genuinely different fill once compression
//! from the weight above is accounted for, which gives a buried cell a real
//! quantity to equalize against with its immediate neighbour — a proper
//! diffusion process, propagating one cell at a time but reaching arbitrarily
//! far given enough frames, unlike the old fixed-radius search.
//!
//! `Cell::aux() == 0` on a `Liquid` cell means "never transferred, treat as
//! full," not "empty" — see `material::LIQUID_FULL`'s own doc for why.

use super::chunk::{Rect, MAX_REACH};
use super::fire;
use super::material::{self, MaterialKind};
use super::surface::CellSurface;
use super::world::World;
use crate::sim::cell::Cell;

pub fn step(world: &mut World) {
    world.begin_step();

    // Sweeping right-to-left on alternate frames cancels the directional bias
    // that a fixed scan order would otherwise bake into every pile and flow.
    let rightward = world.frame.is_multiple_of(2);

    for coord in world.chunks_to_sweep() {
        let Some(region) = world.sweep_region(coord) else {
            continue;
        };
        sweep(world, region, rightward);
    }

    world.end_step();
}

/// Sweep one region against any `CellSurface` — `World` directly for the
/// serial driver above, or a `ChunkView` per active chunk for the M5
/// parallel driver (`parallel.rs`). Generic so both paths run the exact same
/// rule code; there is nothing separate to keep in sync.
pub(crate) fn sweep<S: CellSurface>(surface: &mut S, region: Rect, rightward: bool) {
    for y in (region.min_y..=region.max_y).rev() {
        if rightward {
            for x in region.min_x..=region.max_x {
                update_cell(surface, x, y, rightward);
            }
        } else {
            for x in (region.min_x..=region.max_x).rev() {
                update_cell(surface, x, y, rightward);
            }
        }
    }
}

fn update_cell<S: CellSurface>(surface: &mut S, x: i32, y: i32, rightward: bool) {
    let cell = surface.get(x, y);

    // Arrived here during this sweep. Skip it once so it cannot travel twice in
    // one frame, and clear the flag so it moves normally from here on.
    //
    // Heat and fire (M14) are skipped on this path too, not just movement —
    // deliberately, so every cell gets at most one fire::update call per
    // frame regardless of movement history. Letting a revisited cell get a
    // second call would tick its burn timer twice as fast as an otherwise
    // identical cell that happened not to move that frame. A cell skipped
    // this way simply gets its heat/fire update one frame later, on its next
    // ordinary visit — the same negligible deferral movement itself already
    // accepts here.
    if cell.moved() {
        surface.clear_moved(x, y);
        return;
    }

    // Before movement: a phase change (stone crossing its melting point) must
    // land before this frame's movement dispatch decides how the cell
    // behaves, or it would move as stone for one more frame after already
    // having become lava. `fire::update` may have changed the cell's
    // material, flags or temperature, so it is re-read from the world rather
    // than reusing the `cell` bound above.
    fire::update(surface, x, y);
    let cell = surface.get(x, y);

    match surface.materials().kind(cell.material) {
        MaterialKind::Powder => update_powder(surface, x, y, rightward),
        MaterialKind::Liquid => update_liquid(surface, x, y, rightward),
        MaterialKind::Gas => update_gas(surface, x, y, rightward),
        MaterialKind::Empty | MaterialKind::Solid | MaterialKind::Plant | MaterialKind::Creature => false,
    };
}

/// Falls straight down, then diagonally, then creeps along the slope.
fn update_powder<S: CellSurface>(surface: &mut S, x: i32, y: i32, rightward: bool) -> bool {
    if try_move(surface, x, y, x, y + 1) {
        return true;
    }
    let (first, second) = if surface.rng().flip() { (-1, 1) } else { (1, -1) };
    if try_move(surface, x, y, x + first, y + 1) || try_move(surface, x, y, x + second, y + 1) {
        return true;
    }
    roll_along_slope(surface, x, y, rightward)
}

/// Creep one cell along a slope, toward the nearest place the grain could fall.
///
/// This is what gives a powder an angle of repose instead of a fixed 45
/// degrees. Falling and sliding diagonally can only ever build a 45 degree
/// pile, because a grain stops the moment it has support beneath and to both
/// sides. Rolling lets it keep going down a shallower slope, and the pile comes
/// to rest once no surface grain can see anywhere to fall within its reach —
/// which the material's friction angle sets.
fn roll_along_slope<S: CellSurface>(surface: &mut S, x: i32, y: i32, rightward: bool) -> bool {
    let material = surface.get(x, y).material;
    let reach = surface.materials().get(material).roll_reach_at(x, y);
    if reach <= 0 {
        return false;
    }

    let left = downhill_distance(surface, x, y, -1, 1, reach);
    let right = downhill_distance(surface, x, y, 1, 1, reach);

    // Head for the closer opportunity. Always moving strictly closer to one
    // specific place to fall is what stops a grain drifting back and forth
    // between two of them and keeping its chunk awake forever.
    let dir = match (left, right) {
        (None, None) => return false,
        (Some(_), None) => -1,
        (None, Some(_)) => 1,
        (Some(l), Some(r)) => match l.cmp(&r) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Equal => {
                if surface.rng().flip() {
                    -1
                } else {
                    1
                }
            }
        },
    };

    surface.move_cell(x, y, x + dir, y, (dir > 0) == rightward);
    true
}

/// Falls freely like a powder while there's open space to fall into, then
/// switches to compressible-volume fill transfer — see the module doc for
/// why liquids need a different mechanism from `flow_sideways`/`dispersion`.
fn update_liquid<S: CellSurface>(surface: &mut S, x: i32, y: i32, rightward: bool) -> bool {
    // Free-fall into open space, same whole-cell move as a powder. Not
    // routed through fill transfer, which is deliberately rate-limited by
    // `flow_rate` -- that rate-limit exists for the slower pressure-driven
    // redistribution once a cell is resting against something, and must not
    // also throttle an ordinary unobstructed drop through open air.
    if try_move(surface, x, y, x, y + 1) {
        return true;
    }
    let (first, second) = if surface.rng().flip() { (-1, 1) } else { (1, -1) };
    if try_move(surface, x, y, x + first, y + 1) || try_move(surface, x, y, x + second, y + 1) {
        return true;
    }

    if transfer_liquid_vertical(surface, x, y) {
        return true;
    }

    transfer_liquid_horizontal(surface, x, y, first, rightward) || transfer_liquid_horizontal(surface, x, y, second, rightward)
}

/// Push fill from `(x, y)` down into `(x, y + 1)`, when that cell is the
/// same liquid material with room below `LIQUID_FULL + LIQUID_MAX_COMPRESS`.
/// `try_move` in `update_liquid` already handles every case where the cell
/// below is `Empty` or a different, displaceable-by-density material, so by
/// the time this runs the only remaining case worth handling is "resting on
/// more of the same liquid" -- anything else (a denser solid, a lighter gas)
/// is correctly left alone.
///
/// Never revisits: the sweep is bottom-to-top, so row `y + 1` was already
/// swept this frame regardless of scan direction, same as `try_move`'s own
/// downward case.
fn transfer_liquid_vertical<S: CellSurface>(surface: &mut S, x: i32, y: i32) -> bool {
    if !surface.in_bounds(x, y + 1) {
        return false;
    }
    let src = surface.get(x, y);
    let dst = surface.get(x, y + 1);
    if dst.material != src.material {
        return false;
    }
    let flow_rate = surface.materials().get(src.material).flow_rate;
    let src_fill = liquid_fill(src);
    let dst_fill = liquid_fill(dst);
    let room = (material::LIQUID_FULL + material::LIQUID_MAX_COMPRESS).saturating_sub(dst_fill);
    let amount = flow_rate.min(room).min(src_fill);
    if amount == 0 {
        return false;
    }

    write_liquid_transfer(surface, x, y, src, src_fill - amount, x, y + 1, dst, dst_fill + amount, false);
    true
}

/// Below this fill difference, two horizontally adjacent liquid cells count
/// as settled rather than continuing to trade small amounts back and forth
/// as their surrounding neighbours slowly finish levelling. Started at 8
/// (0.8% of `LIQUID_FULL`) but that left a wide test puddle still visibly
/// settling after ~12,000 frames -- tuned up empirically by directly
/// measuring convergence at each step until settling landed comfortably
/// inside a practical frame budget (under 1,000 frames) without the
/// residual unevenness becoming visible in this module's own flatness
/// checks. 150 is 15% of `LIQUID_FULL` -- a real trade of some precision
/// for settling speed, not a rounding-level tweak.
const MIN_LIQUID_TRANSFER: u16 = 150;

/// How far `transfer_liquid_horizontal` looks past the immediate neighbour
/// for a genuinely emptier cell to level against. Immediate-neighbour-only
/// transfer is correct but slow at width: pure nearest-neighbour diffusion
/// needs on the order of (width²) frames to fully equalise a wide body,
/// since a fill difference can only ever move one cell per frame no matter
/// how large `flow_rate` is -- confirmed concretely, a 100-cell-wide test
/// column was still visibly un-flat (height varying roughly 6-39 cells)
/// after 3000 frames with reach fixed at 1, and raising `flow_rate` alone
/// from 200 to 500 made no measurable difference, exactly as that reasoning
/// predicts. Looking `REACH` cells ahead lets one transfer close much more
/// of a gradient per frame. Small relative to `MAX_REACH` (32) — this is
/// not a reintroduction of the old dispersion-search's failure mode, since
/// unlike that search, finding nothing better within `REACH` never blocks
/// levelling entirely; it only means this particular frame's transfer
/// falls back to the immediate neighbour, and the same diffusion process
/// that fixes the original bug (propagating through cells regardless of
/// distance, just not instantly) still applies beyond `REACH`.
const HORIZONTAL_TRANSFER_REACH: i32 = 8;

/// Compare fill against the emptiest reachable cell within
/// `HORIZONTAL_TRANSFER_REACH` in direction `dir` and transfer up to half
/// the difference (capped by `flow_rate` and that cell's remaining room)
/// toward it -- the step that actually fixes the reported column-leveling
/// bug, since it acts on every cell with a fill difference from a nearby
/// neighbour, not only ones beside a directly visible opening. The scan
/// stops at the first non-`Empty`, non-same-material cell (a wall) — a
/// neighbour of a different material simply does not participate, and
/// nothing beyond a wall is reachable regardless of nominal distance.
///
/// Half the difference, not the whole thing: transferring the whole
/// difference doesn't reach equality, it *overshoots past* it -- two cells
/// at 500/300 swap to 300/500, the same gap with the fuller side now on the
/// other foot, and the next frame's alternating scan direction swaps it
/// right back, forever. Confirmed concretely: an earlier version of this
/// function did exactly that, and a debug run showed `active_chunk_count`
/// still nonzero at 24,000 frames where the correct (halved) version had
/// already settled. Half converges an isolated pair to equality in one step
/// with no overshoot possible; see `MIN_LIQUID_TRANSFER` below for why a
/// long *many-cell* chain still needs a floor on top of that.
fn transfer_liquid_horizontal<S: CellSurface>(surface: &mut S, x: i32, y: i32, dir: i32, rightward: bool) -> bool {
    let src = surface.get(x, y);
    let src_fill = liquid_fill(src);

    // Walk outward, remembering the emptiest same-material-or-empty cell
    // seen so far. Stops at the first wall (a different, non-empty
    // material) -- nothing past a wall is a real candidate, wall or no wall.
    let mut best: Option<(i32, Cell, u16)> = None;
    for step in 1..=HORIZONTAL_TRANSFER_REACH {
        let tx = x + dir * step;
        if !surface.in_bounds(tx, y) {
            break;
        }
        let candidate = surface.get(tx, y);
        if !candidate.is_empty() && candidate.material != src.material {
            break;
        }
        let candidate_fill = if candidate.is_empty() { 0 } else { liquid_fill(candidate) };
        if best.is_none_or(|(_, _, best_fill)| candidate_fill < best_fill) {
            best = Some((tx, candidate, candidate_fill));
        }
    }
    let Some((tx, dst, dst_fill)) = best else {
        return false;
    };

    // `MIN_LIQUID_TRANSFER`, not just `src_fill <= dst_fill`: a wide,
    // many-cell puddle levels one step at a time, so a difference this
    // small is the final, longest-running stretch of that process --
    // treating it as "close enough to flat, stop" is what actually bounds
    // the settling time to something practical, the same trade the residual
    // wedge from sand's own angle-of-repose already makes for powders.
    if src_fill < dst_fill + MIN_LIQUID_TRANSFER {
        return false;
    }

    let flow_rate = surface.materials().get(src.material).flow_rate;
    let room = (material::LIQUID_FULL + material::LIQUID_MAX_COMPRESS).saturating_sub(dst_fill);
    let amount = flow_rate.min(room).min((src_fill - dst_fill) / 2);
    if amount == 0 {
        return false;
    }

    // The scan will reach the destination again this frame only if the
    // transfer moves the same way the scan is sweeping -- same rule
    // `flow_sideways` already uses for the identical reason.
    let revisited = (dir > 0) == rightward;
    write_liquid_transfer(surface, x, y, src, src_fill - amount, tx, y, dst, dst_fill + amount, revisited);
    true
}

/// Shared write-back for both transfer directions: shrink or clear the
/// source, grow or create the destination. A destination built fresh from
/// `Empty` inherits the source's temperature, since it is physically made of
/// liquid that came from there.
#[allow(clippy::too_many_arguments)]
fn write_liquid_transfer<S: CellSurface>(
    surface: &mut S,
    sx: i32,
    sy: i32,
    src: Cell,
    new_src_fill: u16,
    dx: i32,
    dy: i32,
    dst: Cell,
    new_dst_fill: u16,
    dst_revisited: bool,
) {
    if new_src_fill == 0 {
        surface.set(sx, sy, Cell::EMPTY);
    } else {
        let mut new_src = src;
        new_src.set_aux(new_src_fill);
        surface.set(sx, sy, new_src);
    }

    let mut new_dst = if dst.is_empty() {
        Cell::new(src.material, src.shade).with_temperature(src.temperature())
    } else {
        dst
    };
    new_dst.set_aux(new_dst_fill);
    surface.set(dx, dy, new_dst.with_moved(dst_revisited));
}

/// A `Liquid` cell's fill, treating an untouched `aux == 0` as full rather
/// than empty -- see `material::LIQUID_FULL`'s own doc for why that is the
/// correct reading rather than a special case to work around. `pub(crate)`
/// so tests elsewhere (`parallel.rs`) can sum total fill volume rather than
/// cell count -- the actual conserved quantity under this model, since a
/// single full cell can now split its fill across two cells.
#[inline]
pub(crate) fn liquid_fill(cell: Cell) -> u16 {
    let aux = cell.aux();
    if aux == 0 {
        material::LIQUID_FULL
    } else {
        aux
    }
}

/// Rises, then spreads. Gases are the mirror of liquids under gravity.
fn update_gas<S: CellSurface>(surface: &mut S, x: i32, y: i32, rightward: bool) -> bool {
    if try_move(surface, x, y, x, y - 1) {
        return true;
    }
    let (first, second) = if surface.rng().flip() { (-1, 1) } else { (1, -1) };
    if try_move(surface, x, y, x + first, y - 1) || try_move(surface, x, y, x + second, y - 1) {
        return true;
    }

    // Capped for the same reason as `SURFACE_SEARCH`: a rule must not read
    // further than the sweep region is widened.
    let dispersion = (surface.materials().get(surface.get(x, y).material).dispersion as i32).min(MAX_REACH);
    flow_sideways(surface, x, y, first, dispersion, 1, rightward)
        || flow_sideways(surface, x, y, second, dispersion, 1, rightward)
}

/// Walk up to `max` cells in `dir`, stopping at the first obstruction, and move
/// to the furthest reachable cell.
///
/// Walking cell by cell rather than jumping keeps the motion continuous, so
/// liquid cannot tunnel through a one-cell wall. The walk also stops early at
/// the first position with open space to fall into, which is what makes water
/// spill off a ledge instead of sliding past it.
///
/// `support_dy` points from the cell toward whatever would be pressing on it:
/// -1 for liquids, which are pressed from above, +1 for gases, which are
/// pressed from below.
fn flow_sideways<S: CellSurface>(
    surface: &mut S,
    x: i32,
    y: i32,
    dir: i32,
    max: i32,
    support_dy: i32,
    rightward: bool,
) -> bool {
    if max <= 0 {
        return false;
    }
    // The scan will reach the destination again only if the cell is moving the
    // same way the scan is.
    let revisited = (dir > 0) == rightward;

    let fall_dy = -support_dy;

    // Walk as far along the row as the cell can reach, stopping early if it
    // finds somewhere to fall.
    let mut target = x;
    let mut can_fall_at_target = false;
    for step in 1..=max {
        let tx = x + dir * step;
        if !surface.in_bounds(tx, y) || !surface.is_empty(tx, y) {
            break;
        }
        target = tx;
        if surface.in_bounds(tx, y + fall_dy) && surface.is_empty(tx, y + fall_dy) {
            can_fall_at_target = true;
            break;
        }
    }

    if target == x {
        return false;
    }

    // Somewhere to fall, or something stacked on top pushing it aside: move.
    if can_fall_at_target || is_pressured(surface, x, y, support_dy) {
        surface.move_cell(x, y, target, y, revisited);
        return true;
    }

    // Otherwise this is a free-surface cell, and a lateral move at constant
    // height changes nothing about its energy. Moving anyway is what makes
    // water jitter forever and keeps its chunk permanently awake; refusing to
    // move at all leaves water stacked in a stable slope like a powder, because
    // the nearest place it could fall is often further than one dispersion step.
    //
    // So look further along the row for somewhere to fall. Moving toward it
    // strictly reduces the distance, which both levels the surface and
    // terminates, rather than oscillating.
    if downhill_distance(surface, target, y, dir, fall_dy, SURFACE_SEARCH).is_some() {
        surface.move_cell(x, y, target, y, revisited);
        return true;
    }

    false
}

/// How far a free liquid surface looks along its row for somewhere to fall.
///
/// Capped at `MAX_REACH` because sweep regions are widened by exactly that
/// much: looking further would mean acting on a cell that no longer wakes this
/// one when it changes, and the liquid would go stale mid-flow.
const SURFACE_SEARCH: i32 = MAX_REACH;

/// Distance to the nearest cell along the row from which the material could
/// fall, or `None` if there is none within `reach` or the way is blocked.
///
/// The run has to be clear the whole way, because the cell has to travel along
/// it — a gap on the far side of a wall is not reachable.
fn downhill_distance<S: CellSurface>(
    surface: &S,
    x: i32,
    y: i32,
    dir: i32,
    fall_dy: i32,
    reach: i32,
) -> Option<i32> {
    for step in 1..=reach {
        let tx = x + dir * step;
        if !surface.in_bounds(tx, y) || !surface.is_empty(tx, y) {
            return None;
        }
        if surface.in_bounds(tx, y + fall_dy) && surface.is_empty(tx, y + fall_dy) {
            return Some(step);
        }
    }
    None
}

/// Whether a cell is being pressed on by material stacked against gravity.
///
/// The cheapest useful stand-in for hydrostatic pressure: liquid with liquid or
/// something heavier resting on it spreads, liquid with open air above it does
/// not. M3 replaces this with the real thing.
fn is_pressured<S: CellSurface>(surface: &S, x: i32, y: i32, support_dy: i32) -> bool {
    let presser = surface.get(x, y + support_dy);
    if presser.is_empty() {
        return false;
    }
    let self_density = surface.materials().density(surface.get(x, y).material);
    // Solids count: liquid trapped under stone is under pressure, and bedrock's
    // infinite density makes the world floor and ceiling press inward.
    surface.materials().density(presser.material) >= self_density
}

/// Attempt to move the cell at `(x, y)` into `(tx, ty)`.
///
/// A cell always moves into empty space. Otherwise it may displace a fluid by
/// density, and the comparison flips with direction: moving down, the mover
/// must be heavier (sand sinks through water); moving up, lighter (smoke rises
/// through water). Sideways moves never displace, or liquids would churn
/// endlessly through each other and never settle.
fn try_move<S: CellSurface>(surface: &mut S, x: i32, y: i32, tx: i32, ty: i32) -> bool {
    if !surface.in_bounds(tx, ty) {
        return false;
    }

    // Rows are swept bottom to top, so a downward move lands in a row already
    // passed and will not be revisited; an upward move lands in one still to
    // come and must be flagged.
    let revisited = ty < y;

    let dst = surface.get(tx, ty);
    if dst.is_empty() {
        surface.move_cell(x, y, tx, ty, revisited);
        return true;
    }

    let dst_kind = surface.materials().kind(dst.material);
    if !dst_kind.is_displaceable() {
        return false;
    }

    let src_density = surface.materials().density(surface.get(x, y).material);
    let dst_density = surface.materials().density(dst.material);
    let displaces = match (ty - y).signum() {
        1 => src_density > dst_density,
        -1 => src_density < dst_density,
        _ => false,
    };

    if displaces {
        surface.move_cell(x, y, tx, ty, revisited);
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_with_floor() -> World {
        let mut w = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            w.set(x, 127, Cell::new(material::STONE, 0));
        }
        w
    }

    fn run(w: &mut World, frames: usize) {
        for _ in 0..frames {
            step(w);
        }
    }

    #[test]
    fn sand_falls_one_cell_per_frame() {
        let mut w = world_with_floor();
        w.set(10, 0, Cell::new(material::SAND, 0));
        step(&mut w);
        assert!(w.get(10, 0).is_empty());
        assert_eq!(w.get(10, 1).material, material::SAND);
        step(&mut w);
        assert_eq!(w.get(10, 2).material, material::SAND);
    }

    #[test]
    fn sand_comes_to_rest_on_the_floor() {
        let mut w = world_with_floor();
        w.set(10, 0, Cell::new(material::SAND, 0));
        run(&mut w, 200);
        assert_eq!(w.get(10, 126).material, material::SAND);
    }

    #[test]
    fn sand_is_conserved() {
        let mut w = world_with_floor();
        for x in 20..40 {
            for y in 0..10 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        let before = count(&w, material::SAND);
        run(&mut w, 60);
        assert_eq!(count(&w, material::SAND), before, "sand was created or destroyed");
    }

    #[test]
    fn sand_forms_a_pile_rather_than_a_column() {
        let mut w = world_with_floor();
        // Drop a tall thin column onto a point.
        for y in 0..60 {
            w.set(64, y, Cell::new(material::SAND, 0));
        }
        run(&mut w, 500);
        // The base must be wider than the single column it fell from.
        let base_width = (0..128)
            .filter(|&x| w.get(x, 126).material == material::SAND)
            .count();
        assert!(base_width > 3, "pile did not spread: base width {base_width}");
    }

    #[test]
    fn stone_never_moves() {
        let mut w = world_with_floor();
        w.set(10, 10, Cell::new(material::STONE, 0));
        run(&mut w, 50);
        assert_eq!(w.get(10, 10).material, material::STONE);
    }

    #[test]
    fn water_finds_its_level() {
        let mut w = world_with_floor();
        // A tall narrow column of water in the middle.
        for y in 100..127 {
            for x in 62..66 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        run(&mut w, 600);
        // It must have spread well beyond the four columns it started in.
        let width = (0..128)
            .filter(|&x| w.get(x, 126).material == material::WATER)
            .count();
        assert!(width > 20, "water did not spread: width {width}");
    }

    /// Sand cells that the movement rules say could still move.
    ///
    /// A settled world must contain none of these. Any that remain are cells
    /// the sweep stopped examining — frozen rather than at rest.
    ///
    /// Covers rolling as well as falling. Checking only for somewhere to fall
    /// would miss a grain stranded on a slope shallower than its angle of
    /// repose, which is exactly what the position-keyed reach exists to
    /// prevent.
    fn unstable_sand(w: &World) -> Vec<(i32, i32)> {
        let b = w.bounds().unwrap();
        let mut out = Vec::new();
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                if w.get(x, y).material != material::SAND {
                    continue;
                }
                let can_fall = [0, -1, 1]
                    .iter()
                    .any(|dx| w.in_bounds(x + dx, y + 1) && w.is_empty(x + dx, y + 1));
                let reach = w.materials.get(material::SAND).roll_reach_at(x, y);
                let can_roll = reach > 0
                    && (downhill_distance(w, x, y, -1, 1, reach).is_some()
                        || downhill_distance(w, x, y, 1, 1, reach).is_some());
                if can_fall || can_roll {
                    out.push((x, y));
                }
            }
        }
        out
    }

    #[test]
    fn settled_sand_is_never_left_unsupported() {
        let mut w = world_with_floor();
        // A tall block in mid-air, well away from the dirty region the floor
        // created, so it has to collapse entirely on its own.
        for y in 20..100 {
            for x in 40..90 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        run(&mut w, 4000);

        assert_eq!(w.active_chunk_count(), 0, "world never settled");
        let bad = unstable_sand(&w);
        assert!(
            bad.is_empty(),
            "{} sand cells frozen with empty space beneath them, e.g. {:?}",
            bad.len(),
            &bad[..bad.len().min(8)]
        );
    }

    #[test]
    fn every_unstable_cell_is_scheduled_for_examination() {
        // The core invariant of the dirty rectangle system. If a cell can move
        // but no upcoming sweep covers it, it is frozen — and this catches the
        // exact frame it happens rather than the wreckage 4000 frames later.
        let mut w = world_with_floor();
        for y in 20..100 {
            for x in 40..90 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        for frame in 0..4000 {
            step(&mut w);
            for (x, y) in unstable_sand(&w) {
                let coord = crate::sim::chunk::ChunkCoord::containing(x, y);
                let covered = w.sweep_region(coord).is_some_and(|r| r.contains(x, y));
                assert!(
                    covered,
                    "frame {frame}: ({x}, {y}) can move but chunk {coord:?} \
                     will not examine it (region {:?})",
                    w.sweep_region(coord)
                );
            }
        }
    }

    #[test]
    fn sand_is_stable_when_every_chunk_is_swept_in_full() {
        // The control for `settled_sand_is_never_left_unsupported`. If sand
        // settles cleanly here but not there, the movement rules are fine and
        // the fault is in the dirty rectangles deciding what to examine.
        let mut w = world_with_floor();
        for y in 20..100 {
            for x in 40..90 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        for _ in 0..4000 {
            w.wake_all();
            step(&mut w);
        }
        let bad = unstable_sand(&w);
        assert!(
            bad.is_empty(),
            "{} unsupported cells even with sleeping disabled, e.g. {:?}",
            bad.len(),
            &bad[..bad.len().min(8)]
        );
    }

    #[test]
    fn water_settles_instead_of_jittering_forever() {
        // A free liquid surface must come to rest. Cells that slide back and
        // forth look wrong and, worse, keep their chunk permanently awake,
        // which defeats sleeping in any world containing a puddle.
        let mut w = world_with_floor();
        w.paint_circle(64, 20, 10, material::WATER);
        run(&mut w, 1500);
        assert_eq!(
            w.active_chunk_count(),
            0,
            "water never settled; the world will never sleep"
        );
    }

    #[test]
    fn water_settles_flatter_than_a_powder_would() {
        // The distinguishing property of a liquid: it must not hold a slope the
        // way sand does. This is a bound on the residual wedge M2 leaves behind,
        // not a claim that the surface is perfectly level.
        let mut water = world_with_floor();
        water.paint_circle(64, 20, 12, material::WATER);
        run(&mut water, 1500);

        let mut sand = world_with_floor();
        sand.paint_circle(64, 20, 12, material::SAND);
        run(&mut sand, 1500);

        let water_spread = surface_spread(&water, material::WATER);
        let sand_spread = surface_spread(&sand, material::SAND);
        // Currently around 84 columns against sand's 44. A perfectly level
        // surface would reach the full 128, so this is a bound on the residual
        // wedge M2 leaves behind, not a claim of flatness.
        assert!(
            water_spread * 2 > sand_spread * 3,
            "water spread {water_spread} is not meaningfully wider than sand's {sand_spread}"
        );
    }

    /// Width of the material's footprint along the bottom row it occupies.
    fn surface_spread(w: &World, id: material::MaterialId) -> usize {
        (0..128).filter(|&x| w.get(x, 126).material == id).count()
    }

    /// Standard deviation of a `Liquid` material's fill across every column's
    /// bottom-most occupied row -- a direct measure of "how flat is the
    /// surface", used below to confirm a column actually levels rather than
    /// just holding a slope that happens to have decent overall spread.
    fn column_height_spread(w: &World, id: material::MaterialId, floor_y: i32) -> f64 {
        let heights: Vec<i32> = (0..128)
            .filter_map(|x| (0..floor_y).find(|&y| w.get(x, y).material == id).map(|y| floor_y - y))
            .collect();
        if heights.len() < 2 {
            return 0.0;
        }
        let mean = heights.iter().sum::<i32>() as f64 / heights.len() as f64;
        let variance = heights.iter().map(|&h| (h as f64 - mean).powi(2)).sum::<f64>() / heights.len() as f64;
        variance.sqrt()
    }

    #[test]
    fn a_wide_deep_water_column_levels_out_instead_of_only_eroding_at_the_edges() {
        // The scene from the reported bug: a tall, wide column of water on a
        // flat floor with open space on both sides. 100 cells wide, not a
        // smaller one, deliberately -- an independent review found that at
        // 40 cells wide this test passes even with `HORIZONTAL_TRANSFER_
        // REACH` reduced to 1 (immediate-neighbour-only), because pure
        // nearest-neighbour diffusion still manages to level a column that
        // size within a practical frame budget. 100 wide is big enough that
        // it does not: confirmed by temporarily setting
        // `HORIZONTAL_TRANSFER_REACH` to 1 and rerunning this exact test,
        // which fails (`active_chunk_count` stays nonzero at 3000 frames,
        // height varying roughly 6-39 cells across the world -- a visible
        // mound, not a puddle) where the real reach-8 code settles cleanly
        // and flat well within the same budget.
        //
        // Separately, reverting the whole rewrite to the pre-existing
        // `dispersion`-radius search does *not* make this test fail --
        // the old model still levels an isolated column like this one
        // given enough time, because edge cells and the free top surface
        // (`SURFACE_SEARCH`, a much longer lookahead than `dispersion`) can
        // peel material away and let gravity refill from above, reshaping
        // the whole column over many small steps even though no single
        // interior cell can ever move sideways on its own. The reported
        // complaint was about *how that looks while it happens* (a rigid-
        // looking block with an eroding edge, not a fluid slumping
        // naturally) more than whether it eventually finishes -- best
        // confirmed visually via §1's capture tool on this same scene
        // (already done, see `PLAN.md`), not by a single before/after
        // number a slow-but-technically-converging old model could also
        // satisfy given enough frames.
        let mut w = World::new(Rect::new(0, 0, 199, 149));
        for x in 0..200 {
            w.set(x, 149, Cell::new(material::STONE, 0));
        }
        for x in 60..160 {
            for y in 100..149 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        run(&mut w, 3000);

        assert_eq!(w.active_chunk_count(), 0, "water never fully settled");

        let spread = (0..200).filter(|&x| (100..149).any(|y| w.get(x, y).material == material::WATER)).count();
        assert!(spread > 150, "column did not level out: final spread only {spread} columns wide");

        let height_spread = column_height_spread(&w, material::WATER, 149);
        assert!(height_spread < 10.0, "surface is not close to flat: height std-dev {height_spread:.2}");
    }

    #[test]
    fn sand_sinks_through_water() {
        let mut w = world_with_floor();
        for y in 100..127 {
            for x in 0..128 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        w.set(64, 100, Cell::new(material::SAND, 0));
        run(&mut w, 400);
        // Sand ends up below where it started, having displaced water.
        let sand_y = (0..128)
            .find(|&y| (0..128).any(|x| w.get(x, y).material == material::SAND))
            .expect("sand vanished");
        assert!(sand_y > 120, "sand did not sink through water: y = {sand_y}");
    }

    #[test]
    fn smoke_rises() {
        let mut w = world_with_floor();
        w.set(64, 120, Cell::new(material::SMOKE, 0));
        run(&mut w, 200);
        let smoke_y = (0..128)
            .find(|&y| (0..128).any(|x| w.get(x, y).material == material::SMOKE))
            .expect("smoke vanished");
        assert!(smoke_y < 20, "smoke did not rise: y = {smoke_y}");
    }

    #[test]
    fn smoke_rises_through_water() {
        let mut w = world_with_floor();
        for y in 60..127 {
            for x in 0..128 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        w.set(64, 126, Cell::new(material::SMOKE, 0));
        run(&mut w, 400);
        let smoke_y = (0..128)
            .find(|&y| (0..128).any(|x| w.get(x, y).material == material::SMOKE))
            .expect("smoke vanished");
        assert!(smoke_y < 60, "smoke did not rise through water: y = {smoke_y}");
    }

    #[test]
    fn a_settled_world_goes_to_sleep() {
        let mut w = world_with_floor();
        w.set(10, 0, Cell::new(material::SAND, 0));
        run(&mut w, 400);
        // This is the whole point of dirty rectangles: once nothing is moving,
        // no chunk should be scheduled for sweeping.
        assert_eq!(
            w.active_chunk_count(),
            0,
            "world never settled; sleeping is not working"
        );
    }

    #[test]
    fn a_settled_world_wakes_when_painted() {
        let mut w = world_with_floor();
        run(&mut w, 400);
        assert_eq!(w.active_chunk_count(), 0);
        w.paint_circle(64, 10, 4, material::SAND);
        step(&mut w);
        assert!(w.active_chunk_count() > 0, "painting did not wake the world");
    }

    #[test]
    fn a_burning_cell_never_lets_its_chunk_sleep_until_it_burns_out() {
        // M14's version of the same invariant class as
        // `settled_sand_is_never_left_unsupported`: something that still has
        // work to do must never be allowed to fall asleep. A static burning
        // solid has no movement to keep its chunk dirty, so this is entirely
        // dependent on fire::update's own writes doing that job — through the
        // real sweep (`step`), not a direct `fire::update` call, since the
        // property under test is about the scheduler, not the fire logic
        // itself.
        let mut w = world_with_floor();
        // A small basin, or the oil (a liquid) would simply fall the 67
        // cells to the floor and flow away before the first check below ever
        // runs — the fixed position being checked has to actually hold it.
        w.set(63, 61, Cell::new(material::STONE, 0));
        w.set(64, 61, Cell::new(material::STONE, 0));
        w.set(65, 61, Cell::new(material::STONE, 0));
        w.set(62, 60, Cell::new(material::STONE, 0));
        w.set(66, 60, Cell::new(material::STONE, 0));
        let mut burning = Cell::new(material::OIL, 0);
        burning.ignite(60); // 1 second at 60 fps
        w.set(64, 60, burning);

        let mut still_burning_frames = 0;
        for _ in 0..120 {
            step(&mut w);
            if w.get(64, 60).is_burning() || w.get(64, 60).material == material::OIL {
                assert!(
                    w.active_chunk_count() > 0,
                    "chunk slept while oil was still burning or unburnt-but-hot"
                );
                still_burning_frames += 1;
            }
        }
        assert!(still_burning_frames > 0, "test setup did not actually observe any burning frames");

        // And it must eventually settle once burnout completes and residual
        // heat converges — not stay awake forever either.
        run(&mut w, 2000);
        assert_eq!(
            w.active_chunk_count(),
            0,
            "world never settled after the fire burned out"
        );
        assert_eq!(w.get(64, 60).material, material::ASH, "oil should have burned out into ash");
    }

    #[test]
    fn material_keeps_moving_across_a_chunk_boundary() {
        // The classic sleeping bug: material freezes at the seam between chunks.
        let mut w = world_with_floor();
        w.set(64, 60, Cell::new(material::SAND, 0)); // first column of chunk (1, 0)
        run(&mut w, 400);
        assert_eq!(w.get(64, 126).material, material::SAND);

        // And again for a grain that must cross the vertical seam at y = 64.
        let mut w = world_with_floor();
        w.set(10, 60, Cell::new(material::SAND, 0));
        run(&mut w, 400);
        assert_eq!(w.get(10, 126).material, material::SAND);
    }

    /// Spread (footprint width) of a water column whose right edge sits at
    /// world x `edge_x`, after `frames` steps.
    fn water_column_spread_after(edge_x: i32, frames: usize) -> usize {
        let mut w = World::new(Rect::new(0, 0, 199, 149));
        for x in 0..200 {
            w.set(x, 149, Cell::new(material::STONE, 0));
        }
        for x in (edge_x - 30)..edge_x {
            for y in 110..149 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        run(&mut w, frames);
        (0..200).filter(|&x| (110..149).any(|y| w.get(x, y).material == material::WATER)).count()
    }

    #[test]
    fn liquid_leveling_speed_does_not_depend_on_chunk_alignment() {
        // Direct response to a live screenshot the owner shared during this
        // rewrite (F1 chunk overlay on): a water column's visible change was
        // concentrated near chunk-boundary gridlines. Traced to the same
        // dispersion-radius limitation the rest of this rewrite fixes, not a
        // separate boundary bug -- but the fix's own horizontal-transfer
        // writes now cross chunk boundaries routinely, and if any of those
        // writes failed to wake the neighbouring chunk (`CellSurface::set`
        // already does this for every implementer, so nothing here should be
        // able to skip it), a column happening to sit at a chunk-aligned
        // position could stall exactly the way the original bug looked.
        //
        // This isn't a revert-and-confirm-it-fails test the way most of this
        // rewrite's other tests are -- there is no separate wake-the-
        // neighbour logic of this rewrite's own to break, since every
        // transfer already goes through the same `CellSurface::set` seam
        // every other cross-chunk write in the codebase uses (already
        // covered by `world.rs`'s own `a_write_at_a_chunk_edge_wakes_the_
        // neighbour`). What this test adds is specific to the reported
        // scenario: confirming a chunk-aligned column is not measurably
        // slower to level than an unaligned one of the same shape.
        let aligned = water_column_spread_after(64, 400); // right edge exactly on the chunk(0,0)/chunk(1,0) seam
        let unaligned = water_column_spread_after(97, 400); // not a multiple of CHUNK_SIZE (64) anywhere

        assert!(
            aligned * 2 > unaligned,
            "chunk-aligned column spread ({aligned}) is far behind the unaligned one ({unaligned}) -- \
             looks like a real boundary-wake regression, not just ordinary run-to-run variation"
        );
    }

    #[test]
    fn nothing_escapes_the_world() {
        let mut w = World::new(Rect::new(0, 0, 127, 127));
        for x in 0..128 {
            w.set(x, 100, Cell::new(material::SAND, 0));
            w.set(x, 101, Cell::new(material::WATER, 0));
        }
        let before = count(&w, material::SAND) + count(&w, material::WATER);
        run(&mut w, 500);
        let after = count(&w, material::SAND) + count(&w, material::WATER);
        assert_eq!(before, after, "material fell out of the world");
    }

    fn count(w: &World, id: material::MaterialId) -> usize {
        let b = w.bounds().unwrap();
        let mut n = 0;
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                if w.get(x, y).material == id {
                    n += 1;
                }
            }
        }
        n
    }
}
