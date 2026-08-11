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

use super::chunk::{Rect, MAX_REACH};
use super::fire;
use super::material::MaterialKind;
use super::world::World;

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

fn sweep(world: &mut World, region: Rect, rightward: bool) {
    for y in (region.min_y..=region.max_y).rev() {
        if rightward {
            for x in region.min_x..=region.max_x {
                update_cell(world, x, y, rightward);
            }
        } else {
            for x in (region.min_x..=region.max_x).rev() {
                update_cell(world, x, y, rightward);
            }
        }
    }
}

fn update_cell(world: &mut World, x: i32, y: i32, rightward: bool) {
    let cell = world.get(x, y);

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
        world.clear_moved(x, y);
        return;
    }

    // Before movement: a phase change (stone crossing its melting point) must
    // land before this frame's movement dispatch decides how the cell
    // behaves, or it would move as stone for one more frame after already
    // having become lava. `fire::update` may have changed the cell's
    // material, flags or temperature, so it is re-read from the world rather
    // than reusing the `cell` bound above.
    fire::update(world, x, y);
    let cell = world.get(x, y);

    match world.materials.kind(cell.material) {
        MaterialKind::Powder => update_powder(world, x, y, rightward),
        MaterialKind::Liquid => update_liquid(world, x, y, rightward),
        MaterialKind::Gas => update_gas(world, x, y, rightward),
        MaterialKind::Empty | MaterialKind::Solid => false,
    };
}

/// Falls straight down, then diagonally, then creeps along the slope.
fn update_powder(world: &mut World, x: i32, y: i32, rightward: bool) -> bool {
    if try_move(world, x, y, x, y + 1) {
        return true;
    }
    let (first, second) = if world.rng.flip() { (-1, 1) } else { (1, -1) };
    if try_move(world, x, y, x + first, y + 1) || try_move(world, x, y, x + second, y + 1) {
        return true;
    }
    roll_along_slope(world, x, y, rightward)
}

/// Creep one cell along a slope, toward the nearest place the grain could fall.
///
/// This is what gives a powder an angle of repose instead of a fixed 45
/// degrees. Falling and sliding diagonally can only ever build a 45 degree
/// pile, because a grain stops the moment it has support beneath and to both
/// sides. Rolling lets it keep going down a shallower slope, and the pile comes
/// to rest once no surface grain can see anywhere to fall within its reach —
/// which the material's friction angle sets.
fn roll_along_slope(world: &mut World, x: i32, y: i32, rightward: bool) -> bool {
    let material = world.get(x, y).material;
    let reach = world.materials.get(material).roll_reach_at(x, y);
    if reach <= 0 {
        return false;
    }

    let left = downhill_distance(world, x, y, -1, 1, reach);
    let right = downhill_distance(world, x, y, 1, 1, reach);

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
                if world.rng.flip() {
                    -1
                } else {
                    1
                }
            }
        },
    };

    world.move_cell(x, y, x + dir, y, (dir > 0) == rightward);
    true
}

/// Falls like a powder, then flows sideways to find its level.
fn update_liquid(world: &mut World, x: i32, y: i32, rightward: bool) -> bool {
    if try_move(world, x, y, x, y + 1) {
        return true;
    }
    let (first, second) = if world.rng.flip() { (-1, 1) } else { (1, -1) };
    if try_move(world, x, y, x + first, y + 1) || try_move(world, x, y, x + second, y + 1) {
        return true;
    }

    // Capped for the same reason as `SURFACE_SEARCH`: a rule must not read
    // further than the sweep region is widened.
    let dispersion = (world.materials.get(world.get(x, y).material).dispersion as i32).min(MAX_REACH);
    flow_sideways(world, x, y, first, dispersion, -1, rightward)
        || flow_sideways(world, x, y, second, dispersion, -1, rightward)
}

/// Rises, then spreads. Gases are the mirror of liquids under gravity.
fn update_gas(world: &mut World, x: i32, y: i32, rightward: bool) -> bool {
    if try_move(world, x, y, x, y - 1) {
        return true;
    }
    let (first, second) = if world.rng.flip() { (-1, 1) } else { (1, -1) };
    if try_move(world, x, y, x + first, y - 1) || try_move(world, x, y, x + second, y - 1) {
        return true;
    }

    // Capped for the same reason as `SURFACE_SEARCH`: a rule must not read
    // further than the sweep region is widened.
    let dispersion = (world.materials.get(world.get(x, y).material).dispersion as i32).min(MAX_REACH);
    flow_sideways(world, x, y, first, dispersion, 1, rightward)
        || flow_sideways(world, x, y, second, dispersion, 1, rightward)
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
fn flow_sideways(
    world: &mut World,
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
        if !world.in_bounds(tx, y) || !world.is_empty(tx, y) {
            break;
        }
        target = tx;
        if world.in_bounds(tx, y + fall_dy) && world.is_empty(tx, y + fall_dy) {
            can_fall_at_target = true;
            break;
        }
    }

    if target == x {
        return false;
    }

    // Somewhere to fall, or something stacked on top pushing it aside: move.
    if can_fall_at_target || is_pressured(world, x, y, support_dy) {
        world.move_cell(x, y, target, y, revisited);
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
    if downhill_distance(world, target, y, dir, fall_dy, SURFACE_SEARCH).is_some() {
        world.move_cell(x, y, target, y, revisited);
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
fn downhill_distance(
    world: &World,
    x: i32,
    y: i32,
    dir: i32,
    fall_dy: i32,
    reach: i32,
) -> Option<i32> {
    for step in 1..=reach {
        let tx = x + dir * step;
        if !world.in_bounds(tx, y) || !world.is_empty(tx, y) {
            return None;
        }
        if world.in_bounds(tx, y + fall_dy) && world.is_empty(tx, y + fall_dy) {
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
fn is_pressured(world: &World, x: i32, y: i32, support_dy: i32) -> bool {
    let presser = world.get(x, y + support_dy);
    if presser.is_empty() {
        return false;
    }
    let self_density = world.materials.density(world.get(x, y).material);
    // Solids count: liquid trapped under stone is under pressure, and bedrock's
    // infinite density makes the world floor and ceiling press inward.
    world.materials.density(presser.material) >= self_density
}

/// Attempt to move the cell at `(x, y)` into `(tx, ty)`.
///
/// A cell always moves into empty space. Otherwise it may displace a fluid by
/// density, and the comparison flips with direction: moving down, the mover
/// must be heavier (sand sinks through water); moving up, lighter (smoke rises
/// through water). Sideways moves never displace, or liquids would churn
/// endlessly through each other and never settle.
fn try_move(world: &mut World, x: i32, y: i32, tx: i32, ty: i32) -> bool {
    if !world.in_bounds(tx, ty) {
        return false;
    }

    // Rows are swept bottom to top, so a downward move lands in a row already
    // passed and will not be revisited; an upward move lands in one still to
    // come and must be flagged.
    let revisited = ty < y;

    let dst = world.get(tx, ty);
    if dst.is_empty() {
        world.move_cell(x, y, tx, ty, revisited);
        return true;
    }

    let dst_kind = world.materials.kind(dst.material);
    if !dst_kind.is_displaceable() {
        return false;
    }

    let src_density = world.materials.density(world.get(x, y).material);
    let dst_density = world.materials.density(dst.material);
    let displaces = match (ty - y).signum() {
        1 => src_density > dst_density,
        -1 => src_density < dst_density,
        _ => false,
    };

    if displaces {
        world.move_cell(x, y, tx, ty, revisited);
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell::Cell;
    use crate::sim::material;

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
        run(&mut w, 300);
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
