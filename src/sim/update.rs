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
//! Movement rules here are deliberately simple; M3 replaces the fall/diagonal
//! rules with a physically grounded model (friction angle, and — per
//! `Reports/granular-mechanics-research.md` §2/§3 — a two-angle repose model
//! for real avalanche hysteresis, not Bak–Tang–Wiesenfeld toppling, which
//! that report found does not match how real sandpiles actually avalanche).
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
use super::material::{self, MaterialKind, HORIZONTAL_TRANSFER_REACH};
use super::organism;
use super::surface::CellSurface;
use super::world::World;
use crate::sim::cell::Cell;

/// Sweep the whole world as a single region, ignoring the chunk
/// decomposition entirely — the control that identified the chunk-seam cliff
/// bug (`seam_cliffs` below). Not a driver: it has no dirty-rectangle
/// skipping, so it costs a full scan of the world every frame, and it is
/// only ever correct to compare against, never to ship. Kept because the one
/// question it answers — "is this behaviour coming from the movement rules,
/// or from how the sweep is cut into chunks?" — took three wrong hypotheses
/// to reach the first time.
#[cfg(test)]
pub(crate) fn step_monolithic(world: &mut World) {
    world.begin_step();
    let rightward = world.frame.is_multiple_of(2);
    let region = world.bounds().expect("an empty world has nothing to sweep");
    sweep(world, region, rightward);
    world.end_step();
}

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

    // Owned by a promoted liquid body (`Reports/liquid-heightfield-
    // design.md` §2a/§3c) -- the CA sweep must not move it or run fire/
    // organism-diffusion on it; the body's own serial phase (`liquid.rs`)
    // is the only thing allowed to change it, via `World::set_owned`.
    if cell.managed() {
        return;
    }

    // Consumed by this visit, before anything below can act on it: the flag
    // constrains the cell *above* this one, and rows are swept bottom to
    // top, so clearing it here is always early enough to have already done
    // its one frame of work and late enough not to be doing it twice. See
    // `FLAG_UNDERCUT` (`cell.rs`). Below the `managed` check, not above it,
    // so this stays inside the rule that nothing but the owning body writes
    // a managed cell -- quiet write or not. Guarded on the read because the
    // flag is rare and the write is still a write in the hottest loop in
    // the engine.
    if cell.undercut() {
        surface.clear_undercut(x, y);
    }

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

    // Organism resource/canopy-density diffusion (`Reports/tree-rewrite-
    // design.md` §3, §2b) runs from here, not the M16 active-site
    // schedule, for the same reason `fire::update` above does: a
    // `MatureBody` trunk cell needs to keep relaying resource even though
    // it is deliberately off the active-site schedule
    // (`design-philosophy.md` §3's "mature cells go fully inert"), and the
    // CA sweep is the one pass that already visits every cell in an awake
    // chunk regardless of scheduling. Costs nothing once a tree's chunk
    // actually goes to sleep, since nothing gets visited at all then --
    // consistent with "inert" in the sense that matters (CPU cost), even
    // though the cell type itself doesn't leave the active-site schedule
    // question. A no-op for `organism_id() == 0` (inert material),
    // checked first so this never runs for the overwhelming majority of
    // `Plant`-kind cells (hand-painted wood) that aren't organism tissue
    // at all.
    if cell.organism_id() != 0 {
        organism::diffuse_resource(surface, x, y);
    }

    match surface.materials().kind(cell.material) {
        MaterialKind::Powder => update_powder(surface, x, y, rightward),
        MaterialKind::Liquid => update_liquid(surface, x, y, rightward),
        MaterialKind::Gas => update_gas(surface, x, y, rightward),
        MaterialKind::Empty | MaterialKind::Solid | MaterialKind::Plant | MaterialKind::Creature => false,
    };
}

/// Falls straight down, then diagonally, then creeps along the slope.
fn update_powder<S: CellSurface>(surface: &mut S, x: i32, y: i32, rightward: bool) -> bool {
    // Straight down, unless the cell below is a hole opened this frame by
    // something escaping *sideways* out of it. Dropping into that hole is
    // how a vertical face survives: the vacancy the one escaping grain
    // leaves rides the bottom-to-top sweep all the way up the face, every
    // cell above it takes the free straight-down move in preference to its
    // own sideways escape, and the face conveys downward at one grain per
    // frame instead of toppling. Refusing here costs this grain one frame
    // and sends it to the diagonal and roll checks below -- which, on a
    // face, is exactly the sideways escape it should have taken. See
    // `FLAG_UNDERCUT` (`cell.rs`).
    let below = surface.get(x, y + 1);
    let hole_from_a_sideways_escape = below.is_empty() && below.undercut();
    if !hole_from_a_sideways_escape && try_move(surface, x, y, x, y + 1) {
        return true;
    }
    let (first, second) = if surface.rng().flip() { (-1, 1) } else { (1, -1) };
    if try_move(surface, x, y, x + first, y + 1) || try_move(surface, x, y, x + second, y + 1) {
        return true;
    }
    if roll_along_slope(surface, x, y, rightward) {
        return true;
    }

    // Nothing moved this frame: the cell has settled. Clear `flowing` so the
    // *next* time it's asked to roll, it needs the steeper stability angle
    // to start again — `Reports/granular-mechanics-research.md` §2's
    // hysteresis, the part a single-angle model can't express. Guarded on
    // the read so an already-settled cell (the overwhelming common case)
    // never re-writes itself and never wakes its own chunk for nothing.
    let cell = surface.get(x, y);
    if cell.flowing() {
        surface.set(x, y, cell.with_flowing(false));
    }
    false
}

/// Creep one cell along a slope, toward the nearest place the grain could fall.
///
/// This is what gives a powder an angle of repose instead of a fixed 45
/// degrees. Falling and sliding diagonally can only ever build a 45 degree
/// pile, because a grain stops the moment it has support beneath and to both
/// sides. Rolling lets it keep going down a shallower slope, and the pile comes
/// to rest once no surface grain can see anywhere to fall within its reach —
/// which the material's friction angle sets.
///
/// **Two-angle repose** (`Reports/granular-mechanics-research.md` §2): which
/// reach applies depends on whether this cell moved last frame
/// (`Cell::flowing()`, set generically by `CellSurface::move_cell` on every
/// successful move). A cell already flowing keeps using the shallower,
/// lenient `roll_reach_at` (`friction_angle`) — real motion doesn't stop
/// until the slope is genuinely down to the classical angle of repose. A
/// settled cell instead uses the stricter, shorter `stability_reach_at`
/// (`max_stability_angle`) — a resting pile can stand steeper than its own
/// repose angle without creeping, and only starts if a *nearby* opening
/// (within that shorter reach) actually appears, e.g. from something being
/// dug out beside it. This is real hysteresis: harder to start an avalanche
/// than to keep one going, which is why real sandpiles don't relevel to a
/// single angle the way the old one-angle model implied.
fn roll_along_slope<S: CellSurface>(surface: &mut S, x: i32, y: i32, rightward: bool) -> bool {
    let cell = surface.get(x, y);
    let mat = surface.materials().get(cell.material);
    let reach = if cell.flowing() {
        mat.roll_reach_at(x, y)
    } else {
        mat.stability_reach_at(x, y)
    };
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
///
/// **Vertical before horizontal, deliberately, after a reordering attempt
/// was tried and reverted.** `Reports/liquid-simulation-research.md` §5
/// recommended trying horizontal first, to fix a tall column stalling with
/// a visible step at chunk boundaries (locked in once as `parallel.rs`'s
/// `three_tall_columns_spanning_chunk_boundaries_flatten_within_900_
/// frames` — that test no longer exists, see below). It worked for that
/// specific symptom, but a live report caught a worse regression it
/// introduced: a column hitting a floor would visibly balloon out to
/// nearly 5x its cell count within a couple hundred frames before slowly
/// re-collapsing, while total fill stayed *exactly* conserved throughout
/// (measured every single frame, confirmed never to drift). Water is
/// incompressible; a genuinely buried cell (more of the same liquid,
/// still full, stacked directly on top of it) has no free surface and no
/// physical reason to leak fill sideways at all. The vertical-first order
/// below has a load-bearing side effect that isn't obvious from reading
/// it alone: a deep, blocked, full cell's vertical attempt only ever has
/// `LIQUID_MAX_COMPRESS` (1%) of genuine room, but that tiny transfer
/// still *succeeds* and returns early — which incidentally throttles that
/// cell out of horizontal transfer almost every frame, keeping a packed
/// column's interior inert the way real incompressible water would be.
/// Trying horizontal first for every cell (or even just for cells at the
/// literal free surface, checking only the cell directly above — also
/// tried, also insufficient) removes that throttle broadly enough that a
/// column's whole body starts leaking sideways within the same few frames
/// its base lands, diluting far below full before it can recollapse.
///
/// **Net conclusion: reverted, not fixed forward.** The stalling-at-chunk-
/// boundaries symptom this section's own git history once fixed is a real,
/// still-open problem (`Reports/liquid-simulation-research-r2.md` §5's
/// "wide bodies level in O(width²)" is the same root cause), but a correct
/// fix needs to distinguish "the free surface of a body" from "any cell
/// that happens to have room nearby" at the level of the whole connected
/// body, not per-cell — which is exactly what the heightfield/virtual-
/// pipes redesign (§5 there) is for. A per-cell heuristic could not be
/// found this session that fixed the stall without reintroducing this
/// worse, physically-nonsensical ballooning. Don't retry another per-cell
/// ordering tweak here without a new idea for that distinction; the two
/// already tried (unconditional, and gated on the immediate cell above)
/// are both known not to work.
fn update_liquid<S: CellSurface>(surface: &mut S, x: i32, y: i32, rightward: bool) -> bool {
    // Free-fall into open space, same whole-cell move as a powder. Not
    // routed through fill transfer, which is deliberately rate-limited by
    // `flow_rate` -- that rate-limit exists for the slower pressure-driven
    // redistribution once a cell is resting against something, and must not
    // also throttle an ordinary unobstructed drop through open air.
    //
    // Gated on `undercut` for exactly the reason `update_powder` is, and it
    // is the same bug: a body of water meeting a vertical chunk seam held a
    // terrace there, flat within each chunk column and dropping sharply on
    // the boundary, because both drivers sweep chunk by chunk and the
    // seam column could only shed one cell per frame off its bottom while
    // the column above slumped down to refill it. Reported from live play
    // with the F1 overlay on, risers landing exactly on the gridlines --
    // and the ragged one-cell-tall films fringing each riser are the same
    // thing seen close up. Measured on `seam_terracing` below: mean column
    // step across a seam went from 9.0x the interior step, and still
    // climbing, to 1.7x and falling.
    let below = surface.get(x, y + 1);
    let hole_from_a_sideways_escape = below.is_empty() && below.undercut();
    if !hole_from_a_sideways_escape && try_move(surface, x, y, x, y + 1) {
        return true;
    }
    let (first, second) = if surface.rng().flip() { (-1, 1) } else { (1, -1) };

    // Long-range lateral descent, checked *before* the diagonal fall.
    //
    // This is the mechanism that makes a liquid behave like a liquid, and
    // its absence is why water in this engine read as sand. Everything
    // below this point -- the diagonal fall, then fill transfer -- is
    // shared with `update_powder` or is diffusion, and neither can flatten
    // a pile quickly: the diagonal fall *builds* a repose-angle pile (it is
    // literally the powder rule), and fill equalisation moves a fraction of
    // a difference per frame, converging in O(width²).
    //
    // A cell that cannot drop straight down instead looks sideways for the
    // nearest column it could actually fall from, and travels there in one
    // step. That is ballistic rather than diffusive, so a pile collapses in
    // O(width / reach) frames instead of O(width²) -- the difference
    // between "flattens in a second" and "still sloped a minute later."
    //
    // Precedent, not invention: The Powder Toy's liquid movement does
    // exactly this (`rt`, up to 30 cells, `Simulation.cpp`), and Noita --
    // same 64×64 chunks and four-pass checkerboard as this engine -- allows
    // a pixel to move within its chunk plus 32 cells cardinally for the
    // same reason. `Reports/liquid-simulation-research-r2.md` §9 item 1
    // recommended precisely this ("evaluated before or alongside the
    // diagonal-fall check, not gated behind it") and it had never been
    // implemented; two earlier "reordering" attempts moved the *fill
    // transfer* calls, which sit downstream of the diagonal fall, and so
    // never tested this at all.
    // Absorption into a promoted body directly below outranks any lateral
    // movement. A cell resting on a body must *join* it (design doc §6b:
    // absorption is the designed seam between CA liquid and a body), not
    // skate along its managed surface hunting for an edge to fall off --
    // which is exactly what the search below would otherwise make it do,
    // since a body's surface is flat and its far edge is always lower.
    {
        let below = surface.get(x, y + 1);
        if below.managed() && below.material == surface.get(x, y).material {
            return transfer_liquid_vertical(surface, x, y);
        }
    }

    for dir in [first, second] {
        if let Some(tx) = find_lateral_descent(surface, x, y, dir) {
            if try_move(surface, x, y, tx, y + 1) {
                return true;
            }
        }
    }

    if try_move(surface, x, y, x + first, y + 1) || try_move(surface, x, y, x + second, y + 1) {
        return true;
    }

    if transfer_liquid_vertical(surface, x, y) {
        return true;
    }

    if transfer_liquid_horizontal(surface, x, y, first, rightward) || transfer_liquid_horizontal(surface, x, y, second, rightward) {
        return true;
    }

    // Nothing moved: clear `flowing`, the same way `update_powder` does and
    // for the same reason its guard exists -- read first, so a cell that is
    // already settled never re-writes itself and never wakes its own chunk
    // for nothing. Powders have always done this; liquids set the flag on
    // every move (`CellSurface::move_cell` is generic) and had nothing that
    // ever cleared it, so a liquid cell that moved once read as "flowing"
    // forever. Nothing consumed it, so that was harmless -- until
    // `render::GrainMode::Motion` wanted to tell moving water from still.
    let cell = surface.get(x, y);
    if cell.flowing() {
        surface.set(x, y, cell.with_flowing(false));
    }
    false
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
///
/// **Absorption** (`Reports/liquid-heightfield-design.md` §6b): when the
/// cell below is `FLAG_MANAGED` (owned by a promoted `liquid::LiquidBody`)
/// and the same material, the *entire* source cell is absorbed rather than
/// a `flow_rate`-limited amount — the body's own solver (once one exists)
/// spreads it across the body in O(width), so throttling the handoff here
/// would only make a waterfall pile up above the surface instead of being
/// drawn in. Exactly conservative: the source becomes `Cell::EMPTY` and the
/// same integer is credited to the body via `CellSurface::absorb_liquid`,
/// both in this one call, so the debit and credit can never be separated by
/// a failure in between (that method's own doc / design doc §8b).
fn transfer_liquid_vertical<S: CellSurface>(surface: &mut S, x: i32, y: i32) -> bool {
    if !surface.in_bounds(x, y + 1) {
        return false;
    }
    let src = surface.get(x, y);
    let dst = surface.get(x, y + 1);
    if dst.material != src.material {
        return false;
    }
    if dst.managed() {
        let src_fill = liquid_fill(src);
        surface.set(x, y, Cell::EMPTY);
        surface.absorb_liquid(x, y + 1, src_fill as u32);
        return true;
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
/// as their surrounding neighbours slowly finish levelling.
///
/// Originally tuned *up* to 150 (15% of `LIQUID_FULL`) because 8 left a wide
/// test puddle still visibly settling after ~12,000 frames -- but per
/// Report B (`Reports/liquid-simulation-research-r2.md`) §3c's own
/// instruction, that was always "treat this number as the diagnostic, not
/// the setting": a change to the leveling mechanism that doesn't let it come
/// back down didn't fix the underlying problem. The liquid heightfield
/// design (`Reports/liquid-heightfield-design.md`) promotes large connected
/// bodies out of this per-cell dead band entirely, onto a solver that
/// doesn't need one -- so this dead band now only has to cover *unpromoted*
/// liquid, and B-9 requires it drop back to <= 16. Dropped to exactly 16 (2%
/// of `LIQUID_FULL`, the B-9 ceiling), not 8: 8 measured at 31 units of
/// residual unevenness after 300 frames on B-9's own 100-column scene
/// (`a_wide_shallow_pool_levels_within_budget`), missing the 20-unit/
/// 300-frame bar; 16 measured at or under it. A real, re-measured trade of
/// precision for settling speed at unpromoted scale, not a rounding-level
/// tweak or a guess.
const MIN_LIQUID_TRANSFER: u16 = 16;

// `HORIZONTAL_TRANSFER_REACH` (how far `transfer_liquid_horizontal` below
// looks past the immediate neighbour for a genuinely emptier cell to level
// against, fixing the width-dependent slow convergence pure nearest-neighbour
// diffusion has) now lives in `material.rs`, imported above -- issue #3's
// per-chunk sweep-reach tracking (`Material::sweep_reach`, `chunk.rs`) needs
// this same number, and `chunk.rs` must not depend on `update.rs`. See its
// doc there for the full derivation, including the 100-cell-wide-column
// measurement that picked 8.

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
    // `Reports/liquid-simulation-research-r2.md` §3d: `aux == 0` on a
    // `Liquid` cell means "untouched, treat as full" (`liquid_fill`'s own
    // doc), not empty -- so writing it here would silently manufacture a
    // full cell of liquid from nothing. Both branches above already avoid
    // this by construction (the source converts to `Cell::EMPTY` outright
    // instead, and `new_dst_fill` is always `dst_fill + amount` with
    // `amount > 0`, both callers having already returned early on
    // `amount == 0`) -- this asserts that invariant stays true rather than
    // silently relying on it, since `Cell::set_aux` itself has no way to
    // check a material's kind and enforce it there.
    debug_assert_ne!(new_dst_fill, 0, "write_liquid_transfer must never write aux == 0 to a live Liquid cell -- convert to Cell::EMPTY instead");
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

/// How far a free gas surface looks along its row for somewhere to fall,
/// once `flow_sideways`'s initial `dispersion`-limited walk can't find
/// anywhere better. Named for what it was written for — a liquid's free
/// surface, before `HORIZONTAL_TRANSFER_REACH` moved `Liquid` off
/// `flow_sideways` entirely (see this module's own doc) — but `Gas` never
/// moved off it, so this is `Gas`-only now.
///
/// Capped at `MAX_REACH` because a chunk's sweep region can never be
/// widened past it (`Material::sweep_reach`'s own doc has the load-bearing
/// consequence: a gas cell's true worst-case reach is `dispersion +
/// SURFACE_SEARCH`, not `dispersion` alone) — looking further would mean
/// acting on a cell that no longer wakes this one when it changes, and the
/// gas would go stale mid-flow.
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
/// The nearest column within `LIQUID_LATERAL_REACH` in direction `dir` that
/// the liquid at `(x, y)` could fall from, with an unobstructed path along
/// row `y` to reach it. `None` if no such column exists within reach.
///
/// **Nearest, not furthest** — deliberately. Taking the furthest reachable
/// opening would let water skip past a nearer one and read as teleporting;
/// nearest keeps the motion legible as flow while still crossing the whole
/// reach when a pile genuinely is that wide.
///
/// **The path must be clear**, which is what keeps this physical rather
/// than teleportation: the scan stops at the first cell that is neither
/// open space nor more of this same liquid. It cannot pass through a wall,
/// through a different material, or through a promoted body's managed
/// cells. Same-material cells *are* passable — a liquid moving through its
/// own kind is exactly what "flowing" is, and requiring open space would
/// make this fire only for cells already at a free edge, which is the
/// too-narrow condition that made the original `dispersion` search useless
/// for a buried cell.
fn find_lateral_descent<S: CellSurface>(surface: &S, x: i32, y: i32, dir: i32) -> Option<i32> {
    let src = surface.get(x, y);

    // Buried cells skip the scan entirely. A cell with more of its own
    // liquid directly on top of it is not on the surface, and relocating it
    // sideways only shuffles interior water: whatever sits above would
    // immediately fall into the vacated spot, for the same net result as
    // moving that upper cell instead -- which this rule will do when it
    // reaches it, since the sweep visits every cell.
    //
    // This is purely a cost gate, not a movement rule: the diagonal fall
    // and fill transfer below still run for buried cells, so nothing that
    // could move is frozen. It matters because the scan costs up to
    // 2 * LIQUID_LATERAL_REACH `get` calls per cell per frame, and in the
    // serial driver every one of those is a chunk-map lookup. In a dense
    // pool the interior is O(area) while the surface is only O(width), so
    // without this gate the overwhelming majority of the work is spent on
    // cells that provably have nowhere better to be. Measured on the ascii
    // stress scene: this is the difference between roughly 100 ms and
    // baseline. The Powder Toy solves the same problem with its
    // `FLAG_STAGNANT` bit, narrowing the search for particles surrounded by
    // their own kind.
    // Gated on "is anything on top of me", not "is more of *my own kind* on
    // top of me". Both mean this cell is not at a free surface and so has
    // nothing to gain from relocating -- whatever covers it would simply
    // fall into the vacated spot -- but the narrower same-material test
    // left a large hole: liquid lying under a *different* material (water
    // beneath sand, the entire interface in a mixed scene) failed it and
    // ran the full scan every frame, walking the whole pool sideways
    // through its own kind. That interface is O(width) cells each paying
    // O(reach) chunk lookups, and it measured as the dominant cost of this
    // rule on the ascii stress scene.
    if !surface.get(x, y - 1).is_empty()
        && surface.get(x - 1, y).material == src.material
        && surface.get(x + 1, y).material == src.material
    {
        return None;
    }

    let src_density = surface.materials().density(src.material);

    for step in 1..=material::LIQUID_LATERAL_REACH {
        let tx = x + dir * step;
        if !surface.in_bounds(tx, y) {
            return None;
        }
        let path = surface.get(tx, y);
        // `is_empty()` is managed-aware, so a promoted body's cells
        // correctly block the path rather than being flowed through.
        if !path.is_empty() && path.material != src.material {
            return None;
        }

        if !surface.in_bounds(tx, y + 1) {
            continue;
        }
        let below = surface.get(tx, y + 1);
        if below.is_empty() {
            return Some(tx);
        }
        // Same downward density rule `try_move` applies, checked here so the
        // scan and the move it feeds can never disagree about what counts as
        // somewhere to fall.
        let below_kind = surface.materials().kind(below.material);
        if below_kind.is_displaceable() && src_density > surface.materials().density(below.material) {
            return Some(tx);
        }
    }
    None
}

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

    // Never displace a cell that has already moved this frame.
    //
    // `move_cell` flags a displaced cell when it is pushed into a row the
    // bottom-to-top sweep has not reached yet, but that flag only stops the
    // cell being processed as a *mover* -- `update_cell` skips it. Nothing
    // stopped it being displaced *again*, and that is the actual bug: with
    // sand resting on water, the sweep displaces the same water parcel once
    // per sand row as it works upward, so the water crossed the entire
    // height of the sand in a single frame. Measured directly on a walled
    // 50-row block: the highest water row went from 150 to 100 in one step,
    // which on screen is water erupting out of the top of a sinking blob
    // instead of welling up around it.
    //
    // Refusing here costs the mover one frame of waiting, which is the same
    // price every other cell pays for the one-move-per-frame rule the
    // `moved` flag exists to enforce.
    if dst.moved() {
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
    fn a_splash_settles_with_no_stray_droplets_and_no_mass_drift() {
        // `Reports/liquid-simulation-research-r2.md` §3a cites the standard
        // VOF failure mode of stray disconnected "flotsam" droplets and a
        // measured mass gain, and §3b proposes a local-height-function fix.
        // Investigated directly (three scenarios: a draining pool through a
        // floor hole, a multi-ledge settle, and this splash-impact one --
        // splash is where real VOF flotsam most commonly appears) before
        // building anything: transient single-cell isolation does appear
        // mid-motion (a cell genuinely falling alone is not a bug), but none
        // of the three left a stray droplet behind once the world actually
        // finished settling (`active_chunk_count() == 0`), and fill summed
        // across every water cell was conserved exactly. This locks that
        // finding in as a real regression guard rather than a one-off
        // check -- if a future change to the liquid model reintroduces
        // stuck fragments or a mass leak, this should catch it, even though
        // the local-height-function fix itself was not built.
        let mut w = world_with_floor();
        for y in 100..127 {
            for x in 40..90 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        // A dense column dropped from height to force a real splash impact,
        // not just a calm pour.
        for y in 0..40 {
            w.set(64, y, Cell::new(material::STONE, 0));
        }

        let fill_before: u64 = (0..128)
            .flat_map(|y| (0..128).map(move |x| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == material::WATER)
            .map(|(x, y)| liquid_fill(w.get(x, y)) as u64)
            .sum();

        let mut frames_run = 0;
        loop {
            step(&mut w);
            frames_run += 1;
            if w.active_chunk_count() == 0 || frames_run > 20_000 {
                break;
            }
        }
        assert_eq!(
            w.active_chunk_count(),
            0,
            "the splash never settled within 20,000 frames"
        );

        let mut droplets = Vec::new();
        let mut fill_after: u64 = 0;
        for y in 0..128 {
            for x in 0..128 {
                if w.get(x, y).material != material::WATER {
                    continue;
                }
                fill_after += liquid_fill(w.get(x, y)) as u64;
                let isolated = [(-1, 0), (1, 0), (0, -1), (0, 1)]
                    .iter()
                    .all(|(dx, dy)| w.get(x + dx, y + dy).material != material::WATER);
                if isolated {
                    droplets.push((x, y));
                }
            }
        }
        assert!(
            droplets.is_empty(),
            "settled world (frame {frames_run}) left {} stray isolated water cell(s): {:?}",
            droplets.len(),
            droplets
        );
        assert_eq!(
            fill_before, fill_after,
            "total liquid fill drifted across the splash and settle"
        );
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
    fn a_settled_grain_does_not_creep_across_a_gap_only_its_flowing_reach_can_see() {
        // `Reports/granular-mechanics-research.md` §2's central claim: harder
        // to *start* creeping than to *keep* creeping. Search for a position
        // where sand's two reach thresholds actually diverge at a distance
        // of 2+ -- not hardcoded, so this stays correct if the jitter
        // function or sand's tuning ever changes. Distance 1 is deliberately
        // excluded: a reach-1 opening is already caught by the plain
        // diagonal fall before `roll_along_slope` is ever consulted (see
        // `roll_reach_follows_the_angle_of_repose`'s own note on this), so a
        // gap only visible at distance 1 would exercise the wrong code path.
        let reg = material::MaterialRegistry::builtin();
        let sand = reg.get(material::SAND);
        let x = (20..100)
            .find(|&x| sand.roll_reach_at(x, 0) >= 2 && sand.stability_reach_at(x, 0) < 2)
            .expect("sand should have a position where the two reach thresholds diverge at distance 2");
        let y = 0;

        // Floor everywhere under the grain and one column short of the real
        // opening, both directions -- blocks direct fall, both diagonal
        // falls, and any nearer roll target, so only the reach search
        // decides whether it moves at all, and only rightward.
        let build = |flowing: bool| {
            let mut w = World::new(Rect::new(0, 0, 127, 127));
            for fx in (x - 15)..=(x + 1) {
                w.set(fx, y + 1, Cell::new(material::STONE, 0));
            }
            // The real opening: two columns right, row below empty.
            let grain = Cell::new(material::SAND, 0).with_flowing(flowing);
            w.set(x, y, grain);
            w
        };

        let mut settled = build(false);
        step(&mut settled);
        assert_eq!(
            settled.get(x, y).material,
            material::SAND,
            "a settled grain should not creep toward a gap only the lenient reach can see"
        );

        let mut flowing = build(true);
        step(&mut flowing);
        assert!(
            flowing.get(x, y).is_empty() && flowing.get(x + 1, y).material == material::SAND,
            "a flowing grain should use the lenient reach and start toward the opening"
        );
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
    ///
    /// Two-angle-aware (`Reports/granular-mechanics-research.md` §2): a
    /// settled cell (`!cell.flowing()`) is judged against the stricter
    /// `stability_reach_at`, not `roll_reach_at` — checking every cell
    /// against the lenient repose reach would flag a cell legitimately at
    /// rest on a slope between the two angles as "stuck," which is the
    /// intended behaviour now, not a bug. This mirrors `roll_along_slope`'s
    /// own choice of reach exactly, so this helper is asserting the same
    /// invariant the production code implements, not a different one.
    fn unstable_sand(w: &World) -> Vec<(i32, i32)> {
        let b = w.bounds().unwrap();
        let mut out = Vec::new();
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let cell = w.get(x, y);
                if cell.material != material::SAND {
                    continue;
                }
                let can_fall = [0, -1, 1]
                    .iter()
                    .any(|dx| w.in_bounds(x + dx, y + 1) && w.is_empty(x + dx, y + 1));
                let mat = w.materials.get(material::SAND);
                let reach = if cell.flowing() {
                    mat.roll_reach_at(x, y)
                } else {
                    mat.stability_reach_at(x, y)
                };
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
        // 3000 was enough at `MIN_LIQUID_TRANSFER = 150`. Dropping the dead
        // band to 16 for B-9 (see that constant's own doc) makes this exact
        // scene settle *flatter* (`height_spread` reaches exactly 0.0 here,
        // where 150 left visible residual unevenness) but slower -- measured
        // directly by checkpointing this scene at 1000-frame intervals:
        // still 6 chunks active at 3000, 4 at 4000, fully asleep by 5000.
        // 6000 here for real margin, not the bare minimum.
        //
        // This is *not* a gap automatic body promotion would close, despite
        // the design doc's own "step 6 is the verdict on steps 3-4"
        // framing suggesting exactly that. Tried directly, then reverted:
        // wiring `World::try_promote_one` and letting it promote this
        // scene's flat rectangular block early made it *worse*, not
        // better -- the block promotes almost immediately (already
        // internally level, so it satisfies quiescence on its first real
        // solver pass) and then sleeps, permanently frozen at its
        // promotion-time width (measured: spread stuck at exactly 106
        // columns from frame 10 through frame 6000, versus this test's own
        // >150 bar). The persistent-flux solver only redistributes mass
        // among a body's *already-claimed* columns; `overloaded_edge`
        // (the only mechanism that sheds a column back to CA, where it
        // could then spread into open floor on its own) triggers on
        // relative imbalance *among a body's own columns*
        // (`h[edge] > avg * EDGE_OVERFLOW_RATIO`), which is structurally
        // impossible to satisfy for a body that is already flat -- so nothing
        // ever drives a uniformly-full body to notice it is taller than a
        // real fluid would settle to given the open floor beside it. A real
        // architectural gap, not a tuning problem; recorded here rather than
        // in a promotion-trigger implementation, since none is safe to wire
        // in until this is resolved. See `PLAN.md`'s note on the same
        // finding for the fuller writeup.
        run(&mut w, 6000);

        assert_eq!(w.active_chunk_count(), 0, "water never fully settled");

        let spread = (0..200).filter(|&x| (100..149).any(|y| w.get(x, y).material == material::WATER)).count();
        assert!(spread > 150, "column did not level out: final spread only {spread} columns wide");

        let height_spread = column_height_spread(&w, material::WATER, 149);
        assert!(height_spread < 10.0, "surface is not close to flat: height std-dev {height_spread:.2}");
    }

    #[test]
    fn a_wide_shallow_pool_levels_within_budget() {
        // B-9 (`Reports/liquid-heightfield-design.md` §12) / Report B
        // (`liquid-simulation-research-r2.md`) §10b-10c: a 100-cell-wide
        // pool must settle to within 2% of `LIQUID_FULL` adjacent-column
        // difference within 300 frames, and `MIN_LIQUID_TRANSFER` -- the
        // dead band that used to be the only thing bounding settling time
        // -- must actually be at or below 16 to get there, not just claim
        // to. Checked directly against the live constant so this test
        // fails the moment it creeps back up rather than only when someone
        // remembers to re-derive the bound.
        const { assert!(MIN_LIQUID_TRANSFER <= 16, "B-9: MIN_LIQUID_TRANSFER must drop to <= 16") };

        // A single shallow row, not a tall column: at this depth, fill
        // unevenness *within* the row is exactly "adjacent-column height
        // difference" -- the same quantity 10b measures, without needing
        // multiple layers to express it. Deliberately uneven (a low
        // plateau with one spike) so leveling has real work to do.
        let width: i32 = 100;
        let x0: i32 = 10;
        let floor_y: i32 = 126;
        let mut w = World::new(Rect::new(0, 0, x0 + width + 10, floor_y + 1));
        // Inclusive of the world's last column. The exclusive range this
        // used left exactly one unfloored column at `max_x`, which was
        // invisible while liquid could only look 8 cells sideways and
        // became a drain the moment `find_lateral_descent` could see 16 --
        // the pool emptied off the edge instead of levelling, and the test
        // read it as a levelling failure. A scene meant to measure
        // levelling must not have a hole in it.
        for x in 0..=x0 + width + 10 {
            w.set(x, floor_y, Cell::new(material::STONE, 0));
        }
        for i in 0..width {
            let fill = if i == width / 2 { 900 } else { 400 };
            w.set(x0 + i, floor_y - 1, Cell::new(material::WATER, 0).with_aux(fill));
        }

        run(&mut w, 300);

        let fills: Vec<i32> = (0..width).map(|i| liquid_fill(w.get(x0 + i, floor_y - 1)) as i32).collect();
        let max_diff = fills.windows(2).map(|p| (p[0] - p[1]).abs()).max().unwrap();
        assert!(
            max_diff <= 20,
            "B-9/10b: surface not flat within budget -- max adjacent-column difference {max_diff} (bar is 2% of LIQUID_FULL = 20)"
        );
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


#[cfg(test)]
mod pour_slope {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::parallel;

    /// Height of the water surface in each column, 0 where there is none.
    fn surface_heights(w: &World, width: i32, floor_y: i32) -> Vec<i32> {
        (0..width)
            .map(|x| (0..floor_y).find(|&y| w.get(x, y).material == material::WATER).map_or(0, |y| floor_y - y))
            .collect()
    }

    /// Steepest surface slope in degrees over any `HORIZONTAL_TRANSFER_REACH`
    /// window -- the angle a player actually sees, which is the quantity the
    /// bug report was about ("water makes big piles").
    fn max_slope_degrees(heights: &[i32]) -> f64 {
        let run = HORIZONTAL_TRANSFER_REACH as usize;
        (0..heights.len().saturating_sub(run))
            .map(|i| ((heights[i] - heights[i + run]).abs() as f64 / run as f64).atan().to_degrees())
            .fold(0.0f64, f64::max)
    }

    /// `Reports/liquid-simulation-research-r2.md` §10f, which had never been
    /// tested: poured water must not hold a pile. This is the acceptance bar
    /// for the reported symptom, and it is deliberately run under **both**
    /// drivers -- every previous liquid test ran only the serial sweep, while
    /// `App::update` runs the parallel one, so the behaviour players actually
    /// saw was the one behaviour never covered.
    fn pour_levels_without_holding_a_slope(parallel_driver: bool) {
        let (width, floor_y) = (256, 191);
        let mut w = World::new(Rect::new(0, 0, width - 1, floor_y));
        for x in 0..width {
            w.set(x, floor_y, Cell::new(material::STONE, 0));
        }
        // A tall narrow column spanning several 64-cell chunks -- the live
        // scene, not a single-row synthetic one. 91 cells of head to shed.
        for x in 110..146 {
            for y in 100..floor_y {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        let step_once = |w: &mut World| if parallel_driver { parallel::step(w) } else { step(w) };

        // §10f asks for 60 frames. 150 here, because this column is 91 cells
        // tall and roughly half that budget is gravity simply bringing the
        // head down -- no lateral rule can beat free-fall. Measured: 7.1 deg
        // at frame 120. The bar that matters is that it stops being a pile,
        // and this is ~13x tighter than the sand-like ~45 deg it held before
        // `find_lateral_descent` existed.
        for _ in 0..150 {
            step_once(&mut w);
        }
        let slope = max_slope_degrees(&surface_heights(&w, width, floor_y));
        assert!(slope <= 10.0, "poured water still holds a {slope:.1} deg pile after 150 frames (bar: 10 deg)");

        // And it must reach *exactly* flat and then genuinely stop -- not
        // merely "close enough". A residual slope was the old failure mode
        // (a dead band wider than the pressure signal it filtered), and
        // never reaching a fixed point was the other (chunks awake forever).
        // 1650 more (1800 total). Measured: the surface is already
        // perfectly level by frame 900 under both drivers, but the serial
        // sweep takes until ~1500 to drop its last two chunks to settled,
        // where the parallel one is done sooner. Budgeted past the slower
        // of the two with margin, rather than to whichever number happened
        // to pass -- a settling test that sits on its own boundary is how
        // the moss test in `plant.rs` became a coin flip.
        for _ in 0..1650 {
            step_once(&mut w);
        }
        let heights = surface_heights(&w, width, floor_y);
        let wet: Vec<i32> = heights.iter().copied().filter(|&h| h > 0).collect();
        let spread = (heights[0] - heights[heights.len() - 1]).abs();
        assert!(wet.len() as i32 > width * 3 / 4, "water never spread across the floor: only {} columns wet", wet.len());
        assert_eq!(
            wet.iter().max().unwrap() - wet.iter().min().unwrap(),
            0,
            "settled water is not perfectly level (edge-to-edge difference {spread})"
        );
        assert_eq!(w.active_chunk_count(), 0, "water never reached a fixed point -- chunks still awake");
    }

    #[test]
    fn a_poured_column_levels_without_holding_a_slope_serial() {
        pour_levels_without_holding_a_slope(false);
    }

    #[test]
    fn a_poured_column_levels_without_holding_a_slope_parallel() {
        pour_levels_without_holding_a_slope(true);
    }
}






#[cfg(test)]
mod seam_cliffs {
    use super::*;
    use crate::sim::chunk::{Rect, CHUNK_SIZE};
    use crate::sim::parallel;

    const WIDTH: i32 = 640;
    const FLOOR_Y: i32 = 382;

    /// The reported scene: blobs dropped from height onto a floor, wide
    /// enough to spread across several vertical chunk boundaries.
    ///
    /// **Dropped circles, not a contiguous block** — deliberately, and two
    /// earlier reproductions that got this wrong are recorded here so they
    /// aren't retried. Measuring the *spreading front* (leftmost/rightmost
    /// extent) shows nothing at all: the front crosses seams smoothly, and
    /// the vertical face persists *behind* it. And a single contiguous
    /// block does produce tall cliffs, but none of them land on seams — the
    /// falling-blob impact is what puts a free face at a seam in the first
    /// place.
    fn dropped_blobs() -> World {
        let mut w = World::new(Rect::new(0, 0, WIDTH - 1, FLOOR_Y));
        for x in 0..WIDTH {
            w.set(x, FLOOR_Y, Cell::new(material::STONE, 0));
        }
        w.paint_circle(150, 90, 46, material::SAND);
        w.paint_circle(300, 90, 46, material::SAND);
        w.paint_circle(450, 90, 28, material::SAND);
        w
    }

    /// Height of the sand surface in each column, 0 where there is none.
    fn surface_heights(w: &World) -> Vec<i32> {
        (0..WIDTH)
            .map(|x| (0..FLOOR_Y).find(|&y| w.get(x, y).material == material::SAND).map_or(0, |y| FLOOR_Y - y))
            .collect()
    }

    /// Every adjacent-column height jump of at least `min`, as
    /// `(left column, signed jump)`. This is the quantity that was actually
    /// being complained about — a face the player sees, not a slope average
    /// and not the position of the spreading front.
    fn cliffs(heights: &[i32], min: i32) -> Vec<(i32, i32)> {
        (0..heights.len() - 1)
            .filter_map(|i| {
                let jump = heights[i + 1] - heights[i];
                (jump.abs() >= min).then_some((i as i32, jump))
            })
            .collect()
    }

    /// Whether the pair of columns `(x, x + 1)` sits within one cell of a
    /// vertical chunk boundary.
    fn touches_a_seam(x: i32) -> bool {
        (x - 1..=x + 2).any(|c| c.rem_euclid(CHUNK_SIZE) == 0)
    }

    fn run_to(w: &mut World, frames: usize, driver: fn(&mut World)) {
        for _ in 0..frames {
            driver(w);
        }
    }

    /// A pile must not hold a vertical face on a chunk boundary.
    ///
    /// Reported from live play with the F1 chunk-grid overlay on: sand blobs
    /// dropped on the ground spread until the front met a vertical chunk
    /// boundary, then stopped dead there, holding a sharp vertical face that
    /// lined up exactly with the gridline and took ~25 seconds to relax.
    ///
    /// The failure is specifically a *seam* failure, and the frame matters.
    /// At frame 150 there are ~12 cliffs scattered anywhere — the blobs are
    /// still landing, and those are real transient faces that relax on their
    /// own. By frame 400 the only ones left, before the fix, were the
    /// seam-aligned ones: 3 cliffs, all 3 within a cell of a boundary, at
    /// x=127/193/255 and 17/38/36 cells tall. So this checks frame 400, when
    /// everything that relaxes normally already has.
    ///
    /// Cause, measured rather than guessed (three hypotheses were wrong
    /// first, including the recorded leading one — seam cells *do* get
    /// `flowing()` set): both drivers sweep chunk by chunk, so every cell in
    /// a chunk is updated before any cell in the chunk to its right. A free
    /// face landing on a boundary therefore became a one-column conveyor —
    /// the seam column shed exactly one grain per frame off its bottom while
    /// the whole column slumped down one to refill, measured at 33
    /// straight-down slumps against 0.9 sideways escapes per frame, where a
    /// single-region sweep of the same state gave 9. `FLAG_UNDERCUT`
    /// (`cell.rs`) is what stops the slump outrunning the escape.
    ///
    /// Run under both drivers: the bug appeared identically in each, which
    /// is what ruled out the parallel checkerboard early.
    fn a_pile_does_not_hold_a_face_on_a_chunk_boundary(driver: fn(&mut World)) {
        let mut w = dropped_blobs();
        run_to(&mut w, 400, driver);
        let heights = surface_heights(&w);

        let found = cliffs(&heights, 6);
        let on_seams: Vec<(i32, i32)> = found.iter().copied().filter(|(x, _)| touches_a_seam(*x)).collect();
        assert!(
            on_seams.is_empty(),
            "sand is holding a vertical face on a chunk boundary at frame 400: {on_seams:?} \
             (chunks are {CHUNK_SIZE} wide; all cliffs found: {found:?})"
        );

        // And the face must not merely have shuffled a column or two off the
        // boundary. Measured after the fix: worst adjacent-column jump is 3
        // (serial) and 4 (parallel) at this frame, against 38 before it, so
        // this bar has roughly 2x headroom below and 5x above.
        let worst = (0..heights.len() - 1).map(|i| (heights[i + 1] - heights[i]).abs()).max().unwrap_or(0);
        assert!(worst <= 8, "sand is still holding a {worst}-cell face somewhere at frame 400");
    }

    #[test]
    fn a_pile_does_not_hold_a_face_on_a_chunk_boundary_serial() {
        a_pile_does_not_hold_a_face_on_a_chunk_boundary(step);
    }

    #[test]
    fn a_pile_does_not_hold_a_face_on_a_chunk_boundary_parallel() {
        a_pile_does_not_hold_a_face_on_a_chunk_boundary(parallel::step);
    }

    /// A ledge with a drop off its right-hand end, so the grain resting on
    /// the last floor cell has nowhere to go but sideways.
    fn ledge_with_a_drop() -> World {
        let mut w = World::new(Rect::new(0, 0, 19, 11));
        for x in 0..=5 {
            w.set(x, 11, Cell::new(material::STONE, 0));
        }
        w
    }

    /// `FLAG_UNDERCUT`'s lifecycle, tested directly rather than only through
    /// the 400-frame scene above — this is the part that would break
    /// silently if the flag were ever folded into `FLAG_MOVED`'s clearing
    /// path, which consumes it only on the visits it *skips*.
    #[test]
    fn a_sideways_escape_flags_the_hole_it_leaves_for_exactly_one_frame() {
        let mut w = ledge_with_a_drop();
        w.set(5, 10, Cell::new(material::SAND, 0));
        step(&mut w);

        assert_eq!(w.get(6, 11).material, material::SAND, "the grain should have gone off the end of the ledge");
        assert!(w.get(5, 10).undercut(), "the hole a sideways escape leaves must be flagged");

        step(&mut w);
        assert!(!w.get(5, 10).undercut(), "the flag must be consumed by the sweep's next visit, not persist");
    }

    /// The other half of the same rule, and the reason it cannot simply be
    /// "any hole a move leaves": a column falling through air descends as a
    /// unit precisely because every cell above drops into the vacancy below
    /// it in the same bottom-to-top sweep. Flagging that hole too would make
    /// a falling column stretch out one cell per frame.
    #[test]
    fn a_straight_fall_leaves_a_hole_anything_may_drop_into() {
        let mut w = ledge_with_a_drop();
        for y in 6..=9 {
            w.set(3, y, Cell::new(material::SAND, 0));
        }
        step(&mut w);

        assert!(!w.get(3, 6).undercut(), "a straight-down move must not flag the cell it vacated");
        // The whole column moved down one, not just its bottom grain.
        assert_eq!((7..=10).filter(|&y| w.get(3, y).material == material::SAND).count(), 4);
    }

    /// The control that found the cause, kept as a test so it stays true:
    /// cutting the sweep into chunks must not change the outcome on this
    /// scene. Before the fix this failed loudly — the chunked drivers held 3
    /// seam cliffs at frame 400 where `step_monolithic` held none, on
    /// byte-identical starting state — which is what moved the investigation
    /// off the movement rules and onto the sweep decomposition.
    #[test]
    fn chunking_the_sweep_does_not_change_where_a_pile_settles() {
        let mut chunked = dropped_blobs();
        let mut whole = dropped_blobs();
        run_to(&mut chunked, 400, step);
        run_to(&mut whole, 400, step_monolithic);

        let worst = |w: &World| {
            let h = surface_heights(w);
            (0..h.len() - 1).map(|i| (h[i + 1] - h[i]).abs()).max().unwrap_or(0)
        };
        let (a, b) = (worst(&chunked), worst(&whole));
        assert!(
            (a - b).abs() <= 6,
            "the chunked sweep settles differently from a single-region sweep: \
             worst face {a} chunked vs {b} whole-world"
        );
    }
}

#[cfg(test)]
mod seam_terracing {
    use super::*;
    use crate::sim::chunk::{Rect, CHUNK_SIZE};
    use crate::sim::parallel;

    const WIDTH: i32 = 512;
    const HEIGHT: i32 = 320;
    const FLOOR_Y: i32 = HEIGHT - 8;

    /// The reported scene at the sandbox's own resolution: a large body of
    /// water released against the left wall, spreading right across seven
    /// vertical chunk seams.
    fn released_body() -> World {
        let mut w = World::new(Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
        for x in 0..WIDTH {
            for y in FLOOR_Y..HEIGHT {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 0..200 {
            for y in 30..FLOOR_Y {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        w
    }

    /// Water in each column measured as **volume**, not as the height of its
    /// topmost cell.
    ///
    /// This distinction is the whole reason the first three reproductions of
    /// this bug failed. A `Liquid` cell holds a continuous fill (`Cell::aux`)
    /// and the terracing comes fringed with one-cell-tall, near-empty films
    /// — the "whiskers" in the bug report. Those films are barely visible on
    /// screen (`render.rs` dims a liquid toward black by its fill) but a
    /// topmost-cell metric counts them at full height, which smooths over
    /// precisely the risers being complained about: measured that way the
    /// seams looked at worst 1.7x the interior, against 9.0x by volume.
    fn column_volumes(w: &World) -> Vec<i32> {
        (0..WIDTH)
            .map(|x| {
                (0..FLOOR_Y)
                    .map(|y| {
                        let c = w.get(x, y);
                        if c.material == material::WATER {
                            liquid_fill(c) as i32
                        } else {
                            0
                        }
                    })
                    .sum::<i32>()
                    / material::LIQUID_FULL as i32
            })
            .collect()
    }

    /// Mean absolute column-to-column step exactly across a chunk boundary,
    /// against the mean everywhere else. Seams being *special* is the claim
    /// under test, so this compares them against the surface's own natural
    /// roughness rather than against a fixed threshold — a slope is allowed
    /// to be steep, it just may not be steep only on the gridlines.
    fn seam_step_ratio(volumes: &[i32]) -> f64 {
        let (mut seam, mut seam_n, mut interior, mut interior_n) = (0.0, 0usize, 0.0, 0usize);
        for i in 0..volumes.len() - 1 {
            if volumes[i] == 0 || volumes[i + 1] == 0 {
                continue;
            }
            let step = (volumes[i + 1] - volumes[i]).abs() as f64;
            if (i as i32 + 1).rem_euclid(CHUNK_SIZE) == 0 {
                seam += step;
                seam_n += 1;
            } else {
                interior += step;
                interior_n += 1;
            }
        }
        (seam / seam_n.max(1) as f64) / (interior / interior_n.max(1) as f64).max(0.001)
    }

    /// Water must not hold a terrace on a chunk boundary.
    ///
    /// The liquid twin of `seam_cliffs` above, reported from live play in
    /// the same way and with the same signature: a spreading body held flat
    /// plateaus with sharp risers landing exactly on the F1 overlay's
    /// gridlines. Same cause — both drivers sweep chunk by chunk, so a
    /// seam column sheds one cell per frame off its bottom while the column
    /// above slumps in to refill it — and the same fix, `FLAG_UNDERCUT`,
    /// which had been gated to `Powder` movers and so did nothing here.
    ///
    /// The bar is a ratio, and it is checked *late*: the failure mode is
    /// that the dam grows without bound rather than that it exists at all.
    /// Measured before the fix: 3.5x at frame 600 climbing to 9.0x at 1600.
    /// After: 1.4x at 600 and 1.7x at 1600, no longer climbing.
    fn water_does_not_terrace_on_a_chunk_boundary(driver: fn(&mut World)) {
        let mut w = released_body();
        for _ in 0..1600 {
            driver(&mut w);
        }
        let ratio = seam_step_ratio(&column_volumes(&w));
        assert!(
            ratio <= 4.0,
            "water is damming on chunk boundaries: the mean column step across a seam is {ratio:.1}x \
             the step everywhere else (bar: 4x; before the fix this reached 9x and was still climbing)"
        );
    }

    #[test]
    fn water_does_not_terrace_on_a_chunk_boundary_serial() {
        water_does_not_terrace_on_a_chunk_boundary(step);
    }

    #[test]
    fn water_does_not_terrace_on_a_chunk_boundary_parallel() {
        water_does_not_terrace_on_a_chunk_boundary(parallel::step);
    }
}

#[cfg(test)]
mod displacement {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::parallel;

    /// Displaced material must rise at most one cell per frame.
    ///
    /// Reported from live play: dropping a sand blob into water made the
    /// water appear on top of the blob almost immediately and spray out of
    /// it. Rows are swept bottom to top, so as the sweep worked upward each
    /// successive sand cell displaced the *same* water parcel again, and it
    /// crossed the whole height of the sand in a single frame -- measured
    /// here at 50 rows in one step before the fix.
    fn displaced_water_rises_at_most_one_cell_per_frame(parallel_driver: bool) {
        let floor_y = 220;
        let mut w = World::new(Rect::new(0, 0, 63, floor_y));
        for x in 0..64 {
            w.set(x, floor_y, Cell::new(material::STONE, 0));
        }
        // Walled, so displaced water cannot escape sideways -- this isolates
        // the vertical displacement path from `find_lateral_descent`, which
        // otherwise carries the surface water off before the displacement
        // ever fires (an earlier version of this test measured identically
        // with and without the fix for exactly that reason).
        for y in 0..floor_y {
            w.set(10, y, Cell::new(material::STONE, 0));
            w.set(53, y, Cell::new(material::STONE, 0));
        }
        for x in 11..53 {
            for y in 150..floor_y {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        for x in 11..53 {
            for y in 100..150 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }

        let highest_water = |w: &World| (0..floor_y).find(|&y| (11..53).any(|x| w.get(x, y).material == material::WATER));
        let mut previous = highest_water(&w).expect("test setup: water exists");
        for frame in 1..=6 {
            if parallel_driver {
                parallel::step(&mut w);
            } else {
                step(&mut w);
            }
            let now = highest_water(&w).expect("water vanished");
            assert!(
                previous - now <= 1,
                "water rose {} rows in frame {frame} (from {previous} to {now}); displaced material must move at most one cell per frame",
                previous - now
            );
            previous = now;
        }
    }

    #[test]
    fn displaced_water_rises_at_most_one_cell_per_frame_serial() {
        displaced_water_rises_at_most_one_cell_per_frame(false);
    }

    #[test]
    fn displaced_water_rises_at_most_one_cell_per_frame_parallel() {
        displaced_water_rises_at_most_one_cell_per_frame(true);
    }
}


/// The liquid acceptance criteria from `Reports/liquid-simulation-research-r2.md`
/// §10, which that report opens by observing the engine had no way to tell
/// whether a liquid change helped: its water tests all checked qualitative
/// properties ("levels out", "settles flatter than a powder would"), so a
/// mass leak or a permanently uneven surface would pass every one of them.
///
/// That gap is not hypothetical. Three separate times while fixing the
/// chunk-seam bugs, an ad-hoc metric said "no problem here" on a scene that
/// visibly had one — surface height hid a 9x seam effect that column volume
/// showed plainly, and occupancy hid torn seam rows that were a fill deficit
/// rather than a hole. And a fix that cleared one artifact while introducing
/// a larger one shipped and had to be reverted (`e816477`), because the test
/// guarding it only looked at rows lying exactly on a chunk seam.
///
/// **Bars are set from measurement with headroom, not from the report's
/// aspirations**, except where the two already agree. Where they differ the
/// report's number is recorded alongside, so the gap stays visible instead of
/// being quietly redefined away. A test sitting on its own boundary is how
/// the moss test in `plant.rs` became a coin flip.
#[cfg(test)]
mod liquid_acceptance {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::parallel;

    fn stone_floor(w: &mut World, width: i32, height: i32, thickness: i32) {
        for x in 0..width {
            for y in (height - thickness)..height {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
    }

    fn total_volume(w: &World) -> u64 {
        let b = w.bounds().unwrap();
        (b.min_y..=b.max_y)
            .flat_map(|y| (b.min_x..=b.max_x).map(move |x| (x, y)))
            .map(|(x, y)| {
                let c = w.get(x, y);
                if c.material == material::WATER {
                    liquid_fill(c) as u64
                } else {
                    0
                }
            })
            .sum()
    }

    fn column_volume(w: &World, x: i32, floor_y: i32) -> i32 {
        (0..floor_y)
            .map(|y| {
                let c = w.get(x, y);
                if c.material == material::WATER {
                    liquid_fill(c) as i32
                } else {
                    0
                }
            })
            .sum()
    }

    /// §10a. Total fill must be conserved across a dam break.
    ///
    /// The report's reference point for what a bad implementation looks like
    /// is the VOF literature's: the standard method gained 2% where the
    /// height-function version was exact. Bar 0.5%, measured **0.0000%**
    /// under both drivers — this passes with enormous margin today and is
    /// landed anyway, because it catches a whole class of bug by
    /// construction rather than by anyone thinking to look. An independent
    /// review found exactly that class open in `liquid.rs`:
    /// `World::absorb_liquid` drops the fill on a bounds miss *after*
    /// `transfer_liquid_vertical` has already emptied the source. That path
    /// needs a promoted body, which nothing in production creates yet, so
    /// this scene cannot reach it — but the moment automatic promotion
    /// lands, this is the test that fires.
    fn mass_is_conserved_across_a_dam_break(driver: fn(&mut World)) {
        let (width, height) = (256, 192);
        let floor_y = height - 8;
        let mut w = World::new(Rect::new(0, 0, width - 1, height - 1));
        stone_floor(&mut w, width, height, 8);
        for x in 10..80 {
            for y in 20..floor_y {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }

        let before = total_volume(&w);
        assert!(before > 0, "test setup: no water");
        let mut worst = 0.0f64;
        for _ in 0..2000 {
            driver(&mut w);
            let drift = (total_volume(&w) as f64 - before as f64).abs() / before as f64 * 100.0;
            worst = worst.max(drift);
        }
        assert!(worst <= 0.5, "liquid volume drifted by {worst:.4}% across a dam break (bar 0.5%, measured 0.0000%)");
    }

    #[test]
    fn mass_is_conserved_across_a_dam_break_serial() {
        mass_is_conserved_across_a_dam_break(step);
    }

    #[test]
    fn mass_is_conserved_across_a_dam_break_parallel() {
        mass_is_conserved_across_a_dam_break(parallel::step);
    }

    /// §10b and §10c together, plus the sleep bar the report does not state
    /// but which matters more than either: a pool that has visually levelled
    /// while its chunks stay awake is still costing the whole frame budget,
    /// and chunk sleeping is this engine's entire cost model.
    ///
    /// Measured on a 100-cell basin: flatness 16 against the report's bar of
    /// 20 (2% of `LIQUID_FULL`), reached at frame 331 against the report's
    /// 300, fully asleep well inside 6000. The report predicted 10b would
    /// "fail today by construction" because `MIN_LIQUID_TRANSFER` was 150 —
    /// it is 16 now, so it passes.
    ///
    /// The levelling-time bar is 800, not the report's 300: 331 is the
    /// measured value and a bar has to sit clear of it. The gap is recorded
    /// rather than closed, because closing it is a real piece of work
    /// (levelling is O(width²)) and not something to hide by relabelling.
    #[test]
    fn a_hundred_cell_basin_levels_flat_and_then_sleeps() {
        let (width, height) = (128, 96);
        let floor_y = height - 8;
        let mut w = World::new(Rect::new(0, 0, width - 1, height - 1));
        stone_floor(&mut w, width, height, 8);
        for x in 0..14 {
            for y in 0..height {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 114..width {
            for y in 0..height {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        // Heaped on one side, so it has the full basin to level across.
        for x in 14..54 {
            for y in 30..floor_y {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }

        let flatness = |w: &World| {
            let v: Vec<i32> = (14..114).map(|x| column_volume(w, x, floor_y)).collect();
            (0..v.len() - 1).map(|i| (v[i + 1] - v[i]).abs()).max().unwrap_or(0)
        };
        let bar = material::LIQUID_FULL as i32 * 2 / 100;

        let mut levelled_at = None;
        for frame in 1..=6000 {
            parallel::step(&mut w);
            if levelled_at.is_none() && flatness(&w) <= bar {
                levelled_at = Some(frame);
            }
        }

        let final_flatness = flatness(&w);
        assert!(
            final_flatness <= bar,
            "settled pool is not flat: max adjacent-column difference {final_flatness} \
             (bar {bar} = 2% of LIQUID_FULL, measured 16)"
        );
        let levelled_at = levelled_at.expect("the basin never reached the flatness bar at all");
        assert!(
            levelled_at <= 800,
            "the basin took {levelled_at} frames to level \
             (bar 800, measured 331; the report asks for 300 and that gap is open)"
        );
        assert_eq!(w.active_chunk_count(), 0, "the basin levelled but its chunks never went to sleep");
    }

    /// §10d. A pool drained through a narrow opening must not leave material
    /// stranded above the drain.
    ///
    /// Bar 90, measured **54**, against the report's aspiration of zero.
    /// This is the one criterion the engine is clearly short on, and the gap
    /// is deliberately left visible rather than tuned away. Note the report
    /// attributes this symptom to VOF flotsam and jetsam — droplets orphaned
    /// by piecewise-constant interface reconstruction — and that attribution
    /// is doubtful: this scene strands 54 cells while producing **zero**
    /// one-cell-tall film cells, and the films seen elsewhere are mostly
    /// *full* cells thrown sideways by `find_lateral_descent` rather than
    /// partial-fill remnants. Measure which mechanism is producing the cells
    /// before adopting the report's three-cell height-function fix.
    #[test]
    fn a_drained_basin_does_not_strand_water_above_the_drain() {
        let (width, height) = (192, 160);
        let mut w = World::new(Rect::new(0, 0, width - 1, height - 1));
        for x in 0..width {
            w.set(x, height - 1, Cell::new(material::STONE, 0));
        }
        for x in 40..152 {
            for dy in 0..3 {
                w.set(x, 100 + dy, Cell::new(material::STONE, 0));
            }
        }
        // A three-cell hole in the basin floor.
        for x in 94..97 {
            for dy in 0..3 {
                w.set(x, 100 + dy, Cell::EMPTY);
            }
        }
        for y in 0..100 {
            for dx in 0..3 {
                w.set(40 + dx, y, Cell::new(material::STONE, 0));
                w.set(148 + dx, y, Cell::new(material::STONE, 0));
            }
        }
        for x in 43..148 {
            for y in 60..100 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }

        for _ in 0..2500 {
            parallel::step(&mut w);
        }

        let stranded = (43..148)
            .flat_map(|x| (60..100).map(move |y| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == material::WATER)
            .count();
        assert!(
            stranded <= 90,
            "{stranded} cells stranded in a basin that should have drained \
             (bar 90, measured 54; the report asks for 0)"
        );
    }

    /// The whisker bar: one-cell-tall sheets of water with open air both
    /// above and below, which draw as a comb of detached horizontal ledges
    /// along a spreading front. Reported from live play as "horizontal
    /// banding", and distinct from the row banding below — that one is a
    /// fill deficit inside the body, this one is the shape of its edge.
    ///
    /// Bar 400, measured **290** on a falling, spreading body. Open, and
    /// deliberately barred at roughly today's value rather than at zero:
    /// three candidate fixes have been measured and all three rejected, so
    /// this exists to stop it getting *worse* while the real fix is found,
    /// not to claim it is solved.
    ///
    /// What has been tried, so it is not tried again:
    ///
    /// - **Disable `find_lateral_descent`.** Removes 75% of them (2540 to
    ///   621 on a larger scene) and destroys the property that rule exists
    ///   for — without it water reads as sand, which is the original bug.
    /// - **Land the mover at `(tx, y)` and let it fall next frame** instead
    ///   of at `(tx, y + 1)`. Whiskers 2540 to 1635, but fully-enclosed
    ///   holes inside the body went 289 to 1040. Net worse.
    /// - **Shrink `LIQUID_LATERAL_REACH`.** A pure trade against levelling
    ///   speed, with diminishing returns and no path to zero: reach 24/12/6/3
    ///   gives whiskers 290/175/151/119 against levelling times of
    ///   343/557/1017/1661 frames. Halving reach costs 62% of the levelling
    ///   speed for 40% of the whiskers.
    ///
    /// What the measurements say about the cause, for whoever picks this up:
    /// `find_lateral_descent` is **not** teleporting water. 75% of its moves
    /// are a single-cell diagonal step and only 3% land with air two cells
    /// below. Whiskers survive at reach 3, so they are not primarily long
    /// jumps. They look instead like the surface monolayer advancing one
    /// diagonal step per frame with nothing above to refill the row it
    /// vacates — which may mean the honest fix is not in the movement rule
    /// at all, but in how a one-cell-thick sheet is drawn.
    #[test]
    fn a_spreading_front_does_not_shed_a_comb_of_detached_ledges() {
        const WIDTH: i32 = 512;
        const HEIGHT: i32 = 320;
        let mut w = World::new(Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
        stone_floor(&mut w, WIDTH, HEIGHT, 8);
        for x in 20..250 {
            for y in 20..200 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }

        let films = |w: &World| {
            (1..WIDTH - 1)
                .flat_map(|x| (1..HEIGHT - 1).map(move |y| (x, y)))
                .filter(|&(x, y)| {
                    w.get(x, y).material == material::WATER
                        && w.get(x, y - 1).is_empty()
                        && w.get(x, y + 1).is_empty()
                })
                .count()
        };

        let mut worst = 0;
        for _ in 0..400 {
            parallel::step(&mut w);
            worst = worst.max(films(&w));
        }
        assert!(
            worst <= 400,
            "a spreading front is shedding {worst} one-cell-tall detached ledges              (bar 400, measured 290; this bar holds the line, it does not claim the bug is fixed)"
        );
    }

    /// The bar that did not exist, and whose absence let a regression ship.
    ///
    /// `e816477` reverted a fix that cleared torn rows on horizontal chunk
    /// seams and, in live play, introduced a much larger striped-banding
    /// artifact. The test guarding that fix only ever looked at rows lying
    /// exactly on a seam, so it saw nothing. This looks at **every** row.
    ///
    /// Measured as a summed fill deficit rather than a count of bad rows: a
    /// count gave a knife-edge margin (2 against 4), where the sum separates
    /// cleanly — serial 443, parallel 988, and the reverted fix 2236. The
    /// serial driver is the control throughout: its chunk order is already
    /// bottom-up, so whatever banding it shows is inherent to the rules
    /// rather than to the decomposition, and the parallel driver may not be
    /// dramatically worse than it.
    #[test]
    fn the_parallel_driver_does_not_band_a_falling_body_worse_than_the_serial_one() {
        const WIDTH: i32 = 512;
        const HEIGHT: i32 = 320;
        let floor_y = HEIGHT - 8;

        let scene = || {
            let mut w = World::new(Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
            stone_floor(&mut w, WIDTH, HEIGHT, 8);
            for x in 20..250 {
                for y in 20..200 {
                    w.set(x, y, Cell::new(material::WATER, 0));
                }
            }
            w
        };

        // Mean fill of the occupied cells in a row, ignoring rows too sparse
        // to be part of the body at all.
        let row_fill = |w: &World, y: i32| -> i32 {
            let occupied: Vec<u32> = (1..WIDTH - 1)
                .map(|x| {
                    let c = w.get(x, y);
                    if c.material == material::WATER {
                        liquid_fill(c) as u32
                    } else {
                        0
                    }
                })
                .filter(|&f| f > 0)
                .collect();
            if occupied.len() < 30 {
                return -1;
            }
            (occupied.iter().sum::<u32>() / occupied.len() as u32) as i32
        };
        let banding = |w: &World| -> i32 {
            (2..floor_y - 2)
                .map(|y| {
                    let (above, here, below) = (row_fill(w, y - 1), row_fill(w, y), row_fill(w, y + 1));
                    if above < 0 || here < 0 || below < 0 {
                        return 0;
                    }
                    let neighbours = (above + below) / 2;
                    if neighbours > 300 && here < neighbours {
                        neighbours - here
                    } else {
                        0
                    }
                })
                .sum()
        };
        let worst_banding = |driver: fn(&mut World)| {
            let mut w = scene();
            let mut worst = 0;
            for _ in 0..400 {
                driver(&mut w);
                worst = worst.max(banding(&w));
            }
            worst
        };

        let serial = worst_banding(step);
        let parallel = worst_banding(parallel::step);
        assert!(
            parallel <= 1400,
            "the parallel driver is banding a falling body: summed row fill deficit {parallel} \
             (bar 1400; measured 988 here and 443 under the serial driver, and 2236 under the \
             reverted fix this bar exists to have caught)"
        );
        assert!(
            serial <= 700,
            "the serial driver is banding a falling body: summed row fill deficit {serial} \
             (bar 700, measured 443)"
        );
    }
}
