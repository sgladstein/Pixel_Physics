//! M17: structural integrity — destructible building with no solver.
//!
//! Each `Solid` cell stores in `Cell::aux` its distance, in cells, to the
//! nearest anchor — bedrock, or the world edge (the same `Cell::OUT_OF_BOUNDS`
//! sentinel already used everywhere else a rule needs to treat the world's
//! edge as a wall). No polygons, no connected-component labelling, no
//! physics solver.
//!
//! **That distance is no longer the failure criterion**, and this file is
//! now only half the model. Distance decides *which way is downhill toward
//! an anchor* — the support forest — and `load.rs` decides what fails, by
//! comparing the bending moment of everything hanging off a cell against
//! what its section can carry. Reach answers "how far out are you", which
//! is not the question: a crack at a beam's root weakens a cell that was
//! never near any span limit, so a worked root never gave way and an
//! overhang on a two-cell ligament did not notice. See
//! `Reports/fracture-mechanics-design.md`.
//!
//! # Why attachment is stated, not inferred
//!
//! Distance-to-bedrock alone is the wrong question to ask of *bulk*
//! material. A cell buried 500 cells deep inside a mountain has a path
//! length of 500 — vastly over any sane span — while being the most
//! supported cell in the world. Read literally, that model condemns the
//! interior of every mountain.
//!
//! The reason is that the play world is a 2D vertical **slice** through a
//! 3D world (`Reports/worldgen-design.md` §0: "the world is 2D; the
//! worldgen is 3D"). A cave ceiling is held up largely by rock out of
//! plane, which the slice does not contain and cannot see.
//!
//! **Two attempts to infer that from geometry both failed, and the failure
//! is worth recording because it is the same failure twice.** First "a
//! sufficiently confined cell is an anchor", then "thickness scales how far
//! a cell can span." Each was tuned until terrain stood, and each then
//! turned out to have made everything the player built indestructible —
//! reported from play as "it only really takes effect for pretty narrow
//! stone lines." Weaken either one enough for built structures to break and
//! it starts eating the mountain instead. There is no setting that
//! separates them, because the difference is not a property of shape.
//!
//! It is a property of what the material *is*, so it is now stated:
//! `Cell::attached` marks material belonging to the background mass, and an
//! attached cell is an anchor outright. Terrain says so about itself;
//! anything standing in front of it has to earn its support through a real
//! path. **Attachment is lost by destruction** — `break_free`,
//! `rigid::fracture` and `rigid::score_cracks` all drop it — so breaking or
//! even *fracturing* a piece out of a cliff is the moment it stops being
//! held, which is what makes mining produce debris that falls. Under the
//! load model attachment buys *capacity*, never immunity.
//!
//! # Why one step's cost depends on its direction
//!
//! The relaxation used to charge a flat 1 per step regardless of where the
//! support came from, which makes a 1-cell tower built up from the ground
//! fail at exactly the height a 1-cell cantilever reaches sideways. Real
//! rock does not behave that way: it is strong in compression and weak in
//! bending and tension. `support_cost_below`/`_beside`/`_above` split that
//! flat 1 three ways, so leaning out is dear and hanging is dearest.
//!
//! `support_cost_below` was 0 — standing on rock was free — and that is
//! now 1. It existed only to stop towers snapping under the *reach* model;
//! under load a tower stands because it *carries little*, its mass sitting
//! directly above it. The zero also broke the support forest outright:
//! whole regions shared one distance, so "closer to an anchor" stopped
//! ordering support at all. See `load.rs` and `stone.ron`.
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
//! # How terrain is checked, and why it used to be exempt
//!
//! Terrain distances are computed **at generation**, once, by
//! `compute_world_distances`; `tick` then handles everything that disturbs
//! a structure afterwards (painting, erasing, an explosion — see
//! `World::paint_capsule` and `explosion::trigger`).
//!
//! It was not always this way, and the reason is worth keeping because it
//! is the same reason the model above had to change. Originally *nothing*
//! checked terrain: if every `Solid` cell were checked from frame one, the
//! sandbox's own starting terrain would immediately fail — the floor is 8
//! cells thick against a span of 3, and the decorative ledges float with no
//! path to an anchor at all — so both would have started crumbling the
//! instant M17 shipped. Pre-placed terrain's `aux` was simply left at its
//! default of 0, "which is indistinguishable from anchored for any cell
//! nothing ever asks about."
//!
//! That exemption worked for hand-placed terrain and is a landmine at
//! worldgen scale, which `Reports/worldgen-design.md` §6b says outright:
//! "the entire world becomes structurally invalid but reading as anchored,
//! and any change to when checks are scheduled causes global collapse."
//! Confinement is what makes the exemption unnecessary rather than merely
//! deferred — bulk terrain now passes a check it genuinely satisfies
//! instead of one it was never asked to sit. The floor reaches bedrock
//! directly, and the ledges reach it through the walls they are cut into.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::cell::Cell;
use super::chunk::{Rect, CHUNK_SIZE};
use super::material::{self, MaterialId, MaterialKind};
use super::organism;
use super::rigid;
use super::scheduler::{ActiveKind, ActiveSite};
use super::world::World;

/// The four-neighbourhood, in the order every tie in this subsystem breaks
/// on. `load.rs` derives support parents with the same `argmin` the
/// relaxation below uses, so it has to iterate in the identical order or
/// the two disagree about which neighbour holds a cell up whenever two
/// paths cost the same — which is most of a solid slab.
pub const NEIGHBOURS_4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// The eight-neighbourhood, for questions about *space* rather than about
/// support. Support does not travel diagonally here (a shared corner is
/// not a shared face, which is why `NEIGHBOURS_4` is what the relaxation
/// walks) but material does: `rigid::fracture` moves pieces in 8, so
/// anything asking "is there room for this to go anywhere" has to use the
/// same neighbourhood the mover does or it will call a piece trapped that
/// is about to slide out diagonally.
pub const NEIGHBOURS_8: [(i32, i32); 8] =
    [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// Frames between a structural check and its next one, if its distance is
/// still changing. Deliberately faster than plant growth's tick intervals
/// (20-45 frames) — a structural failure reads as more urgent than organic
/// growth, and a fast-but-not-instant cascade is what makes a collapse look
/// progressive rather than either frozen or all-at-once.
const STRUCTURAL_TICK_INTERVAL: u64 = 5;

/// Frames between re-checks of a cell whose *only* support is loose
/// material underneath it.
///
/// # The one anchor that can leave without telling anybody
///
/// `is_resting_on_ground` roots a support chain on a `Powder` beneath, and
/// its own doc justified that with: *"treating a granular pile as ground is
/// safe because it is under the CA sweep's control: if it flows out from
/// underneath, that write dirties the chunk and whatever sat on it is
/// re-examined."*
///
/// **That was not true.** Dirtying a chunk wakes the *sweep*; nothing on
/// that path schedules a `StructuralCheck`, and `World::set` — the write
/// seam every mover goes through — does not either. So a cell that took the
/// ground root read `aux 0`, was judged "holds", stopped rescheduling
/// itself (the tail of `tick`), and was never asked again however
/// completely the pile beneath it drained away.
///
/// Traced on `scene=lavadrop`, cell (272,252): quench stone at frame 45,
/// one rubble grain below it, distance 0, verdict holds, dropped. The
/// rubble sank; sixty frames later the stone was a lone pixel hanging in
/// open water, and `load::evaluate` — which reads the world, not the cache
/// — called it UNSUPPORTED for the rest of the run. `filmstrip`'s `poke=`
/// dropped it instantly, which is what proves it was never re-asked rather
/// than asked and refused.
///
/// # Why here and not at the grain that leaves
///
/// Telling the cell above from `update_powder` was built and measured
/// first. It is the precise trigger and it is **too expensive**: one extra
/// `World::get` per moving grain per frame took `ascii`'s parallel
/// sand-and-water stress from 7.6 ms to 9.8 ms worst frame (minimum of
/// three interleaved runs each), and gating it on `!flowing` — free, the
/// cell is already in hand — bought the cost back and lost the fix, because
/// the grains under a quench crust are all mid-flow. A full screen of sand
/// is that path's worst case by construction, and `CLAUDE.md` is explicit
/// that frame cost is a hard constraint rather than a tiebreaker.
///
/// This costs the CA sweep nothing at all: it is one more site on the
/// structural scheduler, which is already bounded by
/// `scheduler::MAX_SITES_PER_FRAME` and `load::MAX_LOAD_CELLS_PER_FRAME`,
/// and it applies only to cells with no support but the loose stuff below
/// them — the *contact row* of a detached piece, never the piece's volume
/// and never attached terrain, which reaches an anchor by relaxation and
/// takes no ground root at all.
///
/// Twelve times `STRUCTURAL_TICK_INTERVAL` — a second at 60Hz. Slow enough
/// that a settled rubble field is not a treadmill, and the owner has
/// already said a brief hang is acceptable where it buys something; fast
/// enough that a piece left stranded comes down while the player is still
/// looking at what stranded it.
const GROUNDED_RECHECK_INTERVAL: u64 = STRUCTURAL_TICK_INTERVAL * 12;

/// Why a `StructuralCheck` produced the sites it produced, counted per
/// frame and printed by `scheduler::step` when `SCHED_PASS` is set.
///
/// **Why this exists.** `SCHED_PASS` established that a *single* explosion at
/// 8192x2560 leaves the scheduler pinned at `MAX_SITES_PER_FRAME` for the
/// rest of the run, with ~8,100 sites scheduled against 2,000 drained
/// (`Reports/open-bugs-handoff.md` §S). A produced-count says the queue is
/// self-sustaining; it cannot say *which* branch of `tick` sustains it, and
/// the two candidates want opposite fixes:
///
/// - the **distance wavefront** (`moved`), which fans out to five sites and
///   is load-bearing -- dropping it froze `scene=capped` completely; within
///   it, `worsened` is the count-to-infinity climb an unanchored region
///   performs for ever, and `improved` is an ordinary convergence front that
///   terminates;
/// - the **out-of-budget defer** (`budget0`), which is fan-out 1 and is a
///   treadmill rather than a generator.
///
/// `max_aux` is the tell for the first: a settling structure's distances
/// plateau, and a region with no anchor at all climbs towards `u16::MAX` one
/// step per tick, so a max that keeps rising over thousands of frames with
/// the world's material dead still is the count-to-infinity dynamic and
/// nothing else.
///
/// Free when off: every increment is behind `enabled()`, which reads a
/// `OnceLock`, and the whole struct is zero when `SCHED_PASS` is unset.
#[derive(Default, Clone, Copy)]
pub struct TickCensus {
    /// Distance rose -- support got worse, or nothing anchors this at all.
    pub worsened: u32,
    /// Distance fell -- an ordinary convergence front, which terminates.
    pub improved: u32,
    /// Distance unchanged: this tick asked a question rather than moving one.
    pub unmoved: u32,
    /// Deferred because `world.load_budget` was already spent.
    pub budget0: u32,
    /// Deferred by the chain walk itself (`ChainVerdict::Deferred`).
    pub chain_deferred: u32,
    /// Returned early as attached bulk with no crack and no free face.
    pub uninteresting: u32,
    /// Largest distance written this frame.
    pub max_aux: u16,
}

thread_local! {
    static CENSUS: std::cell::Cell<TickCensus> = const { std::cell::Cell::new(TickCensus {
        worsened: 0, improved: 0, unmoved: 0, budget0: 0, chain_deferred: 0, uninteresting: 0, max_aux: 0,
    }) };
}

fn census_enabled() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("SCHED_PASS").is_ok())
}

fn census(f: impl FnOnce(&mut TickCensus)) {
    if !census_enabled() {
        return;
    }
    CENSUS.with(|c| {
        let mut v = c.get();
        f(&mut v);
        c.set(v);
    });
}

/// Read and reset the frame's census. Called once per frame by
/// `scheduler::step`, after the site loop.
pub fn take_tick_census() -> TickCensus {
    CENSUS.with(|c| c.replace(TickCensus::default()))
}

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
    // Organism-owned cells (`Reports/organism-substrate-design.md` §2)
    // route to a completely different check below, never this one: their
    // `aux` holds a cell-type tag and a resource scalar, not a distance,
    // once `organism_id != 0` -- reading or writing it as a distance here
    // would silently corrupt that encoding. `organism_id == 0` (inert
    // material -- hand-painted wood, a fully-reclaimed dead organism's
    // former cells) keeps this exact function, unchanged, below.
    if cell.organism_id() != 0 {
        return organism_structural_tick(world, x, y, cell);
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

    // Copied out rather than held as a `&Material` borrow, because
    // `world.set` below needs `&mut World` and every one of these is `Copy`.
    let (cost_below, cost_beside, cost_above) = {
        let m = world.materials.get(cell.material);
        (m.support_cost_below, m.support_cost_beside, m.support_cost_above)
    };

    // Bedrock is the only outright anchor. Attachment deliberately is *not*
    // one: it buys capacity instead. Anchoring on it made an undercut shelf
    // unfallable -- its interior was still attached, so still held, no
    // matter how much was dug from beneath it. Terrain still stands because
    // the massif genuinely reaches bedrock, not because it is exempt.
    //
    // **Ground is a last-resort root, not a preferred one**, and that
    // distinction is the whole of the dig cascade. Rooting a cell at 0 the
    // moment powder touches its underside makes it a *load sink*: every
    // neighbour with a longer path re-routes its load into it, which is
    // exactly "a sprinkle of sand under a beam holds the beam up". The
    // divisor in `capacity` existed to cancel that -- two modelling errors
    // roughly annulling each other, which is why tuning the divisor never
    // worked and why gating it on `parent.is_none()` moved nothing: with
    // ground rooting eagerly, *every* powder-backed cell was parentless.
    //
    // So a cell resting on powder relaxes from its neighbours like anything
    // else, and only takes the root if that leaves it with no path at all.
    // A slab cell over its own rubble keeps its lateral path and is judged
    // on its own section; a chunk that has landed on a pile with nothing
    // else to hold it still roots, still reads as supported, and still does
    // not shatter. Measured on `scene=worldcrack preset=flat`, one radius-6
    // dig, seeds 1 / 7 / 24301: 894 / 23,042 / 3,844 cells lost before.
    let firm_anchor = NEIGHBOURS_4.iter().any(|&(dx, dy)| world.get(x + dx, y + dy).material == material::BEDROCK);

    let relaxed: u16 = if firm_anchor {
        0
    } else {
        let mut best = u16::MAX;
        let mut has_usable_neighbour = false;
        let mut has_burning_neighbour = false;
        for (dx, dy) in NEIGHBOURS_4 {
            // A fracture between us carries no load, however solid the rock
            // on the far side is.
            if edge_is_cracked(world, x, y, dx, dy) {
                continue;
            }
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
            // What this step costs depends on where the support is coming
            // *from*. `y` grows downward, so `dy == 1` is the cell beneath
            // (standing on it -- compression, cheap) and `dy == -1` is the
            // cell above (hanging from it -- tension, dear). Saturates at
            // u16::MAX, i.e. "unreachable" stays unreachable.
            let step = match dy {
                1 => cost_below,
                -1 => cost_above,
                _ => cost_beside,
            };
            best = best.min(neighbour.aux().saturating_add(step));
        }
        if has_burning_neighbour && !has_usable_neighbour {
            // Every Solid neighbour that exists is mid-burn -- there is no
            // trustworthy data to relax from yet. Defer rather than reading
            // this as "no support at all", which would shatter the cell
            // the instant a neighbour catches fire regardless of its real
            // structural state.
            return vec![reschedule(world, x, y)];
        }
        best
    };

    // The last resort described above: nothing else holds this, so what it
    // is standing on does.
    //
    // `grounded_root` is kept because a 0 that came from *here* is not the
    // same claim as a 0 that came from bedrock: it is provisional on loose
    // material that can flow away without scheduling anything. See
    // `GROUNDED_RECHECK_INTERVAL`, which is what the tail of this function
    // does about it.
    let grounded_root = relaxed == u16::MAX && is_resting_on_ground(world, x, y);
    let new_distance: u16 = if grounded_root { 0 } else { relaxed };

    // Written *before* the failure test, not after, and the order is
    // load-bearing: `load::failing_region` reads stored distances, and the
    // one cell whose value it must not read stale is the cell being judged.
    let moved = new_distance != cell.aux();
    // Judged when the distance has *settled*, or when it just got **worse**.
    //
    // Settling alone is not enough, and the gap is not subtle: a piece with
    // no anchor at all never settles. Its cells climb by one each tick
    // forever -- the count-to-infinity dynamic in this module's own doc --
    // so "wait until it stops changing" waits for something that will not
    // happen, and a blob painted in open air hangs there permanently.
    // Caught by `an_unsupported_foreground_blob_does_not_hang_in_mid_air`.
    //
    // A distance that *rose* is precisely the signal that support got
    // worse, which is the direction that can cause a failure; a distance
    // that fell means a better path was found and nothing can newly break
    // because of it. So the falling half of a convergence wavefront still
    // costs nothing.
    let worsened = new_distance > cell.aux();
    // **The distance wavefront is computed once, here, and every early
    // return below carries it.** It is independent of the load model and
    // must never be skipped by it.
    //
    // It was, and the consequence was total. Three of the returns below --
    // out of budget, uninteresting, deferred -- dropped this fan-out, so a
    // cell could write a new distance and then never tell its neighbours.
    // On `scene=capped` the very first check's support search consumed the
    // whole frame budget, the tick deferred without propagating, and the
    // relaxation stopped dead: at frame **3,000** the scene still had only
    // the 10 sites `build()` seeded, every one of its 15,840 cells still at
    // `aux 0`, and not a single cell had ever been load-evaluated. The
    // column "stood" because nothing had ever asked whether it should --
    // `CLAUDE.md`'s vacuous-test failure, in the acceptance harness rather
    // than the suite, and on the one case that exists to catch regressions.
    // Worse, dispatch is ordered by `(next_frame, x, y)`, so a frozen
    // structure at low `x` wins the budget every frame and starves every
    // other structural check in the world behind it.
    let mut propagate = Vec::new();
    census(|c| {
        if worsened {
            c.worsened += 1;
        } else if moved {
            c.improved += 1;
        } else {
            c.unmoved += 1;
        }
        c.max_aux = c.max_aux.max(new_distance);
    });
    if moved {
        world.set(x, y, cell.with_aux(new_distance));
        propagate.push(reschedule(world, x, y));
        propagate.extend(schedule_solid_neighbours(world, x, y));
        // Judged only once its distance has *settled*, which is both
        // cheaper and more correct.
        //
        // Cheaper, and this is the whole difference between a playable
        // frame and a five-second one: a disturbance sends a wavefront of
        // distance changes across the whole affected structure, and every
        // cell it touches used to pay for a full subtree walk on every
        // change. Measured at 4,456 ms on `scene=strike` and 6,556 ms on
        // `scene=capped`. Each cell reschedules itself when it moves, so
        // every one of them still gets judged -- once, on the tick it stops
        // moving, instead of once per step of the wavefront.
        //
        // More correct because the support forest a load walk reads is
        // exactly the thing that is mid-flight while distances are still
        // changing, which is `Reports/load-model-handoff.md` §5.2's stale-
        // parent hazard. Waiting for the cell to settle sidesteps it.
        if !worsened {
            return propagate;
        }
    }

    /// Reschedule this cell for another look without losing the wavefront.
    macro_rules! defer {
        () => {{
            if !moved {
                propagate.push(reschedule(world, x, y));
            }
            return propagate;
        }};
    }

    // Out of budget for this frame: defer rather than judge, so a cascade
    // spreads over frames instead of spiking one. Deliberately not "assume
    // it holds and stop checking" -- that would silently drop the check.
    if world.load_budget == 0 {
        census(|c| c.budget0 += 1);
        defer!();
    }
    // Attached bulk with no crack and no free face cannot fail, so it does
    // not walk its support chain either. Checked here rather than only
    // inside `evaluate` because the chain walk below costs an evaluation
    // per ancestor, and skipping it for interior rock is the difference
    // between paying for a structure's *surface* and paying for its
    // volume -- `scene=capped`'s column is 11,520 cells of which ~800 are
    // surface.
    if !super::load::is_structurally_interesting(world, x, y) {
        census(|c| c.uninteresting += 1);
        return propagate;
    }

    // The failure criterion, and the whole point of the load model:
    // **torque > capacity**, not reach > span. See `load.rs` -- the piece
    // that comes away is the subtree this cell was actually holding up (or,
    // if nothing holds *it*, the connected region that has come free), so a
    // worked root gives way and takes the span with it instead of the tip
    // dissolving inward one cell at a time.
    // Taken out and put back, the same borrow shape `scheduler::step` and
    // `step_chunk_bodies` both use: the walk needs `&World` while the cache
    // it fills lives on the world itself.
    let mut budget = world.load_budget;
    let mut cache = std::mem::take(&mut world.load_cache);
    let verdict = super::load::failing_along_support_chain(world, x, y, &mut cache, &mut budget);
    world.load_budget = budget;
    world.load_cache = cache;
    if matches!(verdict, super::load::ChainVerdict::Deferred) {
        census(|c| c.chain_deferred += 1);
        defer!();
    }
    if let super::load::ChainVerdict::Failing(failure) = verdict {
        // **Damage stays near what was actually disturbed, and the thing
        // that has to stay near it is the *region*, not the question.**
        //
        // This was a veto on `within_disturbance(x, y)` -- the cell whose
        // scheduled check happened to run. What a failure licenses is
        // `failure.region`: for `Overloaded` the union of supported
        // subtrees (each to `MAX_SUBTREE_CELLS`), for `Unsupported` a
        // detached piece out to `MAX_REGION_CELLS` (20,000). Neither
        // `failure.at` (up to `ROOTWARD_CHECK_STEPS` support-parent hops
        // away) nor any region cell was tested at all, so **one licensed
        // check 16 cells from a blast could legally destroy a region
        // spanning the world**. Measured on the nine-charge harness before
        // this line: damage landing 79 cells out at LOCAL (reach 48) and 37
        // at TIGHT (reach 16). `CLAUDE.md`'s "which object does this rule
        // evaluate -- a cell, a section, or a whole piece?", for the third
        // time on this branch.
        //
        // Judged, then clipped -- never gated earlier. The load model keeps
        // working and the stress view keeps reading true; only the
        // consequence is bounded. Gating the *evaluation* would make
        // far-away rock unfailable in principle, which is the binary
        // immunity four earlier support models were rejected for.
        //
        // # Why this does not overturn the note at the staging call site
        //
        // That block (search `re-argue its case`) records that a judged
        // failure's remainder must not be able to lose, because "every one
        // of those is rock left hanging in open air". **That reasoning
        // stands.** It is about a collapse stranded by an exhausted budget
        // or by a re-asked question, which is a bug. Rock left standing
        // because the *player set a limit* is not a bug: it is the
        // setting's own advertised behaviour -- `wiki/structural-
        // collapse.md` says outright that at NONE "you can undercut a
        // mountain and it will sit there", and LOCAL/TIGHT are the same
        // trade in smaller portions. The staged queue itself is still never
        // re-judged; what reaches it is already inside the licence.
        let region = clip_region_to_licence(world, failure.region);
        if region.is_empty() {
            // **Not `Vec::new()`.** The old veto returned an empty vector
            // -- a fourth early return that did not carry the wavefront the
            // block above says every early return must carry.
            //
            // **Two paths reach here and they carry different things**, and
            // an earlier version of this comment claimed only one of them
            // existed ("control only reaches here when `worsened` is true").
            // That was false, and false about the *primary* path: `worsened`
            // gates only the `moved` branch (`if !worsened { return
            // propagate; }`), so a cell whose distance has **settled** --
            // `moved == false`, which is exactly the state the block at
            // :303 says a cell is judged in -- falls through to here with
            // `propagate` empty. On the rising path `propagate` already
            // holds this cell's reschedule and its neighbour fan-out and the
            // new `aux` has been written; on the settled path this returns
            // nothing, which is what the veto did.
            //
            // **The veto's own argument is restored rather than dropped**,
            // because it was never answered: "a refused cell stops being
            // rescheduled rather than spinning: it will be looked at again
            // when something disturbs it, and that something is exactly what
            // would license it to fail." That is still true of the settled
            // path. What it did not cover is the rising one, where the cell
            // has just been told its distance grew and the neighbours that
            // told it have not yet been informed -- and a piece with no
            // anchor never settles (see the module doc at :267), so on that
            // path "wait to be disturbed" means waiting for a disturbance
            // that the cell itself is generating. Hence: carry the wavefront
            // when it exists, and keep the veto's silence when it does not.
            return propagate;
        }
        // The forest this describes is about to change out from under it.
        world.load_cache.clear();
        // The **clipped** size, deliberately: these counters are read as
        // "how much did this failure take", and after the clip that is what
        // the region is.
        world.structural_failures.record(failure.mode, region.len());
        let reach = (failure.at.0 - x).unsigned_abs() + (failure.at.1 - y).unsigned_abs();
        world.structural_failures.record_reach(reach);
        // Confined rock has nowhere to go, so it cracks where it stands
        // rather than displacing. See `crush_in_place` -- and note this
        // keys the *outcome* on free space, never the criterion: the
        // failure above has already happened and been recorded.
        //
        // Nothing is rescheduled here, deliberately. A cascade is
        // neighbours losing what was holding them up, and nothing has been
        // taken away: the rock is all still standing exactly where it was.
        // There is nothing for a neighbour to learn until something opens
        // a face nearby, and whatever does that schedules its own checks.
        // Counted whether or not the rule is switched on, so that
        // `crush_confined=false` is a clean control: it changes what the
        // engine *does* about a confined failure without changing what it
        // can *see*, which is what makes an A/B on one binary a
        // measurement. It also keeps the guard tests honest -- with the
        // rule off they fail on the behaviour they are about, rather than
        // on a setup assertion that the mechanism never ran.
        let confined = !region_has_free_face(world, &region);
        if confined {
            let depth = burial_depth(world, &region);
            world.structural_failures.record_confined(region.len(), depth);
        }
        if world.crush_confined && confined {
            // Which failure it was decides what is worth doing about it,
            // and both halves of this were measured on `preset=rolling
            // seed=1 strike=20`.
            //
            // **Overloaded** is a bending failure: the rock is over
            // capacity and cracking it is exactly the answer -- it comes
            // apart into blocks where it stands. That writes cracks, which
            // change the support distances, so the relaxation wavefront in
            // `propagate` still has something to carry and is kept.
            //
            // **Unsupported** used to be dropped here outright -- no
            // crush, no fissure, no reschedule -- on this reasoning, which
            // is kept because it still describes a real case: *a pocket
            // already cut free on every side, wedged in a hole its own
            // shape. More cracks would add nothing -- there is nothing
            // left to separate -- and it cannot fall, because there is
            // nowhere to fall to.*
            //
            // **That was right for the world it was written in and is
            // wrong now that cracks sever** (W3). It assumes the region is
            // bounded by *air*, which is what `region_has_free_face`
            // actually asks -- and it asks only about `EMPTY` neighbours.
            // A crack is a bit on a cell edge and never a removed cell, so
            // a chunk the fissures cut out of the middle of solid rock has
            // no empty neighbour at all, takes this branch **every time**,
            // and is bounded not by a hole its own shape but by **intact
            // rock**. There is a great deal left to separate. Dropping it
            // is precisely the owner's *"cracks that fully surround chunks
            // of rock do not break them off"*: the sever landed and its
            // consequence was discarded one branch later, which is why
            // fixing the crack drawing alone would have changed nothing on
            // screen.
            //
            // The air-bounded pocket the old reasoning describes is still
            // real and is not harmed by going through the same door: it is
            // already cracked, `crush_in_place` is idempotent on a set bit
            // (`crushing_the_same_rock_twice_writes_nothing_the_second
            // _time`), so it writes nothing, falls through to
            // `Vec::new()`, and is left exactly as alone as before. What
            // changed is that it is no longer the only thing this branch
            // can be.
            //
            // Confinement still decides what a failure **produces**, never
            // whether it happens -- `a_confined_failure_still_fails_it
            // _just_cannot_travel` is the guard, and "confinement as an
            // anchor" stays retired (`Reports/load-model-handoff.md`
            // §6.1).
            //
            // Getting this split wrong costs real money in both
            // directions. Cracking-and-propagating the unsupported case
            // too was a treadmill: a crushed pocket's distance never
            // settles, so it re-failed, re-crushed idempotently and
            // re-queued itself at 1,120 further confined failures every
            // 400 frames with the world's material dead still throughout.
            // Dropping the wavefront for *both* cases was worse the other
            // way -- stale distances took overload failures from 162 to
            // 382, damage from 919 to 1,691 cells of rock, and pending
            // sites from ~450 to 16,052.
            //
            // A crush that writes **no new fissure** has nothing to
            // propagate. Cracks are bits, so re-crushing rock that is
            // already crushed is idempotent -- it does no damage, and it
            // also does no *good*, while still costing a load walk and a
            // reschedule every time. Measured by the load-concentration
            // session, whose change makes many more cells reach the
            // criterion: `caveshallow` went from 488 confined failures to
            // 2,988 and produced *fewer* fissured cells (332 to 284),
            // which is 6x the work for less output. Stopping when nothing
            // was written turns that from a treadmill into a one-off.
            //
            // **That test is now load-bearing for both modes**, and it is
            // the whole of what keeps the treadmill above from coming
            // back with the unsupported case in it. The number to watch is
            // `FailureCounts::confined` in `scripts/blastsweep.sh`: it
            // climbing without bound is the treadmill, and it is the one
            // reading that tells a working sever apart from one.
            //
            // # The regression W4 shipped with, and what W4b did about it
            //
            // **W4 climbed.** Measured on `scene=worldgen preset=rolling
            // seed=1 explode=300,160,20,180,60` out to frame 5,000, with
            // the world's material dead still throughout (promoted frozen
            // at 186 cells, shattered at 914, `crushed_cells` frozen at
            // 90,512 from frame 2,200): **320 further confined failures
            // every 400 frames, for ever.** The same run with only the
            // crack change (W3) settles at 383 and stops. So the residual
            // is this branch's, not the sever's -- damage terminates,
            // judgement churn does not.
            //
            // And the damage it did before it terminated was worse than
            // the churn. On the nine-charge sweep W4 was the difference
            // between a world that breaks and a world that dissolves:
            //
            // ```text
            //                       before W3/W4   W3 only   W3+W4   W3+W4, confine=0    W4b
            // promoted cells max          11,100     4,872      95         17,639      4,862
            // promoted cells min           1,726       651       8              -        654
            // cracked cells (seed 1)       2,820     4,441  84,051          1,789      6,185
            // unsupported failures         1,030    11,525 131,363            429     16,046
            // furthest failure (cells)        44        36       2             33         47
            // ```
            //
            // 84,051 cracked cells is most of the solid rock in a 512x320
            // world. The contact sheet said it plainer than any of those
            // numbers: the hillside went dark from the crater outward
            // until the whole slope was a cracked stain, and almost nothing
            // flew. That was `crush_in_place`'s own "why not crack every
            // edge" warning arriving by a different road -- *solid on
            // screen and structurally sand*.
            //
            // The mechanism was `CLAUDE.md`'s **which object does this rule
            // evaluate**. `crush_in_place` sizes its star off the region:
            // `length = extent.clamp(CRACK_MIN_LENGTH, CRACK_MAX_LENGTH)`
            // with `CRACK_MIN_LENGTH` 10, `CRACK_PRIMARIES` 3 and
            // `CRACK_FORKS_BASE` 4. That is right for an over-capacity
            // *section*, which is what it was written for. A crack-severed
            // island is a **cell** -- mean failing region size on the deep
            // charge is 1.1 -- so every one-cell island fired three
            // ten-cell rays plus four forks into the rock around it, which
            // severed more one-cell islands, which fired again. The
            // feedback was in the sizing, not in the decision to crush.
            //
            // ## What it cost to take that feedback out, and why no tuning
            // ## of these two levers buys it back
            //
            // **The deep charge's W4 gain and the nine-blast dissolve are
            // the same mechanism.** W4 took the single deep charge from 89
            // promoted cells to 186 and from 10 frames with a body to 22.
            // Three variants were measured on it, all on
            // `explode=300,160,20,180,60 start=58 every=2 count=45`:
            //
            // ```text
            //                                       promoted   frames with a body
            // W3 only                                     89                   10
            // W4 (no floor removed, no gate)             186                   22
            // W4b, sizing scaled from zero only           89                   10
            // W4b, small-end gate only (full star)        89                   10
            // W4b, both (shipped)                         89                   10
            // ```
            //
            // Read the third and fourth rows together: **either** lever
            // alone returns the deep charge to W3, because W4's extra 97
            // cells were severed by sub-six-cell islands firing ten-cell
            // stars -- which is the runaway itself. There is no setting of
            // a minimum length or a fork base that keeps the gain and drops
            // the cascade; they are one behaviour. Anyone who wants that
            // gain back has to get it from a *different* mechanism (a real
            // impulse into confined rock, say), not from this pair of
            // knobs. Do not re-tune them expecting a middle -- it was
            // looked for and it is not there.
            //
            // The gate-only variant is worth recording separately because
            // it is not simply worse: on the nine-blast sweep it promotes
            // *more* than the shipped version (seed 1: 3,399 cells against
            // 1,785) at the price of 10,662 cracked cells against 6,185.
            // It is a live option if the sweep's promotion figure ever
            // matters more than its cracking figure.
            //
            // ## The residual, which is real and is not settled
            //
            // On the same deep charge out to frame 5,000, W4b's `confined`
            // stands at 4,043 at frame 2,200 and 5,723 at frame 5,000 --
            // **+240 every 400 frames, flat, for ever**, against W3-only's
            // 383 settled by frame 1,000 and the pre-branch build's 2. It
            // is milder than W4's +320 and it is *damage-free*: fissures
            // frozen at 610 cells, cracked cells at 1,114 and material lost
            // at 692 from frame 2,200 onward, and the worst frame is 32.4
            // ms against W3-only's 33.3 ms on the same run. So it is
            // judgement churn over a rubble field, not the damage treadmill
            // the `crush_in_place(..) > 0` guard was built to stop. It is
            // still the shape `CLAUDE.md` says to stop for, and it is
            // recorded rather than tuned around. What it costs today is
            // ~7,300 pending sites standing for ever against 400.
            //
            // # W4b: two objects, two rules
            //
            // Both levers named above were taken and nothing else was.
            // `calve_depth`, `POCKET_COLLAR_THICKNESS` and the crack
            // constants are all untouched -- they are shared with the blast
            // and with the overload crush and are right for both, so a
            // retune to damp this would have changed two working mechanisms
            // to fix a third.
            //
            // The **overloaded** half keeps today's sizing byte for byte. It
            // is a *section*, it is what `crush_in_place`'s floors were tuned
            // against, and it is not what regressed: its arm of
            // `rock_with_nowhere_to_go_cracks_where_it_stands` and the
            // `caveshallow` acceptance case are the guards on that.
            //
            // The **unsupported** half is a *severed piece* and gets sizing
            // that scales from zero -- see `CrushedObject`. A one-cell island
            // now draws a zero-length star instead of the same three ten-cell
            // rays a ten-cell section gets, which is where the feedback was.
            let object = match failure.mode {
                super::load::FailureMode::Overloaded => CrushedObject::Section,
                super::load::FailureMode::Unsupported => CrushedObject::SeveredPiece,
            };
            // **A piece with nothing left to separate cracks nothing.** A
            // chip already severed on every side has no interior to break
            // into blocks; cracking it can only damage the rock *around* it,
            // which is the neighbour's business and not this failure's. So
            // below `MIN_FRACTURE_CELLS` -- the constant that already means
            // "smallest failing region worth fracturing at all", and which
            // the non-confined branch below applies to the same question --
            // behave exactly as the pre-W4 code did: the confined failure is
            // recorded (above, unconditionally), nothing is written, and
            // nothing is propagated.
            //
            // **This is the small end, and it is not the size cap the
            // do-not-retry list forbids.** That entry is about `if too_big {
            // return }` -- declining the *largest* cases, so the bigger the
            // collapse the less behaviour it gets, which is backwards. This
            // declines the *smallest*, alongside `MIN_FRACTURE_CELLS` and
            // `MIN_BODY_CELLS` doing the same thing a few lines down, and it
            // is the physically right answer rather than a budget: there is
            // no interior in a six-cell chip to crack. Anyone reading this as
            // the forbidden shape later: check which end it declines.
            if object == CrushedObject::SeveredPiece && region.len() < super::rigid::MIN_FRACTURE_CELLS {
                return Vec::new();
            }
            let crush = crush_in_place(world, &region, failure.at, object);
            if crush.fresh > 0 {
                return propagate;
            }
            // **A crush the leash refused is not a crush that had nothing
            // left to do**, and only the second may drop the wavefront. See
            // `Crush` for why they are indistinguishable by `fresh` alone
            // and what it costs to conflate them.
            if crush.leashed {
                return propagate;
            }
            return Vec::new();
        }
        // Rock does not part company along a drawn line, so the region's
        // rock-facing boundary is chewed up before anything is taken off
        // it. See `erode_failing_boundary` -- this is where the straight
        // 40-cell section cut, the razor-flat bottom of a failing column
        // and the wedge hypotenuse all stop being straight.
        let region = erode_failing_boundary(world, region);
        // R3a: **bound the work, never the breakage.** One failure's
        // region can be the union of whole-column subtrees, or a detached
        // piece out to `load::MAX_REGION_CELLS` (20,000), and nothing
        // between there and here caps what a single tick hands to
        // `fracture`. Take the nearest slice now and leave the rest
        // standing for a tick; see `slice_failing_region` for why the
        // erosion above runs first and why this is not the size cap the
        // do-not-retry list forbids.
        let Sliced { slice: region, remainder } = slice_failing_region(region, failure.at);
        // The remainder's re-visit is *guaranteed*, and it does not go back
        // through the load model to get it. That was tried first -- reschedule
        // the remainder's face and let it re-fail -- and it is recorded here
        // as a dead end because it fails in two separate ways at once:
        //
        // - **It stalls.** Re-judging costs a full support walk per face
        //   cell, and `load::MAX_LOAD_CELLS_PER_FRAME` was already exhausted
        //   by the collapse's own fan-out: measured `budget 0` on every frame
        //   from the fifth onward, sites climbing past 3,000, and the
        //   miniature ligament still **1,132 cells short after 1,200
        //   frames** in dribbles of a dozen cells. A one-tick collapse had
        //   become a twenty-second one.
        // - **It re-asks a question already answered.** The whole region was
        //   judged and licensed as one failure. Making the half that waits
        //   re-argue its case means it can lose -- to `within_disturbance`
        //   at any `chain_reach` but SPREAD, to the 12x attached-bulk bonus
        //   the remainder still carries, to an exhausted budget -- and every
        //   one of those is rock left hanging in open air.
        //
        // So the remainder is *work in progress*, not a fresh question:
        // `World::staged_fractures` holds it and `advance_staged_fractures`
        // takes the next slice off it a few frames later. Pacing then bounds
        // work per frame and nothing else, which is the whole point.
        if !remainder.is_empty() {
            world.staged_fractures.push_back(StagedFracture { region: remainder, at: failure.at, next_frame: world.frame + STRUCTURAL_TICK_INTERVAL });
        }
        // M8: a failure big enough to read as a *chunk* leaves as coherent
        // falling pieces rather than dissolving into loose grains one cell
        // at a time. Falls back to the per-cell conversion when the region
        // is too small to be worth it.
        //
        // `failure.at`, not `(x, y)`: the cell that gave way is the ancestor
        // the chain walk found over its limit, which may be many cells from
        // the one this tick was checking. It is where the impulse belongs.
        // Everything in `region` is about to stop being rock, one way or
        // the other -- this is the damage, so this is where it is measured.
        record_damage_reach_over(world, &region);
        if !super::rigid::fracture_failing_region(world, &region, failure.at) {
            // Declined for being under `MIN_FRACTURE_CELLS`, so this region
            // becomes grit rather than pieces. Counted, because it is the
            // only place the dust outcome is decided and nothing measured
            // it -- see `FailureCounts::crumbled`.
            world.structural_failures.crumbled += 1;
            world.structural_failures.crumbled_cells += region.len() as u32;
            // The region was under `MIN_FRACTURE_CELLS`, so nothing here
            // will ever fly -- this whole branch is grit by construction,
            // and it is the branch that makes `promoted_cells: 0` next to
            // a large `unsupported` mean something specific rather than
            // "the counter is broken".
            let mut grit = 0usize;
            for &(fx, fy) in &region {
                grit += usize::from(break_free(world, fx, fy));
            }
            world.structural_failures.record_shattered(grit);
        }
        // The neighbours that were relying on this as a stepping stone need
        // to recompute too -- this is what turns a break into a *cascade*
        // rather than one isolated piece vanishing.
        // **Measured inert on the big-strike scene, and kept anyway.**
        // Switching this scheduling off entirely produced *bit-identical*
        // output on `preset=flat seed=24301 strike=12` -- 95 overloaded
        // and 75 unsupported failures either way, same regions, same
        // cells -- and moved an 18-run seed sweep's worst case not at all
        // (1,228 both ways). Every cell it schedules was already being
        // scheduled by the distance-relaxation wavefront above, which is
        // the *actual* route a collapse propagates by. Kept because it is
        // the correct thing to do for a disturbance the relaxation does
        // not reach, and costs nothing where the relaxation does; recorded
        // because anyone hunting the chaining will come here first, as
        // this session did, and it is not here.
        let mut next = propagate;
        next.extend(schedule_solid_neighbours(world, x, y));
        for &(fx, fy) in &region {
            next.extend(schedule_solid_neighbours(world, fx, fy));
        }
        return next;
    }

    // Judged along its whole support chain, and holding. If the distance is
    // still moving, keep the relaxation going; otherwise stop being
    // actively rescheduled and let whatever disturbs this cell next
    // schedule it again.
    if moved {
        return propagate;
    }
    // Held up by nothing but the loose stuff underneath, and that can go
    // away silently -- keep asking, slowly. See `GROUNDED_RECHECK_INTERVAL`
    // for the trace this comes from and for why the trigger is not at the
    // grain that leaves.
    if grounded_root {
        return vec![ActiveSite {
            x,
            y,
            kind: ActiveKind::StructuralCheck,
            next_frame: world.frame + GROUNDED_RECHECK_INTERVAL,
        }];
    }
    //
    // Scheduling the parent here instead was tried and reverted. It fixed
    // the same ordering bug `failing_along_support_chain` now fixes, but
    // every settling cell raised a fresh check on its parent, which raised
    // one on *its* parent, faster than the queue drained: measured on
    // `scene=capped` at 26 pending sites climbing to 4,064 with frame cost
    // tracking it from 2.5 ms to 3,160 ms. Walking the chain inside this
    // tick does the same work without putting anything new in the queue.
    Vec::new()
}

/// Structural check for an organism-owned cell (`cell.organism_id() != 0`)
/// — the branch `tick` routes to instead of the aux-cached relaxation
/// above, whose cache `aux` no longer holds once it's carrying a cell-type
/// tag and resource scalar instead (`Reports/organism-substrate-
/// design.md` §2/§5).
///
/// **The distance is cached, just not in `aux`.** It lives on the organism
/// sidecar as `OrganismCell::support` and is recomputed from the anchors
/// outward once per organism tick by `plant::anchor_support`, so this
/// function is a field read and a comparison. It used to run a fresh
/// bounded BFS outward from `(x, y)` on every single check — see
/// `OrganismCell::support` for why that was wrong twice over, and why the
/// fix was the *direction* of the search rather than its budget.
///
/// A no-op for a material with no finite span (moss's own, still — this
/// only actually does anything for tree.ron's wood and rootwood so far).
fn organism_structural_tick(world: &mut World, x: i32, y: i32, cell: Cell) -> Vec<ActiveSite> {
    let material = world.materials.get(cell.material);
    // **`max_cantilever_reach`, not `max_unsupported_span`.** The inert
    // path reinterprets that field as a capacity (`span^2 / 2`, see
    // `load::capacity`), and the two are not on the same scale -- see
    // `MaterialDef::max_cantilever_reach` for the measurement that forced
    // them apart.
    //
    // Scaled by the individual's wood density -- the strength half of
    // `WOOD_DENSITY_ALLELES` (the price half is on `Grow.cost`): dense
    // wood holds a longer reach under more load before snapping to
    // deadwood, cheap wood grows faster and loses more of itself.
    // **The no-finite-span sentinel is read off the *material*, never off
    // the scaled span.** `u16::MAX` is leaf.ron's (and moss's) opt-out
    // from the cantilever rule entirely, and it is a sentinel value, not a
    // large number: 65535 x 0.75 is 49151, which is not the sentinel, so
    // scaling first silently enrolled every pioneer-allele plant's foliage
    // in a rule the opt-out exists to keep it out of. Latent today only
    // because 49151 still dwarfs any real support distance -- and "latent"
    // is what the leaf span was before checks started firing and took the
    // stand from 31,731 cells to 7,171.
    let unbounded = material.max_cantilever_reach == u16::MAX;
    let density = world.organism(cell.organism_id()).map_or(1.0, |s| organism::wood_density(&s.alleles));
    let max_span = ((material.max_cantilever_reach as f32 * density) as u32).min(u16::MAX as u32) as u16;
    let organism_id = cell.organism_id();
    // **Load shortens the span a branch can hold.** `PLAN.md` treats "too
    // much weight breaks a branch" as already in scope, and it was only
    // half-built: `organism_is_supported` measured *distance from an
    // anchor* and nothing else, so a branch with a metre of sand piled on
    // it broke at exactly the same span as a bare one.
    //
    // `Reports/plant-substrate-v2-design.md` §6c, and named as an analogue
    // rather than dressed up as physics: real allowable cantilever span
    // does fall with load, and that ordering is the only property being
    // borrowed. This is not a beam-deflection calculation and must not be
    // described as one. It is a weighted local rule -- which
    // `design-philosophy.md` §2b explicitly permits -- whose *outcome*
    // (which branch breaks, and when) is emergent from what the player
    // actually piled on it.
    let effective_span = max_span.saturating_sub(supported_load(world, x, y, organism_id) / LOAD_PER_SPAN_UNIT);
    // **A field read, where this used to be a fresh BFS per check.**
    // `plant::anchor_support` computes every cell's weighted distance to
    // its organism's anchors once per organism tick, from the anchors
    // outward; see `OrganismCell::support` for the two defects the old
    // outward-from-here search had. `0` for a cell with no sidecar: an
    // organism that has not been walked yet must defer, never fail, because
    // the action this decides is destructive.
    let support = world.organism_cell(x, y).map_or(0, |c| c.support);
    // Two questions off one number, and they are genuinely different.
    // `u16::MAX` is **attachment** -- nothing this cell connects to reaches
    // the ground, so the piece it is part of has come off, however short
    // its own distance would otherwise have been. Anything else is
    // **cantilever** -- how far out along its own load path it sits, against
    // what the material can hold with the load currently on it.
    let detached = support == u16::MAX;
    // **Attachment applies to every organism material; cantilever only to
    // the ones that carry load.** Keeping the two apart is what lets
    // `leaf.ron` opt out of the span rule (`u16::MAX`) and still fall when
    // the twig it hangs on is cut -- the early return this replaced tested
    // the span first and so exempted foliage from *both*.
    //
    // A leaf must opt out, and the reason is a measurement: `leaf.ron`'s
    // span was **1**, written to mean "a leaf holds up nothing but itself"
    // under the old rule, where the number was hops-to-nearest-ground and
    // nothing ever scheduled a check on a leaf anyway. Under the anchor
    // pass the number is reach along the load path, which for any leaf in a
    // crown is tens of cells, so the first run with checks actually firing
    // destroyed every leaf in the stand -- median leaves per tree 1,376 to
    // **1**, and with the income gone the stand fell from 31,731 cells to
    // 7,171. `leaf.ron`'s own doc already says a leaf must not be a load
    // path; this is that sentence enforced instead of asserted.
    let over_span = !detached && !unbounded && support > effective_span;
    if !detached && !over_span {
        // Supported and, unlike the aux-cached path, always an exact
        // answer rather than one still converging -- nothing more to do
        // until something else disturbs this organism's own structure.
        return Vec::new();
    }
    if material.breaks_into.is_none() {
        // Same "not actually participating" framing the aux-cached path
        // above uses for the identical case.
        return Vec::new();
    }
    // **The leash applies to trees too.** This path never consulted
    // `within_disturbance` at all, so `chain_reach` did not exist for
    // organisms at any setting *including NONE* -- measured on the
    // nine-charge harness, which recorded damage landing 35 cells from a
    // live disturbance at NONE while reporting zero rock failures, and the
    // organism path is the only thing that can produce that.
    //
    // At SPREAD the gate is a constant `true` and this changes nothing. At
    // LOCAL/TIGHT/NONE it stops a blast amputating a stand of trees fifty
    // cells away, which no setting currently prevents. `Vec::new()` on a
    // refusal for the same reason the rock path uses it: a refused cell
    // stops being rescheduled and is looked at again when something
    // disturbs it, which is exactly what would license it.
    if !world.within_disturbance(x, y) {
        return Vec::new();
    }
    // Deliberately dropped rather than recorded: this is a limb losing its
    // anchor and becoming deadwood, not rock coming apart, and
    // `shattered_cells` is paired with `promoted_cells` to describe the
    // latter. See `break_free`'s own doc.
    // **Detached tissue comes off as a piece; over-span tissue snaps one
    // cell at a time.** The two failure modes above are genuinely different
    // events and this is where that stops being a distinction without a
    // difference.
    //
    // `detached` means nothing this cell connects to reaches the ground --
    // the piece it belongs to has *already* come off, and what is left is
    // to bring it down. So the object the rule evaluates is the piece, not
    // the cell (`CLAUDE.md`, "which object does this rule evaluate"), and
    // handing one cell at a time to `break_free` is what turned a felled
    // tree into a cone of sawdust: 2,648 cells severed, 45 of them (1.7%)
    // surviving as anything a player could see move.
    //
    // `over_span` is a branch bending past what it can hold. That really is
    // one cell giving way, and the outboard tissue it was carrying becomes
    // detached on the next `plant::anchor_support` pass -- at which point it
    // arrives here through the branch above and comes down as a piece. The
    // graded outcome falls out of the chain rather than being authored into
    // it.
    if detached {
        let region = detached_organism_piece(world, x, y, organism_id);
        record_damage_reach_over(world, &region);
        let (severed, as_pieces) = rigid::fell_severed_tissue(world, &region, (x, y));
        // Counted, where the rock path's grit deliberately is not (see
        // `FailureCounts::shattered_cells`). This is felling's only "did it
        // fire" number: an image cannot tell a crown that came down from a
        // crown that was never asked, and the first `scene=fell` run had a
        // severed trunk, a growing canopy and zero in every other counter.
        world.structural_failures.record_severed_organism(severed);
        world.structural_failures.record_severed_pieces(as_pieces);
        // Every cell of the piece fans out, not just the one that was
        // checked: the piece may have been holding up tissue that is still
        // attached elsewhere, and that tissue is nowhere near `(x, y)`.
        return region.iter().flat_map(|&(rx, ry)| schedule_organism_neighbours(world, rx, ry, organism_id)).collect();
    }
    record_damage_reach_over(world, &[(x, y)]);
    if break_free(world, x, y) {
        world.structural_failures.record_severed_organism(1);
    }
    schedule_organism_neighbours(world, x, y, organism_id)
}

/// How many cells of piled-up load cost one cell of allowable span.
///
/// Four, so `wood`'s span of 8 survives a light dusting and a genuinely
/// buried branch loses most of its reach. Untuned beyond that ordering,
/// and a first-class candidate for the economy pass — the interesting
/// question is whether burying a limb in sand visibly snaps it, which is a
/// look judgement rather than a derivable number.
const LOAD_PER_SPAN_UNIT: u16 = 4;

/// How far out a cell looks for material resting on this organism.
///
/// Small on purpose. This runs per reactive structural check, and the
/// question it answers is local ("is something sitting on me"), not global
/// ("what is the total mass of the pile above"). A wider search would cost
/// more and measure something the rule is not about.
const LOAD_SEARCH_RADIUS: i32 = 3;

/// Non-organism material resting directly on this organism's own cells,
/// within a short radius — the weight term for `effective_span` above.
///
/// Counts only cells *above* organism tissue, since that is what "resting
/// on" means and a grain beside a branch is not load. `Powder` and `Liquid`
/// only: a `Solid` neighbour is a wall the branch might be growing against
/// rather than a burden, and counting it would make a tree weaker for
/// having grown near rock.
fn supported_load(world: &World, x: i32, y: i32, organism_id: u16) -> u16 {
    let mut load = 0u16;
    for dx in -LOAD_SEARCH_RADIUS..=LOAD_SEARCH_RADIUS {
        for dy in -LOAD_SEARCH_RADIUS..=LOAD_SEARCH_RADIUS {
            let (cx, cy) = (x + dx, y + dy);
            // Is this one of ours, and is something sitting on top of it?
            if world.get(cx, cy).organism_id() != organism_id {
                continue;
            }
            let above = world.get(cx, cy - 1);
            if above.organism_id() == organism_id {
                continue; // our own tissue is not a load on itself
            }
            if matches!(world.materials.kind(above.material), MaterialKind::Powder | MaterialKind::Liquid) {
                load = load.saturating_add(1);
            }
        }
    }
    load
}

/// Every cell of the **piece** that `(x, y)` belongs to: the 8-connected
/// run of same-organism tissue that has also lost its anchor.
///
/// # Eight, and it is the single highest-leverage line in the package
///
/// `Grow` places organism cells at eight neighbours, so a crown -- which is
/// mostly diagonal twigs -- is cut at every diagonal by anything that reads
/// it back at four. That is `CLAUDE.md`'s "a traversal must use the same
/// neighbourhood the writer used", and `plant::anchor_support`, which
/// produces the very `support` value this filters on, already walks eight.
/// A four-connected walk here would disagree with the pass that decided
/// which cells are detached in the first place.
///
/// # Why detachment is a membership test and not just connectivity
///
/// A crown half of which is still held has not come off. Taking the whole
/// connected run would rip attached tissue out with it -- the amputation
/// failure in a new costume. `support == u16::MAX` is `anchor_support`'s own
/// verdict that nothing this cell reaches touches the ground, and a cell
/// with no sidecar entry at all reads as `0`, which is the deferral the
/// caller's own doc relies on: an organism that has not been walked yet must
/// defer, never fail, because the action this decides is destructive.
///
/// # Determinism
///
/// The output is sorted, and the frontier is a `VecDeque` walked in a fixed
/// neighbour order; the `HashSet` is only ever asked `contains`/`insert` and
/// is never iterated. `Reports/physical-trees-design-2026-08-23.md` §2a
/// names this as the exact shape of issue #7's live violation, so it is
/// called out rather than left to be read off the code.
///
/// Unbounded, deliberately. A cap here would be a cap on *whether* a big
/// piece comes off, which is the mistake `rigid::fracture_failing_region`'s
/// own doc records having shipped once already.
fn detached_organism_piece(world: &World, x: i32, y: i32, organism_id: u16) -> Vec<(i32, i32)> {
    let mut seen: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::from([(x, y)]);
    let mut queue = std::collections::VecDeque::from([(x, y)]);
    let mut out = Vec::new();
    while let Some((cx, cy)) = queue.pop_front() {
        out.push((cx, cy));
        for (dx, dy) in NEIGHBOURS_8 {
            let next = (cx + dx, cy + dy);
            if seen.contains(&next) {
                continue;
            }
            let cell = world.get(next.0, next.1);
            if cell.organism_id() != organism_id || world.materials.kind(cell.material) != MaterialKind::Plant {
                continue;
            }
            // Only tissue that has *also* come off. See the doc above: a
            // still-anchored neighbour is not part of this piece, and a
            // cell the organism pass has not reached reads as supported and
            // is left for a later check rather than destroyed now.
            if world.organism_cell(next.0, next.1).map_or(0, |c| c.support) != u16::MAX {
                continue;
            }
            seen.insert(next);
            queue.push_back(next);
        }
    }
    out.sort_unstable();
    out
}

/// The organism-owned mirror of `schedule_solid_neighbours` — an organism
/// cell that just broke free might have been the only thing keeping its
/// own same-organism `Plant` neighbours anchored, so they need
/// re-evaluating too. This is what turns one broken branch into a real
/// cascade for a tree, the same way the aux-cached path already does for
/// stone.
fn schedule_organism_neighbours(world: &World, x: i32, y: i32, organism_id: u16) -> Vec<ActiveSite> {
    // **Eight, because `Grow` places at eight.** This walked four, so a
    // cascade through a crown -- which is mostly diagonal twigs -- skipped
    // every diagonally-attached neighbour and the chain simply stopped at
    // the first corner. `Reports/physical-trees-design-2026-08-23.md` §9.2
    // filed it beside `take_fragment`'s identical defect one function away;
    // this is the same rule as that one and as `plant::anchor_support`,
    // which already walks eight and produces the very `support` value this
    // fan-out exists to get recomputed.
    //
    // Guarded by `a_diagonally_attached_twig_is_rescheduled_by_its_
    // neighbour`, which fails against `NEIGHBOURS_4`.
    NEIGHBOURS_8
        .iter()
        .filter_map(|&(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            let neighbour = world.get(nx, ny);
            if neighbour.organism_id() == organism_id && world.materials.kind(neighbour.material) == MaterialKind::Plant {
                Some(reschedule(world, nx, ny))
            } else {
                None
            }
        })
        .collect()
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

/// Whether a failing region has anywhere to go: does any cell of it touch
/// air that is not part of the region itself?
///
/// # Which object this evaluates, and why it must be the region
///
/// A *piece*, not a cell and not a section. That distinction has already
/// cost this project real rework twice -- a bearing rule correct for a
/// piece resting on loose ground was applied per cell and took a slab
/// apart one knife-edge footing at a time -- so it is worth stating: the
/// question "is there room for this to move" is only defined for the whole
/// thing that would move. Asked per cell it is nearly always yes, because
/// every cell of a region touches its own neighbours' vacancies as they
/// go; asked per region it is the physical question.
///
/// 8 neighbours, not 4. A region that can only escape diagonally can still
/// escape, and more to the point the writer this has to agree with --
/// `rigid::fracture`, which promotes and moves pieces -- works in 8, so
/// asking in 4 would call a piece confined that the mover is about to
/// displace anyway.
///
/// Air specifically, by the raw material test: `Cell::is_empty()` is
/// managed-aware and would count a promoted liquid body's container cells
/// as occupied, which is the wrong answer to "could rock move into here".
///
/// # Water is somewhere to go, and reading it as rock cost the owner a bug
///
/// **A liquid the piece outweighs counts as a free face.** Air-only was the
/// original rule and it is wrong in exactly one direction: a piece
/// submerged in water has no empty neighbour at all, so it read as
/// *confined* -- "a pocket already cut free on every side, wedged in a hole
/// its own shape", per the caller's own words -- and the caller's answer to
/// a confined `Unsupported` failure is to record it and **leave it exactly
/// where it is, with nothing rescheduled**. Correct for rock in the middle
/// of a mountain. For a quench crust floating mid-lake it is the whole of
/// the owner's report that stone "freezes in place" instead of sinking.
///
/// `CLAUDE.md`'s "when a rule must tell apart two things that can look
/// identical, state the difference as data", one more time: buried in rock
/// and submerged in water are the same arrangement of not-empty cells, and
/// the difference is a density the materials already carry.
///
/// The **piece's** mean density against the liquid's, on the same "which
/// object does this rule evaluate" argument this function's own doc makes
/// above -- and matching `rigid::clear_or_displaceable`, which is the mover
/// this has to agree with. A slab with one cell of ice in it still sinks; a
/// floe does not.
///
/// Powder is deliberately *not* included even though `rigid::displace` can
/// shove it. A piece buried in sand has to push a granular pile out of the
/// way rather than swap with a fluid, and whether that should count is a
/// separate question with its own scenes; nothing has reported it.
///
/// # A gas is somewhere to go too, and leaving it out stranded rock in mid-air
///
/// This rule read `EMPTY` and, later, a lighter `Liquid`, and silently said
/// **no** to every `Gas`. Steam is not air by the material test, so a stone
/// cell in a cloud of it was "wedged in a hole its own shape" -- and the
/// caller's answer to a confined `Unsupported` failure is to record it,
/// leave it standing, and *reschedule nothing*, so the cell is never asked
/// again however the world changes around it.
///
/// Measured, on `scene=lavadrop`: the quench mints a stone cell at frame
/// 25 whose eight neighbours are five lava and three steam, it fails as
/// `Unsupported` with a one-cell region, `confined` comes back true, and it
/// is dropped from the scheduler for good. Sixty frames later the lava and
/// the steam have gone and it is a lone stone pixel hanging over open
/// water, which is what the owner saw. Poking those positions by hand
/// (`filmstrip`'s `poke=`) drops them immediately, which is what proves the
/// cell was never re-asked rather than asked and refused.
///
/// Rock falls through steam exactly as it falls through air, so a gas the
/// piece outweighs is an open face. The density test is kept rather than
/// waved through -- every gas in the game today is orders of magnitude
/// lighter than any solid, and a rule that says *why* survives one that
/// isn't.
fn face_is_open(world: &World, piece_density: f32, nx: i32, ny: i32) -> bool {
    let material = world.get(nx, ny).material;
    if material == super::material::EMPTY {
        return true;
    }
    world.materials.kind(material) == MaterialKind::Gas && piece_density > world.materials.density(material)
}

/// The original rule, plus gases: is any neighbour of the region open space
/// the piece could move into *without* counting a liquid? Split out so the
/// buoyant case above can reach it without duplicating it -- what `floats`
/// must not do is read water as somewhere to go, and a gas is not water.
fn region_touches_air(
    world: &World,
    region: &[(i32, i32)],
    cells: &std::collections::HashSet<(i32, i32)>,
    piece_density: f32,
) -> bool {
    region.iter().any(|&(x, y)| {
        NEIGHBOURS_8.iter().any(|&(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            !cells.contains(&(nx, ny)) && world.in_bounds(nx, ny) && face_is_open(world, piece_density, nx, ny)
        })
    })
}

/// # Gas counts as air too, and both branches found that independently
///
/// `face_is_open` above admits a gas the piece outweighs. That arm was
/// built twice, once here and once on the explosion branch, from two
/// different symptoms -- which is worth recording, because it means the
/// rule has two unrelated witnesses rather than one.
///
/// The explosion side's witness: `explosion::Tuning::smoke_fraction`
/// backfills 18% of a fresh crater with `SMOKE`, so the hole a blasted
/// chunk is meant to drop into is the densest gas in the world -- and the
/// emptier the blast made it, the more smoke it left. Testing `EMPTY` alone
/// made such a piece **confined**, which sends it down the crush branch to
/// be cracked where it stands instead of allowed to fall. Reported from
/// play as *"chunks of rock that seem fully cracked all the way around stay
/// put and don't fall into the leftover hole/crater"*.
///
/// `explosion.rs` had already had to learn the same fact once, at
/// `an_explosion_clears_a_crater`: *"Not `is_empty()`: `smoke_fraction`
/// backfills part of the crater with `SMOKE`, so the epicentre is
/// legitimately allowed to hold a gas cell afterwards."* Same fact, a third
/// consumer.
///
/// The merge kept `origin/main`'s spelling of the rule, which gates the gas
/// on density and handles a lighter liquid as well; this branch's
/// `is_open_space` helper was deleted rather than left orphaned.
fn region_has_free_face(world: &World, region: &[(i32, i32)]) -> bool {
    let cells: std::collections::HashSet<(i32, i32)> = region.iter().copied().collect();
    let piece_density = if region.is_empty() {
        0.0
    } else {
        region.iter().map(|&(x, y)| world.materials.density(world.get(x, y).material)).sum::<f32>() / region.len() as f32
    };
    // **`floats` first, density second, and the order was decided by
    // measurement.** Density alone took `scene=coldsnap` from 1 overload
    // failure to 23 plus 12 unsupported, against an acceptance bar of zero
    // *unconfined* failures whose entire job is that nothing on that scene
    // is dismantled. Reverting this one arm and changing nothing else put
    // it back to 1/0/1-confined, identical to the pre-change baseline, so
    // the attribution is not in doubt.
    //
    // What density gets wrong there is a **mixed** piece: a region that is
    // mostly ice but has picked up a stone cell averages over 1.0, so the
    // water beside it read as somewhere to go and the sheet was taken apart
    // instead of cracking where it stood. `ice.ron` says why in advance --
    // "the floating below is a flag rather than a density test" -- and
    // `is_resting_on_ground` already asks the flag for the same question.
    // Asking density here made this a second, disagreeing reader of it.
    //
    // Any floating cell makes the whole piece float, which is the
    // conservative direction: it can only ever leave a piece confined that
    // density would have dismantled.
    if region.iter().any(|&(x, y)| world.materials.get(world.get(x, y).material).floats) {
        return region_touches_air(world, region, &cells, piece_density);
    }
    region.iter().any(|&(x, y)| {
        NEIGHBOURS_8.iter().any(|&(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            if cells.contains(&(nx, ny)) || !world.in_bounds(nx, ny) {
                return false;
            }
            if face_is_open(world, piece_density, nx, ny) {
                return true;
            }
            let neighbour = world.get(nx, ny);
            world.materials.kind(neighbour.material) == MaterialKind::Liquid
                && piece_density > world.materials.density(neighbour.material)
        })
    })
}

/// How far a confined region is from the nearest air, in cells, capped at
/// `BURIAL_PROBE_CAP`.
///
/// Distinguishes the two things a confined failure can be, which the
/// boolean cannot and no contact sheet can either: rock one row under a
/// surface that is itself coming apart, and rock in the middle of a
/// mountain. Only the second is the artifact the owner reports. Measured
/// as the *smallest* clearance any cell of the piece has, because that is
/// the nearest place it could move to if it moved at all.
///
/// Only ever called for a region already known to be confined, which is
/// tens of events in a whole run, so the bounded ring search below costs
/// nothing measurable. It is emphatically not something to call per cell
/// per frame.
fn burial_depth(world: &World, region: &[(i32, i32)]) -> u32 {
    let mut best = BURIAL_PROBE_CAP;
    for &(x, y) in region {
        // Expanding square rings: the first ring containing air gives this
        // cell's clearance, and a cell cannot beat a ring the search has
        // already passed, so it stops as soon as it cannot improve `best`.
        for r in 1..best {
            let ring = (-(r as i32)..=(r as i32)).flat_map(|d| {
                let r = r as i32;
                [(x + d, y - r), (x + d, y + r), (x - r, y + d), (x + r, y + d)]
            });
            if ring.into_iter().any(|(nx, ny)| world.in_bounds(nx, ny) && world.get(nx, ny).material == super::material::EMPTY) {
                best = r;
                break;
            }
        }
    }
    best
}

/// How far the burial probe looks before calling a piece "deep". Past this
/// the answer stops mattering: anything 24 cells from the nearest air is
/// unambiguously inside a massif, and the search is quadratic in it.
const BURIAL_PROBE_CAP: u32 = 24;

/// **Which object a confined crush is breaking.** Not which caller asked --
/// the two callers are the two failure modes, but what decides the star's
/// size is the *thing* being cracked, and these are two different things.
///
/// This split exists because the star was sized for one of them and then
/// handed the other, which is `CLAUDE.md`'s **which object does this rule
/// evaluate -- a cell, a section, or a whole piece?** for the third time on
/// this branch. `crush_in_place`'s floors (`CRACK_MIN_LENGTH` 10,
/// `CRACK_FORKS_BASE` 4) were tuned against an over-capacity section and are
/// right for it. Give them to a one-cell island and a chip fires three
/// ten-cell rays and four forks into the rock around it, severing more
/// islands, which fire again -- 84,051 cracked cells and a hillside that
/// went dark from the crater outward. See `tick`'s confined branch for the
/// full measurement.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CrushedObject {
    /// An over-capacity **section**: rock still attached to the world,
    /// carrying more moment than it can bear. It is large by construction
    /// (that is what put it over capacity) and it is *whole*, so there is
    /// interior to break into blocks and a floor under the damage is right
    /// -- a failure this big must not produce cracks too faint to see.
    ///
    /// Keeps the sizing byte for byte: this half was never what regressed.
    Section,
    /// A **severed piece**: a chunk the fissures already cut out of solid
    /// rock, judged unsupported because nothing holds it any more. Its size
    /// is whatever the cracks happened to enclose, and on a cascading world
    /// that is routinely a single cell (mean failing region size 1.1 on the
    /// deep charge). So the damage must genuinely scale from zero: no floor
    /// on the ray length, no fork base. A piece with nothing inside it draws
    /// nothing.
    SeveredPiece,
}

impl CrushedObject {
    /// The floor under how far a section's reveal reaches into the rock
    /// around it. `CRACK_MIN_LENGTH` exists so a small *section* still
    /// produces damage the eye can find -- and it is now also the floor
    /// that makes the rule able to *demonstrate itself*: stone's grain is
    /// 13 cells, so a disc much under ten cells can sit entirely inside one
    /// domain and contain no joint at all.
    ///
    /// A severed piece has no such floor and does not take this path at
    /// all: it reveals only the joints inside itself, so a piece smaller
    /// than the grain writes nothing because there is nothing in it to
    /// write, not because a constant said so. The variant is kept in the
    /// match rather than folded away so the two objects stay visibly two
    /// rules -- see `reveal_joints`.
    fn min_joint_reach(self) -> usize {
        match self {
            CrushedObject::Section => CRACK_MIN_LENGTH,
            CrushedObject::SeveredPiece => 0,
        }
    }
}

/// A confined region fails **in place**: it cracks into blocks and does not
/// go anywhere.
///
/// # The rule, and the one it must not be mistaken for
///
/// Stated by the owner: *"it is stone in the middle of a mountain falling
/// in on itself. It doesn't look right. If it happens in a cave and causes
/// a cave in, or a cliff side falls over, that makes sense — but in solid
/// rock you should just have cracks that propagate and maybe break rock
/// into small pieces that for the most part stay where they are."* Rock
/// confined on every side cannot displace, because there is nowhere for it
/// to move. It fractures where it stands.
///
/// **This is not "confinement as an anchor", which is retired and must not
/// come back** (`Reports/load-model-handoff.md` §6.1). That model inferred
/// *support* from burial, which made thick rock immune to failing at all
/// and is one word away from this. The difference is which end of the
/// pipeline it sits at: confinement decides what a failure **produces**,
/// never whether it happens. A buried cell is still judged, still goes
/// over capacity, and is still recorded as failing here -- it simply
/// cannot fall into rock that is already there. Keep that distinction
/// explicit in anything built on this.
///
/// # Fissures, not graph paper
///
/// The first version cracked on a fixed lattice in world coordinates --
/// every sixth column and row, with a ligament left in each run so the
/// blocks stayed keyed together. The arithmetic was fine and **it looked
/// absurd**: `preset=rolling seed=1 strike=20` drew a perfectly regular
/// 6x6 crosshatch spreading across the hillside, reading as wireframe mesh
/// laid over the rock rather than as broken stone. Nothing in a metric
/// said so; one contact sheet said so immediately.
///
/// So the cracks radiate instead, from the cell that gave way and from a
/// few further origins spread through the piece, which is `score_cracks`'
/// shape -- and that already looks right on screen, because it is what
/// draws the star of fissures round a blow. Directions are keyed on the
/// *site* through `rng::jitter`, never drawn from `world.rng`: a hash of
/// position costs no RNG draw (so a replayed input sequence cannot diverge
/// here), it makes the pattern a property of the rock rather than of the
/// blow, and it means a second failure over the same rock deepens the
/// fissures it already has instead of scribbling new ones beside them.
///
/// Deliberately *not* `rigid::score_cracks` itself, though it is the same
/// idea: that function unbraces the rock either side of every fissure and
/// schedules a structural check at each one. Both are right for a blow and
/// wrong here. Unbracing confined rock tells the load model the inside of
/// a mountain has come free, and rescheduling is what turns a crush into a
/// treadmill.
///
/// # Why not crack every edge
///
/// Because the outcome has to be a *distribution*, not a dissolve. The
/// all-or-nothing failure -- one coherent body or a uniform powder -- is
/// the artifact this project has already rewritten twice, and cracking
/// every edge inside the region would be the powder end of it again: it
/// would disconnect every cell from every other, cut the support paths
/// that carry a mountain, and leave a mass of rock that is solid on screen
/// and structurally sand.
///
/// # Why this needs no "already crushed" state
///
/// A crack is a bit, and setting a set bit changes nothing, so re-running
/// this over the same rock is idempotent -- the second confined failure at
/// the same place draws the same fissures and manufactures nothing. That
/// matters because there is no spare flag on `Cell` (all eight are taken)
/// and because the churn is then only scheduler cost, never accumulating
/// visual damage.
///
/// It is also, by construction, the lever that killed the dig cascade:
/// a failure that creates no loose material cannot undermine its
/// neighbours, which is the positive feedback `Reports/next-session-
/// handoff.md` §1b traces every cascade to.
fn crush_in_place(world: &mut World, region: &[(i32, i32)], at: (i32, i32), object: CrushedObject) -> Crush {
    // **The damage still scales with the piece, and it is still two
    // objects and two rules** -- what changed is what gets drawn. See
    // `reveal_joints` for the mechanism and `CrushedObject` for the split.
    let crush = reveal_joints(world, region, at, object);
    // Counted here rather than inside the reveal, so `FailureCounts`
    // keeps meaning exactly what it did: damage a *confined failure*
    // wrote where it stood. A blast's own joints are the blast report's
    // business (`BlastReport::joints_scored`) and must not land in the
    // structural-failure census.
    //
    // It counts **edges** now rather than visited cells, the same unit
    // `explosion::sever` returns as `fresh`. The two are within a small
    // factor of each other (a cell owns two edges), and what the counter
    // is for is unchanged: `crush_in_place(..) > 0` is the treadmill
    // guard, and "fresh" has to keep meaning "damage that was not already
    // there" or the guard stops guarding.
    world.structural_failures.crushed_cells += crush.fresh;
    crush
}

/// What a confined crush did, and -- when it did nothing -- which of the two
/// opposite reasons applied.
///
/// # Why "wrote nothing" is not one outcome
///
/// `tick`'s confined branch returns `Vec::new()` when the crush writes
/// nothing, and that is deliberate: a crush over rock it has *already*
/// cracked writes nothing, has nothing to propagate, and re-queueing it is
/// the re-crush treadmill measured at 1,120 confined failures per 400 frames
/// on dead-still material.
///
/// But `reveal_joints`' `Section` arm clamps its disc by
/// `World::licence_headroom`, and that clamp defeats
/// `CrushedObject::min_joint_reach`'s floor of ten. Stone's grain is
/// thirteen cells, so a disc shrunk under that floor can sit inside a single
/// domain, contain no joint at all, and write nothing -- for a reason that
/// has nothing to do with the rock already being cracked. Treating that as
/// idempotence drops `propagate`, and `propagate` carries the distance
/// wavefront that the block at the top of `tick` says **every** early return
/// must carry. That is the exact artifact the empty-region return was
/// changed to fix, reintroduced two hundred lines further down, and it is an
/// annulus around every licence boundary rather than a knife edge at it.
///
/// Found by adversarial review rather than by a sweep, and the reason no
/// sweep could see it is worth recording: every statistic
/// `scripts/seedsweep.sh` gates is a *damage* count, and this artifact
/// **reduces** damage, so it reads as the leash working.
///
/// SPREAD is untouched -- `licence_headroom` is `None` there, so `leashed`
/// is never set and the arithmetic is bit-identical.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Crush {
    /// Newly severed edges. `fresh == 0 && !leashed` is the idempotent case
    /// the treadmill guard exists for.
    fresh: u32,
    /// The licence shrank the disc below what the region asked for. The rock
    /// had more to give and the leash is what stopped it.
    leashed: bool,
}

/// A confined failure **reveals the rock's own joints** inside the object
/// that failed, instead of walking a star of wandering fissures across it.
///
/// # What this replaces, and why the walker had to go from here too
///
/// `crush_in_place` used to call `walk_fissures`: three primaries at
/// `CRACK_PRIMARIES`, `CrackStyle::Wander` at 0.9 rad *per cell*, a fork
/// pool of `CRACK_FORKS_BASE` plus one per `CRACK_CELLS_PER_FORK`, drawn to
/// completion in the frame the failure happened. Every one of those three
/// numbers was tuned, and the shape they drew is the one the owner rejected
/// four times: *"it shouldn't look like a scribble"*.
///
/// The blast stopped drawing it at `d5cb19a` and the crush did not, which
/// left the walker running on the **only** path that still ran by default
/// -- and it is worse here than it ever was under a blast, because a crush
/// fires off the relaxation wavefront hundreds of frames after the bang.
/// The owner's review of the ten animated blasts names both halves of that:
/// *"thick lines that appear later in the blast"* (the walker marks an edge
/// **and its mirror**, so it darkens two adjacent cells, where a joint
/// darkens one) and *"other random movements and small random collapses
/// that keep happening"*. Measured on `blast=300,45,20,180,60`, seed 1,
/// read at frame 1,200 against the shipped `confine=0` control: 610 cracked
/// cells in the world against the control's 145, so the crush wrote **76%
/// of every crack cell there is**; 2,365 unsupported judgements against 29;
/// and it **moved less material than not running at all** (2,226 cells
/// promoted against 2,814).
///
/// # What the swap costs, read as a trajectory rather than an instant
///
/// Promoted cells at frame 1,200 fall, 2,226 to 1,213, and that number on
/// its own reads as lost breakage. It is not: it is lost *lateness*.
/// Sampled every 200 frames on the same charge (the bang is at frame 60),
/// cells promoted go
///
/// ```text
///   frame       100    300    500    700    900   1100   1300
///   walker      503    528    528  1,745  2,184  2,226  2,710   ...still climbing
///   fabric      503    693    830  1,155  1,213  1,213  1,213   settled
/// ```
///
/// **Identical at the bang and still rising at 1,300 on the walker.** What
/// the fabric removed is a collapse trickle that never terminates, which is
/// the owner's *"small random collapses that keep happening"* as a number,
/// and cracked cells tell the same story over the same run (233 -> 693 for
/// the walker, 197 -> 379 settling by frame 700 for the fabric). Anyone
/// reading a single late tile and concluding the change costs breakage
/// should sample the curve first: `CLAUDE.md`, assert the property rather
/// than two instants fitted to one trajectory.
///
/// # The cost this does carry, and why no setting of it is cheaper
///
/// `scripts/seedsweep.sh strike=12`, 24 runs, order statistics, measured in
/// this session against `d5cb19a` on the same machine. A strike fires no
/// blast, so this is `Section`'s disc on its own with no fabric already in
/// the rock:
///
/// ```text
///                                  cells lost        rock destroyed
///                                  max     p90       max     p90
///   d5cb19a, the walked star       192     118     1,106     565
///   disc, density 0.9  (shipped)   521     174     1,345     852
///   disc capped at 20 rather than 55
///                                  562       -     2,034       -
///   density 0.5 rather than 0.9  1,229     580     2,308   1,815
///   `Section` bounded by the region
///                                  430     250     1,774   1,086
/// ```
///
/// **Every attempt to damp it made it worse, and the reason is not a
/// tuning error.** A region diced *completely* has no free face anywhere --
/// every block is wedged against its neighbours -- so it is judged confined
/// and stays exactly where it is. A region diced *partially* leaves pieces
/// with an open side, and those fall. So thinning the reveal moves material
/// off the hillside rather than saving it, and the shipped setting is the
/// most complete of the four rather than the gentlest. `CLAUDE.md`: when a
/// term resists tuning in both directions, the trade is structural. The
/// walked star is gentler still and is not available -- it is the pattern
/// the owner rejected four times.
///
/// The region-bounded variant is recorded rather than dismissed: it is
/// worse on rock destroyed and on `cells lost` p90, and on the blast
/// diagnostic it promotes 1,078 against 1,213, but it does hold the *max*
/// down. If the strike sweep's max is ever the number that matters more
/// than its p90, it is one line away.
///
/// # Why the fabric cannot run away here, where the star could
///
/// This is the property that makes the swap safe rather than merely
/// prettier, and it is worth stating plainly because W4's 84,051-cracked-
/// cell dissolve came from exactly the failure it forecloses. The joint set
/// is a **pure function of position** (`fracture_field`): two crushes at
/// two neighbouring sites reveal *the same edges*, so the second writes
/// nothing, returns zero, and `tick`'s guard stops it propagating. Total
/// damage in a patch of rock is bounded by the number of joints in that
/// patch however many failures pass through it. A walker had no such
/// bound: every new origin drew a new star, which is how one severed
/// island manufactured the next.
///
/// The activation draw is deliberately the same `joint_draw` the blast
/// uses, at a density chosen to match `Tuning::joint_density`, so a crush
/// inside a blast's own near field re-selects the boundaries the blast has
/// already severed and writes nothing at all. The noise the owner sees is
/// loudest exactly there, and this is why it goes quiet there first.
///
/// # `detach: false`, restated because this is the easy thing to get wrong
///
/// `explosion::sever` calls `detach_around_crack` and
/// `schedule_structural_check_around` on both cells of every edge it
/// parts. **This must not**, and neither may it open a seam. Unbracing
/// confined rock tells the load model the inside of a mountain has come
/// free; rescheduling is what turns a crush into a treadmill (1,120
/// confined failures per 400 frames with the world's material dead still);
/// and opening removes cells, which is the one thing rock with nowhere to
/// go must not do. Nothing below writes anything but a crack bit.
fn reveal_joints(world: &mut World, region: &[(i32, i32)], at: (i32, i32), object: CrushedObject) -> Crush {
    match object {
        // An over-capacity **section** is still attached to the world and
        // is cracking *into* it: the fissure spreading out of a failing
        // boulder into the massif around it is the thing being modelled,
        // and the massif is by definition not in the region that failed.
        // (Confining the old walker to `region` was tried and was very
        // nearly a no-op -- 189 confined failures wrote 29 cells of
        // fissure between them.) So: a disc about the cell that gave way,
        // sized off the region's own extent between the same two bounds
        // the star's length used.
        CrushedObject::Section => {
            let extent = region.iter().map(|&(x, y)| (x - at.0).abs().max((y - at.1).abs())).max().unwrap_or(0) as usize;
            let mut reach = extent.clamp(object.min_joint_reach(), CRACK_MAX_LENGTH) as f32;
            // **Bounded by the licence, because this is the one consequence
            // that is *sized* rather than enumerated.** Everything else a
            // failure does works from `region`, which `tick` has already
            // clipped; this disc deliberately reaches into the massif around
            // the piece, out to `CRACK_MAX_LENGTH`, and a crush that writes
            // cracks outside the leash is the same leak one channel over --
            // cracked rock carries less, so it is a delayed failure the
            // player was promised would not happen. `None` at SPREAD, where
            // nothing bounds it and the arithmetic is untouched.
            //
            // **Whether the leash shrank it is remembered**, because the
            // two ways this disc can write nothing are opposites and `tick`
            // has to tell them apart. See `Crush`.
            let mut leashed = false;
            if let Some(headroom) = world.licence_headroom(at.0, at.1) {
                if (headroom as f32) < reach {
                    leashed = true;
                }
                reach = reach.min(headroom as f32);
            }
            Crush { fresh: reveal_in_disc(world, at, reach), leashed }
        }
        // A **severed piece** has no claim on the rock outside it. Its
        // damage has to scale from zero, and with a fabric it does so
        // twice over: the reveal is restricted to joints with *both* cells
        // inside the piece, and a piece smaller than the grain contains no
        // such joint at all, so it writes nothing without needing a floor
        // to say so. That is the same conclusion W4b reached by removing
        // `CRACK_MIN_LENGTH` from this arm, arrived at from the geometry
        // instead of from a constant -- and it is strictly tighter, because
        // a zero-length star still marked the cell it started on.
        CrushedObject::SeveredPiece => Crush { fresh: reveal_inside_region(world, region), leashed: false },
    }
}

/// The domain a cell's material is jointed on, or `None` if it is not
/// jointed at all.
///
/// The material gate is a `Vec` index on the resolved `Material` at a call
/// site that already holds the `Cell`, never an `id_of("stone")` string
/// hash in a loop (`CLAUDE.md`: guard hot-path work at the call site that
/// already has the data). Sand, soil, gravel and snow leave
/// `joint_spacing` at `0.0` and fall out here.
///
/// The pitch travels back with the domain because it is per material: two
/// different jointed materials meeting have *different lattices*, and
/// comparing a domain from one against a domain from the other would be
/// comparing lattice coordinates that do not mean the same thing.
fn joint_domain(world: &World, x: i32, y: i32) -> Option<((i32, i32), f32)> {
    if !world.in_bounds(x, y) {
        return None;
    }
    let cell = world.get(x, y);
    if !is_body_material(world, cell.material) {
        return None;
    }
    // Banded, not the flat material constant -- see
    // `fracture_field::pitch_at`. The crush has to read the *same* fabric
    // the blast does or a confined failure inside a blast's near field
    // stops re-selecting the boundaries the blast already severed, and the
    // crush starts writing over it again instead of going silent (which is
    // what `CRUSH_JOINT_DENSITY` matching `default_joint_density` is for).
    let pitch = {
        let m = world.materials.get(cell.material);
        super::fracture_field::pitch_at(world.seed, x, y, m.joint_spacing, m.joint_band_contrast)
    };
    if pitch <= 0.0 {
        return None;
    }
    Some((super::fracture_field::domain(world.seed, x, y, pitch), pitch))
}

/// Sever one edge and nothing else: the crack bit, no mirror, no glow, no
/// unbracing, no reschedule.
///
/// **No mirror write.** The walker has to mark both perpendicular edges of
/// every cell it visits, because a line drawn *through* cells leaves the
/// visited cell joined to the rock on the far side of the line and a
/// 4-connected flood threads straight out through the gap. A domain
/// boundary is not a line drawn through cells: it *is* the edge set, it
/// already contains every edge that separates the two domains, and the
/// mirror would darken a second cell for nothing. That second cell is the
/// owner's *"thick lines"* -- at zoom 1, which is what the app and every
/// GIF use, a crack bit darkens the whole cell.
///
/// Returns whether this was **fresh** damage, the same meaning
/// `explosion::sever` gives its return value and the meaning `tick`'s
/// treadmill guard depends on.
fn sever_joint(world: &mut World, x: i32, y: i32, down: bool) -> bool {
    let cell = world.get(x, y);
    let scored = if down { cell.with_crack_down(true) } else { cell.with_crack_right(true) };
    if scored == cell {
        return false;
    }
    world.set(x, y, scored);
    // A crush is damage even though nothing moves: this cell has stopped
    // being intact rock. Measured per severed edge rather than over the
    // failing region, because the `Section` disc deliberately reaches
    // *into* the massif around the piece that failed -- the region is not
    // where the crack landed.
    if let Some(d) = world.distance_to_live_disturbance(x, y) {
        world.structural_failures.record_damage_reach(d);
    }
    true
}

/// Reveal the joints in a disc of `reach` cells about `at`, and return how
/// many edges were newly severed.
///
/// The ramp is `CRUSH_JOINT_DENSITY` at the centre falling linearly to zero
/// at `reach`, compared against the boundary's own `joint_draw` -- **no
/// hard cut at the radius**. A joint is drawn from the middle outward and
/// stops where the ramp falls under its own draw, so some run much further
/// than their neighbours and the damaged patch has a ragged edge. Clipping
/// at a radius instead is the mistake that shipped the round-3 caves with a
/// sawn-off face (`CAVE_EDGE_FADE`), and the same trick is why
/// `explosion::JointSeams` has none either.
///
/// Keyed on the **pair of domains**, not on the edge: one draw for a whole
/// boundary means a joint is either a full straight segment or absent,
/// where a per-edge draw activates them in a dashed scatter -- which is the
/// scribble complaint wearing a different hat.
fn reveal_in_disc(world: &mut World, at: (i32, i32), reach: f32) -> u32 {
    if reach < 1.0 {
        return 0;
    }
    let r = reach.ceil() as i32;
    let (x0, y0) = (at.0 - r, at.1 - r);
    // One extra row and column: the last cell in the box still has to be
    // able to ask about the neighbour it owns an edge with.
    let (w, h) = ((2 * r + 2) as usize, (2 * r + 2) as usize);
    let idx = |x: i32, y: i32| ((y - y0) as usize) * w + (x - x0) as usize;
    // The domain map for the box, computed once. A cell's domain costs nine
    // hashes and the edge test needs both cells', so caching halves the
    // work outright -- and this runs on a structural failure, tens to
    // hundreds of times in a run, never per cell per frame.
    let mut map: Vec<Option<((i32, i32), f32)>> = vec![None; w * h];
    for y in y0..(y0 + h as i32) {
        for x in x0..(x0 + w as i32) {
            let (dx, dy) = (x - at.0, y - at.1);
            // A cell just outside the reach still has to be mapped: the
            // edge it shares with the last cell *inside* is a real
            // boundary and would otherwise read as "no domain".
            if ((dx * dx + dy * dy) as f32) > (reach + 1.0) * (reach + 1.0) {
                continue;
            }
            map[idx(x, y)] = joint_domain(world, x, y);
        }
    }
    let mut fresh = 0;
    for y in y0..(y0 + h as i32 - 1) {
        for x in x0..(x0 + w as i32 - 1) {
            let Some((home, pitch)) = map[idx(x, y)] else { continue };
            let (dx, dy) = (x - at.0, y - at.1);
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            if d > reach {
                continue;
            }
            let ramp = CRUSH_JOINT_DENSITY * (1.0 - d / reach);
            for down in [false, true] {
                let (nx, ny) = if down { (x, y + 1) } else { (x + 1, y) };
                let Some((other, other_pitch)) = map[idx(nx, ny)] else { continue };
                // Different lattices never share a joint, and two cells in
                // the same domain have no boundary between them. The
                // severing rule is that identity comparison and nothing
                // else -- no threshold, so no width to leak through.
                if other_pitch != pitch || other == home {
                    continue;
                }
                if super::fracture_field::joint_draw(world.seed, home, other) >= ramp {
                    continue;
                }
                fresh += u32::from(sever_joint(world, x, y, down));
            }
        }
    }
    fresh
}

/// Reveal only the joints **inside** a severed piece -- both cells of the
/// edge in `region` -- and return how many were newly severed.
///
/// Flat activation rather than a ramp: the object here is the *piece*, and
/// a piece has no centre for a distance to be measured from. The density is
/// the same one the disc starts at, so a boundary that a nearby crush or
/// blast has already taken is taken again and writes nothing.
///
/// **Both cells, deliberately.** An edge with one cell outside the piece
/// belongs to the collar of rock the fissures already cut -- severing it
/// again writes a crack bit into rock the piece has no claim on, which is
/// precisely how one severed island manufactures the next
/// (`a_severed_chip_with_no_inside_left_cracks_nothing`'s second arm is the
/// guard on it).
fn reveal_inside_region(world: &mut World, region: &[(i32, i32)]) -> u32 {
    let cells: std::collections::HashSet<(i32, i32)> = region.iter().copied().collect();
    let mut fresh = 0;
    for &(x, y) in region {
        let Some((home, pitch)) = joint_domain(world, x, y) else { continue };
        for down in [false, true] {
            let (nx, ny) = if down { (x, y + 1) } else { (x + 1, y) };
            if !cells.contains(&(nx, ny)) {
                continue;
            }
            let Some((other, other_pitch)) = joint_domain(world, nx, ny) else { continue };
            if other_pitch != pitch || other == home {
                continue;
            }
            if super::fracture_field::joint_draw(world.seed, home, other) >= CRUSH_JOINT_DENSITY {
                continue;
            }
            fresh += u32::from(sever_joint(world, x, y, down));
        }
    }
    fresh
}

/// The height of a crush's activation ramp at its own centre -- the
/// fraction of the joints in the thick of it that part.
///
/// `0.9`, which is `explosion::default_joint_density`, and the match is the
/// point rather than a coincidence: a confined failure inside a blast's
/// near field then re-selects the boundaries the blast has already severed
/// and writes nothing, so the crush goes silent exactly where the owner
/// reported the noise. Not `1.0` for the reason that knob is not either --
/// a handful of missing edges is what stops the near field reading as a
/// drawn tessellation.
const CRUSH_JOINT_DENSITY: f32 = 0.9;

/// Walk one star of wandering, forking fissures out of `start`, and return
/// how many fresh crack bits were written.
///
/// # Why a walker and not a fan of rays
///
/// A fissure *wanders and forks*; it does not travel in a straight line.
/// Each walker carries a heading, turns a little at every cell, and
/// occasionally throws a fork off to one side -- so what comes out is one
/// crack spreading through the rock and splitting, rather than spokes on a
/// wheel. `rigid::score_cracks` draws the wheel, and at a blast's scale
/// (12 rays, 30 cells each) the owner read it off the contact sheet
/// immediately: "perfectly uniform and mirrored", an asterisk rather than
/// a fracture star (`Reports/explosion-stone-review.md` §8b).
///
/// Every one of those decisions is `rng::jitter` on the cell the walker is
/// standing on: a hash of position, never a draw from `world.rng`. That
/// keeps the dig, the collapse and the blast out of the replay draw order
/// entirely, and it makes the shape a property of the rock -- work the
/// same stone twice and the crack runs where it ran before, deeper,
/// instead of a second scribble appearing beside the first.
///
/// Everything descends from **one** origin. That is the fix for the
/// owner's verdict on the version before this one -- "it doesn't really
/// look like a spreading crack from a boulder, it just looks like criss
/// cross irregular lines" -- which scattered origins through the piece and
/// drew straight spokes from each. Independent strokes crossing each other
/// read as scribble; a trunk with branches reads as a crack. So there is a
/// single seed point, `primaries` strokes leaving it like the arms of a
/// windscreen star, and every other stroke is a fork off one of those.
///
/// # The two callers, and the one flag that separates them
///
/// **Read in the past tense: the crush is no longer one of them, and this
/// entry point is `#[cfg(test)]` now.** The split below is still exactly
/// live -- `FissureWalks::detach` is a real flag with a real production
/// user, the blast's `crack_rays` star at `true` -- and the `false` arm is
/// still the rule a confined crush obeys, which is why the argument is
/// kept here whole. What changed is what draws it: `crush_in_place` reveals
/// the rock's joint fabric instead of walking a star, and the `detach:
/// false` contract moved with it (see `reveal_joints`, and
/// `a_crush_neither_unbraces_the_rock_nor_reschedules_it`, which is now
/// written against the crush rather than against this).
///
/// `crush_in_place` passed `detach: false`; the blast in `explosion.rs`
/// passes `true`. `detach` turns on the two things
/// `rigid::score_cracks` does per scored cell and this deliberately does
/// not do for a crush:
///
/// - `detach_around_crack` + `schedule_structural_check_around`, which is
///   how a blast's *reach* gets into the structural model at all -- the
///   crater is 20 cells and the fissures run 50, and without the halo the
///   load model never hears about them.
/// - the **crack-tip bonus**: crossing rock that was already cracked
///   before this walk buys extra budget, so a second charge in the same
///   place drives the existing fissures deeper instead of retracing them
///   and writing nothing. Position-keyed wander makes a repeat blast
///   retrace its own paths exactly, so without this the second charge is
///   visually a no-op.
///
/// Both are wrong for a confined crush, and the second is wrong in an
/// expensive way: a crush is *meant* to be idempotent on rock it has
/// already cracked (there is no spare `Cell` flag to record "crushed"), and
/// `tick`'s "a crush that wrote nothing has nothing to propagate" guard is
/// what stopped a crushed pocket re-failing 1,120 times every 400 frames.
/// Give the crush a tip bonus and every re-crush finds fresh budget in its
/// own old damage, which is that treadmill again.
///
/// # Why the primaries share one fork pool
///
/// `primaries` is a count rather than one call per stroke because the fork
/// budget is *shared*: the strokes are processed breadth-first out of one
/// queue and take forks from one pool in that order. Calling this once per
/// primary with the pool split three ways draws a different crack, so the
/// count stays a parameter and `crush_in_place` stayed bit-identical to the
/// version this was extracted from.
#[cfg(test)]
pub(crate) fn walk_fissures(
    world: &mut World,
    start: (i32, i32),
    heading: f32,
    primaries: usize,
    length: usize,
    forks: usize,
    detach: bool,
) -> u32 {
    FissureWalks::new(start, heading, primaries, length, forks, detach).run_to_completion(world)
}

/// One walker's entire state, so a crack can be **put down mid-stroke and
/// picked up on a later frame**.
///
/// Every field here was a local in the loop that used to run to completion
/// inside `walk_fissures`. Two of them are the reason this is a struct
/// rather than a tuple of the obvious three:
///
/// - **`fx`/`fy` must persist across a resume.** The path is carried at
///   sub-cell precision and rounded only to decide which cell it is in (see
///   `step_walker`'s own comment for what axis-stepping did instead).
///   Re-deriving `fx, fy` from `pos` on resume -- the obvious way to store
///   less -- re-centres the walker in its cell every time it is picked up,
///   which throws away the fractional offset the wander has accumulated and
///   straightens the crack toward whatever heading it happens to hold. It is
///   the same quantisation the sub-cell path exists to avoid, just applied
///   once per frame instead of once per cell.
/// - **`budget` and `steps` are kept apart rather than collapsed into one
///   "steps left".** A fork inherits `budget / 2` of its parent's *total*
///   budget, tip bonuses included, not of the parent's remainder; a single
///   remaining-count cannot reconstruct that, and getting it wrong changes
///   every branch length on the crush path -- which has to stay
///   bit-identical.
#[derive(Clone, Copy, Debug)]
struct Walker {
    pos: (i32, i32),
    fx: f32,
    fy: f32,
    heading: f32,
    /// Total sub-cell steps this walker may take, including any crack-tip
    /// bonus it has earned so far.
    budget: usize,
    /// How many of `budget` it has already taken.
    steps: usize,
    /// The hard cap `budget` may be bonused up to.
    ceiling: usize,
    /// Frames still to wait before this walker takes its first step. Zero
    /// for every crush walker; the blast staggers its rays so the star does
    /// not leave the crater as one synchronised starburst. Ignored entirely
    /// by `run_to_completion`, which has no frames to count.
    delay: u16,
    /// Cells still to be entered before the current straight run ends and
    /// the heading kinks. `CrackStyle::Brittle` only -- the wandering style
    /// turns at every cell and never reads this.
    run_left: u16,
    /// Set when the walk ends -- budget spent, out of bounds, or out of
    /// rock. Needed because `is_done` cannot tell "spent" from "stopped at a
    /// free face" from `steps` alone.
    done: bool,
}

/// A star of wandering, forking fissures, in progress.
///
/// This is `walk_fissures`' loop turned inside out so its state can live
/// across frames. It exists because of the owner's verdict on the star the
/// one-call version drew: *"it looks the same everytime, it looks like a
/// graphic stamped on the stone and not a realistic fissure. It would be
/// cool if you could see it grow."* The shape was never the problem by
/// then -- a thing that appears whole on the bang frame and never moves
/// again reads as a decal however organic its outline is. So the *timing*
/// of the writes changed and nothing else did: the pattern for a given site
/// is still a pure function of position (`rng::jitter`, never `world.rng`),
/// so a repeat charge still retraces and deepens its own fissures.
///
/// # Two drivers over one step function
///
/// The repo's own two-drivers shape (`update::step` / `parallel::step`, and
/// `Blasts` / `trigger_tuned` next door in `explosion.rs`), for the same
/// reason: a second copy of the walker would drift from this one the first
/// time either changed.
///
/// - [`run_to_completion`](Self::run_to_completion) drains walker `head`
///   entirely before starting the next -- byte-for-byte the order the
///   single-call version used, which is what keeps `crush_in_place`
///   bit-identical.
/// - [`advance`](Self::advance) is a round-robin: every live walker takes up
///   to N steps per call and forks join the rota on later passes.
///
/// **The two legitimately draw different stars from the same inputs**, and
/// that is not a bug to be chased: the fork pool is shared, so who spends it
/// depends on the order walkers are visited in, and round-robin visits them
/// in a different order than depth-first does. Nothing compares the two --
/// the crush path only ever calls the first, the blast only ever the
/// second -- so the property that matters (each is deterministic in the
/// site) holds for both.
/// Which *shape* a walk draws — the one thing that differs between the
/// crush's cracks and a blast's.
///
/// # Why black rock needs its own style
///
/// The wandering walker turns a little at *every* cell, which integrates
/// into a smooth curve: over a 40-cell ray that reads as a meander. The
/// owner's verdict on the growing star was that the cracks were "a little
/// too organic", and the answer is not less wander but a different
/// *statistic*. Brittle fracture in a hard, homogeneous rock is angular:
/// the crack front runs nearly straight until it meets something that
/// deflects it, then turns sharply and runs straight again. Near-straight
/// segments, sharp kinks, acute branches — a lightning bolt, not a river.
///
/// So this is a distribution swap, not a tuning: the same per-cell
/// position-keyed jitter budget is spent in a few large turns instead of
/// many small ones.
///
/// **The crush keeps `Wander`, byte for byte.** `crush_in_place`'s output
/// is pinned by a PNG hash on `scene=strike` and there is no reason to
/// change what a confined failure inside a mountain looks like in the same
/// pass that changes what a blast looks like. Whether the crush should
/// adopt this later is a question for a sheet, not for this enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CrackStyle {
    /// Turn by up to `CRACK_WANDER` at every cell. The original, and what
    /// every crush draws.
    Wander,
    /// Hold the heading for a run of cells, then kink sharply. See
    /// `brittle_run`/`brittle_kink`.
    Brittle,
}

#[derive(Clone, Debug)]
pub(crate) struct FissureWalks {
    walkers: Vec<Walker>,
    /// The **shared** fork pool. Shared rather than per-walker because that
    /// is what `crush_in_place` was extracted from: strokes are processed
    /// out of one queue and take forks from one pool in that order, and
    /// splitting the pool per stroke draws a different crack.
    forks: usize,
    detach: bool,
    /// Only *pre-existing* damage earns a walker extra reach. Strokes from
    /// one walk cross each other near the origin, so counting cracks this
    /// walk just made would have every blast max out its own bonus
    /// immediately and leave nothing for the next one to build on -- the
    /// trap `rigid::score_cracks` records having fallen into first. It
    /// spans the whole star, and therefore the whole *growth*, not one
    /// frame's worth: reset it per frame and the second frame's walkers
    /// would be bonused by the first frame's own writes.
    scored_now: std::collections::HashSet<(i32, i32)>,
    /// See [`CrackStyle`].
    style: CrackStyle,
    /// Temperature written into every cell the walk crosses, if the cell is
    /// colder — the crack tip's own incandescence. `0` is off, which is what
    /// every crush walk uses: rock parting under load 80 cells inside a
    /// mountain is not hot, and a blast's fracture front genuinely is.
    ///
    /// Owned here rather than written by the caller afterwards because only
    /// the walker knows *which* cells it crossed on any given frame, and the
    /// whole point is that the glow tracks the racing tip rather than
    /// appearing all at once along the finished star.
    glow: i16,
}

impl FissureWalks {
    /// The crush star: `primaries` strokes leaving one origin at even
    /// angles, sharing one fork pool. Exactly what `walk_fissures` took.
    ///
    /// **Test-only since the fabric landed.** This is the constructor that
    /// picks `CrackStyle::Wander`, and the crush was its only production
    /// caller; the blast builds its hybrid star through `empty` +
    /// `add_ray` and has always been `Brittle`. Kept, with the wandering
    /// style and the tests that pin its shape, because `crack_rays > 0` is
    /// still the owner's A/B knob and the walker has to keep working --
    /// see `reveal_joints` for what took its place on the crush.
    #[cfg(test)]
    pub(crate) fn new(start: (i32, i32), heading: f32, primaries: usize, length: usize, forks: usize, detach: bool) -> Self {
        // `Wander`, and no glow: this is the crush constructor, and both
        // halves of that are load-bearing (`CrackStyle`'s own doc).
        let mut walks = Self::empty(detach, CrackStyle::Wander, 0);
        walks.forks = forks;
        for i in 0..primaries {
            walks.push_walker(start, heading + i as f32 * std::f32::consts::TAU / primaries.max(1) as f32, length, 0);
        }
        walks
    }

    /// An empty star, for callers that add their rays one at a time --
    /// the blast's fan, whose spokes each have their own start point,
    /// length and start delay.
    pub(crate) fn empty(detach: bool, style: CrackStyle, glow: i16) -> Self {
        Self { walkers: Vec::new(), forks: 0, detach, scored_now: std::collections::HashSet::new(), style, glow }
    }

    /// Every cell this star has scored so far, in no particular order.
    ///
    /// Exposed for the blast's afterglow, which has to cool the cells the
    /// walkers heated and cannot find them any other way: the star reaches
    /// far outside the scorch disc's own bounding box (a ray starts past the
    /// crater and runs `crack_reach` blast-radii further), so a box scan
    /// large enough to contain it would be mostly untouched rock.
    ///
    /// Iteration order is a `HashSet`'s and therefore not stable between
    /// runs. That is fine for the one caller and must stay that way: it
    /// applies an *order-independent* operation to each cell (a temperature
    /// it computes from that cell alone). Anything order-sensitive needs to
    /// sort first.
    pub(crate) fn scored(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.scored_now.iter().copied()
    }

    /// Add one ray, contributing `forks` to the shared pool and waiting
    /// `delay` calls to `advance` before it starts.
    pub(crate) fn add_ray(&mut self, start: (i32, i32), heading: f32, length: usize, forks: usize, delay: u16) {
        self.forks += forks;
        self.push_walker(start, heading, length, delay);
    }

    fn push_walker(&mut self, pos: (i32, i32), heading: f32, budget: usize, delay: u16) {
        self.walkers.push(Walker {
            pos,
            fx: pos.0 as f32 + 0.5,
            fy: pos.1 as f32 + 0.5,
            heading,
            budget,
            steps: 0,
            ceiling: budget + (super::rigid::CRACK_TIP_BONUS * super::rigid::CRACK_TIP_MAX_STEPS) as usize,
            delay,
            // A fork inherits the segment rhythm rather than the remainder of
            // its parent's run: a branch that leaves at an acute angle and
            // then immediately kinks reads as a tangle, not as a splinter.
            // Rolled from the fork point, so it is still a property of the
            // rock. Not computed at all for `Wander`, which never reads it --
            // one hash per walker, but the crush path must not pay even that.
            run_left: match self.style {
                CrackStyle::Wander => 0,
                CrackStyle::Brittle => brittle_run(pos),
            },
            done: budget == 0,
        });
    }

    /// Every walker has spent its budget or run out of rock.
    pub(crate) fn is_done(&self) -> bool {
        self.walkers.iter().all(|w| w.done)
    }

    /// Draw the whole star now, in one call. Depth-first: each walker is
    /// drained completely, forks included by the time the queue reaches
    /// them, before the next walker starts.
    ///
    /// **Test-only since the fabric landed**, and that is the honest
    /// record of what happened rather than a tidy-up: drawing a whole
    /// 3-ray, 4-fork, 10-to-55-cell star *in a single frame, fully formed*
    /// is half of the owner's complaint about the crush ("thick lines that
    /// appear later in the blast", arriving whole). The blast's own hybrid
    /// star goes through `advance`, a slice per frame, and the crush no
    /// longer walks at all -- see `reveal_joints`. The cursor is a local
    /// now rather than a field on the struct for the same reason: nothing
    /// resumes a one-shot draw, and a written-but-never-read field is a
    /// claim that something does.
    #[cfg(test)]
    pub(crate) fn run_to_completion(&mut self, world: &mut World) -> u32 {
        let mut fissured = 0;
        let mut head = 0;
        while head < self.walkers.len() {
            fissured += self.step_walker(world, head, usize::MAX);
            head += 1;
        }
        fissured
    }

    /// One round-robin pass: every live walker takes up to `steps_per_walker`
    /// steps, and returns the fresh crack bits written this call.
    ///
    /// Forks thrown during a pass join the queue behind the walkers already
    /// in it and get their first turn on the *next* call -- a tip that
    /// splits should not have both halves leap a frame ahead of every other
    /// tip in the star.
    ///
    /// The budget is counted in walker steps, not in cells entered. A step
    /// is one unit of sub-cell travel and usually but not always crosses
    /// into a new cell, so a ray advances slightly less than
    /// `steps_per_walker` cells per frame -- which is the honest thing to
    /// bound anyway, since a step that stays inside its cell still costs the
    /// jitter and the trig.
    pub(crate) fn advance(&mut self, world: &mut World, steps_per_walker: usize) -> u32 {
        let mut fissured = 0;
        // Snapshot the length so forks born during this pass wait for the
        // next one rather than being swept up by the same loop.
        let live = self.walkers.len();
        for i in 0..live {
            if self.walkers[i].done {
                continue;
            }
            if self.walkers[i].delay > 0 {
                self.walkers[i].delay -= 1;
                continue;
            }
            fissured += self.step_walker(world, i, steps_per_walker);
        }
        fissured
    }

    /// The single walker-step core both drivers run, bounded to `max_steps`
    /// steps of walker `i`. Everything that decides where a crack goes lives
    /// here and only here, so the two drivers cannot drift.
    fn step_walker(&mut self, world: &mut World, i: usize, max_steps: usize) -> u32 {
        let Walker { mut pos, mut fx, mut fy, mut heading, mut budget, mut steps, ceiling, mut run_left, .. } = self.walkers[i];
        let mut fissured = 0u32;
        let mut done = false;
        let mut taken = 0usize;
        while steps < budget && taken < max_steps {
            steps += 1;
            taken += 1;
            let wobble = super::rng::jitter(pos.0, pos.1);
            // `wobble` is still drawn for both styles -- the fork chance
            // below reads it, and it must stay the *same* draw at the same
            // point in the sequence or the crush path stops being
            // bit-identical.
            if self.style == CrackStyle::Wander {
                heading += (wobble - 0.5) * CRACK_WANDER;
            }
            let (dx, dy) = (heading.cos(), heading.sin());
            // The path is carried at sub-cell precision and rounded only to
            // decide which cell it is in. Stepping to an axis neighbour
            // instead -- whichever of the four the heading pointed most
            // nearly at -- was tried and is what produces a staircase: the
            // heading turns slowly, so its dominant axis stays the same for
            // many cells, and the crack comes out as long horizontal and
            // vertical runs meeting at right angles. That is the "criss
            // cross" look, and no amount of extra wander fixes it, because
            // the quantisation is doing it rather than the path. `Walker`'s
            // own doc records the resume-shaped version of the same trap.
            fx += dx;
            fy += dy;
            let next = (fx.floor() as i32, fy.floor() as i32);
            if next == pos {
                continue; // still inside the same cell; keep going
            }
            let step = (next.0 - pos.0, next.1 - pos.1);
            if !world.in_bounds(next.0, next.1) {
                done = true;
                break;
            }
            // **One step can cross two cell boundaries at once**, and for a
            // long time only one of them was scored. The path is carried at
            // sub-cell precision (above), so a unit step near 45 degrees
            // crosses both boundaries in the same step and lands on a
            // *diagonal* neighbour -- and the cell between the two is never
            // visited and never marked. The drawn crack is then
            // 8-connected while every consumer of it walks `NEIGHBOURS_4`
            // (`load::is_supported`, `load::detached_piece`,
            // `rigid::take_fragment`), so support zigzags straight through
            // the line and a fissure that visibly goes all the way round a
            // piece cuts nothing off it. That is the owner's *"cracks that
            // fully surround chunks of rock do not break them off"*, and it
            // is a **drawing** bug rather than a structural one:
            // `a_closed_crack_loop_isolates_what_it_encircles` flooded a
            // ring drawn by this walker and escaped to the world border.
            //
            // So a diagonal step is decomposed into the two axis steps it
            // really is and **both** cells are scored, in travel order.
            // The corner is taken **x first, arbitrarily but fixed** -- y
            // first would draw an equally good crack and the choice is not
            // a modelling claim, but it must not be a coin flip: a
            // `world.rng` draw anywhere in this walker would put the blast
            // into the replay draw order and stop a repeat charge
            // retracing its own fissures
            // (`a_brittle_star_is_the_same_star_every_time`).
            //
            // `run_left` is deliberately **not** decremented twice for a
            // decomposed step (see below). The corner is the same unit of
            // travel, not a further cell of fracture.
            let mut hops = [((0, 0), false); 2];
            let hop_count = if step.0 != 0 && step.1 != 0 {
                hops[0] = ((next.0, pos.1), true);
                hops[1] = (next, false);
                2
            } else {
                // Which edge a fissure cuts is the one *perpendicular* to
                // the way it travels -- get this backwards and the crack
                // runs across itself, slicing the rock into rings rather
                // than parting it. `rigid::score_cracks` records the same
                // trap. A decomposed diagonal is two such steps, one on
                // each axis, which is why the flag rides in `hops` rather
                // than being recomputed from `step` below.
                hops[0] = (next, step.0.abs() >= step.1.abs());
                1
            };
            let mut blocked = false;
            for &(at, horizontal) in &hops[..hop_count] {
                let cell = world.get(at.0, at.1);
                // A fissure runs through **rock**, not through the
                // bookkeeping region that was judged, and it stops where
                // the rock does -- there is nothing to split at a free
                // face.
                //
                // Confining it to `region` was tried first and was very
                // nearly a no-op, which only a counter could show: 189
                // confined failures wrote **29 cells of fissure between
                // them**, and produced bit-identical images across two
                // complete rewrites of the crack pattern. An overload's
                // region is the union of the *subtrees* hanging off the
                // failing section -- a sparse, often thin set that the
                // walker leaves on its first step -- so almost every crack
                // died where it was born. It is also the wrong picture: a
                // crack spreading out of a boulder into the massif around
                // it is the thing being modelled, and the massif is by
                // definition not in the region that failed.
                if !is_body_material(world, cell.material) {
                    blocked = true;
                    break;
                }
                if self.detach && cell.cracked() && !self.scored_now.contains(&at) {
                    budget = (budget + super::rigid::CRACK_TIP_BONUS as usize).min(ceiling);
                }
                let mut scored = if horizontal { cell.with_crack_down(true) } else { cell.with_crack_right(true) };
                // Counted off the crack bit alone, and *before* any glow is
                // written: `cells_fissured` means "rock that was not
                // cracked here before", and a cell whose bit was already
                // set but which this walk reheated is not new damage.
                //
                // Still **per visited cell**, which is the meaning the
                // treadmill guard in `tick` reads (`crush_in_place(..) >
                // 0`). A decomposed diagonal visits two cells and can
                // therefore count two, which is right -- they are two cells
                // of fresh fissure. The mirror edge written below is not
                // counted: it is the far side of *this* cell, held by the
                // neighbour under `FLAG_CRACK_RIGHT`'s ownership rule, and
                // counting it would turn this into an edge count wearing a
                // cell count's name.
                if scored != cell {
                    fissured += 1;
                }
                // The crack tip is incandescent while it races (`glow`'s
                // own doc). Never cools a cell that is already hotter --
                // the same one-way rule `scorch` uses, for the same reason:
                // two overlapping blasts, or a crack crossing something on
                // fire.
                if self.glow > 0 && scored.temperature() < self.glow {
                    scored.set_temperature(self.glow);
                }
                if scored != cell {
                    world.set(at.0, at.1, scored);
                }
                // **Both** perpendicular edges, not one -- the other one
                // belongs to the neighbour. Marking only the near edge
                // leaves the visited cell joined to the rock on the far
                // side of the line, so a 4-connected path threads along
                // the crack and out at the first cell the line changes
                // direction on.
                //
                // **It takes both halves of this commit to seal, and each
                // alone is worth nothing** -- measured on
                // `a_closed_crack_loop_isolates_what_it_encircles`, whose
                // flood escapes the ring in three of four builds:
                //
                // ```text
                // one edge, no decomposition   flood 474, escaped
                // both edges, no decomposition flood 465, escaped
                // one edge, decomposed         flood 426, escaped
                // both edges, decomposed       flood 283, sealed (265 enclosed)
                // ```
                //
                // Anyone tempted to back half of this out for the frame
                // cost should read that table as saying the half left
                // standing buys nothing at all.
                //
                // An edge is owned by exactly one of the two cells it
                // separates (`FLAG_CRACK_RIGHT`), so reaching *up* or
                // *left* means writing the **neighbour's** bit.
                let mirror = if horizontal { (at.0, at.1 - 1) } else { (at.0 - 1, at.1) };
                if world.in_bounds(mirror.0, mirror.1) {
                    let neighbour = world.get(mirror.0, mirror.1);
                    if is_body_material(world, neighbour.material) {
                        let scored = if horizontal { neighbour.with_crack_down(true) } else { neighbour.with_crack_right(true) };
                        if scored != neighbour {
                            world.set(mirror.0, mirror.1, scored);
                        }
                    }
                }
                if self.detach {
                    self.scored_now.insert(at);
                    // A fissure is where the rock has parted company with
                    // the mass behind the slice, so it stops claiming to be
                    // braced by it. Done for every cell the walk crosses,
                    // not only the ones whose bit changed: a repeat charge
                    // has to re-loosen a rim that is already scored, or the
                    // second shot throws nothing.
                    //
                    // Called for the decomposed corner as well as for the
                    // destination, and this is the expensive half of the
                    // decomposition rather than the crack bits:
                    // `detach_around` is a 3x3, `attached_span_bonus` is 12
                    // for stone, and attachment is never regained, so
                    // visiting ~40% more cells on an oblique heading
                    // unbraces materially more rock than the extra bits do.
                    //
                    // Left on for both cells because the seed sweep did not
                    // ask for otherwise: over `strike=12` x 6 presets x 4
                    // seeds the order statistics moved the safe way (rock
                    // destroyed max 1,349 -> 1,123, p90 948 -> 582; cells
                    // lost max 583 -> 267). **If that ever inverts, the
                    // first lever is to call this on the primary cell of a
                    // decomposed diagonal only** -- the corner still gets
                    // its crack bit, so the sealing above is untouched and
                    // only the unbracing halves.
                    detach_around_crack(world, at.0, at.1);
                    world.schedule_structural_check_around(at.0, at.1);
                }
                pos = at;
            }
            if blocked {
                done = true;
                break;
            }
            // The brittle style's whole shape, in three lines: the heading
            // is untouched for a run of cells, and then turns sharply. The
            // run is counted in **cells entered**, not in walker steps --
            // a step that stays inside its own cell has not advanced the
            // fracture, and counting those would make a segment's drawn
            // length depend on how obliquely the heading crosses the grid.
            //
            // A decomposed diagonal enters two cells and still counts
            // **once**, for that same reason read the other way round: the
            // corner is bookkeeping about which edges a single unit of
            // travel cut, not a second unit of travel. Counting it would
            // shorten every brittle segment by about 40% on an oblique
            // heading -- retuning the archived crack shape as a side effect
            // of a fix that is not about shape at all, and moving
            // `a_brittle_crack_turns_rarely_and_sharply`, which is the most
            // restrictive shape pin in this file.
            if self.style == CrackStyle::Brittle {
                run_left = run_left.saturating_sub(1);
                if run_left == 0 {
                    heading += brittle_kink(pos);
                    run_left = brittle_run(pos);
                }
            }
            // Forking, not crossing. A branch leaves at a wide angle and
            // gets a shorter budget than its parent, which is what makes
            // the result read as a crack with side-cracks rather than as a
            // tangle -- the second is what several straight rays from
            // several origins produced, and it looked like scribble.
            if self.forks > 0 && wobble < CRACK_FORK_CHANCE {
                self.forks -= 1;
                // Brittle branches leave *acutely* and to a side the rock
                // chooses, where the wandering style always turns the same
                // way by a fixed wide angle. A fixed side is invisible on a
                // meandering crack and glaring on a straight one: every
                // branch of every ray would rake the same way, which reads
                // as a feather rather than as fracture.
                let fork_angle = match self.style {
                    CrackStyle::Wander => CRACK_FORK_ANGLE,
                    CrackStyle::Brittle => {
                        let side = super::rng::jitter(pos.0 + BRITTLE_SIDE_KEY, pos.1);
                        if side < 0.5 { -BRITTLE_FORK_ANGLE } else { BRITTLE_FORK_ANGLE }
                    }
                };
                let (fork_pos, fork_heading, fork_budget) = (pos, heading + fork_angle, budget / 2);
                self.push_walker(fork_pos, fork_heading, fork_budget, 0);
            }
        }
        let w = &mut self.walkers[i];
        w.pos = pos;
        w.fx = fx;
        w.fy = fy;
        w.heading = heading;
        w.budget = budget;
        w.steps = steps;
        w.run_left = run_left;
        w.done = done || steps >= budget;
        fissured
    }
}

/// How many cells a brittle crack runs before it kinks.
///
/// Position-keyed, so the segmentation is a property of the rock like
/// everything else here: 3 to 8 cells, which at the blast's 10-55 cell ray
/// lengths gives a handful of segments per ray -- enough to read as angular,
/// few enough that each segment is visibly straight. Shorter and it is a
/// meander again with extra steps; longer and a ray is one straight spoke,
/// which is the asterisk the walker replaced.
fn brittle_run(pos: (i32, i32)) -> u16 {
    BRITTLE_RUN_MIN + (super::rng::jitter(pos.0, pos.1 + BRITTLE_RUN_KEY) * BRITTLE_RUN_SPREAD) as u16
}

/// How sharply a brittle crack turns at the end of a run, in radians,
/// signed.
///
/// Two draws: one for the size of the turn, one for which way it goes. The
/// side has to be its own draw -- deriving it from the magnitude (turn left
/// when small, right when large, say) couples the two, and a crack that only
/// ever turns hard one way spirals.
///
/// Most kinks are ordinary (0.2-0.6 rad, roughly 11-34 degrees); about one
/// in twelve is a real deflection of up to 0.9. That tail is the difference
/// between "regularly zigzagging" and "brittle": a crack that meets a flaw
/// changes direction *decisively*, and without the rare large turn the ray
/// still tracks its original heading over its whole length.
fn brittle_kink(pos: (i32, i32)) -> f32 {
    let j = super::rng::jitter(pos.0 + BRITTLE_KINK_KEY, pos.1);
    let side = super::rng::jitter(pos.0, pos.1 + BRITTLE_SIDE_KEY);
    let magnitude = if j < BRITTLE_SNAP_CHANCE {
        // Remapped onto its own range rather than reusing `j` raw, which
        // would make every rare deflection a *small* one -- `j` is tiny by
        // construction inside this branch.
        BRITTLE_SNAP_MIN + (j / BRITTLE_SNAP_CHANCE) * (BRITTLE_SNAP_MAX - BRITTLE_SNAP_MIN)
    } else {
        BRITTLE_KINK_MIN + j * BRITTLE_KINK_SPREAD
    };
    if side < 0.5 {
        -magnitude
    } else {
        magnitude
    }
}

/// The brittle style's numbers. Offsets keyed into `rng::jitter`'s two
/// inputs so that a cell's run length, kink size and kink side are three
/// different values rather than the same hash read three times -- the same
/// trick `explosion.rs`'s `JITTER_AXIS_OFFSET` exists for, and arbitrary
/// primes for the same reason.
const BRITTLE_RUN_KEY: i32 = 6_733;
const BRITTLE_KINK_KEY: i32 = 15_486_277;
const BRITTLE_SIDE_KEY: i32 = 2_654_435;
const BRITTLE_RUN_MIN: u16 = 3;
const BRITTLE_RUN_SPREAD: f32 = 5.0;
const BRITTLE_KINK_MIN: f32 = 0.2;
const BRITTLE_KINK_SPREAD: f32 = 0.4;
const BRITTLE_SNAP_CHANCE: f32 = 0.08;
const BRITTLE_SNAP_MIN: f32 = 0.45;
const BRITTLE_SNAP_MAX: f32 = 0.9;
/// Acute, where `CRACK_FORK_ANGLE` is deliberately wide. A brittle branch
/// splits *forward* off the parent -- the shallow Y a struck pane makes --
/// rather than heading off sideways.
const BRITTLE_FORK_ANGLE: f32 = 0.55;

/// How far a confined failure's reveal reaches, bounded by the piece it is
/// reaching out of.
///
/// Scaled to the failing region's own extent between these two, because
/// the damage has to be a *distribution*: the same patch of grain revealed
/// around a 900-cell section and around a 30-cell one is the all-or-nothing
/// outcome in miniature. The floor keeps a small failure from producing
/// damage too faint to see -- and, since `d5cb19a` gave stone a 13-cell
/// grain, from producing none at all; the ceiling keeps a large one from
/// dicing a quarter of the screen in one tick.
///
/// **These two used to bound a fissure walk's length**, alongside three
/// constants that are gone with it: `CRACK_PRIMARIES` (3 strokes out of one
/// origin), `CRACK_FORKS_BASE` (4) and `CRACK_CELLS_PER_FORK` (80, one
/// extra branch per 80 cells of region). The star they sized is what the
/// owner rejected four times, and `reveal_joints` records the measurement
/// that finally took it off the crush as well as off the blast. Anyone
/// restoring a walked crush needs all five numbers, not these two.
const CRACK_MIN_LENGTH: usize = 10;
const CRACK_MAX_LENGTH: usize = 55;

/// How sharply a fissure may turn at each cell, in radians.
///
/// A crack that does not wander is a ray, and a fan of rays reads as a
/// wheel rather than as broken rock. Enough to curve visibly over its
/// length, not so much that it doubles back on itself.
const CRACK_WANDER: f32 = 0.9;

/// How often a walker throws a side-branch, and how wide it leaves.
///
/// Forking is the difference between a crack and a hatch: a branch that
/// leaves at a wide angle and dies sooner than its parent reads as rock
/// splitting, where independent strokes crossing at a point read as the
/// scribble the owner rejected. The *pool* a walk may draw from is a
/// parameter (`FissureWalks::add_ray`); the blast's hybrid `crack_rays`
/// star is the only production caller left that fills it.
const CRACK_FORK_CHANCE: f32 = 0.10;
const CRACK_FORK_ANGLE: f32 = 0.8;

/// Chew the rock-facing boundary off a failing region before anything is
/// broken off it, so what comes away has a torn edge rather than a drawn
/// one.
///
/// # What this is fixing
///
/// Nothing upstream of here puts any noise into a region's outline, and
/// three separate mechanisms in `load.rs` actively make it *straight*:
/// `section_cells` is an axis-aligned run hard-cut at `MAX_SECTION`, the
/// supported-subtree walk breaks ties on a fixed neighbour order (which in
/// a homogeneous distance field is a 45-degree staircase), and a detached
/// piece is just the world geometry it was cut from. So a collapse came
/// down as a perfect column, or as a triangle with a razor hypotenuse, and
/// the owner's verdict off the prototype sheets was exactly that.
///
/// # Erosion only, and why that direction is not arbitrary
///
/// A cell is only ever *dropped* from the region, never added. Adding
/// would take down rock the load model never judged -- which is how a
/// small failure becomes a large one and how the dig cascade
/// (`Reports/next-session-handoff.md` §1b) is built. Dropping is safe in
/// the one direction that matters: a dropped cell stays exactly where it
/// is, still standing, still attached to whatever it was attached to, and
/// still gets rescheduled by the wavefront and by
/// `schedule_solid_neighbours` below. If it is genuinely unable to stand
/// it fails again a tick later and comes down then, which is the
/// progressive, ragged crumble the ethos asks for and the staged behaviour
/// `MAX_SUBTREE_CELLS`' own doc already promises.
///
/// Only cells with a **body-material neighbour outside the region** are
/// candidates: that is the boundary where the piece is tearing away from
/// rock that is staying, and it is the only boundary that reads as a torn
/// edge. A cell on the region's *free* boundary (open air beside it) is
/// already whatever shape the world gave it and holding it back would just
/// leave a floating skin behind.
///
/// # The size floor
///
/// Below `MIN_ERODIBLE_REGION` the region is left completely alone. A
/// three-cell failure eroded by 45% is a one-cell failure, and a failure
/// small enough to erode to nothing never comes down at all -- which turns
/// a graded outcome into an immunity, the exact shape of bug four earlier
/// support models were rejected for. It is also unnecessary: at that size
/// there is no straight line long enough to read as drawn.
///
/// Region connectivity may split, and that is fine -- `rigid::fracture`'s
/// partitioner already handles a disjoint set, and the leftovers become
/// separate later failures.
fn erode_failing_boundary(world: &World, region: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    if region.len() < MIN_ERODIBLE_REGION {
        return region;
    }
    let members: std::collections::HashSet<(i32, i32)> = region.iter().copied().collect();
    let kept: Vec<(i32, i32)> = region
        .iter()
        .copied()
        .filter(|&(x, y)| {
            // Position-keyed, like every other shape decision in this
            // file: no `world.rng` draw, so a replayed input sequence
            // cannot diverge here.
            if super::rng::jitter(x, y) >= BOUNDARY_EROSION {
                return true;
            }
            !NEIGHBOURS_4.iter().any(|&(dx, dy)| {
                let (nx, ny) = (x + dx, y + dy);
                !members.contains(&(nx, ny)) && is_body_material(world, world.get(nx, ny).material)
            })
        })
        .collect();
    // A region eroded to nothing is a failure that silently did not
    // happen. Cannot occur at `MIN_ERODIBLE_REGION` cells unless every one
    // of them is on the rock-facing boundary *and* every one draws under
    // the threshold, but "cannot occur" is how immunities get shipped.
    if kept.is_empty() {
        return region;
    }
    kept
}

/// Odds that a cell on a failing region's rock-facing boundary is left
/// standing this tick instead of coming away with the piece.
///
/// Around a half: high enough that no straight run of any length survives
/// intact, low enough that the piece still reads as the piece that failed
/// rather than as lace. What is dropped is not lost -- it re-fails on a
/// later tick if it genuinely cannot stand.
const BOUNDARY_EROSION: f32 = 0.45;

/// Smallest failing region worth roughening. See `erode_failing_boundary`
/// -- below this a region is small enough that erosion is a coin-flip on
/// whether it comes down at all, and there is no straight edge in it long
/// enough to look drawn anyway.
const MIN_ERODIBLE_REGION: usize = 12;

/// Cells one tick will hand to `rigid::fracture_failing_region`.
///
/// **Internal pacing, not a designer dial.** Nothing about the *outcome*
/// is chosen here -- the whole region still comes down, and a player
/// cannot see this number except as the collapse arriving over several
/// beats instead of one. It is deliberately a `const` rather than a
/// `Tuning` field for that reason: a knob invites tuning the look, and the
/// look is `fracture`'s business.
///
/// # Why there has to be one at all
///
/// Nothing upstream bounds what a single failure hands over.
/// `load::failing_region` returns the union of whole-column supported
/// subtrees for an `Overloaded` failure, and for `Unsupported` it runs
/// `detached_piece` out to `MAX_REGION_CELLS = 20,000` with no per-frame
/// budget of its own. `scene=ligament` measured a **4,420-cell region
/// fractured in one tick**, promoting 142 chunk bodies in a single call;
/// `Reports/explosion-stone-review.md` §1d measured 189 bodies and a
/// deterministic 264 ms frame from the same shape.
/// `Reports/fracture-mechanics-design.md` §3.4 asked for a per-frame cap
/// on fractures and never got one. This is it.
///
/// # Why this is not the forbidden size cap
///
/// `CLAUDE.md`: *"a size cap must bound work, never gate whether something
/// happens"* -- fracture once declined any region over its body-size cap
/// and fell through to per-cell dust, so the bigger the collapse the more
/// certain it dissolved. This bounds **cells per tick** and nothing else.
/// The remainder goes straight onto `World::staged_fractures`, which is
/// drained unconditionally, so it comes down behind the slice whatever
/// else happens; the staging is exactly what `load::MAX_SUBTREE_CELLS`'
/// own doc already promises ("a piece bigger than this simply comes down
/// in stages").
///
/// 1,000 because that is roughly where the measured cost of one
/// `fracture` call stops being invisible, and because the ligament's
/// 4,420 cells then read as four or five bites rather than one -- graded
/// beats binary, and a collapse that resolves over a second reads better
/// than one that resolves instantly.
const FRACTURE_CELLS_PER_TICK: usize = 1_000;

/// Split a failing region into the part that comes apart **this** tick and
/// the part that comes down behind it.
///
/// The slice is grown breadth-first from the cell that actually gave way,
/// so the collapse starts at the break and eats outward -- the neck first,
/// then the span hanging off it -- rather than at whichever end of the
/// region a flood happened to be seeded from.
///
/// # Order: erosion first, then slice
///
/// Deliberate, and the opposite order is wrong in a way that is easy to
/// miss. `erode_failing_boundary` roughens the region's *rock-facing*
/// boundary -- the seam where the piece tears away from rock that is
/// staying. Slicing first would hand erosion a slice whose boundary is
/// mostly the internal cut, so erosion would chew the cut instead of the
/// tear, and the cells it dropped would be cells that are coming down
/// anyway one tick later: the raggedness would land on an edge nobody ever
/// sees and not on the one that reads. Eroding the whole region once also
/// means every later stage inherits an already-torn rock-facing boundary,
/// which is the only boundary a stage has that the player can see -- the
/// cut itself is buried inside a mass that is entirely on its way down.
///
/// # The cut is jittered, for the same reason the tear is
///
/// A plain BFS ring boundary in homogeneous rock is an L1 diamond -- the
/// "perfect column or sharp triangle" the owner rejected off the prototype
/// sheets, re-manufactured by a brand new mechanism. The frontier is
/// therefore ordered on `distance + jitter(x, y)` rather than on distance
/// alone, which wobbles the cut by a couple of cells at no cost worth
/// measuring. Position-keyed like every other shape decision in this file:
/// no `world.rng` draw, so a replayed input sequence cannot diverge here.
///
/// `remainder` is empty whenever the region already fits in one tick,
/// which is the overwhelmingly common case and costs one length
/// comparison.
fn slice_failing_region(region: Vec<(i32, i32)>, broke_at: (i32, i32)) -> Sliced {
    if region.len() <= FRACTURE_CELLS_PER_TICK {
        return Sliced { slice: region, remainder: Vec::new() };
    }
    // `broke_at` is the ancestor the chain walk found over its limit, and
    // it is *not* guaranteed to be in the region -- an `Overloaded`
    // failure's region is the subtree it was holding up, and erosion above
    // may have dropped it in any case. Seed from the region cell nearest
    // it instead, which degrades gracefully to "the same cell" whenever it
    // is present.
    let mut left: std::collections::HashSet<(i32, i32)> = region.iter().copied().collect();
    let seed = *region
        .iter()
        .min_by_key(|&&(x, y)| ((x - broke_at.0).unsigned_abs() + (y - broke_at.1).unsigned_abs(), x, y))
        .expect("a region over the pacing cap is not empty");
    // Ordered by `(key, x, y)` so the pop order is a total order on
    // position and nothing here depends on hash iteration order --
    // determinism is required (`PLAN.md`), and a `HashSet` drain would
    // quietly break it.
    let mut frontier = std::collections::BinaryHeap::new();
    let key = |d: u32, x: i32, y: i32| d * JITTER_STEPS + (super::rng::jitter(x, y) * JITTER_STEPS as f32) as u32;
    left.remove(&seed);
    frontier.push(std::cmp::Reverse((key(0, seed.0, seed.1), seed.0, seed.1, 0u32)));
    let mut slice = Vec::with_capacity(FRACTURE_CELLS_PER_TICK);
    while let Some(std::cmp::Reverse((_, x, y, depth))) = frontier.pop() {
        slice.push((x, y));
        if slice.len() >= FRACTURE_CELLS_PER_TICK {
            break;
        }
        for (dx, dy) in NEIGHBOURS_4 {
            let next = (x + dx, y + dy);
            if left.remove(&next) {
                frontier.push(std::cmp::Reverse((key(depth + 1, next.0, next.1), next.0, next.1, depth + 1)));
            }
        }
    }
    // Anything still on the frontier was claimed out of `left` but never
    // popped, so it has to go back or it is silently deleted -- neither in
    // the slice nor in the remainder, which is material vanishing out of a
    // region the load model judged. The same trap `rigid::take_fragment`
    // documents at its own tail, and it is just as easy to walk into here.
    let mut remainder: Vec<(i32, i32)> = frontier.into_iter().map(|std::cmp::Reverse((_, x, y, _))| (x, y)).collect();
    remainder.extend(left.iter().copied());
    // Sorted for the same reason the heap carries coordinates: the
    // leftovers come off a `HashSet`, and their order reaches
    // `fracture`'s seed choice on the next slice.
    remainder.sort_unstable();
    Sliced { slice, remainder }
}

/// What one tick takes off a failing region, and what it leaves standing.
/// See `slice_failing_region`.
struct Sliced {
    /// Handed to `rigid::fracture_failing_region` now.
    slice: Vec<(i32, i32)>,
    /// Everything else, which comes down behind it — see `StagedFracture`.
    remainder: Vec<(i32, i32)>,
}

/// A failure the engine has already judged, part-way through coming down.
///
/// Not a queue of *questions* — a queue of **work**. Everything in `region`
/// has been through the load model once, inside one failure, and the only
/// reason it is still standing is `FRACTURE_CELLS_PER_TICK`. It is
/// deliberately not re-judged on the way back out: see the note at the call
/// site in `tick` for the two ways that stalled and stranded it.
pub struct StagedFracture {
    /// Everything still to come down, most recently sliced first.
    pub region: Vec<(i32, i32)>,
    /// Where the piece originally gave way — the impulse origin, and the
    /// point each further slice is grown from, so the collapse keeps eating
    /// outward from the break instead of restarting somewhere else.
    pub at: (i32, i32),
    /// Frame the next slice is due.
    pub next_frame: u64,
}

/// Take the next slice off the oldest staged fracture, if one is due.
///
/// Called once per frame from `scheduler::step`, and costs one `is_empty`
/// test in every frame where nothing is mid-collapse — which is nearly all
/// of them.
///
/// **Deliberately outside the site scheduler.** A staged slice is not a
/// question about a cell, so it has no business competing for
/// `MAX_SITES_PER_FRAME` or for `load::MAX_LOAD_CELLS_PER_FRAME` with the
/// checks that *are* questions — and it must not be starved by them, which
/// is exactly what happened when this was driven by rescheduled structural
/// checks (measured: `budget 0` from frame five onward and a collapse still
/// unfinished 1,200 frames later).
///
/// One slice per call and per `STRUCTURAL_TICK_INTERVAL` frames: fast
/// enough that a 4,400-cell overhang is down in well under a second, slow
/// enough that the stages read as stages. A collapse that arrives in one
/// frame reads as a glitch; the same collapse over half a second reads as
/// consequence (`Reports/explosion-stone-review.md` §3).
pub fn advance_staged_fractures(world: &mut World) {
    if world.staged_fractures.front().is_none_or(|s| world.frame < s.next_frame) {
        return;
    }
    let staged = world.staged_fractures.pop_front().expect("checked non-empty above");
    // What is left of it, which is not necessarily what was put here: a
    // neighbouring failure, a landing body or a later blast can have taken
    // some of these cells in the frames since. Anything that is no longer
    // body material is simply not there to break -- and note the filter is
    // on the *material now*, not on what was there when the failure was
    // judged, so a cell whose rock left and whose hole a landing chunk then
    // filled breaks as the new rock. That is the right answer for a
    // coordinate that was going to be air, and there is no cheap way to
    // tell the two apart: `Cell` carries no identity.
    let region: Vec<(i32, i32)> = staged.region.into_iter().filter(|&(x, y)| is_body_material(world, world.get(x, y).material)).collect();
    if region.is_empty() {
        return;
    }
    // The support forest this was derived from is about to change again --
    // same reason `tick` clears it before breaking anything.
    world.load_cache.clear();
    world.structural_failures.record_staged(region.len().min(FRACTURE_CELLS_PER_TICK));
    let Sliced { slice, remainder } = slice_failing_region(region, staged.at);
    // Same measurement as `tick`'s, one tick later: a paced slice is damage
    // arriving from a queue the reach never re-consults, which is precisely
    // the thing the containment number has to be able to see.
    record_damage_reach_over(world, &slice);
    if !super::rigid::fracture_failing_region(world, &slice, staged.at) {
        // Same grit-by-construction branch as `tick`'s, one tick later --
        // and it has to book the same counter, which it did not when
        // `FailureCounts::crumbled` arrived. That counter was written
        // against a build where `tick` held the *only* decline-to-grit
        // path; staging added a second one and the count was left behind,
        // so it under-read on precisely the large collapses it exists to
        // describe. Measured on `scene=room wall=8 dig=1`: 7,284 of 8,045
        // failed cells arrive here rather than through `tick`, and the
        // grit line reported 8. `slice`, not `region` -- the remainder is
        // re-judged on a later tick and would otherwise be counted twice.
        world.structural_failures.crumbled += 1;
        world.structural_failures.crumbled_cells += slice.len() as u32;
        let mut grit = 0usize;
        for &(fx, fy) in &slice {
            grit += usize::from(break_free(world, fx, fy));
        }
        world.structural_failures.record_shattered(grit);
    }
    if !remainder.is_empty() {
        world.staged_fractures.push_front(StagedFracture { region: remainder, at: staged.at, next_frame: world.frame + STRUCTURAL_TICK_INTERVAL });
    }
}

/// How finely `slice_failing_region` interleaves jitter with BFS depth.
/// One ring of depth is worth this many key steps, so a jitter draw can
/// move a cell by at most one ring either way -- enough to break the
/// diamond, not enough to let the slice grow holes.
const JITTER_STEPS: u32 = 16;

/// Retain only the cells of a failing region the current `chain_reach`
/// licenses.
///
/// A **clip**, not a veto: a failure that is licensed anywhere still
/// happens, it just cannot eat past the leash. See `tick`'s own note at the
/// call site for why this does not contradict the staging block's "a
/// remainder must not be able to lose".
///
/// At SPREAD the licence is universal, so the region is handed straight
/// back untouched -- no scan, no reallocation, and the shipped setting pays
/// nothing at all for the leash existing.
///
/// **Runs before `erode_failing_boundary`**, so the clipped edge is chewed
/// up like any other rock-facing boundary rather than reading as a knife
/// cut across the stone.
fn clip_region_to_licence(world: &World, region: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    if world.chain_reach == i32::MAX {
        return region;
    }
    let mut region = region;
    region.retain(|&(x, y)| world.within_disturbance(x, y));
    region
}

/// Record how far a consequence actually landed from the nearest *live*
/// disturbance -- the `FailureCounts::max_damage_reach` half of the
/// containment pair, and the only number in the census that can see the
/// artifact `chain_reach` exists to bound.
///
/// **Called only where something stops being what it was**: the region
/// handed to the fracturer, each paced slice, the organism path's
/// `break_free`, and (in `sever_joint`) the cells a crush fissures.
/// Scheduling a check, refusing one, and a landing body's own re-checks are
/// all *work* rather than damage and are deliberately not recorded -- mixing
/// them in would make the number unreadable, since a contained world is
/// full of checks that changed nothing.
///
/// One `record` per event rather than one per cell. The statistic is a max
/// either way; taking it locally keeps the per-cell cost to the ring scan
/// alone.
fn record_damage_reach_over(world: &mut World, cells: &[(i32, i32)]) {
    let mut furthest: Option<i32> = None;
    for &(x, y) in cells {
        if let Some(d) = world.distance_to_live_disturbance(x, y) {
            furthest = Some(furthest.map_or(d, |f: i32| f.max(d)));
        }
    }
    if let Some(d) = furthest {
        world.structural_failures.record_damage_reach(d);
    }
}

/// Convert one cell to its material's `breaks_into`, returning whether it
/// actually converted.
///
/// The return value is the grit half of the "did anything move" pair, and
/// it is a return value rather than a `record_shattered` call in here for
/// one reason: **two of this function's three callers are destruction and
/// the third is a tree dying.** The conversion is identical in all three --
/// same `breaks_into` lookup, same unattached result -- but a limb that
/// lost its anchor becoming deadwood is not rock coming apart, and it fires
/// on its own schedule all through any world with vegetation in it. Counted
/// here it would swamp the number on exactly the generated worlds the
/// counter exists to judge. So the two destruction callers record what this
/// tells them and the organism path deliberately drops it; see
/// `FailureCounts::shattered_cells`.
///
/// A cell whose material has no configured debris is left alone rather than
/// deleted, and reports `false`: counting a decline would make grit look
/// like it happened.
#[must_use]
fn break_free(world: &mut World, x: i32, y: i32) -> bool {
    // Resolved per cell rather than passed in by the caller, because a
    // failing region is no longer one cell and need not be one material --
    // a shelf can be part stone and part whatever was built onto it. A cell
    // whose material has no configured debris is left alone rather than
    // deleted: "not actually participating" beats silently destroying
    // content an author forgot to pair `breaks_into` with.
    let Some(into) = world.materials.get(world.get(x, y).material).breaks_into else {
        return false;
    };
    let shades = world.materials.get(into).palette.len().max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    let temp = world.get(x, y).temperature();
    // Deliberately *not* carrying `attached` across: whatever comes free is
    // no longer backed by the mass it broke out of. `Cell::new` starts
    // unattached, so this is the transition rather than an omission -- see
    // `FLAG_ATTACHED`, and note it is what turns digging into a cliff into
    // debris that actually falls.
    world.set(x, y, Cell::new(into, shade).with_temperature(temp));
    world.add_pressure_impulse(x, y, COLLAPSE_IMPULSE_RADIUS, COLLAPSE_IMPULSE_STRENGTH);
    true
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

/// Compute every eligible cell's distance to an anchor across the whole
/// world in one converged pass, writing each into `Cell::aux`.
///
/// This is what makes generated terrain *genuinely* structurally valid
/// rather than merely exempt from checking — `Reports/worldgen-design.md`
/// §6b ("the structural-integrity landmine"): before this, untouched
/// terrain kept `aux = 0`, "which is indistinguishable from 'anchored'…
/// at worldgen scale it is a landmine: the entire world becomes
/// structurally invalid but reading as anchored." Cells that genuinely
/// cannot reach an anchor come out of here at `u16::MAX`, which is honest,
/// instead of at 0, which was a lie that happened to look right.
///
/// # Why this is a direct pass and must not go through the scheduler
///
/// The obvious implementation — schedule a `StructuralCheck` on every
/// terrain cell and let the existing reactive relaxation converge — is
/// wrong twice over, and the second way is a visible bug rather than a
/// slowdown. `scheduler::step` processes at most `MAX_SITES_PER_FRAME`
/// (2000) sites per frame, so a world's worth of terrain spreads across
/// many frames; and during that partial convergence the count-to-infinity
/// dynamic described in the module doc is live. A cell whose true distance
/// is small can climb past its own span *before* the real anchor value
/// reaches it, break, and take its neighbours with it — terrain visibly
/// crumbling on the first frames of a fresh world, which is precisely the
/// "global collapse" §6b predicts. Converging first and writing once has
/// no transient state for that to happen in.
///
/// # Cost, as measured rather than as hoped
///
/// Paid once at generation and never per frame — reported by
/// `examples/ascii.rs`, which times it against the same terrain built
/// without this pass so the figure is attributed rather than asserted.
/// At 2048x640 -- the size the world shipped at when this was measured; it
/// has since grown to 8192x2560 and this has not been re-measured there --
/// measured with a probe splitting the two halves:
///
/// | | seeding scan | relaxation | total |
/// |---|---|---|---|
/// | via `World::get` | 193 ms | 386 ms | 579 ms |
/// | via the flat mirror | not split | not split | 175 ms |
///
/// The split is from a temporary probe and was only needed to decide what
/// to fix; the "after" figure is `examples/ascii.rs`'s, which is the number
/// of record. 512x320 came down the same way, 76.2 ms to 15.0 ms.
///
/// **An earlier version of this doc named the seeding scan as the thing
/// that dominates, and at 2048x640 that is the smaller half.** It was true
/// on a one-screen world, where most cells are sky and the search really
/// does run along free surfaces: cells inside bulk rock reach bedrock
/// through free downward steps and settle immediately. A world that is
/// mostly bulk rock inverts it — nearly every cell is relaxable, so the
/// search runs through volume. Acting on the old claim would have fixed
/// the third of the cost that was written down and left the rest.
///
/// Both halves were the same cost: `World::get` is a bounds check plus a
/// `HashMap<ChunkCoord, Chunk>` lookup *per read*, and each cell is read
/// five times over before the search starts (issue #5's "~164k hashed
/// `World::get` calls… index the chunk directly instead", in a new place).
/// So the world is mirrored into a flat `Vec<Cell>` filled by walking
/// chunks directly, the whole search runs on array indices, and the world
/// is touched again only to write results back.
///
/// This still scales with world *volume*, and that stops being true under
/// M10 streaming, where it becomes a per-chunk pass (§6b: "a cheap BFS from
/// bedrock, once per chunk", with anchor distance living on the coarse
/// layer) and the scan is bounded by a chunk. It no-ops on an unbounded
/// world rather than pretending to handle that case here.
pub fn compute_world_distances(world: &mut World) {
    let Some(bounds) = world.bounds() else {
        return; // unbounded (M10) -- see the doc above
    };

    // A flat mirror of the world, filled by walking chunks directly.
    //
    // Both loops below used `World::get` per cell, which is a bounds check
    // plus a `HashMap<ChunkCoord, Chunk>` lookup *per read*, and each cell
    // is read five times over (itself plus four neighbours) before the
    // search even starts. That was affordable while the world was one
    // screen. Measured at 2048x640 -- the size the world shipped at then,
    // since grown to 8192x2560 and not re-measured there -- with the probe
    // splitting the two halves:
    //
    //   seeding scan   193 ms
    //   relaxation     386 ms
    //
    // Note which one is bigger, because the doc above used to claim the
    // opposite. The seeding scan dominates on a *small* world, where most
    // cells are sky; on a world that is mostly bulk rock, every solid cell
    // is relaxable and the search runs through volume rather than along
    // free surfaces, so the relaxation wins. Optimising only the scan --
    // which is the fix the doc named -- would have bought a third of the
    // problem and been reported as the whole of it.
    //
    // So the mirror serves both. Everything either loop needs is per-cell
    // (material, organism id, the two crack bits, aux), so a `Vec<Cell>`
    // indexed by position answers every read with a bounds-free array
    // index, and the world is touched again only to write results back.
    let w = (bounds.max_x - bounds.min_x + 1) as usize;
    let h = (bounds.max_y - bounds.min_y + 1) as usize;
    let idx = |x: i32, y: i32| (y - bounds.min_y) as usize * w + (x - bounds.min_x) as usize;
    let inside = |x: i32, y: i32| x >= bounds.min_x && x <= bounds.max_x && y >= bounds.min_y && y <= bounds.max_y;

    // **Three bytes per cell, not twelve.** The mirror was a `Vec<Cell>`,
    // which is the whole 12-byte cell -- temperature, burn timer, shade and
    // all -- and at the sizes this now has to serve that is 240 MiB of
    // transient allocation on top of the world's own 240 MiB, which is most
    // of why peak RSS during generation is roughly double the steady grid
    // (measured: 539 MiB at 8192x2560).
    //
    // Everything the two loops below actually read is: the material (for
    // `relaxable`, for the bedrock anchor test, and to price a step), the
    // two crack bits, and whether an organism owns the cell. That is a
    // `u16` and three bits. Splitting it also makes the hot inner loop
    // touch a quarter of the cache lines it used to.
    let mut mat = vec![material::EMPTY.0; w * h];
    let mut bits = vec![0u8; w * h];
    for chunk in world.chunks() {
        let (ox, oy) = chunk.coord.origin();
        for ly in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let (x, y) = (ox + lx, oy + ly);
                // A chunk may hang over the world edge; the mirror is
                // exactly the bounds, so the overhang is dropped rather
                // than wrapped into the opposite side.
                if inside(x, y) {
                    let c = chunk.get_world(x, y);
                    let i = idx(x, y);
                    mat[i] = c.material.0;
                    bits[i] = u8::from(c.crack_right())
                        | (u8::from(c.crack_down()) << 1)
                        | (u8::from(c.organism_id() != 0) << 2);
                }
            }
        }
    }
    const CRACK_RIGHT: u8 = 1;
    const CRACK_DOWN: u8 = 2;
    const OWNED: u8 = 4;

    // `is_relaxable` and the support costs both go through
    // `world.materials`, which is borrowed immutably for the whole search
    // while `world` itself must stay writable at the end. Resolved by
    // answering both questions up front, per material id rather than per
    // cell: there are a handful of materials and a million cells.
    let material_count = world.materials.len();
    let body: Vec<bool> = (0..material_count).map(|m| is_body_material(world, MaterialId(m as u16))).collect();
    let costs: Vec<(u16, u16, u16)> = (0..material_count)
        .map(|m| {
            let mat = world.materials.get(MaterialId(m as u16));
            (mat.support_cost_below, mat.support_cost_above, mat.support_cost_beside)
        })
        .collect();
    let relaxable = |i: usize| {
        body.get(mat[i] as usize).copied().unwrap_or(false) && bits[i] & OWNED == 0
    };

    // Seed: anchors at 0, everything else at "unreachable" so a cell the
    // search never reaches ends up honestly unsupported rather than
    // accidentally reading as anchored.
    let mut dist = vec![u16::MAX; w * h];
    // **A dial queue, not a binary heap.** Every step cost here is a
    // material's `support_cost_*`, and those are 1..5 across the whole
    // registry (default 1). Dijkstra over small integer weights does not
    // need a comparison heap: buckets indexed by distance modulo
    // `max_step + 1` pop in nondecreasing order for free, which turns
    // O(E log V) into O(E) and removes the per-push allocation entirely.
    //
    // Determinism is unaffected and does not need the heap's old
    // `(distance, x, y)` tiebreak: a shortest-path distance is a unique
    // minimum, so the order cells are *popped* in cannot change the `dist`
    // array they produce. The tiebreak was insurance against a
    // `BinaryHeap` whose order among equals is unspecified; a bucket has no
    // such freedom to constrain.
    let max_step = costs.iter().map(|&(b, a, s)| b.max(a).max(s)).max().unwrap_or(1).max(1) as usize;
    let ring = max_step + 1;
    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); ring];
    let mut queued = 0usize;
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            if !relaxable(idx(x, y)) {
                continue;
            }
            // **Outside the world counts as bedrock.** `World::get` returns
            // `Cell::OUT_OF_BOUNDS`, whose material *is* `BEDROCK`, so the
            // world edge anchors everything against it -- the same rule that
            // makes the edge a wall the gnome can stand on. The mirror has
            // to reproduce that rather than treat off-world as nothing: a
            // first version guarded these reads with a bounds check, which
            // un-anchored every cell touching the edge and took six load
            // tests down with it.
            let anchored = NEIGHBOURS_4.iter().any(|&(dx, dy)| {
                let (nx, ny) = (x + dx, y + dy);
                !inside(nx, ny) || mat[idx(nx, ny)] == material::BEDROCK.0
            });
            if anchored {
                dist[idx(x, y)] = 0;
                buckets[0].push(idx(x, y) as u32);
                queued += 1;
            }
        }
    }

    // Relax, popping buckets in nondecreasing distance. `level` is the
    // distance currently being drained; a cell reached with a stale (larger)
    // distance is skipped exactly as the heap version skipped a superseded
    // entry.
    let mut level = 0u16;
    while queued > 0 {
        let slot = level as usize % ring;
        while let Some(packed) = buckets[slot].pop() {
            queued -= 1;
            let i = packed as usize;
            if dist[i] != level {
                continue; // superseded by a shorter path already processed
            }
            let (x, y) = (bounds.min_x + (i % w) as i32, bounds.min_y + (i / w) as i32);
            for (dx, dy) in NEIGHBOURS_4 {
                let (nx, ny) = (x + dx, y + dy);
                if !inside(nx, ny) {
                    continue;
                }
                let ni = idx(nx, ny);
                // `edge_is_cracked` against the mirror. Each edge is owned by
                // exactly one of the two cells it separates, so reaching left
                // or up means asking the *neighbour* about its own right or
                // down edge -- see that function, which stays as the
                // world-reading version everything outside this pass uses.
                let cracked = match (dx, dy) {
                    (1, 0) => bits[i] & CRACK_RIGHT != 0,
                    (-1, 0) => bits[idx(nx, y)] & CRACK_RIGHT != 0,
                    (0, 1) => bits[i] & CRACK_DOWN != 0,
                    (0, -1) => bits[idx(x, ny)] & CRACK_DOWN != 0,
                    _ => false,
                };
                if cracked || !relaxable(ni) {
                    continue;
                }
                // The cost is paid by the cell being *supported* -- so it
                // reads the neighbour's own material, and the direction is
                // from the neighbour back to (x, y), which is the negation
                // of the offset used to reach it. `dy == -1` means (x, y)
                // sits below the neighbour, i.e. the neighbour is standing
                // on it. Getting this backwards would silently price towers
                // as cantilevers.
                let (below, above, beside) = costs[mat[ni] as usize];
                let step = match dy {
                    -1 => below,
                    1 => above,
                    _ => beside,
                };
                let candidate = level.saturating_add(step);
                if candidate < dist[ni] {
                    dist[ni] = candidate;
                    buckets[candidate as usize % ring].push(ni as u32);
                    queued += 1;
                }
            }
        }
        if level == u16::MAX {
            break;
        }
        level += 1;
    }

    // Write back, only where the answer differs from what the cell already
    // holds. On generated terrain that is nearly every solid cell, so this
    // is not an optimisation so much as an assurance that a cell nothing
    // decided about is left exactly as it was -- including its dirty state,
    // which `World::set` is what maintains.
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            let i = idx(x, y);
            if !relaxable(i) {
                continue;
            }
            let d = dist[i];
            // The mirror no longer carries the whole cell, so the write-back
            // reads the live one. It is one `World::get` per *relaxable*
            // cell, paid only where something is about to be written anyway.
            let cell = world.get(x, y);
            if cell.aux() != d {
                world.set(x, y, cell.with_aux(d));
            }
        }
    }
}

/// Re-converge distances over `region` in one pass, seeded from the
/// already-correct values around it.
///
/// # Why a stroke needs this and a reactive relaxation is not enough
///
/// `tick` relaxes one cell per scheduled check, and reschedules its
/// neighbours `STRUCTURAL_TICK_INTERVAL` frames later — so a wavefront
/// advances roughly one cell per 5 frames. That is the right shape for a
/// *disturbance*, which is local and whose consequences should arrive
/// progressively. It is the wrong shape for material appearing from
/// nothing: a freshly painted cell starts at `aux = 0`, which reads as
/// anchored, and the true distance has to climb from there one round at a
/// time. A 192-cell column needs ~192 rounds — over fifteen seconds at
/// 60 Hz — during which the structure is either wrongly immune or
/// half-converged, and *then* it decides whether to fall.
///
/// That is the shape of "I built a thing and it collapsed ten seconds
/// later", which is the single most-reported complaint about building
/// here, and it is a scheduling artifact rather than anything the load
/// model believes.
///
/// Generated terrain never had this problem because
/// `compute_world_distances` converges it in one pass before anything
/// looks at it. This gives the brush the same treatment, scoped to what
/// the stroke touched.
///
/// # Why the seeding is not the same as the world pass
///
/// `compute_world_distances` can start every cell at "unreachable" because
/// it is computing the whole world. Here the rock *outside* `region`
/// already holds correct values, and they are the boundary condition: the
/// heap is seeded both with anchors inside the region and with every
/// already-known cell just outside it, so a stroke against a cliff inherits
/// the cliff's distances instead of rediscovering them. Cells inside start
/// at `u16::MAX`, which is honest — a cell the search never reaches is
/// genuinely unsupported rather than accidentally reading as anchored,
/// which is `Reports/worldgen-design.md` §6b's landmine in miniature.
pub fn relax_region(world: &mut World, region: Rect) {
    let mut heap: BinaryHeap<Reverse<(u16, i32, i32)>> = BinaryHeap::new();
    for y in region.min_y..=region.max_y {
        for x in region.min_x..=region.max_x {
            let cell = world.get(x, y);
            if !is_relaxable(world, cell) {
                continue;
            }
            // **Prototype switch, not a shipping one** (`SETTLE_GROUND=0`).
            // Three functions in this module answer "what anchors a cell"
            // three different ways, and the difference decides whether the
            // converged pass in `explosion.rs` is a fix or an immunity:
            //
            //   compute_world_distances   bedrock and the world edge, only
            //   relax_region (here)       ...plus `is_resting_on_ground`, eagerly
            //   tick                      ...but only as a *last resort*
            //
            // `tick`'s own comment calls that last-resort rule "the whole of
            // the dig cascade": rooting eagerly on powder makes a cell a load
            // sink, which is "a sprinkle of sand under a beam holds the beam
            // up". Over a brush stroke the discrepancy is small. Over a blast
            // box eight times the charge it may not be, and a queue that goes
            // quiet because everything got rooted at 0 is indistinguishable
            // from one that goes quiet because it converged. Setting this to
            // 0 reproduces `compute_world_distances`' rule and separates them.
            let ground_anchors = {
                use std::sync::OnceLock;
                static ON: OnceLock<bool> = OnceLock::new();
                *ON.get_or_init(|| std::env::var("SETTLE_GROUND").map_or(true, |v| v != "0"))
            };
            let anchored = NEIGHBOURS_4.iter().any(|&(dx, dy)| world.get(x + dx, y + dy).material == material::BEDROCK)
                || (ground_anchors && is_resting_on_ground(world, x, y));
            let distance = if anchored { 0 } else { u16::MAX };
            world.set(x, y, cell.with_aux(distance));
            if anchored {
                heap.push(Reverse((0, x, y)));
            }
        }
    }
    // The boundary condition: everything just outside the region keeps the
    // distance it already had, and seeds the search inward.
    for y in (region.min_y - 1)..=(region.max_y + 1) {
        for x in (region.min_x - 1)..=(region.max_x + 1) {
            if region.contains(x, y) {
                continue;
            }
            let cell = world.get(x, y);
            if is_relaxable(world, cell) && cell.aux() < u16::MAX {
                heap.push(Reverse((cell.aux(), x, y)));
            }
        }
    }

    while let Some(Reverse((distance, x, y))) = heap.pop() {
        if world.get(x, y).aux() != distance {
            continue; // superseded by a shorter path already processed
        }
        for (dx, dy) in NEIGHBOURS_4 {
            if edge_is_cracked(world, x, y, dx, dy) {
                continue;
            }
            let (nx, ny) = (x + dx, y + dy);
            // Only ever writes *inside* the region -- the values outside are
            // the boundary condition and must not be disturbed by a stroke
            // that did not touch them.
            if !region.contains(nx, ny) {
                continue;
            }
            let neighbour = world.get(nx, ny);
            if !is_relaxable(world, neighbour) {
                continue;
            }
            let step = {
                let m = world.materials.get(neighbour.material);
                match dy {
                    -1 => m.support_cost_below,
                    1 => m.support_cost_above,
                    _ => m.support_cost_beside,
                }
            };
            let candidate = distance.saturating_add(step);
            if candidate < neighbour.aux() {
                world.set(nx, ny, neighbour.with_aux(candidate));
                heap.push(Reverse((candidate, nx, ny)));
            }
        }
    }
}

/// Whether this cell's `aux` is a structural distance that
/// `compute_world_distances` may read and write.
///
/// Organism-owned cells are excluded outright: once `organism_id != 0`,
/// `aux` holds a cell-type tag and a resource scalar
/// (`Reports/organism-substrate-design.md` §2), and writing a distance over
/// it would silently corrupt that encoding — the same reason `tick` routes
/// them to `organism_structural_tick` instead of relaxing them in place.
fn is_relaxable(world: &World, cell: Cell) -> bool {
    is_body_material(world, cell.material) && cell.organism_id() == 0
}

// Section depth and crack weakening used to live here, as multipliers on
// `max_unsupported_span`. They are not deleted -- they moved to `load.rs`
// and became multipliers on *capacity*, which is what they were always
// reaching for. Section depth also changed shape on the way: it is squared
// (bending capacity grows with the square of depth, the dominant term in a
// real beam) and is measured perpendicular to wherever the support is
// coming from rather than always vertically. See `load::capacity`.

/// Whether a fracture separates `(x, y)` from its neighbour at `(dx, dy)`.
///
/// The single rule the whole crack mechanic rests on: support does not cross
/// a crack. Nothing else has to detect "a fissure has gone all the way
/// around this piece" — once it has, the piece cannot reach an anchor
/// through any uncracked path, its distance runs away, and the existing
/// failure machinery breaks it off. A crack that only partly encircles
/// something changes nothing structurally, which is right: a scored face is
/// still a face.
///
/// Each edge is owned by exactly one of the two cells it separates (see
/// `FLAG_CRACK_RIGHT`), so reaching left or up means asking the *neighbour*
/// about its own right or down edge.
pub fn edge_is_cracked(world: &World, x: i32, y: i32, dx: i32, dy: i32) -> bool {
    match (dx, dy) {
        (1, 0) => world.get(x, y).crack_right(),
        (-1, 0) => world.get(x - 1, y).crack_right(),
        (0, 1) => world.get(x, y).crack_down(),
        (0, -1) => world.get(x, y - 1).crack_down(),
        _ => false,
    }
}

/// Strip background attachment from the material newly exposed by removing
/// the cell at `(x, y)`.
///
/// This is the "background becomes ground" transition, and without it
/// mining produces nothing at all: terrain declares itself attached
/// (`Cell::attached`) and an attached cell anchors outright, so carving into
/// a cliff just deletes cells while everything around them stays
/// permanently held. Nothing can ever break off, because nothing is ever
/// unheld.
///
/// So cutting rock costs its neighbours their backing. A cell that has just
/// been exposed to open space stops being part of the mass and becomes
/// ordinary foreground material, which then has to earn its support through
/// a real path like anything else. Rock with solid ground still behind it
/// simply relaxes to a distance of 1 from the attached material next to it
/// and stays exactly where it is — so a tunnel lining does not sag — while
/// rock left hanging over a void finds no path, exceeds its span, and comes
/// down. That difference is the whole mechanic, and it needs no radius, no
/// falloff and no separate stability pass.
///
/// Deliberately only the 4-neighbours, evaluated per removed cell rather
/// than as a disc around a brush stroke. It is O(1) per cell erased, it
/// tracks an arbitrarily-shaped excavation exactly, and it cannot detach
/// material the player never actually reached.
///
/// Attachment is only ever lost, never regained — see `FLAG_ATTACHED`. A
/// weathered dig face staying foreground afterwards is correct: it has been
/// broken once already.
pub fn detach_exposed_neighbours(world: &mut World, x: i32, y: i32) {
    detach_around(world, x, y, DETACH_DEPTH);
}

/// Strip background attachment from every cell within `r` of `(x, y)`.
///
/// Split out of `detach_exposed_neighbours` because *removing* material is
/// no longer the only thing that loosens rock. **Fracturing it does too**,
/// and that is what `rigid::score_cracks` calls this for: a fissure is the
/// plane along which rock has parted company with the mass behind it, so
/// scored rock stops claiming to be braced by material out of plane.
///
/// Reported from play, and this is the fix for it: *"when background
/// objects break to become foreground it is too conservative — it basically
/// only breaks really close to the strike"*, alongside *"when I shatter
/// background objects it looks like I have shattered through but it doesn't
/// drop."* Both were the same cause. A blow only ever loosened the rock it
/// physically bit out (a radius-7 swing reaches 4 cells), while its cracks
/// ran three times further and changed nothing about attachment — so the
/// visible fissures were decoration, and the shelf they crossed kept a
/// twelvefold capacity bonus it had visibly stopped deserving.
///
/// Measured on `scene=worked`: without this the six blows shed the shelf's
/// lower ten rows and left a 3-deep skin standing 160 cells out, at 92% of
/// its capacity — right at the margin, which is why it read as "shattered
/// through but still up".
fn detach_around(world: &mut World, x: i32, y: i32, r: i32) {
    for dy in -r..=r {
        for dx in -r..=r {
            let (nx, ny) = (x + dx, y + dy);
            let cell = world.get(nx, ny);
            if !cell.attached() || !is_body_material(world, cell.material) {
                continue;
            }
            world.set(nx, ny, cell.with_attached(false));
            world.schedule_structural_check_around(nx, ny);
        }
    }
}

/// How far a *fracture* loosens rock either side of itself, in cells.
///
/// Smaller than `DETACH_DEPTH`: a cut face has had material physically
/// taken off it, while a crack has only parted — but not zero, for exactly
/// the reason `DETACH_DEPTH` is not 1. A one-cell band around a fissure
/// makes everything that later breaks away a one-cell sheet, and pieces can
/// only be as thick as the loosened rock they came from.
const CRACK_DETACH_DEPTH: i32 = 1;

/// Loosen the rock around a fracture at `(x, y)`. See `detach_around`.
pub fn detach_around_crack(world: &mut World, x: i32, y: i32) {
    detach_around(world, x, y, CRACK_DETACH_DEPTH);
}

/// How deep into the rock a cut loosens material, in cells.
///
/// Deliberately more than 1. At a depth of one this stripped a single-cell
/// *skin* off the dig face, so everything that subsequently broke away was a
/// one-cell sheet — reported from play as debris that "look like thin
/// individual pixel lines" rather than chunks. Pieces can only be as thick
/// as the loosened rock they are cut from.
///
/// `pub(crate)` so `World::paint_capsule_as` can size the disturbance it
/// records off the band its own erase actually loosened, rather than
/// restating the number and drifting from it.
pub(crate) const DETACH_DEPTH: i32 = 3;

/// Whether `(x, y)` is simply sitting on top of something that can hold it.
///
/// The relaxation otherwise only accepts `Solid`/`Plant` neighbours as
/// support, which is right for spanning a gap and badly wrong for a rock
/// lying on the ground: a chunk that landed on its own rubble had *no* valid
/// support neighbour at all, so its distance ran away to the maximum,
/// exceeded its span, and it shattered where it lay. Reported from play as
/// debris that "when they hit the ground they turn to powder."
///
/// **Powder specifically**, and this is the whole subtlety. Solid support is
/// already handled properly by the relaxation, which asks whether the cell
/// below can *itself* reach an anchor. Accepting solid here as well was a
/// bug with a long history in this file: a blob floating in mid-air rests on
/// its own lower cells, so every cell of it would declare itself grounded
/// and the blob would hang there forever — the same self-consistent fixed
/// point that made two earlier support models fail. Powder is the one case
/// the relaxation genuinely cannot see, because a `Powder` cell carries no
/// distance to consult.
///
/// Treating a granular pile as ground is safe in a way that treating solid
/// as ground is not: powder is under the CA sweep's control, so if it flows
/// out from underneath, that write dirties the chunk and whatever was
/// sitting on it gets re-examined.
///
/// **Gases stay excluded; `Liquid` is now excluded only for material that
/// does not claim to float.** The original rule read "floating is buoyancy,
/// not support, and nothing here models it", which was true and free while
/// nothing floated. Ice does, and the claim is a bit on the material rather
/// than a shape this function could infer — see `MaterialDef::floats` for
/// what went wrong without it and why the flag is opt-in. The safety
/// argument for powder carries over unchanged and is the reason this is a
/// small change rather than a new mechanism: a `Liquid` is under the CA
/// sweep's control too, so if it flows out from underneath, that write
/// dirties the chunk and whatever was floating on it is re-examined.
///
/// Only the cell *directly below*: this answers "is it standing on the
/// ground", not "is there a second, weaker way to span a gap sideways".
/// **Delegates to `load::rests_on_ground`, and used to be a second copy of
/// it.** Two modules answering the same question separately is the shape
/// `CLAUDE.md` warns about, and it went wrong the moment one of them
/// learned something: `load.rs` grew a rule about grains swallowed inside a
/// piece (see `load::grain_is_footing`, and the raft it found floating on a
/// pond) and this one would have gone on rooting chains on them.
fn is_resting_on_ground(world: &World, x: i32, y: i32) -> bool {
    super::load::rests_on_ground(world, x, y)
}

/// Whether `material` participates in the structural system at all —
/// `Solid` (M17) or `Plant` (architecture item 9, trees). Named after the
/// same helper `rigid.rs` (M8) already has for exactly the same "which
/// kinds of cell count as body, not air or clutter" question, since it's
/// the identical question asked for a different purpose.
pub fn is_body_material(world: &World, material: MaterialId) -> bool {
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
        // Deduped inside `schedule_active_site` itself, not here -- see
        // that method's own doc for why the check needed to move there
        // (this was its first home, but `fire.rs`'s burnout fan-out
        // reaches the heap through a different path that skipped it).
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

    /// Record that something actually happened at `(x, y)` — a blow, a
    /// cut, a blast. See `World::chain_reach`: with a reach set, failures
    /// are only permitted near one of these, so anything that should be
    /// able to *cause* a collapse has to report itself here.
    ///
    /// Deliberately coarse. The ring holds only the most recent few,
    /// because anything older has fallen outside `chain_window`.
    /// `extent` is the **outer limit of the damage this verb does itself**,
    /// in cells from `(x, y)`: the wound, not the chain. See
    /// `Disturbance::extent`, and note the signature deliberately has no
    /// default -- a volume verb that quietly recorded a point disturbance
    /// is the bug this argument exists to make impossible.
    ///
    /// # Why a new record coalesces into an overlapping one
    ///
    /// The doc above used to end *"and a player cannot disturb dozens of
    /// places in the same second"*. True of a player, and false of the
    /// world once the verbs that are not the player started reporting
    /// themselves: a burnout removes structural material and a phase
    /// change adds it, both per cell and both inside the sweep, so a
    /// burning wood writes disturbances by the hundred. Unbounded pushes
    /// would evict the player's own dig within a frame of a fire starting,
    /// and the licence would track whatever burned most recently rather
    /// than anything anyone did -- which is precisely the delayed cave-in
    /// `chain_window`'s ten seconds exist to preserve.
    ///
    /// So a record close enough to a live one **refreshes it instead of
    /// taking a slot**, widening its extent if this verb's wound is
    /// bigger. The ring then holds disturbances proportional to the
    /// *spatial extent* of what is happening rather than to the number of
    /// cells involved, which is the quantity `chain_reach` is about.
    ///
    /// # Why the merge radius is half the reach and not the reach
    ///
    /// A coalesced record keeps the **older** point, so the box it
    /// licenses is off-centre from the event that refreshed it by up to
    /// the merge radius. Merging at the full `chain_reach` would let that
    /// offset reach `chain_reach` too, and `TIGHT` would quietly license
    /// failures 32 cells from a blow rather than 16. Half bounds the
    /// effective reach at 1.5x the setting, which is slop a coarse policy
    /// can carry.
    ///
    /// Keeping the older point rather than moving it to the new one is the
    /// deliberate direction of that error: over-licensing shows up as a
    /// collapse reaching slightly further than the label promises, and
    /// under-licensing shows up as *"I hit it and nothing happened"* --
    /// which is the failure this whole leash has to avoid being.
    ///
    /// Coalescing keys on `chain_reach` alone and not on `licence_radius`:
    /// the question is whether two *records* are near enough to be one
    /// record, which is about where they happened, not about how far each
    /// licenses. Two big-extent blasts a long way apart are two events.
    pub fn record_disturbance(&mut self, x: i32, y: i32, extent: i32) {
        let frame = self.frame;
        let extent = extent.max(0);
        let reach = self.chain_reach;
        if reach != i32::MAX {
            let merge = reach / 2;
            if let Some(d) = self
                .disturbances
                .iter_mut()
                .find(|d| (x - d.x).abs().max((y - d.y).abs()) <= merge)
            {
                d.frame = frame;
                // The wider wound wins. A small record must never shrink
                // the licence a big one already established, or a single
                // cell of quench beside a blast would quietly re-leash it.
                d.extent = d.extent.max(extent);
                return;
            }
        }
        self.disturbances.push_back(Disturbance { x, y, extent, frame });
        while self.disturbances.len() > MAX_DISTURBANCES {
            self.disturbances.pop_front();
        }
    }

    /// How far from a disturbance of this `extent` a consequence may land,
    /// in cells — `chain_reach` measured from the **edge of the wound**
    /// rather than from its centre.
    ///
    /// # Why the wound has to be inside its own licence
    ///
    /// A blast records one disturbance, at its epicentre, and its own
    /// damage is a *volume*: the crater alone is 41 cells across at radius
    /// 20 and the joint fabric reaches `radius * joint_reach` (2.4) past
    /// that, further still on an unconfined shot. A reach of 16 measured
    /// from the epicentre is inside the charge's own hole. Leashing on that
    /// makes TIGHT crack rock and never break it, which is the owner's
    /// *original* complaint reintroduced by the fix for a different one.
    ///
    /// So "what you struck" scales with the tool, and LOCAL/TIGHT leash
    /// **the chain beyond the wound** rather than the wound itself.
    ///
    /// # NONE is exempt, and that is deliberate
    ///
    /// At `chain_reach == 0` the extent does not apply and the behaviour is
    /// exactly what it was: no failure is ever licensed anywhere. NONE is
    /// the hard off switch, it is the owner's working escape hatch, and it
    /// is the one setting that currently does what it says
    /// (`wiki/structural-collapse.md`: "only what you struck is ever
    /// destroyed. Nothing collapses afterwards, at all"). This commit must
    /// not quietly redefine it.
    ///
    /// `saturating_add` because SPREAD is `i32::MAX`.
    fn licence_radius(&self, extent: i32) -> i32 {
        if self.chain_reach == 0 {
            return 0;
        }
        self.chain_reach.saturating_add(extent)
    }

    /// Whether a failure at `(x, y)` is close enough in space and recent
    /// enough in time to something that was disturbed.
    ///
    /// Chebyshev distance rather than Euclidean: the question is "is this
    /// within the box around what I hit", the cheap answer is the right
    /// shape, and a circle would only make the corners of a blast behave
    /// differently from its sides for no reason anyone could see.
    pub fn within_disturbance(&self, x: i32, y: i32) -> bool {
        if self.chain_reach == i32::MAX {
            return true;
        }
        self.disturbances.iter().any(|d| {
            self.frame.saturating_sub(d.frame) <= self.chain_window && (x - d.x).abs().max((y - d.y).abs()) <= self.licence_radius(d.extent)
        })
    }

    /// Distance from `(x, y)` to the nearest disturbance still inside
    /// `chain_window`, or `None` if there is none.
    ///
    /// **The measurement twin of `within_disturbance`** -- same ring, same
    /// age test, same Chebyshev metric -- so "how far did damage travel" is
    /// reported in the units the `F9` setting is written in. A metric in
    /// different units from the gate cannot be compared against the gate's
    /// own number, which is the entire point of having it.
    ///
    /// `None` rather than `0` when nothing is live, and the difference
    /// matters: `0` is "perfectly contained", so a world with no live
    /// disturbance at all would report the best containment there is while
    /// eating itself. `CLAUDE.md`: ask what the metric says when nothing is
    /// wrong. A stale disturbance licenses nothing and must not flatter it
    /// either, which is why the age test is here and not only in the gate.
    pub fn distance_to_live_disturbance(&self, x: i32, y: i32) -> Option<i32> {
        self.disturbances
            .iter()
            .filter(|d| self.frame.saturating_sub(d.frame) <= self.chain_window)
            // **Measured from the edge of the wound, not from its centre**,
            // for exactly the reason `licence_radius` adds the extent there:
            // otherwise the metric and the gate stop being in the same units
            // and the number stops meaning "how far past the licence did
            // damage travel". A cell inside the wound reads `0`.
            .map(|d| ((x - d.x).abs().max((y - d.y).abs()) - d.extent).max(0))
            .min()
    }

    /// How much further a consequence standing at `(x, y)` may reach before
    /// it leaves the licence, or `None` when nothing bounds it (SPREAD).
    ///
    /// The bound a *radius* needs, as opposed to the yes/no
    /// `within_disturbance` a cell needs. `crush_in_place`'s section disc is
    /// the one consequence that is sized rather than enumerated, and a
    /// crush that writes cracks outside the leash is the same leak one
    /// channel over.
    ///
    /// `0` (rather than `None`) when there is a live disturbance but no
    /// headroom left: that is "you may not reach at all", which is a real
    /// answer and not the absence of one.
    pub fn licence_headroom(&self, x: i32, y: i32) -> Option<i32> {
        if self.chain_reach == i32::MAX {
            return None;
        }
        self.disturbances
            .iter()
            .filter(|d| self.frame.saturating_sub(d.frame) <= self.chain_window)
            .map(|d| (self.licence_radius(d.extent) - (x - d.x).abs().max((y - d.y).abs())).max(0))
            .max()
            .or(Some(0))
    }

    /// Drop staged fracture work the current `chain_reach` no longer
    /// licenses.
    ///
    /// The staged queue is deliberately ungated once a failure has been
    /// judged (`structural::advance_staged_fractures` -- a remainder must
    /// not be able to lose to the budget or to the load model and leave
    /// rock hanging in open air). A **player changing the setting** is the
    /// one exception, and it has to be, or `F9` reads as doing nothing: the
    /// aftermath the player is trying to stop keeps arriving at
    /// `FRACTURE_CELLS_PER_TICK` cells a tick from a queue the new setting
    /// never sees.
    ///
    /// Only ever called when the reach *tightens* (`App::cycle_chain_mode`),
    /// so it can only ever leave more standing than the player asked for,
    /// never less. At `NONE` it empties the queue outright, which is exactly
    /// what NONE advertises -- `wiki/structural-collapse.md`: "only what you
    /// struck is ever destroyed. Nothing collapses afterwards, at all."
    pub fn relicense_staged_fractures(&mut self) {
        let mut staged = std::mem::take(&mut self.staged_fractures);
        for entry in staged.iter_mut() {
            entry.region.retain(|&(x, y)| self.within_disturbance(x, y));
        }
        staged.retain(|entry| !entry.region.is_empty());
        self.staged_fractures = staged;
    }
}

/// Something that actually happened to the world, and how big it was.
///
/// # Why an extent, and why it is not optional
///
/// `chain_reach` is a leash on **consequences**, and a leash measured from
/// a single point cannot tell the wound from the chain. A blast records one
/// disturbance at its epicentre while its own damage is a volume: at radius
/// 20 the crater is 41 cells across and the joint fabric reaches
/// `radius * joint_reach` past it, further on an unconfined shot. TIGHT's
/// 16 cells is inside the charge's own hole, so leashing on the point alone
/// makes TIGHT crack rock and never break it -- the owner's original
/// complaint, reintroduced by the fix for a different one.
///
/// It also closes a leak found separately: a blast strips `attached` (a 12x
/// capacity bonus, never regained) out to ~97 cells while recording a
/// licence of 16, so at TIGHT it quietly pre-weakened a large region it was
/// not allowed to break -- primed to fail later from something unrelated,
/// which is the "delayed, apparently causeless collapse" shape
/// `load::ROOTWARD_CHECK_STEPS` was reduced to bound.
///
/// The signature of `record_disturbance` carries it rather than a defaulted
/// overload existing beside it: three production callers, and a default
/// would let a new volume verb silently record a point, which is precisely
/// the bug. A verb with no natural extent passes `0` and reproduces the old
/// behaviour exactly.
#[derive(Clone, Copy, Debug)]
pub struct Disturbance {
    pub x: i32,
    pub y: i32,
    /// Outer limit of the damage the verb does *itself*, in cells from
    /// `(x, y)`. Read off the tool, never guessed: `explosion` takes the
    /// reach its own joint fabric actually used (which C3's surface-burst
    /// fix stretches by up to 2x), `rigid::strike` its swing's crack reach,
    /// `rigid::mine_swept` its chisel's.
    pub extent: i32,
    pub frame: u64,
}

/// How many recent disturbances are remembered.
///
/// Was 16, on the reasoning that a player cannot disturb more places than
/// that inside one `chain_window`. Still true of a player, and the world
/// is not a player: since fire burnouts and structural phase changes
/// report themselves too, a fire front or a lava pour writes disturbances
/// along its whole length. `record_disturbance` coalesces those at
/// `chain_reach / 2` -- 8 cells at the default -- so 64 slots cover a
/// ~512-cell span of simultaneous activity before the ring starts
/// evicting, which is a scene-wide fire rather than an ordinary one.
///
/// What eviction would cost is specific and is why this is not left small:
/// the oldest entry goes first, and the oldest entry is the player's dig
/// from nine seconds ago -- exactly the delayed cave-in `chain_window` is
/// generous in order to preserve. A blast that wants a wider licence
/// should still raise `chain_reach` rather than report itself many times.
///
/// The cost of the larger ring is a linear scan of it, paid once per
/// record and once per cell that has *already reached a failing verdict*
/// (`within_disturbance`) -- never per cell per frame.
const MAX_DISTURBANCES: usize = 64;

/// How long a disturbance keeps licensing failures near it, in frames —
/// ten seconds at 60Hz.
///
/// Generous deliberately. The owner's requirement is that *"collapse must
/// be obvious and delayed, so the player can get supports in first"*, and
/// a short window would make an undermined roof safe the moment you
/// stopped digging, which is the opposite of that.
pub const CHAIN_WINDOW_FRAMES: u64 = 600;

/// Named settings for `World::chain_reach`, cycled with `F9`.
///
/// A selector rather than a decision, for the same reason the grain modes
/// and the spoil modes are: the owner has said both *"they chain too far
/// and too much"* and *"collapse must be obvious and delayed, so the
/// player can get supports in first"*, and those pull opposite ways. Which
/// is right is a question for the hand, not for argument -- five grain
/// modes behind one key settled in minutes what no amount of still images
/// had.
///
/// Ordered from the shipped behaviour outward, so cycling is a tour from
/// "as it is" to the far end.
///
/// # Why `TIGHT` is not first, though it was asked for
///
/// The owner asked for `TIGHT` as the default and it was built that way,
/// then measured and backed out **on `scene=room wall=5 dig=3`**, the
/// acceptance pair that encodes "cutting a wall brings the room down":
///
/// ```text
///   chain_reach   failing cells   roofed void left
///   TIGHT (16)              244               100%
///   LOCAL (48)            1,959                23%
///   SPREAD               1,959                23%
/// ```
///
/// **Re-measured after `section_share` landed**, because a concurrent
/// branch warned it would move the SPREAD baseline from 1,975 to 2,733.
/// On this scene it did not -- 1,959 with sharing on, against 1,975
/// without. Their figure presumably came from a different scene or an
/// earlier shape of the port; recorded either way, because the warning was
/// right in principle and checking it cost one command.
///
/// **The 238 is this branch's own number and is worth reading correctly.**
/// `scene=room` builds its walls with `paint_capsule_as`, and D1's brush
/// fix records a disturbance per structural cell written -- so constructing
/// the room blankets its own walls with licences. With that suppressed the
/// same run measures **41** cells. Both leave the roofed void at 100%: the
/// brush fix makes TIGHT strictly better here and nowhere near enough,
/// which is why it does not change the conclusion. Reconciled against a
/// concurrent branch that measured 41-47 and correctly guessed the cause.
///
/// At TIGHT the room does not come down at all. The reason is geometric
/// and is not a bug: `licence_radius` is `chain_reach + extent`, a
/// radius-3 chisel's extent is 5, and the ceiling of a 200-wide room fails
/// as one region reaching ~100 cells from the cut -- so
/// `clip_region_to_licence` correctly keeps only the part within reach.
/// (`relicense_staged_fractures` is *not* the mechanism here and an
/// earlier version of this note said it was: that one fires only from
/// `App::cycle_chain_mode`, so a world constructed at a reach never runs
/// it. The clip is what a harness scene measures.) This
/// page's own wiki entry already named that trade as the open question
/// (*"a long span can lose the part near the blast and leave the far part
/// standing on nothing"*); making TIGHT the default makes it the default
/// experience, which is a decision for a playtest and not for a merge.
///
/// `LOCAL` delivers the containment that was actually asked for -- damage
/// stays near the blow -- without that cost. Changing the default is one
/// line here plus `World::new`'s mirror, guarded by
/// `the_default_chain_reach_is_the_first_chain_mode`.
pub struct ChainMode {
    pub name: &'static str,
    pub note: &'static str,
    pub reach: i32,
}

pub const CHAIN_MODES: [ChainMode; 4] = [
    ChainMode { name: "SPREAD", note: "damage travels as far as the load says - costs nothing, chains furthest", reach: i32::MAX },
    ChainMode { name: "LOCAL", note: "damage stays within a wide room of what you hit", reach: 48 },
    ChainMode { name: "TIGHT", note: "the wound and its surroundings only - undermining still bites", reach: 16 },
    ChainMode { name: "NONE", note: "only what you struck ever fails - nothing collapses later", reach: 0 },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::organism;
    use crate::sim::scheduler;
    use crate::sim::update;

    /// The load model with no `chain_reach` leash -- see
    /// `World::without_chain_limit` for why the model's own tests take it
    /// off and the game does not.
    fn test_world() -> World {
        World::new(Rect::new(0, 0, 63, 63)).without_chain_limit()
    }

    /// Whatever stone currently breaks into, read from the registry rather
    /// than named directly — retargeting `stone.ron`'s `breaks_into` should
    /// not silently turn these into assertions about a material the engine
    /// no longer produces.
    fn stone_debris(w: &World) -> MaterialId {
        w.materials.get(material::STONE).breaks_into.expect("stone must define a breaks_into")
    }

    fn run(w: &mut World, frames: usize) {
        for _ in 0..frames {
            w.begin_step();
            scheduler::step(w);
            w.end_step();
        }
    }

    // ---- buoyancy: `MaterialDef::floats` (the ice milestone) ------------

    /// A raft of ice on water is held up by the water, and the paired
    /// negative is the whole point of the test: **the same geometry in
    /// stone is not**.
    ///
    /// Geometry cannot tell a sheet floating on a pond from a slab
    /// suspended in one, which is `CLAUDE.md`'s "when a rule must tell
    /// apart two things that can look identical, state the difference as
    /// data" -- so the difference is a bit on the material and this test
    /// asserts on the bit, not on the shape. A single assertion about ice
    /// would pass just as well if the predicate had been widened to accept
    /// any `Liquid` below anything, which is the change that would quietly
    /// make undercut rock below a lake unfallable.
    #[test]
    fn a_floating_solid_is_held_up_by_the_water_under_it_and_a_sinking_one_is_not() {
        for (name, expect_held) in [("ice", true), ("stone", false)] {
            let mut w = test_world();
            let id = w.materials.id_of(name).expect("both materials ship");
            // A basin with no floor under the middle of the water, so the
            // only thing beneath the raft is liquid. Walls of bedrock, or
            // the water would run out from under it and the test would be
            // measuring drainage.
            for y in 30..40 {
                w.set(19, y, Cell::new(material::BEDROCK, 0));
                w.set(45, y, Cell::new(material::BEDROCK, 0));
            }
            for x in 19..=45 {
                w.set(x, 40, Cell::new(material::BEDROCK, 0));
            }
            for x in 20..45 {
                for y in 32..40 {
                    w.set(x, y, Cell::new(material::WATER, 0));
                }
            }
            // A raft in the middle of the span, touching neither wall: the
            // *only* thing that can hold it is what is underneath.
            for x in 28..37 {
                w.set(x, 31, Cell::new(id, 0));
            }
            compute_world_distances(&mut w);

            let held = is_resting_on_ground(&w, 32, 31);
            assert_eq!(
                held, expect_held,
                "{name} on water: is_resting_on_ground said {held}, expected {expect_held} -- \
                 buoyancy is opt-in per material and must not leak to material that never asked"
            );
        }
    }

    /// **A cloud of steam is somewhere to fall, exactly as air is.**
    ///
    /// `region_has_free_face` read `EMPTY` and, later, a lighter `Liquid`,
    /// and silently said no to every `Gas` -- so a stone cell in the steam
    /// a quench throws off was "wedged in a hole its own shape", recorded
    /// as a confined `Unsupported` failure and **left standing with nothing
    /// rescheduled**. The steam then blew away and the stone hung in open
    /// air for the rest of the run: 23 such cells on `scene=lavadrop` and
    /// 31 on `scene=lavapour`, which is what the owner reported as rock
    /// "stuck in the middle of the water".
    ///
    /// Asserted on the **confined counter** as well as on the cell,
    /// because the two claims come apart: a cell that broke free for some
    /// other reason and a cell the free-face rule let go look identical
    /// afterwards, and only the counter says which rule ran.
    #[test]
    fn a_solid_in_a_cloud_of_gas_is_not_confined() {
        let mut w = test_world();
        let steam = w.materials.id_of("steam").expect("steam.ron should be embedded");
        let debris = stone_debris(&w);
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                w.set(32 + dx, 32 + dy, Cell::new(steam, 0));
            }
        }
        // One unattached stone cell in the middle of it, touching nothing
        // solid and nothing empty.
        w.set(32, 32, Cell::new(material::STONE, 0));
        compute_world_distances(&mut w);
        w.schedule_structural_check_around(32, 32);
        run(&mut w, 60);

        assert_eq!(
            w.structural_failures.confined, 0,
            "a gas the piece outweighs is a free face -- {} confined failures says the rule still reads steam as rock",
            w.structural_failures.confined
        );
        assert_eq!(
            w.get(32, 32).material,
            debris,
            "the stone should have come free into debris; it is still {}",
            w.materials.get(w.get(32, 32).material).name
        );
    }

    /// **A grain the piece has swallowed does not hold the piece up — and
    /// two grains do not either, which is the case that shipped.**
    ///
    /// The first version of this test placed **one** grain inside the raft
    /// and passed against a rule that asked whether the grain was enclosed
    /// by body material on all four faces. Two adjacent grains defeat that
    /// rule outright, because each is the other's non-body neighbour — and
    /// they went out. `scene=rockdrop`'s 600-cell slab then sat in open air
    /// 30 rows above its pond from frame 40 to 400, held there by pockets
    /// like this:
    ///
    /// ```text
    /// 170 ......###############################ooo#####..##.#####
    /// 171 ......######################oooooooooo###....##########
    /// ```
    ///
    /// so the pair case is asserted here explicitly rather than left to
    /// follow from the single one. See `load::grain_is_footing`, which now
    /// asks what the grain is *standing on* instead of what is beside it.
    ///
    /// Four cases, and the two positives matter as much as the negatives: a
    /// rule that had simply stopped believing in rubble would take
    /// `scene=ligament`'s slab out of the air, which is not wanted.
    #[test]
    fn a_grain_swallowed_by_a_piece_is_not_a_footing() {
        // A raft sitting just above the bedrock floor, so a grain placed
        // *under* it has something real to stand on. The first fixture put
        // the raft in mid-air and asserted that a grain hanging under it
        // was a footing -- which the new rule correctly denies, because a
        // grain with air beneath it is falling.
        let raft_with = |grains: &[(i32, i32)]| {
            let mut w = test_world();
            let sand = w.materials.id_of("sand").expect("sand.ron should be embedded");
            for x in 0..64 {
                w.set(x, 63, Cell::new(material::BEDROCK, 0));
            }
            for x in 24..40 {
                for y in 58..62 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            for &(gx, gy) in grains {
                w.set(gx, gy, Cell::new(sand, 0));
            }
            compute_world_distances(&mut w);
            w
        };

        // Under the piece, standing on bedrock: a real footing.
        let w = raft_with(&[(32, 62)]);
        assert!(
            is_resting_on_ground(&w, 32, 61),
            "a grain between the piece and the bedrock is exactly what `rests_on_ground` is for"
        );

        // One grain walled inside the piece.
        let w = raft_with(&[(32, 60)]);
        assert!(
            !is_resting_on_ground(&w, 32, 59),
            "a lone grain inside the piece is filler, not footing"
        );

        // **Two adjacent grains inside the piece** -- the shipped bug.
        let w = raft_with(&[(32, 60), (33, 60)]);
        assert!(
            !is_resting_on_ground(&w, 32, 59),
            "two grains inside the piece are still filler; each being the other's neighbour must not make them ground"
        );

        // And a grain with nothing under it is falling, whatever is above.
        let mut w = raft_with(&[]);
        let sand = w.materials.id_of("sand").expect("sand.ron should be embedded");
        for x in 24..40 {
            for y in 30..34 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        w.set(32, 34, Cell::new(sand, 0));
        compute_world_distances(&mut w);
        assert!(
            !is_resting_on_ground(&w, 32, 33),
            "a grain hanging in mid-air under a raft holds nothing up"
        );
    }

    /// **Ground that flows away without telling anybody.**
    ///
    /// `is_resting_on_ground` roots a support chain on a `Powder`
    /// underneath, on the recorded argument that "if it flows out from
    /// underneath, that write dirties the chunk and whatever sat on it is
    /// re-examined". It does not: waking a chunk schedules no
    /// `StructuralCheck`, and `World::set` -- the seam every mover goes
    /// through -- schedules none either. So the cell above read `aux 0`,
    /// was judged "holds", stopped rescheduling itself, and was never asked
    /// again.
    ///
    /// The grain is removed with a bare `set`, deliberately: that is the
    /// *whole* claim under test. Anything that schedules a check on the way
    /// past -- an erase through the brush, a blast, a `break_free` -- would
    /// paper over the defect and make this pass against the broken code.
    /// Nothing here runs the CA sweep either, for the same reason.
    ///
    /// See `GROUNDED_RECHECK_INTERVAL`, and note the wait: this asserts the
    /// cell comes down *eventually*, not instantly, which is the trade that
    /// keeps the cost off the CA hot path.
    #[test]
    fn a_cell_left_standing_on_nothing_is_asked_again() {
        let mut w = test_world();
        let sand = w.materials.id_of("sand").expect("sand.ron should be embedded");
        let debris = stone_debris(&w);
        // The grain sits **on the bedrock**, not in mid-air thirty rows
        // above it. The first fixture did the latter, and
        // `load::grain_is_footing` was later taught that a grain with
        // nothing under it is falling rather than bearing -- so the stone
        // came down on the first check and the setup assertion below,
        // which is meant to be the boring half, went red. A fixture that
        // does not contain the situation reads exactly like a broken
        // mechanism.
        for x in 0..64 {
            w.set(x, 63, Cell::new(material::BEDROCK, 0));
        }
        w.set(32, 62, Cell::new(sand, 0));
        w.set(32, 61, Cell::new(material::STONE, 0));
        compute_world_distances(&mut w);
        w.schedule_structural_check_around(32, 61);
        run(&mut w, 60);
        assert_eq!(
            w.get(32, 61).material,
            material::STONE,
            "the grain is still there, so the stone should still be standing on it"
        );

        // The grain goes, and nothing is told.
        w.set(32, 62, Cell::EMPTY);
        run(&mut w, 240);

        assert_eq!(
            w.get(32, 61).material,
            debris,
            "the ground went and nothing rescheduled the cell above it -- still {}",
            w.materials.get(w.get(32, 61).material).name
        );
    }

    /// **Ice participates in the load model**: freeze a bridge over open
    /// air, take away what holds it up, and it comes apart into snow.
    ///
    /// The other half of the buoyancy pair above, and the reason `floats`
    /// grants *ground* rather than immunity: a sheet on water is supported,
    /// and a span over nothing is not. Leaving ice's
    /// `max_unsupported_span` unset would have passed the freeze-over case
    /// and failed this one -- `load::capacity` reads that sentinel as "not
    /// in the structural system at all" and returns infinite capacity, so
    /// the bridge would hang there. See `MaterialDef::floats`.
    ///
    /// The debris material is read from the registry rather than named, the
    /// same way `stone_debris` does it: retargeting ice.ron's
    /// `breaks_into` should not silently turn this into an assertion about
    /// a material the engine no longer makes.
    #[test]
    fn ice_participates_in_load() {
        let mut w = test_world();
        let ice = w.materials.id_of("ice").expect("ice.ron should be embedded");
        let debris = w.materials.get(ice).breaks_into.expect("ice must define a breaks_into");
        // Two piers of bedrock with a long ice span between them, in air --
        // no water anywhere, so nothing floats and the span is carried by
        // its ends alone.
        for y in 30..40 {
            w.set(10, y, Cell::new(material::BEDROCK, 0));
            w.set(60, y, Cell::new(material::BEDROCK, 0));
        }
        for x in 10..=60 {
            w.set(x, 29, Cell::new(ice, 0));
        }
        compute_world_distances(&mut w);
        run(&mut w, 30);
        let spanning = (11..60).filter(|&x| w.get(x, 29).material == ice).count();
        assert!(spanning > 40, "the bridge should stand while both piers hold it: only {spanning} of 49 cells left");

        // Erase one pier. Erasing delivers no load and no impulse -- it is
        // the cheapest way to ask "was this being held", which is exactly
        // the question here.
        for y in 30..40 {
            w.set(60, y, Cell::EMPTY);
        }
        w.schedule_structural_check_around(60, 29);
        run(&mut w, 240);

        let left = (11..60).filter(|&x| w.get(x, 29).material == ice).count();
        let snow = (0..64)
            .flat_map(|x| (0..64).map(move |y| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == debris)
            .count();
        assert!(left < spanning, "the bridge lost nothing when its support went: {left} cells against {spanning}");
        // **The counter, not just the picture.** A span that quietly slid
        // sideways under the CA sweep and one the load model took down look
        // the same in a cell count, and only one of them is this mechanism
        // (`CLAUDE.md`: "did it fire at all" needs a counter).
        //
        // Either criterion counts. Measured: the 49-cell cantilever off one
        // remaining pier goes by **overload**, one event carrying the whole
        // span, rather than by running past its `max_unsupported_span` --
        // the bending moment on a span that long exceeds ice's capacity
        // before the distance does. Asserting specifically on `unsupported`
        // failed here for that reason, which is worth recording: the two
        // modes are interchangeable from the player's side and not from the
        // model's.
        let f = w.structural_failures;
        assert!(
            f.overloaded + f.unsupported > 0,
            "no structural failure fired at all, so whatever moved the ice was not the load model \
             (overloaded {}, unsupported {})",
            f.overloaded,
            f.unsupported
        );
        // Graded debris, not nothing and not one coherent slab: measured
        // 14 cells of snow off a 49-cell span, the rest going as bodies in
        // flight. `CLAUDE.md`'s ethos section: a destructive event that
        // produces no debris is not finished.
        assert!(snow > 0, "ice that gives way should come away as {} -- graded debris, not nothing", w.materials.get(debris).name);
    }

    /// The rule that `compute_world_distances`'s flat mirror had to be told
    /// about explicitly, and the one thing that cannot be inferred from the
    /// cells themselves: **off-world reads are bedrock.** `World::get`
    /// answers out-of-bounds with `Cell::OUT_OF_BOUNDS`, whose material is
    /// `BEDROCK`, so a cell against the world edge is anchored by the edge.
    ///
    /// Six load tests failed when a rewrite bounds-checked those reads away,
    /// which is coverage but not a statement: every one of them failed for a
    /// reason several steps downstream of the cause, and the shared cause
    /// was only obvious once all six were read together. This says it once.
    #[test]
    fn a_cell_against_the_world_edge_is_anchored_by_the_edge() {
        let mut w = test_world();
        let bounds = w.bounds().expect("test world is bounded");
        // A column hard against the left wall with nothing below it: its only
        // possible anchor is the edge itself. Deliberately nowhere near the
        // bedrock floor, so a pass that reached the floor instead would give
        // a large distance rather than zero.
        for y in 4..12 {
            w.set(bounds.min_x, y, Cell::new(material::STONE, 0));
        }
        compute_world_distances(&mut w);
        assert_eq!(
            w.get(bounds.min_x, 4).aux(),
            0,
            "a cell touching the world edge should be anchored at distance 0;              off-world reads must answer as bedrock the way `World::get` does"
        );

        // ...and the same column one cell in is *not* anchored, or the
        // assertion above would pass on a pass that anchored everything.
        for y in 4..12 {
            w.set(bounds.min_x + 1, y, Cell::new(material::STONE, 0));
        }
        compute_world_distances(&mut w);
        assert!(
            w.get(bounds.min_x + 1, 4).aux() > 0,
            "a cell one column in from the edge is supported *through* its neighbour, not by the edge"
        );
    }

    /// `run`, plus the per-organism passes — i.e. what production actually
    /// executes, via `World::step_active_sites`.
    ///
    /// **Every organism test has to use this**, and the reason is the whole
    /// point of the support rewrite. A cell's distance to its anchors is no
    /// longer searched for at check time; `plant::anchor_support` computes
    /// it from the anchors outward once per organism tick, and
    /// `organism_structural_tick` reads the result. A harness that drives
    /// only `scheduler::step` never runs that pass, so every cell keeps
    /// `OrganismCell::support`'s default of 0 — "anchored" — and nothing
    /// can ever fail.
    ///
    /// That is not a weakening of these tests, it is them driving the real
    /// path: three of the four failed the moment the new mechanism landed,
    /// which is exactly what `CLAUDE.md` asks for when a mechanism is
    /// replaced ("deliberately break the replacement and confirm the old
    /// tests fail — if they still pass, delete them rather than porting
    /// them"). They did fail, so they were testing something, and they get
    /// ported.
    ///
    /// Organisms tick on `(frame + organism_id) % ORGANISM_TICK_INTERVAL`,
    /// so a frame count below ~45 can miss the pass entirely.
    fn run_organisms(w: &mut World, frames: usize) {
        for _ in 0..frames {
            w.begin_step();
            w.step_active_sites();
            w.end_step();
        }
    }

    #[test]
    fn overlapping_schedule_structural_check_around_calls_do_not_duplicate() {
        // `Reports` code-review-findings item #2: `schedule_structural_
        // check_around` fans out to 5 positions per call, and disturbance
        // sites routinely overlap (adjacent cleared cells in an explosion
        // share neighbours). Two adjacent calls here share exactly 2 of
        // their 5 positions each -- (10,10)'s own 5 are (10,10),(9,10),
        // (11,10),(10,9),(10,11); (11,10)'s own 5 are (11,10),(10,10),
        // (12,10),(11,9),(11,11) -- so 8 distinct positions total, not 10,
        // and deduping should land on exactly 8 pending sites, not 10.
        let mut w = test_world();
        w.schedule_structural_check_around(10, 10);
        w.schedule_structural_check_around(11, 10);
        assert_eq!(
            w.active_site_count(),
            8,
            "two overlapping schedule_structural_check_around calls should dedup to 8 distinct positions, not the raw 10"
        );
    }

    #[test]
    fn a_painted_column_knows_its_own_distances_immediately() {
        // `Reports/destruction-plan.md` B3. A painted cell starts at
        // `aux = 0`, which reads as *anchored*, and a reactive relaxation
        // climbs from there at roughly one cell per
        // `STRUCTURAL_TICK_INTERVAL` frames -- so a 30-cell column would
        // spend ~30 rounds either wrongly immune or half-converged before
        // deciding anything. That delay is the shape of "I built a thing
        // and it collapsed ten seconds later".
        //
        // Zero frames are run below, deliberately: the claim is that the
        // stroke itself leaves the structure converged.
        let mut w = test_world();
        let top = 34;
        w.paint_capsule((30, 63), (30, top), 1, material::STONE, 1.0);

        assert_eq!(w.get(30, 63).aux(), 0, "the base sits on the world edge, which is an anchor");
        let at_top = w.get(30, top).aux();
        assert!(
            (25..u16::MAX).contains(&at_top),
            "the top of a 30-cell column should already know it is ~29 steps from an anchor, found {at_top}"
        );
        // The failure this guards against is specifically reading as
        // anchored, which is what `aux = 0` means everywhere else.
        assert_ne!(at_top, 0, "a freshly painted column top read as anchored");
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
    fn a_stone_cantilever_exceeding_its_tolerance_breaks_free() {
        // Was a 6-tall vertical stack, back when every step cost a flat 1
        // and stone's span was 3. That scenario no longer breaks anything
        // and *should* not: `support_cost_below: 0` means standing on rock
        // is free, so a column stands to any height (see
        // `a_stone_tower_stands_however_tall_it_gets`). Leaning sideways is
        // what still accumulates, so an over-span cell is now a cantilever.
        let mut w = test_world();
        let span = w.materials.get(material::STONE).max_unsupported_span as i32;
        // Anchored at the left world edge (x=0's own out-of-bounds
        // neighbour reads as the BEDROCK sentinel), reaching well past the
        // span. One cell thick, so it is never confined.
        for x in 0..=(span + 6) {
            w.set(x, 30, Cell::new(material::STONE, 0));
        }
        let tip = span + 6;
        w.schedule_structural_check(tip, 30);
        run(&mut w, 400);

        // Not "did the tip become rubble" -- that pins *which* of two
        // outcomes happened rather than the claim. A failing region is
        // handed to `rigid::fracture`, which promotes anything at or above
        // `MIN_BODY_CELLS` to a falling body (leaving `EMPTY` behind) and
        // converts the rest to debris in place; which one a given cell gets
        // depends on the material's own fragment ladder, so this assertion
        // broke the moment stone got a rung added. What the test is about
        // is that an over-reaching cantilever gives way and its anchored
        // root does not.
        let debris = stone_debris(&w);
        let tip_material = w.get(tip, 30).material;
        assert!(
            tip_material == debris || tip_material == material::EMPTY,
            "a cantilever cell past stone's reach should have given way, found {tip_material:?}"
        );
        assert_eq!(w.get(0, 30).material, material::STONE, "the anchored root of the cantilever should still be standing");
    }

    /// A slab cut free by cracks in the *middle of a massif* must **crack
    /// where it stands**, and the identical slab with a cavity above it
    /// must break up and move.
    ///
    /// **A paired comparison, and it has to be**, because either half
    /// alone is passable by cheating in one direction. "Nothing moves"
    /// passes the first by making rock indestructible, which is how four
    /// earlier support models died; "everything moves" passes the second.
    /// The pair is what pins the rule to *free space* rather than to
    /// strength.
    ///
    /// It is also the guard against the replacement artifact rather than
    /// the original one: the way this fix fails is not by leaving the
    /// mid-mountain collapse in place, it is by suppressing collapse
    /// everywhere and leaving cliffs hanging in the air.
    ///
    /// # What this used to assert, and why that was the bug
    ///
    /// It asserted that all 32 cells of the buried slab were **still
    /// intact stone**, and that passed for a reason nobody wanted: an
    /// `Unsupported` failure that was also confined was discarded
    /// outright at `tick` -- no crush, no fissure, nothing. "It cracks
    /// where it stands" was the name of the test and the one thing it did
    /// not check. The owner reported the consequence directly: *cracks
    /// that fully surround chunks of rock do not break them off*.
    ///
    /// So the claim is restated as what the mechanism now promises, in the
    /// two halves that can be told apart: the buried slab writes **fresh
    /// fissures** and **leaves no hole behind**, and the opened one
    /// **promotes cells to moving bodies**. Deliberately not "every cell
    /// of the buried slab is still stone" -- that pins one of two outcomes
    /// rather than the claim, and it is the exact assertion that made this
    /// test agree with the bug. Rubble left standing in the slab's own
    /// footprint would be a legitimate outcome; a hole where the slab was
    /// would not.
    #[test]
    fn rock_with_nowhere_to_go_cracks_where_it_stands() {
        /// The slab is x 22..30 by y 33..37.
        ///
        /// **It sat at x 27..35 and had to move five cells left**, and that
        /// is a fact about the scene rather than a widened bar. A confined
        /// failure now reveals the rock's own joint fabric instead of
        /// walking a star through it (`reveal_joints`), and stone's grain
        /// is 13 cells -- the slab at its old address fell *entirely inside
        /// one Worley domain*, so it contained no joint and there was
        /// nothing in it to part along. That is the mechanism being right,
        /// not the test being wrong; but a scene that asks "does confined
        /// rock crack where it stands" and hands the rule a piece with no
        /// joint in it cannot answer either way (`CLAUDE.md`: check that a
        /// step can demonstrate itself). Five cells left it straddles the
        /// boundary, and the setup assertion below says so out loud so a
        /// later change to the grain or the world seed fails here with a
        /// reason instead of looking like a dead mechanism.
        ///
        /// **Growing it instead was tried first and quietly changed the
        /// scene**: at 20x12 the slab is wide enough that stone's 12x
        /// attached-span bonus holds it up, so only its 42 fringe cells
        /// were judged at all and each brought a 5-cell region -- under
        /// `MIN_FRACTURE_CELLS`, declined before any crush. The slab has to
        /// stay small enough to fail whole.
        const SLAB_CELLS: usize = 8 * 4;
        // A slab isolated by cracks on every side, so nothing holds it and
        // it must be judged as failing. `open` decides the one thing under
        // test: whether there is anywhere for it to go.
        fn massif_with_isolated_slab(open: bool) -> World {
            let mut w = test_world();
            for y in 20..50 {
                for x in 10..50 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            // Cut the slab loose: cracks all the way round x 22..30, y
            // 33..37. Support cannot cross a fracture, so this leaves it
            // with no parent at all in either world.
            for x in 22..30 {
                let top = w.get(x, 32);
                w.set(x, 32, top.with_crack_down(true));
                let bottom = w.get(x, 37);
                w.set(x, 37, bottom.with_crack_down(true));
            }
            for y in 33..37 {
                let left = w.get(21, y);
                w.set(21, y, left.with_crack_right(true));
                let right = w.get(29, y);
                w.set(29, y, right.with_crack_right(true));
            }
            if open {
                // A cavity **directly** above the slab -- the cave-in
                // case. Carved after the cracks and reaching down to y=32
                // so its floor is the slab's own top row, which is the
                // whole point: a first version stopped at y=31 and left a
                // row of rock between the two, so the "open" world was
                // just as confined as the buried one and the test compared
                // a case against itself. A scene that does not contain the
                // situation it is named for looks exactly like a broken
                // mechanism.
                for y in 28..33 {
                    for x in 21..31 {
                        w.set(x, y, Cell::EMPTY);
                    }
                }
            }
            // Distances *after* the cracks, not before: the relaxation
            // skips cracked edges, so this is what actually leaves the
            // slab with no path to an anchor. Computing first and cracking
            // second leaves every cell holding a stale, perfectly good
            // distance and nothing ever fails -- which is a test that
            // passes while exercising nothing.
            compute_world_distances(&mut w);
            // Every cell of the slab, because only a cell with a cracked
            // or empty neighbour is `is_structurally_interesting`, and an
            // interior one is not: scheduling the middle of the slab is
            // scheduling a cell the model deliberately never looks at.
            for y in 33..37 {
                for x in 22..30 {
                    w.schedule_structural_check(x, y);
                }
            }
            w
        }

        // **Still intact stone**, not "became debris", and the difference
        // has bitten this file before. A failing region goes to
        // `rigid::fracture`, which promotes anything at or above
        // `MIN_BODY_CELLS` to a falling body -- leaving `EMPTY` behind --
        // and converts only the rest to debris in place. Which one a given
        // slab gets depends on the material's fragment ladder, so counting
        // debris pins one of two outcomes rather than the claim. The claim
        // is that the rock either stayed or did not.
        let slab_intact =
            |w: &World| (33..37).flat_map(|y| (22..30).map(move |x| (x, y))).filter(|&(x, y)| w.get(x, y).material == material::STONE).count();
        // And the weaker question, which is the one the buried half asks:
        // is there still *something* in every cell the slab occupied.
        // Stone that crushed to rubble in its own footprint passes this
        // and fails `slab_intact`; a slab that travelled fails both.
        let slab_present =
            |w: &World| (33..37).flat_map(|y| (22..30).map(move |x| (x, y))).filter(|&(x, y)| w.get(x, y).material != material::EMPTY).count();

        let mut buried = massif_with_isolated_slab(false);
        run(&mut buried, 300);
        let mut opened = massif_with_isolated_slab(true);
        run(&mut opened, 300);

        assert!(
            buried.structural_failures.confined > 0,
            "test setup: the buried slab was never judged as a confined failure, so this proves nothing"
        );
        // And the second half of the setup, which the counter above cannot
        // see: the slab has to *contain a joint* for "it cracks where it
        // stands" to have an answer. One domain identity comparison per
        // interior edge, exactly the rule `reveal_inside_region` applies.
        let pitch = buried.materials.get(material::STONE).joint_spacing;
        let interior_joints = (33..37)
            .flat_map(|y| (22..30).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let home = super::super::fracture_field::domain(buried.seed, x, y, pitch);
                [(1, 0), (0, 1)].iter().any(|&(dx, dy)| {
                    (x + dx) < 30 && (y + dy) < 37 && super::super::fracture_field::domain(buried.seed, x + dx, y + dy, pitch) != home
                })
            })
            .count();
        assert!(
            interior_joints > 0,
            "test setup: the slab lies entirely inside one Worley domain, so there is no joint in it to reveal -- move it, or the grain changed"
        );
        // The buried half. Fissures, because that is what "cracks where it
        // stands" means and it is what was missing; and no hole, because
        // rock with nowhere to go must not have gone there anyway.
        assert!(
            buried.structural_failures.crushed_cells > 0,
            "the buried slab was judged, found unsupported, and then left entirely alone -- confined rock has to crack where it stands"
        );
        assert_eq!(
            buried.structural_failures.promoted_cells, 0,
            "{} cells of the buried slab were promoted to moving bodies -- there is nowhere in solid rock for them to move to",
            buried.structural_failures.promoted_cells
        );
        assert_eq!(
            slab_present(&buried),
            SLAB_CELLS,
            "the buried slab left a hole behind: {} of {SLAB_CELLS} cells still hold material",
            slab_present(&buried)
        );
        // The open half, on the counter rather than on the census: "moved"
        // is a displacement and every other number in `FailureCounts` is a
        // judgement (`world.rs`'s own note on `promoted_cells`). A slab
        // that failed, was recorded, and then sat exactly where it was
        // reads identically to this one on any of them.
        assert!(
            opened.structural_failures.promoted_cells > 0,
            "the same slab with a cavity above it promoted nothing -- it was judged as failing and never moved"
        );
        assert!(
            slab_intact(&opened) < SLAB_CELLS,
            "the same slab with a cavity above it must still come apart -- {} of {SLAB_CELLS} cells still intact",
            slab_intact(&opened)
        );
    }

    /// A severed piece's damage must **scale with the piece**, right down
    /// to zero -- a chip with no inside left cracks nothing.
    ///
    /// # What this is guarding, and from which direction
    ///
    /// `crush_in_place`'s star used to be sized with a floor
    /// (`CRACK_MIN_LENGTH` 10, `CRACK_FORKS_BASE` 4) that was tuned against
    /// an over-capacity *section*. When cracks learned to sever, that same
    /// star started being handed one-cell islands, and each one fired three
    /// ten-cell rays into the rock around it, severing more islands, which
    /// fired again: 84,051 cracked cells on a nine-blast rolling world, most
    /// of the solid rock in it, and a hillside that went dark from the
    /// crater outward while almost nothing flew.
    ///
    /// So this is a **paired** test, because either half alone is passable
    /// by cheating. "Nothing cracks" passes the chip by suppressing the
    /// confined crush entirely, which is the mechanism
    /// `rock_with_nowhere_to_go_cracks_where_it_stands` exists to defend;
    /// "everything cracks" passes the slab and is the runaway above. The
    /// pair pins the damage to the *size of the piece*.
    ///
    /// The chip's arm looks past the counter at the rock **around** it as
    /// well, and that is the assertion that would catch the runaway coming
    /// back: a restored floor writes its rays outward, into cells the chip
    /// does not occupy, which is exactly how one severed island manufactures
    /// the next one.
    #[test]
    fn a_severed_chip_with_no_inside_left_cracks_nothing() {
        // Cracks all the way round a `w` x `h` block at (30, 34), in the
        // middle of a massif, so nothing holds it and there is no empty
        // neighbour anywhere: unsupported *and* confined, which is the one
        // branch under test. Deliberately isolated on all four edges --
        // `crack_down` is the edge *below* a cell and `crack_right` the edge
        // to its right, so the bottom row's own `crack_down` and the right
        // column's own `crack_right` are the far sides.
        fn massif_with_severed_piece(w_cells: i32, h_cells: i32) -> (World, Vec<(i32, i32)>) {
            let mut w = test_world();
            for y in 8..60 {
                for x in 6..58 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            let (x0, y0) = (20, 24);
            let (x1, y1) = (x0 + w_cells - 1, y0 + h_cells - 1);
            for x in x0..=x1 {
                let above = w.get(x, y0 - 1);
                w.set(x, y0 - 1, above.with_crack_down(true));
                let bottom = w.get(x, y1);
                w.set(x, y1, bottom.with_crack_down(true));
            }
            for y in y0..=y1 {
                let left = w.get(x0 - 1, y);
                w.set(x0 - 1, y, left.with_crack_right(true));
                let right = w.get(x1, y);
                w.set(x1, y, right.with_crack_right(true));
            }
            // Distances *after* the cracks: the relaxation skips cracked
            // edges, so this is what actually leaves the piece with no path
            // to an anchor. The other order leaves every cell holding a
            // stale, perfectly good distance and nothing ever fails.
            compute_world_distances(&mut w);
            let piece: Vec<(i32, i32)> = (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| (x, y))).collect();
            for &(x, y) in &piece {
                w.schedule_structural_check(x, y);
            }
            (w, piece)
        }

        // Every crack bit in the world that is not on one of the piece's own
        // cells. The setup writes some of these itself (the collar that cuts
        // the piece loose), so it is the *change* that is the assertion.
        fn cracks_outside(w: &World, piece: &[(i32, i32)]) -> usize {
            (0..64)
                .flat_map(|y| (0..64).map(move |x| (x, y)))
                .filter(|p| !piece.contains(p))
                .filter(|&(x, y)| w.get(x, y).crack_down() || w.get(x, y).crack_right())
                .count()
        }

        // Two cells by two: below `MIN_FRACTURE_CELLS`, and physically a
        // chip that is already severed on all four sides. There is nothing
        // inside it left to separate.
        let (mut chip, chip_cells) = massif_with_severed_piece(2, 2);
        assert!(chip_cells.len() < super::super::rigid::MIN_FRACTURE_CELLS);
        let cracks_before = cracks_outside(&chip, &chip_cells);
        run(&mut chip, 300);
        assert!(
            chip.structural_failures.confined > 0,
            "test setup: the chip was never judged as a confined failure, so this proves nothing"
        );
        assert_eq!(
            chip.structural_failures.crushed_cells, 0,
            "a four-cell severed chip cracked {} cells -- it has no inside left to separate",
            chip.structural_failures.crushed_cells
        );
        assert_eq!(
            cracks_outside(&chip, &chip_cells),
            cracks_before,
            "the chip wrote fissures into the rock *around* it -- that is how one severed island manufactures the next"
        );

        // The other end of the same rule: a piece with an interior still
        // comes apart where it stands.
        //
        // **It was 6x6 and had to grow to 20x18.** A confined failure now
        // reveals joints rather than walking a star, and stone's grain is
        // 13 cells -- a 36-cell piece fits inside one Worley domain and has
        // no joint in it, which is the *same* answer the mechanism gives
        // the chip and for the same physical reason. The claim under test
        // is "a piece with an interior comes apart", so the scene has to
        // contain a piece with an interior at the grain the rule reads. It
        // fails in sub-regions of 11 to 49 cells rather than whole (a piece
        // this wide is partly held up by stone's attached-span bonus),
        // which is fine here and is *not* fine for
        // `rock_with_nowhere_to_go_cracks_where_it_stands` -- see the note
        // there on why that one moved instead of growing.
        let (mut slab, slab_cells) = massif_with_severed_piece(20, 18);
        assert!(slab_cells.len() >= super::super::rigid::MIN_FRACTURE_CELLS);
        let slab_cracks_before = cracks_outside(&slab, &slab_cells);
        run(&mut slab, 300);
        assert!(
            slab.structural_failures.confined > 0,
            "test setup: the slab was never judged as a confined failure, so this proves nothing"
        );
        assert!(
            slab.structural_failures.crushed_cells > 0,
            "a 360-cell severed slab cracked nothing -- scaling the damage from zero must not turn the crush off"
        );
        // **And this is the arm that catches the runaway coming back.** The
        // gate above hides that change from the chip -- a four-cell chip is
        // declined before anything is sized -- so without a bar here the
        // pair would pass while one severed island went back to
        // manufacturing the next.
        //
        // The measured value is **0** now and the bar is not set on it. A
        // severed piece reveals only joints with *both* cells inside
        // itself, so it writes nothing outside by construction; the
        // history is 49 crack bits when the walked star scaled with the
        // piece and 199 with the section's floor restored to it, and the
        // failure mode the bar exists for is `SeveredPiece` being given the
        // section's *disc* by a later edit, which on this scene is ~130
        // edges of fabric mostly outside the piece. 24 sits clear of zero
        // and far clear of all three artifacts.
        let slab_spread = cracks_outside(&slab, &slab_cells) - slab_cracks_before;
        assert!(
            slab_spread <= 24,
            "the slab wrote {slab_spread} crack bits into the rock around it (measured 0 when a severed piece reveals only its own interior joints; 49 and 199 for the two walked stars this replaced) -- the damage is being sized for the wrong object again"
        );
    }

    /// The rule is keyed on the *outcome*, never on the criterion, and
    /// this is what says so out loud.
    ///
    /// The retired "confinement as an anchor" model
    /// (`Reports/load-model-handoff.md` §6.1) inferred *support* from
    /// burial, which made thick rock immune to failing at all. It is one
    /// word away from this one, so the distinction gets a test: a buried
    /// slab must still be **recorded as failing**. If a later change
    /// quietly turns this into "confined rock does not fail", the count
    /// goes to zero and this fails.
    #[test]
    fn a_confined_failure_still_fails_it_just_cannot_travel() {
        let mut w = test_world();
        for y in 20..50 {
            for x in 10..50 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        for x in 27..35 {
            let top = w.get(x, 32);
            w.set(x, 32, top.with_crack_down(true));
            let bottom = w.get(x, 37);
            w.set(x, 37, bottom.with_crack_down(true));
        }
        for y in 33..37 {
            let left = w.get(26, y);
            w.set(26, y, left.with_crack_right(true));
            let right = w.get(34, y);
            w.set(34, y, right.with_crack_right(true));
        }
        compute_world_distances(&mut w);
        for y in 33..37 {
            for x in 27..35 {
                w.schedule_structural_check(x, y);
            }
        }
        run(&mut w, 300);

        let f = w.structural_failures;
        assert!(
            f.overloaded + f.unsupported > 0,
            "a buried slab must still be judged and still fail -- confinement decides what a failure produces, not whether it happens"
        );
        assert!(f.confined > 0, "and that failure must be recorded as the confined kind");
    }

    #[test]
    fn a_stone_tower_stands_however_tall_it_gets() {
        // The other half of the direction-weighting claim, and the reason
        // the test above had to change shape. Rock is strong in
        // compression: a wall or pillar built up from the ground is
        // supported at every step, so it must not snap at the same reach a
        // cantilever does. One cell wide and unattached, so nothing else --
        // this is purely `support_cost_below: 0` doing the work.
        let mut w = test_world();
        let span = w.materials.get(material::STONE).max_unsupported_span as i32;
        assert!(span < 40, "test needs a tower taller than the span to be meaningful");
        for y in (63 - 40)..=63 {
            w.set(30, y, Cell::new(material::STONE, 0));
        }
        w.schedule_structural_check(30, 63 - 40); // the topmost cell
        run(&mut w, 400);

        for y in (63 - 40)..=63 {
            assert_eq!(w.get(30, y).material, material::STONE, "a ground-anchored tower crumbled at y={y}");
        }
    }

    #[test]
    fn an_overloaded_wood_beam_comes_down_as_deadwood() {
        // Architecture item 9: structural integrity extended to `Plant`,
        // exercised with wood's own numbers (`max_unsupported_span` 8, so a
        // capacity base of 32, against `breaks_into: "deadwood"`).
        //
        // Was `a_wood_beam_exceeding_its_span_breaks_into_deadwood`, and it
        // asserted the tip specifically became deadwood *in place*. It
        // failed on the switch to load, reporting `EMPTY` -- which is the
        // new model working rather than a regression: a cantilever fails at
        // its root and the whole span drops, so the tip is now lifted out
        // of the grid as part of a falling body instead of crumbling where
        // it stood. Pinning which of the two happened was pinning the
        // mechanism, so the claim is restated as the outcome: it is no
        // longer standing there as wood, and it turned into wood's own
        // debris somewhere.
        let mut w = test_world();
        let wood = w.materials.id_of("wood").unwrap();
        // A 12-cell horizontal beam anchored only at its left end (touching
        // the world's left edge). Root moment is 1+2+...+11 = 66 against a
        // capacity of 32, so it must give way.
        for x in 0..12 {
            w.set(x, 30, Cell::new(wood, 0));
        }
        w.schedule_structural_check(11, 30);
        run(&mut w, 200);

        assert_ne!(w.get(11, 30).material, wood, "an overloaded wood beam should not still be standing");
        let deadwood = w.materials.id_of("deadwood").unwrap();
        // Anywhere in the world: deadwood is a `Powder`, so once it is free
        // it falls like anything else and this beam has nothing under it.
        let debris_somewhere = (0..64).any(|x| (0..64).any(|y| w.get(x, y).material == deadwood))
            || w.chunk_bodies.iter().any(|b| b.cells.iter().any(|c| c.material == wood));
        assert!(debris_somewhere, "the beam vanished instead of becoming debris or a falling body");
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
        // **Deliberately NOT `test_world()`**: this one keeps the shipped
        // `chain_reach` leash on. The claim is end-to-end -- burning the
        // base brings the rest down -- and half of that end is
        // `fire.rs`'s burnout reporting itself via
        // `CellSurface::record_disturbance`, without which `TIGHT` refuses
        // the failure and the trunk hangs. Unleashing the world here would
        // pass whether or not the burnout reports anything, which is
        // exactly the vacuous guard `CLAUDE.md` warns about. The burn is
        // at x=0 and the far end at x=11, inside TIGHT's 16.
        let mut w = World::new(Rect::new(0, 0, 63, 63));
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
        // A cantilever, not the vertical stack this used to use -- see
        // `a_stone_cantilever_exceeding_its_tolerance_breaks_free`
        // for why a stack no longer breaks at all.
        let span = w.materials.get(material::STONE).max_unsupported_span as i32;
        for x in 0..=(span + 6) {
            w.set(x, 30, Cell::new(material::STONE, 0));
        }
        let (tx, ty) = (span + 6, 30); // the tip, past the span, will break
        assert_eq!(w.field_at(tx, ty).pressure, 0.0, "test setup should start at ambient pressure");

        w.schedule_structural_check(tx, ty);
        run(&mut w, 200);

        // Either outcome counts as "it broke": the cell is now rubble in
        // place, or it left the grid entirely as part of a chunk body. A
        // failure takes the whole overhang now, so the tip is usually lifted
        // rather than converted -- pinning it to rubble specifically would
        // be pinning which of the two happened, which is not what this test
        // is about.
        let debris = stone_debris(&w);
        let broke = w.get(tx, ty).material == debris || w.get(tx, ty).material == material::EMPTY;
        assert!(broke, "test setup should have broken the tip, found {:?}", w.get(tx, ty).material);
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
        let debris = stone_debris(&w);
        assert_eq!(w.get(3, 30).material, material::STONE, "the intact bridge should not have collapsed yet");

        // Cut the support at the anchored end -- the rest of the bridge is
        // now 1 cell further from any anchor than it was.
        w.set(0, 30, Cell::EMPTY);
        w.schedule_structural_check_around(0, 30);
        run(&mut w, 200);

        assert_eq!(w.get(3, 30).material, debris, "the far end of the bridge never collapsed after its support was cut");
    }

    /// `World::default` and `CHAIN_MODES[0]` are two copies of the same
    /// number, and a drift between them would mean a fresh world does not
    /// behave like the mode `F9` names on screen -- the same class of bug
    /// `the_defaults_are_the_first_feel_of_each_list` guards for the
    /// player tuning.
    #[test]
    fn the_default_chain_reach_is_the_first_chain_mode() {
        assert_eq!(
            World::new(Rect::new(0, 0, 63, 63)).chain_reach,
            CHAIN_MODES[0].reach,
            "a fresh world must start in the mode F9 reports as the first one"
        );
    }

    /// A reach limit keeps damage near what was actually disturbed, and
    /// the paired case proves it is the *limit* doing it rather than the
    /// scene having become unbreakable.
    ///
    /// The same bridge as above, cut the same way. Unlimited reach is the
    /// shipped behaviour and the far end must still come down; with the
    /// reach pulled in and the cut recorded far away, it must not.
    ///
    /// **The cut records no disturbance of its own here**, deliberately:
    /// the test sets one somewhere else, so that "near a disturbance" is
    /// the only thing separating the two runs. Letting the cut record
    /// itself would license the collapse in both and the test would pass
    /// while exercising nothing.
    #[test]
    fn a_reach_limit_keeps_damage_near_what_was_disturbed() {
        fn cut_bridge(reach: i32, disturb_at: Option<(i32, i32)>) -> World {
            let mut w = test_world();
            w.chain_reach = reach;
            for x in 0..4 {
                w.set(x, 30, Cell::new(material::STONE, 0));
            }
            w.schedule_structural_check(3, 30);
            run(&mut w, 200);
            w.set(0, 30, Cell::EMPTY);
            w.schedule_structural_check_around(0, 30);
            if let Some((dx, dy)) = disturb_at {
                // Extent `0`: an erasure has no wound of its own to
                // license, and the point of this scene is the *reach*.
                w.record_disturbance(dx, dy, 0);
            }
            run(&mut w, 200);
            w
        }

        let debris = stone_debris(&test_world());
        let unlimited = cut_bridge(i32::MAX, None);
        assert_eq!(
            unlimited.get(3, 30).material,
            debris,
            "with no reach limit the far end must still collapse -- otherwise this proves nothing about the limit"
        );

        // Reach 4, and the only disturbance is 40 cells away: the bridge
        // is outside it, so nothing there may fail.
        let limited = cut_bridge(4, Some((50, 30)));
        assert_eq!(
            limited.get(3, 30).material,
            material::STONE,
            "damage far from anything that was disturbed must not happen once a reach is set"
        );

        // ...and the same limit with the disturbance *on* the bridge lets
        // it go, which is what stops this being "a reach limit makes rock
        // invincible".
        let licensed = cut_bridge(4, Some((1, 30)));
        assert_eq!(
            licensed.get(3, 30).material,
            debris,
            "a reach limit must still permit damage next to what was actually disturbed"
        );
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
        // A cantilever, matching the shape
        // `a_stone_cantilever_exceeding_its_tolerance_breaks_free`
        // now uses -- a vertical stack no longer breaks at any height, so
        // it can no longer show a span change doing anything.
        const LENGTH: i32 = 24;
        for x in 0..=LENGTH {
            w.set(x, 30, Cell::new(material::STONE, 0));
        }
        let stone = w.materials.id_of("stone").unwrap();
        assert!(
            w.materials.get(stone).max_unsupported_span < LENGTH as u16,
            "test assumes the shipped span is shorter than the cantilever, or widening it proves nothing"
        );
        // Widen stone's tolerance via a manufactured, more tolerant material
        // set loaded through a synthetic reload rather than mutating the
        // registry directly, since that's the only public way content
        // changes at runtime. Deliberately says nothing about
        // `attached_span_bonus` or `support_cost_*`, which also checks that
        // their defaults leave a one-cell-thick beam relaxing at a flat 1
        // per lateral step exactly as it did before those fields existed.
        let dir = std::env::temp_dir().join("pixel-physics-m17-span-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("stone.ron"),
            "(name: \"stone\", kind: Solid, density: 2.5, colors: [(128,128,132)], max_unsupported_span: 40, breaks_into: \"gravel\")",
        )
        .unwrap();
        w.materials.reload(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        w.schedule_structural_check(LENGTH, 30);
        run(&mut w, 400);
        assert_eq!(w.get(LENGTH, 30).material, material::STONE, "widening the span at runtime should have let the long cantilever stand");
    }

    // --- Confinement: the minimum self-supporting thickness --------------
    //
    // The three confinement tests that lived here are deleted, not
    // ported. They asserted that a slab of exactly `2r + 1` thickness held
    // itself up, which was true of a mechanism that no longer exists --
    // `is_confined` is gone, replaced by stated attachment. Worse, they had
    // become *vacuous*: with the mechanism removed they passed only because
    // an undisturbed slab's cells sit at a self-consistent distance of 0 and
    // stop rescheduling, so nothing was being exercised at all. A test that
    // cannot fail is worse than no test. What they were reaching for --
    // "thickness should decide how far rock spans" -- is now
    // `MaterialDef::attached_span_bonus`, and `undercut` in
    // `examples/filmstrip.rs` is the case that actually shows it.

    /// A one-cell cantilever `length` long, anchored at the left world edge.
    fn cantilever(length: i32) -> World {
        let mut w = test_world();
        for x in 0..=length {
            w.set(x, 30, Cell::new(material::STONE, 0));
        }
        w
    }

    #[test]
    fn a_scored_span_gives_way_where_an_intact_one_holds() {
        // The whole point of damage that accumulates: the same beam, the
        // same reach, different outcome because one of them has been worked
        // at. Without this a crack is a decal.
        let probe = test_world();
        let span = probe.materials.get(material::STONE).max_unsupported_span as i32;
        let length = span - 4; // comfortably within reach when undamaged
        drop(probe);

        let mut intact = cantilever(length);
        intact.schedule_structural_check(length, 30);
        run(&mut intact, 600);
        assert_eq!(
            intact.get(length, 30).material,
            material::STONE,
            "test setup: an undamaged beam this short should hold, or the comparison below proves nothing"
        );

        let mut scored = cantilever(length);
        // Score the beam near its root, where a cantilever is most stressed.
        for x in 3..6 {
            let c = scored.get(x, 30).with_crack_down(true).with_crack_right(true);
            scored.set(x, 30, c);
        }
        scored.schedule_structural_check(length, 30);
        scored.schedule_structural_check_around(4, 30);
        run(&mut scored, 600);

        assert_ne!(
            scored.get(length, 30).material,
            material::STONE,
            "a beam fractured at its root should give way at a reach the same beam holds undamaged"
        );
    }

    #[test]
    fn brushed_stone_is_laid_down_intact_and_attachment_is_a_bonus_not_immunity() {
        // Was `brushed_stone_is_foreground_and_unattached_terrain_is_not`,
        // which pinned the opposite claim. Restated rather than deleted,
        // because the thing it was really guarding is still live and still
        // the crux: **attachment must buy capacity, never immunity.**
        //
        // What changed is only which material starts with it. Reported from
        // play: "I don't want my constructions to just immediately fall
        // down or to have to work at all to make sure they are structurally
        // stable, but I do want it to break realistically." So undamaged
        // material -- terrain and construction alike -- is held, and damage
        // is what makes it answerable to physics.
        //
        // The failure this must keep catching is the one four earlier
        // support models died of: if attachment were an *exemption* rather
        // than a multiplier, one chip would level a castle, because a
        // structure standing only by exemption has no answer the moment
        // anything asks. See `Reports/building-rethink.md` §3a.
        let mut w = test_world();
        w.paint_capsule((32, 32), (32, 32), 6, material::STONE, 1.0);
        assert!(w.get(32, 32).attached(), "brushed stone should be laid down intact");

        let m = w.materials.get(material::STONE);
        assert!(m.attached_span_bonus > 1, "intact rock should get a real bonus, or being undamaged buys nothing");
        assert!(m.attached_span_bonus < u16::MAX, "the bonus must stay finite -- immunity is the failure mode, not the goal");

        // And it is genuinely a multiplier: the same cell damaged carries
        // strictly less, which is what makes a wound spread only as far as
        // the damage actually reached.
        let intact = crate::sim::load::capacity(&w, 32, 32);
        let damaged = w.get(32, 32).with_attached(false);
        w.set(32, 32, damaged);
        let loosened = crate::sim::load::capacity(&w, 32, 32);
        assert!(loosened < intact, "damage must cost real capacity, found {loosened} against {intact}");
        assert!(loosened > 0, "damaged rock must still carry something, or a chip cascades through the whole structure");
    }
    #[test]
    fn an_unsupported_foreground_blob_does_not_hang_in_mid_air() {
        // The reversal above, stated as behaviour. Known gap it does *not*
        // cover: this only asserts the blob stops being intact stone, not
        // that it comes apart into a satisfying mix of chunks and rubble.
        // That is the fracture-distribution work -- "everything either
        // disintegrates into powder or breaks off as a large piece; there
        // needs to be more rubble" -- and it is not built yet.
        let mut w = test_world();
        w.paint_capsule((32, 25), (32, 25), 6, material::STONE, 1.0);
        // The CA sweep runs too, and has to. Rubble is a `Powder`, and
        // `is_resting_on_ground` counts powder directly below as support --
        // so debris that has not been given a chance to fall away props up
        // whatever is above it, and the blob stops mid-collapse. That is a
        // harness artifact rather than a rule to change: in the running game
        // the sweep clears it in the same frames.
        for _ in 0..900 {
            w.begin_step();
            scheduler::step(&mut w);
            w.end_step();
            crate::sim::rigid::step_chunk_bodies(&mut w);
            update::step(&mut w);
        }
        assert_ne!(w.get(32, 25).material, material::STONE, "a foreground blob with nothing under it should not hang in mid-air");
    }

    // --- Organism-owned cells: the real `organism_structural_tick` -------
    //
    // Tree rewrite step 5: `organism_structural_tick` was a documented,
    // deliberate no-op until now. These mirror the plain-`Solid` span
    // tests above exactly, just with organism-owned wood (`organism_id !=
    // 0`, so `tick` routes through `organism_structural_tick`'s own BFS
    // instead of the aux-cached relaxation) -- built directly via
    // `push_organism`/`set` rather than simulating real `Grow` calls,
    // since growth's own direction is randomized and irrelevant to what's
    // being tested here.

    /// Pin `wood`'s cantilever reach for one test world.
    ///
    /// **A test about the mechanism must not depend on the shipped
    /// constant.** `max_cantilever_reach` is set from a measurement of a
    /// real stand (support max 77, so 96 with headroom) and will be
    /// re-derived again when the load model grows the plant's own weight.
    /// The beams below are 6 and 12 cells, so against the shipped value
    /// every one of them would sit comfortably inside the limit and these
    /// tests would pass while exercising nothing -- the exact failure
    /// `CLAUDE.md` describes as a superseded mechanism's tests still
    /// passing.
    ///
    /// Same synthetic-reload route `editing_max_unsupported_span_while_
    /// running_changes_what_stands` uses, since that is the only public way
    /// content changes at runtime.
    fn pin_wood_reach(w: &mut World, reach: u16) {
        // Unique per call: the tests run in parallel and three of them pin
        // the same reach, so a name keyed only on the value had them
        // deleting the directory out from under each other mid-reload.
        static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("pixel-physics-organism-reach-{reach}-{seq}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("wood.ron"),
            format!(
                "(name: \"wood\", kind: Plant, density: 0.9, colors: [(92,64,40)],                  max_unsupported_span: 8, max_cantilever_reach: {reach}, breaks_into: \"deadwood\")"
            ),
        )
        .unwrap();
        w.materials.reload(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }

    fn organism_wood_cell(w: &mut World, organism_id: u16) -> Cell {
        let wood = w.materials.id_of("wood").unwrap();
        Cell::new(wood, 0).with_organism_id(organism_id).with_aux(organism::pack_cell_type(organism::CellType::MatureBody))
    }

    /// §9.2 of `Reports/physical-trees-design-2026-08-23.md`, guarded
    /// rather than filed: `schedule_organism_neighbours` walked
    /// `NEIGHBOURS_4` while `Grow` places organism cells at **eight**, so a
    /// cascade through a crown stopped at the first diagonal.
    ///
    /// **This fails against the four-neighbour walk** -- the diagonal twig
    /// is the *only* same-organism neighbour here, so the returned fan-out
    /// is empty rather than short, which is the difference between a
    /// cascade that under-reaches and one that does not happen at all.
    #[test]
    fn a_diagonally_attached_twig_is_rescheduled_by_its_neighbour() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let organism_id = w.push_organism(tree_species).expect("a fresh world has organism slots free");
        let branch = organism_wood_cell(&mut w, organism_id);
        w.set(20, 20, branch);
        // Diagonal only: nothing is orthogonally adjacent to (20, 20).
        let twig = organism_wood_cell(&mut w, organism_id);
        w.set(21, 21, twig);

        let sites = schedule_organism_neighbours(&w, 20, 20, organism_id);
        assert_eq!(
            sites.iter().map(|s| (s.x, s.y)).collect::<Vec<_>>(),
            vec![(21, 21)],
            "a twig attached only at a corner must still be re-asked when its neighbour breaks -- `Grow` placed it there at eight"
        );
    }

    /// The other half of the same rule, and the reason it is a *test* and
    /// not just the eight-ring: a diagonal neighbour belonging to a
    /// **different** organism, or to no organism, is not part of this
    /// cascade and must not be dragged into it.
    #[test]
    fn schedule_organism_neighbours_ignores_a_diagonal_that_is_not_ours() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let mine = w.push_organism(tree_species).expect("a fresh world has organism slots free");
        let theirs = w.push_organism(tree_species).expect("a fresh world has organism slots free");
        let branch = organism_wood_cell(&mut w, mine);
        w.set(20, 20, branch);
        let other = organism_wood_cell(&mut w, theirs);
        w.set(21, 21, other);
        w.set(19, 19, Cell::new(material::STONE, 0));

        assert!(
            schedule_organism_neighbours(&w, 20, 20, mine).is_empty(),
            "neither another organism's tissue nor inert rock is part of this organism's cascade"
        );
    }

    /// The piece walk `organism_structural_tick` hands to the fragment
    /// ladder: 8-connected, same-organism, **and detached**.
    ///
    /// The membership test is the part worth guarding. Taking the whole
    /// connected run instead would rip still-anchored tissue out with the
    /// piece -- the amputation failure in a new costume -- and a crown half
    /// of which is still held has not come off.
    #[test]
    fn a_detached_piece_walk_takes_the_diagonal_run_and_stops_at_held_tissue() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let organism_id = w.push_organism(tree_species).expect("a fresh world has organism slots free");
        // A diagonal staircase of five cells, plus one orthogonal
        // neighbour of the last that is still anchored.
        for i in 0..5 {
            let cell = organism_wood_cell(&mut w, organism_id);
            w.set(20 + i, 20 + i, cell);
        }
        let held = organism_wood_cell(&mut w, organism_id);
        w.set(25, 24, held);
        for i in 0..5 {
            w.organism_cell_mut(20 + i, 20 + i).expect("sidecar").support = u16::MAX;
        }
        w.organism_cell_mut(25, 24).expect("sidecar").support = 3;

        let piece = detached_organism_piece(&w, 20, 20, organism_id);
        assert_eq!(
            piece,
            (0..5).map(|i| (20 + i, 20 + i)).collect::<Vec<_>>(),
            "the whole diagonal run has come off and must arrive as one piece; the anchored cell beside it has not"
        );
    }

    #[test]
    fn an_organism_tree_beam_within_its_span_stays_wood() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let organism_id = w.push_organism(tree_species).expect("an organism slot is free");
        pin_wood_reach(&mut w, 8);
        // Anchored at the left end by a stone cell directly below it --
        // organism_is_supported's own generalization of "touches BEDROCK"
        // to "touches Solid ground". 6 cells, within wood's span of 8.
        w.set(0, 31, Cell::new(material::STONE, 0));
        for x in 0..6 {
            let cell = organism_wood_cell(&mut w, organism_id);
            w.set(x, 30, cell);
        }
        w.schedule_structural_check(5, 30); // the far end, distance 5
        run_organisms(&mut w, 200);

        for x in 0..6 {
            assert_eq!(w.get(x, 30).organism_id(), organism_id, "an anchored, in-span organism beam broke (or lost its organism_id) at x={x}");
        }
    }

    #[test]
    fn an_organism_tree_beam_exceeding_its_span_breaks_into_deadwood() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let organism_id = w.push_organism(tree_species).expect("an organism slot is free");
        pin_wood_reach(&mut w, 8);
        w.set(0, 31, Cell::new(material::STONE, 0));
        // 12 cells -- longer than the reach pinned above, so the far end (distance
        // 11) is unsupported once actually checked.
        for x in 0..12 {
            let cell = organism_wood_cell(&mut w, organism_id);
            w.set(x, 30, cell);
        }
        w.schedule_structural_check(11, 30);
        run_organisms(&mut w, 200);

        let deadwood = w.materials.id_of("deadwood").unwrap();
        assert_eq!(w.get(11, 30).material, deadwood, "an over-span organism-owned wood cell should have broken into deadwood");
        assert_eq!(w.get(11, 30).organism_id(), 0, "broken-free debris should no longer belong to the organism");
    }

    #[test]
    fn cutting_an_organism_trees_support_collapses_the_far_side() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let organism_id = w.push_organism(tree_species).expect("an organism slot is free");
        // Away from x=0 deliberately -- touching the world edge itself
        // would anchor the beam via the same out-of-bounds-reads-as-
        // BEDROCK sentinel `is_anchor` relies on elsewhere, which is
        // exactly the confound an earlier version of this test tripped
        // over (the stone anchor below could be removed with no effect,
        // since the beam's own base cell at x=0 was already anchored by
        // the edge regardless). x=10 keeps every neighbour used here well
        // inside the 64-wide test world.
        w.set(10, 31, Cell::new(material::STONE, 0));
        // 4 cells -- comfortably within span 8 while anchored.
        for x in 10..14 {
            let cell = organism_wood_cell(&mut w, organism_id);
            w.set(x, 30, cell);
        }
        w.schedule_structural_check(13, 30);
        run_organisms(&mut w, 200);
        assert_eq!(w.get(13, 30).organism_id(), organism_id, "test setup: the intact, anchored beam should not have broken yet");

        // Cut the anchor itself -- every cell in the beam is now
        // unsupported, not just one step further away, since it was the
        // *only* thing touching Solid ground.
        w.set(10, 31, Cell::EMPTY);
        w.schedule_structural_check_around(10, 31);
        w.schedule_structural_check(10, 30);
        run_organisms(&mut w, 200);

        assert_ne!(w.get(13, 30).organism_id(), organism_id, "the far end of the organism beam never collapsed after its only anchor was cut");
    }
    /// The other half of §6d's mirror: weight breaks a branch.
    ///
    /// A paired comparison of two identical cantilevers, one bare and one
    /// buried, for the same reason the soil-reinforcement test uses one --
    /// whether a given span survives at all depends on `max_unsupported_
    /// span`, the anchor rule and the search bound, none of which this is
    /// about. Comparing two runs cancels all of it and leaves only the
    /// load term.
    #[test]
    fn a_loaded_branch_breaks_at_a_shorter_span_than_a_bare_one() {
        let build = |loaded: bool| -> World {
            let mut w = test_world();
            let wood = w.materials.id_of("wood").expect("wood is compiled in");
            let organism = w.push_organism(w.species.id_of("tree").expect("tree is compiled in")).expect("an organism slot is free");
            // Pinned so the load term is what decides, not the shipped
            // reach -- a 9-cell branch is far inside the real 96.
            pin_wood_reach(&mut w, 8);
            // A trunk on the ground, and a cantilever reaching out from it.
            for y in 30..40 {
                w.set(10, y, Cell::new(material::STONE, 0));
            }
            // Nine cells, so the tip sits exactly `wood`'s span (8) from
            // the anchor. That length is the whole point: a bare branch
            // survives at 8 and a loaded one, whose effective span is 7,
            // does not. Shorter and both live; longer and both die, which
            // is what the first version of this test measured (8 cells kept
            // in both runs) and why it proved nothing.
            for x in 11..=19 {
                w.set(x, 30, Cell::new(wood, 0).with_organism_id(organism).with_aux(organism::pack_cell_type(organism::CellType::MatureBody)));
            }
            if loaded {
                // Sand piled directly on the branch.
                for x in 11..=19 {
                    w.set(x, 29, Cell::new(material::SAND, 0));
                }
            }
            // Reactive, never proactive: nudge the far end so the check runs.
            w.schedule_structural_check_around(19, 30);
            w
        };

        let surviving = |w: &World| -> usize {
            let wood = w.materials.id_of("wood").unwrap();
            (11..=19).filter(|&x| w.get(x, 30).material == wood).count()
        };

        let mut bare = build(false);
        let mut loaded = build(true);
        run_organisms(&mut bare, 200);
        run_organisms(&mut loaded, 200);

        assert!(
            surviving(&loaded) < surviving(&bare),
            "a branch buried in sand should break back further than a bare one: loaded kept {} cells, bare kept {}",
            surviving(&loaded),
            surviving(&bare)
        );
    }

    /// A slab of attached stone with room for a walk to run through it.
    fn massif(w: &mut World) {
        for y in 0..64 {
            for x in 0..64 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
    }

    /// A square of massif centred on `at`, as a failing region would
    /// arrive: `extent` is what `crush_in_place` measures off it, so this
    /// is how a test says "a section this big gave way here".
    fn block_region(at: (i32, i32), half: i32) -> Vec<(i32, i32)> {
        (at.1 - half..=at.1 + half).flat_map(|y| (at.0 - half..=at.0 + half).map(move |x| (x, y))).collect()
    }

    /// W1's crush half, and **the single most dangerous line in this
    /// change**: a confined crush must leave attachment and the scheduler
    /// alone. Unbracing confined rock tells the load model the inside of a
    /// mountain has come free, and rescheduling is what turns a crush into
    /// a treadmill (1,120 confined failures per 400 frames on dead-still
    /// material) -- both recorded as measured dead ends in
    /// `crush_in_place`'s own doc.
    ///
    /// **Written against the crush, not against the walker.** It used to
    /// call `walk_fissures(.., detach: false)`, which after the fabric
    /// landed would have gone on passing whatever the crush did -- exactly
    /// `CLAUDE.md`'s superseded test that runs, passes and exercises
    /// nothing. `explosion::sever` *does* call `detach_around_crack` and
    /// `schedule_structural_check_around`, and reaching for it here is the
    /// obvious way to get this wrong, so the assertion now sits on the
    /// function that would.
    /// A crush the leash squeezed must say so, not look idempotent.
    ///
    /// The two ways a `Section` crush writes nothing are opposites:
    /// idempotent re-crushing of rock already cracked (which must **not**
    /// propagate, or it is the 1,120-per-400-frames treadmill), and a disc
    /// the licence shrank below the grain (which **must** propagate, or the
    /// distance wavefront is dropped). `Crush::leashed` is what tells them
    /// apart; before it existed both returned `0` and `tick` treated them
    /// identically.
    ///
    /// Paired against SPREAD, where `licence_headroom` is `None` and the
    /// arithmetic is untouched -- without that arm "leashed is false" also
    /// passes against a build where the clamp never runs.
    #[test]
    fn a_crush_the_leash_shrank_is_not_a_crush_that_had_nothing_to_do() {
        fn crush(reach: i32, disturbance: (i32, i32)) -> Crush {
            let mut w = test_world();
            massif(&mut w);
            compute_world_distances(&mut w);
            w.chain_reach = reach;
            w.record_disturbance(disturbance.0, disturbance.1, 0);
            crush_in_place(&mut w, &block_region((32, 32), 8), (32, 32), CrushedObject::Section)
        }

        let free = crush(i32::MAX, (32, 32));
        assert!(free.fresh > 0, "test setup: at SPREAD this crush must write something, it wrote {}", free.fresh);
        assert!(!free.leashed, "SPREAD has no licence to be squeezed by, yet the crush reported one");

        // 10 cells away at TIGHT (16): the cell is comfortably licensed, so
        // the region survives the clip -- but the headroom left for a
        // *radius* is 6, under `CrushedObject::min_joint_reach`'s floor of
        // 10. That is the annulus around every licence boundary, not a
        // knife edge on it.
        let squeezed = crush(16, (42, 32));
        assert!(
            squeezed.leashed,
            "the licence shrank the disc from 10 to 6 and the crush did not report it, so `tick` cannot tell this from idempotence"
        );
    }

    #[test]
    fn a_crush_neither_unbraces_the_rock_nor_reschedules_it() {
        let mut w = test_world();
        massif(&mut w);
        let scheduled_before = w.active_site_count();
        let region = block_region((32, 32), 8);
        let written = crush_in_place(&mut w, &region, (32, 32), CrushedObject::Section).fresh;

        assert!(written > 0, "test setup: the crush should have severed something");
        assert_eq!(w.active_site_count(), scheduled_before, "a confined crush must schedule no structural checks");
        let unattached = (0..64).map(|x| (0..64).filter(|&y| !w.get(x, y).attached()).count()).sum::<usize>();
        assert_eq!(unattached, 0, "a confined crush must not unbrace anything: {unattached} cells lost attachment");
        // And it must not *remove* anything either. A blast opens its near
        // joints into one-cell seams of void and grit; rock with nowhere to
        // go has to stay where it is, so the crush scores and never opens.
        let holes = (0..64).map(|x| (0..64).filter(|&y| w.get(x, y).material != material::STONE).count()).sum::<usize>();
        assert_eq!(holes, 0, "a confined crush removed {holes} cells -- it must score joints, never open them");
    }

    /// The other half of the same flag, and the reason it exists: a blast's
    /// reach only gets into the structural model through the halo.
    #[test]
    fn a_blast_walk_unbraces_and_reschedules_what_it_scores() {
        let mut w = test_world();
        massif(&mut w);
        let scheduled_before = w.active_site_count();
        let written = walk_fissures(&mut w, (32, 32), 0.7, 3, 30, 4, true);

        assert!(written > 0, "test setup: the walk should have scored something");
        assert!(w.active_site_count() > scheduled_before, "a blast's fissures must schedule the checks that feed the load model");
        let unattached = (0..64).map(|x| (0..64).filter(|&y| !w.get(x, y).attached()).count()).sum::<usize>();
        assert!(unattached > 0, "a blast's fissures must unbrace the rock either side of them");
    }

    /// Idempotence on re-crush, which has no flag to record it and never
    /// needed one: a crack is a bit and the joint set is a pure function of
    /// position, so a second confined failure over the same rock reveals
    /// the same edges and manufactures nothing. `tick`'s "a crush that
    /// wrote nothing has nothing to propagate" guard is what stopped a
    /// crushed pocket re-failing 1,120 times every 400 frames, and it reads
    /// exactly this number.
    ///
    /// The fabric makes this stronger than the walker did rather than
    /// merely keeping it: a walker was idempotent only because its wander
    /// was position-keyed *from the same origin*, so a second failure a
    /// cell over drew a whole new star. Two crushes at two neighbouring
    /// sites now reveal the *same* edges, which is the second assertion
    /// here and is why a fabric crush cannot run away the way W4's did.
    #[test]
    fn crushing_the_same_rock_twice_writes_nothing_the_second_time() {
        let mut w = test_world();
        massif(&mut w);
        let region = block_region((32, 32), 8);
        let first = crush_in_place(&mut w, &region, (32, 32), CrushedObject::Section).fresh;
        let second = crush_in_place(&mut w, &region, (32, 32), CrushedObject::Section).fresh;
        assert!(first > 0, "test setup: the first crush should have severed something");
        assert_eq!(second, 0, "a re-crush wrote {second} fresh edges; confined damage must not accumulate");
        // And the property that bounds a cascade rather than one repeat:
        // sweep a 5x5 grid of *neighbouring* failures over the same patch,
        // then sweep it again. The first sweep finds whatever the first
        // crush had not reached; the second finds nothing at all, because
        // between them they have revealed every joint there is in that
        // rock. A walker drew a fresh star per origin and had no such
        // ceiling -- which is how W4 reached 84,051 cracked cells.
        let sweep = |w: &mut World| {
            let mut total = 0;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let at = (32 + dx * 3, 32 + dy * 3);
                    total += crush_in_place(w, &block_region(at, 8), at, CrushedObject::Section).fresh;
                }
            }
            total
        };
        let _ = sweep(&mut w);
        let again = sweep(&mut w);
        assert_eq!(again, 0, "a second sweep of 25 neighbouring crushes wrote {again} more edges -- confined damage is accumulating without a ceiling");
    }

    /// And the inverse for a blast, which is why the crack-tip bonus had to
    /// be ported: position-keyed wander means a repeat charge in the same
    /// spot retraces its own fissures *exactly*, so without extra budget
    /// bought at a pre-existing crack tip the second shot would be a
    /// visual no-op.
    #[test]
    fn a_second_charge_in_the_same_place_drives_its_fissures_deeper() {
        let mut w = test_world();
        massif(&mut w);
        let first = walk_fissures(&mut w, (32, 32), 0.7, 3, 30, 4, true);
        let second = walk_fissures(&mut w, (32, 32), 0.7, 3, 30, 4, true);
        assert!(first > 0, "test setup: the first charge should have scored something");
        assert!(second > 0, "a repeat charge wrote nothing at all -- the crack-tip bonus is not reaching fresh rock");
    }

    /// **The resume pin.** A walk put down and picked up must draw exactly
    /// the crack it would have drawn in one go.
    ///
    /// This is the one place the incremental driver could be quietly wrong
    /// and still look plausible on a contact sheet: `Walker` carries the
    /// path at sub-cell precision (`fx`/`fy`), and the obvious way to store
    /// less is to re-derive them from `pos` on resume. That re-centres the
    /// walker in its cell every frame, throwing away the fractional offset
    /// the wander has built up -- the same quantisation that turned an
    /// earlier axis-stepping walker into right-angled "criss cross" runs,
    /// only applied once per frame instead of once per cell. The result is
    /// a straighter crack, which reads as *fine* unless you have the
    /// one-shot version beside it.
    ///
    /// One ray and no forks on purpose: with a fork pool in play the two
    /// drivers legitimately differ (see `FissureWalks`' own doc), and that
    /// difference would mask this one.
    #[test]
    fn a_walk_resumed_a_step_at_a_time_draws_the_same_crack_as_one_run() {
        let cracked_cells = |w: &World| -> Vec<(i32, i32)> {
            (0..64).flat_map(|y| (0..64).map(move |x| (x, y))).filter(|&(x, y)| w.get(x, y).cracked()).collect()
        };

        let mut one_shot = test_world();
        massif(&mut one_shot);
        let mut whole = FissureWalks::new((32, 32), 0.7, 1, 40, 0, false);
        let written_whole = whole.run_to_completion(&mut one_shot);

        let mut piecewise = test_world();
        massif(&mut piecewise);
        let mut staged = FissureWalks::new((32, 32), 0.7, 1, 40, 0, false);
        let mut written_staged = 0;
        for _ in 0..200 {
            written_staged += staged.advance(&mut piecewise, 1);
            if staged.is_done() {
                break;
            }
        }

        assert!(written_whole > 10, "test setup: the walk should have scored a real crack, got {written_whole}");
        assert!(staged.is_done(), "the staged walk never finished inside its step budget");
        assert_eq!(written_whole, written_staged, "the staged walk wrote a different number of cells than the one-shot walk");
        assert_eq!(
            cracked_cells(&one_shot),
            cracked_cells(&piecewise),
            "the staged walk drew a different crack -- sub-cell position is not surviving the resume"
        );
    }

    /// **The severing pin: a crack that goes all the way round a piece must
    /// actually cut it off.**
    ///
    /// `FLAG_CRACK_RIGHT`'s own doc is the promise this tests -- *"a crack
    /// has gone all the way around a piece" needs no detection of its own;
    /// the piece simply stops being able to reach an anchor* -- and until
    /// this test existed nothing checked that the walker's output could
    /// keep it. It could not: `step_walker` carries the path at sub-cell
    /// precision, so a unit step near 45 degrees crosses **both** cell
    /// boundaries at once and lands on a diagonal neighbour, leaving the
    /// cell between unvisited and unscored. The drawn crack is 8-connected
    /// while every consumer of it -- `load::is_supported`,
    /// `load::detached_piece`, `rigid::take_fragment` -- walks
    /// `NEIGHBOURS_4`, so support zigzags straight through the line.
    ///
    /// Measured against the one-edge-per-visited-cell walker this replaced:
    /// the flood below escaped to the world border, and
    /// `load::failing_region` on the enclosed rock returned `None`. Both
    /// halves are kept, because they answer different questions -- the
    /// flood says *the geometry does not seal*, and the load call says *the
    /// model therefore never sees a piece*, and a fix that repaired one
    /// without the other would be worthless.
    ///
    /// **Drawn by the real walker, deliberately.** Hand-placed crack bits
    /// (which is what every other crack test in this file uses) test
    /// `edge_is_cracked`, not the thing that is wrong. The loop is a
    /// diamond rather than a rectangle for the same reason: an
    /// axis-aligned box is drawn almost entirely by axis-aligned steps,
    /// which were never the broken case. Every side of a diamond is a
    /// staircase of diagonal steps, which is exactly the case that leaked.
    /// Short brittle rays laid along each side, because a ray long enough
    /// to draw a whole side wanders off it -- `CRACK_WANDER` is 0.9 rad per
    /// cell and even `Brittle` kinks every 3 to 8 cells, so a side is drawn
    /// as overlapping stubs whose starts are exact.
    #[test]
    fn a_closed_crack_loop_isolates_what_it_encircles() {
        const C: i32 = 32;
        const R: i32 = 12;
        let mut w = test_world();
        massif(&mut w);

        // One stub per cell along each of the four sides, each running four
        // steps (about two cells) in the side's own direction. `Brittle`
        // holds its heading for at least `BRITTLE_RUN_MIN` = 3 cells, so a
        // stub this short is dead straight and the union is the diamond.
        let sides = [
            ((C, C - R), (1, 1)),   // top-right side, heading +x +y
            ((C + R, C), (-1, 1)),  // bottom-right
            ((C, C + R), (-1, -1)), // bottom-left
            ((C - R, C), (1, -1)),  // top-left
        ];
        let mut walks = FissureWalks::empty(false, CrackStyle::Brittle, 0);
        for ((sx, sy), (dx, dy)) in sides {
            let heading = (dy as f32).atan2(dx as f32);
            for t in 0..R {
                walks.add_ray((sx + dx * t, sy + dy * t), heading, 4, 0, 0);
            }
        }
        let written = walks.run_to_completion(&mut w);
        assert!(written > 20, "test setup: the ring should have scored a real crack, got {written} cells");

        // The engine's own crossing rule, flooded from the middle: this is
        // `load::is_supported`'s inner loop with the anchor test taken out,
        // so what it measures is exactly what the load model would see.
        let mut seen: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::from([(C, C)]);
        let mut queue = std::collections::VecDeque::from([(C, C)]);
        let mut escaped = false;
        while let Some((cx, cy)) = queue.pop_front() {
            if (cx - C).abs() + (cy - C).abs() > R + 2 {
                escaped = true;
                break;
            }
            for (dx, dy) in NEIGHBOURS_4 {
                let next = (cx + dx, cy + dy);
                if seen.contains(&next) || edge_is_cracked(&w, cx, cy, dx, dy) {
                    continue;
                }
                if !is_body_material(&w, w.get(next.0, next.1).material) {
                    continue;
                }
                seen.insert(next);
                queue.push_back(next);
            }
        }
        // A diamond of radius R holds 2R(R-1)+1 cells strictly inside its
        // perimeter. The band is wide because the ring is drawn by a walker
        // and not by a compass: it wanders a cell or two either way, and
        // whether a given perimeter cell ends up inside or outside the
        // flood is not the claim. The claim is that the flood is *bounded*
        // and is the size of the piece rather than the size of the world.
        let inside = 2 * R * (R - 1) + 1;
        assert!(
            !escaped,
            "the crack ring leaked: the flood reached {} cells and walked past the ring -- support crosses the line the walker drew",
            seen.len()
        );
        assert!(
            seen.len() as i32 > inside / 3,
            "the flood found only {} cells inside a ring enclosing about {inside} -- the ring cut the interior up rather than round it",
            seen.len()
        );

        // And the load model must agree, which is the half that matters on
        // screen. Judged one cell in from the ring: `is_structurally
        // _interesting` skips attached rock with no cracked edge and no
        // empty neighbour, so the dead centre of the piece is a cell the
        // model deliberately never looks at.
        let probe = (C, C - R + 2);
        let mut budget = u32::MAX;
        let failure = super::super::load::failing_region(&w, probe.0, probe.1, &mut super::super::load::Cache::default(), &mut budget)
            .expect("rock with a crack all the way round it must be judged as failing");
        assert_eq!(
            failure.mode,
            super::super::load::FailureMode::Unsupported,
            "the enclosed piece failed, but as an overload rather than for want of anything holding it up"
        );
        assert!(
            failure.region.len() as i32 > inside / 3,
            "the detached piece was {} cells inside a ring enclosing about {inside} -- the model found a crumb, not the chunk",
            failure.region.len()
        );
    }

    /// **The brittle style turns rarely and hard, where the wandering one
    /// turns constantly and softly.**
    ///
    /// Asserted on the *headings*, not on the picture, and that is the point
    /// of the test. The obvious world-level statistic -- how far the crack
    /// gets from where it started -- cannot tell these two apart at all:
    /// measured, a 40-step wandering ray reaches 37.5 cells and a brittle
    /// one 35.4, because the per-cell wander is a zero-mean random walk in
    /// the heading and mostly cancels itself over a short ray. It is not the
    /// *net* curvature that reads as organic on screen, it is that the
    /// curvature is spread evenly over every single cell, and only the turn
    /// sequence shows that.
    ///
    /// So: a brittle walk turns at a handful of cells out of sixty, and
    /// every turn it makes is at least `BRITTLE_KINK_MIN`. "No per-cell
    /// wander" is exactly the second half of that, stated as an assertion.
    #[test]
    fn a_brittle_crack_turns_rarely_and_sharply() {
        // The heading after every single step, read off the walker itself --
        // `mod tests` is inside this module, so the private state is
        // reachable and this needs no accessor of its own.
        let turns_of = |style: CrackStyle| -> Vec<f32> {
            let mut w = test_world();
            massif(&mut w);
            let mut walks = FissureWalks::empty(false, style, 0);
            walks.add_ray((32, 32), 0.7, 60, 0, 0);
            let mut headings = vec![walks.walkers[0].heading];
            for _ in 0..60 {
                walks.advance(&mut w, 1);
                headings.push(walks.walkers[0].heading);
                if walks.is_done() {
                    break;
                }
            }
            headings.windows(2).map(|p| p[1] - p[0]).filter(|d| d.abs() > 1e-6).collect()
        };

        let wander = turns_of(CrackStyle::Wander);
        let brittle = turns_of(CrackStyle::Brittle);
        assert!(wander.len() > 40, "test setup: the wandering walk should turn at nearly every cell, turned {} times", wander.len());
        assert!(
            brittle.len() * 3 < wander.len(),
            "the brittle walk turned {} times against the wandering walk's {} -- it is still wandering",
            brittle.len(),
            wander.len()
        );
        assert!(!brittle.is_empty(), "the brittle walk never turned at all -- a straight ray is the asterisk this replaced");
        for turn in &brittle {
            assert!(
                turn.abs() >= BRITTLE_KINK_MIN,
                "a brittle walk made a {turn:.3} rad turn -- kinks are sharp by definition, anything smaller is per-cell wander"
            );
            assert!(turn.abs() <= BRITTLE_SNAP_MAX + 1e-6, "a brittle kink of {turn:.3} rad is past the deflection ceiling");
        }
    }

    /// And it is still a property of the *rock*: same site, same star, every
    /// time. Position-keyed jitter only -- a `world.rng` draw anywhere in the
    /// segmented walker would put the blast into the replay draw order and
    /// stop a repeat charge retracing its own fissures.
    #[test]
    fn a_brittle_star_is_the_same_star_every_time() {
        let draw = || -> Vec<(i32, i32)> {
            let mut w = test_world();
            massif(&mut w);
            let mut walks = FissureWalks::empty(false, CrackStyle::Brittle, 0);
            for i in 0..3 {
                walks.add_ray((32, 32), 0.7 + i as f32, 30, 1, 0);
            }
            walks.run_to_completion(&mut w);
            (0..64).flat_map(|y| (0..64).map(move |x| (x, y))).filter(|&(x, y)| w.get(x, y).cracked()).collect()
        };
        let first = draw();
        assert!(first.len() > 20, "test setup: the star should have scored a real crack, got {}", first.len());
        assert_eq!(first, draw(), "the same site drew a different brittle star on the second run");
    }

    /// The crack tip writes its own heat, and only when asked to. A crush
    /// walk (`glow: 0`) must leave the rock exactly as cold as it found it --
    /// rock parting under load inside a mountain is not incandescent.
    #[test]
    fn only_a_glowing_walk_heats_the_rock_it_cracks() {
        let hot_cells = |glow: i16| -> usize {
            let mut w = test_world();
            massif(&mut w);
            let mut walks = FissureWalks::empty(true, CrackStyle::Brittle, glow);
            walks.add_ray((32, 32), 0.7, 30, 0, 0);
            walks.run_to_completion(&mut w);
            (0..64)
                .flat_map(|y| (0..64).map(move |x| (x, y)))
                .filter(|&(x, y)| w.get(x, y).temperature() > super::super::cell::AMBIENT_TEMPERATURE)
                .count()
        };
        assert_eq!(hot_cells(0), 0, "a walk with no glow heated the rock anyway");
        assert!(hot_cells(300) > 0, "a glowing walk left no heat behind at all");
    }

    /// W4's one hard rule. Erosion only: a cell may be dropped from a
    /// failing region, never added to it. Adding takes down rock the load
    /// model never judged, which is how a small failure becomes a large one
    /// and how the dig cascade is built.
    #[test]
    fn eroding_a_failing_boundary_only_ever_drops_cells() {
        let mut w = test_world();
        massif(&mut w);
        let region: Vec<(i32, i32)> = (20..30).flat_map(|y| (20..32).map(move |x| (x, y))).collect();
        let members: std::collections::HashSet<(i32, i32)> = region.iter().copied().collect();
        let eroded = erode_failing_boundary(&w, region.clone());

        assert!(eroded.len() <= region.len(), "erosion grew the region from {} to {}", region.len(), eroded.len());
        for cell in &eroded {
            assert!(members.contains(cell), "erosion invented a cell at {cell:?} that was never judged failing");
        }
        assert!(eroded.len() < region.len(), "a region buried in rock should have lost some of its boundary");
    }

    /// The interior is not a boundary. A cell with four neighbours inside
    /// the region has nothing to tear away from, so it must survive
    /// whatever its jitter says -- otherwise erosion is a uniform thinning
    /// and the piece comes down as lace.
    #[test]
    fn erosion_leaves_the_inside_of_a_failing_region_alone() {
        let mut w = test_world();
        massif(&mut w);
        let region: Vec<(i32, i32)> = (20..30).flat_map(|y| (20..32).map(move |x| (x, y))).collect();
        let kept: std::collections::HashSet<(i32, i32)> = erode_failing_boundary(&w, region).into_iter().collect();
        for y in 21..29 {
            for x in 21..31 {
                assert!(kept.contains(&(x, y)), "interior cell ({x}, {y}) was eroded");
            }
        }
    }

    /// The size floor, which is the difference between a graded outcome and
    /// an immunity: a failure eroded to nothing never comes down at all.
    #[test]
    fn a_small_failing_region_is_left_whole() {
        let mut w = test_world();
        massif(&mut w);
        let region: Vec<(i32, i32)> = (0..MIN_ERODIBLE_REGION as i32 - 1).map(|i| (20 + i, 20)).collect();
        assert_eq!(
            erode_failing_boundary(&w, region.clone()),
            region,
            "a region under the floor must come down whole, not be thinned into never falling"
        );
    }

    /// A piece with open air all round it is already whatever shape the
    /// world gave it. Only the boundary where it is tearing away from rock
    /// that is *staying* reads as a torn edge, so a free-standing region
    /// must come through untouched.
    #[test]
    fn erosion_only_bites_where_the_region_meets_standing_rock() {
        let mut w = test_world();
        let region: Vec<(i32, i32)> = (20..30).flat_map(|y| (20..32).map(move |x| (x, y))).collect();
        for &(x, y) in &region {
            w.set(x, y, Cell::new(material::STONE, 0));
        }
        assert_eq!(
            erode_failing_boundary(&w, region.clone()),
            region,
            "a region surrounded by air has no rock-facing boundary and must not be eroded"
        );
    }

    // --- R3a: fracture pacing -------------------------------------------

    /// `examples/filmstrip.rs`'s `ligament` scene, in miniature and in the
    /// suite: a big attached overhang hung off a cliff by a deliberately
    /// thin neck, with one structural check at the neck and nothing struck
    /// or erased. Returns the overhang's cells.
    ///
    /// Built to the scene's proportions rather than as a free-floating
    /// blob, and the difference is not cosmetic. A thick block of stone with
    /// air all round it **does not fail at all** -- its own section carries
    /// it, which is documented, intended behaviour (`snap`'s scene note says
    /// so in as many words), so a 44x44 slab hung in mid-air produced zero
    /// bites over 1,200 frames and measured nothing about pacing. What
    /// actually produces a four-figure failing region is a *cantilever*: the
    /// neck is over capacity, and the region is everything it was holding
    /// up.
    fn ligament(w: &mut World) -> Vec<(i32, i32)> {
        let mut fill = |x0: i32, x1: i32, y0: i32, y1: i32| {
            for y in y0..y1 {
                for x in x0..x1 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
        };
        fill(0, 40, 20, 150); // the cliff
        fill(40, 50, 60, 64); // the neck: 4 deep, 10 long
        fill(50, 160, 40, 80); // the overhang: 40 deep, 110 long
        let overhang: Vec<(i32, i32)> = (40..80).flat_map(|y| (50..160).map(move |x| (x, y))).collect();
        assert!(overhang.len() > FRACTURE_CELLS_PER_TICK, "test setup: the overhang must be over the pacing cap, is {}", overhang.len());
        overhang
    }

    /// How much of the massif is still standing inside the overhang's
    /// footprint — stone that is still flagged `attached`.
    ///
    /// **A plain stone census is the wrong metric here, and it took a
    /// measurement to see it.** Debris that comes down lands, and a landed
    /// `ChunkBody` writes its cells back as stone: 218 stone cells sat
    /// inside the footprint at the end of the *unpaced* control run, of
    /// which exactly one was attached — and that one was the neck stub,
    /// outside the footprint proper. "Stone is still there" therefore
    /// cannot tell rock that never fell from rubble that fell and piled up
    /// again, which is the only distinction this test is about. `attached`
    /// can: `break_free`, `promote` and `rigid`'s landing path all
    /// deliberately drop it (see `FLAG_ATTACHED`), so anything that still
    /// has it is rock that has never come away.
    fn intact_rock_left(w: &World, cells: &[(i32, i32)]) -> usize {
        cells.iter().filter(|&&(x, y)| w.get(x, y).material == material::STONE && w.get(x, y).attached()).count()
    }

    /// Run the world the way `an_unsupported_foreground_blob_does_not_hang_in_mid_air`
    /// does — scheduler, bodies and the CA sweep together — recording the
    /// frames on which standing stone actually went away.
    fn collapse_stages(w: &mut World, cells: &[(i32, i32)], frames: usize) -> Vec<(usize, usize)> {
        let mut stages = Vec::new();
        let mut last = intact_rock_left(w, cells);
        for frame in 0..frames {
            w.begin_step();
            scheduler::step(w);
            w.end_step();
            crate::sim::rigid::step_chunk_bodies(w);
            update::step(w);
            let now = intact_rock_left(w, cells);
            if now < last {
                stages.push((frame, last - now));
            }
            last = now;
        }
        stages
    }

    /// **The contract R3a exists to keep: pacing bounds the work, never the
    /// breakage.**
    ///
    /// Two halves, and each one alone is passable by cheating. "It comes
    /// down in stages" is satisfied by a collapse that stalls half way, and
    /// "it all comes down" is satisfied by doing the whole thing in one
    /// tick — which is the 4,420-cell, 142-body frame this change exists to
    /// stop. So both are asserted here, on the same run.
    #[test]
    fn an_oversized_detached_slab_comes_down_over_several_ticks() {
        // Hand-placed geometry no verb touched, so unleashed -- see
        // `World::without_chain_limit`. This is about how a slab arrives,
        // not about what licenses it.
        let mut w = World::new(Rect::new(0, 0, 255, 159)).without_chain_limit();
        let slab = ligament(&mut w);
        compute_world_distances(&mut w);
        w.schedule_structural_check_around(45, 62);

        let stages = collapse_stages(&mut w, &slab, 1_200);

        assert!(
            stages.len() >= 2,
            "the slab came down in {} bite(s) -- pacing did not fire, so a region {} cells big went to `fracture` in one call: {stages:?}",
            stages.len(),
            slab.len()
        );
        // A *frame*, not a tick: the scheduler runs many structural checks
        // per frame, so a frame may legitimately hold one paced slice plus
        // several small failures around it. Measured 1,009 here against
        // 4,420 in a single frame before the change, so the bar is set at
        // twice the cap -- headroom over the measurement, per convention,
        // and nowhere near enough to let the whole overhang through.
        let biggest = stages.iter().map(|&(_, n)| n).max().unwrap_or(0);
        assert!(
            biggest <= FRACTURE_CELLS_PER_TICK * 2,
            "one frame took {biggest} cells off the slab against a per-tick cap of {FRACTURE_CELLS_PER_TICK} -- the cap is not bounding the work: {stages:?}"
        );
        assert_eq!(
            intact_rock_left(&w, &slab),
            0,
            "{} cells of the overhang are still standing after 1,200 frames -- the remainder was stranded, which turns pacing into the immunity `CLAUDE.md` forbids ({stages:?})",
            intact_rock_left(&w, &slab)
        );
    }

    /// The trap the handoff called out by name: the remainder's re-visit
    /// must not rest on `within_disturbance`.
    ///
    /// At the shipped `chain_reach` (`SPREAD`, `i32::MAX`) that gate is a
    /// constant `true`, so the test above cannot see this at all — it would
    /// pass against a version that asks the load model for permission again
    /// and only survives because permission is free. Here the reach is
    /// `TIGHT` and the one disturbance sits at the neck, far outside the box
    /// around the far end of the overhang. The staged queue is not supposed
    /// to care: everything on it was licensed once, as one failure, and
    /// `advance_staged_fractures` never asks a second time.
    ///
    /// # Ignored, and it is a decision waiting to be made rather than a bug
    ///
    /// **This scene cannot pass once the region is clipped, and it is not
    /// the staging that broke.** Measured: 383 of the overhang's 4,400
    /// cells come down (the part inside TIGHT's 33x33 box around the neck)
    /// and **4,017 stay standing** — as a slab with air under it, because
    /// the clip removed the middle of the connection and refused the rest.
    /// The staged queue still drains everything it is given; what it is
    /// given is now the clipped region, so `stages.len() >= 2` still holds
    /// and only `intact_rock_left == 0` fails.
    ///
    /// So the two halves of this commit genuinely disagree here. The clip
    /// is deliberate and its consequence is stated in the wiki ("rock that
    /// was holding nothing up can be left standing" at LOCAL/TIGHT). This
    /// test's `intact_rock_left == 0` is the opposite claim — the wiki's
    /// older *"nothing stops half way and leaves rock hanging in the air,
    /// and no setting anywhere makes the rest of it safe"* — and the two
    /// cannot both be true at a bounded reach.
    ///
    /// Kept rather than edited, per `CLAUDE.md`'s "a revert keeps the
    /// knowledge": the reproduction is exact, and whichever way the
    /// question is settled this is the scene that shows it. What replaces
    /// it as a live guard is
    /// `a_paced_remainder_falls_even_after_its_licence_has_gone` below,
    /// which pins the property this test was *named* for — the queue is
    /// never re-judged — in a form the clip does not make vacuous.
    ///
    /// **The open question, for whoever picks this up:** is a 4,000-cell
    /// slab left hanging in open air at TIGHT better or worse than the
    /// unleashed cascade it replaces? It is not obviously better, and it is
    /// the one outcome the load model has spent four support models
    /// avoiding.
    #[ignore = "clipping the region contradicts this scene's `intact_rock_left == 0`; see the doc above -- a decision, not a bug"]
    #[test]
    fn a_paced_remainder_falls_even_when_the_disturbance_cannot_reach_it() {
        let mut w = World::new(Rect::new(0, 0, 255, 159));
        let slab = ligament(&mut w);
        compute_world_distances(&mut w);
        w.chain_reach = CHAIN_MODES.iter().find(|m| m.name == "TIGHT").expect("a TIGHT chain mode").reach;
        w.record_disturbance(45, 62, 0);
        w.schedule_structural_check_around(45, 62);
        // The reach really cannot see the far end, or this is the previous
        // test wearing a different hat.
        assert!(!w.within_disturbance(159, 79), "test setup: TIGHT must not reach the far corner of the overhang");

        let stages = collapse_stages(&mut w, &slab, 1_200);

        assert!(stages.len() >= 2, "the slab came down in one bite, so this run never exercised the paced path: {stages:?}");
        assert_eq!(
            intact_rock_left(&w, &slab),
            0,
            "{} cells still standing: the paced remainder never came down, so something between the failure and `advance_staged_fractures` is still asking the load model for permission",
            intact_rock_left(&w, &slab)
        );
    }

    /// **The guard the 64x64 bridge could not be.**
    ///
    /// `a_reach_limit_keeps_damage_near_what_was_disturbed` is a 4-cell
    /// bridge in a 64x64 world -- the whole scene is smaller than one
    /// region, so "the region extends far beyond the reach" has no room to
    /// happen there and that test cannot fail for the reported artifact at
    /// all. It stays, because a disturbance that licenses *nothing* is a
    /// real case; this is the one that can see the leak.
    ///
    /// The ligament owns a 4,400-cell region reaching 110 cells past the
    /// neck, and the check that finds it is **licensed** -- the disturbance
    /// is right there. Before the clip, that one licensed check destroyed
    /// the whole span; the reach bounded where the question was asked and
    /// never what the answer ate.
    ///
    /// Asserted on `max_damage_reach` (the P1a instrument) rather than on
    /// one cell's material: the question is the *extent* of what was
    /// destroyed, and a single named cell cannot say it. Paired with a
    /// SPREAD arm, per `CLAUDE.md` -- without it, "nothing landed past 16
    /// cells" also passes against a scene in which nothing broke at all.
    #[test]
    fn a_reach_limit_clips_the_region_a_licensed_failure_eats() {
        fn collapse(reach: i32) -> World {
            let mut w = World::new(Rect::new(0, 0, 255, 159));
            let slab = ligament(&mut w);
            compute_world_distances(&mut w);
            w.chain_reach = reach;
            // Extent `0`: this scene has no tool in it, only a check. The
            // extent is tested separately, and mixing the two here would
            // make it impossible to say which one held the damage in.
            w.record_disturbance(45, 62, 0);
            w.schedule_structural_check_around(45, 62);
            collapse_stages(&mut w, &slab, 1_200);
            w
        }

        let spread = collapse(i32::MAX);
        assert!(
            spread.structural_failures.max_damage_reach > 90,
            "test setup: at SPREAD this failure must eat well past any reach we then set -- it reached {} cells",
            spread.structural_failures.max_damage_reach
        );

        let tight = CHAIN_MODES.iter().find(|m| m.name == "TIGHT").expect("a TIGHT chain mode").reach;
        let leashed = collapse(tight);
        assert!(
            leashed.structural_failures.max_damage_reach <= tight as u32,
            "damage landed {} cells from the only disturbance against a reach of {tight} -- the licence bounds the question, not the region",
            leashed.structural_failures.max_damage_reach
        );
        assert!(
            leashed.structural_failures.overloaded + leashed.structural_failures.unsupported > 0,
            "nothing failed at all at TIGHT, which is a reach limit making rock invincible rather than leashing it"
        );
    }

    /// Smoke beside a piece is somewhere it can go, so the piece is not
    /// confined.
    ///
    /// Paired, and the pairing is the test: the same block of rock with one
    /// cell of its surroundings set to `EMPTY` and to `SMOKE` must give the
    /// same answer, because `rigid::clear_or_displaceable` will shove the
    /// gas aside either way. Before this, only the `EMPTY` arm passed, and
    /// the consequence was not academic -- `explosion::Tuning::smoke_fraction`
    /// fills 18% of a fresh crater with gas, so the hole a blasted chunk is
    /// meant to drop into was the one place in the world that read as solid.
    ///
    /// The third arm is the one that keeps this honest: packed solid with
    /// *rock* it must still be confined, or the predicate has stopped being
    /// able to say no at all.
    #[test]
    fn smoke_beside_a_piece_is_not_the_same_as_rock_beside_it() {
        fn confined_with(fill: Option<MaterialId>) -> bool {
            let mut w = World::new(Rect::new(0, 0, 63, 63));
            for y in 20..40 {
                for x in 20..40 {
                    w.set(x, y, Cell::new(super::super::material::STONE, 0));
                }
            }
            let region: Vec<(i32, i32)> = (28..32).flat_map(|x| (28..32).map(move |y| (x, y))).collect();
            // One cell against the region's side, and nothing else changed.
            match fill {
                Some(m) => w.set(27, 29, Cell::new(m, 0)),
                None => w.set(27, 29, Cell::EMPTY),
            }
            !region_has_free_face(&w, &region)
        }

        assert!(
            confined_with(Some(super::super::material::STONE)),
            "test setup: buried in solid rock the piece must read as confined, or this predicate cannot say no"
        );
        let open = confined_with(None);
        let smoky = confined_with(Some(super::super::material::SMOKE));
        assert!(!open, "test setup: one empty cell beside the piece must give it a free face");
        assert_eq!(smoky, open, "smoke beside the piece read as confined={smoky} where air read as confined={open}");
    }

    /// A blast at TIGHT breaks the rock it actually hit, and does not break
    /// rock well outside it.
    ///
    /// **Both halves, or it passes by making rock invincible.** The wound
    /// half is the reason `Disturbance::extent` exists: a charge of radius
    /// 20 wakes a joint fabric out past 48 cells, so a reach of 16 measured
    /// from the epicentre is inside the charge's own crater, and leashing on
    /// that would crack rock and never break it -- the owner's original
    /// complaint, reintroduced by the fix for a different one.
    ///
    /// Paired against the *same blast* with the extent forced to zero, which
    /// is what the point-disturbance model was. `CLAUDE.md`: compare two
    /// runs, not one run against a remembered number.
    ///
    /// Judged on **cells promoted** -- rock lifted out of the grid as a
    /// moving body -- and not on the failure counters, which count
    /// judgements rather than damage. See the comment at the assertion for
    /// the measurement that forced the change.
    ///
    /// Driven by `Blasts::trigger_with`, never `trigger_tuned`: only the
    /// former records a disturbance at all, so a guard written on the other
    /// has zero disturbances, behaves identically at every mode, and is
    /// vacuous.
    #[test]
    fn a_disturbance_extent_licenses_the_wound_but_not_the_chain() {
        use crate::sim::explosion::Blasts;
        use crate::sim::particle::ParticleSystem;

        let tight = CHAIN_MODES.iter().find(|m| m.name == "TIGHT").expect("a TIGHT chain mode").reach;
        // A deep charge in a solid massif, so the failures that follow are
        // the rock's and not the scene's edges.
        fn massif(reach: i32) -> World {
            let mut w = World::new(Rect::new(0, 0, 255, 159));
            for y in 40..160 {
                for x in 0..256 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            for y in 158..160 {
                for x in 0..256 {
                    w.set(x, y, Cell::new(material::BEDROCK, 0));
                }
            }
            compute_world_distances(&mut w);
            w.chain_reach = reach;
            w
        }
        let fire = |w: &mut World, flatten: bool| {
            let mut particles = ParticleSystem::new();
            let mut blasts = Blasts::new();
            blasts.trigger_with(w, &mut particles, 128, 100, 20, 180.0);
            let extent = w.disturbances.back().expect("a blast records a disturbance").extent;
            if flatten {
                // The point-disturbance model, on the identical blast.
                let d = w.disturbances.pop_back().expect("just read it");
                w.disturbances.push_back(Disturbance { extent: 0, ..d });
            }
            for _ in 0..600 {
                w.begin_step();
                scheduler::step(w);
                w.end_step();
                crate::sim::rigid::step_chunk_bodies(w);
                update::step(w);
                blasts.step(w, &mut particles);
            }
            extent
        };

        let mut wound = massif(tight);
        let extent = fire(&mut wound, false);
        assert!(
            extent > tight,
            "test setup: a radius-20 charge must record a wound wider than TIGHT's own reach, recorded {extent} against {tight}"
        );

        let mut point = massif(tight);
        fire(&mut point, true);

        // **Rock that actually came away, not rock that was flagged.**
        // `overloaded_cells`/`unsupported_cells` sum the *failing region* at
        // every `record` call, and `record` runs before the free-face test,
        // the erosion, the slicing and the fracture -- so the same rock is
        // counted again on every re-evaluation and the totals can be large
        // on a run where nothing moved at all. That made this guard read
        // backwards once `origin/main`'s `grain_is_footing` landed and rubble
        // stopped anchoring: the *leashed* arm, unable to fail decisively,
        // grinds the same rock over and over -- **86 unsupported events
        // against the licensed arm's 30** -- and out-accumulates the arm
        // that collapsed once and was done. Measured at frame 600:
        //
        //     quantity                 wound     point     ordering
        //     region sum (was)          1022      1586     inverted
        //     promoted cells (now)       840       649     wound +29%
        //     stone destroyed            657       648     wound +1.4%
        //
        // `promoted_cells` is the one with a margin worth a bar, and it is
        // the honest question besides: did the blast's own seams come *away*.
        // Stone destroyed orders correctly and is rejected on headroom --
        // `CLAUDE.md` wants a bar set from measurement with room, not one
        // sitting on it. Shortening the run was rejected too: it passes at
        // 100 and 200 frames, but `CHAIN_WINDOW_FRAMES` is 600, so the
        // licence is live for the whole run and stopping early would be
        // tuning to green rather than measuring anything.
        //
        // Fourth time a count in this repo has caught a mode shift rather
        // than a behaviour change -- see §17g's `roomcut` and case 6's
        // `strike`. `CLAUDE.md`: a failure count is not a damage count.
        let with_extent = wound.structural_failures.promoted_cells;
        let without = point.structural_failures.promoted_cells;
        assert!(
            with_extent > without,
            "the extent bought nothing: {with_extent} cells came away with the wound licensed against {without} with a point licence -- TIGHT is leashing the blast's own seams"
        );

        // ...and the chain past the wound is still leashed. Anything the
        // structural model destroyed sits inside `reach + extent`, measured
        // from the edge of the wound by `distance_to_live_disturbance`.
        assert!(
            wound.structural_failures.max_damage_reach <= tight as u32,
            "damage landed {} cells past the wound against a reach of {tight}",
            wound.structural_failures.max_damage_reach
        );
    }

    /// **NONE is the hard off switch and stays one**, extent or no extent.
    ///
    /// The exemption is deliberate and is pinned here rather than left to
    /// read off `licence_radius`: NONE is the owner's working escape hatch
    /// and the one setting that currently does what it says
    /// (`wiki/structural-collapse.md`: "only what you struck is ever
    /// destroyed. Nothing collapses afterwards, at all"). A commit that
    /// gave every disturbance a volume could very easily have handed NONE
    /// one too.
    #[test]
    fn none_still_means_none() {
        let mut w = World::new(Rect::new(0, 0, 255, 159));
        let slab = ligament(&mut w);
        compute_world_distances(&mut w);
        w.chain_reach = CHAIN_MODES.iter().find(|m| m.name == "NONE").expect("a NONE chain mode").reach;
        // A wound big enough to cover the entire overhang, which at any
        // other setting would license all of it. Recorded in open air above
        // the neck, so that "zero failures" is the literal claim: at reach 0
        // the licence is the struck cell and nothing else, and the struck
        // cell here is sky.
        w.record_disturbance(45, 30, 200);
        w.schedule_structural_check_around(45, 62);
        assert!(!w.within_disturbance(159, 79), "NONE licensed the far corner of the overhang through a 200-cell extent");
        assert!(!w.within_disturbance(45, 62), "NONE licensed the neck 32 cells away through a 200-cell extent");

        let stages = collapse_stages(&mut w, &slab, 1_200);

        assert_eq!(
            w.structural_failures.overloaded + w.structural_failures.unsupported,
            0,
            "{} failure(s) fired at NONE with a 200-cell extent recorded",
            w.structural_failures.overloaded + w.structural_failures.unsupported
        );
        assert_eq!(intact_rock_left(&w, &slab), slab.len(), "rock came away at NONE: {stages:?}");

        // ...and the identical scene at TIGHT does come apart, so the
        // exemption is what held it and not an inert scene. `CLAUDE.md`:
        // compare two runs.
        let mut loud = World::new(Rect::new(0, 0, 255, 159));
        let slab = ligament(&mut loud);
        compute_world_distances(&mut loud);
        loud.chain_reach = CHAIN_MODES.iter().find(|m| m.name == "TIGHT").expect("a TIGHT chain mode").reach;
        loud.record_disturbance(45, 30, 200);
        loud.schedule_structural_check_around(45, 62);
        collapse_stages(&mut loud, &slab, 1_200);
        assert!(
            loud.structural_failures.overloaded + loud.structural_failures.unsupported > 0,
            "test setup: the same scene and the same 200-cell extent produced no failure at TIGHT either, so NONE was never what stopped it"
        );
    }

    /// A refused failure still carries its distance wavefront.
    ///
    /// Control only reaches the clip when `worsened` is true, so `propagate`
    /// already holds this cell's reschedule and its neighbour fan-out, and
    /// the new `aux` has been written. The veto this replaced returned
    /// `Vec::new()` and discarded both -- a fourth early return that did not
    /// carry the wavefront the block at the top of `tick` says every early
    /// return must, and it dropped it specifically for *rising* distances,
    /// which is the direction that propagates collapse. The consequence of
    /// dropping it, measured once already: frame 3,000 with 15,840 cells
    /// never load-evaluated.
    ///
    /// `tick` is called directly rather than through the scheduler, because
    /// the claim is about its **return value** -- the sites it hands back
    /// are the wavefront, and nothing downstream can distinguish "returned
    /// nothing" from "had nothing to return".
    #[test]
    fn a_refused_failure_still_carries_its_distance_wavefront() {
        let mut w = World::new(Rect::new(0, 0, 255, 159));
        ligament(&mut w);
        compute_world_distances(&mut w);
        w.chain_reach = CHAIN_MODES.iter().find(|m| m.name == "TIGHT").expect("a TIGHT chain mode").reach;
        // Far from the overhang: every failure out here is refused.
        w.record_disturbance(250, 20, 0);

        // The neck's own bottom face: free below, so structurally
        // interesting, and it is the cell this scene exists to make fail
        // (`ROOTWARD_CHECK_STEPS`' own doc: "the neck now fails on its own
        // check").
        let (x, y) = (45, 63);
        assert!(!w.within_disturbance(x, y), "test setup: the cell under test must be outside the licence");
        let cell = w.get(x, y);
        // Knocked back to zero so this tick sees the distance **rise**.
        // That is the path this guard is about -- the one where `propagate`
        // is non-empty and dropping it loses a fan-out. The settled path
        // reaches the same return with `propagate` already empty, where
        // there is nothing to lose and nothing to guard.
        w.set(x, y, cell.with_aux(0));

        // ...and it really is a failing cell, or the refusal never happens
        // and the test passes against anything.
        let mut cache = std::mem::take(&mut w.load_cache);
        let mut budget = w.load_budget;
        let verdict = super::super::load::failing_along_support_chain(&w, x, y, &mut cache, &mut budget);
        assert!(
            matches!(verdict, super::super::load::ChainVerdict::Failing(_)),
            "test setup: the cell under test must be judged failing, or nothing is ever refused here"
        );
        drop(verdict);
        w.load_cache = Default::default();
        w.load_budget = crate::sim::load::MAX_LOAD_CELLS_PER_FRAME;

        let site = ActiveSite { x, y, kind: ActiveKind::StructuralCheck, next_frame: w.frame };
        let out = tick(&mut w, &site);

        assert_eq!(
            w.structural_failures.overloaded + w.structural_failures.unsupported,
            0,
            "test setup: the failure was not refused, so this measured the success path"
        );
        assert!(
            !out.is_empty(),
            "a refused failure returned no sites at all -- the distance wavefront was computed and then thrown away, which is how a structure stops being load-evaluated entirely"
        );
    }

    /// **The queue is work, not a question** — restated in a form the
    /// region clip does not make vacuous.
    ///
    /// The test above put the disturbance where its licence could not
    /// reach the far end. That no longer discriminates: `tick` clips the
    /// region to the licence *before* staging it, so everything on the
    /// queue is inside the licence by construction and "the remainder falls
    /// even though the licence cannot see it" is true for free.
    ///
    /// What still has to hold, and is the thing the original test was named
    /// for, is that `advance_staged_fractures` never asks a second time. So
    /// the licence is made wide enough to stage the whole overhang, and then
    /// **taken away mid-collapse** — a disturbance ageing past
    /// `chain_window` does exactly this in play, and a player pressing `F9`
    /// does it deliberately (that path goes through
    /// `relicense_staged_fractures`, which is the *one* sanctioned
    /// exception). Everything already queued must still come down.
    #[test]
    fn a_paced_remainder_falls_even_after_its_licence_has_gone() {
        let mut w = World::new(Rect::new(0, 0, 255, 159));
        let slab = ligament(&mut w);
        compute_world_distances(&mut w);
        w.chain_reach = CHAIN_MODES.iter().find(|m| m.name == "TIGHT").expect("a TIGHT chain mode").reach;
        // A wound wide enough that the clip keeps the whole overhang, so
        // the region really is over `FRACTURE_CELLS_PER_TICK` and really
        // does stage.
        w.record_disturbance(45, 62, 120);
        w.schedule_structural_check_around(45, 62);
        assert!(w.within_disturbance(159, 79), "test setup: the licence must cover the whole overhang, or nothing large is ever staged");

        let mut staged = false;
        for _ in 0..1_200 {
            w.begin_step();
            scheduler::step(&mut w);
            w.end_step();
            crate::sim::rigid::step_chunk_bodies(&mut w);
            update::step(&mut w);
            if !w.staged_fractures.is_empty() {
                staged = true;
                break;
            }
        }
        assert!(staged, "test setup: nothing was ever staged, so this run exercises no queue at all");
        let queued: usize = w.staged_fractures.iter().map(|f| f.region.len()).sum();
        assert!(queued > 0, "test setup: the queue is empty");

        // The licence evaporates. Everything already judged must still land.
        w.disturbances.clear();
        assert!(!w.within_disturbance(45, 62), "test setup: the licence must really be gone");

        let stages = collapse_stages(&mut w, &slab, 1_200);

        assert!(stages.len() >= 2, "the slab came down in one bite, so this run never exercised the paced path: {stages:?}");
        assert_eq!(
            intact_rock_left(&w, &slab),
            0,
            "{} cells still standing of the {queued} that were queued: the staged remainder is being re-judged against a licence it never had to satisfy",
            intact_rock_left(&w, &slab)
        );
    }

    /// The other half of `F9`: a **tightened** setting must reach the work
    /// already in flight.
    ///
    /// The test above pins that a staged remainder is *not* re-judged, which
    /// is deliberate and stays. `relicense_staged_fractures` is the single
    /// exception, and without it switching to NONE mid-aftermath reads as
    /// doing nothing at all -- the collapse keeps arriving at
    /// `FRACTURE_CELLS_PER_TICK` a tick from a queue the new setting never
    /// consults, which is precisely the reported complaint.
    ///
    /// Run at SPREAD with **no disturbance recorded**, deliberately: SPREAD
    /// licenses everything without one, so the ring is empty when the
    /// setting tightens and the new licence covers nothing anywhere. That is
    /// what NONE advertises ("only what you struck is ever destroyed"), and
    /// it also makes the second half of the assertion mean what it says --
    /// nothing else in the world can newly fail either, so anything that
    /// comes down after the switch came off the queue.
    /// Relicensing **keeps the licensed half** of a staged region, and that
    /// is the half no existing guard could see.
    ///
    /// Its sibling below tightens a queue with an *empty* disturbance ring,
    /// so every cell is unlicensed and the entry is dropped whole. So does
    /// the `App` guard. Both therefore pass unchanged against
    /// `fn relicense_staged_fractures(&mut self) { self.staged_fractures.clear(); }`
    /// -- they cannot tell "drop what is unlicensed" from "drop everything",
    /// which is the only property the function is named for.
    ///
    /// This one puts a real disturbance in the ring and stages a region that
    /// straddles its edge, so a `clear()` fails it and a `retain` passes.
    /// Built by hand rather than by running a collapse, deliberately: the
    /// point is the predicate, and driving it through a scene would make the
    /// test depend on which cells that scene happened to stage.
    #[test]
    fn relicensing_keeps_the_part_of_a_staged_region_that_is_still_licensed() {
        // Unleashed explicitly. This used to read `assert!(w.chain_reach
        // == i32::MAX, "test setup: a new world starts at SPREAD")`, which
        // was true when it was written and stopped being so when `TIGHT`
        // became the default -- the test needs to *start* wide so the
        // tighten below has something to narrow.
        let mut w = World::new(Rect::new(0, 0, 255, 159)).without_chain_limit();
        // Extent 10, so the licence at LOCAL reaches 48 + 10 = 58 cells.
        w.record_disturbance(60, 60, 10);
        let licensed = (60, 60);
        let outside = (60 + 58 + 20, 60);
        assert!(w.chain_reach == i32::MAX, "test setup: this case starts unleashed");

        w.staged_fractures.push_back(StagedFracture { region: vec![licensed, outside], at: licensed, next_frame: w.frame });

        let local = CHAIN_MODES.iter().find(|m| m.name == "LOCAL").expect("a LOCAL chain mode").reach;
        w.chain_reach = local;
        assert!(w.within_disturbance(licensed.0, licensed.1), "test setup: the near cell must still be licensed at LOCAL");
        assert!(!w.within_disturbance(outside.0, outside.1), "test setup: the far cell must not be");

        w.relicense_staged_fractures();

        assert_eq!(
            w.staged_fractures.len(),
            1,
            "the whole entry was dropped although half of it was still licensed -- this is `clear()` wearing a `retain`'s name"
        );
        assert_eq!(
            w.staged_fractures[0].region,
            vec![licensed],
            "relicensing kept {:?}, wanted only the licensed cell",
            w.staged_fractures[0].region
        );
    }

    #[test]
    fn tightening_the_chain_mode_drops_staged_work_it_no_longer_licenses() {
        // Starts unleashed on purpose: the queue has to fill before there
        // is anything to tighten *onto*, and at the shipped `TIGHT` with
        // nothing disturbed this hand-placed slab never stages a thing --
        // which would make the case pass against any implementation, the
        // exact failure its own comment below warns about.
        let mut w = World::new(Rect::new(0, 0, 255, 159)).without_chain_limit();
        let slab = ligament(&mut w);
        compute_world_distances(&mut w);
        w.schedule_structural_check_around(45, 62);

        // Run only until something is actually queued -- "did it fire at
        // all" is a counter question, and a test that tightened an empty
        // queue would pass against any implementation whatsoever.
        let mut staged = false;
        for _ in 0..1_200 {
            w.begin_step();
            scheduler::step(&mut w);
            w.end_step();
            crate::sim::rigid::step_chunk_bodies(&mut w);
            update::step(&mut w);
            if !w.staged_fractures.is_empty() {
                staged = true;
                break;
            }
        }
        assert!(staged, "test setup: nothing was ever staged, so this run exercises no queue at all");
        let standing = intact_rock_left(&w, &slab);
        assert!(standing > 0, "test setup: the overhang was already gone before the setting changed");

        // The player presses `F9` through to NONE, mid-aftermath.
        w.chain_reach = CHAIN_MODES.iter().find(|m| m.name == "NONE").expect("a NONE chain mode").reach;
        w.relicense_staged_fractures();
        assert!(
            w.staged_fractures.is_empty(),
            "{} staged fracture(s) survived a switch to NONE -- at reach 0 with nothing disturbed the queue holds nothing the setting licenses",
            w.staged_fractures.len()
        );

        let stages = collapse_stages(&mut w, &slab, 400);
        assert_eq!(
            intact_rock_left(&w, &slab),
            standing,
            "{} more cells came down after the player asked for NONE: {stages:?}",
            standing - intact_rock_left(&w, &slab)
        );
    }

    /// The slice is grown from the break, and it is one connected piece —
    /// not a scatter of whatever a `HashSet` iterated first. A slice that
    /// is not contiguous with the failure reads as rock evaporating at
    /// random rather than as a collapse eating outward from where it gave
    /// way.
    #[test]
    fn a_paced_slice_is_one_piece_grown_from_the_break() {
        let region: Vec<(i32, i32)> = (0..60).flat_map(|y| (0..60).map(move |x| (x, y))).collect();
        let Sliced { slice, remainder } = slice_failing_region(region.clone(), (0, 0));

        assert_eq!(slice.len(), FRACTURE_CELLS_PER_TICK, "the slice should be exactly one tick's worth");
        assert_eq!(slice.len() + remainder.len(), region.len(), "material went missing between the slice and the remainder");
        let taken: std::collections::HashSet<(i32, i32)> = slice.iter().copied().collect();
        // Connected, checked by flooding it back out from the seed.
        let mut seen = std::collections::HashSet::from([(0, 0)]);
        let mut stack = vec![(0, 0)];
        while let Some((x, y)) = stack.pop() {
            for (dx, dy) in NEIGHBOURS_4 {
                let next = (x + dx, y + dy);
                if taken.contains(&next) && seen.insert(next) {
                    stack.push(next);
                }
            }
        }
        assert_eq!(seen.len(), slice.len(), "the slice is in {} disconnected parts", slice.len() - seen.len() + 1);
        // And the far corner is nowhere near it.
        assert!(!taken.contains(&(59, 59)), "the slice reached the far corner of a 3,600-cell region -- it is not growing from the break");
        // Nothing is in both halves, which is the other way material can go
        // missing here -- a cell fractured now and queued for later would be
        // promoted twice.
        assert!(!remainder.iter().any(|c| taken.contains(c)), "a cell is in both the slice and the remainder");
    }

    /// The jittered frontier, stated as the property it exists for: a plain
    /// breadth-first slice of homogeneous rock is an L1 diamond, which is
    /// the "perfect column or sharp triangle" the owner rejected off the
    /// prototype sheets. Measured on the cut's own profile — how far out
    /// along each row the slice reaches — a diamond gives a step of exactly
    /// one cell per row and nothing else.
    #[test]
    fn a_paced_cut_is_not_a_drawn_diamond() {
        let region: Vec<(i32, i32)> = (0..60).flat_map(|y| (0..60).map(move |x| (x, y))).collect();
        let Sliced { slice, .. } = slice_failing_region(region, (30, 30));
        let mut reach: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
        for &(x, y) in &slice {
            let e = reach.entry(y).or_insert(i32::MIN);
            *e = (*e).max(x);
        }
        let mut rows: Vec<i32> = reach.keys().copied().collect();
        rows.sort_unstable();
        let steps: Vec<i32> = rows.windows(2).filter(|p| p[1] == p[0] + 1).map(|p| reach[&p[1]] - reach[&p[0]]).collect();
        assert!(steps.len() > 20, "test setup: the slice should span plenty of rows, spans {}", steps.len());
        assert!(
            steps.iter().any(|&s| s.abs() > 1),
            "every row of the cut steps by exactly one cell -- that is a drawn diamond, which is what the frontier jitter exists to break"
        );
    }

    /// **The strength half of the wood-density allele, on the scene the
    /// load model already trusts.**
    ///
    /// Same beam, same sand, same pinned reach as
    /// `a_loaded_branch_breaks_at_a_shorter_span_than_a_bare_one` — the
    /// only difference between the two runs is the individual's density
    /// allele, so nothing but the multiplier can explain the outcome. The
    /// pinned reach of 8 is chosen so the tip's distance (8) falls
    /// *between* the two alleles' effective spans: the pioneer's
    /// `⌊8 × 0.75⌋ − 1 = 5` cannot hold it and the dense allele's
    /// `⌊8 × 1.35⌋ − 1 = 9` can. Against the shipped reach of 96 both
    /// would stand and this would test nothing, which is exactly what
    /// `pin_wood_reach` exists for.
    ///
    /// The price half lives on `Grow.cost` and is measured on a stand, not
    /// here — one number for both, so tuning cannot turn the trade into a
    /// free lunch.
    #[test]
    fn dense_wood_holds_a_longer_loaded_branch() {
        let build = |allele: u8| -> World {
            let mut w = test_world();
            let wood = w.materials.id_of("wood").expect("wood is compiled in");
            let organism = w.push_organism(w.species.id_of("tree").expect("tree is compiled in")).expect("an organism slot is free");
            if let Some(state) = w.organism_mut(organism) {
                state.alleles[organism::LOCUS_WOOD_DENSITY] = allele;
            }
            pin_wood_reach(&mut w, 8);
            for y in 30..40 {
                w.set(10, y, Cell::new(material::STONE, 0));
            }
            for x in 11..=19 {
                w.set(x, 30, Cell::new(wood, 0).with_organism_id(organism).with_aux(organism::pack_cell_type(organism::CellType::MatureBody)));
            }
            // Sand on the branch, the same pile the load test uses. The
            // load term is what the density multiplier has to compose
            // with -- §4.1's claim is about a branch under piled load, not
            // a bare one -- and it is what puts the as-authored allele on
            // the failing side too (see the third run below).
            for x in 11..=19 {
                w.set(x, 29, Cell::new(material::SAND, 0));
            }
            w.schedule_structural_check_around(19, 30);
            w
        };

        let surviving = |w: &World| -> usize {
            let wood = w.materials.id_of("wood").unwrap();
            (11..=19).filter(|&x| w.get(x, 30).material == wood).count()
        };

        let mut pioneer = build(0);
        let mut authored = build(1);
        let mut dense = build(2);
        run_organisms(&mut pioneer, 200);
        run_organisms(&mut authored, 200);
        run_organisms(&mut dense, 200);

        assert_eq!(surviving(&dense), 9, "the dense allele's effective span (9) covers the whole beam and it should stand intact; kept {}", surviving(&dense));
        assert!(
            surviving(&pioneer) < surviving(&dense),
            "cheap wood must break back further under the same load: pioneer kept {} cells, dense kept {}",
            surviving(&pioneer),
            surviving(&dense)
        );
        // **The middle allele is the control that says the multiplier did
        // the work.** As authored (x1.0) the effective span is 7 against a
        // tip at 8, so this beam fails too -- the dense run above is not
        // standing because the scene is easy, it is standing because 1.35
        // bought it two more cells of reach.
        assert!(
            surviving(&authored) < surviving(&dense),
            "the as-authored allele should break here as well, or the dense run proves nothing: authored kept {} cells, dense kept {}",
            surviving(&authored),
            surviving(&dense)
        );
    }
}
