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
///
/// Shared with `structural::tick`, which uses it as the floor on a confined
/// *severed piece* worth cracking at all: a chip this small has nothing left
/// inside it to separate. Deliberately the same number as the one here, so
/// "too small to come apart into pieces" means one thing in this engine
/// rather than two.
pub(crate) const MIN_FRACTURE_CELLS: usize = 6;

/// Largest a single body may be. Past this one piece stops reading as a
/// chunk broken off a wall and starts reading as the wall gliding intact,
/// and the per-substep fit test gets expensive.
///
/// This is a cap on a *fragment*, not on a collapse — it used to gate
/// whether a region was promoted at all, which meant a large failure
/// declined and dissolved to dust instead. See
/// `try_promote_failing_region`.
const MAX_BODY_CELLS: usize = 400;

// The ceiling on a single collapse (`MAX_FRACTURE_CELLS`) lived here while
// this module decided which cells were failing. It moved with that decision
// to `load::MAX_REGION_CELLS`, which is where a region's size is now
// bounded -- during the accumulation, so a truncated region also reports
// that its load figures are floors rather than totals.

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

/// How far the extra shove at a fracture's own joint reaches, and how much
/// harder it hits than the collapse's broad displacement. Tight and sharp:
/// this exists to give the break a *location*, not to add another blast.
const SNAP_RADIUS: i32 = 4;
const SNAP_PRESSURE_FACTOR: f32 = 0.75;

/// Pressure a landing writes, per square root of cell count per unit of
/// impact speed, and the speed below which it writes none. The floor is
/// what stops a long rubble cascade firing an impulse per settling grain:
/// a piece that merely came loose and dropped a cell has not *landed*.
const LANDING_PRESSURE: f32 = 1.5;
const LANDING_MIN_SPEED: f32 = 1.5;

/// Smallest swing a strike can be, whatever the brush is set to. Sized so
/// the smallest possible blow still takes a visible bite out of a cliff
/// and throws something: at 6 the core is 2 and the chip is 4, which is
/// ~50 cells loosened against `MIN_FRACTURE_CELLS`, and cracks run 18.
const MIN_STRIKE_RADIUS: i32 = 6;

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
///
/// Shared with `structural::walk_fissures`, which needs the identical rule
/// for the identical reason and only on its blast path -- its wander is
/// position-keyed, so a repeat charge retraces its own fissures exactly and
/// would otherwise write nothing at all the second time.
pub(crate) const CRACK_TIP_BONUS: i32 = 2;

/// How many times a single ray may claim `CRACK_TIP_BONUS`. Bounds the walk
/// so working one spot forever cannot drive a fissure across the whole
/// world, while still leaving several blows' worth of room to grow.
pub(crate) const CRACK_TIP_MAX_STEPS: i32 = 6;

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
    /// Fastest this body has ever travelled, in cells per frame.
    ///
    /// Recorded rather than read at landing because by the time `advance`
    /// gives up, the velocity has already been damped by the collisions
    /// that stopped it -- `COLLISION_RETENTION` is 0.3, and a body settles
    /// only after `STALL_FRAMES_BEFORE_SETTLING` frames of going nowhere.
    /// Reading `vy` there would report that everything lands gently, which
    /// is the opposite of what a forty-cell drop should feel like.
    peak_speed: f32,
}

impl ChunkBody {
    /// A body placed directly, at rest. Test-only, and deliberately so:
    /// production bodies only ever come from `try_promote_failing_region`
    /// or `strike`, which derive velocity and spin from the break that
    /// made them, and a constructor that let callers skip that would be a
    /// way to create debris that never broke off anything.
    #[cfg(test)]
    pub fn at(cells: Vec<BodyCell>, x: f32, y: f32) -> Self {
        Self { cells, x, y, vx: 0.0, vy: 0.0, spin: 0.0, stalled: 0, peak_speed: 0.0 }
    }

    /// The same, already falling. Test-only for the same reason.
    #[cfg(test)]
    pub fn falling(cells: Vec<BodyCell>, x: f32, y: f32, vy: f32) -> Self {
        Self { vy, ..Self::at(cells, x, y) }
    }

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

    /// The body's current extent in world space, as
    /// `(min_x, min_y, max_x, max_y)`.
    ///
    /// Exists for the renderer: a body is drawn off-grid on top of the
    /// per-cell pass, so the dirty-rect machinery has no idea it moved and
    /// needs to be told which pixels to repaint.
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for cell in &self.cells {
            let (x, y) = self.cell_position(cell);
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        (x0, y0, x1, y1)
    }

    /// Where `cell` currently sits in world coordinates.
    pub fn cell_position(&self, cell: &BodyCell) -> (i32, i32) {
        (self.x.round() as i32 + cell.dx, self.y.round() as i32 + cell.dy)
    }
}

/// Break an already-decided failing `region` apart into falling pieces,
/// returning whether it did.
///
/// Returns `false` — leaving the caller to fall back to per-cell
/// `breaks_into` conversion — only when the region is too small to be worth
/// fracturing at all.
///
/// # Why this no longer decides *which* cells fail
///
/// It used to. `label_failing_region` flooded "connected, unattached,
/// distance above zero" from the failing cell and took the whole appendage,
/// and that flood existed for exactly one reason: under the reach model the
/// root of a cantilever never failed on its own, so something had to
/// reconstruct the piece that should have come away. It produced roughly
/// the right outcome by the wrong mechanism, and it degraded precisely
/// where it mattered — a partially cracked overhang has no single
/// distance-defined appendage to take.
///
/// `load::failing_region` now answers that question properly, from the
/// support forest: the piece is the subtree the failing cell was actually
/// holding up, or, when nothing holds the cell either, the connected region
/// that has genuinely come free. **Both had to change in the same commit.**
/// Leaving the flood in beside a root that now fails on its own means one
/// failure detaches twice.
///
/// # No upper bound, deliberately
///
/// `MAX_BODY_CELLS` caps a *single* body, and `fracture` splits whatever it
/// is given into fragments of at most 32-128 cells, so a large collapse
/// simply yields more pieces. Gating on it meant anything bigger than 400
/// cells fell through to per-cell conversion and dissolved — the bigger the
/// piece, the more certain it turned to dust, which is exactly backwards
/// and is the same mistake `strike` had. Reported from play as a thick
/// column cap where "part breaks into chunks ... the other parts just
/// crumble to dust". A size cap belongs on a fragment, never on whether a
/// region breaks at all.
pub fn fracture_failing_region(world: &mut World, region: &[(i32, i32)], broke_at: (i32, i32)) -> bool {
    if region.len() < MIN_FRACTURE_CELLS {
        return false;
    }
    fracture(world, region, broke_at);
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
fn fracture(world: &mut World, region: &[(i32, i32)], broke_at: (i32, i32)) {
    // **A big collapse calves blocks; a small one chips.** This passed a
    // flat `0` for as long as the ladder has existed, which meant a
    // structural collapse drew fragment targets of 2..64 cells while a
    // *blow* of the same material drew 8..256 -- so everything that came
    // down after the event was systematically smaller than what the event
    // itself threw, and two of the six rungs landed under `MIN_BODY_CELLS`
    // and became grit before shape was even considered.
    //
    // Reported from play as the second of three complaints off the rolling
    // world: *"could the pattern of cracks be more heterogeneous, so the
    // chunks that break off are different sizes"*. The joint fabric is one
    // half of that answer and this is the other, and this half is the one
    // that was simply never wired: `size_bias` already existed, already
    // took the right shape, and only `strike` and the blast shell were
    // passing it anything.
    //
    // Sized off the region's own half-extent rather than a cell count, for
    // the same reason `strike` sizes off the brush radius: what the ladder
    // is asking is "how big is the thing that came apart", and a long thin
    // shelf and a compact block of the same area do not come apart the
    // same. Half-extent puts a collapse on exactly the scale a blow of that
    // reach would be on, which keeps one ladder rather than inventing a
    // second calibration.
    let half = region
        .iter()
        .fold((i32::MAX, i32::MIN, i32::MAX, i32::MIN), |(x0, x1, y0, y1), &(x, y)| (x0.min(x), x1.max(x), y0.min(y), y1.max(y)));
    let extent = ((half.1 - half.0) / 2).max((half.3 - half.2) / 2);
    fracture_with_impulse(world, region, None, size_bias(extent), Some(broke_at))
}

/// As `fracture`, but every fragment is thrown away from `origin` at
/// `force`. This is what makes a *blow* different from a collapse: the same
/// rock comes apart either way, but struck rock leaves the wound rather than
/// sagging out of it.
fn fracture_with_impulse(world: &mut World, region: &[(i32, i32)], impulse: Option<((f32, f32), f32)>, size_bias: u32, broke_at: Option<(i32, i32)>) {
    // Every destructive event owes feedback (`Reports/design-philosophy.md`
    // §0a), and a collapse is one. `break_free` has always written this for
    // a single cell converting to rubble; a region coming apart into chunks
    // is a louder version of the same event and was silently writing
    // nothing, so a structure that failed by *cracking* -- now the common
    // case -- shoved no air at all while one that crumbled did.
    if !region.is_empty() {
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for &(x, y) in region {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        // Two impulses, because a collapse is two events and they happen
        // in different places. The whole piece displaces air as it goes --
        // sized to *reach* all of it, since sizing from the cell count left
        // the far end of anything long and thin outside the impulse
        // entirely, and a 22-cell cantilever shoved no air at its own tip.
        let radius = (((x1 - x0) / 2).max((y1 - y0) / 2) + 1).max(2);
        let strength = (region.len() as f32).sqrt() * COLLAPSE_PRESSURE;
        world.add_pressure_impulse((x0 + x1) / 2, (y0 + y1) / 2, radius, strength);
        // And the fracture itself is a sharp, *local* event at the joint
        // that gave way. Without this the collapse has no origin the eye
        // can find: `World::add_pressure_impulse` paints a flat disc with
        // no falloff, so the region-wide shove above is identical at the
        // neck and a hundred cells out along the piece that fell. Reported
        // as the difference between "the neck snapped" and "some rock
        // fell" -- see `Reports/destruction-plan.md` A3.
        //
        // `broke_at` is `None` for a blast, which has no single failing
        // cell; `strike` and `fracture_shell` pass their own origin, which
        // is the same idea for a different cause.
        if let Some((ox, oy)) = broke_at {
            world.add_pressure_impulse(ox, oy, SNAP_RADIUS, strength * SNAP_PRESSURE_FACTOR);
        }
    }
    let mut remaining: Vec<(i32, i32)> = region.to_vec();
    remaining.sort_unstable(); // deterministic seed order, as `label_failing_region` already guarantees
    let mut left: HashSet<(i32, i32)> = remaining.iter().copied().collect();

    for &seed in &remaining {
        if !left.contains(&seed) {
            continue;
        }
        // A power-of-two ladder. Uniform over the exponent means each
        // doubling is half as likely per cell of material consumed, which
        // is the heavy-tailed shape fragmentation actually has.
        //
        // How many rungs is the *material's* business now
        // (`MaterialDef::fragment_rungs`), not a constant: slate shears
        // into plates where a brittle crust shatters into grit, and
        // "different materials should break differently" is an explicit
        // near-term goal. Read from the seed cell, since a mixed region
        // should break the way the rock at each seed does.
        //
        // A wider blow takes bigger pieces off. `size_bias` shifts the
        // whole ladder up, so a heavy swing or a large blast calves slabs
        // where a light tap produces chips -- without flattening the
        // distribution, which is what stops it reading as all-or-nothing
        // again.
        let rungs = world.materials.get(world.get(seed.0, seed.1).material).fragment_rungs;
        let target = (1usize << (1 + world.rng.below(rungs) as usize + size_bias as usize)).min(MAX_BODY_CELLS);
        let fragment = take_fragment(world, &mut left, seed, target);
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
///
/// **A fissure is a fragmentation seam.** The flood refuses to cross a
/// cracked edge, so a collapsing region comes apart along the cracks that
/// are already in it rather than along BFS ring boundaries. Before this,
/// cracks severed *support* (`load.rs` asks `edge_is_cracked` everywhere)
/// and were then completely ignored when the thing they had undermined
/// actually broke: the flood ran straight across them and the only random
/// thing about a fragment was its size, never its shape. Whole collapses
/// therefore came down as L1 rings -- "perfect columns or sharp
/// triangles", the owner's second complaint off the prototype sheets.
///
/// Costs one bit test per admitted cell, at event time only.
///
/// A fragment cut small by dense cracking falls through to
/// `shatter_to_rubble` at the call site, which is the graded outcome and
/// not a fallback: heavily fissured rock *should* come away as grit while
/// lightly fissured rock comes away as slabs.
fn take_fragment(world: &World, left: &mut HashSet<(i32, i32)>, seed: (i32, i32), target: usize) -> Vec<(i32, i32)> {
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
            if !left.contains(&next) {
                continue;
            }
            // Tested *before* the claim, never after: `left.remove` is how
            // a cell is claimed for this fragment, so removing it and then
            // declining it would delete material from the region entirely
            // -- neither in a fragment nor in the pool for the next seed,
            // which is precisely what the conservation assertions exist to
            // catch. `edge_is_cracked` owns each edge from one side (see
            // its doc), so the step direction is handed to it as-is and it
            // asks the neighbour about its own bit when stepping left or
            // up.
            if super::structural::edge_is_cracked(world, x, y, dx, dy) {
                continue;
            }
            left.remove(&next);
            stack.push_back(next);
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
        peak_speed: 0.0,
    });
    // Counted here, on the line after the push, and not at any call site:
    // this is the only place in the engine where rock leaves the grid as a
    // moving piece, so it is the only place the count cannot drift out of
    // step with what happened. `fracture_with_impulse` (collapse),
    // `calve_collar` and `fracture_shell` (blast) all arrive through this
    // door, and so will whatever is added next.
    //
    // It is also the *only* field of `FailureCounts` that measures a
    // displacement. Everything else is recorded at `structural.rs`'s
    // `record`, which runs before the free-face test, the erosion, the
    // slicing and the fracture -- so `unsupported: 400` is entirely
    // consistent with nothing at all having moved, which is exactly the
    // shape of the owner's "no pieces move, ever" against a harness
    // reporting hundreds of failures.
    world.structural_failures.record_promoted(cells.len());
}

/// Convert one cell to its material's `breaks_into`, losing attachment —
/// whatever comes free is no longer backed by the mass it broke out of.
///
/// `pub(crate)` for `explosion::JointSeams`, which opens a joint into a seam
/// of void *and grit* and wants the same conversion, the same attachment
/// loss and — importantly — the same `record_shattered` bookkeeping. Grit
/// counted anywhere else would not appear in the promoted-to-shattered ratio
/// that says whether a break has a *distribution* of sizes behind it.
pub(crate) fn shatter_to_rubble(world: &mut World, x: i32, y: i32) {
    let cell = world.get(x, y);
    let Some(into) = world.materials.get(cell.material).breaks_into else {
        return; // no configured debris: leave it rather than deleting content
    };
    let shades = world.materials.get(into).palette.len().max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    let temp = cell.temperature();
    world.set(x, y, Cell::new(into, shade).with_temperature(temp));
    // After the conversion, never before the `breaks_into` check above:
    // the decline leaves the cell exactly as it was, and counting a
    // decline would make grit look like it happened. Grit is half of the
    // "a few blocks, more cobbles, a lot of grit" distribution and its
    // pair, `promoted_cells`, is the other -- neither number says anything
    // about the shape of a break on its own.
    world.structural_failures.record_shattered(1);
    world.schedule_structural_check_around(x, y);
}


/// Squared distance from a point to the segment `a..b`, in cell units.
///
/// Integer throughout: this decides which cells a cut removes, and
/// determinism is required, so it must not depend on float rounding.
fn distance2_to_segment(p: (i32, i32), a: (i32, i32), b: (i32, i32)) -> i32 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let (apx, apy) = (p.0 - a.0, p.1 - a.1);
    let len2 = abx * abx + aby * aby;
    if len2 == 0 {
        return apx * apx + apy * apy; // degenerate: a plain disc about `a`
    }
    // Project onto the segment and clamp to it, keeping the parameter as a
    // rational `t = dot / len2` so nothing leaves integer arithmetic.
    let dot = (apx * abx + apy * aby).clamp(0, len2);
    let (qx, qy) = (a.0 * len2 + abx * dot, a.1 * len2 + aby * dot);
    let (dx, dy) = (p.0 * len2 - qx, p.1 * len2 - qy);
    // Divide the squared distance back out of the squared scale.
    ((dx as i64 * dx as i64 + dy as i64 * dy as i64) / (len2 as i64 * len2 as i64)) as i32
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
///
/// Returns how many distinct cells were scored with a fresh fissure —
/// `explosion.rs`'s blast report needs a count for its "did it fire at all"
/// line (`CLAUDE.md`: a discrete event needs a counter, not just a
/// picture), and `scored_now`'s own dedup is exactly that count, already
/// computed for the crack-tip-bonus logic below. `pub(crate)` rather than
/// private: the blast's crack halo (R1,
/// `Reports/explosion-stone-review.md` §4) calls this from `explosion.rs`,
/// the one other caller in the crate besides `strike`/`mine_swept` here.
pub(crate) fn score_cracks(world: &mut World, cx: i32, cy: i32, from: i32, length: i32, rays: u32) -> u32 {
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
            // A fissure is where the rock has parted company with the mass
            // behind the slice, so it stops claiming to be braced by it.
            // This is what carries a blow's *reach* into the structural
            // model: the bite is 4 cells wide and the cracks run 21, and
            // before this only the bite loosened anything -- so the cracks
            // were visible damage that changed nothing about what the rock
            // could carry. See `structural::detach_around_crack`.
            super::structural::detach_around_crack(world, x, y);
            world.schedule_structural_check_around(x, y);
            r += 1;
        }
    }
    scored_now.len() as u32
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
/// Cut rock away precisely, and *tell the structure about it*.
///
/// # Why the eraser is not this
///
/// `Reports/design-philosophy.md` §0a records the original sin of this
/// whole subsystem: destruction could only be provoked by **erasing**
/// support, and "an eraser delivers no load and no impulse, so nothing
/// ever failed from being struck." `strike` fixed that for blunt force
/// and nothing fixed it for *precise removal* -- so the eraser is still a
/// magic wand that deletes matter with no consequence, which is the one
/// destructive verb in the engine that a player can use without the
/// structural model noticing.
///
/// That gap became urgent the moment rooms did: carving a doorway through
/// a wall is precise removal, and doing it with the eraser means the
/// lintel above never learns it is now spanning a gap.
///
/// So this is `strike`'s pipeline at a smaller, aimed scale: it takes the
/// bite, it loosens the rock around the wound, it scores short cracks, it
/// shoves the air, and what it frees is handed to `fracture` rather than
/// deleted. Deliberately *not* a blast -- no minimum radius, no thrown
/// slabs, no cracks running twenty cells. A chisel, not a hammer.
///
/// # `spoil_yield`, and why it lives here rather than in one caller
///
/// The fraction of the bite that stays behind as rubble; the rest leaves
/// the world. It is here because **every digger has to share one spoil
/// model**, and for a while they did not: `player::dig` mined and then
/// thinned by `Tuning::dig_yield`, while `App::mine` mined and took
/// whatever fell out. So the gnome honoured the knob and the `D` key
/// ignored it -- the same one-number-two-verbs shape as `brush_radius`,
/// and a real problem now that the gnome and the creatures are the things
/// that actually excavate: they and the sandbox verb would have dug
/// different holes in the same rock.
///
/// **This is a parameter and not a constant on purpose.** A blanket
/// vanish written into this function was measured and reverted: it breaks
/// `player::at_full_yield_nothing_leaves_the_world`, whose contract is
/// that at yield 1.0 a dig may *move* material but never delete it, and
/// which is the promise that keeps the eraser from creeping back in
/// through the mining verb. Vanishing is `spoil_yield = 0.0`, a setting,
/// not a rule -- see `player::SPOIL_MODES`.
///
/// Returns how many cells actually left the world, which is what a caller
/// needs to report a bite: it is not derivable from the radius, because
/// the disc may be part air already and because a material with no
/// `breaks_into` is declined rather than deleted.
pub fn mine(world: &mut World, cx: i32, cy: i32, radius: i32, spoil_yield: f32) -> usize {
    mine_swept(world, (cx, cy), (cx, cy), radius, spoil_yield)
}

/// `mine`, cutting the **capsule** swept from `from` to `to` rather than a
/// disc at a point.
///
/// # Why a bore has to be swept and not stamped
///
/// A disc of radius r is `2r+1` tall only on its exact centre line, and
/// narrower everywhere else. A tunnel driven as a row of discs therefore
/// has a scalloped roof and floor, and its *usable* height is the scallop,
/// not the disc. The owner spotted the shape on sight -- "why are you
/// digging tunnels a row of circles instead of a tunnel" -- and it had
/// already cost more than looks: the gnome is `PLAYER_HEIGHT` 14 cells
/// tall, a radius-7 bite is 15 cells at its middle, and the pinch between
/// two bites measured **13**. He dug a bore, walked to the mouth of it and
/// stopped there for good, wedged between a floor bump and a ceiling in a
/// passage his own height. Dumped at frame 400: rock at y=299, air 300 to
/// 312, rock at 313.
///
/// Sweeping the bite from the last one to this one fills exactly that
/// pinch, so a run of bites is a corridor of constant `2r+1` height with
/// no scallops in it at all.
///
/// The distance is capped by the caller (see `player::dig`): a swept cut
/// between two points far apart would carve a trench across everything
/// between them, which is right for a continuing bore and wrong for a
/// digger who has just walked somewhere else.
pub fn mine_swept(world: &mut World, from: (i32, i32), to: (i32, i32), radius: i32, spoil_yield: f32) -> usize {
    let radius = radius.max(1);
    let (cx, cy) = to;
    // The swept region's bounding box, then a point-to-segment test per
    // cell inside it. Cheap enough at these radii, and it degenerates to
    // exactly the old disc when `from == to`.
    let (lo_x, hi_x) = (from.0.min(cx) - radius, from.0.max(cx) + radius);
    let (lo_y, hi_y) = (from.1.min(cy) - radius, from.1.max(cy) + radius);
    let mut loosened = Vec::new();
    for y in lo_y..=hi_y {
        for x in lo_x..=hi_x {
            if distance2_to_segment((x, y), from, to) > radius * radius || !world.in_bounds(x, y) {
                continue;
            }
            if !is_body_material(world, x, y) || world.get(x, y).organism_id() != 0 {
                continue;
            }
            shatter_to_rubble(world, x, y); // the cut itself: material, not vacuum
            loosened.push((x, y));
        }
    }
    if loosened.is_empty() {
        return 0;
    }
    // Short, so a chisel weakens what it is cutting through without
    // fissuring the whole wall the way a swing does. Return value (cells
    // scored) ignored here -- only the blast report in `explosion.rs` needs
    // a count.
    score_cracks(world, cx, cy, radius, radius + MINE_CRACK_REACH, MINE_CRACK_RAYS);
    // The rock around the cut stops being braced, which is what makes a
    // doorway's lintel notice the doorway.
    for &(x, y) in &loosened {
        super::structural::detach_exposed_neighbours(world, x, y);
        world.schedule_structural_check_around(x, y);
    }
    world.add_pressure_impulse(cx, cy, radius.max(2), loosened.len() as f32 * MINE_PRESSURE);
    // A cut is a disturbance: it licenses failures near it. See
    // `World::chain_reach`. The extent is the chisel's own outer crack
    // reach -- the same `radius + MINE_CRACK_REACH` handed to `score_cracks`
    // above, so the licence covers exactly the rock this cut damaged and no
    // more (`structural::Disturbance::extent`).
    world.record_disturbance(cx, cy, radius + MINE_CRACK_REACH);
    // Last, so everything above sees the full bite as rubble. That
    // ordering is not incidental: it is exactly what `player::dig` did
    // when the thinning lived there, so moving it in here changes the
    // gnome's behaviour by nothing at all while giving the `D` key the
    // model it never had.
    thin_to_spoil(world, &loosened, spoil_yield)
}

/// Keep a `spoil_yield` fraction of the freshly broken cells and let the
/// rest leave the world.
///
/// # Why a dig has to remove volume at all
///
/// `mine` conserves cells -- rock becomes rubble in place -- so without
/// this a bore holds exactly the volume the rock did and nothing has been
/// dug. Reported from the second playtest in those words: "the material
/// breaks but goes nowhere, so you cannot really make a cave." Shoving the
/// pieces aside only works while there is somewhere to shove them, and
/// inside a massif there is not.
///
/// # Why an even stride and not a random subset
///
/// Two reasons, and the second is the one that matters. A random subset
/// clumps, so a bore comes out with lumps of surviving rubble in some
/// places and clean holes in others, which reads as the cut having missed.
/// And a stride consumes no RNG at all, keeping the dig out of
/// `world.rng`'s draw order entirely, so a replayed input sequence cannot
/// diverge here -- determinism is required, per `PLAN.md`.
///
/// # Why the removed cells are not thrown as particles
///
/// That was tried first, for exactly the right reason: material blinking
/// out of existence is the "no debris, no consequence" failure this
/// project has rejected before. It does not work. `ParticleSystem`
/// particles *land*, writing themselves back into the grid, so the puff
/// quietly undid the removal -- measured on `scene=tunnel`, 941 cells
/// dusted and 119 actually gone from the world. The debris requirement is
/// met by the kept fraction instead, which is real rubble at the digger's
/// feet, plus the pressure impulse above for smoke and grit to react to.
fn thin_to_spoil(world: &mut World, fresh: &[(i32, i32)], spoil_yield: f32) -> usize {
    let total = fresh.len();
    if total == 0 {
        return 0;
    }
    let keep = ((total as f32) * spoil_yield.clamp(0.0, 1.0)).round() as usize;
    if keep >= total {
        return 0; // yield 1.0: a dig may move material, never delete it
    }
    let mut dusted = 0usize;
    let mut kept_so_far = 0usize;
    for (i, &(x, y)) in fresh.iter().enumerate() {
        // Bresenham-style: keep this one if the running quota crosses an
        // integer here. Spreads `keep` survivors evenly across `total`.
        let quota = (i + 1) * keep / total;
        if quota > kept_so_far {
            kept_so_far = quota;
            continue;
        }
        if world.materials.kind(world.get(x, y).material) != MaterialKind::Powder {
            continue; // `shatter_to_rubble` declined this one (no `breaks_into`)
        }
        world.set(x, y, Cell::EMPTY);
        dusted += 1;
    }
    dusted
}

/// How far a chisel's fissures run past its own bite, and how many. Both
/// small: this is the precise verb, and cracks that ran like a blow's
/// would make careful work impossible.
const MINE_CRACK_REACH: i32 = 2;
const MINE_CRACK_RAYS: u32 = 3;

/// Pressure a cut writes, per cell removed. Enough that smoke and grit
/// react, far below a blow.
const MINE_PRESSURE: f32 = 0.4;

pub fn strike(world: &mut World, cx: i32, cy: i32, radius: i32, force: f32) {
    // A blow has a floor, and the brush does not.
    //
    // Reported from play as "striking a cliff does nothing", with the
    // brush at R2 -- which is a perfectly good size for *drawing* and an
    // absurd one for *hitting*. At radius 2 the arithmetic below gives a
    // core of 1 and a chip of 2: about five cells pulverized, eight
    // loosened, and cracks reaching six cells. All of that is correct, and
    // on the face of a cliff it is invisible.
    //
    // Scaling the swing off the brush was a deliberate choice -- "the tool
    // the player is already sizing is the tool that decides how hard they
    // hit, rather than introducing a second invisible number" -- and it is
    // still right at the top end, where a wide brush should calve slabs.
    // It was wrong to let it go all the way down, because the two tools
    // want different sizes: you draw with a pencil and you swing a hammer.
    // So the brush still scales the blow, from a floor that always lands.
    let radius = radius.max(MIN_STRIKE_RADIUS);
    // A blow is a disturbance: it licenses failures near it. See
    // `World::chain_reach`. The extent is the swing's own crack reach --
    // the same `radius * CRACK_REACH` handed to `score_cracks` below, so
    // the licence covers exactly the rock this blow scored
    // (`structural::Disturbance::extent`).
    //
    // **Recorded after the floor, not before it.** `radius` is rebound one
    // line up, and a licence taken from the unfloored brush radius would be
    // narrower than the swing that actually lands -- which is the same
    // mistake as recording a point for a volume, one argument along.
    world.record_disturbance(cx, cy, radius * CRACK_REACH);
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
    // reach is the whole point of striking rather than erasing. Return
    // value (cells scored) ignored here -- only the blast report in
    // `explosion.rs` needs a count.
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
        fracture_with_impulse(world, &loosened, Some(((cx as f32, cy as f32), force)), size_bias(radius), Some((cx, cy)));
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
///
/// `confinement` is R2's probe result (`explosion::probe_confinement`, via
/// `explosion::Confinement`) — **required, not optional** (`Reports/
/// explosion-stone-review.md` §5). Without gating this loop too, a
/// "contained" sector would still get its rim unattached and fractured by
/// the call below even though `clear_annulus` never cleared a cell in it,
/// which quietly reproduces the self-refilling bruise the containment gate
/// exists to remove. Only `explosion.rs` calls this today, so the argument
/// is always real confinement data, never a "no gating" sentinel.
pub fn fracture_shell(world: &mut World, origin: (i32, i32), inner: i32, outer: i32, force: f32, size_bias: u32, confinement: super::explosion::Confinement) {
    let loosened = loosen_shell(world, origin, inner, outer, confinement, ShellSectors::Open);
    if loosened.len() >= MIN_FRACTURE_CELLS {
        fracture_with_impulse(world, &loosened, Some(((origin.0 as f32, origin.1 as f32), force)), size_bias, Some(origin));
    }
}

/// Which half of a blast's ring a shell scan is interested in.
///
/// The blast's own stages only ever want `Open` — a contained sector's rim
/// must stay exactly where it is, which is the whole of R2. The calving
/// collar wants both, in two different places: the open rim it can throw
/// into the void, and the *contained* pocket wall, which is the only rock a
/// fully buried charge has anywhere to move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellSectors {
    /// Sectors that vented — reach equals the blast radius.
    Open,
    /// Sectors that read as buried, cleared only to the crush pocket.
    Contained,
}

/// Break a collar off the rim along the cracks, once the blast's fissure
/// star has finished growing. Returns how many cells were handed to the
/// fragmenter.
///
/// This is `fracture_shell`'s sibling and shares its scan
/// (`loosen_shell`), and the difference between them is *when* and *which
/// way*: the shell fracture runs on every stage, on the rock the blast is
/// breaking, thrown outward. This runs once, seconds later, on standing
/// rock the star has just cut wedges out of, thrown inward (the caller
/// passes a negative force) so the wedges tumble into the hole.
///
/// The seams are the cracks: `take_fragment` will not flood across a cracked
/// edge, so what comes away is bounded by the fissures the player watched
/// race outward rather than by BFS rings. That is the entire reason the
/// calving waits for the star to finish instead of happening on the bang
/// frame — before the walks have run there is nothing to break *along*.
#[allow(clippy::too_many_arguments)]
pub fn calve_collar(
    world: &mut World,
    origin: (i32, i32),
    inner: i32,
    outer: i32,
    force: f32,
    size_bias: u32,
    confinement: super::explosion::Confinement,
    sectors: ShellSectors,
) -> u32 {
    let loosened = loosen_shell(world, origin, inner, outer, confinement, sectors);
    if loosened.len() < MIN_FRACTURE_CELLS {
        // Below the fragmenter's own floor, and deliberately nothing else:
        // converting a handful of cells to rubble here would make a blast
        // with almost no rim to give eat a little of it anyway, every time.
        return 0;
    }
    fracture_with_impulse(world, &loosened, Some(((origin.0 as f32, origin.1 as f32), force)), size_bias, Some(origin));
    loosened.len() as u32
}

/// The annulus scan both `fracture_shell` and `calve_collar` run: collect
/// the body cells in `inner..outer` whose sector matches `sectors`,
/// unattaching each as it goes.
///
/// One function rather than two copies of the loop for the reason this file
/// keeps re-learning: the sector test, the raggedness and the loosening rule
/// all have to agree with `Blast::clear_annulus` about where this blast's
/// edge is, and a second copy would drift from that agreement the first time
/// either changed.
fn loosen_shell(
    world: &mut World,
    origin: (i32, i32),
    inner: i32,
    outer: i32,
    confinement: super::explosion::Confinement,
    sectors: ShellSectors,
) -> Vec<(i32, i32)> {
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
            // A contained sector's shell stays put -- see this function's
            // own doc. Asked through the same `ragged_sector_limit`
            // `clear_annulus` used to decide what actually cleared, so the
            // two never disagree about where this blast's edge is: the
            // reach is the smoothed one, and the per-cell raggedness is the
            // same draw, so the thrown shell's boundary is the crater's own
            // boundary rather than a clean 22.5-degree pie cut laid over a
            // ragged hole.
            let is_open = super::explosion::ragged_sector_limit(&confinement.sector_reach, dx, dy, x, y) >= confinement.radius as f32;
            if is_open != (sectors == ShellSectors::Open) {
                continue;
            }
            let cell = world.get(x, y);
            if !is_body_material(world, x, y) || cell.organism_id() != 0 {
                continue;
            }
            // **A blast loosens what it fractures**, exactly as a blow
            // does -- `strike` unattaches its whole chip zone before
            // handing it to `fracture`, and this had no equivalent.
            //
            // This used to skip `cell.attached()` outright, on the
            // reasoning that the rim "has already been through
            // `detach_exposed_neighbours` and is genuinely no longer
            // braced". That was true only while everything a player built
            // was unattached to begin with. Now that undamaged material is
            // *intact* by default (`Reports/building-rethink.md`), the only
            // cells that predicate accepted were the narrow
            // `DETACH_DEPTH` band around the crater -- so a blast against a
            // solid structure cleared its hole and threw almost nothing.
            // Reported from play as "explosions don't work well with these
            // more solid structures".
            //
            // Being inside a blast shell *is* the damage. Loosening here
            // says so, and leaves the rest of the structure intact to be
            // judged on its own merits by the load model afterwards.
            if cell.attached() {
                world.set(x, y, cell.with_attached(false));
            }
            loosened.push((x, y));
        }
    }
    loosened.sort_unstable(); // deterministic seed order
    loosened
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
    body.peak_speed = body.peak_speed.max((body.vx * body.vx + body.vy * body.vy).sqrt());

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
    // Built once per call and lent downward, rather than rebuilt inside
    // `displace` for every cell that needs shoving.
    //
    // Reported from play: "when something big breaks into lots of little
    // pieces, the performance gets bad." This was one of the reasons. The
    // set is the body's own cells, so building it per displaced cell made
    // a single substep O(cells²) in hashing *and* allocated a fresh
    // `HashSet` each time -- and the fracture work multiplied how many
    // bodies are in flight at once by roughly eight, so a collapse paid
    // that cost a hundred times a frame.
    let occupied: HashSet<(i32, i32)> = body.cells.iter().map(|c| body.cell_position(c)).collect();

    for cell in &body.cells {
        let (tx, ty) = (ox + cell.dx, oy + cell.dy);
        let (cx, cy) = body.cell_position(cell);
        if (tx, ty) == (cx, cy) {
            continue; // this cell is not actually changing position this substep
        }
        if !world.in_bounds(tx, ty) || !clear_or_displaceable(world, &occupied, tx, ty) {
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
fn clear_or_displaceable(world: &mut World, occupied: &HashSet<(i32, i32)>, x: i32, y: i32) -> bool {
    // A cell the body itself currently occupies is not an obstacle: bodies
    // are lifted out of the grid on promotion, so anything found here
    // belongs to the world, not to this body -- but a *neighbouring* body
    // cell's destination may legitimately be this one mid-move.
    if world.is_empty(x, y) {
        return true;
    }
    let kind = world.materials.kind(world.get(x, y).material);
    if !matches!(kind, MaterialKind::Powder | MaterialKind::Liquid | MaterialKind::Gas) {
        return false; // solid, plant, creature -- a real obstruction
    }
    if displace(world, occupied, x, y) {
        return true;
    }
    // **Gas never blocks, even when there is nowhere to shove it.** Powder
    // and liquid that cannot be moved are treated as solid, because deleting
    // them would manufacture a hole in the world's material budget
    // (`a_body_moving_through_powder_destroys_no_material`). A gas has no
    // such claim: it is already the one kind the engine removes on its own
    // (`Material::dissipation`, `update.rs`), and the alternative here is a
    // boulder resting on smoke.
    //
    // Reported from play as *"chunks of rock that seem fully cracked all the
    // way around stay put and don't fall into the leftover hole/crater"*, and
    // this was one of the causes. `Tuning::smoke_fraction` backfills 18% of a
    // cleared crater with `SMOKE`, so the hole a chunk is meant to fall into
    // is the one place in the world densely full of gas -- and a fresh crater
    // has no empty cell within `DISPLACE_SEARCH` to shove it to, so
    // `displace` fails there almost every time. The chunk moved no substep,
    // hit `STALL_FRAMES_BEFORE_SETTLING` and re-embedded roughly where it
    // started.
    //
    // Measured, paired, `blast=300,45,20,180,60` on rolling seed 1, against
    // the `smoke=0` control the same commit adds to `filmstrip`: at frame 80
    // the shipped setting had **1 body / 10 cells** still in flight against
    // the control's **6 bodies / 100 cells**, and peak concurrent bodies ran
    // 8 against 16. The airborne mass collapsed between frames 74 and 80 --
    // 174 -> 105 -> 97 -> 10 -- exactly where the smoke backfill lands.
    if kind == MaterialKind::Gas {
        world.set(x, y, Cell::EMPTY);
        return true;
    }
    false
}

/// Shove the loose cell at `(x, y)` to the nearest empty cell that the body
/// is not about to occupy, returning whether it found somewhere.
///
/// Ring by ring, so material surfaces at the nearest opening rather than
/// teleporting to whichever cell scan order happened to reach first — the
/// same shape, and the same reasoning, as `particle::land`'s own
/// nearest-empty search.
fn displace(world: &mut World, occupied: &HashSet<(i32, i32)>, x: i32, y: i32) -> bool {
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
///   support anything, and nothing could be brought down by it. This is
///   also the one re-check `load::powder_surcharge` relies on: a settled
///   body is the bounded "something heavy just arrived on top of you"
///   event, and the powder sweep itself schedules nothing. Every cell is
///   scheduled, not just the footprint row, which is the superset the
///   surcharge needs — do not narrow it to the bottom row as an
///   optimisation without re-reading that function's re-check note.
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

    // Landing owes feedback, and paid none until now.
    //
    // `Reports/design-philosophy.md` §0a: a destructive event that shoves
    // no air and marks nothing is not finished. `break_free` writes an
    // impulse per broken cell and `fracture_with_impulse` writes one per
    // collapse, but the *landing* -- a slab falling forty cells and
    // arriving -- wrote nothing at all. The most visible moment of a
    // collapse was the only silent one.
    //
    // Scaled by how hard it actually hit and by how much of it there is,
    // so a pebble tapping down is not a shelf arriving. A body that was
    // only ever nudged (a fragment that came loose and settled a cell
    // lower) writes nothing, which is what keeps this from firing
    // constantly during a long rubble cascade.
    let speed = body.peak_speed;
    if speed < LANDING_MIN_SPEED {
        return;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for cell in &body.cells {
        let (x, y) = body.cell_position(cell);
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let radius = (((x1 - x0) / 2).max((y1 - y0) / 2) + 1).max(2);
    let strength = (body.cells.len() as f32).sqrt() * speed * LANDING_PRESSURE;
    world.add_pressure_impulse((x0 + x1) / 2, y1, radius, strength);
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
    /// A rectangle of loose stone, returned as the region a failure would
    /// hand to `fracture_failing_region`.
    ///
    /// Deciding *which* cells fail is no longer this module's job -- it
    /// moved to `load::failing_region`, which is where the criterion lives
    /// -- so these tests supply the region rather than provoking one. The
    /// stamped-on `aux` is now needed by exactly one of them
    /// (`a_settled_body_does_not_immediately_re_break`, which has to be
    /// able to *see* that landing resets it) and is kept only for that.
    fn loose_slab(w: &mut World, x0: i32, y0: i32, width: i32, height: i32) -> Vec<(i32, i32)> {
        let stale = w.materials.get(material::STONE).max_unsupported_span + 1;
        let mut region = Vec::new();
        for y in y0..(y0 + height) {
            for x in x0..(x0 + width) {
                w.set(x, y, Cell::new(material::STONE, 0).with_aux(stale));
                region.push((x, y));
            }
        }
        region.sort_unstable();
        region
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
        let region = loose_slab(&mut w, 20, 20, 6, 3); // 18 cells, inside MIN..=MAX
        assert!(fracture_failing_region(&mut w, &region, region[0]), "a slab this size should promote");

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
        let region = loose_slab(&mut w, 10, 10, 20, 12); // 240 cells, plenty to break up
        assert!(fracture_failing_region(&mut w, &region, region[0]));

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
        let region = loose_slab(&mut w, 20, 20, 2, 2); // 4 cells
        assert!(!fracture_failing_region(&mut w, &region, region[0]));
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
        let region = loose_slab(&mut w, 0, 10, 60, 10); // 600 cells, well past MAX_BODY_CELLS
        assert!(fracture_failing_region(&mut w, &region, region[0]), "a large collapse should still break up, not decline");

        assert!(w.chunk_bodies.len() > 1, "600 cells should yield many pieces, not one");
        assert_eq!(total_debris(&w), 600, "fracture lost or duplicated material");
        assert_eq!(count_material(&w, material::STONE), 0, "every cell of the region should have left the grid");
    }

    #[test]
    fn a_slab_that_falls_and_lands_shoves_air_where_it_hits() {
        // `Reports/destruction-plan.md` D2. The most visible moment of a
        // collapse -- the arrival -- was the only one that wrote no field
        // footprint at all, which `design-philosophy.md` §0a names as one
        // of the three things that make a destructive event unfinished.
        let mut w = test_world();
        for x in 0..64 {
            w.set(x, 63, Cell::new(material::BEDROCK, 0));
        }
        let region = loose_slab(&mut w, 20, 10, 6, 3);
        assert!(fracture_failing_region(&mut w, &region, region[0]));
        assert!(!w.chunk_bodies.is_empty(), "test setup: the slab should have promoted to at least one body");

        // The break's own impulse is written around the slab's *original*
        // position (y=10..12, radius 4), so measuring down at the floor
        // below cannot pick it up by accident -- which is what makes this
        // a test of the landing rather than of the promotion.
        assert_eq!(w.field_at(32, 60).pressure, 0.0, "test setup: the floor should start at ambient pressure");

        for _ in 0..300 {
            step_chunk_bodies(&mut w);
            crate::sim::update::step(&mut w);
        }
        assert!(w.chunk_bodies.is_empty(), "test setup: the bodies should have settled");

        // Somewhere along the floor, where it actually came down.
        let felt = (0..64).any(|x| (55..64).any(|y| w.field_at(x, y).pressure.abs() > 0.5));
        assert!(felt, "a slab fell the height of the world, landed, and shoved no air at all");
    }

    #[test]
    fn digging_a_doorway_tells_the_wall_above_it() {
        // The verb gap `Reports/design-philosophy.md` §0a names as this
        // subsystem's original sin, closed for precise removal: "an eraser
        // delivers no load and no impulse, so nothing ever failed from
        // being struck." Carving a doorway with the eraser leaves the
        // lintel above it unaware it is now spanning a gap; carving it with
        // `mine` must not.
        let mut w = test_world();
        for y in 20..50 {
            for x in 10..50 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        crate::sim::structural::compute_world_distances(&mut w);
        let before = count_material(&w, material::STONE);
        assert_eq!(w.field_at(30, 45).pressure, 0.0, "test setup: starts at ambient pressure");

        // Yield 1.0: this test is about the cut *telling* the structure,
        // not about spoil, so nothing is allowed to leave the world and
        // confuse the counts below.
        mine(&mut w, 30, 45, 4, 1.0);

        assert!(count_material(&w, material::STONE) < before, "digging removed nothing");
        // The three things the eraser does not do, which are the whole
        // point of this being a verb rather than a deletion.
        assert!(w.field_at(30, 45).pressure.abs() > 0.0, "a cut should shove the air");
        assert!(w.active_site_count() > 0, "the structure was never told anything was cut out of it");
        let loosened = (25..36).any(|x| (40..50).any(|y| {
            let c = w.get(x, y);
            c.material == material::STONE && !c.attached()
        }));
        assert!(loosened, "the rock around the cut is still claiming to be intact");
    }

    /// One spoil model, and it is the *verb's*, not the caller's.
    ///
    /// The bug this guards: `player::dig` thinned by `Tuning::dig_yield`
    /// and `App::mine` did not, so the gnome and the `D` key dug different
    /// holes in the same rock. Now that the gnome and the creatures are
    /// what actually excavate, the sandbox verb agreeing with them is the
    /// whole point -- so the assertion is on `mine` itself, which is the
    /// one place all three go through.
    ///
    /// Both ends are checked deliberately. At 1.0 nothing may leave, which
    /// is the promise that stops the eraser creeping back in through the
    /// mining verb; at 0.0 the bite must actually go, which is the promise
    /// that a cave can exist at all -- `mine` conserves cells, so without
    /// thinning a bore holds exactly the volume the rock did.
    #[test]
    fn one_spoil_model_governs_every_digger() {
        let solid = |yield_fraction: f32| {
            let mut w = test_world();
            for y in 20..50 {
                for x in 10..50 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            crate::sim::structural::compute_world_distances(&mut w);
            let before = occupied_cells(&w);
            let dusted = mine(&mut w, 30, 45, 4, yield_fraction);
            (before - occupied_cells(&w), dusted)
        };

        let (gone_at_full, dusted_at_full) = solid(1.0);
        assert_eq!(gone_at_full, 0, "at yield 1.0 a dig may move material but never delete it");
        assert_eq!(dusted_at_full, 0, "and it must not claim to have dusted any");

        let (gone_at_void, dusted_at_void) = solid(0.0);
        assert!(gone_at_void > 0, "at yield 0.0 the bite must actually leave, or no cave can ever open");
        assert_eq!(gone_at_void, dusted_at_void, "the returned count is what left the world");

        // Graded, not binary: the whole reason this is a fraction is that
        // the owner is undecided between "rock simply goes" and "collecting
        // the rubble is the game", and the middle has to be reachable.
        let (gone_at_half, _) = solid(0.5);
        assert!(
            gone_at_half > 0 && gone_at_half < gone_at_void,
            "yield 0.5 removed {gone_at_half}, which is not between 0 and the full bite of {gone_at_void}"
        );
    }

    /// Two bites offset from each other leave a corridor with no pinch in
    /// it, where two stamped discs leave one.
    ///
    /// **A paired comparison, and it has to be**: "the swept cut is open
    /// at the midpoint" is true of a wide enough disc too. What is under
    /// test is the *difference*, which is the pinch between consecutive
    /// bites — 13 cells clear where the bore is 15, against a gnome 14
    /// tall, which is why he stood at the mouth of his own tunnel and
    /// never went in.
    ///
    /// Yield 0.0 so the cut leaves air rather than rubble: this is about
    /// the shape of the hole, and rubble sitting in it would be counted as
    /// "not clear" while being something the digger can wade through.
    #[test]
    fn a_swept_bite_leaves_no_pinch_between_it_and_the_last_one() {
        const R: i32 = 5;
        let solid = || {
            let mut w = test_world();
            for y in 10..60 {
                for x in 0..64 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            crate::sim::structural::compute_world_distances(&mut w);
            w
        };
        // Clear cells in the column halfway between the two bite points.
        let clear_at = |w: &World, x: i32| (10..60).filter(|&y| w.get(x, y).material == material::EMPTY).count();

        let (a, b) = ((30, 35), (30 + R + 2, 35));
        let midpoint = (a.0 + b.0) / 2;

        let mut stamped = solid();
        mine(&mut stamped, a.0, a.1, R, 0.0);
        mine(&mut stamped, b.0, b.1, R, 0.0);

        let mut swept = solid();
        mine(&mut swept, a.0, a.1, R, 0.0);
        mine_swept(&mut swept, a, b, R, 0.0);

        let (pinch, corridor) = (clear_at(&stamped, midpoint), clear_at(&swept, midpoint));
        assert!(
            corridor > pinch,
            "a swept bite should widen the pinch between two bites: stamped left {pinch} clear cells at the midpoint, swept left {corridor}"
        );
        // And the corridor is the *full* bore, not merely wider: `2R + 1`
        // is what the bite is on its centre line, and the whole point is
        // that a swept cut is that tall everywhere rather than only there.
        assert_eq!(
            corridor,
            (2 * R + 1) as usize,
            "the swept corridor should be the full {} cells tall at the midpoint, was {corridor}",
            2 * R + 1
        );
    }

    /// Cells of anything that is not air, counted raw: `Cell::is_empty()`
    /// is managed-aware and would answer a different question.
    fn occupied_cells(world: &World) -> usize {
        let mut n = 0;
        for y in 0..64 {
            for x in 0..64 {
                if world.get(x, y).material != material::EMPTY {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn even_the_smallest_brush_lands_a_blow_worth_seeing() {
        // Reported from play: "striking a cliff does nothing", with the
        // brush at R2. The swing scales off the brush, and at radius 2 that
        // is a core of 1 and a chip of 2 -- about five cells removed on the
        // face of a cliff, which is invisible. Asserts the floor: the
        // smallest possible brush still takes a real bite and throws
        // something.
        let mut w = test_world();
        for y in 10..60 {
            for x in 10..60 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        let before = count_material(&w, material::STONE);

        strike(&mut w, 35, 35, 1, 6.0);

        let removed = before - count_material(&w, material::STONE);
        assert!(removed >= 20, "the smallest brush should still take a real bite out of a cliff, removed {removed}");
        assert!(!w.chunk_bodies.is_empty(), "a blow should throw pieces, not just make a hole");
    }

    #[test]
    fn a_collapse_shoves_air_at_the_joint_that_gave_way() {
        // The impulse used to be centred on the region's bounding box,
        // which for a subtree failure is the middle of the piece that fell
        // -- often a long way from the joint that actually broke. So the
        // air moved, the grit jumped, and none of it happened where the
        // player was looking. `Reports/destruction-plan.md` A3.
        //
        // A long thin region, because that is the shape where the two
        // origins are far apart and the difference is visible: 60 cells
        // wide, breaking at its left end.
        let mut w = test_world();
        let region = loose_slab(&mut w, 0, 10, 60, 3);
        let neck = (0, 10);
        let middle = (30, 11);
        assert_eq!(w.field_at(neck.0, neck.1).pressure, 0.0, "test setup should start at ambient pressure");

        assert!(fracture_failing_region(&mut w, &region, neck));

        let at_neck = w.field_at(neck.0, neck.1).pressure.abs();
        let at_middle = w.field_at(middle.0, middle.1).pressure.abs();
        assert!(at_neck > 0.5, "the joint that broke should have been shoved, found {at_neck}");
        assert!(
            at_neck > at_middle,
            "the collapse should read as starting at the joint ({at_neck}) rather than mid-piece ({at_middle})"
        );
    }

    #[test]
    fn fracture_touches_only_the_region_it_was_given() {
        // Was `only_cells_past_their_span_join_the_body`, asserting the
        // flood predicate this module used to own. The predicate moved to
        // `load::failing_region`; the claim worth keeping here is the one
        // about *this* function, and it is not vacuous -- an overhang about
        // to come down is still adjacent to the mountain, and a fracture
        // that grew its region by even one ring would take the mountain
        // with it.
        let mut w = test_world();
        let region = loose_slab(&mut w, 20, 20, 6, 3);
        for y in 20..23 {
            for x in 26..40 {
                w.set(x, y, Cell::new(material::STONE, 0)); // aux 0 -- well supported, and adjacent
            }
        }
        assert!(fracture_failing_region(&mut w, &region, region[0]));
        assert_eq!(
            count_material(&w, material::STONE),
            42,
            "fracture reached past the region it was handed, into cells that were still supported"
        );
    }

    #[test]
    fn a_body_falls_and_settles_back_into_the_grid() {
        let mut w = test_world(); // no floor
        for x in 0..64 {
            w.set(x, 63, Cell::new(material::BEDROCK, 0));
        }
        let region = loose_slab(&mut w, 20, 10, 6, 3);
        assert!(fracture_failing_region(&mut w, &region, region[0]));
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
        let region = loose_slab(&mut w, 20, 10, 6, 3);
        assert!(fracture_failing_region(&mut w, &region, region[0]));
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
        let region = loose_slab(&mut w, 20, 10, 6, 3);
        assert!(fracture_failing_region(&mut w, &region, region[0]));

        for _ in 0..300 {
            step_chunk_bodies(&mut w);
        }

        assert_eq!(count_material(&w, material::SAND), sand_before, "the body deleted sand it passed through instead of displacing it");
        assert_eq!(total_debris(&w), 18, "the body itself lost cells");
    }

    /// A falling chunk goes *through* smoke, not onto it.
    ///
    /// The paired shape `CLAUDE.md` asks for: the same slab dropped down the
    /// same shaft, once packed with smoke and once left open, must end at the
    /// same height. Before this, `clear_or_displaceable` treated every kind
    /// but `Powder` and `Liquid` as a real obstruction, so the smoke arm
    /// stalled in mid-air and re-embedded three frames later -- and because
    /// `Tuning::smoke_fraction` backfills a fresh crater, the one place in
    /// the world thick with gas is exactly the hole the player is watching a
    /// chunk fail to fall into.
    ///
    /// The shaft is packed solid with smoke deliberately: `displace` searches
    /// `DISPLACE_SEARCH` rings for an *empty* cell, so a thin wisp would be
    /// shoved aside and pass even under the old rule. What this pins is the
    /// case with nowhere to shove it to, which is the case a crater is.
    #[test]
    fn a_falling_body_is_not_held_up_by_smoke() {
        fn drop_through(fill: Option<MaterialId>) -> i32 {
            let mut w = test_world();
            for x in 0..64 {
                w.set(x, 63, Cell::new(material::BEDROCK, 0));
            }
            // Walls, so the shaft cannot vent sideways -- otherwise `displace`
            // finds room outside it and the two arms stop differing.
            for y in 30..63 {
                w.set(19, y, Cell::new(material::BEDROCK, 0));
                w.set(30, y, Cell::new(material::BEDROCK, 0));
            }
            if let Some(m) = fill {
                for y in 40..63 {
                    for x in 20..30 {
                        w.set(x, y, Cell::new(m, 0));
                    }
                }
            }
            let region = loose_slab(&mut w, 22, 31, 6, 3);
            assert!(fracture_failing_region(&mut w, &region, region[0]), "the slab must promote");
            for _ in 0..400 {
                step_chunk_bodies(&mut w);
            }
            // Lowest stone cell anywhere: where the chunk came to rest.
            (0..64).rev().find(|&y| (0..64).any(|x| w.get(x, y).material == material::STONE)).expect("the slab is somewhere")
        }

        let open = drop_through(None);
        let smoky = drop_through(Some(material::SMOKE));
        assert!(open > 55, "test setup: down an empty shaft the slab should reach the floor, it stopped at y={open}");
        assert_eq!(smoky, open, "smoke held the chunk up at y={smoky} when an open shaft put it at y={open}");
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
                let region = loose_slab(&mut w, 20, 20, 6, 3);
                fracture_failing_region(&mut w, &region, region[0]);
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

    /// W3: a fissure is a fragmentation seam, not decoration.
    ///
    /// A cracked edge already severs *support* everywhere in `load.rs`, and
    /// was then ignored by the thing that actually breaks the rock -- the
    /// flood ran straight across it, so the only random property of a
    /// fragment was its size and every collapse came apart along BFS rings.
    #[test]
    fn a_fragment_does_not_flood_across_a_fissure() {
        let mut w = test_world();
        let region = loose_slab(&mut w, 20, 20, 8, 6); // 48 cells
        // A clean vertical cut down the middle: every cell in column 23 has
        // its right edge scored, so nothing may pass from x<=23 to x>=24
        // (and, through `edge_is_cracked`'s mirror rule, nothing may come
        // back the other way either).
        for y in 20..26 {
            let cell = w.get(23, y);
            w.set(23, y, cell.with_crack_right(true));
        }
        let mut left: HashSet<(i32, i32)> = region.iter().copied().collect();
        let fragment = take_fragment(&w, &mut left, (20, 20), 48);

        assert!(!fragment.is_empty(), "test setup: the flood should still take the seeded half");
        assert!(
            fragment.iter().all(|&(x, _)| x <= 23),
            "the flood crossed a scored edge: {:?}",
            fragment.iter().find(|&&(x, _)| x > 23)
        );
        assert_eq!(fragment.len() + left.len(), 48, "material was lost between the fragment and the remaining pool");
    }

    /// The same rule read from the other side, which is the half that is
    /// easy to get wrong: `crack_right`/`crack_down` are *directional* edge
    /// bits owned by one of the two cells, so stepping left or up has to
    /// ask the neighbour about its own bit. A version that only tested the
    /// `+x`/`+y` steps passes the test above and leaks in both the others.
    #[test]
    fn a_fissure_blocks_the_flood_from_either_side() {
        let mut w = test_world();
        let region = loose_slab(&mut w, 20, 20, 8, 6);
        for y in 20..26 {
            let cell = w.get(23, y);
            w.set(23, y, cell.with_crack_right(true));
        }
        // Seeded on the *far* side this time, so every crossing attempt is
        // a `-x` step asking cell 23 about its own right edge.
        let mut left: HashSet<(i32, i32)> = region.iter().copied().collect();
        let fragment = take_fragment(&w, &mut left, (27, 20), 48);
        assert!(
            fragment.iter().all(|&(x, _)| x >= 24),
            "the flood crossed a scored edge travelling left: {:?}",
            fragment.iter().find(|&&(x, _)| x < 24)
        );

        // And the horizontal edge, both ways, for the `crack_down` half.
        let mut w = test_world();
        let region = loose_slab(&mut w, 20, 20, 8, 6);
        for x in 20..28 {
            let cell = w.get(x, 22);
            w.set(x, 22, cell.with_crack_down(true));
        }
        let mut left: HashSet<(i32, i32)> = region.iter().copied().collect();
        let down = take_fragment(&w, &mut left, (20, 20), 48);
        assert!(down.iter().all(|&(_, y)| y <= 22), "the flood crossed a scored edge travelling down");
        let mut left: HashSet<(i32, i32)> = region.iter().copied().collect();
        let up = take_fragment(&w, &mut left, (20, 25), 48);
        assert!(up.iter().all(|&(_, y)| y >= 23), "the flood crossed a scored edge travelling up");
    }

    /// W2 seen from `fracture_shell`: the per-cell test that decides
    /// whether this cell's direction is contained went from a binary sector
    /// skip to the same jittered limit `clear_annulus` uses, so the crater
    /// and the shell thrown off its rim agree about where the rim is. What
    /// must not move is the *decision* at either extreme -- a contained
    /// blast still throws nothing, an open one still throws.
    #[test]
    fn a_ragged_rim_still_holds_a_fully_contained_shell_and_throws_an_open_one() {
        let radius = 20;
        let build = |w: &mut World| {
            for y in 0..64 {
                for x in 0..64 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
        };

        let mut contained = test_world();
        build(&mut contained);
        fracture_shell(
            &mut contained,
            (32, 32),
            radius,
            radius + 3,
            50.0,
            1,
            super::super::explosion::Confinement { sector_reach: [7; super::super::explosion::CONFINEMENT_SECTORS], radius },
        );
        assert!(contained.chunk_bodies.is_empty(), "a fully contained blast must still throw nothing off its rim");
        assert_eq!(count_material(&contained, material::STONE), 64 * 64, "a contained blast must not convert rim material");

        let mut open = test_world();
        build(&mut open);
        fracture_shell(
            &mut open,
            (32, 32),
            radius,
            radius + 3,
            50.0,
            1,
            super::super::explosion::Confinement {
                sector_reach: [radius as u8; super::super::explosion::CONFINEMENT_SECTORS],
                radius,
            },
        );
        assert!(!open.chunk_bodies.is_empty(), "an open blast must still throw its rim as pieces");
    }
}
