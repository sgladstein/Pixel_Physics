//! M17: structural integrity — destructible building with no solver.
//!
//! Each `Solid` cell stores in `Cell::aux` its distance, in cells, to the
//! nearest anchor — bedrock, or the world edge (the same `Cell::OUT_OF_BOUNDS`
//! sentinel already used everywhere else a rule needs to treat the world's
//! edge as a wall). A cell whose distance exceeds its material's
//! `max_unsupported_span` is unsupported and breaks free, converting into
//! `breaks_into` — loose material the ordinary CA sweep then makes fall.
//! No polygons, no connected-component labelling, no physics solver; M8
//! upgrades the "falls as loose material" part into coherent tumbling
//! bodies later without this milestone needing to change.
//!
//! # Why this is a label-correcting relaxation, not a one-shot BFS
//!
//! Distance is a shortest-path property: `d = 0` at an anchor, otherwise
//! `d = 1 + min(neighbouring Solid cells' d)`. Recomputing it for one cell
//! is cheap; the question is when to bother. This module never scans the
//! whole world — `structural_tick` recomputes exactly one cell's distance
//! from its neighbours' *already-stored* values, and only propagates
//! (reschedules its `Solid` neighbours too) when the recomputed value
//! actually *changed*. A cell whose distance is stable simply stops being
//! rescheduled, exactly like a moss tip with nowhere left to grow —
//! propagation is a wavefront that spreads outward from a disturbance and
//! dies out on its own once nothing more is changing, which is what keeps
//! this "bounded by the size of the affected structure" the way the plan
//! asks for, not a cost proportional to world size.
//!
//! # Why checks are scheduled reactively, never at world-gen time
//!
//! If every `Solid` cell were checked from frame one, the sandbox's own
//! starting terrain would immediately fail: the floor is 8 cells thick
//! (deeper than stone's span of 3) and the decorative ledges float with no
//! path to an anchor at all. Under a literal reading of the mechanic both
//! would start crumbling the instant this milestone shipped — not a
//! demonstration of anything, just a surprise regression in what M1-M15
//! already established looked right. `structural_tick` is only ever
//! scheduled by something that actually disturbs a structure — painting
//! new `Solid` material, erasing it, an explosion clearing a radius (see
//! `World::paint_capsule` and `explosion::trigger`) — never by world
//! generation. Pre-placed terrain's `aux` simply stays at its default (0)
//! forever unless something reaches over and disturbs it, which is
//! indistinguishable from "anchored" for any cell nothing ever asks about.
//! Once a player builds and then breaks something, the mechanic is exactly
//! as real as the plan describes.

use super::cell::Cell;
use super::material::{self, MaterialId, MaterialKind};
use super::scheduler::{ActiveKind, ActiveSite};
use super::world::World;

const NEIGHBOURS_4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// Frames between a structural check and its next one, if its distance is
/// still changing. Deliberately faster than plant growth's tick intervals
/// (20-45 frames) — a structural failure reads as more urgent than organic
/// growth, and a fast-but-not-instant cascade is what makes a collapse look
/// progressive rather than either frozen or all-at-once.
const STRUCTURAL_TICK_INTERVAL: u64 = 5;

/// Recompute one `Solid` cell's distance to the nearest anchor from its
/// neighbours' currently-stored distances. Dispatched from
/// `scheduler::step` via `ActiveKind::StructuralCheck`.
pub fn tick(world: &mut World, site: &ActiveSite) -> Vec<ActiveSite> {
    let (x, y) = (site.x, site.y);
    let cell = world.get(x, y);
    // Architecture item 9: `Plant` too, not just `Solid` -- a tree trunk is
    // exactly as capable of being unsupported as a stone span is. See
    // `Cell::aux`'s own doc for why the slot this borrows was actually free.
    if !is_body_material(world, cell.material) {
        return Vec::new(); // no longer part of the structural system (destroyed, converted) -- nothing to track
    }
    // Deferred while burning, conservatively rather than out of necessity --
    // `Cell::aux` and the burn timer are separate fields now (`Cell`'s own
    // doc), so a burning cell's anchor distance is valid to read. Kept as a
    // defer anyway: the cell may still change material out from under this
    // check the moment it burns out (`burns_into`), and the burning cell
    // already keeps its chunk awake on its own via `fire.rs`, so nothing is
    // lost by waiting -- once it stops burning, a later disturbance (or this
    // same check, re-scheduled) picks the distance question back up.
    if cell.is_burning() {
        return vec![reschedule(world, x, y)];
    }

    let is_anchor = NEIGHBOURS_4.iter().any(|&(dx, dy)| world.get(x + dx, y + dy).material == material::BEDROCK);

    let new_distance: u16 = if is_anchor {
        0
    } else {
        let mut min_neighbour = u16::MAX;
        let mut has_usable_neighbour = false;
        let mut has_burning_neighbour = false;
        for (dx, dy) in NEIGHBOURS_4 {
            let neighbour = world.get(x + dx, y + dy);
            if !is_body_material(world, neighbour.material) {
                continue;
            }
            // Same conservative defer as the guard above: a burning
            // neighbour's `aux` is a valid distance now, not an aliased burn
            // timer, but the neighbour may still change material out from
            // under this the moment it burns out -- skipped for the same
            // reason, not because reading it would be wrong today.
            if neighbour.is_burning() {
                has_burning_neighbour = true;
                continue;
            }
            has_usable_neighbour = true;
            min_neighbour = min_neighbour.min(neighbour.aux());
        }
        if has_burning_neighbour && !has_usable_neighbour {
            // Every Solid neighbour that exists is mid-burn -- there is no
            // trustworthy data to relax from yet. Defer rather than reading
            // this as "no support at all", which would shatter the cell
            // the instant a neighbour catches fire regardless of its real
            // structural state.
            return vec![reschedule(world, x, y)];
        }
        min_neighbour.saturating_add(1) // saturates at u16::MAX, i.e. "unreachable" stays unreachable
    };

    let material = world.materials.get(cell.material);
    if new_distance > material.max_unsupported_span {
        let Some(into) = material.breaks_into else {
            // No configured fallback: this material's span is finite but it
            // has nowhere to fall back to. Treated as "not actually
            // participating" rather than silently deleting content a
            // author forgot to pair `breaks_into` with -- stays Solid,
            // stops checking (matches an unset span's own behaviour).
            return Vec::new();
        };
        break_free(world, x, y, into);
        // The neighbours that were relying on this cell as a stepping
        // stone need to recompute too -- this is what turns a single break
        // into a *cascade* rather than one isolated cell vanishing.
        return schedule_solid_neighbours(world, x, y);
    }

    if new_distance == cell.aux() {
        return Vec::new(); // stable -- stop being actively rescheduled
    }

    world.set(x, y, cell.with_aux(new_distance));
    let mut next = vec![reschedule(world, x, y)];
    next.extend(schedule_solid_neighbours(world, x, y));
    next
}

/// Small enough that one cell breaking free is a puff, not a blast --
/// `explosion::trigger` uses strengths of 150-200 for an actual explosion.
/// This exists so a structural failure has *any* field footprint at all
/// (`Reports/emergent-world-architecture.md` §5c: previously `break_free`
/// swapped the cell and returned silently, the only kind of destructive
/// event in the engine with none). `break_free` runs once per broken cell,
/// called repeatedly as a cascade progresses one reactive tick at a time
/// (`structural.rs`'s own module doc), so a real collapse of many cells
/// naturally accumulates into a larger disturbance than a single cell
/// breaking alone, with no extra bookkeeping needed to make that happen.
const COLLAPSE_IMPULSE_RADIUS: i32 = 3;
const COLLAPSE_IMPULSE_STRENGTH: f32 = 4.0;

fn break_free(world: &mut World, x: i32, y: i32, into: MaterialId) {
    let shades = world.materials.get(into).palette.len().max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    let temp = world.get(x, y).temperature();
    world.set(x, y, Cell::new(into, shade).with_temperature(temp));
    world.add_pressure_impulse(x, y, COLLAPSE_IMPULSE_RADIUS, COLLAPSE_IMPULSE_STRENGTH);
}

fn schedule_solid_neighbours(world: &World, x: i32, y: i32) -> Vec<ActiveSite> {
    NEIGHBOURS_4
        .iter()
        .filter_map(|&(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            if is_body_material(world, world.get(nx, ny).material) {
                Some(reschedule(world, nx, ny))
            } else {
                None
            }
        })
        .collect()
}

/// Whether `material` participates in the structural system at all —
/// `Solid` (M17) or `Plant` (architecture item 9, trees). Named after the
/// same helper `rigid.rs` (M8) already has for exactly the same "which
/// kinds of cell count as body, not air or clutter" question, since it's
/// the identical question asked for a different purpose.
fn is_body_material(world: &World, material: MaterialId) -> bool {
    matches!(world.materials.kind(material), MaterialKind::Solid | MaterialKind::Plant)
}

/// Due `STRUCTURAL_TICK_INTERVAL` frames from now, so a cascade advances one
/// step at a time rather than resolving its entire chain in a single frame —
/// see the module doc's note on why this reads as progressive collapse.
fn reschedule(world: &World, x: i32, y: i32) -> ActiveSite {
    ActiveSite { x, y, kind: ActiveKind::StructuralCheck, next_frame: world.frame + STRUCTURAL_TICK_INTERVAL }
}

impl World {
    /// Schedule a structural check at `(x, y)` for the next scheduler pass —
    /// `next_frame` is stamped in here rather than by callers, so every
    /// call site doesn't have to know `STRUCTURAL_TICK_INTERVAL` exists.
    /// Harmless (and cheap) to call on a position that isn't part of the
    /// structural system (`Solid`/`Plant`) at all; `tick` above no-ops
    /// immediately in that case.
    pub fn schedule_structural_check(&mut self, x: i32, y: i32) {
        let frame = self.frame;
        self.schedule_active_site(ActiveSite { x, y, kind: ActiveKind::StructuralCheck, next_frame: frame });
    }

    /// Schedule structural checks for `(x, y)` itself and every 4-neighbour
    /// (`is_body_material` decides which of them actually matter; harmless
    /// to call on the rest). The shape every disturbance site (painting,
    /// erasing, an explosion's cleared radius) actually wants: whatever
    /// just changed might itself need re-evaluating (if it's now `Solid`/
    /// `Plant`), and anything touching it might have just lost or gained a
    /// support.
    pub fn schedule_structural_check_around(&mut self, x: i32, y: i32) {
        self.schedule_structural_check(x, y);
        for (dx, dy) in NEIGHBOURS_4 {
            self.schedule_structural_check(x + dx, y + dy);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::scheduler;
    use crate::sim::update;

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 63, 63))
    }

    fn run(w: &mut World, frames: usize) {
        for _ in 0..frames {
            w.begin_step();
            scheduler::step(w);
            w.end_step();
        }
    }

    #[test]
    fn a_short_stone_span_stays_solid() {
        let mut w = test_world();
        // A stack 3 cells tall resting on row 63 -- the world's real bottom
        // edge for a 0..=63 world (its own neighbour at y=64 reads as the
        // out-of-bounds BEDROCK sentinel, which is what `is_anchor` keys
        // on) -- well within stone's span of 3.
        w.set(30, 63, Cell::new(material::STONE, 0)); // touches the bottom edge -> anchor
        w.set(30, 62, Cell::new(material::STONE, 0));
        w.set(30, 61, Cell::new(material::STONE, 0));
        w.schedule_structural_check_around(30, 63);
        w.schedule_structural_check_around(30, 62);
        w.schedule_structural_check_around(30, 61);
        run(&mut w, 200);

        assert_eq!(w.get(30, 63).material, material::STONE, "an anchored short stack broke");
        assert_eq!(w.get(30, 62).material, material::STONE, "an anchored short stack broke");
        assert_eq!(w.get(30, 61).material, material::STONE, "an anchored short stack broke");
    }

    #[test]
    fn a_stone_span_exceeding_its_tolerance_breaks_into_gravel() {
        let mut w = test_world();
        // A stack 6 cells tall resting on the real bottom edge (y=63) --
        // stone's span is 3, so the cells at distance 4 and 5 (the top two)
        // are unsupported once actually checked.
        for i in 0..6 {
            w.set(30, 63 - i, Cell::new(material::STONE, 0));
        }
        w.schedule_structural_check(30, 63 - 5); // the topmost cell, distance 5
        run(&mut w, 200);

        let gravel = w.materials.id_of("gravel").unwrap();
        assert_eq!(w.get(30, 63 - 5).material, gravel, "an over-span stone cell should have broken into gravel");
    }

    #[test]
    fn a_wood_beam_exceeding_its_span_breaks_into_deadwood() {
        // Architecture item 9: structural integrity extended to `Plant`,
        // exercised through the exact same span/break mechanism the stone
        // test above already proves, just with wood's own numbers (span 8,
        // `breaks_into: "deadwood"`).
        let mut w = test_world();
        let wood = w.materials.id_of("wood").unwrap();
        // A 12-cell horizontal beam anchored only at its left end (touching
        // the world's left edge) -- longer than wood's span of 8, so the
        // far end is unsupported once actually checked.
        for x in 0..12 {
            w.set(x, 30, Cell::new(wood, 0));
        }
        w.schedule_structural_check(11, 30); // the far end, distance 11
        run(&mut w, 200);

        let deadwood = w.materials.id_of("deadwood").unwrap();
        assert_eq!(w.get(11, 30).material, deadwood, "an over-span wood cell should have broken into deadwood");
    }

    #[test]
    fn a_short_wood_beam_stays_wood() {
        // The other half of the claim above: a beam within wood's own span
        // (8) must not break just because `Plant` is now checked at all.
        let mut w = test_world();
        let wood = w.materials.id_of("wood").unwrap();
        for x in 0..6 {
            w.set(x, 30, Cell::new(wood, 0));
        }
        w.schedule_structural_check(5, 30); // distance 5, within span
        run(&mut w, 200);

        assert_eq!(w.get(5, 30).material, wood, "a wood beam within its own span broke anyway");
    }

    #[test]
    fn burning_a_trees_base_collapses_the_rest_of_the_trunk() {
        // The end-to-end claim architecture item 9 actually cares about:
        // fire.rs's burnout path now schedules a structural check around
        // whatever it just burned away, so losing the base of a tree to
        // fire brings the rest of it down too -- not just erasing/painting
        // the base (already covered by the beam tests above), but burning
        // it, which is the realistic way a tree's own base disappears.
        let mut w = test_world();
        let wood = w.materials.id_of("wood").unwrap();
        // A 12-cell horizontal beam again (simpler to reason about than a
        // vertical trunk, and exercises the identical span/anchor logic),
        // anchored at the left edge.
        for x in 0..12 {
            w.set(x, 30, Cell::new(wood, 0));
        }
        run(&mut w, 200);
        assert_eq!(w.get(11, 30).material, wood, "test setup: the intact, anchored beam should not have broken yet");

        // Burn out the anchored end specifically -- ignite it with a timer
        // short enough to burn out well within this test's own run. Needs
        // the CA sweep running too, not just the scheduler (`run`'s own
        // loop above), since fire.rs's `update` -- and therefore the
        // burnout that schedules the structural recheck -- is only ever
        // invoked from there. Unlike the beam tests above, this can't just
        // wrap `update::step` in its own manual `begin_step`/`end_step`
        // pair the way `run`'s loop does for `scheduler::step` alone --
        // `update::step` already calls both internally, so doing it again
        // here double-advanced `world.frame` and mis-promoted the
        // dirty-rect state -- an earlier version of this test wrapped it
        // anyway and `active_chunk_count()` stayed stuck at 0 forever,
        // never actually burning the cell out at all.
        let mut burning = w.get(0, 30);
        burning.ignite(5);
        w.set(0, 30, burning);
        for _ in 0..400 {
            update::step(&mut w);
            scheduler::step(&mut w);
        }

        // Not "did (11, 30) become deadwood" -- deadwood is `Powder`, so
        // once it breaks free it's also subject to gravity like anything
        // else, and this test's beam has nothing under it to land on. The
        // observable claim is "no longer standing there as wood", plus a
        // sanity check that it actually became debris somewhere nearby
        // rather than simply vanishing.
        assert_ne!(w.get(11, 30).material, wood, "the far end of the beam never collapsed after fire burned away its anchor");
        let deadwood = w.materials.id_of("deadwood").unwrap();
        let ash = material::ASH;
        // The full world, not just near the beam's original height -- with
        // nothing underneath it, freed deadwood (a `Powder`) falls under
        // gravity same as anything else, and this world is only 64 cells
        // deep, so "somewhere in it" is still a meaningful check, not a
        // vacuous one.
        let debris_somewhere = (0..64).any(|x| (0..64).any(|y| { let m = w.get(x, y).material; m == deadwood || m == ash }));
        assert!(debris_somewhere, "the collapsed beam left no deadwood or ash debris anywhere in the world");
    }

    #[test]
    fn breaking_free_writes_a_pressure_impulse() {
        // Architecture §5c: previously break_free swapped the cell and
        // returned silently -- the only kind of destructive event in the
        // engine with no field footprint at all (explosion::trigger writes
        // one; a structural collapse didn't). run()'s own helper never
        // calls field::step, so the field grid never decays during this
        // test -- whatever add_pressure_impulse writes stays exactly as
        // written until read, no timing sensitivity to worry about.
        let mut w = test_world();
        for i in 0..6 {
            w.set(30, 63 - i, Cell::new(material::STONE, 0));
        }
        let (tx, ty) = (30, 63 - 5); // the topmost cell, distance 5, will break
        assert_eq!(w.field_at(tx, ty).pressure, 0.0, "test setup should start at ambient pressure");

        w.schedule_structural_check(tx, ty);
        run(&mut w, 200);

        let gravel = w.materials.id_of("gravel").unwrap();
        assert_eq!(w.get(tx, ty).material, gravel, "test setup should have broken the topmost cell");
        assert!(
            w.field_at(tx, ty).pressure.abs() > 0.5,
            "a structural break should have written a pressure impulse into the field, found {}",
            w.field_at(tx, ty).pressure
        );
    }

    #[test]
    fn cutting_a_bridges_support_makes_the_far_side_collapse() {
        let mut w = test_world();
        // A short horizontal bridge, anchored only at its left end (touching
        // the world's left edge). Every cell is within span=3 of the anchor
        // while the whole bridge is intact.
        for x in 0..4 {
            w.set(x, 30, Cell::new(material::STONE, 0));
        }
        w.schedule_structural_check(3, 30);
        run(&mut w, 200);
        let gravel = w.materials.id_of("gravel").unwrap();
        assert_eq!(w.get(3, 30).material, material::STONE, "the intact bridge should not have collapsed yet");

        // Cut the support at the anchored end -- the rest of the bridge is
        // now 1 cell further from any anchor than it was.
        w.set(0, 30, Cell::EMPTY);
        w.schedule_structural_check_around(0, 30);
        run(&mut w, 200);

        assert_eq!(w.get(3, 30).material, gravel, "the far end of the bridge never collapsed after its support was cut");
    }

    #[test]
    fn pre_placed_terrain_is_never_retroactively_checked() {
        // A floor thicker than stone's span (8 rows vs span 3) and a
        // floating, disconnected ledge -- built directly, the way
        // `app::build_terrain` does, never through a scheduling hook.
        let mut w = test_world();
        for x in 0..64 {
            for y in 55..63 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 10..20 {
            w.set(x, 20, Cell::new(material::STONE, 0)); // floating, touches nothing
        }
        run(&mut w, 500);

        for x in 0..64 {
            for y in 55..63 {
                assert_eq!(w.get(x, y).material, material::STONE, "pre-placed floor crumbled without ever being disturbed");
            }
        }
        for x in 10..20 {
            assert_eq!(w.get(x, 20).material, material::STONE, "pre-placed floating ledge crumbled without ever being disturbed");
        }
        assert_eq!(w.active_site_count(), 0, "nothing should have been scheduled for undisturbed terrain");
    }

    #[test]
    fn a_burning_solid_neighbours_burn_timer_is_never_read_as_its_distance() {
        // A real bug an independent review originally caught: the
        // neighbour-scanning loop read a burning `Solid` neighbour's
        // `aux()` -- its burn-timer countdown, not a distance -- as if it
        // were structural data. `Cell::aux` and the burn timer are separate
        // fields now, so the specific misread this test was written against
        // is no longer even possible; kept as a regression test for the
        // conservative defer-while-burning behaviour itself (see the
        // comments at both `is_burning()` checks in `tick` above), which
        // this scenario still exercises for real. Reachable in real play via
        // `explosion::trigger`'s fireball ring, which force-ignites nearby
        // material through `World::ignite_circle` regardless of
        // flammability, including stone.
        let mut w = test_world();
        // A is on fire with a burn timer (500 frames) far larger than any
        // shipped material's span -- if the old aliasing bug ever came back,
        // this is the value that would wrongly read as A's distance.
        w.set(30, 61, Cell::new(material::STONE, 0));
        let mut a = w.get(30, 61);
        a.ignite(500);
        w.set(30, 61, a);
        // B is A's only Solid neighbour and otherwise unsupported -- if B
        // ever computed a distance from A's data while A is burning, it
        // would come from the 500-frame timer, not a real distance, and
        // instantly exceed stone's span of 3.
        w.set(30, 60, Cell::new(material::STONE, 0));

        let site = ActiveSite { x: 30, y: 60, kind: ActiveKind::StructuralCheck, next_frame: 0 };
        tick(&mut w, &site);

        assert_eq!(
            w.get(30, 60).material,
            material::STONE,
            "B broke, which means it read A's burn timer as a distance instead of deferring"
        );
        assert_eq!(w.get(30, 60).aux(), 0, "B's distance should not have been computed while its only Solid neighbour is unusable (burning)");
    }

    #[test]
    fn editing_max_unsupported_span_while_running_changes_what_stands() {
        let mut w = test_world();
        // Same 6-tall stack resting on the real bottom edge (y=63) as
        // `a_stone_span_exceeding_its_tolerance_breaks_into_gravel` -- the
        // topmost cell sits at distance 5, past the shipped span of 3.
        for i in 0..6 {
            w.set(30, 63 - i, Cell::new(material::STONE, 0));
        }
        let stone = w.materials.id_of("stone").unwrap();
        assert!(w.materials.get(stone).max_unsupported_span < 5, "test assumes the shipped span is under 5");
        // Widen stone's tolerance via a manufactured, more tolerant material
        // set loaded through a synthetic reload rather than mutating the
        // registry directly, since that's the only public way content
        // changes at runtime.
        let dir = std::env::temp_dir().join("pixel-physics-m17-span-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("stone.ron"),
            "(name: \"stone\", kind: Solid, density: 2.5, colors: [(128,128,132)], max_unsupported_span: 10, breaks_into: \"gravel\")",
        )
        .unwrap();
        w.materials.reload(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        w.schedule_structural_check(30, 63 - 5);
        run(&mut w, 200);
        assert_eq!(w.get(30, 63 - 5).material, material::STONE, "widening the span at runtime should have let the tall stack stand");
    }
}
