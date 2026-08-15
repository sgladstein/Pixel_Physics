//! M8: rigid bodies — deliberately started narrow.
//!
//! The plan's own words for this milestone: "the largest single
//! milestone — treat as its own project" and "the risk concentration;
//! deferred as far as it sensibly can be." Its full pipeline is
//! connected-component labeling → marching squares contour → Douglas-Peucker
//! simplification → `earcutr` triangulation → a `rapier2d` collider →
//! erase-step-re-rasterize every frame. That is real, separate engineering
//! at every stage, not one afternoon's work, and the plan's own execution-
//! order reasoning explicitly warns against rushing it: "the most exciting
//! item and the one most likely to consume months without a playable
//! result."
//!
//! This module covers the pipeline's first two stages: connected-component
//! labeling over the CA grid, and boundary/contour extraction from a
//! labeled component. Douglas-Peucker simplification, `earcutr`
//! triangulation, and the `rapier2d` collider/step/re-rasterize loop are
//! not yet built. Nothing here pulls in `rapier2d` or commits to any of
//! that later pipeline's design decisions — those are better made
//! deliberately than rushed at the tail of an unattended session. See
//! `PLAN.md`'s M8 section and progress log for what's built versus what
//! remains.
//!
//! # Why `trace_contours` isn't the classic marching-squares algorithm
//!
//! Classic marching squares walks a *continuous* scalar field sampled at
//! grid corners, and needs its 16-case lookup table (plus a tie-break rule
//! for the two ambiguous "saddle" cases) specifically because interpolated
//! corner values can disagree about which way a contour bends between them.
//! This engine's input is already a *binary* per-cell occupancy grid — a
//! cell is either in the component or it isn't, nothing to interpolate —
//! so the exact same problem (turn a filled region into its boundary
//! polygon) has a simpler, unambiguous solution: every unit square cell
//! contributes a directed boundary edge on each side that faces a
//! non-filled neighbour, oriented so the filled interior is always on the
//! edge's right (a clockwise walk around outer boundaries, since this
//! engine's `y` grows downward like screen space); interior edges between
//! two filled cells never appear at all, because *neither* side of that
//! edge sees a non-filled neighbour. Stitching those directed edges
//! head-to-tail by shared endpoints reconstructs every closed loop — outer
//! boundaries and hole boundaries alike, with hole boundaries falling out
//! already wound the opposite way, no special-casing needed. This is
//! marching squares' actual job for exactly the input this pipeline stage
//! ever has to handle, without carrying machinery (interpolation, the
//! saddle tie-break) built for a continuous-field case that can't occur
//! here.
//!
//! **Known limitation, not handled**: a component that touches itself at
//! exactly one shared *corner* (two cells diagonally adjacent, connected to
//! each other only through a longer 4-connected path elsewhere — a
//! "pinch point") produces a corner point with two outgoing boundary edges,
//! and the last one processed silently wins. `label_component`'s strict
//! 4-connectivity makes this rare in practice (most bodies a player would
//! actually cut free are simply-connected blobs), but the geometry is
//! genuinely wrong wherever it happens — the dropped edge reroutes that
//! lobe's walk into the *other*, already-closed lobe's cycle rather than
//! back to its own start, so the traced ring is not a valid simple polygon
//! there. An independent review found and this module now guards against a
//! worse failure mode than "wrong geometry": without the `visited`-membership
//! check in `trace_contours`' inner walk, that reroute cycles the sibling
//! lobe forever, since it never revisits the dropped lobe's own `start`
//! point — an unbounded hang, not a bounded wrong answer. The guard turns
//! that hang into a bounded, wrong, terminating ring (on par with the
//! classic algorithm's own saddle-case ambiguity, which is the severity
//! this was always meant to have); it does not make the ring correct.
//! Resolving that needs the same kind of tie-break rule marching squares
//! uses for saddles — not implemented here.

use std::collections::{HashMap, HashSet};

use super::cell::Cell;
use super::material::{self, MaterialId, MaterialKind};
use super::world::World;

const NEIGHBOURS_4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// Per-frame downward acceleration, matching `particle.rs`'s own `GRAVITY`.
/// Deliberately the same number: debris thrown by a blast and debris that
/// broke off a ceiling are the same event to a player, and two different
/// fall rates in one scene reads as a bug even when neither is wrong.
const GRAVITY: f32 = 0.15;

/// Terminal speed per axis, again matching `particle.rs`. Also what bounds
/// the substep count in `advance`, so a body can never be asked to test
/// hundreds of intermediate positions in one frame.
const MAX_SPEED_PER_AXIS: f32 = 6.0;

/// Speed retained by the axis that hit something. Not zero: a slab landing
/// on one corner should slide and settle rather than stopping dead the
/// instant any single cell makes contact.
const COLLISION_RETENTION: f32 = 0.3;

/// Consecutive frames a body may fail to move before it is re-rasterized
/// back into the grid. Not "velocity is small" — a body wedged against a
/// wall can hold a large downward velocity forever while going nowhere, and
/// keying on speed leaves it flying in place, invisible to the CA and
/// permanently costing a frame slot.
const STALL_FRAMES_BEFORE_SETTLING: u8 = 3;

/// Smallest failing region worth flying as a coherent body. Below this, the
/// per-cell `breaks_into` conversion looks the same and costs less — a
/// single cell "tumbling" is just a grain.
const MIN_BODY_CELLS: usize = 8;

/// Smallest failing region worth fracturing at all. Below this the region is
/// left to the caller's per-cell conversion, which looks the same and costs
/// less.
const MIN_FRACTURE_CELLS: usize = 6;

/// Largest a single body may be. Past this one piece stops reading as a
/// chunk broken off a wall and starts reading as the wall gliding intact,
/// and the per-substep fit test gets expensive.
///
/// This is a cap on a *fragment*, not on a collapse — it used to gate
/// whether a region was promoted at all, which meant a large failure
/// declined and dissolved to dust instead. See
/// `try_promote_failing_region`.
const MAX_BODY_CELLS: usize = 400;

/// Ceiling on a single collapse, in cells. Not a "too big to be a chunk"
/// test -- a region this size becomes many chunks -- just a bound on how
/// much work one failure may do in one frame.
const MAX_FRACTURE_CELLS: usize = 4000;

/// How far `displace` looks for somewhere to shove a loose cell the body is
/// moving into. Bounded and small on purpose — this is a shove, not a
/// placement solver.
const DISPLACE_SEARCH: i32 = 4;

/// Quarter-turns accumulated per unit of speed per frame. Tuned so a chunk
/// falling a few dozen cells turns once or twice rather than spinning like a
/// pinwheel — the point is for it to read as tumbling rock, not as debris in
/// a blender.
const SPIN_PER_SPEED: f32 = 0.012;

/// Pressure written into the field per unit of strike force.
const STRIKE_PRESSURE: f32 = 6.0;

/// Pressure a collapse writes, per square root of its cell count — so a
/// shelf coming down is felt more than a pebble breaking loose, without a
/// large collapse producing an explosion-sized shockwave.
const COLLAPSE_PRESSURE: f32 = 4.0;

/// How far past a blow's own radius its fractures run. Cracks reaching
/// further than the damage is what lets a player work a fissure across a
/// span rather than having to chew through it.
const CRACK_REACH: i32 = 3;

/// Fractures scored per blow. Few enough to read as distinct fissures
/// rather than a shatter pattern.
const CRACK_RAYS: u32 = 5;

/// Extra reach a ray gains from running into existing damage. Stress
/// concentrates at a crack tip, so repeated blows in one place drive a
/// fissure deeper instead of scribbling a new one each time.
const CRACK_TIP_BONUS: i32 = 2;

/// How many times a single ray may claim `CRACK_TIP_BONUS`. Bounds the walk
/// so working one spot forever cannot drive a fissure across the whole
/// world, while still leaving several blows' worth of room to grow.
const CRACK_TIP_MAX_STEPS: i32 = 6;

/// How much a blow of this radius shifts the fragment-size ladder. A tap
/// chips; a heavy swing calves slabs. Capped at 2 (a 128-cell ceiling)
/// because past that a "fragment" is the whole wall and stops reading as a
/// piece broken off it.
fn size_bias(radius: i32) -> u32 {
    match radius {
        0..=5 => 0,
        6..=11 => 1,
        _ => 2,
    }
}

/// 4-connected flood fill over `Solid` cells starting at `(seed_x, seed_y)`.
///
/// Returns `None` if the seed cell itself isn't `Solid` — there is no
/// component to label. Otherwise returns every `Solid` cell reachable from
/// the seed by a chain of 4-connected `Solid` neighbours (never diagonal —
/// see `diagonal_only_contact_does_not_connect_two_components` for why that
/// distinction is deliberate and tested, not incidental: two blobs touching
/// only at a corner are not one physical body).
///
/// `max_cells` bounds the search — flood-filling an entire connected floor
/// (the sandbox's own is one contiguous slab spanning the whole world
/// width) would otherwise walk the whole world for a single "cut this
/// clump free" request. Capped rather than unbounded, the same shape
/// `structural.rs`'s reactive-only scheduling uses to avoid paying for the
/// whole world on a local disturbance: a caller that actually wants "the
/// chunk the player just cut loose" passes a cap sized to a plausible body,
/// not `usize::MAX`.
pub fn label_component(world: &World, seed_x: i32, seed_y: i32, max_cells: usize) -> Option<Vec<(i32, i32)>> {
    if !is_body_material(world, seed_x, seed_y) {
        return None;
    }
    if max_cells == 0 {
        return Some(Vec::new());
    }

    let mut visited: HashSet<(i32, i32)> = HashSet::new();
    visited.insert((seed_x, seed_y));
    let mut stack = vec![(seed_x, seed_y)];
    let mut out = Vec::new();

    while let Some((x, y)) = stack.pop() {
        out.push((x, y));
        if out.len() >= max_cells {
            break;
        }
        for (dx, dy) in NEIGHBOURS_4 {
            let (nx, ny) = (x + dx, y + dy);
            if visited.contains(&(nx, ny)) {
                continue;
            }
            if is_body_material(world, nx, ny) {
                visited.insert((nx, ny));
                stack.push((nx, ny));
            }
        }
    }

    Some(out)
}

/// `Solid` and not `BEDROCK` — bedrock is the world's anchor/boundary
/// material (and, via `Cell::OUT_OF_BOUNDS`, what every out-of-bounds read
/// returns too — see `structural.rs`'s identical single-check trick for
/// treating "literal bedrock" and "the world edge" as the same case), never
/// a body a player could cut free. Without this exclusion the flood fill
/// walks onto the world's own boundary wall the first time it reaches any
/// edge-adjacent cell and never stops until `max_cells` — caught by
/// `a_component_smaller_than_the_cap_returns_its_true_size` reporting 1000
/// cells for a 5-cell blob before this check existed.
fn is_body_material(world: &World, x: i32, y: i32) -> bool {
    let material = world.get(x, y).material;
    world.materials.kind(material) == MaterialKind::Solid && material != material::BEDROCK
}

/// A point in grid-corner space: cell `(x, y)` occupies the unit square
/// from corner `(x, y)` to `(x + 1, y + 1)`.
pub type Point = (i32, i32);

/// Extract every closed boundary contour of `cells` (typically a
/// `label_component` result) as ordered rings of grid-corner points, each
/// ring closed (its last edge returns to its first point, not repeated in
/// the output). A simply-connected blob returns exactly one ring, its outer
/// boundary; a component with a hole in it returns the outer boundary plus
/// one further ring per hole, wound the opposite way. See the module doc
/// for why this is marching squares' actual job for a binary occupancy
/// grid, done without the continuous-field machinery the classic algorithm
/// needs and this input never requires. Returns an empty `Vec` for an empty
/// input.
pub fn trace_contours(cells: &[Point]) -> Vec<Vec<Point>> {
    let filled: HashSet<Point> = cells.iter().copied().collect();

    // One directed edge per side of a filled cell that faces a non-filled
    // neighbour, keyed by its start point -- see the module doc for the
    // orientation convention and why interior edges never get inserted at
    // all (both cells on either side of an interior edge see a filled
    // neighbour there, so neither contributes one).
    let mut next: HashMap<Point, Point> = HashMap::new();
    for &(x, y) in &filled {
        if !filled.contains(&(x + 1, y)) {
            next.insert((x + 1, y), (x + 1, y + 1)); // right side, downward
        }
        if !filled.contains(&(x - 1, y)) {
            next.insert((x, y + 1), (x, y)); // left side, upward
        }
        if !filled.contains(&(x, y - 1)) {
            next.insert((x, y), (x + 1, y)); // top side, rightward
        }
        if !filled.contains(&(x, y + 1)) {
            next.insert((x + 1, y + 1), (x, y + 1)); // bottom side, leftward
        }
    }

    let mut starts: Vec<Point> = next.keys().copied().collect();
    starts.sort(); // deterministic output order, not load-bearing for correctness

    let mut visited: HashSet<Point> = HashSet::new();
    let mut rings = Vec::new();
    for start in starts {
        if visited.contains(&start) {
            continue;
        }
        let mut ring = vec![start];
        visited.insert(start);
        let mut current = start;
        // A pinch point (see the module doc) silently drops one lobe's own
        // exit edge at the shared corner, rerouting it into the OTHER
        // lobe's already-closed cycle -- which never revisits this walk's
        // own `start`, so `n == start` alone never fires and the walk
        // cycles the sibling lobe forever, growing `ring` without bound.
        // `visited.contains(&n)` catches that: the sibling lobe's points
        // were already inserted earlier in *this* walk, so hitting any of
        // them again (not just `start`) means the walk has looped back on
        // itself and must stop. For an ordinary simple ring this never
        // trips before the `n == start` check above already does, since
        // `start` is the first point inserted into `visited` and nothing
        // else in a genuine simple polygon repeats until the ring closes.
        while let Some(&n) = next.get(&current) {
            if n == start {
                break;
            }
            if visited.contains(&n) {
                break; // looped back into an already-visited point without reaching start -- a pinch point; stop rather than looping forever
            }
            ring.push(n);
            visited.insert(n);
            current = n;
        }
        rings.push(ring);
    }
    rings
}

/// One cell of a `ChunkBody`, stored as an offset from the body's origin so
/// the whole body moves by changing two floats rather than every cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyCell {
    pub dx: i32,
    pub dy: i32,
    pub material: MaterialId,
    /// Carried through the flight so a chunk lands with the same grain it
    /// broke off with — re-rolling it would make a landing visibly "pop".
    pub shade: u8,
}

/// A coherent piece of broken structure, in flight.
///
/// This is the pipeline's first *gameplay* wiring: `label_component` and
/// `trace_contours` above have always been pure queries with nothing calling
/// them. A body is promoted out of a structural failure, falls as one piece,
/// and re-rasterizes back into the grid when it comes to rest.
///
/// # Why this is not `rapier2d`, yet
///
/// The full M8 pipeline continues past here into Douglas-Peucker,
/// `earcutr`, and a real collider. Debris that breaks off, tumbles, lands
/// and becomes terrain again needs gravity, a grid fit test and a settle
/// rule — not a constraint solver — and roughly everything in this module
/// except `advance`'s integration step is shared with the rapier version if
/// it ever lands. So the integrator is deliberately the *last* decision
/// rather than the first, per this repo's own "for 'does this look right',
/// ship a runtime selector rather than choosing" convention.
///
/// Two things that stay true either way and are worth stating before that
/// choice is made: `Reports/coupling-research.md` §0.2 flags that rapier's
/// `enhanced-determinism` cannot be combined with its `parallel` feature,
/// against a determinism requirement `PLAN.md` reversed to *required*; and
/// §4 flags that a body must never be stepped inside the CA sweep, since one
/// spanning two chunks of the same checkerboard parity would write to both
/// and break `parallel.rs`'s write-disjointness proof. This runs in its own
/// serial phase for exactly that reason.
#[derive(Clone, Debug)]
pub struct ChunkBody {
    pub cells: Vec<BodyCell>,
    /// Origin in world space. Fractional, so a body accumulates sub-cell
    /// motion instead of only ever moving in whole-cell jumps.
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    /// Accumulated turn, in quarter-turns. A body tips as it falls rather
    /// than gliding down flat, which was the single most-reported thing
    /// wrong with how debris looked: "when they do [break off] they just
    /// fall directly perfectly flat down."
    ///
    /// Quarter turns, not free rotation, and deliberately so. A cell grid
    /// has no way to represent a slab at 37 degrees without resampling it,
    /// and resampling a rotating body is where the classic re-rasterization
    /// leaks and holes come from (`PLAN.md` records the pitfall and the
    /// inverse-mapping fix). At 90 degrees the transform is exact — an
    /// offset maps to another offset with no interpolation at all — so a
    /// tumbling chunk can never gain or lose a cell.
    spin: f32,
    stalled: u8,
}

impl ChunkBody {
    /// Turn the whole body a quarter turn about its origin.
    ///
    /// `(dx, dy) -> (-dy, dx)`, which is exact on a grid: every cell lands
    /// on exactly one other cell, so this cannot leak or duplicate material
    /// however many times it is applied.
    fn rotate_quarter(&mut self) {
        for cell in &mut self.cells {
            let (dx, dy) = (cell.dx, cell.dy);
            cell.dx = -dy;
            cell.dy = dx;
        }
    }

    /// Where `cell` currently sits in world coordinates.
    pub fn cell_position(&self, cell: &BodyCell) -> (i32, i32) {
        (self.x.round() as i32 + cell.dx, self.y.round() as i32 + cell.dy)
    }
}

/// Promote the connected region of *already-failing* structure containing
/// `(x, y)` into a `ChunkBody`, returning whether it did.
///
/// Returns `false` — leaving the caller to fall back to per-cell
/// `breaks_into` conversion — when the region is too small to read as a
/// chunk or too large to be one.
///
/// # Why the flood fill has a failure predicate rather than reusing
/// `label_component`
///
/// `label_component` labels connected *solid*, which for an overhang that
/// is about to come down includes the entire mountain it is still attached
/// to. What a body needs is the connected set of cells that are themselves
/// past their span — which is a different question with a conveniently
/// cheap answer, because `structural.rs` has already written every cell's
/// distance into `aux` by the time a break fires. So this is a flood over
/// "solid, and `aux` exceeds this material's `max_unsupported_span`".
///
/// # What this does *not* catch, measured rather than assumed
///
/// **Only a detached region becomes a chunk. Progressive span failure does
/// not, and cannot.** This was verified by counting bodies rather than by
/// looking at the result, which mattered: a mined roof coming down as loose
/// rubble and one coming down as a coherent slab are nearly indis-
/// tinguishable in a contact sheet, and the picture read as "chunks are
/// working" while `filmstrip`'s own body count said `bodies 0` throughout.
///
/// The two cases differ for a real reason. In `filmstrip`'s `snap` scene a
/// shelf is cut off its pillar, so *nothing* in it can reach an anchor; its
/// cells then climb in lockstep (the count-to-infinity dynamic in
/// `structural.rs`'s module doc) and cross their span within a tick or two
/// of each other, so the flood finds the whole region at once — 54 cells,
/// one body. In the `mine` scene the roof is still attached to an anchored
/// stub at each end, so its cells sit at genuinely different distances and
/// the failure wavefront walks inward one cell per
/// `STRUCTURAL_TICK_INTERVAL`. At the instant the first cell crosses, its
/// neighbour is still one short of crossing, and the region is one cell.
///
/// That is the physically right answer rather than a gap to paper over: a
/// roof still held at both ends is not a chunk that fell off, it is a span
/// crumbling, and it should come down as loose material. Widening the
/// predicate to "within a cell or two of failing" would sweep in cells that
/// genuinely stand. Getting chunks out of *mining* specifically would need
/// a different trigger — reachability computed forward rather than read
/// from the cached distance — and is deliberately not attempted here.
pub fn try_promote_failing_region(world: &mut World, x: i32, y: i32) -> bool {
    let Some(region) = label_failing_region(world, x, y, MAX_FRACTURE_CELLS) else {
        return false;
    };
    // No upper bound. `MAX_BODY_CELLS` caps a *single* body, and `fracture`
    // splits whatever it is given into fragments of at most 32-128 cells, so
    // a large collapse should simply yield more pieces. Gating on it meant
    // anything bigger than 400 cells fell through to per-cell conversion and
    // dissolved -- the bigger the piece, the more certain it turned to dust,
    // which is exactly backwards and is the same mistake `strike` had.
    // Reported from play: a thick column cap where "part breaks into chunks
    // ... the other parts just crumble to dust".
    if region.len() < MIN_FRACTURE_CELLS {
        return false;
    }
    fracture(world, &region);
    true
}

/// Break `region` apart into a *distribution* of debris — several bodies of
/// differing size plus loose rubble — rather than one outcome for all of it.
///
/// # Why a distribution and not a single body
///
/// This used to promote the whole region as one `ChunkBody` if it was large
/// enough and convert every cell to rubble if it was not. Reported from
/// play: "it also seems like everything either disintegrates into powder or
/// breaks off as a large piece; there needs to be more rubble when things
/// break." Both outcomes were individually plausible and the *binary* was
/// the problem — real rock fracture produces a size distribution, a few
/// blocks and more cobbles and a lot of grit, and its absence reads as fake
/// on sight regardless of how each half behaves.
///
/// Fragment sizes are drawn from a power-of-two ladder, so small pieces are
/// common and large ones rare without needing a tuned curve — the shape
/// falls out of the draw rather than being authored, which is the line
/// `Reports/design-philosophy.md` §2b draws. Anything that comes out below
/// `MIN_BODY_CELLS` is not worth flying as a rigid body and becomes rubble,
/// so the grit is a *consequence* of the same draw rather than a separate
/// fudge factor.
fn fracture(world: &mut World, region: &[(i32, i32)]) {
    fracture_with_impulse(world, region, None, 0)
}

/// As `fracture`, but every fragment is thrown away from `origin` at
/// `force`. This is what makes a *blow* different from a collapse: the same
/// rock comes apart either way, but struck rock leaves the wound rather than
/// sagging out of it.
fn fracture_with_impulse(world: &mut World, region: &[(i32, i32)], impulse: Option<((f32, f32), f32)>, size_bias: u32) {
    // Every destructive event owes feedback (`Reports/design-philosophy.md`
    // §0a), and a collapse is one. `break_free` has always written this for
    // a single cell converting to rubble; a region coming apart into chunks
    // is a louder version of the same event and was silently writing
    // nothing, so a structure that failed by *cracking* -- now the common
    // case -- shoved no air at all while one that crumbled did.
    if !region.is_empty() {
        // Centred on the region and sized to *reach* all of it. Sizing the
        // radius from the cell count instead looked reasonable and left the
        // far end of anything long and thin outside the impulse entirely --
        // a 22-cell cantilever collapsing shoved no air at its own tip.
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for &(x, y) in region {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        let radius = (((x1 - x0) / 2).max((y1 - y0) / 2) + 1).max(2);
        let strength = (region.len() as f32).sqrt() * COLLAPSE_PRESSURE;
        world.add_pressure_impulse((x0 + x1) / 2, (y0 + y1) / 2, radius, strength);
    }
    let mut remaining: Vec<(i32, i32)> = region.to_vec();
    remaining.sort_unstable(); // deterministic seed order, as `label_failing_region` already guarantees
    let mut left: HashSet<(i32, i32)> = remaining.iter().copied().collect();

    for &seed in &remaining {
        if !left.contains(&seed) {
            continue;
        }
        // 2, 4, 8, 16 or 32 cells. Uniform over the exponent means each
        // doubling is half as likely per cell of material consumed, which
        // is the heavy-tailed shape fragmentation actually has.
        // A wider blow takes bigger pieces off. `size_bias` shifts the
        // whole ladder up, so a heavy swing or a large blast calves slabs
        // where a light tap produces chips -- without flattening the
        // distribution, which is what stops it reading as all-or-nothing
        // again.
        let target = (1usize << (1 + world.rng.below(5) as usize + size_bias as usize)).min(MAX_BODY_CELLS);
        let fragment = take_fragment(&mut left, seed, target);
        if fragment.len() >= MIN_BODY_CELLS {
            promote(world, &fragment, impulse);
        } else {
            for &(fx, fy) in &fragment {
                shatter_to_rubble(world, fx, fy);
            }
        }
    }
}

/// Flood up to `target` connected cells out of `left`, removing them.
fn take_fragment(left: &mut HashSet<(i32, i32)>, seed: (i32, i32), target: usize) -> Vec<(i32, i32)> {
    // Breadth-first, not depth-first. A DFS flood snakes: it follows one
    // arm to its end before coming back, so fragments came out as long
    // stringy runs — "thin individual pixel lines" rather than chunks. BFS
    // grows outward in rings from the seed, which gives compact, blocky
    // pieces of roughly the requested size.
    let mut out = Vec::new();
    let mut stack = std::collections::VecDeque::from([seed]);
    left.remove(&seed);
    while let Some((x, y)) = stack.pop_front() {
        out.push((x, y));
        if out.len() >= target {
            break;
        }
        for (dx, dy) in NEIGHBOURS_4 {
            let next = (x + dx, y + dy);
            if left.remove(&next) {
                stack.push_back(next);
            }
        }
    }
    // Anything still stacked was claimed out of `left` but never reached
    // before the size cap, so it has to go back or it is silently deleted --
    // material that belonged to the region and ends up neither in a fragment
    // nor in the pool for the next one. Caught by the conservation
    // assertions, which is exactly what they are for.
    for pending in stack {
        left.insert(pending);
    }
    out
}

/// Lift `cells` out of the grid as one coherent falling body.
fn promote(world: &mut World, cells: &[(i32, i32)], impulse: Option<((f32, f32), f32)>) {
    let (ox, oy) = cells[0];
    let body_cells: Vec<BodyCell> = cells
        .iter()
        .map(|&(cx, cy)| {
            let cell = world.get(cx, cy);
            BodyCell { dx: cx - ox, dy: cy - oy, material: cell.material, shade: cell.shade }
        })
        .collect();
    for &(cx, cy) in cells {
        world.set(cx, cy, Cell::EMPTY);
    }
    for &(cx, cy) in cells {
        world.schedule_structural_check_around(cx, cy);
    }
    // A piece breaks *outward*, not straight down: seed a sideways nudge
    // and a starting tilt from which side of the fracture it came off. Both
    // are what stop a shattered shelf from falling as a neat stack of
    // perfectly level bars.
    let spread = (ox % 5 - 2) as f32 * 0.18;
    // Thrown outward from the blow, if there was one. Falls back to the
    // sideways nudge a collapse gets, so a piece that merely came loose
    // still does not drop perfectly straight.
    let (vx, vy) = match impulse {
        Some(((fx, fy), force)) => {
            let (dx, dy) = (ox as f32 - fx, oy as f32 - fy);
            let distance = (dx * dx + dy * dy).sqrt().max(1.0);
            // Falls off with distance, so the rock nearest the blow is
            // thrown hardest and the far edge merely sags free -- a uniform
            // push reads as an explosion regardless of how hard you hit.
            let scale = force / distance;
            (dx / distance * scale + spread, dy / distance * scale)
        }
        None => (spread, 0.0),
    };
    world.chunk_bodies.push(ChunkBody {
        cells: body_cells,
        x: ox as f32,
        y: oy as f32,
        vx,
        vy,
        spin: ((ox + oy) % 3) as f32 * 0.3,
        stalled: 0,
    });
}

/// Convert one cell to its material's `breaks_into`, losing attachment —
/// whatever comes free is no longer backed by the mass it broke out of.
fn shatter_to_rubble(world: &mut World, x: i32, y: i32) {
    let cell = world.get(x, y);
    let Some(into) = world.materials.get(cell.material).breaks_into else {
        return; // no configured debris: leave it rather than deleting content
    };
    let shades = world.materials.get(into).palette.len().max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    let temp = cell.temperature();
    world.set(x, y, Cell::new(into, shade).with_temperature(temp));
    world.schedule_structural_check_around(x, y);
}

/// Flood the connected cells that are solid, unowned, and already past
/// their own material's span -- see below for why that was wrong.
///
/// # Why the whole appendage, not the cells that are over span
///
/// Flooding only cells already past their span made structures dissolve
/// from the tip inward instead of cracking, for two compounding reasons.
/// The cascade is progressive, so at the instant the first cell crosses,
/// its neighbours have not, and the set is one cell -- below
/// `MIN_FRACTURE_CELLS`, so it fell through to per-cell conversion and the
/// fracture path never ran. And when a player builds outward the failing
/// cell is the one just placed at the very tip, with nothing beyond it, so
/// the set is one cell however patient the cascade is.
///
/// Real cantilevers fail at the root and the whole span drops. With
/// `support_cost_below` at 0 a cell's stored distance is its horizontal
/// reach out from whatever holds it up, so anchored material sits at 0 and
/// anything hanging off it is 1 or more. Flooding "connected, unattached,
/// distance above zero" therefore takes the entire appendage and stops dead
/// at the anchor, with no threshold to tune. A tower is untouched: every
/// cell of it is at 0 through that same free downward step.
fn label_failing_region(world: &World, seed_x: i32, seed_y: i32, max_cells: usize) -> Option<Vec<(i32, i32)>> {
    let hanging = |x: i32, y: i32| {
        let cell = world.get(x, y);
        if !is_body_material(world, x, y) || cell.organism_id() != 0 {
            return false;
        }
        // Attached rock is held by the mass behind it and is not hanging off
        // anything; it must never be swept into a falling piece by a
        // neighbour's failure. Everything else with a non-zero distance is,
        // by definition, reaching out from something rather than sitting on
        // it -- see this function's doc for why that is the right set.
        !cell.attached() && cell.aux() > 0
    };
    if !hanging(seed_x, seed_y) {
        return None;
    }

    let mut visited: HashSet<(i32, i32)> = HashSet::new();
    visited.insert((seed_x, seed_y));
    let mut stack = vec![(seed_x, seed_y)];
    let mut out = Vec::new();
    while let Some((x, y)) = stack.pop() {
        out.push((x, y));
        if out.len() >= max_cells {
            break; // caller treats an over-cap region as "too big to be a chunk"
        }
        for (dx, dy) in NEIGHBOURS_4 {
            let next = (x + dx, y + dy);
            if visited.contains(&next) {
                continue;
            }
            // A fracture is where the piece ends: walking across one would
            // drag down rock it is no longer joined to.
            if super::structural::edge_is_cracked(world, x, y, dx, dy) {
                continue;
            }
            if hanging(next.0, next.1) {
                visited.insert(next);
                stack.push(next);
            }
        }
    }
    // Sorted so promotion order (and therefore the body's cell order, and
    // therefore every fit test and landing write below) is identical run to
    // run -- a `HashSet`-driven flood is not, which is issue #7's exact
    // determinism trap in a new place.
    out.sort_unstable();
    Some(out)
}

/// Score fractures radiating out from a blow.
///
/// A crack runs *along* the ray, so it separates the rock either side of
/// that line: travelling horizontally it cuts the down edges of the cells it
/// crosses, travelling vertically the right edges. Which is to say the edge
/// marked is the one perpendicular to the direction of travel — get that
/// backwards and the cracks run across the ray instead of along it, cutting
/// the rock into rings rather than wedges.
///
/// Diagonal rays leave a staircase of cuts that does not fully separate
/// anything, and that is correct rather than a limitation: a partial crack
/// should weaken rock without detaching it. Capacity counts cracked edges,
/// so a staircase weakens; only a continuous cut separates.
///
/// **Cracks prefer to extend cracks.** Running into existing damage buys the
/// ray extra reach, because a crack tip is where stress concentrates — so
/// hitting the same spot repeatedly drives a fissure deeper rather than
/// scribbling a fresh one each time, and the player can work a crack along
/// deliberately. That is the difference between damage that accumulates and
/// damage that merely repeats.
fn score_cracks(world: &mut World, cx: i32, cy: i32, from: i32, length: i32, rays: u32) {
    // Ray directions are keyed on the *site*, not drawn fresh per blow, and
    // that is what makes damage accumulate. Jittering per call sent every
    // hit off in new directions, so a second blow on the same spot scored
    // fresh fissures beside the old ones instead of driving them deeper --
    // the crack-tip bonus below could almost never find anything to extend.
    // It is also the more physical reading: how a given rock splits at a
    // given point is a property of the rock, not of the swing.
    let jitter = super::rng::jitter(cx, cy);
    // Only *pre-existing* damage earns a ray extra reach. Rays from one blow
    // cross each other near the origin, so counting cracks this call just
    // made had every strike max out its own bonus immediately and left
    // nothing for the next one to build on -- the reach was identical after
    // one blow and after two.
    let mut scored_now: HashSet<(i32, i32)> = HashSet::new();
    for i in 0..rays {
        let theta = (i as f32 + jitter) * std::f32::consts::TAU / rays as f32;
        let (dx, dy) = (theta.cos(), theta.sin());
        let horizontal = dx.abs() > dy.abs();
        let mut reach = length;
        let mut r = from;
        while r <= reach {
            let x = cx + (dx * r as f32).round() as i32;
            let y = cy + (dy * r as f32).round() as i32;
            if !world.in_bounds(x, y) || !is_body_material(world, x, y) {
                break; // a crack stops at a free face; there is nothing left to split
            }
            let cell = world.get(x, y);
            if cell.cracked() && !scored_now.contains(&(x, y)) {
                reach = (reach + CRACK_TIP_BONUS).min(length + CRACK_TIP_BONUS * CRACK_TIP_MAX_STEPS);
            }
            let scored = if horizontal { cell.with_crack_down(true) } else { cell.with_crack_right(true) };
            world.set(x, y, scored);
            scored_now.insert((x, y));
            world.schedule_structural_check_around(x, y);
            r += 1;
        }
    }
}

/// Hit the rock at `(cx, cy)` hard enough to break it.
///
/// # Why destruction needs a verb of its own
///
/// Until this existed the only way to provoke a structural failure was to
/// *erase* support and wait for the consequences. An eraser delivers no load
/// and no impulse, so nothing ever failed from being struck -- the mechanic
/// worked and still felt inert, which is the failure
/// `Reports/design-philosophy.md` §0a records as "a mechanic needs a verb".
/// Waiting several ticks for a cascade to notice you removed something is
/// also the opposite of feedback.
///
/// A blow does three things at once, and all three are the point:
///
/// - **Pulverizes** the cells it lands on. That is the bite taken out.
/// - **Loosens** the rock around the wound, stripping its attachment to the
///   background mass so it is no longer braced by the rock behind it. This
///   is the same `detach_exposed_neighbours` transition digging goes
///   through, applied over the whole struck area at once.
/// - **Fractures and throws** what it loosened, immediately rather than via
///   the cascade, so the pieces leave the wound on the frame you hit it.
///
/// Struck rock that was already unsupported still gets picked up by the
/// ordinary structural cascade afterwards, so hitting the base of an
/// overhang both throws chips *and* brings the overhang down a moment later.
pub fn strike(world: &mut World, cx: i32, cy: i32, radius: i32, force: f32) {
    // Three zones, and the split is what makes a blow read as a blow rather
    // than as a hole appearing. The core is pulverized -- that is the bite.
    // A thin shell around it chips off immediately, so every hit produces
    // visible flying rock whether or not anything structural gives way. Past
    // that the rock is *scored*: damage that shows and accumulates without
    // detaching, which is the state the whole mechanic was missing.
    let core = (radius / 3).max(1);
    let chip = (radius * 2 / 3).max(core + 1);
    let mut loosened = Vec::new();
    for dy in -chip..=chip {
        for dx in -chip..=chip {
            let (x, y) = (cx + dx, cy + dy);
            let d2 = dx * dx + dy * dy;
            if d2 > chip * chip || !world.in_bounds(x, y) {
                continue;
            }
            if !is_body_material(world, x, y) || world.get(x, y).organism_id() != 0 {
                continue;
            }
            if d2 <= core * core {
                shatter_to_rubble(world, x, y); // the bite
            } else {
                loosened.push((x, y));
            }
        }
    }
    // Cracks reach well past the rock the blow actually removes -- that
    // reach is the whole point of striking rather than erasing.
    score_cracks(world, cx, cy, chip, radius * CRACK_REACH, CRACK_RAYS);
    // Loosen first, so the fracture below sees rock that is no longer
    // claiming to be braced by the massif.
    for &(x, y) in &loosened {
        let cell = world.get(x, y);
        if cell.attached() {
            world.set(x, y, cell.with_attached(false));
        }
    }
    // No upper bound here, deliberately. `MAX_BODY_CELLS` exists to stop a
    // *single* body being enormous; `fracture` splits whatever it is given
    // into fragments of at most 32 cells, so a wide blow just yields more
    // pieces. Gating on it meant a big strike loosened several hundred cells
    // and then threw nothing at all -- the larger the swing, the less
    // happened, which is exactly backwards.
    if loosened.len() >= MIN_FRACTURE_CELLS {
        fracture_with_impulse(world, &loosened, Some(((cx as f32, cy as f32), force)), size_bias(radius));
    }
    // Every destructive event owes feedback (`Reports/design-philosophy.md`
    // §0a). A blow shoves the air as well as the rock, which is what makes
    // smoke and loose grit near the impact react instead of a hit landing in
    // total silence.
    world.add_pressure_impulse(cx, cy, radius.max(2), force * STRIKE_PRESSURE);
    for &(x, y) in &loosened {
        world.schedule_structural_check_around(x, y);
    }
}

/// Break the loosened rock around a blast into chunks and throw them.
///
/// Explosions cleared their crater and spawned single-cell debris particles,
/// which is the right treatment for sand and the wrong one for rock: a blast
/// against stone produced a clean hole and a spray of grit, never a piece.
/// Reported as wanting the opposite -- "you explode a rock to make a cave and
/// pieces should crack."
///
/// Called after a blast stage has finished clearing, on the shell it just
/// exposed. Only unattached solid cells are eligible, which means the rim
/// has already been through `structural::detach_exposed_neighbours` and is
/// genuinely no longer braced by the mass behind it.
pub fn fracture_shell(world: &mut World, origin: (i32, i32), inner: i32, outer: i32, force: f32, size_bias: u32) {
    let mut loosened = Vec::new();
    // An annulus, not a disc. Scanning the whole disc also swept up the
    // crater's *interior* -- material the blast was still working through --
    // and converted it before the front reached it, which left later stages
    // nothing to clear. The wall is what comes away, not the hole.
    for dy in -outer..=outer {
        for dx in -outer..=outer {
            let (x, y) = (origin.0 + dx, origin.1 + dy);
            let d2 = dx * dx + dy * dy;
            if d2 > outer * outer || d2 < inner * inner || !world.in_bounds(x, y) {
                continue;
            }
            let cell = world.get(x, y);
            if cell.attached() || !is_body_material(world, x, y) || cell.organism_id() != 0 {
                continue;
            }
            loosened.push((x, y));
        }
    }
    if loosened.len() >= MIN_FRACTURE_CELLS {
        loosened.sort_unstable(); // deterministic seed order
        fracture_with_impulse(world, &loosened, Some(((origin.0 as f32, origin.1 as f32), force)), size_bias);
    }
}

/// Advance every body one frame, settling any that have come to rest.
///
/// Runs in its own serial phase after the CA sweep, never inside it — see
/// `ChunkBody`'s own doc for the write-disjointness reason, which is the
/// same reason `World::step_liquid_bodies` sits where it does.
pub fn step_chunk_bodies(world: &mut World) {
    // Taken out and put back, because advancing a body needs `&mut World`
    // while the loop holds the collection -- the same borrow shape (and the
    // same fix) `scheduler::step` and `ParticleSystem::step` both use.
    let taken = std::mem::take(&mut world.chunk_bodies);
    let mut still_moving = Vec::with_capacity(taken.len());
    for mut body in taken {
        if advance(world, &mut body) {
            still_moving.push(body);
        } else {
            settle(world, &body);
        }
    }
    // Anything promoted *during* this loop (a settling body can bring more
    // structure down) is already sitting in `world.chunk_bodies`, so extend
    // rather than assign -- assigning would silently discard it, which is
    // the bug `World::pop_due_active_site`'s doc records the scheduler
    // hitting for the identical reason.
    still_moving.append(&mut world.chunk_bodies);
    world.chunk_bodies = still_moving;
}

/// Returns whether the body is still in flight; `false` means it has come
/// to rest and should be re-rasterized.
fn advance(world: &mut World, body: &mut ChunkBody) -> bool {
    body.vy += GRAVITY;

    // Tip while falling. Spin accumulates with speed, so a piece that has
    // barely come loose turns slowly and one in a long drop tumbles -- and a
    // body that is already at rest does not rotate at all, which matters
    // because a settled chunk snapping through 90 degrees would look like a
    // glitch rather than physics.
    body.spin += (body.vx.abs() + body.vy.abs()) * SPIN_PER_SPEED;
    if body.spin >= 1.0 {
        body.spin -= 1.0;
        let turned = {
            let mut probe = body.clone();
            probe.rotate_quarter();
            // Only turn if the turned shape actually fits. Otherwise a body
            // wedged in a gap would rotate straight through the wall beside
            // it, which is the one way this transform can cheat.
            blocked_axis(world, &probe, probe.x, probe.y).is_none()
        };
        if turned {
            body.rotate_quarter();
        }
    }

    body.vx = body.vx.clamp(-MAX_SPEED_PER_AXIS, MAX_SPEED_PER_AXIS);
    body.vy = body.vy.clamp(-MAX_SPEED_PER_AXIS, MAX_SPEED_PER_AXIS);

    let distance = (body.vx * body.vx + body.vy * body.vy).sqrt();
    let steps = distance.ceil().max(1.0) as i32;
    let (step_x, step_y) = (body.vx / steps as f32, body.vy / steps as f32);

    // Substepped at a maximum of one cell, for the same anti-tunnelling
    // reason `particle::advance_and_check_landing` documents: a body moving
    // several cells in one unclamped jump could cross a thin floor entirely
    // without any sampled position ever landing inside it.
    let mut moved = false;
    for _ in 0..steps {
        let (next_x, next_y) = (body.x + step_x, body.y + step_y);
        match blocked_axis(world, body, next_x, next_y) {
            None => {
                body.x = next_x;
                body.y = next_y;
                moved = true;
            }
            Some(axis) => {
                // Kill only the axis that actually hit something, so a body
                // landing on a floor keeps sliding sideways instead of
                // stopping dead in mid-air.
                match axis {
                    Axis::Horizontal => body.vx *= -COLLISION_RETENTION,
                    Axis::Vertical => body.vy *= COLLISION_RETENTION,
                    Axis::Both => {
                        body.vx *= -COLLISION_RETENTION;
                        body.vy *= COLLISION_RETENTION;
                    }
                }
                break;
            }
        }
    }

    body.stalled = if moved { 0 } else { body.stalled.saturating_add(1) };
    body.stalled < STALL_FRAMES_BEFORE_SETTLING
}

enum Axis {
    Horizontal,
    Vertical,
    Both,
}

/// Which axis of a move into `(next_x, next_y)` is obstructed, if either.
///
/// Loose material in the way is *displaced*, not deleted — the trap
/// `Reports/coupling-research.md` §4 names explicitly ("any cell the body
/// moves into must have its contents displaced, not deleted… the engine's
/// conservation tests will catch it only if they are extended to cover
/// rigid bodies, which they currently are not"). They are now:
/// `a_body_moving_through_powder_destroys_no_material`.
fn blocked_axis(world: &mut World, body: &ChunkBody, next_x: f32, next_y: f32) -> Option<Axis> {
    let (ox, oy) = (next_x.round() as i32, next_y.round() as i32);
    let mut horizontal = false;
    let mut vertical = false;

    for cell in &body.cells {
        let (tx, ty) = (ox + cell.dx, oy + cell.dy);
        let (cx, cy) = body.cell_position(cell);
        if (tx, ty) == (cx, cy) {
            continue; // this cell is not actually changing position this substep
        }
        if !world.in_bounds(tx, ty) || !clear_or_displaceable(world, body, tx, ty) {
            // Attribute the block to the axis this cell was moving along.
            if tx != cx {
                horizontal = true;
            }
            if ty != cy {
                vertical = true;
            }
        }
    }

    match (horizontal, vertical) {
        (false, false) => None,
        (true, false) => Some(Axis::Horizontal),
        (false, true) => Some(Axis::Vertical),
        (true, true) => Some(Axis::Both),
    }
}

/// Whether `(x, y)` is available for a body cell to occupy — either already
/// empty, or holding loose material that could be shoved aside. Actually
/// performs the shove when it can, which is why this takes `&mut World`.
fn clear_or_displaceable(world: &mut World, body: &ChunkBody, x: i32, y: i32) -> bool {
    // A cell the body itself currently occupies is not an obstacle: bodies
    // are lifted out of the grid on promotion, so anything found here
    // belongs to the world, not to this body -- but a *neighbouring* body
    // cell's destination may legitimately be this one mid-move.
    if world.is_empty(x, y) {
        return true;
    }
    let kind = world.materials.kind(world.get(x, y).material);
    if !matches!(kind, MaterialKind::Powder | MaterialKind::Liquid) {
        return false; // solid, plant, creature -- a real obstruction
    }
    displace(world, body, x, y)
}

/// Shove the loose cell at `(x, y)` to the nearest empty cell that the body
/// is not about to occupy, returning whether it found somewhere.
///
/// Ring by ring, so material surfaces at the nearest opening rather than
/// teleporting to whichever cell scan order happened to reach first — the
/// same shape, and the same reasoning, as `particle::land`'s own
/// nearest-empty search.
fn displace(world: &mut World, body: &ChunkBody, x: i32, y: i32) -> bool {
    let occupied: HashSet<(i32, i32)> = body.cells.iter().map(|c| body.cell_position(c)).collect();
    for ring in 1..=DISPLACE_SEARCH {
        for dy in -ring..=ring {
            for dx in -ring..=ring {
                if dx.abs() != ring && dy.abs() != ring {
                    continue; // interior already covered by a smaller ring
                }
                let (nx, ny) = (x + dx, y + dy);
                if occupied.contains(&(nx, ny)) {
                    continue; // no point shoving material into the body's own path
                }
                if world.in_bounds(nx, ny) && world.is_empty(nx, ny) {
                    let moving = world.get(x, y);
                    world.set(nx, ny, moving);
                    world.set(x, y, Cell::EMPTY);
                    return true;
                }
            }
        }
    }
    false // nowhere to put it -- treat as solid rather than deleting it
}

/// Write a settled body's cells back into the grid as ordinary CA material.
///
/// Three things here are easy to leave out and each is a real bug:
///
/// - **`aux` is reset.** A body's cells were promoted *because* their stored
///   distance exceeded their span. Writing that value back means the cell
///   fails its very next structural check and re-breaks on the spot, which
///   reads as debris that can never come to rest.
/// - **Structural checks are scheduled around the landing.** Otherwise a
///   landed chunk is invisible to the system that created it — it could not
///   support anything, and nothing could be brought down by it.
/// - **A cell with nowhere to go searches** rather than being dropped, so
///   landing on uneven ground does not quietly delete the overlapping part
///   of the body.
fn settle(world: &mut World, body: &ChunkBody) {
    for cell in &body.cells {
        let (x, y) = body.cell_position(cell);
        // `Cell::new` starts unattached and with `aux` at 0, and both are
        // deliberate. A body only exists because it broke out of something,
        // so it is no longer backed by the mass it came from -- landing must
        // not silently re-attach it, or a chunk that fell would become
        // immovable terrain wherever it happened to stop.
        let fresh = Cell::new(cell.material, cell.shade);
        if world.in_bounds(x, y) && world.is_empty(x, y) {
            world.set(x, y, fresh);
            continue;
        }
        if let Some((nx, ny)) = nearest_free(world, x, y) {
            world.set(nx, ny, fresh);
        }
        // Genuinely nowhere within reach -- dropped, matching `particle::
        // land`'s own last resort for a grain with no legal rest position.
    }
    for cell in &body.cells {
        let (x, y) = body.cell_position(cell);
        world.schedule_structural_check_around(x, y);
    }
}

fn nearest_free(world: &World, x: i32, y: i32) -> Option<(i32, i32)> {
    for ring in 1..=DISPLACE_SEARCH {
        for dy in -ring..=ring {
            for dx in -ring..=ring {
                if dx.abs() != ring && dy.abs() != ring {
                    continue;
                }
                let (nx, ny) = (x + dx, y + dy);
                if world.in_bounds(nx, ny) && world.is_empty(nx, ny) {
                    return Some((nx, ny));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell::Cell;
    use crate::sim::chunk::Rect;
    use crate::sim::material;

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 63, 63))
    }

    #[test]
    fn returns_none_for_a_non_solid_seed() {
        let mut w = test_world();
        w.set(10, 10, Cell::new(material::SAND, 0));
        assert!(label_component(&w, 10, 10, 1000).is_none());
        assert!(label_component(&w, 20, 20, 1000).is_none(), "an empty seed should also return None");
    }

    #[test]
    fn labels_every_cell_of_a_simple_connected_blob() {
        let mut w = test_world();
        // An L-shape, deliberately not a filled rectangle -- exercises the
        // flood fill actually following connectivity, not just bounding-box
        // math.
        let cells = [(10, 10), (11, 10), (12, 10), (10, 11), (10, 12)];
        for &(x, y) in &cells {
            w.set(x, y, Cell::new(material::STONE, 0));
        }

        let mut found = label_component(&w, 10, 10, 1000).unwrap();
        found.sort();
        let mut expected: Vec<(i32, i32)> = cells.to_vec();
        expected.sort();
        assert_eq!(found, expected);
    }

    #[test]
    fn does_not_cross_into_a_disconnected_component() {
        let mut w = test_world();
        for x in 10..13 {
            w.set(x, 10, Cell::new(material::STONE, 0));
        }
        // A second blob, far enough away to share no neighbour with the first.
        for x in 30..33 {
            w.set(x, 10, Cell::new(material::STONE, 0));
        }

        let found = label_component(&w, 10, 10, 1000).unwrap();
        assert_eq!(found.len(), 3, "labeled cells from a component it was never seeded in");
        assert!(found.iter().all(|&(x, _)| x < 20), "crossed into the second, disconnected blob");
    }

    #[test]
    fn diagonal_only_contact_does_not_connect_two_components() {
        let mut w = test_world();
        // Two single cells touching only at a shared corner.
        w.set(10, 10, Cell::new(material::STONE, 0));
        w.set(11, 11, Cell::new(material::STONE, 0));

        let found = label_component(&w, 10, 10, 1000).unwrap();
        assert_eq!(found, vec![(10, 10)], "a diagonal-only neighbour should not be part of the same component");
    }

    #[test]
    fn touching_the_world_boundary_does_not_flood_along_the_edge() {
        // Regression: `Cell::OUT_OF_BOUNDS` reads as `material::BEDROCK`,
        // whose `MaterialKind` is `Solid` -- a component touching the
        // world's edge would otherwise flood onto the boundary "wall" and
        // never stop until `max_cells`, the same way `structural.rs`'s
        // anchor check has to treat literal bedrock and the world edge as
        // one case. A small blob placed directly against the edge must
        // report only itself, not thousands of imaginary boundary cells.
        let mut w = test_world();
        for y in 5..8 {
            w.set(0, y, Cell::new(material::STONE, 0)); // touches the left world edge
        }
        let found = label_component(&w, 0, 6, 1000).unwrap();
        assert_eq!(found.len(), 3, "a boundary-touching blob should report only its own cells, not the world edge");
    }

    #[test]
    fn does_not_include_literal_bedrock() {
        let mut w = test_world();
        w.set(10, 10, Cell::new(material::STONE, 0));
        w.set(11, 10, Cell::new(material::BEDROCK, 0));
        w.set(12, 10, Cell::new(material::STONE, 0));

        let found = label_component(&w, 10, 10, 1000).unwrap();
        assert_eq!(found, vec![(10, 10)], "bedrock should never be part of a cuttable body, and should block the flood fill from reaching stone on its far side");
    }

    #[test]
    fn respects_the_max_cells_cap() {
        let mut w = test_world();
        for x in 0..60 {
            w.set(x, 10, Cell::new(material::STONE, 0));
        }
        let found = label_component(&w, 0, 10, 10).unwrap();
        assert_eq!(found.len(), 10, "the cap should have stopped the flood fill exactly at max_cells");
    }

    #[test]
    fn a_component_smaller_than_the_cap_returns_its_true_size() {
        let mut w = test_world();
        for x in 0..5 {
            w.set(x, 10, Cell::new(material::STONE, 0));
        }
        let found = label_component(&w, 0, 10, 1000).unwrap();
        assert_eq!(found.len(), 5, "a cap far larger than the component should not affect its reported size");
    }

    /// The shoelace formula, signed: positive for a clockwise ring in this
    /// engine's y-down coordinate space (matching `trace_contours`'
    /// documented orientation), negative for counter-clockwise. `abs()` of
    /// the result is the ring's enclosed area in unit cells -- used
    /// throughout these tests as a correctness check that's robust to
    /// exactly which point a ring happens to start at, unlike asserting an
    /// exact point sequence.
    fn signed_area(ring: &[Point]) -> i64 {
        let mut sum: i64 = 0;
        for i in 0..ring.len() {
            let (x0, y0) = ring[i];
            let (x1, y1) = ring[(i + 1) % ring.len()];
            sum += (x0 as i64) * (y1 as i64) - (x1 as i64) * (y0 as i64);
        }
        sum / 2
    }

    #[test]
    fn empty_input_produces_no_contours() {
        assert!(trace_contours(&[]).is_empty());
    }

    #[test]
    fn a_single_cell_traces_a_unit_square() {
        let rings = trace_contours(&[(0, 0)]);
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0], vec![(0, 0), (1, 0), (1, 1), (0, 1)]);
    }

    #[test]
    fn contour_area_matches_cell_count_for_a_filled_rectangle() {
        let cells = [(0, 0), (1, 0), (0, 1), (1, 1)];
        let rings = trace_contours(&cells);
        assert_eq!(rings.len(), 1, "a filled rectangle has no interior boundary to trace");
        assert_eq!(signed_area(&rings[0]).abs(), 4, "the traced boundary should enclose exactly the 4 filled cells");
    }

    #[test]
    fn contour_area_matches_cell_count_for_an_l_shape() {
        // The same L-shape `label_component`'s own tests use -- concave,
        // not just a rectangle, so this actually exercises following the
        // boundary around an inside corner.
        let cells = [(10, 10), (11, 10), (12, 10), (10, 11), (10, 12)];
        let rings = trace_contours(&cells);
        assert_eq!(rings.len(), 1);
        assert_eq!(signed_area(&rings[0]).abs(), 5);
    }

    #[test]
    fn a_hole_produces_a_second_oppositely_wound_ring() {
        let mut cells = Vec::new();
        for x in 0..3 {
            for y in 0..3 {
                if (x, y) != (1, 1) {
                    cells.push((x, y));
                }
            }
        }
        let rings = trace_contours(&cells);
        assert_eq!(rings.len(), 2, "a 3x3 block with its center missing should trace an outer boundary and one hole boundary");

        let areas: Vec<i64> = rings.iter().map(|r| signed_area(r)).collect();
        let (outer, hole) = if areas[0].abs() > areas[1].abs() { (areas[0], areas[1]) } else { (areas[1], areas[0]) };
        assert_eq!(outer.abs(), 9, "the outer ring should enclose the full 3x3 area");
        assert_eq!(hole.abs(), 1, "the hole ring should enclose exactly the one missing cell");
        assert_ne!(outer.signum(), hole.signum(), "a hole boundary should wind opposite to the outer boundary");
    }

    #[test]
    fn a_pinch_point_terminates_rather_than_hanging() {
        // Regression: an independent review found that two cells touching
        // only at a shared corner -- the minimal pinch-point case the
        // module doc already documents as producing wrong geometry -- could
        // make the inner walk loop forever instead of degrading to a
        // bounded, wrong ring. The start-point collision at the pinch
        // corner silently drops one lobe's own exit edge, rerouting it into
        // the OTHER (already-closed) lobe's cycle, which never revisits the
        // dropped lobe's own `start` -- so the walk's original `n == start`
        // guard alone never fires, and it cycles the sibling lobe forever,
        // growing `ring` without bound. Run on a background thread with a
        // timeout so a regression fails this test instead of hanging the
        // whole suite.
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(trace_contours(&[(0, 1), (1, 0)]));
        });
        let result = rx.recv_timeout(std::time::Duration::from_secs(5));
        assert!(result.is_ok(), "trace_contours hung on a pinch-point input instead of terminating");
    }

    // --- M8 chunk bodies ------------------------------------------------

    /// A slab of stone whose cells are already marked as failing, so
    /// promotion can be exercised without waiting out a real structural
    /// cascade. `aux` past stone's span is exactly the state
    /// `structural::tick` leaves a cell in at the moment it breaks.
    fn failing_slab(w: &mut World, x0: i32, y0: i32, width: i32, height: i32) {
        let over_span = w.materials.get(material::STONE).max_unsupported_span + 1;
        for y in y0..(y0 + height) {
            for x in x0..(x0 + width) {
                w.set(x, y, Cell::new(material::STONE, 0).with_aux(over_span));
            }
        }
    }

    fn count_material(w: &World, m: MaterialId) -> usize {
        (0..64).map(|x| (0..64).filter(|&y| w.get(x, y).material == m).count()).sum()
    }

    /// Every cell of the original structure, wherever it ended up: still in
    /// the grid as stone, converted to rubble, or lifted into a body. What
    /// fracture must conserve is *material*, not any one form of it.
    fn total_debris(w: &World) -> usize {
        let rubble = w.materials.get(material::STONE).breaks_into.expect("stone must define a breaks_into");
        count_material(w, material::STONE) + count_material(w, rubble) + w.chunk_bodies.iter().map(|b| b.cells.len()).sum::<usize>()
    }

    #[test]
    fn a_failing_region_is_promoted_and_leaves_the_grid() {
        let mut w = test_world();
        failing_slab(&mut w, 20, 20, 6, 3); // 18 cells, inside MIN..=MAX
        assert!(try_promote_failing_region(&mut w, 20, 20), "a slab this size should promote");

        assert_eq!(
            count_material(&w, material::STONE),
            0,
            "every cell of a fractured region must leave the grid, as a body or as rubble"
        );
        assert_eq!(total_debris(&w), 18, "fracture conserved the wrong amount of material");
        assert!(!w.chunk_bodies.is_empty(), "an 18-cell region should yield at least one body-sized fragment");
    }

    #[test]
    fn fracture_produces_a_mix_of_chunks_and_rubble_not_one_outcome() {
        // The reported complaint this exists for: "everything either
        // disintegrates into powder or breaks off as a large piece; there
        // needs to be more rubble when things break." Both halves have to be
        // present in the *same* failure, and the sizes have to vary -- a
        // single body, or a uniform dissolve, are the two things this must
        // fail on.
        let mut w = test_world();
        failing_slab(&mut w, 10, 10, 20, 12); // 240 cells, plenty to break up
        assert!(try_promote_failing_region(&mut w, 10, 10));

        let rubble = w.materials.get(material::STONE).breaks_into.unwrap();
        assert!(count_material(&w, rubble) > 0, "a fracture that produced no loose rubble at all is the all-or-nothing failure");
        assert!(w.chunk_bodies.len() > 1, "a 240-cell region should break into several pieces, not one");
        assert_eq!(total_debris(&w), 240, "fracture lost or duplicated material");

        let sizes: Vec<usize> = w.chunk_bodies.iter().map(|b| b.cells.len()).collect();
        let smallest = sizes.iter().min().unwrap();
        let largest = sizes.iter().max().unwrap();
        assert!(largest > smallest, "every fragment came out the same size, so the distribution is not doing anything: {sizes:?}");
    }

    #[test]
    fn a_region_too_small_to_read_as_a_chunk_is_not_promoted() {
        // The other half of the size gate. Below MIN_BODY_CELLS a "tumbling
        // body" is just a grain, and the caller falls back to per-cell
        // `breaks_into` conversion -- so this must decline, and must leave
        // the cells alone for that fallback to convert.
        let mut w = test_world();
        failing_slab(&mut w, 20, 20, 2, 2); // 4 cells
        assert!(!try_promote_failing_region(&mut w, 20, 20));
        assert!(w.chunk_bodies.is_empty());
        assert_eq!(count_material(&w, material::STONE), 4, "a declined region must be left in the grid for the caller to convert");
    }

    #[test]
    fn a_large_collapse_becomes_many_chunks_rather_than_dust() {
        // This asserted the opposite until play showed the opposite was
        // wrong. A region over `MAX_BODY_CELLS` used to decline promotion
        // and fall through to per-cell conversion, so the *bigger* the
        // collapse the more certain it dissolved -- a thick column's cap
        // came down as dust while smaller pieces broke into chunks.
        //
        // `MAX_BODY_CELLS` caps a single body, and `fracture` splits a
        // region into fragments regardless of how large it is, so size was
        // never a reason to decline. Kept as the reproduction, with the
        // claim reversed.
        let mut w = test_world();
        failing_slab(&mut w, 0, 10, 60, 10); // 600 cells, well past MAX_BODY_CELLS
        assert!(try_promote_failing_region(&mut w, 0, 10), "a large collapse should still break up, not decline");

        assert!(w.chunk_bodies.len() > 1, "600 cells should yield many pieces, not one");
        assert_eq!(total_debris(&w), 600, "fracture lost or duplicated material");
        assert_eq!(count_material(&w, material::STONE), 0, "every cell of the region should have left the grid");
    }

    #[test]
    fn only_cells_past_their_span_join_the_body() {
        // The reason this uses its own predicate instead of
        // `label_component`: an overhang about to come down is still
        // attached to the mountain, and labelling connected *solid* would
        // fly the mountain too.
        let mut w = test_world();
        failing_slab(&mut w, 20, 20, 6, 3); // failing
        for y in 20..23 {
            for x in 26..40 {
                w.set(x, y, Cell::new(material::STONE, 0)); // aux 0 -- well supported, and adjacent
            }
        }
        assert!(try_promote_failing_region(&mut w, 20, 20));
        assert_eq!(
            count_material(&w, material::STONE),
            42,
            "fracture reached past the failing region into cells that were still supported"
        );
    }

    #[test]
    fn a_body_falls_and_settles_back_into_the_grid() {
        let mut w = test_world(); // no floor
        for x in 0..64 {
            w.set(x, 63, Cell::new(material::BEDROCK, 0));
        }
        failing_slab(&mut w, 20, 10, 6, 3);
        assert!(try_promote_failing_region(&mut w, 20, 10));
        let start_y = w.chunk_bodies[0].y;

        // The CA sweep runs too: fracture now emits loose rubble alongside
        // bodies, and rubble is a `Powder` that only falls through the
        // ordinary sweep. Stepping bodies alone left it hanging in mid-air
        // at its original height, which is what this assertion caught.
        for _ in 0..200 {
            step_chunk_bodies(&mut w);
            crate::sim::update::step(&mut w);
        }

        assert!(w.chunk_bodies.is_empty(), "the bodies never settled");
        assert_eq!(total_debris(&w), 18, "material was lost or duplicated between fracture and landing");
        // It came to rest lower than it started, i.e. it actually fell
        // rather than settling where it was promoted.
        let rubble = w.materials.get(material::STONE).breaks_into.unwrap();
        let landed = (0..64)
            .flat_map(|x| (0..64).map(move |y| (x, y)))
            .filter(|&(x, y)| matches!(w.get(x, y).material, m if m == material::STONE || m == rubble))
            .map(|(_, y)| y)
            .min()
            .unwrap();
        assert!((landed as f32) > start_y, "the body settled at or above where it started ({landed} vs {start_y})");
    }

    #[test]
    fn a_settled_body_does_not_immediately_re_break() {
        // A body is promoted *because* its cells' stored distance exceeded
        // their span. Writing that value back on landing would fail the very
        // next structural check and re-break on the spot, which reads as
        // debris that can never come to rest.
        let mut w = test_world();
        for x in 0..64 {
            w.set(x, 63, Cell::new(material::BEDROCK, 0));
        }
        failing_slab(&mut w, 20, 10, 6, 3);
        assert!(try_promote_failing_region(&mut w, 20, 10));
        for _ in 0..200 {
            step_chunk_bodies(&mut w);
        }
        assert!(w.chunk_bodies.is_empty(), "test setup: the body should have settled");

        let span = w.materials.get(material::STONE).max_unsupported_span;
        for x in 0..64 {
            for y in 0..64 {
                let c = w.get(x, y);
                if c.material == material::STONE {
                    assert!(c.aux() <= span, "a landed cell kept its pre-flight distance at ({x}, {y}), so it will re-break immediately");
                    assert!(!c.attached(), "landed debris must not re-attach to the background, or a fallen chunk becomes immovable terrain");
                }
            }
        }
    }

    #[test]
    fn a_body_moving_through_powder_destroys_no_material() {
        // `Reports/coupling-research.md` §4 names this exact trap: "any cell
        // the body moves into must have its contents displaced, not
        // deleted... the engine's conservation tests will catch it only if
        // they are extended to cover rigid bodies, which they currently are
        // not." This is that extension.
        let mut w = test_world();
        for x in 0..64 {
            w.set(x, 63, Cell::new(material::BEDROCK, 0));
        }
        for y in 40..50 {
            for x in 10..50 {
                w.set(x, y, Cell::new(material::SAND, 0));
            }
        }
        let sand_before = count_material(&w, material::SAND);
        failing_slab(&mut w, 20, 10, 6, 3);
        assert!(try_promote_failing_region(&mut w, 20, 10));

        for _ in 0..300 {
            step_chunk_bodies(&mut w);
        }

        assert_eq!(count_material(&w, material::SAND), sand_before, "the body deleted sand it passed through instead of displacing it");
        assert_eq!(total_debris(&w), 18, "the body itself lost cells");
    }

    #[test]
    fn a_detached_shelf_promotes_through_the_real_cascade() {
        // Every promotion test above hands `try_promote_failing_region` a
        // slab whose `aux` is already past the span. This one never touches
        // `aux`: it builds a structure, runs the genuine reactive cascade,
        // and checks a body comes out -- which is the only way to know the
        // trigger fires in play rather than only when a test sets it up to.
        let mut w = World::new(Rect::new(0, 0, 63, 63));
        for x in 0..64 {
            w.set(x, 63, Cell::new(material::BEDROCK, 0));
        }
        // A thick anchored pillar, with a 3-cell-thick shelf off its side.
        // Thicker than stone's confinement diameter and the shelf would
        // anchor itself and hang there forever, which is documented,
        // intended behaviour -- see structural.rs's module doc.
        for y in 30..63 {
            for x in 5..15 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for y in 40..43 {
            for x in 15..40 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        crate::sim::structural::compute_world_distances(&mut w);

        // Cut the shelf off, through the same eraser path a player uses.
        for y in 40..43 {
            w.paint_capsule((15, y), (15, y), 0, material::EMPTY, 1.0);
        }

        let mut peak_bodies = 0;
        let mut peak_cells = 0;
        for _ in 0..400 {
            w.begin_step();
            crate::sim::scheduler::step(&mut w);
            w.end_step();
            step_chunk_bodies(&mut w);
            peak_bodies = peak_bodies.max(w.chunk_bodies.len());
            peak_cells = peak_cells.max(w.chunk_bodies.iter().map(|b| b.cells.len()).sum::<usize>());
        }

        assert!(peak_bodies > 0, "a fully detached shelf never promoted to a chunk body");
        assert!(
            peak_cells >= MIN_BODY_CELLS,
            "the promoted body was smaller than the minimum chunk size, so it should not have promoted at all"
        );
        assert!(w.chunk_bodies.is_empty(), "the body never settled back into the grid");
    }

    #[test]
    fn a_strike_throws_pieces_out_of_solid_rock() {
        // The verb. Erasing removes support but delivers no load, so before
        // this nothing could fail from being *hit* -- a mechanic that worked
        // and still felt inert (`Reports/design-philosophy.md` §0a).
        //
        // Struck into the middle of a solid attached mass, so nothing here
        // is failing for structural reasons: any piece that leaves is
        // leaving because it was hit.
        let mut w = test_world();
        for y in 10..50 {
            for x in 10..50 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        assert!(w.chunk_bodies.is_empty());

        strike(&mut w, 30, 30, 8, 6.0);

        assert!(!w.chunk_bodies.is_empty(), "a strike into solid rock threw no pieces at all");
        // Thrown *outward*: every body should be moving away from where it
        // was hit, not merely dropped.
        for body in &w.chunk_bodies {
            let (dx, dy) = (body.x - 30.0, body.y - 30.0);
            let outward = body.vx * dx + body.vy * dy;
            assert!(outward > 0.0, "a fragment at ({}, {}) was thrown toward the blow, not away from it", body.x, body.y);
        }
        // And it left a wound rather than only loosening things.
        assert_ne!(w.get(30, 30).material, material::STONE, "the strike did not pulverize the cells it landed on");
    }

    /// How far the furthest fracture reaches from `(cx, cy)`.
    fn crack_reach(w: &World, cx: i32, cy: i32) -> i32 {
        let mut furthest = 0;
        for x in 0..64 {
            for y in 0..64 {
                if w.get(x, y).cracked() {
                    let d2 = (x - cx) * (x - cx) + (y - cy) * (y - cy);
                    furthest = furthest.max((d2 as f32).sqrt() as i32);
                }
            }
        }
        furthest
    }

    #[test]
    fn working_the_same_spot_drives_a_crack_deeper() {
        // Stress concentrates at a crack tip, so a second blow on damaged
        // rock should *extend* the fissure rather than scribble a fresh one
        // beside it. This is what makes damage accumulate instead of merely
        // repeat, and what lets a player work a crack along deliberately
        // rather than chewing through a span.
        // Two independently built worlds rather than a clone: `World` is not
        // `Clone`, and building twice also proves the difference comes from
        // the second blow rather than from any state the first left behind.
        let slab = |w: &mut World| {
            for y in 4..60 {
                for x in 4..60 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
        };
        let mut once = test_world();
        slab(&mut once);
        let mut twice = test_world();
        slab(&mut twice);

        strike(&mut once, 32, 32, 5, 3.0);
        let after_one = crack_reach(&once, 32, 32);

        strike(&mut twice, 32, 32, 5, 3.0);
        strike(&mut twice, 32, 32, 5, 3.0);
        let after_two = crack_reach(&twice, 32, 32);

        assert!(after_one > 0, "a single blow scored no fractures at all");
        assert!(
            after_two > after_one,
            "a second blow on the same spot should drive the fissure further out ({after_one} -> {after_two}), not just re-score the same rock"
        );
    }

    #[test]
    fn struck_rock_stops_being_part_of_the_background_mass() {
        // Loosening is what lets the *cascade* finish the job after the blow
        // -- hit the base of an overhang and it should both throw chips now
        // and bring the overhang down a moment later. Rock that stayed
        // attached would stay braced and never follow.
        let mut w = test_world();
        for y in 10..50 {
            for x in 10..50 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        strike(&mut w, 30, 30, 6, 4.0);

        // Two zones, two claims. Inside the blow the rock is *loosened* --
        // stripped of its backing, so the cascade can finish what the hit
        // started. Beyond it the rock is *scored* -- still part of the
        // massif, but carrying fractures that weaken it and that a later
        // blow can drive deeper. Checking only the first would pass with
        // cracks doing nothing at all.
        let loosened = (10..50)
            .flat_map(|x| (10..50).map(move |y| (x, y)))
            .find(|&(x, y)| {
                let d2 = (x - 30) * (x - 30) + (y - 30) * (y - 30);
                d2 > 4 && d2 <= 16 && w.get(x, y).material == material::STONE
            });
        if let Some((x, y)) = loosened {
            assert!(!w.get(x, y).attached(), "rock inside the blow at ({x}, {y}) is still claiming to be braced by the massif");
        }
        let scored = (10..50)
            .flat_map(|x| (10..50).map(move |y| (x, y)))
            .any(|(x, y)| {
                let d2 = (x - 30) * (x - 30) + (y - 30) * (y - 30);
                d2 > 36 && w.get(x, y).cracked()
            });
        assert!(scored, "the blow scored no fractures beyond the rock it removed, so it cannot be worked at");
    }

    #[test]
    fn promotion_order_is_identical_run_to_run() {
        // The flood fill runs over a `HashSet`, whose iteration order is
        // randomized per process -- issue #7's determinism trap in a new
        // place. The body's cell order decides every fit test and landing
        // write, so it has to be stable.
        let orders: Vec<Vec<(i32, i32)>> = (0..8)
            .map(|_| {
                let mut w = test_world();
                failing_slab(&mut w, 20, 20, 6, 3);
                try_promote_failing_region(&mut w, 20, 20);
                w.chunk_bodies[0].cells.iter().map(|c| (c.dx, c.dy)).collect()
            })
            .collect();
        for (i, order) in orders.iter().enumerate().skip(1) {
            assert_eq!(order, &orders[0], "run {i} produced a different cell order");
        }
    }

    #[test]
    fn disjoint_cells_produce_separate_rings() {
        let cells = [(0, 0), (50, 50)];
        let rings = trace_contours(&cells);
        assert_eq!(rings.len(), 2, "two disjoint single cells should trace as two separate unit squares");
        for ring in &rings {
            assert_eq!(signed_area(ring).abs(), 1);
        }
    }
}
