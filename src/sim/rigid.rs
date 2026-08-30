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

/// The eight-neighbour ring, for a flood over material whose *writer* placed
/// it at eight — `MaterialDef::woody`, and `Grow` is that writer.
///
/// Deliberately **not** the flood rock uses. Two rock blobs touching at a
/// corner are not one physical body, which
/// `diagonal_only_contact_does_not_connect_two_components` guards on
/// purpose; a crown, which is mostly diagonal twigs, is one plant and cutting
/// it at every diagonal is what turned a fell into sawdust. Both are
/// `CLAUDE.md`'s "a traversal must use the same neighbourhood the writer
/// used" -- the writer just differs by material, which is why the choice is
/// content and not a constant.
const NEIGHBOURS_8: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

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
pub const MIN_BODY_CELLS: usize = 8;

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

/// The fastest a body may be allowed to turn, in quarter-turns per frame.
///
/// **A legibility bound, not a physical one.** At 1.0 a body snaps through
/// ninety degrees every frame, which reads as a flicker rather than as a
/// tumble, and every one of those turns has to clear `rotation_fits` in a
/// pose it occupies for a sixtieth of a second. Half of that is a full turn
/// in eight frames, which is as fast as anything here needs to look.
const MAX_SPIN_RATE: f32 = 0.5;

/// How many quarter turns a body may take because it landed out of balance.
///
/// Four, so a piece may come the whole way round once and then stop. This
/// is `topple`'s termination argument and it is deliberately a hard count
/// rather than a convergence test: two poses can each read as unbalanced in
/// the other's favour, and a body trading turns with itself for ever is a
/// chunk that never sleeps — which `CLAUDE.md` prices at ~8 ms/frame,
/// because what it defeats is the dirty-rect render skip.
const MAX_TOPPLE_TURNS: u8 = 4;

/// How far outside the middle of its footing a settled piece's centre of
/// mass has to sit before it goes over, as a fraction of the footing's own
/// half-width.
///
/// The middle-third rule — the same kern `load::bearing_moment` is built on
/// (`KERN_DENOMINATOR`), read here as a question about *a piece* rather than
/// about a cell. That distinction is the finding this constant exists to
/// act on: `Reports/plant-mechanics-handoff-2026-08-29.md` §3.2 measured
/// what happens when the load model's clamp is simply let through to `log`,
/// and the answer is that it **crushes** the pieces rather than laying them
/// down (settled lying/upright went 3/8 to 1/11, `log` 833 cells to 716)
/// — because the load model's only verdict for a thing that fails is
/// `breaks_into`. The test was never the missing part. The outcome was.
const TIPPING_KERN: f32 = 1.0 / 3.0;

/// `FALL=off` puts a body back on the pre-2026-08-29 rule: no rate seeded
/// from the break, no tipping test on landing, and the speed term turning
/// one way as it always did.
///
/// **The control, and it is here because the alternative is not one.**
/// Comparing this build against the last one compares two *binaries*, so
/// every instrument added alongside the mechanism is missing from the arm it
/// is being measured against -- which is exactly how the first reading of
/// this change went wrong: the census that could see per-piece orientation
/// existed only in the new build, so the comparison fell back on a
/// cluster-level statistic that provably cannot answer the question. One env
/// switch holding the semantic rule fixed and changing nothing else is
/// `CLAUDE.md`'s own remedy, and it also keeps these runs reproducible after
/// the mechanism has been tuned past them.
/// Whether this piece is organism tissue — a limb, rather than rock someone
/// blew up.
///
/// **The fall is scoped to tissue, and that is a budget decision rather than
/// a claim about physics.** `angular_acceleration` is as true of a slab of
/// stone as of a branch, and the rock line has the same defect on record:
/// `Reports/open-bugs-handoff.md` §Q's `scene=worked` needles are rubble
/// standing on end. But rock destruction's constants and its gates are
/// calibrated against how debris tumbles *now*, and `CLAUDE.md` is explicit
/// that a change reallocating a budget you have not costed is not scoped,
/// merely started.
///
/// It is costed enough to say what it would take. Under
/// `a_disturbance_extent_licenses_the_wound_but_not_the_chain` — a radius-20
/// charge in a massif at TIGHT, paired against the same blast with the
/// disturbance's extent forced to zero — seeding rock leaves the *licensed*
/// arm almost unchanged (promoted 701 -> 723, stone left 29,577 -> 29,490)
/// and roughly doubles the **leashed** one (promoted 506 -> **1,217**,
/// shattered 276 -> 365, stone left 29,555 -> 29,356). Everything it
/// destroys is still inside the leash — `max_damage_reach` holds at 16 — so
/// the chain rule is intact; what changes is that landed debris in more
/// poses gets re-judged and more of it comes away, which is §Z3's shape on
/// rock. That is a real question about the destruction line and it wants its
/// own measurement, which §Q already says has to be run on `scene=worked`
/// before anything there is tuned.
fn is_tissue(cells: &[BodyCell]) -> bool {
    cells.iter().any(|c| c.organism_id != 0)
}

fn fall_enabled() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| !matches!(std::env::var("FALL").as_deref(), Ok("off")))
}

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
/// and throws something: at 6 the core is 2 and the chip is 6, which is
/// ~100 cells loosened against `MIN_FRACTURE_CELLS`, and cracks run 18.
/// (The chip figure was 4 until the chip zone went to the full radius --
/// see `strike`.)
const MIN_STRIKE_RADIUS: i32 = 6;

/// How far past a blow's own radius its fractures run. Cracks reaching
/// further than the damage is what lets a player work a fissure across a
/// span rather than having to chew through it.
pub const CRACK_REACH: i32 = 3;

/// How far past its own radius a blow opens the rock's joints.
///
/// **2, where the rays ran to `CRACK_REACH` (3), and the difference is
/// cost rather than looks.** A ray star is sparse -- five lines, ~85
/// cells -- so unbracing every cell it scored over the full three-times
/// reach was cheap. The joint fabric is *dense*: it opens several hundred
/// edges over the same disc, and `structural::detach_around_crack`
/// schedules a check for every cell it unbraces. At 3x, one radius-14 blow
/// into a solid massif left `acceptance.sh`'s `strike` case with 2,284
/// pending scheduler sites against its bar of 1,500 -- `open-bugs-
/// handoff.md` §S's backlog that climbs instead of draining.
///
/// **Bounding the unbracing by distance instead was tried and abandoned**,
/// because the cascade is chaotic in that parameter rather than monotone:
/// measured on the same case, 1,553 sites at twice the chip radius, 3,109
/// at 1.75x, 1,277 at 1.5x, while `scene=worked`'s shelf needed the wide
/// one to give way at all. A value that clears both cases there is landing
/// on luck. Cutting the reach cuts the number of joints rather than
/// filtering them afterwards, which is the honest version of the same
/// saving.
const BLOW_JOINT_REACH: i32 = 2;

/// Fractures scored per blow, when a blow drew rays.
///
/// **Nothing reads this any more** — `strike` reveals the rock's joints
/// instead (`structural::shatter_joints_around`), on the owner's verdict
/// that the rays read as lines drawn on stone. Kept, with its number, so
/// the tuning that shipped is on the record beside the reason it went;
/// `MINE_CRACK_RAYS` is the live one.
#[allow(dead_code)]
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

/// What a hand tool may cut: `is_body_material` **plus `Plant`**.
///
/// # Why this is a second predicate and not a widened first one
///
/// `is_body_material` above is `Solid` alone, and that is the real reason
/// the pick and the chisel could not touch a tree -- not the
/// `organism_id() != 0` tests that used to sit beside it in `strike` and
/// `mine_swept` and that `open-bugs-handoff.md` §D1 names. Removing those
/// changed nothing at all, measured: four blows across a 26-cell bole took
/// **0 cells** and left every counter at zero, because `wood` is
/// `MaterialKind::Plant` and never reached the organism test at all. Two
/// gates, one visible in the report and one not, and only the invisible one
/// was load-bearing -- `CLAUDE.md`'s "a change that moves *nothing* is
/// different evidence from one that moves a little", read the right way
/// round for once.
///
/// It stays separate because `is_body_material`'s other callers are
/// `label_component` and `trace_contours`, which answer "what piece of
/// *rock* is this" for the M8 body pipeline. Widening it there would change
/// what a component *is* on every scene in the engine to fix two verbs.
///
/// `structural::is_body_material` -- the structural system's own, `Solid |
/// Plant` since architecture item 9 -- is the one this now agrees with, and
/// that agreement is the point: a trunk is exactly as capable of being cut
/// as a stone span is of being undercut.
///
/// **Public since `player::bore_slice` needs it.** The bore's working face
/// is "the nearest slab with something breakable in it", and asking that
/// question with a second, separately-written predicate is how a preview
/// starts disagreeing with the cut it previews.
pub fn is_tool_target(world: &World, x: i32, y: i32) -> bool {
    let material = world.get(x, y).material;
    matches!(world.materials.kind(material), MaterialKind::Solid | MaterialKind::Plant) && material != material::BEDROCK
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
    /// The organism this cell belonged to when it left the grid, or `0` for
    /// inert material.
    ///
    /// **Two bytes into existing padding** (`dx`/`dy` are `i32`,
    /// `material` is a `MaterialId` and `shade` a `u8`), which is why this
    /// is the answer rather than growing `Cell` -- whose flags are full at
    /// 8/8 (`Reports/load-model-handoff.md`) -- or a position-keyed side
    /// table, which is a recorded dead end (`Reports/dead-ends.md` §572).
    ///
    /// It buys three things at once, and all three were live defects:
    ///
    /// - `settle` writes the piece back as **dead tissue**
    ///   (`MaterialDef::severs_into`) rather than as live `wood`, so a
    ///   landed limb rots, feeds the litter layer and is something
    ///   `decay.rs` can see. A `wood` wall someone painted has no organism
    ///   id and still lands as the wall it was, which is why the test is
    ///   this field and not the material.
    /// - the felling census can report plant mass *in flight*. S1's own
    ///   line read "bodies carrying plant material 0 of 0 body cells" and
    ///   that zero is the number this package had to move.
    /// - `promote` can decline to schedule a structural check around tissue
    ///   that is already leaving -- the amputation landmine
    ///   (`CLAUDE.md`'s structural-check gotcha) firing from *inside* the
    ///   fall, which `Reports/felling-blockers.md` §3 step 4 predicted.
    pub organism_id: u16,
}

/// Which way a body goes through a quarter turn.
///
/// **A signed direction, and it was not free.** The transform below had one
/// handedness, so every body in the engine could only ever turn one way.
/// That is invisible while nothing turns at all — measured on `scene=fell`
/// before this, a whole felled tree asked for **zero** quarter turns — and
/// it becomes wrong the moment rotation is seeded from physics, because a
/// limb whose mass hangs to the left of the break falls to the left.
///
/// `Reports/plant-mechanics-handoff-2026-08-29.md` §3.5 costed the reverse
/// as "three forward turns and a fit probe on each intermediate pose".
/// `Ccw` is the exact inverse permutation instead: one turn, still exact on
/// a grid, and no intermediate pose to check or to leak material through.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Turn {
    /// Clockwise on screen. `y` points down, so this is the direction mass
    /// hanging to the **right** of a pivot swings.
    Cw,
    /// Anticlockwise on screen.
    Ccw,
}

impl Turn {
    /// The offset `(dx, dy)` turned a quarter about the origin:
    /// `Cw` is `(-dy, dx)` and `Ccw` its exact inverse, `(dy, -dx)`.
    ///
    /// **The single definition of the transform.** `ChunkBody::rotate_quarter`
    /// applies it and `ChunkBody::turned_offset` predicts it, and the two
    /// must not be able to drift apart — see `rotation_fits` for what a probe
    /// that disagrees with the move it guards is worth.
    fn applied(self, dx: i32, dy: i32) -> (i32, i32) {
        match self {
            Turn::Cw => (-dy, dx),
            Turn::Ccw => (dy, -dx),
        }
    }

    /// The turn a signed angular quantity implies, with positive clockwise
    /// to match the screen's `y`-down handedness.
    fn of(signed: f32) -> Self {
        if signed < 0.0 {
            Turn::Ccw
        } else {
            Turn::Cw
        }
    }

    /// `+1` clockwise, `-1` anticlockwise — the factor that puts a scalar
    /// spin budget on the same axis as the direction it is turning in.
    fn sign(self) -> f32 {
        match self {
            Turn::Cw => 1.0,
            Turn::Ccw => -1.0,
        }
    }
}

/// The stump a severed crown is still swinging on.
///
/// # Why the pieces need to share one of these
///
/// **A felled tree came apart before it fell.** `fell_severed_tissue` runs
/// the fragment ladder over the whole severed region at the *instant of the
/// cut*, so on `scene=fell` **56 separate bodies exist before a cell has
/// moved**, each falling on its own. The owner's verdict on the animation
/// (card `20260829T141251798Z-6040b0`) named it exactly: *"because the whole
/// thing just comes apart and falls directly downward, it looks unrealistic
/// ... it should hinge at the trunk, hit the ground, and the bottom branches
/// break off from the impact ... not just unzip and fall to the ground"*.
///
/// **The fragments are not the problem and are not being taken away** —
/// they are what stopped a crown reading as sawdust, and `deadleaf.ron` and
/// `fracture`'s own doc record what happens without them. What was missing
/// is that they had no shared motion. So every fragment off one cut carries
/// the same hinge, and while it does, its velocity is the rigid-body
/// `omega x r` about that stump rather than its own ballistic fall. A piece
/// forty cells up the trunk travels four times as fast as one ten cells up,
/// which is what makes the assembly *sweep* instead of dropping, and what
/// puts the base on the ground while the crown is still in the air.
///
/// # Gravity is replaced, not added
///
/// `alpha` is computed **from** gravity (`angular_acceleration`), so a
/// hinged body that also took `GRAVITY` on its `vy` would be counting the
/// same force twice and would sag out of its own arc. `advance` skips the
/// gravity line while a hinge is held. The stump supplies the reaction; that
/// is what a hinge *is*.
///
/// # It is released by the first thing it hits
///
/// A real tree stops being hinged when it lands, and so does this: `landed`
/// clears it on the first vertical collision, and the piece keeps the
/// velocity it had. That is where "the bottom branches break off from the
/// impact" comes from — the low pieces arrive first, at low speed, and the
/// crown is still swinging above them.
#[derive(Clone, Copy, Debug)]
struct Hinge {
    /// The cut, in world cells — `load::Failure::at`.
    pivot: (i32, i32),
    /// Angular acceleration about it, radians per frame squared, signed.
    /// One value shared by every fragment of one severance, which is what
    /// makes them one tree rather than fifty-six.
    alpha: f32,
    /// Angular velocity so far. Starts at zero, because a tree that has just
    /// been cut is not yet moving.
    omega: f32,
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
    /// How fast it is turning, in quarter-turns per frame, **signed**.
    ///
    /// `spin` above is an *angle* and this is its rate, which is the
    /// distinction the mechanism was missing: everything a body knew about
    /// turning came from how fast it was travelling, and travelling fast is
    /// not the same as having been swung. A trunk cut through at the base
    /// has no speed at all in the frame it comes free and every reason to
    /// go over.
    spin_rate: f32,
    /// How fast that rate is changing, in quarter-turns per frame squared —
    /// the angular acceleration `promote` read off the break.
    ///
    /// Held rather than applied once, because a piece that breaks off is
    /// still being turned by its own weight about the joint that gave way;
    /// that is what a felled trunk pivoting on its stump *is*. It is what
    /// makes the outcome graded with nothing to tune: for a limb of length
    /// `L` breaking at one end the acceleration goes as `3g/(2L)`, so a
    /// bole comes over about once across a fifty-cell drop and a twig
    /// tumbles several times in the same fall. See `angular_acceleration`.
    spin_accel: f32,
    /// Quarter turns this body has taken because it was out of balance
    /// where it landed, rather than because it was turning in the air.
    ///
    /// Capped by `MAX_TOPPLE_TURNS`, and the cap is the whole termination
    /// argument for `topple`: two poses that each read as unbalanced in the
    /// other's favour would otherwise trade turns for ever, and a body that
    /// never settles is a chunk that never sleeps.
    topples: u8,
    /// The offset this body turns about. See `ChunkBody::centre_of`.
    pivot: (i32, i32),
    /// The stump this piece is still swinging on, while it still is. See
    /// `Hinge`.
    hinge: Option<Hinge>,
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
    /// Whether this body is holding its own footprint against the world —
    /// see `reserved`.
    ///
    /// **Latched on first contact with fluid, never set at promotion**, and
    /// that gate is not an optimisation. Holding the footprint from birth
    /// costs a dry scene nothing to *maintain* and changes its outcome:
    /// with the space it stands in closed, a body stops shedding cells to
    /// collisions on landing, so more of it arrives intact, so the load
    /// model is handed a bigger connected region to judge. Measured on
    /// `scene=strike`, which is dry: the same two failures went from 503 to
    /// 1,372 cells and the worst frame from **20 ms to 118 ms**, against a
    /// 60 ms budget. Nothing about that scene was reported as wrong.
    ///
    /// In air the reservation buys nothing anyway -- there is no fluid to
    /// keep out -- so latching it the first time the body actually meets
    /// liquid confines the whole mechanism to the bug it was built for and
    /// leaves every dry scene byte-identical.
    reserved: bool,
}

impl ChunkBody {
    /// A body placed directly, at rest. Test-only, and deliberately so:
    /// production bodies only ever come from `try_promote_failing_region`
    /// or `strike`, which derive velocity and spin from the break that
    /// made them, and a constructor that let callers skip that would be a
    /// way to create debris that never broke off anything.
    #[cfg(test)]
    pub fn at(cells: Vec<BodyCell>, x: f32, y: f32) -> Self {
        let pivot = Self::centre_of(&cells);
        Self { cells, x, y, vx: 0.0, vy: 0.0, spin: 0.0, spin_rate: 0.0, spin_accel: 0.0, topples: 0, pivot, hinge: None, stalled: 0, peak_speed: 0.0, reserved: false }
    }

    /// The same, already falling. Test-only for the same reason.
    #[cfg(test)]
    pub fn falling(cells: Vec<BodyCell>, x: f32, y: f32, vy: f32) -> Self {
        Self { vy, ..Self::at(cells, x, y) }
    }

    /// The cell a body turns **about**: the centre of the offsets it was
    /// promoted with, rounded down to a whole cell.
    ///
    /// **Not the origin, and that was a real defect rather than a
    /// refinement.** `promote` puts the origin at `cells[0]`, which after
    /// the sort is a corner of the piece, so turning about it swings the
    /// whole body round that corner: for the 25x35 bole `scene=fell`
    /// produces, the far end travels some fifty cells in the one frame the
    /// turn takes. `rotation_fits` checks the final footprint and not the
    /// swept path, so what that buys is a body teleporting across whatever
    /// stands between the two poses — and, far more often, a turn that is
    /// simply refused, because a pose fifty cells sideways is a pose inside
    /// the hillside. Turning about the centre keeps the piece where it is
    /// and asks only whether it fits *there*, which is the question the
    /// probe was written to answer.
    ///
    /// **Fixed at promotion and never recomputed**, which is what makes
    /// four turns the identity. Deriving it from the current offsets each
    /// time looks tidier and drifts: the mean is floored, so a body whose
    /// true centre sits at a half-cell comes out of a turn with its floored
    /// centre a cell away, the next turn pivots about *that*, and the piece
    /// walks. Measured on a six-cell L, a clockwise turn followed by an
    /// anticlockwise one left it a cell from where it started -- a position
    /// leak with no ledger to catch it, and the kind of thing that surfaces
    /// months later as "debris drifts". The stored pivot cannot drift, and
    /// it stays the centre of the shape because the shape stays congruent
    /// to the one it was measured on.
    ///
    /// Geometric, not mass-weighted: it needs no `World`, so it cannot go
    /// stale against a borrow, and at this resolution the two differ by a
    /// cell or so even on a limb that is half foliage. What it gives up is
    /// that a leafy limb turns about its middle rather than about its
    /// heavier woody end.
    fn centre_of(cells: &[BodyCell]) -> (i32, i32) {
        // `(0, 0)` under `FALL=off` is the origin, which is the pre-change
        // rule -- the control has to hold *all* of the semantics fixed, not
        // just the two new mechanisms, or it is measuring a third thing.
        if cells.is_empty() || !fall_enabled() {
            return (0, 0);
        }
        let n = cells.len() as i32;
        let (sx, sy) = cells.iter().fold((0i32, 0i32), |(sx, sy), c| (sx + c.dx, sy + c.dy));
        (sx.div_euclid(n), sy.div_euclid(n))
    }

    /// Turn the whole body a quarter turn about `pivot`.
    ///
    /// Exact on a grid twice over: a quarter turn maps every cell onto
    /// exactly one other cell, so it cannot leak or duplicate material, and
    /// the pivot is a whole cell that never moves, so four turns put every
    /// offset back bit for bit.
    fn rotate_quarter(&mut self, turn: Turn) {
        let (px, py) = self.pivot;
        for cell in &mut self.cells {
            let (rx, ry) = turn.applied(cell.dx - px, cell.dy - py);
            cell.dx = px + rx;
            cell.dy = py + ry;
        }
    }

    /// Where `cell` would sit after `rotate_quarter(turn)`, without turning
    /// anything.
    ///
    /// Exists so `rotation_fits` can ask about the turned footprint without
    /// cloning the body — and, more to the point, so the predicted turn and
    /// the performed one come from **one** definition of the transform. A
    /// probe that predicted a different turn from the one that then
    /// happened would be worse than no probe at all, which is close to what
    /// the previous version managed; see `rotation_fits`.
    fn turned_cell_position(&self, cell: &BodyCell, turn: Turn) -> (i32, i32) {
        let (px, py) = self.pivot;
        let (rx, ry) = turn.applied(cell.dx - px, cell.dy - py);
        (self.x.round() as i32 + px + rx, self.y.round() as i32 + py + ry)
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
    fracture_with_impulse(world, region, None, size_bias(extent), Some(broke_at), false, None);
}

/// As `fracture`, but every fragment is thrown away from `origin` at
/// `force`. This is what makes a *blow* different from a collapse: the same
/// rock comes apart either way, but struck rock leaves the wound rather than
/// sagging out of it.
fn fracture_with_impulse(
    world: &mut World,
    region: &[(i32, i32)],
    impulse: Option<((f32, f32), f32)>,
    size_bias: u32,
    broke_at: Option<(i32, i32)>,
    tissue: bool,
    hinge: Option<Hinge>,
) -> (usize, usize) {
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

    // Returned rather than read back off `FailureCounts`, because the two
    // tallies this feeds are different questions: `promoted_cells` is
    // world-wide and cumulative, and the felling census needs *this event's*
    // split to state a promoted share of severed mass at all.
    let (mut promoted_cells, mut grit_cells) = (0usize, 0usize);
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
        //
        // **Where the ladder starts is the material's business too**, and
        // that is the half that was missing rather than the number of
        // rungs. `wood` drew rock's {2, 4, 8, 16, 32}, two rungs of which
        // are under `MIN_BODY_CELLS` and so are grit before shape is
        // considered -- 1.7% of a felled tree survived as pieces. See
        // `MaterialDef::fragment_floor`.
        let material = world.materials.get(world.get(seed.0, seed.1).material);
        let (rungs, floor, diagonal) = (material.fragment_rungs, material.fragment_floor, material.woody);
        // **Only wood seeds a fragment of severed tissue.** Foliage may be
        // *claimed* by a fragment the flood reaches it through -- a leafy
        // limb comes off with its leaves on -- but it never starts one and
        // never chooses one's size, which is `Reports/physical-trees-
        // design-2026-08-23.md` §5.3's actual argument: a leaf is not a
        // small log and must not get a draw off wood's ladder. Anything no
        // woody seed reaches is scattered after the loop.
        if tissue && !diagonal {
            continue;
        }
        // Clamped rather than left to wrap: `floor` and `size_bias` are
        // both content-reachable, and a shift past the word size is a panic
        // in debug and nonsense in release. `MAX_BODY_CELLS` caps the result
        // anyway, so anything at or above its exponent is the same draw.
        let exponent = (floor as usize + world.rng.below(rungs) as usize + size_bias as usize).min(usize::BITS as usize - 1);
        let target = (1usize << exponent).min(MAX_BODY_CELLS);
        let fragment = take_fragment(world, &mut left, seed, target, diagonal);
        // The `world` argument is this branch's size-biased ladder: a wide
        // blow calves slabs where a tap chips.
        //
        // **`origin/main`'s `record_fragment(len, promoted)` call stood here
        // and was dropped, deliberately.** It split one fragment's mass into
        // `promoted_cells`/`shattered_cells` -- which this branch already
        // records, at the two choke points that cannot drift out of step
        // with what happened: `record_promoted` inside `promote` (the line
        // that actually pushes the body, so `fracture_with_impulse`,
        // `calve_collar` and `fracture_shell` all reach it through one
        // door) and `record_shattered` inside `shatter_to_rubble` and
        // `break_free`. Keeping both counted every promoted fragment's
        // cells twice and every shattered fragment's twice over. The richer
        // pair wins because it also carries `promoted_bodies` and
        // `promoted_sizes`, which `record_fragment` has no way to fill.
        // `size_buckets`, main's other counter here, is untouched: it is
        // fed from `record` and measures the failing *region*, not the
        // fragment, so it is a different quantity and both are kept.
        // **Severed tissue always flies, whatever size it came out.**
        //
        // `MIN_BODY_CELLS` is a judgement about *rock*: below eight cells a
        // tumbling body and a grain look the same, and rock wants grit --
        // the ethos asks for a few blocks, more cobbles, a lot of grit. A
        // plant does not. Owner, 2026-08-30: *"limbs always land as pieces
        // nothing should be turning to dust at all!"*, which is the third
        // time the same complaint has been made about this pipeline (*"the
        // branches fall off as whole pieces (good), but then hit the ground
        // and turn to dust"*, and *"some are disintegrating a little"*).
        //
        // A twig is a small twig, not sawdust, so the floor does not apply
        // to it. Rock is untouched.
        if tissue || fragment.len() >= MIN_BODY_CELLS {
            promote(world, &fragment, impulse, broke_at, hinge);
            promoted_cells += fragment.len();
        } else {
            for &(fx, fy) in &fragment {
                // **Organism grit is counted apart from rock grit, and
                // does not schedule.** `shattered_cells` deliberately
                // excludes the organism path (its own doc: a tree shedding
                // deadwood fires all through any vegetated world and would
                // swamp the number that says how *rock* came apart), and
                // `structural::break_free` -- the conversion this replaces
                // -- schedules nothing, leaving the fan-out to
                // `schedule_organism_neighbours`. Scheduling here instead
                // would fire checks inside a crown that is still coming
                // down, which is the amputation gotcha `promote` above
                // declines for the same reason.
                if tissue {
                    if convert_to_debris(world, fx, fy) {
                        grit_cells += 1;
                    }
                } else {
                    shatter_to_rubble(world, fx, fy);
                    grit_cells += 1;
                }
            }
        }
    }
    // **Whatever no fragment claimed.** For rock this is empty by
    // construction -- every cell of the region is a candidate seed, so the
    // loop above consumes all of it. For severed tissue it is the foliage
    // that hung off nothing a promoted piece reached, and it scatters, which
    // is the third tier arriving as a consequence of the same draw rather
    // than as a separate pass over the region.
    //
    // Walks `remaining` -- sorted -- rather than `left`, which is a
    // `HashSet`: iterating that would make the *order* of conversion depend
    // on the hasher, and `shatter_to_rubble`/`convert_to_debris` both draw a
    // shade from `world.rng`. Same determinism rule as the seed loop, and
    // the same one §2a names.
    if tissue {
        // **Foliage no woody fragment claimed comes down as a cluster, not
        // as scatter.** It used to convert cell by cell to litter, which is
        // the other half of the dust the owner keeps seeing: a crown is
        // roughly a third leaf by count, so a limb whose wood all landed in
        // one fragment could still shed hundreds of cells of powder around
        // it.
        //
        // Grouped before promoting rather than promoted per cell, and that
        // is the difference between a leafy spray falling together and a
        // thousand one-cell bodies costing a thousand body slots. A cluster
        // of leaves hangs together off its twig -- the same fact
        // `plant::anchor_support` now encodes -- so it falls together too.
        while let Some(&seed) = remaining.iter().find(|c| left.contains(c)) {
            let cluster = take_fragment(world, &mut left, seed, usize::MAX, true);
            if cluster.is_empty() {
                continue;
            }
            promote(world, &cluster, impulse, broke_at, hinge);
            promoted_cells += cluster.len();
        }
    } else {
        for &cell in &remaining {
            if !left.remove(&cell) {
                continue;
            }
            shatter_to_rubble(world, cell.0, cell.1);
            grit_cells += 1;
        }
    }
    (promoted_cells, grit_cells)
}

/// Take a severed piece of an organism off the grid as a **distribution**:
/// logs, branch-scale pieces, and a scatter of foliage.
///
/// Returns `(cells removed, cells that left as promoted body cells)`.
///
/// # Why this exists at all, and what it replaces
///
/// `structural::organism_structural_tick` used to convert **one cell** per
/// check, straight to `breaks_into`. That is a `Powder` with a friction
/// angle, so a felled tree's two and a half thousand cells did exactly what
/// the granular rules say they must and piled into a cone of sawdust. The
/// owner's word for it was "a tree disintegrating into dust"; there was no
/// piece anywhere in the pipeline, so no piece could come out of it.
/// Measured on one cut: 2,648 cells severed, **45 promoted (1.7%)**.
/// `Reports/physical-trees-design-2026-08-23.md` §1 and §5.
///
/// # The three tiers, and why they are three mechanisms rather than one
///
/// `prior-art-destruction.md` §2.4: nobody derives a debris size
/// distribution from physics, and that is not a gap -- every shipped game
/// stacks one mechanism per size class. So:
///
/// - **Foliage scatters.** A crown is roughly a third leaf by cell count,
///   and a leaf is not a small log. It comes off `breaks_into` to litter
///   and never touches the ladder, where it would otherwise pad a draw into
///   a "piece" that is mostly foliage. Gated on `MaterialDef::woody`, read
///   at this call site off the resolved `Material`.
/// - **Wood goes on the ladder**, at `wood.ron`'s own floor of
///   {32, 64, 128, 256, 400} rather than rock's {2, 4, 8, 16, 32}, flooding
///   at eight because that is the neighbourhood `Grow` placed it at.
/// - **What comes out under `MIN_BODY_CELLS` is grit**, by the same draw --
///   so the dust is a *consequence* of the distribution rather than a
///   separate fudge, which is the shape `fracture`'s own doc argues for.
///
/// # No size cap on the region, deliberately
///
/// `MAX_BODY_CELLS` bounds a single fragment and the ladder splits whatever
/// it is given, so a whole crown simply yields more pieces. A cap on the
/// *decision* would mean the bigger the fall the more certain it dissolved
/// -- written twice in `CLAUDE.md` and recorded three times in
/// `dead-ends.md`, and it is exactly the failure this function exists to
/// undo.
/// Where the severed piece was **standing**, which is what it swings about.
///
/// **Not `broke_at`, and the difference is the whole mechanism.**
/// `load::Failure::at` is the one cell the support check happened to
/// evaluate, and for a whole crown that is an arbitrary point on it.
/// Measured on `scene=fell`: the region spans x 201..320, `broke_at` is
/// `(201, 170)` — its far *left* edge, halfway up — and the region's centre
/// of mass sits 58.5 cells to the right of it and 2.0 cells below. `r` is
/// therefore very nearly horizontal, so `omega x r` points almost straight
/// **down**, and a hinge built on it is indistinguishable from dropping. The
/// arithmetic was right and the pivot was not.
///
/// The cut face is the region's own **lowest row**, at that row's horizontal
/// centre of mass: for a felled tree that is the stump's cross-section, and
/// `r` then points up the trunk, so the swing is horizontal — a fall.
///
/// **What it gives up**, named rather than discovered later: a limb that
/// breaks off *sideways* has its lowest row at its own drooping tip rather
/// than at the shoulder it tore from, so it swings about the wrong end.
/// `broke_at` is the better answer for that case and the worse one for this,
/// and telling them apart wants the attachment face — the region cells still
/// touching standing tissue — which is a bigger change than this. The scene
/// the owner is judging is a felled trunk.
fn cut_face(world: &World, region: &[(i32, i32)], broke_at: (i32, i32)) -> (i32, i32) {
    let Some(&lowest) = region.iter().map(|(_, y)| y).max() else {
        return broke_at;
    };
    let (mut mass, mut moment) = (0.0f64, 0.0f64);
    for &(x, y) in region.iter().filter(|&&(_, y)| y == lowest) {
        let m = world.materials.density(world.get(x, y).material) as f64;
        mass += m;
        moment += m * x as f64;
    }
    if mass <= 0.0 {
        return broke_at;
    }
    ((moment / mass).round() as i32, lowest)
}

pub(crate) fn fell_severed_tissue(world: &mut World, region: &[(i32, i32)], broke_at: (i32, i32)) -> (u32, u32) {
    if region.is_empty() {
        return (0, 0);
    }
    // **The whole region goes to the ladder, foliage included — but only
    // wood may *seed* a fragment.**
    //
    // Converting every leaf to litter here, before the ladder ran, was the
    // first shape of this function and it is what the owner saw frame by
    // frame: *"the branches fall off as whole pieces (good), but then hit
    // the ground and turn to dust"*. The pieces were fine. What turned to
    // dust was the **foliage**, and it did so at the instant of severance —
    // roughly 1,570 cells of `litter` powder created in a single frame,
    // against 710 cells of `log` at rest. Litter outnumbered the piece tier
    // 2.3 to 1, it is the brightest thing on screen, and the pieces fell
    // through it and were buried in it.
    //
    // A leaf still never seeds a fragment and never sizes one — the draw is
    // made at a woody seed, from wood's own ladder — so
    // `Reports/physical-trees-design-2026-08-23.md` §5.3 holds where it
    // argues foliage must not be *on* the ladder. What it did not argue,
    // and what was wrong, is that foliage must leave the branch at the
    // moment the branch does. A leafy limb comes off a real tree with its
    // leaves on it; they let go later, in their own time, and that is what
    // `MaterialDef::severs_into` on `leaf.ron` now does when the piece
    // lands.
    //
    // Foliage that no promoted piece claims — a leaf whose twig fell below
    // `MIN_BODY_CELLS`, or one hanging off nothing — still scatters, so the
    // third tier is intact and is now a *consequence* of the same draw
    // rather than a separate pass over the region.
    let extent = {
        let half = region
            .iter()
            .fold((i32::MAX, i32::MIN, i32::MAX, i32::MIN), |(x0, x1, y0, y1), &(x, y)| (x0.min(x), x1.max(x), y0.min(y), y1.max(y)));
        ((half.1 - half.0) / 2).max((half.3 - half.2) / 2)
    };
    // **One hinge for the whole severance, computed before anything is
    // lifted**, and this is what makes the fragments a falling tree rather
    // than fifty-six independent drops. `angular_acceleration` over the
    // *entire* severed region about the cut is the tree's own — the same
    // arithmetic a single fragment gets, asked of the thing that is actually
    // pivoting. See `Hinge` for why they must share it and why gravity is
    // then replaced rather than added.
    //
    // Read here rather than inside `fracture_with_impulse` because the
    // ladder empties the grid as it goes: by the second fragment the cells
    // the sum needs are gone.
    let stump = cut_face(world, region, broke_at);
    let alpha = angular_acceleration(world, region, stump);
    // **`HINGE_PROBE=1` prints the hinge's own arithmetic**, which is the
    // only way to tell a hinge that is working from one that is pivoting
    // about the wrong point -- the two are indistinguishable on a contact
    // sheet, and that cost a render and a wrong reading here. Read `r` : if
    // the centre of mass is level with the pivot rather than above it, the
    // swing is a fall whatever `alpha` says.
    if std::env::var("HINGE_PROBE").is_ok() {
        let (mut mass, mut torque, mut inertia, mut lift) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        for &(cx, cy) in region {
            let m = world.materials.density(world.get(cx, cy).material) as f64;
            let (dx, dy) = ((cx - stump.0) as f64, (cy - stump.1) as f64);
            mass += m;
            torque += m * dx;
            inertia += m * (dx * dx + dy * dy);
            lift += m * dy;
        }
        let mass = mass.max(1.0);
        println!(
            "  HINGE: {} cells, mass {mass:.0}, stump {stump:?} (broke_at {broke_at:?}), \
             centre of mass r = ({:.1}, {:.1}), sum(m*r2) {inertia:.0}, alpha {alpha:.3e} rad/f2",
            region.len(),
            torque / mass,
            lift / mass,
        );
    }
    let hinge = (alpha != 0.0).then_some(Hinge { pivot: stump, alpha, omega: 0.0 });
    let (promoted, grit) = fracture_with_impulse(world, region, None, size_bias(extent), Some(broke_at), true, hinge);
    ((promoted + grit) as u32, promoted as u32)
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
fn take_fragment(world: &World, left: &mut HashSet<(i32, i32)>, seed: (i32, i32), target: usize, diagonal: bool) -> Vec<(i32, i32)> {
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
        // **Four or eight, decided by the seed's own material.** See
        // `NEIGHBOURS_8` and `MaterialDef::woody`: rock is four on purpose
        // and organism tissue must be eight because that is the
        // neighbourhood `Grow` wrote it at. Read once per fragment from the
        // resolved `Material`, never per cell and never by name.
        //
        // A slice of the same array either way, so the eight-case costs one
        // extra bound and nothing at four.
        let ring: &[(i32, i32)] = if diagonal { &NEIGHBOURS_8 } else { &NEIGHBOURS_4 };
        for &(dx, dy) in ring {
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
            // A **diagonal** step crosses no single edge, so it is asked
            // about the two L-shaped routes that get to the same cell and
            // admitted if either is clear. `edge_is_cracked` answers
            // `false` for a diagonal offset, so without this a crack could
            // never shape a piece of anything flooding at eight -- the
            // seams would be there and the flood would walk round the
            // corner of every one of them.
            let blocked = if dx != 0 && dy != 0 {
                let via_x = super::structural::edge_is_cracked(world, x, y, dx, 0) || super::structural::edge_is_cracked(world, x + dx, y, 0, dy);
                let via_y = super::structural::edge_is_cracked(world, x, y, 0, dy) || super::structural::edge_is_cracked(world, x, y + dy, dx, 0);
                via_x && via_y
            } else {
                super::structural::edge_is_cracked(world, x, y, dx, dy)
            };
            if blocked {
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

/// One quarter turn, in radians. `ChunkBody::spin` counts quarter turns
/// because that is the only rotation a cell grid can hold exactly, so every
/// angular quantity arriving from physics has to be divided by this on the
/// way in.
const QUARTER_TURN: f32 = std::f32::consts::FRAC_PI_2;

/// How hard the piece `cells` is being turned about the joint that gave
/// way, in quarter-turns per frame squared and signed — positive clockwise,
/// which on a `y`-down screen means mass hanging to the right of the break.
///
/// `alpha = g * sum(m*d) / sum(m*r^2)`, with `d` the **horizontal** lever
/// arm from the break (a vertical force has no moment about a vertical
/// arm) and `r` the full distance. Nothing here is tuned.
///
/// # Why this and not the breaking torque
///
/// Seeding from torque alone gives `spin` proportional to `m*d`, so the
/// heaviest piece spins hardest — backwards, and the thing `SPIN_PER_SPEED`
/// records being tuned away from. Dividing by the second moment is what
/// turns it around: for a uniform limb of `L` cells breaking at one end the
/// sums are `L(L+1)/2` over `L(L+1)(2L+1)/6`, so `alpha = 3g/(2L+1)` —
/// **inversely proportional to length**, converging on the textbook
/// `3g/2L`. A bole comes over about once across a fifty-cell drop and a twig
/// tumbles several times in the same fall, with no constant in between.
///
/// # It self-limits, and that is the parallel-axis theorem doing it
///
/// `sum(m*r^2)` about the break is `I_com + M*D^2` for a piece whose centre
/// is `D` away, so a small fragment far from the break gets `g/D` (it is on
/// the end of a long lever and swings slowly) and one *at* the break gets
/// `g*M*D/I_com`, which goes to zero as `D` does. The maximum over `D` is
/// `g/2k` for a piece of gyration radius `k` — so the smaller the fragment
/// the faster it can tumble, and nothing can be seeded into a spin its own
/// size cannot express. `MAX_SPIN_RATE` bounds what is left.
///
/// Accumulated in `f64`: a 400-cell body 300 cells from the break sums
/// terms of order 90,000, and `f32` has 24 bits of mantissa.
fn angular_acceleration(world: &World, cells: &[(i32, i32)], broke_at: (i32, i32)) -> f32 {
    let (bx, by) = broke_at;
    let (mut torque, mut inertia) = (0.0f64, 0.0f64);
    for &(cx, cy) in cells {
        let mass = world.materials.density(world.get(cx, cy).material) as f64;
        let (dx, dy) = ((cx - bx) as f64, (cy - by) as f64);
        torque += mass * dx;
        inertia += mass * (dx * dx + dy * dy);
    }
    if inertia <= 0.0 {
        // Every cell of the piece is the break cell itself. There is no
        // lever, so there is no turn -- and no division either.
        return 0.0;
    }
    (GRAVITY as f64 * torque / inertia) as f32
}

/// Lift `cells` out of the grid as one coherent falling body.
fn promote(world: &mut World, cells: &[(i32, i32)], impulse: Option<((f32, f32), f32)>, broke_at: Option<(i32, i32)>, hinge: Option<Hinge>) {
    let (ox, oy) = cells[0];
    let body_cells: Vec<BodyCell> = cells
        .iter()
        .map(|&(cx, cy)| {
            let cell = world.get(cx, cy);
            BodyCell { dx: cx - ox, dy: cy - oy, material: cell.material, shade: cell.shade, organism_id: cell.organism_id() }
        })
        .collect();
    // Read before the grid is emptied, since that write is what clears the
    // organism id.
    let organism_cells: Vec<bool> = cells.iter().map(|&(cx, cy)| world.get(cx, cy).organism_id() != 0).collect();
    // **The seed for the fall**, and read here for the same reason: it
    // weighs each cell by its own material, and the write below empties
    // them.
    //
    // `None` is "there is no single joint that gave way", and it leaves the
    // body turning exactly as it did before this existed. Every production
    // caller has a joint today -- the collapse path passes `Failure::at`,
    // and `strike`, `calve_collar` and `fracture_shell` each pass their own
    // origin -- so this arm is the shape of the argument rather than a live
    // case, and it is the door a genuinely origin-less event would come in
    // through.
    // Read once: the struct literal below moves `body_cells`, and both the
    // spin seed and the hinge are gated on the same question.
    let falls = fall_enabled() && is_tissue(&body_cells);
    // Divided into quarter turns here, because `spin` is the only thing in
    // the engine that counts them -- `angular_acceleration` is radians, and
    // so is `Hinge::alpha`, which takes it unconverted.
    let spin_accel = broke_at.filter(|_| falls).map_or(0.0, |at| angular_acceleration(world, cells, at) / QUARTER_TURN);
    let heading = Turn::of(spin_accel);
    for &(cx, cy) in cells {
        world.set(cx, cy, Cell::EMPTY);
    }
    // **Not around tissue that is already leaving.** The organism support
    // search is hop-bounded, so a check fired inside a crown that is coming
    // down reads everything past the span limit as unsupported and converts
    // it to deadwood -- `CLAUDE.md`'s structural-check amputation gotcha,
    // firing from *inside* the fall, which is the one place it can do the
    // most damage: the piece the ladder has just built gets taken apart
    // behind it. `Reports/felling-blockers.md` §3 step 4 predicted this and
    // `Reports/physical-trees-design-2026-08-23.md` §5.5 is why
    // `BodyCell::organism_id` exists to make it testable.
    //
    // Inert cells keep their check, and that half is load-bearing too: a
    // stone ledge that has just lost what was resting on it has genuinely
    // changed and must be asked again. The pair is guarded by
    // `promoting_tissue_does_not_schedule_checks_into_the_crown_it_left`.
    for (&(cx, cy), &was_organism) in cells.iter().zip(&organism_cells) {
        if !was_organism {
            world.schedule_structural_check_around(cx, cy);
        }
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
        pivot: ChunkBody::centre_of(&body_cells),
        cells: body_cells,
        x: ox as f32,
        y: oy as f32,
        vx,
        vy,
        // The starting tilt, now pointed the way the piece is going to
        // turn: a body seeded anticlockwise that began at +0.6 would have
        // to unwind six-tenths of a quarter turn before its first one.
        spin: heading.sign() * ((ox + oy) % 3) as f32 * 0.3,
        spin_rate: 0.0,
        spin_accel,
        topples: 0,
        // **Every fragment of one severance carries the same hinge**, which
        // is the whole of what makes them one falling tree rather than
        // fifty-six independent drops. See `Hinge`.
        hinge: hinge.filter(|_| falls),
        stalled: 0,
        peak_speed: 0.0,
        reserved: false,
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
    // Taken *after* the push, so the walk below reads a world in which the
    // piece has already been lifted out and cannot find its own cells.
    let towards_liquid = falling_towards_liquid(world, cells);
    if towards_liquid {
        let footprint: HashSet<(i32, i32)> = cells.iter().copied().collect();
        claim_footprint(world, &footprint);
        if let Some(body) = world.chunk_bodies.last_mut() {
            body.reserved = true;
        }
    }
}

/// One cell of a promoted body's footprint: materially empty, and **held**
/// so that nothing else moves into the space the body is standing in.
///
/// # Why a body's own footprint has to be reserved
///
/// A promoted body is lifted out of the grid, and for most of this engine's
/// life the cells it left behind were written plain `Cell::EMPTY`. That is
/// the space the body is *in*, and writing it empty tells the CA sweep,
/// `try_move`, `particle::land` and every other body that it is free. In
/// air nothing notices. In water it is fatal: water pours into the body's
/// own footprint, so the body finds fluid in front of it with nowhere to go,
/// stalls, is re-rasterized, is judged unsupported and fractures again --
/// and again. Measured on `scene=rockdrop`, a 600-cell slab produced
/// **2,515 cells' worth of chunks** on its way down and left `rock -600,
/// rubble +572`: the same rock broken four times over, which is what
/// reached play as *"chunks of rock hit the water and then start
/// disintegrating into grit."* Only 2,834 of 10,849 displacement attempts
/// succeeded, and a printed walk showed water standing inside the body's
/// own footprint.
///
/// `FLAG_MANAGED` is exactly the reservation this needs and already exists
/// for the liquid heightfield bodies: `Cell::is_empty` is managed-aware, so
/// one flag closes the footprint to all of the above at once, and
/// `demote_body_at` is a no-op for it because `body_index` only holds
/// liquid bodies.
///
/// **It only works paired with `exchange_with_fluid`.** Reserving the
/// footprint on its own was built once and lost 1,821 cell-equivalents of
/// water, because every search that hands displaced fluid somewhere to go
/// (`make_way_behind`'s walk, `settle`'s `nearest_free`/`surface_above`)
/// reads `is_empty` and so can no longer see the cells the body is
/// vacating -- which are the one place that fluid belongs. See
/// `Reports/open-bugs-handoff.md` §1h.
fn reserved() -> Cell {
    Cell::EMPTY.with_managed(true)
}

/// How far below a new piece to look for the liquid it is heading into.
///
/// Sets how early a body takes its footprint (`ChunkBody::reserved`) — and
/// therefore how much of its fall is protected from other pieces. Reserving
/// only on *contact* costs `scene=rockdrop` 133 cells of the slab (242 of
/// 600 surviving as rock against 375), because bodies shed cells into each
/// other's unreserved footprints on the way down. Reserving *always* costs
/// a dry scene 5.5x its worst frame — see `ChunkBody::reserved` and
/// `Reports/open-bugs-handoff.md` §1j. Forty-eight rows is a long drop at
/// this world's scale and still terminates fast on a dry scene, where the
/// walk hits the ground within a few cells.
const LIQUID_LOOKAHEAD: i32 = 48;

/// Whether any column of a new piece's underside finds liquid before it
/// finds anything else, within `LIQUID_LOOKAHEAD` rows.
///
/// The walk stops at the **first non-empty cell** in each column, so it is
/// asking "what is this piece going to land in", not "is there water
/// somewhere below". Rock over a floor with a pond under the floor reads
/// false, which is right: it will never touch the pond.
fn falling_towards_liquid(world: &World, cells: &[(i32, i32)]) -> bool {
    let own: HashSet<(i32, i32)> = cells.iter().copied().collect();
    let mut lowest: HashMap<i32, i32> = HashMap::new();
    for &(x, y) in cells {
        let e = lowest.entry(x).or_insert(y);
        *e = (*e).max(y);
    }
    for (&x, &y) in &lowest {
        for dy in 1..=LIQUID_LOOKAHEAD {
            let (nx, ny) = (x, y + dy);
            if own.contains(&(nx, ny)) {
                continue; // still inside the piece's own outline
            }
            if !world.in_bounds(nx, ny) {
                break;
            }
            let material = world.get(nx, ny).material;
            if material == super::material::EMPTY {
                continue; // open air: keep looking down this column
            }
            if world.materials.kind(material) == MaterialKind::Liquid {
                return true;
            }
            break; // something solid first: this column lands on that
        }
    }
    false
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
    if !convert_to_debris(world, x, y) {
        return; // no configured debris: left exactly as it was
    }
    // After the conversion, never before the `breaks_into` check above:
    // the decline leaves the cell exactly as it was, and counting a
    // decline would make grit look like it happened. Grit is half of the
    // "a few blocks, more cobbles, a lot of grit" distribution and its
    // pair, `promoted_cells`, is the other -- neither number says anything
    // about the shape of a break on its own.
    world.structural_failures.record_shattered(1);
    world.schedule_structural_check_around(x, y);
}

/// The conversion itself: `(x, y)` becomes its material's `breaks_into`,
/// unattached, at the same temperature. Returns whether anything happened.
///
/// Split out of `shatter_to_rubble` because the *bookkeeping* around it is
/// not universal even though the write is. Rock grit is recorded and
/// schedules a check on its neighbours; organism grit does neither, and
/// both of those are load-bearing rather than an omission --
/// `FailureCounts::shattered_cells` excludes the organism path on purpose,
/// and a check scheduled inside a crown that is coming down amputates what
/// is left of it (`CLAUDE.md`'s structural-check gotcha). This is the part
/// they agree on.
fn convert_to_debris(world: &mut World, x: i32, y: i32) -> bool {
    let cell = world.get(x, y);
    let Some(into) = world.materials.get(cell.material).breaks_into else {
        return false; // no configured debris: leave it rather than deleting content
    };
    let shades = world.materials.get(into).palette.len().max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    let temp = cell.temperature();
    world.set(x, y, Cell::new(into, shade).with_temperature(temp));
    true
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
            // model: the bite is 7 cells wide and the cracks run 21, and
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
            // **Living tissue is cut like anything else.** This used to
            // read `|| world.get(x, y).organism_id() != 0`, which meant the
            // chisel could not touch a tree at all: a gnome could bore
            // through granite and not through a sapling
            // (`open-bugs-handoff.md` §D1). The exclusion was defensive --
            // an organism cell's `aux` is a cell-type tag, not a structural
            // distance -- and nothing here reads `aux`. `shatter_to_rubble`
            // goes through `World::set`, which unregisters the cell from
            // its organism on the way, and `plant::anchor_support` then
            // re-walks the plant from its anchors on the organism's next
            // tick and finds whatever the cut severed unreached.
            if !is_tool_target(world, x, y) {
                continue;
            }
            shatter_to_rubble(world, x, y); // the cut itself: material, not vacuum
            loosened.push((x, y));
        }
    }
    finish_cut(world, &loosened, (cx, cy), radius, spoil_yield)
}

/// `mine`, cutting an axis-aligned **rectangle** rather than a disc or a
/// capsule. Bounds are inclusive and may be given in either order.
///
/// # Why a rectangle exists beside the capsule
///
/// The capsule is the free-hand cut: point anywhere, take a round bite out
/// of the near face. It makes tunnels of a sort, and the sort is a
/// *worm-hole* -- the bore wanders wherever the cursor was, its walls are
/// round, and how much clearance it leaves is something you find out by
/// walking into it. The gnome's default dig is a rectangle for the reason
/// `player::bore_rect` sets its size from `PLAYER_HEIGHT`: a passage cut to
/// the shape of the thing that has to walk down it is a passage you can
/// *see* is walkable before you cut it, and a corridor with flat walls and
/// a flat floor reads as hewn rather than as gnawed.
///
/// The crack, pressure and disturbance feedback below is the capsule's,
/// keyed on the rectangle's half-diagonal so a big square shakes more air
/// than a small one -- the same relationship the capsule has to its radius.
pub fn mine_rect(world: &mut World, a: (i32, i32), b: (i32, i32), spoil_yield: f32) -> usize {
    let (lo_x, hi_x) = (a.0.min(b.0), a.0.max(b.0));
    let (lo_y, hi_y) = (a.1.min(b.1), a.1.max(b.1));
    let centre = ((lo_x + hi_x) / 2, (lo_y + hi_y) / 2);
    // Half the shorter side, which is the largest disc the rectangle
    // contains -- so a long corridor does not claim the crack reach of a
    // blast just for being long.
    let radius = (((hi_x - lo_x) / 2).min((hi_y - lo_y) / 2)).max(1);
    let mut loosened = Vec::new();
    for y in lo_y..=hi_y {
        for x in lo_x..=hi_x {
            // Same three questions the capsule asks, in the same order --
            // see `mine_swept` for why living tissue is cut like anything
            // else and why bedrock is not.
            if !world.in_bounds(x, y) || !is_tool_target(world, x, y) {
                continue;
            }
            shatter_to_rubble(world, x, y);
            loosened.push((x, y));
        }
    }
    finish_cut(world, &loosened, centre, radius, spoil_yield)
}

/// Everything a cut owes the world once its cells are rubble: cracks past
/// the cut, the bracing the removed rock was providing, a shove of air, the
/// failure licence, and the spoil thinning.
///
/// Shared by every shape of cut rather than copied per shape. The ordering
/// is load-bearing and is documented inline; `thin_to_spoil` is last so
/// everything above it sees the full bite as rubble.
fn finish_cut(world: &mut World, loosened: &[(i32, i32)], (cx, cy): (i32, i32), radius: i32, spoil_yield: f32) -> usize {
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
    for &(x, y) in loosened {
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
    thin_to_spoil(world, loosened, spoil_yield)
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

/// Returns **how many cells the blow actually acted on** — pulverized plus
/// loosened. Not decorative: a swing at open air, a swing that lands on
/// bedrock and a swing that calves a slab are three different events that
/// look identical from the call site, and `CLAUDE.md`'s rule about pairing
/// an "it fired" counter with an effect counter from the far side of the
/// call is what this return value is. `player::smash` prints it; `App::
/// strike` ignores it.
/// **A blow that lands on a piece already broken off bursts it into
/// grit**, and returns how many cells that took.
///
/// Owner's verdict on the hammer, 2026-08-29: *"have the pieces fall off
/// in chunks. if the chunks are hit, they break into stone dust."* The
/// second half could not happen at all before this, and the reason is not
/// obvious from `strike`: `promote` writes `Cell::EMPTY` into the grid and
/// holds a flying body's footprint as a **managed-empty reservation**, so
/// `world.get` at a chunk's position reports empty, `is_tool_target` says
/// no, and a swing passes clean through the rock it just knocked loose.
/// The one thing in the world a hammer most obviously ought to hit was the
/// one thing it could not.
///
/// **The whole body goes, not the cells the disc covers.** Taking a bite
/// out of a body means re-deriving its shape, mass distribution and spin
/// about a hole, and the felt result would be worse anyway: a piece you
/// hit should *burst*, which is what the complaint describes. So a body
/// with any cell inside the blow is settled back into the grid where it
/// is and every cell of it shattered — stone to rubble, a log to
/// deadwood, by each cell's own `breaks_into`, since `shatter_to_rubble`
/// reads the material and not a constant.
///
/// `settle` is reused rather than reimplemented: it already gives the
/// footprint back before writing into it, relocates a cell whose square is
/// occupied, and schedules the checks. A relocated cell keeps its own
/// material rather than becoming grit — the position `shatter_to_rubble`
/// is then aimed at is not where it ended up — which is a handful of cells
/// on a large body and not worth a second index to fix.
fn burst_bodies_at(world: &mut World, cx: i32, cy: i32, radius: i32) -> usize {
    // Taken out and put back for the borrow shape `step_chunk_bodies`
    // records: settling a body needs `&mut World` while the loop holds the
    // collection.
    let taken = std::mem::take(&mut world.chunk_bodies);
    let mut kept = Vec::with_capacity(taken.len());
    let mut burst = Vec::new();
    for body in taken {
        let hit = body.cells.iter().any(|c| {
            let (x, y) = body.cell_position(c);
            let (dx, dy) = (x - cx, y - cy);
            dx * dx + dy * dy <= radius * radius
        });
        if hit {
            burst.push(body);
        } else {
            kept.push(body);
        }
    }
    // Put the survivors back *before* settling, so a body that settling
    // brings down lands in a live collection rather than being discarded
    // — the same trap `step_chunk_bodies`' `extend`-not-assign guards.
    world.chunk_bodies = kept;
    let mut dusted = 0;
    for body in &burst {
        settle(world, body);
        for cell in &body.cells {
            let (x, y) = body.cell_position(cell);
            if world.in_bounds(x, y) {
                shatter_to_rubble(world, x, y);
            }
        }
        dusted += body.cells.len();
    }
    dusted
}

/// **Take out every joint-bounded block within `reach` whose outline the
/// damage has parted all the way round**, thrown from `origin` at `force`.
/// Returns the cells released.
///
/// Shared by the two events that open joints — `strike` (the hammer and the
/// sandbox blow) and `explosion::Blast::calve` — because the alternative is
/// two copies of "and now promote what the cracks cut free" that drift the
/// first time either changes, which is the mistake `loosen_shell`'s own doc
/// records this file having made before.
///
/// # Why this is not the load model's job
///
/// Because the load model is *right* and the answer is still wrong. Measured
/// by the explosion lane on `worldcrack rolling 7`: a 108-cell slab, cut free
/// on every side by the fissures, reports **104 of 108 cells holding** at a
/// budget nothing can exhaust — because five of its cells rest on rubble, and
/// a boulder resting on gravel genuinely does not fall. Waiting for
/// "unsupported" to fire is waiting for something that should not fire. The
/// trigger is **severance**, not unsupportedness: a piece the cracks have cut
/// out has stopped being terrain, whether or not it then moves.
///
/// So expect several released pieces to barely travel — their own seam grit
/// fills the gap they would drop into. **Count promotions, not
/// displacement.** What the release buys even when nothing moves is that the
/// piece is now a body: it can be struck, shoved, ridden and settled.
///
/// # The bound is the event's own reach, deliberately
///
/// `dead-ends.md` records a complete lattice with no ligaments being tried
/// and rejected: every block became an island, 2,400 confined failures per
/// 400 frames, the world dead still. Releasing *every* enclosed domain in
/// the world is that failure's cousin. `reach` keeps it to the rock this
/// event actually damaged, and `MIN_BODY_CELLS` keeps a released block a
/// piece rather than grit.
pub(crate) fn calve_free_blocks(world: &mut World, origin: (i32, i32), reach: f32, force: f32) -> usize {
    let blocks = super::structural::free_blocks_around(world, origin, reach);
    let mut freed = 0;
    for block in &blocks {
        if block.len() < MIN_BODY_CELLS {
            continue; // below a piece; leave it for the cascade to grit
        }
        freed += block.len();
        // `promote` directly, never `fracture_with_impulse`: that ladder is
        // joint-blind and is exactly what was re-cutting a bounded block
        // into fragments, so the outline and the piece were different
        // shapes.
        // **No hinge.** A `Hinge` is for a piece still pivoting on what it
        // broke off (`fell_severed_tissue`'s crown on its stump); a block
        // the joints have cut out is free on *every* side by definition —
        // that is what `free_blocks_around` tests — so there is nothing
        // for it to swing about. It leaves on the blow's impulse alone.
        promote(world, block, Some(((origin.0 as f32, origin.1 as f32), force)), Some(origin), None);
    }
    freed
}

pub fn strike(world: &mut World, cx: i32, cy: i32, radius: i32, force: f32) -> usize {
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
    // `BLOW_JOINT_REACH`, matching what the joints below actually reach --
    // the licence covers exactly the rock this blow damaged, which is the
    // same argument as recording it after the radius floor rather than
    // before. It was `CRACK_REACH` while a blow drew rays to that reach.
    world.record_disturbance(cx, cy, radius * BLOW_JOINT_REACH);
    // Three zones, and the split is what makes a blow read as a blow rather
    // than as a hole appearing. The core is pulverized -- that is the bite.
    // A thin shell around it chips off immediately, so every hit produces
    // visible flying rock whether or not anything structural gives way. Past
    // that the rock is *scored*: damage that shows and accumulates without
    // detaching, which is the state the whole mechanic was missing.
    // **Pieces already in flight, first.** A chunk the blow overlaps is
    // burst into grit where it is (`burst_bodies_at`), and it runs before
    // the grid loop below so those cells are back in the world in time to
    // be counted and to be seen. Without this a swing at the rock you just
    // knocked loose does literally nothing — see that function's doc for
    // why the grid says the cell is empty.
    let dusted_bodies = burst_bodies_at(world, cx, cy, radius);
    // **No pulverized core.** Owner's instruction, 2026-08-29: *"maybe as
    // a first step just no dust, it forms cracks and then large pieces
    // fall specifically from the existing crack line."* A blow used to
    // `shatter_to_rubble` a `radius / 3` disc outright, which is grit made
    // by fiat rather than by anything breaking, and it is the first thing
    // the eye sees -- it appears on the swing frame, where a piece has to
    // wait for its outline to close. What is left is the honest ladder:
    // the whole disc is *loosened*, and whatever comes off it below
    // `MIN_BODY_CELLS` is grit as a **consequence** of the draw, which is
    // the distinction `fracture`'s own doc draws.
    //
    // Zero rather than deleted, so the three zones are still legible and
    // putting a core back is one number.
    let core = 0;
    // **The chip zone is the blow's whole radius**, and it used to be two
    // thirds of it. Reported from playtest of the gnome's hammer: *"it
    // mostly makes big strike lines instead of breaking rock into
    // pieces"*, and the sheet said exactly that -- a starburst of five
    // thin black rays across a couple of dozen cells of untouched grey
    // rock, with no visible wound at the middle of it.
    //
    // The arithmetic is the whole complaint. At the gnome's `hammer_
    // radius: 7` the old split gave a chip of 4 against cracks running
    // `radius * CRACK_REACH` = 21, so the *visible line* was 17 cells long
    // and the *visible damage* was 4 -- a bit over four to one, in favour
    // of the thing that is only a mark. Taking the chip to the full radius
    // makes it 14 against 7, and it does it by growing the half that is
    // rock coming apart rather than by shortening the half that reads as
    // force reaching past the wound (`CRACK_REACH`'s own doc, and the
    // reason it was left alone).
    //
    // It also fixes the *count* of pieces, which is the other half of
    // "into pieces". The loosened ring was ~37 cells at r7, against a
    // fragment ladder that draws 4..64 at `size_bias(7) == 1` -- so one
    // seed routinely swallowed the lot and a blow calved a single lump.
    // Measured on `scene=smash`: 20 cells as chunks over 2 bodies at two
    // blows. The full-radius ring is ~140 cells, which is several draws
    // off the same ladder: a few blocks, more cobbles, a lot of grit,
    // which is `CLAUDE.md`'s first law and was the one thing the old
    // split could not produce.
    let chip = radius.max(core + 1);
    let mut loosened = Vec::new();
    let mut pulverized = 0;
    for dy in -chip..=chip {
        for dx in -chip..=chip {
            let (x, y) = (cx + dx, cy + dy);
            let d2 = dx * dx + dy * dy;
            if d2 > chip * chip || !world.in_bounds(x, y) {
                continue;
            }
            // Living tissue too -- see `mine_swept`'s note on why the
            // `organism_id() != 0` exclusion that used to be here went.
            // This is the axe: without it the pick could break a mountain
            // and not a twig, and `design-philosophy.md` §0a's "no verb
            // behind the effect" applied to every plant in the world.
            if !is_tool_target(world, x, y) {
                continue;
            }
            if core > 0 && d2 <= core * core {
                // Counted by **what changed**, not by what was attempted:
                // `shatter_to_rubble` declines a material with no
                // `breaks_into` and leaves the cell exactly as it was, and
                // a count that included the declines would report a blow
                // on unbreakable rock as a hit.
                let was = world.get(x, y).material;
                shatter_to_rubble(world, x, y); // the bite
                pulverized += usize::from(world.get(x, y).material != was);
            } else {
                loosened.push((x, y));
            }
        }
    }
    // **The rock's own joints, not a star of rays.** Owner's verdict on
    // the hammer, 2026-08-29: *"fully get rid of the lines that it makes
    // -- it should just make cracks similar to an explosion and have the
    // pieces fall off in chunks."* `score_cracks` drew five straight
    // radial rays out to `radius * CRACK_REACH`, keyed on the site so
    // repeats deepened them, and at the gnome's `hammer_radius: 7` that
    // was a 17-cell line against 4 cells of visible damage -- a starburst
    // stamped on untouched grey rock, which is exactly the reading the
    // blast's own three rejections produced before it stopped drawing
    // walkers at `d5cb19a` (`fracture_field`'s module doc). The blast
    // reads the joint fabric now; so does a blow.
    //
    // The reach is unchanged, and so is the ramp's shape: joints part
    // densely at the impact and thin out to nothing, so the damaged patch
    // is graded and ragged-edged rather than a disc with a rim.
    // `structural::shatter_joints_around` also unbraces and schedules what
    // it severed, which is what turns an enclosed domain into a piece that
    // falls -- the half `score_cracks` carried in its own loop.
    //
    // `score_cracks` itself stays: `mine` still uses it at `radius +
    // MINE_CRACK_REACH`, a short enough reach that it reads as a cut
    // fraying its edges rather than as lines drawn on rock.
    // Flat at full density out to the chip radius, then ramping to
    // nothing at `radius * CRACK_REACH`. The flat zone is the half that
    // makes pieces rather than dashes: everything inside `chip` is already
    // gone, so the first joints a blow can reach are at exactly that
    // radius, and under a pure ramp they part at well under 1 and enclose
    // nothing. Held at full density, the ring of domains around the wound
    // closes and falls out.
    super::structural::shatter_joints_around(world, (cx, cy), (radius * BLOW_JOINT_REACH) as f32, chip as f32);
    // **And now take out whatever the cracks have completely surrounded,
    // as whole blocks along the outline the player can see.**
    //
    // Owner's verdict, 2026-08-29, on the sheet of the reveal plus the
    // repeat bonus: *"The chunks should break off along the crack pattern
    // that is already there. this mostly looks like small dust breaking
    // first... it forms cracks and then large pieces fall specifically
    // from the existing crack line when they completely surround a
    // chunk."*
    //
    // The enclosure was already real -- a crack is a place support cannot
    // cross, so a fully bounded block is genuinely unsupported and the
    // cascade did find it. What it did with it is the problem:
    // `structural::tick` hands a failing region to `fracture`, which
    // re-cuts it on the power-of-two ladder, so a 170-cell block bounded
    // by four visible joints came apart into ladder-sized fragments and
    // grit. The outline and the piece were different shapes, which is
    // exactly what "the chunks should break off along the crack pattern"
    // says is wrong. Promoting the block whole makes them the same shape.
    //
    // Each block goes through `promote` directly rather than through
    // `fracture_with_impulse`: the ladder is what would break it up again.
    let freed = calve_free_blocks(world, (cx, cy), (radius * BLOW_JOINT_REACH) as f32, force);
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
        fracture_with_impulse(world, &loosened, Some(((cx as f32, cy as f32), force)), size_bias(radius), Some((cx, cy)), false, None);
    }
    // Every destructive event owes feedback (`Reports/design-philosophy.md`
    // §0a). A blow shoves the air as well as the rock, which is what makes
    // smoke and loose grit near the impact react instead of a hit landing in
    // total silence.
    world.add_pressure_impulse(cx, cy, radius.max(2), force * STRIKE_PRESSURE);
    for &(x, y) in &loosened {
        world.schedule_structural_check_around(x, y);
    }
    pulverized + loosened.len() + dusted_bodies + freed
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
        fracture_with_impulse(world, &loosened, Some(((origin.0 as f32, origin.1 as f32), force)), size_bias, Some(origin), false, None);
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
    fracture_with_impulse(world, &loosened, Some(((origin.0 as f32, origin.1 as f32), force)), size_bias, Some(origin), false, None);
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
    // **Swinging on the stump, or falling.** Never both: `alpha` is computed
    // from gravity, so a hinged body that also took `GRAVITY` here would
    // count the same force twice and sag out of its own arc. See `Hinge`.
    if !swing_on_hinge(body) {
        body.vy += GRAVITY;
    }

    // Tip while falling. Two terms, and they answer different questions.
    //
    // **The rate is the physics** -- `spin_accel` is what the break is
    // doing to this piece, read off its own mass distribution about the
    // joint that gave way (`angular_acceleration`), and integrated here
    // because a piece that has come loose is still being turned by its own
    // weight about that joint. A felled trunk pivoting on its stump is
    // exactly this and nothing else.
    //
    // **The speed term is the texture**, and it is left alone. It
    // accumulates with speed, so a piece that has barely come loose turns
    // slowly and one in a long drop tumbles; a body already at rest does
    // not rotate at all, which matters because a settled chunk snapping
    // through 90 degrees reads as a glitch rather than as physics. What is
    // new is only that it now carries the *sign* of the rate, so the two
    // terms cannot fight: a body seeded to go anticlockwise is not dragged
    // back the other way by how fast it is falling. With no seed at all --
    // a blast, which has no single failing cell to break about --
    // `Turn::of(0.0)` is `Cw` and this line is byte-for-byte what it was.
    body.spin_rate = (body.spin_rate + body.spin_accel).clamp(-MAX_SPIN_RATE, MAX_SPIN_RATE);
    let heading = Turn::of(body.spin_rate);
    body.spin += body.spin_rate + heading.sign() * (body.vx.abs() + body.vy.abs()) * SPIN_PER_SPEED;
    // Read off the accumulated *angle*, never off the rate: `spin` is what
    // has actually been banked, and a body whose rate has just changed sign
    // still owes the turn it was part-way through.
    let due = if body.spin >= 1.0 {
        Some(Turn::Cw)
    } else if body.spin <= -1.0 {
        Some(Turn::Ccw)
    } else {
        None
    };
    if let Some(turn) = due {
        body.spin -= turn.sign();
        // Only turn if the turned shape actually fits. Otherwise a body
        // wedged in a gap would rotate straight through the wall beside it,
        // which is the one way this transform can cheat.
        //
        // Asked of the **pre-turn** body: `mean_density` and `body_floats`
        // read each `BodyCell`'s own material, so a quarter turn — which
        // permutes offsets and neither adds nor removes a cell — cannot
        // move either, and `rotation_fits` does not read `reach`. The
        // version this replaces built a rotated clone purely to compute a
        // shape identical to this one, and then asked the wrong question
        // with it; see `rotation_fits` for both halves of that.
        let turned = rotation_fits(world, body, turn, shape_of(world, body));
        // Counted, because a probe that always answers "clear" looks
        // exactly like a probe that works — which is how this went unnoticed
        // for the life of the mechanism. `CLAUDE.md`: "did it fire at all"
        // needs a counter, not a picture.
        world.structural_failures.record_rotation(turned);
        if turned {
            apply_turn(world, body, turn);
        }
    }

    // **Water pushes back.** Until this, nothing did: a body kept the
    // `GRAVITY` it had accumulated in the air all the way to the bottom of
    // a pond, and the only thing that ever took speed off it was hitting
    // something. Reported from play, on the very drop the piece-integrity
    // fix was meant to improve: *"it falls at slightly odd rates. There is
    // a first group of chunks that drop too fast and then the rest that
    // come together with the grit later."*
    //
    // Measured on `scene=rockdrop`: pieces reach **6.00 cells/frame** in
    // the twenty rows of air -- the `MAX_SPEED_PER_AXIS` clamp -- and go
    // into the water still carrying it, while the rubble they came from
    // sinks under one. That is the two groups.
    if let Some(fluid) = surrounding_liquid(world, body) {
        let density = world.materials.density(fluid);
        drag_through_liquid(body, shape_of(world, body), density);
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
    // Computed once per advance rather than inside `blocked_axis`, which
    // runs per substep and again for the rotation probe. Both are O(cells)
    // and neither can change while the body is in flight -- a quarter turn
    // permutes the offsets and does not add or remove one.
    let shape = shape_of(world, body);
    // Taken before the substep loop, because a collision damps `vy` by
    // `COLLISION_RETENTION` on the way through -- the same reason
    // `peak_speed` is recorded rather than read at landing.
    let entry_speed = body.vy;
    let mut moved = false;
    for _ in 0..steps {
        let (next_x, next_y) = (body.x + step_x, body.y + step_y);
        let step = try_step(world, body, next_x, next_y, shape);
        match step.axis {
            None => {
                // **The reservation is latched here, on first contact with
                // fluid**, and only then -- see `ChunkBody::reserved` for
                // what holding it from birth cost a dry scene. Claimed
                // before the exchange, so the cells the body is giving up
                // are already materially empty when the exchange looks for
                // somewhere to put the water.
                if !body.reserved && !step.swaps.is_empty() {
                    claim_footprint(world, &step.occupied);
                    body.reserved = true;
                }
                if body.reserved {
                    exchange_with_fluid(world, &step.occupied, step.motion, &step.swaps, shape.reach);
                    restamp_footprint(world, &step.occupied, step.motion);
                }
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
                    Axis::Vertical => {
                        body.vy *= COLLISION_RETENTION;
                        landed(body);
                    }
                    Axis::Both => {
                        body.vx *= -COLLISION_RETENTION;
                        body.vy *= COLLISION_RETENTION;
                        landed(body);
                    }
                }
                break;
            }
        }
    }

    if moved {
        report_entry_splash(world, body, entry_speed);
    }
    body.stalled = if moved { 0 } else { body.stalled.saturating_add(1) };
    if body.stalled < STALL_FRAMES_BEFORE_SETTLING {
        return true;
    }
    // It has stopped moving. Before it becomes terrain, ask whether it is
    // standing on anything it could actually stand on.
    topple(world, body)
}

/// Advance the body's swing about its stump by one frame, returning whether
/// it is on one at all.
///
/// The velocity is **set**, not accumulated: a point on a rigid body turning
/// about a fixed pivot has `v = omega x r` exactly, and integrating an
/// acceleration towards that instead lets the pieces drift off the arc and
/// out of formation, which is the unzip this exists to remove.
///
/// `r` is measured from the pivot to the body's own centre, once per frame.
/// Screen `y` points down, so the clockwise tangent at `r` is `(-r.y, r.x)` —
/// the same handedness as `Turn::Cw`, and it has to be, or a crown would
/// lean one way and its pieces spin the other.
fn swing_on_hinge(body: &mut ChunkBody) -> bool {
    let Some(hinge) = body.hinge.as_mut() else {
        return false;
    };
    hinge.omega += hinge.alpha;
    let (px, py) = (hinge.pivot.0 as f32, hinge.pivot.1 as f32);
    let (cx, cy) = (body.x + body.pivot.0 as f32, body.y + body.pivot.1 as f32);
    let (rx, ry) = (cx - px, cy - py);
    body.vx = -hinge.omega * ry;
    body.vy = hinge.omega * rx;
    true
}

/// **The hinge does not survive the landing**, and a collision takes the turn
/// off a piece the same way it takes the speed off it.
///
/// Called where `vy` is already being damped by `COLLISION_RETENTION`, and
/// the pair is the whole point: the linear half was there from the start and
/// the angular half was not, because until 2026-08-29 nothing seeded a turn
/// worth damping.
///
/// **What it cost to leave out is a body that never lands.** `spin_accel` is
/// held for the whole flight (`angular_acceleration`), so a piece sitting on
/// the ground kept winding up; every couple of frames it took a quarter
/// turn, each turn let it drop or shift a cell, and dropping a cell resets
/// `stalled`, so it ratcheted along and never reached
/// `STALL_FRAMES_BEFORE_SETTLING`. Measured on `scene=fell species=tree
/// frame0=10800`: **238 bodies promoted against the control's 45**, one still
/// in flight 1,050 frames after the cut, and the harness reporting that
/// nothing ever came to rest. Caught by the negative control the handoff
/// asked for -- a settled pile that never stops moving is a chunk that never
/// sleeps, which `CLAUDE.md` prices at ~8 ms/frame because what it defeats is
/// the dirty-rect render skip.
///
/// The rate is damped rather than zeroed, matching `vy`: a rock that lands
/// tumbling keeps a little of it. The *acceleration* is zeroed outright,
/// because the joint it was turning about is behind it.
fn landed(body: &mut ChunkBody) {
    body.spin_rate *= COLLISION_RETENTION;
    body.spin_accel = 0.0;
    // **And the stump lets go.** The piece keeps whatever velocity the swing
    // gave it and is ballistic from here, which is what makes the low
    // fragments arrive first and stop while the crown is still coming over.
    body.hinge = None;
}

/// Turn the body, taking its footprint reservation with it if it holds one.
///
/// One door, so the reserved case cannot be forgotten at a new call site --
/// which is the shape of the leak `rotate_reserved`'s own doc records: a
/// turn that moved the body and not its reservation grew permanent
/// wedge-shaped air pockets inside a pond, with the ledger perfectly
/// balanced throughout.
fn apply_turn(world: &mut World, body: &mut ChunkBody, turn: Turn) {
    if body.reserved {
        rotate_reserved(world, body, turn);
    } else {
        body.rotate_quarter(turn);
    }
}

/// A piece that has come to rest out of balance goes over instead of
/// becoming terrain. Returns whether it did, in which case it stays in
/// flight for another frame.
///
/// # The outcome the load model does not have
///
/// `load::bearing_moment` **is** a tipping test — a no-tension bed that
/// reduces to "is the centre of mass inside the middle third of the contact
/// width" — and it is not what was missing. Measured
/// (`Reports/plant-mechanics-handoff-2026-08-29.md` §3.2): letting that
/// clamp reach `log` at all does not lay a single piece down, it **crushes**
/// them, because the load model's two verdicts are *holds* and *fails* and
/// failing means `breaks_into` — convert where you stand. Settled
/// lying/upright/square went 3/8/2 to 1/11/1 and `log` lost 117 cells to
/// `deadwood`. The pieces it condemned were the ones that had a footing.
///
/// So the kern test lives here instead, on the far side of the flight,
/// where the piece is still a *body* and going over is something it can
/// actually do. Nothing about the load model changes.
///
/// # Why it terminates
///
/// Three ways, and the first two are the ones that matter in a pile. A turn
/// that does not fit is refused and the body settles as it stands, wedged,
/// which is a legitimate resting state. A turn that fits usually lays the
/// piece flatter, and a flat piece's centre of mass is inside its own
/// footing, so it does not ask again. `MAX_TOPPLE_TURNS` catches the
/// remainder: two poses can each read as unbalanced in the other's
/// favour, and there is no argument from geometry that says they cannot.
fn topple(world: &mut World, body: &mut ChunkBody) -> bool {
    if !fall_enabled() || !is_tissue(&body.cells) || body.topples >= MAX_TOPPLE_TURNS {
        return false;
    }
    let Some(turn) = tipping_turn(world, body) else {
        return false;
    };
    let fits = rotation_fits(world, body, turn, shape_of(world, body));
    // Its **own** counter, not the in-flight one -- see
    // `FailureCounts::topples_asked`. The two fire for different reasons at
    // different moments, and a single number that moved could not say which
    // of them did it.
    world.structural_failures.record_topple(fits);
    if !fits {
        return false;
    }
    apply_turn(world, body, turn);
    body.topples += 1;
    body.stalled = 0;
    // The tip **is** this body's quarter turn, and the flight is over.
    // Leaving the accumulator standing would spend it again a frame later,
    // and leaving the acceleration standing would keep winding up a piece
    // that is now lying on the ground -- the hinge it was turning about is
    // behind it.
    body.spin = 0.0;
    body.spin_rate = 0.0;
    body.spin_accel = 0.0;
    true
}

/// Which way a settled body is out of balance, or `None` if it is seated.
///
/// The footing is every cell of the body with something under it that is
/// not this same body and not a fluid — `load::bearing_moment`'s reading of
/// a contact, which its own doc argues for at length: a pile is lumpy and
/// conforms, so the run of material the piece *stands on* is the width it
/// will have once the grain beneath has settled, not the patchy contact of
/// this instant. Judging the instant dismantles a slab lying on its own
/// rubble one cell at a time.
///
/// Mass-weighted, because a limb that came down with its foliage on is
/// half leaf by count and a fifth of it by weight (`leaf` is 0.25 against
/// `wood`'s 0.8), and it is the weight that decides which way it goes.
fn tipping_turn(world: &World, body: &ChunkBody) -> Option<Turn> {
    if body.cells.is_empty() {
        return None;
    }
    let own: HashSet<(i32, i32)> = body.cells.iter().map(|c| body.cell_position(c)).collect();
    let (mut mass, mut moment) = (0.0f32, 0.0f32);
    let (mut lo, mut hi) = (i32::MAX, i32::MIN);
    for cell in &body.cells {
        let (x, y) = body.cell_position(cell);
        let m = world.materials.density(cell.material).max(f32::MIN_POSITIVE);
        mass += m;
        moment += m * x as f32;
        if own.contains(&(x, y + 1)) {
            continue;
        }
        if stands_on(world, x, y + 1) {
            lo = lo.min(x);
            hi = hi.max(x);
        }
    }
    if hi < lo {
        // Nothing under any of it. The body is not resting, it is wedged or
        // hung up on its sides, and there is no footing to be eccentric to.
        return None;
    }
    let centre_of_mass = moment / mass;
    let centre_of_footing = (lo + hi) as f32 / 2.0;
    let half_width = (hi - lo + 1) as f32 / 2.0;
    let eccentricity = centre_of_mass - centre_of_footing;
    (eccentricity.abs() > half_width * TIPPING_KERN).then(|| Turn::of(eccentricity))
}

/// Whether `(x, y)` is something a piece can bear on.
///
/// Out of bounds counts: the world's own floor and walls hold a body up as
/// surely as rock does. Powder counts too — a slab lying on scree is
/// standing on the scree, which is the same reading `load::bearing_moment`
/// takes. Liquid and gas do not: nothing here stands on water.
fn stands_on(world: &World, x: i32, y: i32) -> bool {
    if !world.in_bounds(x, y) {
        return true;
    }
    let there = world.get(x, y).material;
    there != material::EMPTY && !matches!(world.materials.kind(there), MaterialKind::Liquid | MaterialKind::Gas)
}

/// Turn the body a quarter, taking its footprint reservation with it.
///
/// **A rotation is the one body transform that is not a translation**, so
/// `restamp_footprint` — which derives the vacated and entered sets from a
/// single integer offset — cannot describe it, and calling
/// `rotate_quarter` directly leaks the whole pre-turn footprint. Found by
/// looking at the artifact: `scene=rockdrop` grew wedge-shaped air pockets
/// standing permanently inside the pond, because a reserved cell is not
/// `is_empty` and no water would close over it. The ledger was perfectly
/// balanced throughout — nothing was lost, so no conservation guard could
/// have caught it, and only the picture showed it.
fn rotate_reserved(world: &mut World, body: &mut ChunkBody, turn: Turn) {
    let before: HashSet<(i32, i32)> = body.cells.iter().map(|c| body.cell_position(c)).collect();
    body.rotate_quarter(turn);
    let after: HashSet<(i32, i32)> = body.cells.iter().map(|c| body.cell_position(c)).collect();
    for &(x, y) in before.difference(&after) {
        let cell = world.get(x, y);
        if world.in_bounds(x, y) && cell.material == material::EMPTY && cell.managed() {
            world.set_owned(x, y, Cell::EMPTY);
        }
    }
    for &(x, y) in after.difference(&before) {
        if world.in_bounds(x, y) && world.get(x, y).material == material::EMPTY {
            world.set_owned(x, y, reserved());
        }
    }
}

/// Whether the body's quarter-turned footprint would fit where it stands.
///
/// **Read-only, and that is why it is its own function rather than a call
/// into `try_step`.** Two separate defects made the obvious version wrong,
/// and the first hid the second for the life of the mechanism.
///
/// *It was vacuous.* What stood here was `try_step(world, &probe, probe.x,
/// probe.y, …)` on a rotated clone. `try_step` derives each cell's target
/// from a single integer offset — `(tx, ty) = (ox + cell.dx, oy +
/// cell.dy)` — so probing a rotation *at the body's own position* made
/// every cell's target identical to its own current position, the
/// `(tx, ty) == (cx, cy)` guard skipped the entire body, `horizontal` and
/// `vertical` were never set, and `axis` came back `None` unconditionally.
/// The guard's own comment said it stopped a wedged body turning through
/// the wall beside it. It never once fired, on any body, in any scene.
/// Recorded as `Reports/open-bugs-handoff.md` bug K (and §1i, which is the
/// same defect written up twice); the reproduction is
/// `a_wedged_body_will_not_rotate_through_the_wall`, which was `#[ignore]`d
/// against it and is live again with this.
///
/// *And routing it back through `try_step` with a corrected offset is not
/// the fix either*, for the reason the first defect concealed:
/// `clear_or_displaceable` **mutates** as it answers — `displace` shoves
/// powder and the `Gas` arm calls `world.set(…, Cell::EMPTY)` inline. A
/// probe built on it would rearrange the world to find out whether a turn
/// it may then refuse is legal, which is bug J's speculative-write defect
/// on a path that throws the answer away half the time. So this asks the
/// same *classification* question `clear_or_displaceable` asks, and does
/// nothing with the answer but return it.
///
/// The body's own pre-turn footprint is free by construction, and is
/// checked explicitly rather than relied upon: `promote` clears those cells
/// to `Cell::EMPTY` and `claim_footprint` re-stamps them managed-but-
/// materially-empty, so today they would pass the raw-`EMPTY` test anyway —
/// but a rotation that a body could not perform *into its own cells* would
/// be absurd, and saying so here keeps it true if that ever changes.
fn rotation_fits(world: &World, body: &ChunkBody, turn: Turn, shape: BodyShape) -> bool {
    let before: HashSet<(i32, i32)> = body.cells.iter().map(|c| body.cell_position(c)).collect();
    body.cells.iter().all(|cell| {
        let (x, y) = body.turned_cell_position(cell, turn);
        if before.contains(&(x, y)) {
            return true; // the body is already standing here
        }
        if !world.in_bounds(x, y) {
            return false;
        }
        let there = world.get(x, y);
        // The raw material test, not `is_empty` — same reason
        // `clear_or_displaceable` gives: a managed cell is materially empty
        // and is somewhere a body may be.
        if there.material == material::EMPTY {
            return true;
        }
        match world.materials.kind(there.material) {
            // Liquid yields to something that would sink through it and
            // holds up something that floats, matching
            // `clear_or_displaceable`'s own arm. The two have to agree, for
            // the reason `BodyShape::floats` records: a piece promoted for
            // a move the mover then refuses is the 41-repeat-failure state
            // a raft already sat in once.
            MaterialKind::Liquid => !shape.floats && shape.density > world.materials.density(there.material),
            // **Powder yields, without asking whether there is anywhere for
            // it to go**, and that is deliberately more permissive than the
            // real move, which answers that with `displace`'s ring search.
            // A read-only equivalent would have to walk the same rings and
            // discard the result, per cell, per turn — and refusing on a
            // failed search would stop a piece tumbling the moment it
            // touched its own debris, which is the medium a collapse
            // happens *in*. The cheat this guard exists to stop is turning
            // through a **wall**; rubble is not one.
            MaterialKind::Powder | MaterialKind::Gas => true,
            // Solid, plant, creature — a real obstruction, in
            // `clear_or_displaceable`'s own words.
            _ => false,
        }
    })
}

/// The body's density, buoyancy and reach, as one value — computed once per
/// `advance` and lent to everything in it. All three are O(cells) and none
/// can change while the body is in flight.
fn shape_of(world: &World, body: &ChunkBody) -> BodyShape {
    BodyShape { density: mean_density(world, body), floats: body_floats(world, body), reach: body_extent(body) + 1 }
}

/// The liquid a body is *inside*, if it is inside one.
///
/// Sampled one cell above the top of its own bounding box, at the middle
/// column: that is outside the footprint (so it is never the body's own
/// reservation), and it turns to liquid exactly when the body has gone a
/// row under the surface — which is what "submerged" means for this. One
/// `World::get` per body per frame.
fn surrounding_liquid(world: &World, body: &ChunkBody) -> Option<MaterialId> {
    let (mut x0, mut y0, mut x1) = (i32::MAX, i32::MAX, i32::MIN);
    for cell in &body.cells {
        let (x, y) = body.cell_position(cell);
        x0 = x0.min(x);
        x1 = x1.max(x);
        y0 = y0.min(y);
    }
    if x1 < x0 {
        return None;
    }
    let probe = ((x0 + x1) / 2, y0 - 1);
    if !world.in_bounds(probe.0, probe.1) {
        return None;
    }
    let material = world.get(probe.0, probe.1).material;
    (world.materials.kind(material) == MaterialKind::Liquid).then_some(material)
}

/// Buoyancy and a terminal speed, for one frame of sinking.
///
/// Two terms, and they answer different halves of the report:
///
/// - **Buoyancy** takes back the share of `GRAVITY` the displaced water
///   carries, so rock sinks slower than it falls and something barely
///   denser than water barely sinks at all. It is the density ratio and
///   nothing tuned.
/// - **A terminal speed**, because buoyancy alone does not stop a body
///   *accelerating* — it only halves the rate. `scene=rockdrop`'s pieces
///   reached `MAX_SPEED_PER_AXIS` **inside the pond**, not in the air above
///   it: 6.00 cells/frame at the bottom of a thirty-row descent.
///
/// **A clamp rather than a drag term, and that was measured before it was
/// chosen.** Quadratic drag (`v * |v|`) is the physically nicer model and
/// produces the same uniform descent — and it costs rock, because it slows
/// the *whole* descent rather than only its fast end, so pieces arrive
/// later, one at a time, onto a pile that is already there, and get
/// re-judged and re-broken. Paired on `scene=rockdrop`: quadratic drag at
/// the coefficient that gives the tightest descent leaves **341 of 600
/// cells as stone**, the clamp leaves **420**, and the do-nothing control
/// leaves 450 while showing the artifact. The clamp buys the look for
/// almost nothing.
fn drag_through_liquid(body: &mut ChunkBody, shape: BodyShape, fluid_density: f32) {
    let ratio = shape.density / fluid_density.max(f32::EPSILON);
    let carried = (1.0 / ratio.max(f32::EPSILON)).min(1.0);
    body.vy -= GRAVITY * carried;
    // `reach` is the extent plus one, because the ring search that shoves
    // fluid out of the way wants the plus one. A terminal velocity wants the
    // body itself.
    let size = (shape.reach - 1).max(1) as f32;
    let excess = (ratio - 1.0).max(0.0);
    let terminal =
        (2.0 * GRAVITY * size * excess / SINK_DRAG_COEFFICIENT).sqrt().max(NEUTRAL_SINK_SPEED);
    body.vy = body.vy.min(terminal);
    body.vx = body.vx.clamp(-terminal, terminal);
    // **And water takes the same share of the turn that it takes of the
    // weight**, which is `carried` above and so costs no new constant: a
    // piece barely denser than water barely sinks and barely turns, and a
    // stone one keeps about three-fifths of its rate per frame and has
    // stopped turning inside ten.
    //
    // This is not tidiness. `spin_accel` is held for the whole flight (see
    // `angular_acceleration`), which is right for a piece falling through
    // air for a few dozen frames and wrong for one that never lands: a
    // raft breaking up mid-pond wound itself to `MAX_SPIN_RATE` and
    // **stayed on the surface**, 34 of 80 cells still standing at rows
    // 84..100 after 600 frames and identically after 3,000 -- wedged, not
    // slow. That is `an_unsupported_raft_sinks_through_a_deep_pond`'s own
    // bug coming back by a new route, and the test caught it.
    body.spin_accel *= 1.0 - carried;
    body.spin_rate *= 1.0 - carried;
}

/// The drag coefficient a sinking body is given, which with `GRAVITY` and
/// the body's own size and density is its whole terminal velocity.
///
/// # A real terminal velocity, because a flat one was the remaining complaint
///
/// This replaced a flat `SINK_SPEED` of 1.6, normalised on density alone, and
/// the readout that condemned it is the paired extreme `examples/filmstrip.rs`
/// now prints: on `scene=rockdrop`, *"smallest piece 4 cells across at 1.76,
/// largest 17 across at 1.75"*. Grit and a block, same speed to two decimals.
/// Play said *"still reads as too fast"*, and a size-blind clamp is why --
/// the only speed it can pick is one that is wrong for most of the pieces.
///
/// Terminal velocity balances weight-minus-buoyancy against drag. In two
/// dimensions mass goes as `d²` and frontal area as `d`, so
///
/// ```text
///     v = sqrt( 2 g d (rho_body/rho_fluid - 1) / Cd )
/// ```
///
/// -- the square root of the body's *size*, which is the term that was
/// missing. Stone in water at this coefficient: a 3-cell chunk 0.71, an
/// 8-cell 1.16, a 20-cell slab 1.84. A boulder plummets and grit drifts,
/// which is the thing a single number cannot express.
///
/// **Cd = 2.0 is the blunt-body figure, and the regime was checked rather
/// than assumed.** At roughly 1.8 cm to the cell a 4-cell fragment is ~7 cm
/// across moving ~1 m/s, so Re is about 8 x 10^4: fully inertial, where
/// Newton drag holds and an irregular tumbling solid sits at Cd 1-2. Flat
/// plates are nearer 2 and spheres nearer 0.5; pixel rubble is neither, and
/// the upper end of the blunt range is also the end that reads right.
///
/// # This deliberately re-opens a spread that an earlier complaint closed
///
/// The flat clamp was set to make everything from one break arrive together,
/// against *"a first group of chunks that drop too fast and then the rest
/// that come together with the grit later"*. Size dependence spreads arrival
/// times again -- on purpose, and it is not a regression to that report. What
/// that report was about was a **43x** spread (0.14 to 6.00) produced by
/// nothing braking a body at all, so the ordering was by how long a piece had
/// been falling and meant nothing. This is a **2.6x** spread ordered by size:
/// the big pieces lead, the grit follows, and that is what a break in water
/// looks like.
///
/// The measured cost of slowing the descent is recorded on
/// `drag_through_liquid`: pieces that arrive later arrive one at a time onto
/// a pile that is already there, and get re-judged and re-broken. Watch
/// surviving stone on `rockdrop` and `lavadrop` when tuning this.
const SINK_DRAG_COEFFICIENT: f32 = 2.0;

/// The floor under `SINK_DRAG_COEFFICIENT`'s size-and-density scaling: a body
/// only just denser than the liquid still goes down, slowly, rather than
/// hanging in it.
const NEUTRAL_SINK_SPEED: f32 = 0.1;

/// How fast a body has to be falling before it throws a crown, in cells per
/// frame.
///
/// A body settling the last cell into a pool should slide in; one that has
/// fallen any distance should not. That sentence is the specification; the
/// number below is set from it rather than from a scene, because the
/// quantity it gates is the *feel* of an impact and a bar tuned to one pond
/// depth would be a bar about that pond.
///
/// **The arithmetic that used to be written here was wrong, and the value it
/// justified is nonetheless right.** It read *"`GRAVITY` is 0.05/frame, so
/// this is roughly the speed reached after a ten-cell drop"*. `GRAVITY` is
/// **0.15** (see the constant above), so `v = sqrt(2 g d)` puts 0.7 at a
/// **1.6-cell** drop; a real ten-cell drop reaches 1.73.
///
/// Which of the two the constant should be is settled by the specification
/// and not by the arithmetic: one cell of fall reaches 0.55, so 0.7 is the
/// bar that separates a settle from a fall, and 1.73 would be a bar that
/// ignores everything short of a storey. Left where it is, with the
/// derivation corrected -- but noting it was never *measured* to be 0.7,
/// only argued to be, and the argument had the wrong number in it.
const SPLASH_MIN_ENTRY_SPEED: f32 = 0.7;

/// Report splash sites beside a body that has just broken a free surface.
///
/// # A solid slab cannot splash where it lands, and that is not a bug in the splash
///
/// Reported from play: *"I don't see any splash or difference in any of
/// these"*, with the suggestion that the test should use clumps rather than
/// scattered grains. The premise is backwards and the instinct was right
/// anyway, which is worth writing down.
///
/// Backwards, because `particle::throw_splashes` refuses any site whose
/// cell above is not empty, and a falling clump *is* the thing standing in
/// that space -- `scene=splash`'s own fixture says so: "loose grains rather
/// than a block: a splash site needs air above the displaced water, which a
/// solid slab never leaves". A denser clump would make the effect
/// disappear, not appear.
///
/// Right anyway, because a falling **rigid body** reported no splash site
/// at all, from anywhere: `clear_or_displaceable` shoves whole columns of
/// water out of a body's way and never went near `report_splash`. Rock into
/// water was silent, and no amount of tuning the CA rule could have found
/// it.
///
/// The crown goes at the **rim**, one column outside the footprint on each
/// side, which is both where the model can put it (air above) and where one
/// physically forms -- under the body there is nowhere for the water to go
/// but sideways, and sideways is here. `throw_splashes` fans out from each
/// site on its own, so two reports are a crown and not two droplets.
///
/// Straddling the surface is the trigger, not "is wet": the scan starts one
/// row above the body's own top and stops one below its leading edge, so a
/// body already under water finds no free surface in range and reports
/// nothing. That is what keeps this from firing every frame of a long sink.
fn report_entry_splash(world: &mut World, body: &ChunkBody, entry_speed: f32) {
    if entry_speed < SPLASH_MIN_ENTRY_SPEED {
        return;
    }
    let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for c in &body.cells {
        let (px, py) = body.cell_position(c);
        x0 = x0.min(px);
        x1 = x1.max(px);
        y0 = y0.min(py);
        y1 = y1.max(py);
    }
    if x1 < x0 {
        return;
    }
    for x in [x0 - 1, x1 + 1] {
        for y in (y0 - 1)..=(y1 + 1) {
            if !world.in_bounds(x, y) {
                continue;
            }
            if world.materials.kind(world.get(x, y).material) != MaterialKind::Liquid {
                continue;
            }
            // The free surface, and only it: `throw_splashes` checks the
            // same thing a frame later and the two have to agree.
            if world.in_bounds(x, y - 1) && world.get(x, y - 1).material == crate::sim::material::EMPTY {
                crate::sim::surface::CellSurface::report_splash(world, x, y, 1.0);
            }
            break; // the topmost liquid in this column is the only candidate
        }
    }
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
/// The two whole-body quantities the collision test needs: how heavy it is
/// on average, and how far the fluid in front of it may have to travel to
/// get behind it. Both are O(cells) and constant for a body in flight, so
/// they are computed once in `advance` rather than per substep.
#[derive(Clone, Copy)]
struct BodyShape {
    density: f32,
    /// Whether any cell of the body claims buoyancy
    /// (`MaterialDef::floats`). Asked **before** density, matching
    /// `structural::region_has_free_face` -- the two have to agree or a
    /// piece gets promoted for a move the mover then refuses, which is
    /// exactly the 41-repeat-failure state a raft sat in before this. Any
    /// floating cell floats the whole piece: the conservative direction.
    floats: bool,
    reach: i32,
}

fn try_step(world: &mut World, body: &ChunkBody, next_x: f32, next_y: f32, shape: BodyShape) -> Step {
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
    // The body's **current** footprint, unchanged: this set stops the ring
    // shove below putting material into the body's own path, and every
    // powder case in the engine was tuned against exactly this behaviour.
    //
    // `exchange_with_fluid` and `restamp_footprint` need the **next**
    // footprint as well, and get it from this same set offset by the
    // body's whole-cell motion. One set, two queries: every cell moves by
    // the same integer offset this substep, so "will the body be here
    // after the move" is "was the body there before it", shifted. A second
    // `HashSet` per substep is the obvious alternative and is not worth it
    // -- `blocked_axis` runs per substep per body and the hashing above is
    // already its largest cost.
    let occupied: HashSet<(i32, i32)> = body.cells.iter().map(|c| body.cell_position(c)).collect();
    let motion = (ox - body.x.round() as i32, oy - body.y.round() as i32);
    // Fluid the body is about to move into, gathered rather than shoved.
    // **Two passes, and that is the point**: where the fluid goes is a
    // property of the whole move (see `exchange_with_fluid`), so it cannot
    // be decided one cell at a time while the move is still in question.
    // A body that turns out to be blocked now leaves the water where it
    // was, instead of having shoved half of it on the way to finding out.
    let mut swaps: Vec<(i32, i32)> = Vec::new();

    for cell in &body.cells {
        let (tx, ty) = (ox + cell.dx, oy + cell.dy);
        let (cx, cy) = body.cell_position(cell);
        if (tx, ty) == (cx, cy) {
            continue; // this cell is not actually changing position this substep
        }
        if !world.in_bounds(tx, ty) || !clear_or_displaceable(world, &occupied, tx, ty, shape, &mut swaps) {
            // Attribute the block to the axis this cell was moving along.
            if tx != cx {
                horizontal = true;
            }
            if ty != cy {
                vertical = true;
            }
        }
    }

    let axis = match (horizontal, vertical) {
        (false, false) => None,
        (true, false) => Some(Axis::Horizontal),
        (false, true) => Some(Axis::Vertical),
        (true, true) => Some(Axis::Both),
    };
    Step { axis, occupied, motion, swaps }
}

/// What one substep's test found: whether it is blocked, and everything the
/// commit needs if it is not.
///
/// Returned rather than acted on, because **where the fluid goes is a
/// property of the whole move** and cannot be settled cell by cell while
/// the move is still in question — see `exchange_with_fluid`. A body that
/// turns out to be blocked now leaves the water where it was instead of
/// having shoved half of it on the way to finding out.
struct Step {
    axis: Option<Axis>,
    /// The footprint before the move. `occupied` shifted by `motion` is the
    /// footprint after it: every cell moves by the same integer offset, so
    /// one set answers both questions.
    occupied: HashSet<(i32, i32)>,
    motion: (i32, i32),
    /// Liquid cells the body is about to enter, to be exchanged for the
    /// cells it is about to leave.
    swaps: Vec<(i32, i32)>,
}

/// Take the footprint the body is standing in, so nothing else moves into
/// it — see `reserved`. Called once, on a body's first contact with fluid.
fn claim_footprint(world: &mut World, occupied: &HashSet<(i32, i32)>) {
    for &(x, y) in occupied {
        if world.in_bounds(x, y) && world.get(x, y).material == material::EMPTY {
            world.set_owned(x, y, reserved());
        }
    }
}

/// Hand the fluid the body is about to displace the cells the body is
/// about to leave, one for one.
///
/// # A rigid translation is an exchange, and searching for somewhere to
/// put the water is what could not see that
///
/// A body moving by an integer offset vacates exactly as many cells as it
/// enters, so the fluid in front can be **paired** with the space behind by
/// construction rather than searched for. That is the whole fix: there is
/// no failure mode left in which a submerged body has nowhere to put the
/// water and therefore stops.
///
/// `make_way_behind` was the per-cell approximation of this and could not
/// see the pairing -- it walked one cell straight back and gave up if that
/// walk found nothing, which inside a pond it usually did, and it read
/// `is_empty`, so reserving the footprint (`reserved`) blinded it
/// completely. It is gone; this is what replaced it. Do not reintroduce a
/// search here: the count matches exactly, and a search is how the water
/// ends up somewhere the motion cannot account for.
///
/// The walk back along `-motion` is not just a way to find *a* free cell —
/// it is what keeps the look `make_way_behind` was reaching for, *what is
/// in front ends up behind*. A body sinking straight down lifts the water
/// over its own top rather than teleporting it to the nearest gap.
fn exchange_with_fluid(world: &mut World, occupied: &HashSet<(i32, i32)>, motion: (i32, i32), swaps: &[(i32, i32)], reach: i32) {
    if swaps.is_empty() || motion == (0, 0) {
        return;
    }
    // The cells the body is leaving: in the footprint now, and not in it
    // after the move. One `HashSet` and no second allocation -- every cell
    // moves by the same offset, so "will the body still be here" is "was
    // the body one step back", which `occupied` already answers.
    //
    // **Materially empty only.** A vacating cell is normally this body's
    // own reservation, but not always: `restamp_footprint` declines to
    // stamp a cell that already holds something, so a cell the body walked
    // over without clearing is still in the footprint and still holds
    // water. Writing a swap into it deletes that water -- measured at
    // **920 cell-equivalents** on `scene=rockdrop` before this filter, all
    // of it inside the hundred frames of the plunge.
    let mut vacating: HashSet<(i32, i32)> = occupied
        .iter()
        .copied()
        .filter(|&(x, y)| !occupied.contains(&(x - motion.0, y - motion.1)))
        .filter(|&(x, y)| world.in_bounds(x, y) && world.get(x, y).material == material::EMPTY)
        .collect();
    for &(sx, sy) in swaps {
        let moving = world.get(sx, sy);
        let mut target = None;
        let (mut px, mut py) = (sx - motion.0, sy - motion.1);
        // Straight back through the body, to the cell on its far side that
        // is being given up. Bounded by the body's own extent, never by a
        // constant, for the reason `BodyShape::reach` records.
        for _ in 0..reach {
            if vacating.remove(&(px, py)) {
                target = Some((px, py));
                break;
            }
            if !occupied.contains(&(px, py)) {
                break; // walked out of the body without finding one
            }
            px -= motion.0;
            py -= motion.1;
        }
        // A ragged edge on a diagonal move can leave a cell whose own
        // column gives nothing up. There is still a spare -- the counts
        // match -- so take the nearest of them rather than dropping the
        // water, which is the one outcome this whole path exists to avoid.
        let target = target.or_else(|| {
            let pick = vacating.iter().copied().min_by_key(|&(x, y)| (x - sx).abs() + (y - sy).abs());
            if let Some(p) = pick {
                vacating.remove(&p);
            }
            pick
        });
        let Some((tx, ty)) = target else { continue };
        world.set(tx, ty, moving);
        world.set(sx, sy, reserved());
    }
}

/// Move the footprint reservation with the body: give back what it is
/// leaving, hold what it is entering.
///
/// Only the perimeter changes, so this is O(cells) with two hash lookups
/// each and writes only where the answer actually differs — a body in
/// flight does not re-dirty its whole interior every substep.
///
/// `set_owned` rather than `set`, here and in `rotate_reserved` and
/// `settle`'s release: `World::set` demotes the owner of any `FLAG_MANAGED`
/// cell it overwrites, which is a `body_index` lookup on **every** write
/// this makes, and the cell being overwritten is this body's own
/// reservation. `set_owned` is the sanctioned bypass for exactly that, and
/// it still goes through `write_cell`, so chunks are woken as they must be
/// — a cleared reservation has to be seen by the sweep or no water flows
/// into the space the body just left.
fn restamp_footprint(world: &mut World, occupied: &HashSet<(i32, i32)>, motion: (i32, i32)) {
    if motion == (0, 0) {
        return;
    }
    for &(x, y) in occupied {
        if !occupied.contains(&(x - motion.0, y - motion.1)) {
            // Vacated. Give it back only if it is still *our* reservation:
            // `exchange_with_fluid` may already have put water here, and
            // that water is the point.
            let cell = world.get(x, y);
            if world.in_bounds(x, y) && cell.material == material::EMPTY && cell.managed() {
                world.set_owned(x, y, Cell::EMPTY);
            }
        }
        let (nx, ny) = (x + motion.0, y + motion.1);
        if !occupied.contains(&(nx, ny)) && world.in_bounds(nx, ny) && world.get(nx, ny).material == material::EMPTY {
            world.set_owned(nx, ny, reserved());
        }
    }
}

/// The body's largest extent in cells, either axis — how far the fluid in
/// front of it may have to travel to get behind it.
fn body_extent(body: &ChunkBody) -> i32 {
    let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for c in &body.cells {
        x0 = x0.min(c.dx);
        x1 = x1.max(c.dx);
        y0 = y0.min(c.dy);
        y1 = y1.max(c.dy);
    }
    if x1 < x0 {
        return 1;
    }
    (x1 - x0).max(y1 - y0) + 1
}

/// The body's mean material density — what decides whether it sinks in a
/// liquid or rides on it.
///
/// A mean rather than a per-cell test, because buoyancy is a statement
/// about the whole piece: a stone slab with one cell of ice in it still
/// sinks, and asking the question per cell would let the same body sink at
/// one end and float at the other.
fn mean_density(world: &World, body: &ChunkBody) -> f32 {
    if body.cells.is_empty() {
        return 0.0;
    }
    let total: f32 = body.cells.iter().map(|c| world.materials.density(c.material)).sum();
    total / body.cells.len() as f32
}

/// Whether any cell of the body claims buoyancy. See `BodyShape::floats`.
fn body_floats(world: &World, body: &ChunkBody) -> bool {
    body.cells.iter().any(|c| world.materials.get(c.material).floats)
}
/// Whether the body may move into `(x, y)` — clearing whatever is there if
/// it is loose enough to shove, or recording it for the exchange if it is
/// fluid.
///
/// Powder is shoved here and now, by `displace`'s ring search, exactly as
/// it always was: every dry scene in the engine is tuned against that
/// behaviour, and the reported bug is water. Liquid is only *recorded* —
/// see `exchange_with_fluid` for why it cannot be decided one cell at a
/// time.
fn clear_or_displaceable(
    world: &mut World,
    occupied: &HashSet<(i32, i32)>,
    x: i32,
    y: i32,
    shape: BodyShape,
    swaps: &mut Vec<(i32, i32)>,
) -> bool {
    // **The raw material test, not `is_empty`.** Anything materially empty
    // is somewhere this body may be: air, its own reserved footprint (a
    // different cell of this same body is standing there right now and is
    // about to move too), and another body's reserved footprint. The last
    // of those is deliberate rather than overlooked -- bodies have always
    // passed through each other, and making them collide is a separate
    // change with its own consequences for a collapse with two dozen
    // pieces in flight. `is_empty` here would report the body's own
    // footprint as an obstruction and stop every move it makes.
    if world.get(x, y).material == material::EMPTY {
        return true;
    }
    let kind = world.materials.kind(world.get(x, y).material);
    if !matches!(kind, MaterialKind::Powder | MaterialKind::Liquid | MaterialKind::Gas) {
        return false; // solid, plant, creature -- a real obstruction
    }
    // **Measured at the scene, not by a unit guard, and that is recorded
    // rather than papered over.** Ablating this whole arm leaves
    // `rigid.rs`'s own sinking guards green -- the raft there fails, breaks
    // into rubble, and rubble is a `Powder` that sinks by the ordinary CA
    // rule, so the body path is never the thing under test. What the arm
    // does buy shows on a real scene: `filmstrip scene=lavapour` ends with
    // **9 unattached solids reaching no anchor with it and 19 without**
    // (against 154 before any of this work). Anyone tempted to delete it
    // should re-run that, not the unit tests.
    if kind == MaterialKind::Liquid {
        if shape.floats || shape.density <= world.materials.density(world.get(x, y).material) {
            return false; // it floats on this: the liquid holds it up
        }
        // **Never a failure.** The body vacates exactly as many cells as it
        // enters, so there is somewhere for this by construction. The old
        // per-cell walk gave up when it could not find a hole nearby, and
        // inside a pond it could not: 2,834 of 10,849 attempts succeeded,
        // and each failure stalled the body into being broken up again.
        swaps.push((x, y));
        return true;
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

/// What a landing writes into a body cell's `aux` — shared by the two
/// seams, `rigid::settle` and `particle::place_landed`.
///
/// Three-valued rather than a pair of booleans because the three are
/// mutually exclusive answers to one question, and `CLAUDE.md`'s harness
/// rule wants a knob whose value can be read back rather than inferred
/// from which of two flags happened to be set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LandingAux {
    /// `Cell::new`'s 0 — **the bug**, kept only as the measurement baseline.
    /// On an inert `Solid` that reads as *bedrock-adjacent*.
    Zero,
    /// "No known path, earn one." Honest, and corrected by `tick` in a
    /// single improvement round — but not until the next structural check.
    Max,
    /// The value `tick` would compute, computed now. The shipping default.
    Seed,
}

/// `SETTLE_AUX=zero|max|seed`, defaulting to `seed`.
///
/// `SETTLE_AUX_MAX=1` is still honoured as an alias for `max`: the four-arm
/// ablation in `Reports/structural-support-model.md` §6.4 and the two
/// entries in `Reports/dead-ends.md` name it, and those runs have to stay
/// reproducible.
fn settle_aux_mode() -> LandingAux {
    use std::sync::OnceLock;
    static MODE: OnceLock<LandingAux> = OnceLock::new();
    *MODE.get_or_init(|| match std::env::var("SETTLE_AUX").as_deref() {
        Ok("zero") => LandingAux::Zero,
        Ok("max") => LandingAux::Max,
        Ok("seed") => LandingAux::Seed,
        // The legacy alias. `SETTLE_AUX_MAX=1` is what §6.4's ablation and
        // two `dead-ends.md` entries name, so it has to keep reproducing
        // those runs; `=0` is the `Cell::new` baseline they were measured
        // against. Absent, the shipping default applies.
        _ => match std::env::var("SETTLE_AUX_MAX").as_deref() {
            Ok("0") => LandingAux::Zero,
            Ok(_) => LandingAux::Max,
            Err(_) => LandingAux::Seed,
        },
    })
}

/// Which `LandingAux` arm is live, for harnesses and diagnostics.
/// `CLAUDE.md`: a knob nobody can see the value of is a knob nobody can
/// tell is disconnected.
pub fn settle_aux_mode_name() -> &'static str {
    match settle_aux_mode() {
        LandingAux::Zero => "zero",
        LandingAux::Max => "max",
        LandingAux::Seed => "seed",
    }
}

/// Write one landed body cell, giving it an anchor distance rather than
/// `Cell::new`'s 0.
///
/// **`Cell::new` starts a landed cell unattached and at `aux` 0, and only
/// the first of those is right.** Unattached is deliberate and stays: a
/// body exists because it broke out of something, so landing must not
/// silently re-attach it. The 0 was never a decision — it is the struct's
/// default, and on an inert `Solid` the slot is a distance to bedrock, so
/// it reads as *at an anchor*. `settle` and `particle::land` are the two
/// seams through which the world writes body material at rest, and both
/// were writing it; `Reports/structural-support-model.md` §6 has the
/// write-seam trap, and §6.4 the four-arm ablation that showed neither seam
/// alone accounts for it (23% and 1% apart, 99.5% together).
///
/// **Called at the landing position, not the aimed-at one.** `settle`'s
/// four arms can relocate a cell several rows from `body.cell_position`,
/// and the seeded value is a function of where it ended up. Getting this
/// wrong would seed from the neighbours of a hole the cell is not in.
///
/// This does **not** schedule the structural check — `settle` schedules
/// around both the aimed-at and the landed position afterwards, and the
/// union of the two is load-bearing for a reason its own comment gives.
fn place_settled(world: &mut World, x: i32, y: i32, fresh: Cell) {
    let placed = if !super::structural::is_body_material(world, fresh.material) {
        fresh
    } else {
        match settle_aux_mode() {
            LandingAux::Zero => fresh,
            LandingAux::Max => fresh.with_aux(u16::MAX),
            LandingAux::Seed => fresh.with_aux(super::structural::seed_landing_aux(world, x, y, fresh.material)),
        }
    };
    world.set(x, y, placed);
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
    // **How it came to rest**, taken here because this is the only moment a
    // piece's own extent is unambiguous: a frame later it is grid cells,
    // touching whatever else landed beside it, and no census downstream can
    // tell one log from two. See `FailureCounts::settled_lying` for the
    // measurement that made this necessary. Gated at `MIN_BODY_CELLS` to
    // match what "a piece" means everywhere else in this pipeline.
    if body.cells.len() >= MIN_BODY_CELLS {
        let (x0, y0, x1, y1) = body.bounds();
        world.structural_failures.record_settled_pose(x1 - x0, y1 - y0);
    }
    // **Where each cell actually landed, not where it was aimed.**
    // The checks below used to be scheduled around `cell_position`, which
    // is only the same place when the cell went in unmoved. A cell that had
    // to be relocated was then never checked at wherever it really ended
    // up -- so a relocated cell could sit unsupported forever, which is the
    // hanging-stone report in miniature and reached the screen as a scatter
    // of stone standing in open sky over a pond.
    // **Give the footprint back before writing into it.**
    //
    // Every cell of a promoted body is held as a managed-empty
    // reservation (`reserved`), and `is_empty` is managed-aware -- so
    // without this, every cell of a settling body fails the fast path
    // below and goes down the displacement arms instead, hunting for
    // somewhere to put an "occupant" that is nothing but the body's own
    // reservation. That is not a hypothetical: it is precisely how the
    // first attempt at reserving footprints lost **1,821 cell-equivalents
    // of water** on `scene=rockdrop` (`Reports/open-bugs-handoff.md` §1h)
    // -- `nearest_free` and `surface_above` handed the displaced water a
    // footprint cell that this same loop then overwrote with stone.
    //
    // Only cells that are still *ours*: a body may be settling because
    // something arrived in its way. And only when the body took a footprint
    // at all -- one that never met fluid never held one.
    for cell in body.cells.iter().filter(|_| body.reserved) {
        let (x, y) = body.cell_position(cell);
        if world.in_bounds(x, y) {
            let held = world.get(x, y);
            if held.material == material::EMPTY && held.managed() {
                world.set_owned(x, y, Cell::EMPTY);
            }
        }
    }
    let mut landed: Vec<(i32, i32)> = Vec::with_capacity(body.cells.len());
    for cell in &body.cells {
        let (x, y) = body.cell_position(cell);
        // `Cell::new` starts unattached and with `aux` at 0, and both are
        // deliberate. A body only exists because it broke out of something,
        // so it is no longer backed by the mass it came from -- landing must
        // not silently re-attach it, or a chunk that fell would become
        // immovable terrain wherever it happened to stop.
        // **A landed piece of a tree is not a tree.** `Cell::new` writes
        // the material the body took off with, which for a promoted limb is
        // live `wood` -- inert, unattached to any organism, invisible to
        // `decay.rs`, and drawn as bark on something that is finished
        // growing. `MaterialDef::severs_into` is the piece tier
        // (`wood` -> `log`), and it is consulted only for a cell that
        // *left the grid as tissue*, so a `wood` wall someone painted and
        // then knocked down still lands as the wall it was.
        //
        // The shade is re-rolled rather than carried, because the palettes
        // are different lengths and a bark index into a log's four colours
        // is a different colour, not the same grain -- the one case where
        // `BodyCell::shade`'s "never re-roll" rule does not apply, since
        // the material changed underneath it.
        // **The `aux` a landed cell gets is decided at the write, by
        // `place_settled` below** -- not here, because it depends on where
        // the cell actually ends up and the four arms below can relocate it
        // several rows from where it was aimed. `Cell::new`'s 0 is the
        // false anchor §S is about; see `place_settled` for what replaces
        // it and why.
        let fresh = match (cell.organism_id != 0).then(|| world.materials.get(cell.material).severs_into).flatten() {
            Some(into) => {
                // **Shade by where the cell sits in the piece, not at
                // random -- a body needs a surface or it reads as gravel.**
                //
                // This path re-rolls because the material changes under the
                // cell (`wood` -> `log`) and a bark index into a log's four
                // colours is a different colour, not the same grain; that
                // part is right and stands. What was wrong is that it rolled
                // *per cell*, so a 311-cell log was 311 independent draws
                // over four tones -- salt-and-pepper, which is exactly what
                // grit looks like. Measured on the settled `scene=fell`
                // pile at 8x: a 33x52 coherent body, a third of the crop
                // wide, indistinguishable from the powder around it.
                //
                // `dead-ends.md` already carries three attempts to fix this
                // on `log`'s *palette* -- a wide spread (speckle), a grey
                // (read as tissue dying), a warm mid-brown (vanished into
                // litter). Three fixes failing the same way is `CLAUDE.md`'s
                // own signal that the approach is wrong rather than the
                // tuning: the missing thing is not a colour, it is that a
                // fragment has no *shape* unless its tone tells you where
                // its top is. So the tone comes from depth below the
                // piece's own upper surface, lightest at the top and
                // darkest inside, which is how any lit solid reads.
                //
                // Rock never had this defect because its arm below *carries*
                // the flight shade, and the flight shade came from the rock
                // it broke out of. Only the tier-changing path re-rolled.
                //
                // **The shade is carried, never re-rolled, and that is now
                // unconditional.** Owner, twice over: *"The color should not
                // change at all (not sure how many times I have to say
                // this)"*, then *"and the trunks and branches shouldn't
                // change colour either."*
                //
                // This path used to re-roll, on the reasoning that a piece
                // lands in a tier whose palette is a different length, so the
                // old index would mean a different colour. That was true and
                // it was the wrong thing to fix. The tiers now carry their
                // source's palette entry for entry -- `log` <- `wood`,
                // `deadleaf` <- `leaf` -- so index `k` *is* the same colour
                // and copying it makes the change invisible, which is what
                // was asked for.
                //
                // A modulo rather than a plain copy because a shorter source
                // palette must still land somewhere valid: `rootwood` has
                // four entries against `log`'s thirty-two.
                //
                // **This restores `BodyCell::shade`'s standing rule** -- never
                // re-roll, because a landing that changes colour reads as a
                // pop -- to the one path that had an exception to it. Two
                // things go with it: the depth-and-surface shading this
                // replaced (which invented a lit crust for a piece that
                // should simply look like what it broke off), and the rng
                // draw that fed it. Dropping the draw shifts every later
                // random event in the world, so a run across this change is
                // not cell-identical with one before it; that is stated
                // rather than worked around, because keeping a draw alive
                // purely to preserve a stream is the kind of thing a later
                // reader deletes as dead and silently changes the world.
                let shades = world.materials.get(into).palette.len().max(1);
                let shade = (cell.shade as usize % shades) as u8;
                // Counted here rather than by censusing the material,
                // because the two answer different questions and only this
                // one is about the *pipeline*: a world census of `log`
                // measures what is still standing, which decay and fire eat
                // into, while this measures what the fall actually
                // delivered. Read against
                // `FailureCounts::severed_organism_pieces` -- the gap
                // between them is `settle_lost_cells` and nothing else, and
                // when it is not, something between promotion and landing
                // is eating pieces.
                world.structural_failures.record_settled_tissue(1);
                Cell::new(into, shade)
            }
            None => Cell::new(cell.material, cell.shade),
        };
        if world.in_bounds(x, y) && world.is_empty(x, y) {
            place_settled(world, x, y, fresh);
            landed.push((x, y));
            continue;
        }
        // **A body that comes to rest underwater takes the water's place.**
        //
        // Without this the arm below is reached with every cell of the body
        // sitting in water and no empty cell within four rings of any of
        // them, so `nearest_free` fails for all of them and the whole body
        // is dropped. Measured on a 40x2 stone raft settling mid-pond:
        // **80 cells in, 9 cells out** -- 71 destroyed, silently, at the
        // one moment a body stops being tracked. It was reachable before
        // and rare, because a body that could not sink never got deep
        // enough for the ring search to fail; making pieces sink is exactly
        // what walks into it.
        //
        // The water goes **up**, not to the nearest hole: a submerged
        // volume displaces its own volume to the surface, and the column
        // above is where the surface is. Unbounded by design, for the
        // reason the ring search is the wrong shape here at all -- the
        // distance is set by how deep the body sank, which is a property
        // of the pond and not a constant anyone can pick.
        if world.in_bounds(x, y) {
            let occupant = world.get(x, y);
            let kind = world.materials.kind(occupant.material);
            // **Liquid only, deliberately.** A powder arm was tried and
            // withdrawn: it changes how a body settles into its own rubble
            // on every dry scene in the engine, which is a separate
            // question with its own tuned acceptance cases, and nothing has
            // reported it. The reported bug is water.
            if kind == MaterialKind::Liquid
                && !world.materials.get(cell.material).floats
                && world.materials.density(cell.material) > world.materials.density(occupant.material)
            {
                // Somewhere for what was here to go: the nearest opening
                // first, because near a free surface that is where a shoved
                // cell belongs, and only then straight up. Powder as well as
                // liquid, because a piece settling into its own rubble hits
                // the identical arm -- and without it the fallback below
                // throws the *body's* cell to the surface instead, which put
                // stone in the sky over the pond in exactly this test.
                let home = nearest_free(world, x, y).or_else(|| surface_above(world, x, y));
                if let Some((nx, ny)) = home {
                    world.set(nx, ny, occupant);
                    place_settled(world, x, y, fresh);
                    landed.push((x, y));
                    continue;
                }
            }
        }
        if let Some((nx, ny)) = nearest_free(world, x, y) {
            place_settled(world, nx, ny, fresh);
            landed.push((nx, ny));
            continue;
        }
        // Nothing empty within reach, but a **directly adjacent** liquid
        // this cell outweighs is still somewhere to go.
        //
        // Four neighbours and not a ring search, and that is a frame-cost
        // decision made from measurement rather than taste. A ring search
        // here, plus a straight-up walk for the body's own cell as a final
        // fallback, took `scene=ligament` from **18.1 ms to 86.6 ms** --
        // against a 60 ms bar -- with byte-identical failure counts either
        // side, because the ligament's one failure settles ~4,400 cells in
        // a single frame and every one of them paid an 81-cell scan and a
        // walk up the whole world. The scene has no liquid in it at all, so
        // it was pure toll. Anything reached from here has to be O(1) in
        // the common case and cost nothing on a dry scene.
        let swapped = [(0, 1), (-1, 0), (1, 0), (0, -1)].iter().find_map(|&(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            if !world.in_bounds(nx, ny) {
                return None;
            }
            let occupant = world.get(nx, ny);
            if world.materials.kind(occupant.material) != MaterialKind::Liquid
                || world.materials.get(cell.material).floats
                || world.materials.density(cell.material) <= world.materials.density(occupant.material)
            {
                return None;
            }
            surface_above(world, nx, ny).map(|home| (nx, ny, occupant, home))
        });
        if let Some((nx, ny, occupant, (sx, sy))) = swapped {
            world.set(sx, sy, occupant);
            place_settled(world, nx, ny, fresh);
            landed.push((nx, ny));
            continue;
        }
        // A column full to the top of the world -- genuinely nowhere,
        // matching `particle::land`'s own last resort for a grain with no
        // legal rest position.
        //
        // **Counted, because it is a real and un-fixed loss and it is about
        // to become much more visible.** A body that lands in a pile of its
        // own debris hits this arm for every cell of itself that has no
        // opening within `nearest_free`'s rings, and a felled tree lands in
        // a large pile of its own debris. `Reports/open-bugs-handoff.md`
        // §1c has the standing figure (~10% of a body's cells); this is
        // that number made readable per run rather than fixed here, which
        // is deliberately out of T1's scope -- fixing it means deciding
        // where a cell with nowhere to go *should* end up, and that is a
        // settling question rather than a fragmentation one.
        world.structural_failures.record_settle_loss(1);
    }
    // **Both where a cell was aimed and where it actually went**, and the
    // union rather than either alone.
    //
    // Aimed-at only was the original, and it misses a relocated cell
    // entirely -- so a cell shoved several rows away was never checked at
    // wherever it ended up and could stand unsupported forever, which
    // reached the screen as stone hanging in open sky over a pond.
    //
    // Landed-at only is the obvious correction and it is *also* wrong, in
    // the opposite direction: a cell with nowhere to go is dropped and has
    // no landing position, and its neighbours have still just lost a
    // possible support. Dropping those checks cut the cascade measurably --
    // `scene=roomcut` fell from 17 overload failures to 8 (and 638
    // unsupported cells to 344) against an acceptance bar that exists
    // precisely to show a cut wall coming apart.
    for cell in &body.cells {
        let (x, y) = body.cell_position(cell);
        world.schedule_structural_check_around(x, y);
    }
    for &(x, y) in &landed {
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

/// The first materially-empty cell straight up from `(x, y)`, or `None` if
/// the column is full to the top of the world.
///
/// Where a displaced liquid goes when a body settles into it — see
/// `settle`.
///
/// `is_empty()` rather than the raw material test, and the raw test was a
/// real bug rather than a style choice: `is_empty()` is managed-aware, and
/// a reserved cell — a promoted body's footprint (`reserved`), or a liquid
/// heightfield body's container — is *not* somewhere a loose water cell may
/// be written. Writing there puts the water inside a body that is about to
/// overwrite it, which is one of the two halves of the 1,821-cell loss
/// §1h records. (`structural::region_has_free_face` keeps the raw test on
/// purpose and is not the same question: a reserved cell genuinely is air
/// for the purpose of "is this region open to the sky".)
fn surface_above(world: &World, x: i32, y: i32) -> Option<(i32, i32)> {
    let min_y = world.bounds()?.min_y;
    let mut ny = y - 1;
    while ny >= min_y {
        if world.is_empty(x, ny) {
            return Some((x, ny));
        }
        ny -= 1;
    }
    None
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

    /// The load model with no `chain_reach` leash -- see
    /// `World::without_chain_limit` for why the model's own tests take it
    /// off and the game does not.
    fn test_world() -> World {
        World::new(Rect::new(0, 0, 63, 63)).without_chain_limit()
    }

    #[test]
    fn a_blow_bursts_a_chunk_that_is_still_in_the_air() {
        // **The owner's third ask, and the positive control it needed.**
        // 2026-08-29: *"have the pieces fall off in chunks. if the chunks
        // are hit, they break into stone dust."* A *landed* chunk is grid
        // stone again and a blow already turned it to rubble; a chunk in
        // flight is a `ChunkBody`, and `promote` writes `Cell::EMPTY` into
        // its footprint — so `is_tool_target` said no and a swing passed
        // straight through it.
        //
        // **This test exists because the sheet could not see the fix.**
        // `filmstrip scene=smash` came back byte-identical for cells
        // broken, cracked, chunks and dust across the change, which is
        // `CLAUDE.md`'s tell for a mechanism that never ran: the gnome
        // swings every 90 frames and every piece has settled long before
        // the next blow. Identical output is not evidence of a small
        // effect, it is evidence of no execution — so the control is
        // constructed rather than observed.
        let mut w = test_world();
        let cells: Vec<BodyCell> = (0..4)
            .flat_map(|dx| (0..4).map(move |dy| BodyCell { dx, dy, material: material::STONE, shade: 0, organism_id: 0 }))
            .collect();
        w.chunk_bodies.push(ChunkBody::falling(cells, 30.0, 30.0, 0.0));
        assert_eq!(w.chunk_bodies.len(), 1, "test setup: one body in flight");
        let rubble = w.materials.id_of("rubble").expect("stone breaks into rubble");

        let acted = strike(&mut w, 31, 31, 6, 3.0);

        assert!(w.chunk_bodies.is_empty(), "the blow must burst the body it landed on, not pass through it");
        let grit = (28..36).flat_map(|y| (28..36).map(move |x| (x, y))).filter(|&(x, y)| w.get(x, y).material == rubble).count();
        assert!(grit >= 12, "the burst body must be lying there as grit; found {grit} of 16 cells");
        assert!(acted >= 16, "the return value must count the body it dusted, or the counter cannot say this fired: {acted}");
    }

    #[test]
    fn a_blow_cracks_along_the_rocks_own_joints_and_not_in_rays() {
        // **The owner's first ask, as a property rather than a picture.**
        // 2026-08-29, on a blind A/B of two ray tunings: *"neither, fully
        // get rid of the lines that it makes -- it should just make cracks
        // similar to an explosion."* `score_cracks` drew five straight
        // radial rays, which cross domain interiors freely; the joint
        // fabric can only ever mark an edge whose two cells are in
        // *different* Worley domains (`fracture_field`'s module doc: the
        // rule is an identity comparison, so it has no width to leak
        // through). That difference is checkable without looking at
        // anything.
        //
        // Put `score_cracks(world, cx, cy, chip, radius * CRACK_REACH,
        // CRACK_RAYS)` back in `strike` and this goes red: rays run
        // through the middle of domains, which is what made them read as
        // lines drawn on stone.
        let mut w = World::new(Rect::new(0, 0, 127, 127)).without_chain_limit();
        for y in 0..128 {
            for x in 0..128 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        strike(&mut w, 64, 64, 7, 3.0);

        let pitch_at = |x: i32, y: i32| {
            let m = w.materials.get(w.get(x, y).material);
            super::super::fracture_field::pitch_at(w.seed, x, y, m.joint_spacing, m.joint_band_contrast)
        };
        let (mut on_joint, mut off_joint) = (0usize, 0usize);
        for y in 0..127 {
            for x in 0..127 {
                let cell = w.get(x, y);
                for (down, marked) in [(false, cell.crack_right()), (true, cell.crack_down())] {
                    if !marked {
                        continue;
                    }
                    let (nx, ny) = if down { (x, y + 1) } else { (x + 1, y) };
                    let (p, q) = (pitch_at(x, y), pitch_at(nx, ny));
                    // **Only edges whose two cells are still comparable
                    // jointed rock.** A cracked cell the blow then
                    // shattered is rubble, which has no lattice at all, so
                    // its edge cannot be classified either way -- measured,
                    // 5 of 105 edges here. That is bookkeeping, not a line:
                    // `score_cracks` marks `is_body_material` cells only, so
                    // the ray pattern this guards against never lands on
                    // rubble either and is fully visible in what is left.
                    if p <= 0.0 || p != q {
                        continue;
                    }
                    if super::super::fracture_field::domain(w.seed, x, y, p) != super::super::fracture_field::domain(w.seed, nx, ny, q) {
                        on_joint += 1;
                    } else {
                        off_joint += 1;
                    }
                }
            }
        }
        // The positive control first: a blow into solid stone has to have
        // written *something*, or the ratio below is 0/0 and passes for
        // the worst possible reason.
        assert!(on_joint + off_joint > 20, "the blow scored almost nothing in rock it could classify: {on_joint} on joints, {off_joint} off");
        assert_eq!(off_joint, 0, "{off_joint} crack edges do not lie on a joint -- something is drawing lines again");
    }

    #[test]
    fn a_blow_calves_a_whole_block_in_the_shape_the_cracks_drew() {
        // **The owner's ask, as the property it actually is.** 2026-08-29:
        // *"The chunks should break off along the crack pattern that is
        // already there... large pieces fall specifically from the
        // existing crack line when they completely surround a chunk."*
        //
        // The enclosure was already real and the cascade already found it;
        // what it did with it was hand the region to `fracture`, whose
        // power-of-two ladder is joint-blind, so the piece that came out
        // was not the polygon that was drawn. This asserts the two are the
        // same shape: at least one body a blow promotes lies **entirely
        // inside one joint domain**, which a ladder-cut fragment of a disc
        // essentially never does.
        //
        // Delete the `free_blocks_around` loop from `strike` and this goes
        // red -- what is left promotes fragments of the loosened annulus,
        // which straddle domains by construction.
        let mut w = World::new(Rect::new(0, 0, 191, 191)).without_chain_limit();
        for y in 4..188 {
            for x in 4..188 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        // Two blows: the first opens the fabric, the second closes the
        // outlines it declined (`structural::JOINT_REPEAT_BONUS`). One is
        // not reliably enough on a given seed, which is the mechanic
        // working rather than a flaky test.
        strike(&mut w, 96, 96, 10, 3.0);
        strike(&mut w, 96, 96, 10, 3.0);

        assert!(!w.chunk_bodies.is_empty(), "the blows promoted nothing at all");
        let pitch_at = |x: i32, y: i32, m: &crate::sim::material::Material| {
            crate::sim::fracture_field::pitch_at(w.seed, x, y, m.joint_spacing, m.joint_band_contrast)
        };
        let whole = w.chunk_bodies.iter().any(|b| {
            if b.cells.len() < MIN_BODY_CELLS {
                return false;
            }
            let mut one: Option<(i32, i32)> = None;
            b.cells.iter().all(|c| {
                let (x, y) = b.cell_position(c);
                let m = w.materials.get(c.material);
                let p = pitch_at(x, y, m);
                if p <= 0.0 {
                    return false;
                }
                let d = crate::sim::fracture_field::domain(w.seed, x, y, p);
                *one.get_or_insert(d) == d
            })
        });
        let sizes: Vec<usize> = w.chunk_bodies.iter().map(|b| b.cells.len()).collect();
        assert!(whole, "no promoted body is a single joint block -- the piece is not the shape the cracks drew; body sizes {sizes:?}");
    }

    #[test]
    fn a_second_blow_completes_what_the_first_one_left_open() {
        // **The owner's ask after the joint fabric landed**, 2026-08-29:
        // *"none of the cracks fully complete to break a chunk off...
        // multiple hammer hits should result in those cracks completely
        // surrounding the chunk and then the whole chunk falls out as one
        // piece."*
        //
        // `fracture_field::joint_draw` is a pure function of the domain
        // pair, so without `structural::JOINT_REPEAT_BONUS` the boundaries
        // a first blow declines are declined identically for ever: hitting
        // the same rock again writes **nothing**, and a domain missing one
        // edge of its outline is never enclosed and never comes away. That
        // is the mechanism this asserts, and it is asserted as a *count of
        // fresh edges* rather than by eye, because a picture cannot show
        // that a boundary is complete.
        //
        // Set `JOINT_REPEAT_BONUS` to 0.0 and this goes red.
        let slab = |w: &mut World| {
            for y in 4..124 {
                for x in 4..124 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
        };
        let cracked_edges = |w: &World| -> usize {
            (4..124)
                .flat_map(|y| (4..124).map(move |x| (x, y)))
                .map(|(x, y)| usize::from(w.get(x, y).crack_right()) + usize::from(w.get(x, y).crack_down()))
                .sum()
        };
        let mut w = World::new(Rect::new(0, 0, 127, 127)).without_chain_limit();
        slab(&mut w);
        // Away from the rock the first blow removes, so the count is about
        // joints opening and not about cells disappearing.
        strike(&mut w, 64, 64, 7, 3.0);
        let after_one = cracked_edges(&w);
        strike(&mut w, 64, 64, 7, 3.0);
        let after_two = cracked_edges(&w);

        assert!(after_one > 20, "the first blow opened almost nothing: {after_one} edges");
        assert!(
            after_two > after_one,
            "a second blow on rock the first one cracked must open the boundaries it declined ({after_one} -> {after_two}); \
             without that a domain missing one edge is never enclosed and no chunk ever comes away"
        );
    }

    #[test]
    fn a_blow_that_misses_a_chunk_leaves_it_flying() {
        // The other half, and the reason the one above is not enough: a
        // rule that bursts every body in the world would pass it. The
        // disc is the blow's own radius and nothing wider.
        let mut w = test_world();
        let cells: Vec<BodyCell> = (0..4)
            .flat_map(|dx| (0..4).map(move |dy| BodyCell { dx, dy, material: material::STONE, shade: 0, organism_id: 0 }))
            .collect();
        w.chunk_bodies.push(ChunkBody::falling(cells, 50.0, 50.0, 0.0));
        strike(&mut w, 10, 10, 6, 3.0);
        assert_eq!(w.chunk_bodies.len(), 1, "a blow twenty cells away must not touch a body in flight");
    }

    /// A world with a pond deep enough that no empty cell is within
    /// `DISPLACE_SEARCH` of a submerged body -- which is the whole point.
    /// `pond_top` leaves open air above so the displaced water has a
    /// surface to rise to.
    fn pond_world(pond_top: i32) -> World {
        let mut w = World::new(Rect::new(0, 0, 127, 127)).without_chain_limit();
        for x in 0..128 {
            for y in 124..128 {
                w.set(x, y, Cell::new(material::BEDROCK, 0).with_attached(true));
            }
        }
        for x in 10..118 {
            for y in pond_top..124 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        w
    }

    fn drive(w: &mut World, frames: usize) {
        for _ in 0..frames {
            crate::sim::parallel::step(w);
            step_chunk_bodies(w);
            crate::sim::scheduler::step(w);
        }
    }

    fn count_of(w: &World, name: &str) -> usize {
        let id = w.materials.id_of(name).expect("material");
        (0..128).flat_map(|y| (0..128).map(move |x| (x, y))).filter(|&(x, y)| w.get(x, y).material == id).count()
    }

    fn rows_of(w: &World, name: &str) -> Option<(i32, i32)> {
        let id = w.materials.id_of(name).expect("material");
        let ys: Vec<i32> =
            (0..128).flat_map(|y| (0..128).map(move |x| (x, y))).filter(|&(x, y)| w.get(x, y).material == id).map(|(_, y)| y).collect();
        ys.iter().copied().min().zip(ys.iter().copied().max())
    }

    /// Every cell of rock in the world, whichever phase of broken it is in.
    ///
    /// **Stone *or* rubble, and asking for only one of them is how the
    /// first version of these guards came to pass against the very code
    /// they were written to catch.** A raft that fails, breaks into rubble
    /// and floats there has no `stone` left, so a bar phrased as "no stone
    /// remains near the surface" reads clean on exactly the artifact it is
    /// about. `CLAUDE.md`'s vacuous-guard trap, hit while writing the
    /// guard for it.
    fn rock_rows(w: &World) -> Vec<i32> {
        let rubble = w.materials.id_of("rubble").expect("rubble");
        (0..128)
            .flat_map(|y| (0..128).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let m = w.get(x, y).material;
                m == material::STONE || m == rubble
            })
            .map(|(_, y)| y)
            .collect()
    }

    /// The owner's second playtest report, as a bar: "when lava turns to
    /// stone it seems to freeze in place... it should sink."
    ///
    /// **Mid-pond and 45 columns from either shore**, well past stone's
    /// `max_unsupported_span` of 16, so the raft is genuinely unsupported
    /// and the structural model says so. What used to stop it was not the
    /// model at all: `displace` looks for an *empty* cell within four rings
    /// to shove the water into, a pond has none, so water read as a wall.
    /// Measured before the fix: **72 stone cells still standing at rows
    /// 84..90 after 600 frames**, with 41 repeat failures. The control that
    /// named the cause is the shallow-pond case below.
    #[test]
    fn an_unsupported_raft_sinks_through_a_deep_pond() {
        let mut w = pond_world(80);
        for x in 44..84 {
            for y in 80..82 {
                w.set(x, y, Cell::new(material::STONE, 0));
                w.schedule_structural_check(x, y);
            }
        }
        drive(&mut w, 600);
        let rows = rock_rows(&w);
        assert!(!rows.is_empty(), "the rock cannot all have vanished");
        let deep = rows.iter().filter(|&&y| y > 100).count();
        assert!(
            deep * 5 >= rows.len() * 4,
            "the raft should have gone to the bottom of the pond: only {deep} of {} rock cells are below row 100 (rows {:?}..{:?})",
            rows.len(),
            rows.iter().min(),
            rows.iter().max()
        );
    }

    /// **A piece sinking through water holds one speed**, instead of
    /// accelerating to the engine's own speed cap on the way down.
    ///
    /// Reported from play, on the drop the piece-integrity fix had just
    /// improved: *"it falls at slightly odd rates. There is a first group
    /// of chunks that drop too fast and then the rest that come together
    /// with the grit later."* Measured on `scene=rockdrop` with nothing
    /// holding a body back: **6.00 cells/frame** -- `MAX_SPEED_PER_AXIS`,
    /// reached *inside* the pond rather than in the air above it -- and a
    /// contact sheet with pieces on the floor while others were still at
    /// the surface.
    ///
    /// Asserted as a **ceiling on the whole descent**, not as an average:
    /// the complaint is about the fast end, and a mean over a group that
    /// includes the slow ones would hide it. Buoyancy alone does not fix
    /// it -- measured at **5.27 cells/frame** with buoyancy alone against 5.60
    /// with nothing at all -- so the guard has to be a speed rather than a
    /// rate, and both of those were red-checked against it.
    /// A tank deep enough for a body to actually reach its terminal, walled
    /// so the water cannot spread away from where the fixture says it is.
    ///
    /// `pond_world` is 44 rows deep, and at these speeds a body needs ~20
    /// rows just to accelerate up to its cap -- so a 16-cell body in it is
    /// measured mid-acceleration and reports a terminal it never reached.
    fn deep_tank() -> World {
        let mut w = World::new(Rect::new(0, 0, 95, 255));
        for x in 0..96 {
            for y in 250..256 {
                w.set(x, y, Cell::new(material::BEDROCK, 0).with_attached(true));
            }
        }
        for y in 0..250 {
            w.set(9, y, Cell::new(material::STONE, 0).with_attached(true));
            w.set(86, y, Cell::new(material::STONE, 0).with_attached(true));
        }
        for x in 10..86 {
            for y in 40..250 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        w
    }

    /// The fastest a square stone body `side` cells across goes once it is
    /// well down the tank -- its terminal, since a terminal is a cap and
    /// gravity holds it there.
    fn terminal_of(side: i32) -> f32 {
        let mut w = deep_tank();
        let cells: Vec<BodyCell> = (0..side)
            .flat_map(|dx| (0..side).map(move |dy| BodyCell { dx, dy, material: material::STONE, shade: 0, organism_id: 0 }))
            .collect();
        w.chunk_bodies.push(ChunkBody::falling(cells, 40.0, 45.0, 0.0));
        let mut fastest = 0.0f32;
        for _ in 0..900 {
            crate::sim::parallel::step(&mut w);
            step_chunk_bodies(&mut w);
            for b in &w.chunk_bodies {
                // The middle of the tank: past the acceleration run-up at
                // the top, clear of the floor at the bottom.
                if b.y > 140.0 && b.y < 210.0 {
                    fastest = fastest.max((b.vx * b.vx + b.vy * b.vy).sqrt());
                }
            }
        }
        fastest
    }

    /// A big piece sinks faster than a small one, by the square root of its
    /// size.
    ///
    /// # The guard the flat clamp did not have, and could not have failed
    ///
    /// `SINK_SPEED` normalised terminal velocity on density alone, so every
    /// stone body in the world sank at 1.8 whatever its size -- measured on
    /// `scene=rockdrop` as *"smallest piece 4 cells across at 1.76, largest
    /// 17 across at 1.75"*. Every test in this file passed, because none of
    /// them ever compared two sizes; `a_piece_sinking_through_water_does_not
    /// _run_away` bounds one body against an absolute number, which a flat
    /// clamp satisfies perfectly.
    ///
    /// A paired comparison instead, per `CLAUDE.md`: three sizes in the same
    /// tank cancels everything the rule under test is not about.
    #[test]
    fn a_big_piece_sinks_faster_than_a_small_one() {
        let (small, middling, large) = (terminal_of(3), terminal_of(8), terminal_of(16));
        println!("terminal: 3 across {small:.2}, 8 across {middling:.2}, 16 across {large:.2} cells/frame");
        assert!(small > 0.0, "test setup: the 3-cell body never reached the measurement band");
        assert!(
            small < middling && middling < large,
            "terminals do not rise with size: {small:.2}, {middling:.2}, {large:.2}"
        );
        // `v = sqrt(2 g d (rho - 1) / Cd)`, so the ratio between two sizes is
        // the square root of the ratio of their sizes and nothing else --
        // no constant in it to fit, which is why this asserts the ratio
        // rather than the three speeds. Loose because a body is measured
        // against a moving column of water it is itself pushing about.
        let predicted = (16.0f32 / 3.0).sqrt();
        let measured = large / small;
        assert!(
            (measured / predicted - 1.0).abs() < 0.35,
            "16-across sinks {measured:.2}x the 3-across; the square-root law predicts {predicted:.2}x"
        );
    }

    #[test]
    fn a_piece_sinking_through_water_does_not_run_away() {
        let mut w = pond_world(80);
        // **Walled**, for the reason
        // `a_body_entering_water_at_speed_reports_a_crown_and_one_sliding_in_does_not`
        // records: the bare pond spreads to the world edges and drops seven
        // rows, so "the surface" is not where the fixture says it is -- and
        // a body four rows below row 80 is then still in open air. Measured
        // without the walls: 4.85 cells/frame "underwater", all of it above
        // the actual water.
        for y in 0..124 {
            w.set(9, y, Cell::new(material::STONE, 0).with_attached(true));
            w.set(118, y, Cell::new(material::STONE, 0).with_attached(true));
        }
        let cells: Vec<BodyCell> =
            (0..6).flat_map(|dx| (0..4).map(move |dy| BodyCell { dx, dy, material: material::STONE, shade: 0, organism_id: 0 })).collect();
        // Dropped from high enough to be moving fast when it arrives, which
        // is the case that showed the artifact.
        w.chunk_bodies.push(ChunkBody::falling(cells, 60.0, 20.0, 2.0));
        let mut fastest_wet = 0.0f32;
        for _ in 0..600 {
            crate::sim::parallel::step(&mut w);
            step_chunk_bodies(&mut w);
            for b in &w.chunk_bodies {
                // Only while it is actually in the water: the fall through
                // air above the pond is not what this is about, and gating
                // on it is what keeps the test from passing merely because
                // the body never got up to speed.
                if b.y > 84.0 {
                    fastest_wet = fastest_wet.max((b.vx * b.vx + b.vy * b.vy).sqrt());
                }
            }
        }
        assert!(fastest_wet > 0.0, "test setup: the body never reached the water, so this asserts nothing");
        // A six-across body of stone caps at about 1.16 under
        // `SINK_DRAG_COEFFICIENT`; the bar is triple that, comfortably under
        // the 6.00 the artifact reached and well clear of the transient a
        // body carries in with it.
        assert!(
            fastest_wet < 3.6,
            "a piece reached {fastest_wet:.2} cells/frame underwater; it is accelerating through the pond rather than sinking"
        );
    }

    /// **A piece with no water under it never takes a footprint**, and this
    /// guard is about frame cost rather than about looks.
    ///
    /// Holding the footprint closes the space a body is standing in, which
    /// on a dry scene changes the *outcome*: the body stops shedding cells
    /// to collisions on landing, more of it arrives intact, and the load
    /// model is handed a bigger connected region to judge — whose cost is
    /// superlinear in region size. Measured on `scene=strike`, which has no
    /// water in it at all: the same two failures went from 503 to 1,372
    /// cells and the worst frame from **20 ms to 118 ms**, against a 60 ms
    /// budget. See `LIQUID_LOOKAHEAD` and `Reports/open-bugs-handoff.md`
    /// §1j.
    #[test]
    fn a_piece_with_no_water_under_it_never_takes_a_footprint() {
        // Unleashed for the same reason `test_world` is: the slab is hand
        // placed and no verb touched it, so at the shipped `chain_reach`
        // nothing licenses it to come free and there is no body to ask
        // about a footprint. See `World::without_chain_limit`.
        let mut w = World::new(Rect::new(0, 0, 127, 127)).without_chain_limit();
        for x in 0..128 {
            for y in 124..128 {
                w.set(x, y, Cell::new(material::BEDROCK, 0).with_attached(true));
            }
        }
        // An unsupported slab in open air over bare bedrock.
        for x in 50..70 {
            for y in 60..64 {
                w.set(x, y, Cell::new(material::STONE, 0));
                w.schedule_structural_check(x, y);
            }
        }
        drive(&mut w, 20);
        assert!(!w.chunk_bodies.is_empty(), "test setup: the slab should have promoted to at least one body");
        assert!(
            w.chunk_bodies.iter().all(|b| !b.reserved),
            "{} of {} bodies took a footprint over dry ground",
            w.chunk_bodies.iter().filter(|b| b.reserved).count(),
            w.chunk_bodies.len()
        );

        // And the paired positive, so this cannot pass by the mechanism
        // being dead: the identical slab over a pond takes one **while it
        // is still in the air**. Sampled early on purpose -- once a body
        // reaches the water it latches on contact anyway, so a late sample
        // passes at `LIQUID_LOOKAHEAD` of zero and tests nothing.
        let mut wet = pond_world(80);
        for x in 50..70 {
            for y in 60..64 {
                wet.set(x, y, Cell::new(material::STONE, 0));
                wet.schedule_structural_check(x, y);
            }
        }
        drive(&mut wet, 4);
        assert!(!wet.chunk_bodies.is_empty(), "test setup: the slab should have promoted over the pond too");
        let (top, _) = rows_of(&wet, "water").expect("the pond is still there");
        let lowest = wet.chunk_bodies.iter().flat_map(|b| b.cells.iter().map(|c| b.cell_position(c).1)).max().unwrap_or(0);
        assert!(lowest < top, "test setup: the pieces should still be above the water at row {top} (lowest cell at {lowest})");
        assert!(
            wet.chunk_bodies.iter().any(|b| b.reserved),
            "no body took a footprint while still {} rows above the pond; LIQUID_LOOKAHEAD is not reaching",
            top - lowest
        );
    }

    /// **A body's footprint is reserved while it flies, and given back
    /// when it stops.** Nothing may be left holding space afterwards.
    ///
    /// The leak this exists for was found in a picture, not a number:
    /// `rotate_quarter` moved the body and not its reservation, so
    /// `scene=rockdrop` grew wedge-shaped **air pockets standing
    /// permanently inside the pond** — a reserved cell is not `is_empty`,
    /// so no water would ever close over it. The water ledger was
    /// perfectly balanced the whole time, because nothing had been lost;
    /// only the contact sheet showed it. Hence a guard on the *holes*
    /// rather than on the volume.
    #[test]
    fn a_body_leaves_no_reservation_behind_when_it_settles() {
        let mut w = pond_world(80);
        // Tall and narrow, so it accumulates enough speed to roll: the
        // leak was in the rotation path and a body that never turns does
        // not reach it.
        for x in 60..66 {
            for y in 40..56 {
                w.set(x, y, Cell::new(material::STONE, 0));
                w.schedule_structural_check(x, y);
            }
        }
        drive(&mut w, 600);
        assert!(w.chunk_bodies.is_empty(), "test setup: everything should have come to rest inside the budget");
        let held: Vec<(i32, i32)> = (0..128)
            .flat_map(|y| (0..128).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let c = w.get(x, y);
                c.material == material::EMPTY && c.managed()
            })
            .collect();
        assert!(
            held.is_empty(),
            "{} cells are still reserved with no body over them, e.g. {:?} -- each one is a hole water can never fill",
            held.len(),
            &held[..held.len().min(6)]
        );
    }

    /// **A body sinking through a pond neither eats nor makes water.**
    ///
    /// The bar the first attempt at footprint reservation failed: it kept
    /// bodies whole and lost **1,821 cell-equivalents** of water on
    /// `scene=rockdrop`, because every search that hands displaced fluid
    /// somewhere to go reads `is_empty` and so could not see the cells the
    /// body was vacating. `exchange_with_fluid` pairs them by construction
    /// instead. See `Reports/open-bugs-handoff.md` §1h.
    ///
    /// Asserted to the **cell**, not to a percentage: this path moves whole
    /// cells and their fill together, so there is no rounding for a
    /// tolerance to absorb, and a loose bar here is how a slow leak hides.
    #[test]
    fn a_body_sinking_through_a_pond_conserves_the_water_it_displaces() {
        let mut w = pond_world(80);
        for x in 44..84 {
            for y in 78..82 {
                w.set(x, y, Cell::new(material::STONE, 0));
                w.schedule_structural_check(x, y);
            }
        }
        // After one step, so the pond has taken its own shape and the
        // number being compared is the settled one rather than the
        // fixture's.
        drive(&mut w, 1);
        // **Plus the bank, or this measures evaporation.** `water_equivalents`
        // counts what is standing in the grid; `evaporation::tick` moves a
        // drying surface into `World::atmospheric_bank`, which is where the
        // weather gives it back. Reading only the first half showed a
        // 113.7-cell "leak" on a fixture with no body in it at all.
        let ledger = |w: &World| crate::sim::weather::water_equivalents(w) + w.atmospheric_bank;
        let before = ledger(&w);
        drive(&mut w, 600);
        let after = ledger(&w);
        assert!(w.chunk_bodies.is_empty(), "test setup: everything should have come to rest inside the budget");
        assert!(
            (after - before).abs() < 1.0,
            "a raft sinking through the pond moved the ledger from {before:.1} to {after:.1} cell-equivalents"
        );
    }

    /// **What reaches the floor is rock, not dust.**
    ///
    /// Reported from play twice: *"they don't look like chunks when they
    /// fall, they are still mostly dust when they sink"*, and then *"chunks
    /// of rock hit the water and then start disintegrating into grit
    /// instead of tumbling down as rock chunks."*
    ///
    /// **A material census, not an event count**, and that distinction is
    /// the whole reason the first version of this work was reported as
    /// fixed when it was not: `FailureCounts` and the peak-bodies counter
    /// both said 85% of the mass left as chunks while `scene=rockdrop`
    /// ended `rock -600, rubble +572` -- every cell of the slab ground to
    /// powder, the same rock broken four times over. A player watches
    /// neither counter; he watches what is lying on the bottom.
    #[test]
    fn a_slab_that_sinks_arrives_as_rock_rather_than_as_powder() {
        let mut w = pond_world(80);
        for x in 44..84 {
            for y in 78..82 {
                w.set(x, y, Cell::new(material::STONE, 0));
                w.schedule_structural_check(x, y);
            }
        }
        drive(&mut w, 600);
        let stone = count_of(&w, "stone");
        let rubble = count_of(&w, "rubble");
        // 160 cells went in. Measured **151 stone against 4 rubble** with
        // the exchange, and **0 stone against 160 rubble** without it --
        // run on a worktree at the parent commit rather than by ablating a
        // line here, so the comparison is against the code that shipped.
        // The bar is half the slab: far below what it does and far above
        // anything the bug ever allowed, which was nothing at all.
        assert!(
            stone * 2 >= 160,
            "only {stone} of 160 cells reached the floor as stone ({rubble} as rubble); the slab is being re-broken on its way down"
        );
    }

    /// **A boulder entering water at speed reports a crown; one sliding in
    /// does not.**
    ///
    /// Reported from play as *"I don't see any splash"*, on a scene of
    /// scattered sand — and the honest answer was that a falling rigid body
    /// had never reported a splash site from anywhere.
    /// `clear_or_displaceable` shoves whole columns of water out of a
    /// body's way and never went near `report_splash`, so rock into water
    /// was silent. See `report_entry_splash`.
    ///
    /// The paired negative is what makes this a test of *entry* rather than
    /// of wetness: the identical body started at the surface at rest is
    /// under water before gravity gets it past `SPLASH_MIN_ENTRY_SPEED`, so
    /// the scan finds no free surface beside it and it reports nothing. A
    /// version without that control would pass against a rule that splashed
    /// every frame of a long sink.
    ///
    /// # Why this counts *sites*, and where the droplet count is asserted
    ///
    /// A site only becomes a droplet if the water it names is full —
    /// `particle::SPLASH_MIN_FILL`, because taking a whole droplet out of a
    /// part-empty cell is water from nowhere. A settled pond's top row is
    /// the remainder of its volume and is usually not full: this fixture's
    /// measures 570 to 800 within ten frames of settling, so every site is
    /// reported and every one correctly declined. That is `throw_splashes`
    /// working, and it made a first version of this test read as though the
    /// reporting were dead.
    ///
    /// The end-to-end number is measured on the scene instead, where a pool
    /// deep enough to stay full at the surface exists: `filmstrip
    /// scene=rockdrop` drops a 600-cell slab and throws **61 droplets
    /// against 2** with `report_entry_splash` ablated, at an unchanged 25
    /// chunk bodies in flight.
    #[test]
    fn a_body_entering_water_at_speed_reports_a_crown_and_one_sliding_in_does_not() {
        let sites_from = |start_y: f32, vy: f32| {
            let mut w = pond_world(80);
            // Walled, unlike `pond_world` on its own: that pond has no
            // sides, spreads to the world edges and drops seven rows, so
            // "the surface" is not where the fixture says it is.
            for y in 0..124 {
                w.set(9, y, Cell::new(material::STONE, 0).with_attached(true));
                w.set(118, y, Cell::new(material::STONE, 0).with_attached(true));
            }
            let cells: Vec<BodyCell> = (0..6)
                .flat_map(|dx| (0..4).map(move |dy| BodyCell { dx, dy, material: material::STONE, shade: 0, organism_id: 0 }))
                .collect();
            let mut body = ChunkBody::at(cells, 60.0, start_y);
            body.vy = vy;
            w.chunk_bodies.push(body);
            let mut reported = 0;
            for _ in 0..60 {
                crate::sim::parallel::step(&mut w);
                step_chunk_bodies(&mut w);
                // Sampled here rather than at the end: `begin_step` clears
                // the list every frame, which is what makes it a per-frame
                // candidate set rather than a running total.
                reported += w.splash_sites.len();
            }
            reported
        };

        // Dropped from six rows up: well past `SPLASH_MIN_ENTRY_SPEED`
        // when it reaches the water at row 80.
        let fast = sites_from(74.0, 1.4);
        assert!(fast > 0, "a boulder hitting open water at speed reported no splash site at all");

        // Started at the surface at rest. `GRAVITY` is 0.15/frame, so one row
        // of fall reaches 0.55 and it is a couple of rows under before it is
        // moving fast enough to qualify -- by which point it is no longer at
        // a free surface to splash from. (This comment said 0.05 and
        // "several rows"; both were wrong, and `SPLASH_MIN_ENTRY_SPEED`'s
        // doc records where that number came from.)
        let gentle = sites_from(79.0, 0.0);
        assert!(
            gentle < fast,
            "a body sliding in gently reported {gentle} sites against {fast} for one arriving at speed -- \
             the speed gate is not doing anything"
        );
    }

    /// The control for the one above, and the measurement that named the
    /// cause rather than guessing it: the identical raft on a pond shallow
    /// enough that open air *is* inside the four-ring shove already sank
    /// before the fix. If this ever fails, the problem is not buoyancy.
    #[test]
    fn the_same_raft_already_sank_when_the_pond_was_shallow() {
        let mut w = pond_world(121);
        for x in 44..84 {
            for y in 121..123 {
                w.set(x, y, Cell::new(material::STONE, 0));
                w.schedule_structural_check(x, y);
            }
        }
        drive(&mut w, 600);
        let (_, bottom) = rows_of(&w, "stone").expect("the raft is still somewhere");
        assert!(bottom >= 122, "a raft on a three-row pond should reach the floor (bottom row {bottom})");
    }

    /// **Buoyancy is a flag, not a density comparison** -- `ice.ron` says so
    /// and `is_resting_on_ground` already reads it. Asking density instead
    /// took `scene=coldsnap` from 1 overload failure to 23 plus 12
    /// unsupported, because a mostly-ice region that has picked up one
    /// stone cell averages over water's 1.0.
    ///
    /// The same geometry as the sinking raft, in the one material that
    /// claims to float. It must not sink, and it must not be dismantled.
    #[test]
    fn a_raft_of_something_buoyant_does_not_sink() {
        // **A manufactured buoyant material, not `ice`.** Ice is the one
        // shipped material that sets `floats`, and it cannot be used here:
        // its melting point is *below* ambient on purpose -- it survives
        // only while a front's cold band is overhead -- so a fresh ice cell
        // in a world with no weather in it melts on its first visit. Written
        // with ice first and it failed with the floe simply gone, which is
        // `CLAUDE.md`'s "a scene that contradicts the code will look like a
        // bug in the code" in its cheapest form.
        //
        // Loaded through a synthetic reload, the same way the span test
        // below widens stone, because that is the only public way content
        // changes at runtime. Two properties are deliberate: it is
        // **heavier than water** (1.2 against 1.0), so the only thing that
        // can be holding it up is the flag rather than density; and its
        // span matches ice's 48, so the structural model is not
        // simultaneously trying to take the sheet apart and the test is
        // about buoyancy alone.
        let mut w = pond_world(80);
        let dir = std::env::temp_dir().join("pixel-physics-buoyant-raft");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("cork.ron"),
            "(name: \"cork\", kind: Solid, density: 1.2, floats: true, colors: [(200,170,120)], \
              max_unsupported_span: 48, breaks_into: \"rubble\")",
        )
        .unwrap();
        w.materials.reload(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        let cork = w.materials.id_of("cork").expect("the manufactured material loaded");
        for x in 44..84 {
            for y in 80..82 {
                w.set(x, y, Cell::new(cork, 0));
                w.schedule_structural_check(x, y);
            }
        }
        // **Short, because ice melts at ambient.** `ice.ron`'s melting
        // point is *below* 20 C on purpose -- it survives only while a
        // front's cold band is overhead -- so a 600-frame run in a world
        // with no weather in it asserts nothing except that ice melts.
        // Written that way first and it failed with the floe simply gone,
        // which is `CLAUDE.md`'s "a scene that contradicts the code will
        // look like a bug in the code" in its cheapest form. 150 frames is
        // several times what the stone raft needs to visibly move.
        drive(&mut w, 600);
        let (top, bottom) = rows_of(&w, "cork").expect("the floe is still somewhere");
        // The paired half of `an_unsupported_raft_sinks_through_a_deep_pond`
        // above: identical geometry, identical run length, and that one
        // ends past row 100 on the pond floor. Bounded well clear of it
        // rather than pinned to the opening rows, because a sheet embedded
        // in the surface does settle a little as the water it displaced
        // levels out -- what must not happen is that it goes under.
        assert!(
            bottom < 95,
            "a buoyant raft should stay near the surface, not sink to rows {top}..{bottom} -- and it is denser than the water, so only the flag can be holding it up"
        );
    }

    /// **A body that settles underwater must not be destroyed.**
    ///
    /// It was: `settle` places a cell only into empty space, falls back to a
    /// four-ring search for one, and drops it otherwise -- and a body at
    /// rest in a pond has water in every one of its cells and no empty cell
    /// within reach of any of them. Measured on the raft above: **80 cells
    /// in, 9 cells out**, silently, at the one moment a body stops being
    /// tracked.
    ///
    /// Counts rock rather than positions, because where it ends up is the
    /// other tests' business and this one is only about the census.
    #[test]
    fn a_body_settling_underwater_loses_nothing() {
        let mut w = pond_world(80);
        let mut placed = 0;
        for x in 44..84 {
            for y in 80..82 {
                w.set(x, y, Cell::new(material::STONE, 0));
                w.schedule_structural_check(x, y);
                placed += 1;
            }
        }
        drive(&mut w, 600);
        let after = count_of(&w, "stone") + count_of(&w, "rubble");
        // **A bar set from measurement with the gap left visible, not a
        // claim of perfection.** 80 in, 64 out here, against 9 out before
        // the swap arm existed. The residue is not about water: the same
        // raft dropped in plain air onto bedrock comes back 72 of 80,
        // because `settle` reaches four rings for an empty cell and a body
        // that came to rest overlapping the floor -- which rotation and the
        // fractional origin make ordinary -- loses whatever sits deeper
        // than that. That general landing loss is logged separately in
        // `Reports/open-bugs-handoff.md`; this bar is the underwater case
        // it used to be eight times worse than.
        let lost = placed - after;
        assert!(
            lost <= 20,
            "{placed} cells of rock went into the pond and {after} came out ({lost} lost; the bar is 20, measured 16)"
        );
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
        //
        // **The `aux` half of this test was retired 2026-08-27, and the way
        // it went is worth more than the assertion was.**
        //
        // It asserted a *proxy* -- `aux <= max_unsupported_span` on the
        // landed cells -- and never ran a structural check, so it could not
        // observe re-breaking at all, only guess at it from a stored number.
        // When `settle` changed to write `u16::MAX` instead of `Cell::new`'s
        // 0 (`Reports/structural-support-model.md` §6.4: a landed body
        // claiming `aux 0` is one of the two false anchors that produce §S),
        // the proxy went red while the behaviour it stands for was fine.
        //
        // The obvious repair was to assert the property instead -- settle,
        // run the structural system, check the debris is still there. That
        // was written, and it passes, **and it is blind**. Two faults were
        // injected and it stayed green through both: `settle` writing
        // `span + 1`, and `is_resting_on_ground` returning `false` outright.
        // The reason is the scene: the slab lands on the bedrock floor, so
        // `load::is_anchor` answers through `touches_bedrock` whatever the
        // stored distance says, and debris on bedrock cannot fail. A guard
        // that cannot go red is not weak, it is blind (`CLAUDE.md`), so it
        // was not kept.
        //
        // What actually stands behind the landing value is whole-world and
        // not a unit test: the `RECONVERGE_AT` oracle (36,348 wrong cells ->
        // **186**, climb bucket 0) and `scripts/acceptance.sh` green on all
        // 23 cases. `!attached()` below is kept because it *is* checkable
        // here -- re-attaching is a property of the write, not of the scene.
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
        let landed = (0..64)
            .flat_map(|x| (0..64).map(move |y| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == material::STONE)
            .count();
        assert!(landed > 0, "test setup: nothing landed as stone, so the assertions below are vacuous");

        for (x, y) in (0..64).flat_map(|x| (0..64).map(move |y| (x, y))) {
            if w.get(x, y).material == material::STONE {
                assert!(
                    !w.get(x, y).attached(),
                    "landed debris must not re-attach to the background, or a fallen chunk becomes immovable terrain"
                );
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

    #[test]
    fn working_a_face_eats_into_it_along_one_pattern() {
        // **This replaces `working_the_same_spot_drives_a_crack_deeper`,
        // and it failed rather than quietly passing** -- which is the
        // whole reason it is being rewritten instead of deleted. That test
        // measured `crack_reach` -- the furthest cracked cell from the
        // impact, a helper deleted with it -- and asserted a second blow
        // pushed it further out. That
        // was `CRACK_TIP_BONUS`, a property of the ray walker, and the
        // rays are gone (`strike`'s crack call; `Reports/dead-ends.md`).
        //
        // The player-facing promise underneath it survives and is what is
        // asserted here, in the two halves it actually has:
        //
        // 1. **the pattern belongs to the rock, not to the swing** -- so
        //    repeats deepen one set of fissures instead of scribbling
        //    fresh ones beside them, which is the complaint the fabric
        //    exists to answer;
        // 2. **and working the face gets you through it** -- the wound
        //    advances, which is what `face_toward` does for a real swing
        //    and what "a span you cannot chew through can still be worked"
        //    means now.
        let slab = |w: &mut World| {
            for y in 4..60 {
                for x in 4..60 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
        };
        // Two independently built worlds rather than a clone: `World` is
        // not `Clone`, and building twice also proves the sameness below
        // comes from the fabric rather than from state the first left.
        let mut once = test_world();
        slab(&mut once);
        let mut again = test_world();
        slab(&mut again);
        strike(&mut once, 32, 32, 5, 3.0);
        strike(&mut again, 32, 32, 5, 3.0);
        let cracks = |w: &World| -> Vec<(i32, i32, bool, bool)> {
            (4..60)
                .flat_map(|y| (4..60).map(move |x| (x, y)))
                .filter(|&(x, y)| w.get(x, y).cracked())
                .map(|(x, y)| (x, y, w.get(x, y).crack_right(), w.get(x, y).crack_down()))
                .collect()
        };
        assert!(!cracks(&once).is_empty(), "a single blow scored no fissures at all");
        assert_eq!(cracks(&once), cracks(&again), "the same blow on the same rock must open the same joints, or repeats scribble");

        // And the face advances. The second blow is aimed where the first
        // one's near face ended up, which is what `player::face_toward`
        // hands `strike` for a real swing.
        let stone_left = |w: &World| (4..60).flat_map(|y| (4..60).map(move |x| (x, y))).filter(|&(x, y)| w.get(x, y).material == material::STONE).count();
        let after_one = stone_left(&once);
        strike(&mut once, 32 + 5, 32, 5, 3.0);
        assert!(stone_left(&once) < after_one, "working the face has to keep taking rock off it: {after_one} -> {}", stone_left(&once));
    }

    // --- T1: the fragment ladder's floor, the flood's neighbourhood, and
    // --- the three tiers a severed organism comes apart into.

    /// A `wood` cell owned by `organism_id`, the same shape
    /// `structural.rs`'s own organism tests build.
    fn organism_wood(w: &mut World, organism_id: u16) -> Cell {
        let wood = w.materials.id_of("wood").unwrap();
        Cell::new(wood, 0).with_organism_id(organism_id)
    }

    /// A world with a tree species pushed and a bedrock floor, for the
    /// severance tests below.
    fn tissue_world() -> (World, u16) {
        let mut w = test_world();
        for x in 0..64 {
            w.set(x, 63, Cell::new(material::BEDROCK, 0).with_attached(true));
        }
        let species = w.species.id_of("tree").expect("tree species must be loaded");
        let id = w.push_organism(species).expect("a fresh world has organism slots free");
        (w, id)
    }

    /// `fragment_floor` defaults to 1, which is the exponent the ladder
    /// started at before the field existed -- so every `.ron` that says
    /// nothing about it must draw exactly the fragment sizes it always did.
    ///
    /// Asserted on `stone`, the material every destruction acceptance case
    /// in the repo is calibrated against.
    #[test]
    fn the_fragment_ladder_floor_defaults_to_the_exponent_that_shipped() {
        let w = test_world();
        assert_eq!(
            w.materials.get(material::STONE).fragment_floor,
            1,
            "moving rock's ladder is not what `fragment_floor` is for -- stone must draw 2..32 exactly as it always has"
        );
        assert!(!w.materials.get(material::STONE).woody, "rock floods at four on purpose; see `NEIGHBOURS_8`");
    }

    /// And `wood` is the material that asked for it. Read off the registry
    /// rather than the file, so a `.ron` edit that does not reach the
    /// binary (`CLAUDE.md`'s `include_str!` gotcha) fails here rather than
    /// producing a sweep of identical runs.
    #[test]
    fn wood_starts_its_ladder_where_a_log_is_possible() {
        let w = test_world();
        let wood = w.materials.id_of("wood").expect("wood");
        let m = w.materials.get(wood);
        assert!(m.woody, "wood must fragment at eight and sever as pieces");
        // The smallest rung has to clear `MIN_BODY_CELLS`, which is the
        // whole defect: two of the default ladder's five rungs were under
        // it and were grit before shape was considered.
        assert!(
            1usize << m.fragment_floor >= MIN_BODY_CELLS,
            "wood's smallest fragment target ({}) must be able to become a body at all",
            1usize << m.fragment_floor
        );
        assert_eq!(w.materials.get(m.severs_into.expect("wood needs a piece tier")).name, "log");
    }

    /// The single highest-leverage line in the package, guarded directly:
    /// the same diagonal staircase floods as **one** fragment for organism
    /// tissue and as **five** for rock.
    ///
    /// A crown is mostly diagonal twigs, so a four-connected flood cuts it
    /// at every one of them before the size ladder gets a say.
    #[test]
    fn organism_tissue_floods_diagonally_where_rock_does_not() {
        let (mut w, id) = tissue_world();
        let staircase: Vec<(i32, i32)> = (0..5).map(|i| (20 + i, 20 + i)).collect();
        for &(x, y) in &staircase {
            let cell = organism_wood(&mut w, id);
            w.set(x, y, cell);
        }
        let mut left: HashSet<(i32, i32)> = staircase.iter().copied().collect();
        let woody = take_fragment(&w, &mut left, (20, 20), 48, true);
        assert_eq!(woody.len(), 5, "`Grow` placed these at eight, so a flood that reads them back must too");

        let mut left: HashSet<(i32, i32)> = staircase.iter().copied().collect();
        let rock = take_fragment(&w, &mut left, (20, 20), 48, false);
        assert_eq!(
            rock.len(),
            1,
            "rock stays four-connected on purpose -- see `diagonal_only_contact_does_not_connect_two_components`"
        );
    }

    /// The three tiers, on one severed region: wood becomes a **piece**,
    /// the foliage hanging off that wood **rides down with it**, and
    /// foliage no piece reaches **scatters**.
    ///
    /// The middle claim is the one that changed and the one worth guarding.
    /// Converting every leaf at the instant of severance is what the owner
    /// saw as *"the branches fall off as whole pieces (good), but then hit
    /// the ground and turn to dust"* — roughly 1,570 cells of `litter`
    /// created in a single frame against 710 of `log` at rest. A leafy limb
    /// comes off a real tree with its leaves on.
    #[test]
    fn a_severed_limb_carries_its_own_foliage_down() {
        let (mut w, id) = tissue_world();
        let leaf = w.materials.id_of("leaf").expect("leaf");
        let log = w.materials.id_of("log").expect("log");
        // A 12x4 slab of wood -- comfortably over `MIN_BODY_CELLS` -- with
        // a row of foliage hanging off its underside.
        let mut region = Vec::new();
        for x in 20..32 {
            for y in 20..24 {
                let cell = organism_wood(&mut w, id);
                w.set(x, y, cell);
                region.push((x, y));
            }
            w.set(x, 24, Cell::new(leaf, 0).with_organism_id(id));
            region.push((x, 24));
        }
        region.sort_unstable();

        let (severed, as_pieces) = fell_severed_tissue(&mut w, &region, (20, 20));
        assert_eq!(severed, region.len() as u32, "every cell of the region left the tree");
        assert!(as_pieces >= 32, "48 cells of wood at wood's own ladder must promote as pieces, not grit -- got {as_pieces}");
        assert!(!w.chunk_bodies.is_empty(), "a promoted piece is a body, and a body is the thing a player can watch move");
        assert!(
            w.chunk_bodies.iter().flat_map(|b| b.cells.iter()).all(|c| c.organism_id == id),
            "a body lifted out of a tree has to remember which tree, or it lands as inert wood"
        );
        // The foliage left the grid **with** the wood rather than being
        // converted where it hung.
        let carried = w.chunk_bodies.iter().flat_map(|b| b.cells.iter()).filter(|c| c.material == leaf).count();
        assert!(carried >= 8, "the leaves hanging off a promoted limb must ride down on it, not turn to powder in mid-air -- {carried} of 12 carried");
        assert_eq!(w.materials.get(log).name, "log", "test setup: the piece tier must exist");
    }

    /// And the half that must **not** change: a leaf never *seeds* a
    /// fragment and never sizes one, so a clump of pure foliage takes no
    /// draw off wood's ladder.
    ///
    /// This is `Reports/physical-trees-design-2026-08-23.md` §5.3's actual
    /// argument, and it is what keeps "leaves ride down" from quietly
    /// becoming "leaves are logs".
    ///
    /// **What changed, and why the old assertion had to go.** This used to
    /// require the unclaimed clump to *scatter* — every cell converted to
    /// `litter` where it hung. Owner, 2026-08-30: *"limbs always land as
    /// pieces nothing should be turning to dust at all!"*, the third time
    /// that complaint has been made about this pipeline. Scattering is the
    /// other half of the dust: a crown is roughly a third leaf by count, so
    /// a limb whose wood all landed in one fragment still shed hundreds of
    /// powder cells around it. Unclaimed foliage now comes down as its own
    /// cluster body.
    ///
    /// The seed rule is what this still guards, and it is untouched: the
    /// clump is not promoted by *taking a rung off wood's ladder*, it is
    /// promoted whole by the sweep that follows, so a leaf still never
    /// chooses a fragment size.
    #[test]
    fn foliage_no_piece_reaches_falls_as_a_clump_and_never_seeds_a_fragment() {
        let (mut w, id) = tissue_world();
        let leaf = w.materials.id_of("leaf").expect("leaf");
        let mut region = Vec::new();
        for x in 20..32 {
            for y in 20..22 {
                w.set(x, y, Cell::new(leaf, 0).with_organism_id(id));
                region.push((x, y));
            }
        }
        region.sort_unstable();

        let (severed, as_pieces) = fell_severed_tissue(&mut w, &region, (20, 20));
        assert_eq!(severed, region.len() as u32, "every cell of the region left the tree");
        assert_eq!(as_pieces, region.len() as u32, "and none of it turned to dust on the way");
        assert_eq!(w.chunk_bodies.len(), 1, "the clump is connected, so it comes down as one thing rather than 24");
        assert!(
            w.chunk_bodies[0].cells.iter().all(|c| c.material == leaf),
            "and it is still foliage -- promoting it must not have drawn a rung off wood's ladder"
        );
        for &(x, y) in &region {
            assert!(w.get(x, y).is_empty(), "every cell left the grid with the body rather than converting in place");
        }
    }

    /// A landed piece is **dead tissue**, not live wood — and a `wood` wall
    /// somebody painted is still a wall.
    ///
    /// The pair is the test. Keying the conversion on the material would
    /// turn a knocked-down player-built wall into logs; keying it on
    /// `BodyCell::organism_id` is what tells the two apart.
    #[test]
    fn a_landed_tree_piece_settles_as_log_and_painted_wood_does_not() {
        let (mut w, id) = tissue_world();
        let wood = w.materials.id_of("wood").expect("wood");
        let log = w.materials.id_of("log").expect("log");
        let bar = |dx: i32, organism_id: u16| BodyCell { dx, dy: 0, material: wood, shade: 0, organism_id };

        settle(&mut w, &ChunkBody::at((0..6).map(|dx| bar(dx, id)).collect(), 10.0, 40.0));
        settle(&mut w, &ChunkBody::at((0..6).map(|dx| bar(dx, 0)).collect(), 30.0, 40.0));

        for dx in 0..6 {
            assert_eq!(w.get(10 + dx, 40).material, log, "a promoted limb lands as dead tissue, not as living wood");
            assert_eq!(w.get(10 + dx, 40).organism_id(), 0, "and it is not re-attached to the tree it came off");
            assert_eq!(w.get(30 + dx, 40).material, wood, "a painted `wood` wall has no organism id and must land as the wall it was");
        }
    }

    /// `promote` declines to schedule a structural check around organism
    /// cells that are already leaving.
    ///
    /// The organism support search is hop-bounded, so a check fired inside
    /// a crown that is coming down reads everything past the span limit as
    /// unsupported and converts it to deadwood -- `CLAUDE.md`'s amputation
    /// gotcha, firing from inside the fall. Inert neighbours keep their
    /// check, because a ledge that just lost what was on it has genuinely
    /// changed.
    #[test]
    fn promoting_tissue_does_not_schedule_checks_into_the_crown_it_left() {
        let (mut w, id) = tissue_world();
        let piece: Vec<(i32, i32)> = (20..30).map(|x| (x, 20)).collect();
        for &(x, y) in &piece {
            let cell = organism_wood(&mut w, id);
            w.set(x, y, cell);
        }
        // The rest of the crown, still attached, one row up.
        for x in 20..30 {
            let cell = organism_wood(&mut w, id);
            w.set(x, 19, cell);
        }
        w.begin_step();
        w.take_touched_chunks();
        let before = w.active_site_count();
        promote(&mut w, &piece, None, None, None);
        assert_eq!(
            w.active_site_count(),
            before,
            "a check scheduled into tissue that is already leaving is the amputation landmine firing from inside the fall"
        );

        let mut inert = test_world();
        let slab: Vec<(i32, i32)> = (20..30).map(|x| (x, 20)).collect();
        for &(x, y) in &slab {
            inert.set(x, y, Cell::new(material::STONE, 0));
            inert.set(x, y - 1, Cell::new(material::STONE, 0));
        }
        inert.begin_step();
        let before = inert.active_site_count();
        promote(&mut inert, &slab, None, None, None);
        assert!(
            inert.active_site_count() > before,
            "rock that lost what was resting on it must still be re-asked -- the decline is about tissue, not about promotion"
        );
    }

    /// Same build, same seed, same answer -- including the new 8-connected
    /// walk, which is the place `Reports/physical-trees-design-2026-08-23.md`
    /// §2a names as the exact shape of issue #7's live determinism
    /// violation (iterating a `HashSet`).
    #[test]
    fn severing_the_same_limb_twice_produces_the_same_pieces() {
        let sizes = || {
            let (mut w, id) = tissue_world();
            let mut region = Vec::new();
            for x in 20..40 {
                for y in 20..28 {
                    let cell = organism_wood(&mut w, id);
                    w.set(x, y, cell);
                    region.push((x, y));
                }
            }
            region.sort_unstable();
            fell_severed_tissue(&mut w, &region, (20, 20));
            let mut out: Vec<Vec<(i32, i32)>> =
                w.chunk_bodies.iter().map(|b| b.cells.iter().map(|c| (c.dx, c.dy)).collect()).collect();
            out.iter_mut().for_each(|c| c.sort_unstable());
            out
        };
        assert_eq!(sizes(), sizes(), "the piece walk and the ladder must not depend on hash order");
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
        let fragment = take_fragment(&w, &mut left, (20, 20), 48, false);

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
        let fragment = take_fragment(&w, &mut left, (27, 20), 48, false);
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
        let down = take_fragment(&w, &mut left, (20, 20), 48, false);
        assert!(down.iter().all(|&(_, y)| y <= 22), "the flood crossed a scored edge travelling down");
        let mut left: HashSet<(i32, i32)> = region.iter().copied().collect();
        let up = take_fragment(&w, &mut left, (20, 25), 48, false);
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

    /// **Bug K's reproduction, and it fails today.** A body wedged in a slot
    /// must refuse to rotate when the turned shape would land in rock.
    ///
    /// `advance` builds a probe, turns it, and asks `try_step` whether the
    /// turned shape fits — but it passes the probe its **own** position as
    /// the target. `try_step`'s first guard is
    ///
    /// ```text
    ///     if (tx, ty) == (cx, cy) { continue }
    /// ```
    ///
    /// and with `ox = probe.x.round()` against `cell_position`'s
    /// `probe.x.round() + cell.dx`, the two are equal for *every* cell by
    /// construction. So every cell is skipped, `horizontal` and `vertical`
    /// are never set, `axis` is always `None`, and the caller reads that as
    /// "the turn fits". A body wedged in a gap rotates straight through the
    /// wall beside it — which is exactly the cheat the probe's own comment
    /// says it exists to prevent.
    ///
    /// **Live again as of the fix**, having been `#[ignore]`d against the
    /// defect rather than deleted, per `CLAUDE.md`: the reproduction
    /// outlives the report, so whoever fixed the probe got a red test
    /// instead of a paragraph. It is still the only thing in the engine
    /// that can tell a working probe from a vacuous one by assertion — a
    /// probe that always answers "clear" looks identical to one that works
    /// — with `FailureCounts::rotations_refused` as the running readout
    /// beside it. Was present in *both* parents of the water merge; see
    /// `open-bugs-handoff.md` bug K.
    ///
    /// The scene is a one-cell-tall slot in solid stone with a 3x1 bar lying
    /// in it. `rotate_quarter` maps `(dx, dy) -> (-dy, dx)`, so the turned
    /// bar is 1x3 and its two new cells land in the rock above and below.
    #[test]
    fn a_wedged_body_will_not_rotate_through_the_wall() {
        let mut w = test_world();
        for y in 0..64 {
            for x in 0..64 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        // The slot the bar lies in, and nothing else, is open.
        for x in 14..=16 {
            w.set(x, 32, Cell::EMPTY);
        }
        let bar = |dx: i32| BodyCell { dx, dy: 0, material: material::STONE, shade: 0, organism_id: 0 };
        let body = ChunkBody::at(vec![bar(-1), bar(0), bar(1)], 15.0, 32.0);

        // The setup check first: if the turned bar does not actually land in
        // rock, a passing assertion below proves nothing.
        for cell in &body.cells {
            let (px, py) = body.turned_cell_position(cell, Turn::Cw);
            if (px, py) != (15, 32) {
                assert_eq!(
                    w.get(px, py).material,
                    material::STONE,
                    "test setup: the turned bar must land in rock at ({px}, {py}), or this proves nothing"
                );
            }
        }
        assert!(
            !rotation_fits(&w, &body, Turn::Cw, shape_of(&w, &body)),
            "the turned bar overlaps rock above and below it, and the fit probe reported no obstruction at all"
        );

        // ...and the other half, which is what stops the fix being "return
        // false": clear the two cells the turn needs and it must fit. A
        // probe that always refuses would pass the assertion above and is
        // exactly as useless as the one that always allowed.
        w.set(15, 31, Cell::EMPTY);
        w.set(15, 33, Cell::EMPTY);
        assert!(
            rotation_fits(&w, &body, Turn::Cw, shape_of(&w, &body)),
            "with the slot opened above and below, the turned bar fits and the probe should say so"
        );
    }


    /// **The stump, not the cell that happened to fail.**
    ///
    /// `load::Failure::at` is one arbitrary cell of a severed region, and on
    /// `scene=fell` it is the crown's far *left* edge — 58 cells from the
    /// trunk. A hinge built on it has an almost horizontal `r`, so `omega x
    /// r` points straight down and the swing is indistinguishable from
    /// dropping. This pins the thing that fixed it.
    #[test]
    fn the_cut_face_is_the_stump_rather_than_the_cell_that_failed() {
        let mut w = test_world();
        // A crown 20 wide sitting on a 4-wide stump, and a `broke_at` off at
        // the crown's left edge -- the shape the real scene produced.
        let mut region: Vec<(i32, i32)> = Vec::new();
        for y in 10..20 {
            for x in 10..30 {
                region.push((x, y));
            }
        }
        // Odd width, so the stump's centre is a whole cell and this test
        // does not quietly depend on which way `round` breaks a half.
        for y in 20..24 {
            for x in 18..21 {
                region.push((x, y));
            }
        }
        for &(x, y) in &region {
            w.set(x, y, Cell::new(material::STONE, 0));
        }
        let broke_at = (10, 12);
        let stump = cut_face(&w, &region, broke_at);
        assert_eq!(stump, (19, 23), "the hinge must sit on the bottom of the stump, not at {broke_at:?}");

        // ...and the difference is not cosmetic: about the stump the centre
        // of mass is overhead, about `broke_at` it is off to one side.
        let mass_centre_x = region.iter().map(|c| c.0).sum::<i32>() as f32 / region.len() as f32;
        let mass_centre_y = region.iter().map(|c| c.1).sum::<i32>() as f32 / region.len() as f32;
        let (up_dx, up_dy) = (mass_centre_x - stump.0 as f32, mass_centre_y - stump.1 as f32);
        let (side_dx, side_dy) = (mass_centre_x - broke_at.0 as f32, mass_centre_y - broke_at.1 as f32);
        assert!(up_dy.abs() > up_dx.abs(), "about the stump the mass must be overhead: ({up_dx:.1}, {up_dy:.1})");
        assert!(side_dx.abs() > side_dy.abs(), "about the failing cell it is off to the side: ({side_dx:.1}, {side_dy:.1})");
    }

    /// **The whole mechanism, as a ratio.** A piece high on the trunk travels
    /// several times faster than one near the base, and that is what makes
    /// the assembly sweep rather than drop: it is one rigid tree expressed
    /// through fifty-odd independent bodies.
    ///
    /// The fault put back is the third assertion — with no hinge the same
    /// body's velocity is untouched here and it simply falls.
    #[test]
    fn a_hinged_piece_sweeps_and_one_at_the_stump_barely_moves() {
        let block = |ox: i32, oy: i32| {
            let cells: Vec<BodyCell> = (0..3)
                .flat_map(|dy| (0..3).map(move |dx| BodyCell { dx, dy, material: material::STONE, shade: 0, organism_id: 1 }))
                .collect();
            let mut b = ChunkBody::at(cells, ox as f32, oy as f32);
            b.hinge = Some(Hinge { pivot: (30, 40), alpha: 0.01, omega: 0.0 });
            b
        };

        // High up the trunk: 19 cells above the stump.
        let mut high = block(30, 20);
        assert!(swing_on_hinge(&mut high));
        assert!(high.vx.abs() > 10.0 * high.vy.abs(), "a piece up the trunk must swing sideways, not fall: ({}, {})", high.vx, high.vy);

        // Down at the stump: 1 cell above it.
        let mut low = block(30, 38);
        assert!(swing_on_hinge(&mut low));
        assert!(
            high.vx.abs() > 8.0 * low.vx.abs(),
            "the top must outrun the base -- that ratio is the sweep: {} against {}",
            high.vx,
            low.vx
        );

        // And with the stump gone it is an ordinary falling body: `advance`
        // gives it gravity and this gives it nothing.
        let mut free = block(30, 20);
        free.hinge = None;
        let (vx, vy) = (free.vx, free.vy);
        assert!(!swing_on_hinge(&mut free), "a body with no hinge is not on one");
        assert_eq!((free.vx, free.vy), (vx, vy), "and its velocity must be left entirely alone");
    }

    /// The stump lets go the moment the piece hits something, which is where
    /// "the bottom branches break off from the impact" comes from: the low
    /// pieces arrive first and stop, while the crown is still coming over.
    #[test]
    fn the_stump_lets_go_on_impact() {
        let cells: Vec<BodyCell> =
            (0..4).map(|dx| BodyCell { dx, dy: 0, material: material::STONE, shade: 0, organism_id: 1 }).collect();
        let mut body = ChunkBody::at(cells, 30.0, 20.0);
        body.hinge = Some(Hinge { pivot: (30, 40), alpha: 0.01, omega: 0.5 });
        body.spin_accel = 0.02;
        assert!(body.hinge.is_some(), "test setup");
        landed(&mut body);
        assert!(body.hinge.is_none(), "a landing must release the stump, or the piece keeps being driven along the arc");
        assert_eq!(body.spin_accel, 0.0, "and the break's own wind-up goes with it");
    }

    /// **The positive control for the fall, run against the closed form.**
    ///
    /// A uniform limb of `L` cells breaking at one end is the one case
    /// `angular_acceleration` can be checked against arithmetic rather than
    /// against itself: the sums are `L(L+1)/2` over `L(L+1)(2L+1)/6`, so the
    /// answer is exactly `3g/(2L+1)` radians per frame squared, converging
    /// from below on the textbook `3g/2L` for a rod about its end.
    ///
    /// Tight, on a pure function over a hand-built region, which per
    /// `CLAUDE.md` cannot be blind in an interesting way — and it pins the
    /// property the whole mechanism rests on, which is that the answer is
    /// **inversely proportional to length**. A model seeded from the
    /// breaking torque instead would grow with `L` here, so this test tells
    /// the two apart.
    #[test]
    fn a_limb_breaking_at_one_end_accelerates_as_three_g_over_two_l() {
        let mut w = test_world();
        let mut previous = f32::INFINITY;
        for l in [2i32, 4, 12, 40] {
            let cells: Vec<(i32, i32)> = (1..=l).map(|i| (10 + i, 20)).collect();
            for &(x, y) in &cells {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
            let radians = angular_acceleration(&w, &cells, (10, 20));
            let closed_form = 3.0 * GRAVITY / (2.0 * l as f32 + 1.0);
            assert!((radians - closed_form).abs() < 1e-6, "L={l}: {radians} rad/frame^2 against the closed form {closed_form}");
            assert!(radians < 3.0 * GRAVITY / (2.0 * l as f32), "L={l}: should converge on 3g/2L from below");
            assert!(radians < previous, "L={l}: a longer limb must turn more slowly, not less");
            previous = radians;
        }
    }

    /// It is a ratio of two mass sums, so a uniform limb gives the same
    /// answer whatever it is made of — and a limb that is *not* uniform
    /// does not.
    ///
    /// Worth pinning because the density lookup is the one part of the sum
    /// that could silently become a constant (a wrong `MaterialId`, a
    /// default), and a mechanism that ignored mass would pass every other
    /// test here.
    #[test]
    fn the_break_weighs_the_piece_rather_than_counting_it() {
        let mut w = test_world();
        let cells: Vec<(i32, i32)> = (1..=8).map(|i| (10 + i, 20)).collect();
        for &(x, y) in &cells {
            w.set(x, y, Cell::new(material::STONE, 0));
        }
        let stone = angular_acceleration(&w, &cells, (10, 20));
        for &(x, y) in &cells {
            w.set(x, y, Cell::new(material::SAND, 0));
        }
        let sand = angular_acceleration(&w, &cells, (10, 20));
        assert!((stone - sand).abs() < 1e-7, "a uniform limb's acceleration must not depend on the material: {stone} vs {sand}");

        // Now load the far half and leave the near half light. The mass has
        // moved outward, so both sums grow -- but the second moment grows
        // with the square of the arm and the torque only linearly, so this
        // must come out *slower*, not faster.
        for &(x, y) in cells.iter().skip(4) {
            w.set(x, y, Cell::new(material::STONE, 0));
        }
        let tip_heavy = angular_acceleration(&w, &cells, (10, 20));
        assert!(tip_heavy < sand, "mass moved out along the arm must slow the turn: {tip_heavy} against {sand}");
    }

    /// Which way it goes, and the case that must not go anywhere.
    ///
    /// The balanced arm is the one that matters for scheduling: a trunk cut
    /// through at its base has its whole crown directly overhead, so this
    /// is what `scene=fell`'s biggest piece actually reads, and it is why
    /// `topple` exists at all. If a standing bole came off the break
    /// already spinning, the tipping test on landing would be decoration.
    #[test]
    fn the_break_turns_the_way_the_mass_hangs() {
        let mut w = test_world();
        let right: Vec<(i32, i32)> = (1..=8).map(|i| (30 + i, 20)).collect();
        let left: Vec<(i32, i32)> = (1..=8).map(|i| (30 - i, 20)).collect();
        let overhead: Vec<(i32, i32)> = (1..=8).map(|i| (30, 20 - i)).collect();
        for group in [&right, &left, &overhead] {
            for &(x, y) in group.iter() {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        assert!(angular_acceleration(&w, &right, (30, 20)) > 0.0, "mass hanging right of the break turns clockwise");
        assert!(angular_acceleration(&w, &left, (30, 20)) < 0.0, "mass hanging left of the break turns anticlockwise");
        assert_eq!(angular_acceleration(&w, &overhead, (30, 20)), 0.0, "a balanced column has no reason to prefer either way");
        assert_eq!(Turn::of(angular_acceleration(&w, &left, (30, 20))), Turn::Ccw);
    }

    /// **Four quarter turns are the identity, on the body and on the
    /// grid.** Put the fault back and it fails: turning about the origin
    /// instead of the centre walks a 3x21 bar clear across the world.
    ///
    /// The second assertion is the one the mechanism needed. A body whose
    /// far end teleports fifty cells in one frame is a body that fails
    /// `rotation_fits` everywhere except open sky, because the pose it is
    /// asking about is fifty cells inside the hillside.
    #[test]
    fn a_quarter_turn_pivots_about_the_piece_and_four_of_them_come_home() {
        let cells: Vec<BodyCell> = (0..21)
            .flat_map(|dy| (0..3).map(move |dx| BodyCell { dx, dy, material: material::STONE, shade: 0, organism_id: 0 }))
            .collect();
        let mut body = ChunkBody::at(cells, 30.0, 30.0);
        let before: Vec<(i32, i32)> = body.cells.iter().map(|c| body.cell_position(c)).collect();
        let (bx0, by0, bx1, by1) = body.bounds();

        body.rotate_quarter(Turn::Cw);
        let (ax0, ay0, ax1, ay1) = body.bounds();
        assert_eq!((ax1 - ax0, ay1 - ay0), (by1 - by0, bx1 - bx0), "a quarter turn swaps the extents");
        let centre_moved = (((ax0 + ax1) - (bx0 + bx1)).abs()).max(((ay0 + ay1) - (by0 + by1)).abs());
        assert!(centre_moved <= 2, "the piece must turn in place, not swing about a corner: its centre moved {} half-cells", centre_moved);

        for _ in 0..3 {
            body.rotate_quarter(Turn::Cw);
        }
        let after: Vec<(i32, i32)> = body.cells.iter().map(|c| body.cell_position(c)).collect();
        let (mut a, mut b) = (before.clone(), after);
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b, "four quarter turns must put every cell back exactly where it started");
    }

    /// Anticlockwise is the exact inverse permutation, not three more
    /// clockwise turns -- which is what `Reports/plant-mechanics-handoff-
    /// 2026-08-29.md` §3.5 costed it at, and what would have needed a fit
    /// probe on each of two intermediate poses.
    #[test]
    fn the_two_turns_undo_each_other() {
        let cells: Vec<BodyCell> = [(0, 0), (1, 0), (2, 0), (2, 1), (2, 2), (5, 7)]
            .into_iter()
            .map(|(dx, dy)| BodyCell { dx, dy, material: material::STONE, shade: 0, organism_id: 0 })
            .collect();
        let mut body = ChunkBody::at(cells, 12.0, 40.0);
        let before: Vec<(i32, i32)> = body.cells.iter().map(|c| body.cell_position(c)).collect();
        body.rotate_quarter(Turn::Cw);
        body.rotate_quarter(Turn::Ccw);
        let after: Vec<(i32, i32)> = body.cells.iter().map(|c| body.cell_position(c)).collect();
        assert_eq!(before, after, "Ccw must undo Cw exactly, cell for cell and in order");
    }

    /// **The tipping test, both ways round.** The negative control is the
    /// load-bearing half: a piece that is well seated must not be
    /// re-promoted, or a settled pile never stops moving and chunks never
    /// sleep -- which `CLAUDE.md` prices at ~8 ms/frame, because what it
    /// defeats is the dirty-rect render skip.
    #[test]
    fn a_piece_standing_on_its_end_tips_and_one_lying_flat_does_not() {
        let mut w = test_world();
        for x in 0..64 {
            for y in 40..64 {
                w.set(x, y, Cell::new(material::BEDROCK, 0).with_attached(true));
            }
        }
        // A column two cells wide and twelve tall, resting on the floor,
        // with a lump hung off the top right. The footing is the two-cell
        // base; the mass is three cells to the right of it.
        let mut cells: Vec<BodyCell> = (0..12)
            .flat_map(|dy| (0..2).map(move |dx| BodyCell { dx, dy, material: material::STONE, shade: 0, organism_id: 1 }))
            .collect();
        cells.extend((2..6).map(|dx| BodyCell { dx, dy: 0, material: material::STONE, shade: 0, organism_id: 1 }));
        let top_heavy = ChunkBody::at(cells, 30.0, 28.0);
        assert_eq!(tipping_turn(&w, &top_heavy), Some(Turn::Cw), "a column with its mass overhanging to the right must go over to the right");

        // The same body laid down: a twelve-cell footing, mass inside it.
        let flat: Vec<BodyCell> = (0..12)
            .flat_map(|dx| (0..2).map(move |dy| BodyCell { dx, dy, material: material::STONE, shade: 0, organism_id: 0 }))
            .collect();
        let lying = ChunkBody::at(flat, 30.0, 38.0);
        assert_eq!(tipping_turn(&w, &lying), None, "a slab lying flat on the floor is seated and must be left alone");

        // And a body with nothing under it at all is not resting on
        // anything, so there is no footing to be eccentric to.
        let hung: Vec<BodyCell> = (0..4).map(|dx| BodyCell { dx, dy: 0, material: material::STONE, shade: 0, organism_id: 0 }).collect();
        let in_the_air = ChunkBody::at(hung, 30.0, 10.0);
        assert_eq!(tipping_turn(&w, &in_the_air), None, "a body in open air has no footing and must not be judged against one");
    }

    /// End to end, through `step_chunk_bodies`: the column above actually
    /// goes over rather than becoming a standing pillar of terrain.
    ///
    /// The bar is the orientation of what is left on the grid, which is the
    /// quantity `filmstrip`'s `log_pieces` census reads and the one the
    /// owner's complaint is about -- *"the long skinny vertical pieces
    /// should fall over, instead of all standing upright"*.
    #[test]
    fn a_top_heavy_piece_ends_up_lying_down_rather_than_standing_as_terrain() {
        let mut w = test_world();
        for x in 0..64 {
            for y in 40..64 {
                w.set(x, y, Cell::new(material::BEDROCK, 0).with_attached(true));
            }
        }
        // **Carrying an organism id, and that is not incidental**: the fall
        // is scoped to tissue (`is_tissue`), so a body of anonymous rock
        // would exercise the gate rather than the rule and pass this test
        // for the wrong reason. `stone` has no `severs_into`, so it still
        // lands as stone and the census below is unchanged.
        let mut cells: Vec<BodyCell> = (0..12)
            .flat_map(|dy| (0..2).map(move |dx| BodyCell { dx, dy, material: material::STONE, shade: 0, organism_id: 1 }))
            .collect();
        cells.extend((2..6).map(|dx| BodyCell { dx, dy: 0, material: material::STONE, shade: 0, organism_id: 1 }));
        w.chunk_bodies.push(ChunkBody::at(cells, 30.0, 28.0));
        for _ in 0..200 {
            step_chunk_bodies(&mut w);
            if w.chunk_bodies.is_empty() {
                break;
            }
        }
        assert!(w.chunk_bodies.is_empty(), "the body must come to rest inside the budget rather than tipping for ever");
        let stone: Vec<(i32, i32)> = (0..64).flat_map(|x| (0..40).map(move |y| (x, y))).filter(|&(x, y)| w.get(x, y).material == material::STONE).collect();
        assert!(!stone.is_empty(), "the piece has to still be there");
        let width = stone.iter().map(|c| c.0).max().unwrap() - stone.iter().map(|c| c.0).min().unwrap() + 1;
        let height = stone.iter().map(|c| c.1).max().unwrap() - stone.iter().map(|c| c.1).min().unwrap() + 1;
        assert!(width > height, "it settled {width} wide by {height} tall -- a piece that overhangs its own footing must not become a standing pillar");
        assert!(w.structural_failures.topples_asked > 0, "the tipping test never ran, so whatever laid this down was not it");
    }

}

/// Guards for `is_tool_target` — that a hand tool reaches living tissue,
/// and that it still reaches rock the way it always did.
///
/// Written as unit fixtures rather than as another `scene=fell` bar for
/// `CLAUDE.md`'s guard rule: a plant grown from `DEFAULT_WORLD_SEED` is one
/// individual and cannot be swept, and the thing being guarded here is a
/// *predicate*, which hand-placed cells test exactly. `scripts/
/// acceptance.sh`'s `fell` case guards the pipeline these sit under.
#[cfg(test)]
mod tool_target_tests {
    use super::*;
    use crate::sim::cell::Cell;
    use crate::sim::chunk::Rect;
    use crate::sim::material;

    /// A block of `wood` with a floor under it. `organism_id` is set on the
    /// wood so the cells are living tissue rather than the inert
    /// hand-painted wood `structural.rs`'s burning-tree test uses — which
    /// `Reports/felling-blockers.md` §1 names as a superseded test that has
    /// never once exercised the organism branch.
    fn wood_block(organism_id: u16) -> World {
        let mut w = World::new(Rect::new(0, 0, 63, 63));
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        for x in 0..64 {
            for y in 60..64 {
                w.set(x, y, Cell::new(material::BEDROCK, 0).with_attached(true));
            }
        }
        for x in 24..40 {
            for y in 40..60 {
                w.set(x, y, Cell::new(wood, 0).with_organism_id(organism_id));
            }
        }
        w
    }

    fn wood_cells(w: &World) -> usize {
        let wood = w.materials.id_of("wood").expect("wood");
        (0..64).flat_map(|x| (0..64).map(move |y| (x, y))).filter(|&(x, y)| w.get(x, y).material == wood).count()
    }

    /// **The D2 guard.** A blow on a living trunk has to remove living
    /// trunk. Until `is_tool_target` existed this took exactly zero cells:
    /// `is_body_material` is `Solid` alone and `wood` is `Plant`, so the
    /// whole chip zone was skipped before the `organism_id` test the bug
    /// report named was ever reached.
    #[test]
    fn a_blow_cuts_living_wood() {
        let mut w = wood_block(7);
        let before = wood_cells(&w);
        strike(&mut w, 32, 50, 5, 6.0);
        let after = wood_cells(&w);
        assert!(after < before, "a blow on living wood took nothing: {before} cells before, {after} after");
    }

    /// The same blow on the same geometry with no organism owning it.
    /// Paired with the test above rather than asserted alone, because the
    /// pair is what says the predicate keys on *material kind* and not on
    /// ownership -- and a single-arm assertion would pass just as happily
    /// against a rule that had merely swapped which half it excluded.
    #[test]
    fn a_blow_cuts_inert_wood_too() {
        let mut w = wood_block(0);
        let before = wood_cells(&w);
        strike(&mut w, 32, 50, 5, 6.0);
        assert!(wood_cells(&w) < before, "a blow on hand-painted wood took nothing");
    }

    /// The chisel, which had the identical gate one function along.
    #[test]
    fn a_chisel_bores_through_living_wood() {
        let mut w = wood_block(7);
        let before = wood_cells(&w);
        let dusted = mine(&mut w, 32, 50, 4, 0.5);
        assert!(wood_cells(&w) < before, "a chisel bit on living wood took nothing");
        assert!(dusted > 0, "a dig at yield 0.5 removed no volume at all");
    }

    /// Bedrock stays exempt. `is_tool_target` widened the *kind* test and
    /// must not have widened the bedrock exclusion with it -- the world's
    /// boundary wall is not a thing a player may cut free, and
    /// `is_body_material`'s own doc records what happens when a flood fill
    /// reaches it.
    #[test]
    fn a_blow_does_not_cut_bedrock() {
        let mut w = wood_block(7);
        let floor: usize =
            (0..64).flat_map(|x| (0..64).map(move |y| (x, y))).filter(|&(x, y)| w.get(x, y).material == material::BEDROCK).count();
        strike(&mut w, 32, 62, 6, 8.0);
        let after: usize =
            (0..64).flat_map(|x| (0..64).map(move |y| (x, y))).filter(|&(x, y)| w.get(x, y).material == material::BEDROCK).count();
        assert_eq!(floor, after, "a blow cut into bedrock");
    }

    /// **The other half of §D1**: the brush was the one destructive verb
    /// the leash could not see, so at LOCAL/TIGHT/NONE erasing a trunk
    /// licensed nothing and the crown stayed up. Asserts the disturbance
    /// exists and that it actually covers the wound, which is the part an
    /// extent of zero would silently get wrong.
    #[test]
    fn erasing_with_the_brush_records_a_disturbance() {
        let mut w = wood_block(7);
        w.chain_reach = 8;
        assert!(!w.within_disturbance(32, 50), "nothing has been disturbed yet");
        w.paint_capsule((28, 50), (36, 50), 2, material::EMPTY, 1.0);
        assert!(w.within_disturbance(32, 50), "an erase through living wood recorded no disturbance");
        assert!(w.within_disturbance(36, 50), "the disturbance does not cover the far end of its own stroke");
    }

    /// Painting is not destruction, but it does change what holds what up,
    /// and the disturbance is what licenses the re-evaluation. Kept as its
    /// own case so a later change that narrows the record to erases only
    /// has to be a deliberate one.
    #[test]
    fn painting_structure_records_a_disturbance_too() {
        let mut w = wood_block(0);
        w.chain_reach = 8;
        w.paint_capsule((10, 50), (10, 50), 2, material::STONE, 1.0);
        assert!(w.within_disturbance(10, 50), "painting stone recorded no disturbance");
    }


    /// A stroke over open air is not a disturbance and must not evict a
    /// real one from the sixteen-entry ring.
    #[test]
    fn a_stroke_that_changes_no_structure_records_nothing() {
        let mut w = wood_block(0);
        w.chain_reach = 8;
        w.paint_capsule((2, 2), (6, 2), 2, material::EMPTY, 1.0);
        assert!(!w.within_disturbance(4, 2), "erasing empty sky recorded a disturbance");
    }
}
