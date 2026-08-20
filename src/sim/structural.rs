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
    let new_distance: u16 = if relaxed == u16::MAX && is_resting_on_ground(world, x, y) { 0 } else { relaxed };

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
        defer!();
    }
    // Damage stays near what was actually disturbed, if a reach is set.
    //
    // Checked here rather than earlier so the cell is still *evaluated*:
    // the load model keeps working, the stress view keeps reading true,
    // and only the consequence is withheld. Gating the evaluation instead
    // would make far-away rock unfailable in principle, which is the
    // binary immunity four earlier support models were rejected for.
    //
    // `Vec::new()`, so a refused cell stops being rescheduled rather than
    // spinning: it will be looked at again when something disturbs it, and
    // that something is exactly what would license it to fail.
    if let super::load::ChainVerdict::Failing(_) = verdict {
        if !world.within_disturbance(x, y) {
            return Vec::new();
        }
    }
    if let super::load::ChainVerdict::Failing(failure) = verdict {
        // The forest this describes is about to change out from under it.
        world.load_cache.clear();
        world.structural_failures.record(failure.mode, failure.region.len());
        let reach = (failure.at.0 - x).unsigned_abs() + (failure.at.1 - y).unsigned_abs();
        world.structural_failures.record_reach(reach);
        let region = failure.region;
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
            // **Unsupported** is a pocket already cut free on every side,
            // wedged in a hole its own shape. More cracks would add
            // nothing -- there is nothing left to separate -- and it
            // cannot fall, because there is nowhere to fall to. So it is
            // recorded and left alone, and the wavefront is dropped
            // because nothing changed for anything to learn about.
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
            if failure.mode == super::load::FailureMode::Overloaded {
                // A crush that writes **no new fissure** has nothing to
                // propagate. Cracks are bits, so re-crushing rock that is
                // already crushed is idempotent -- it does no damage, and
                // it also does no *good*, while still costing a load walk
                // and a reschedule every time. Measured by the
                // load-concentration session, whose change makes many more
                // cells reach the criterion: `caveshallow` went from 488
                // confined failures to 2,988 and produced *fewer* fissured
                // cells (332 to 284), which is 6x the work for less
                // output. Stopping when nothing was written turns that
                // from a treadmill into a one-off.
                if crush_in_place(world, &region, failure.at) > 0 {
                    return propagate;
                }
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
        if !super::rigid::fracture_failing_region(world, &region, failure.at) {
            for &(fx, fy) in &region {
                break_free(world, fx, fy);
            }
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
/// design.md` §2/§5). No per-cell cached distance to relax incrementally
/// — `organism_is_supported` below runs a fresh bounded BFS every call
/// instead, from `(x, y)` outward rather than from a stored anchor list
/// (`OrganismState` still doesn't track one — real future work, not faked
/// here with a placeholder that wouldn't mean anything yet). Cheap enough
/// for now: `max_span` bounds the search radius, and organisms stay small
/// (this session's own live-verification tops out in the tens of cells).
/// A no-op for a material with no finite span (moss's own, still — this
/// only actually does anything for tree.ron's wood so far).
fn organism_structural_tick(world: &mut World, x: i32, y: i32, cell: Cell) -> Vec<ActiveSite> {
    let material = world.materials.get(cell.material);
    let max_span = material.max_unsupported_span;
    if max_span == u16::MAX {
        return Vec::new(); // this species' material doesn't participate at all (e.g. moss)
    }
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
    if organism_is_supported(world, x, y, organism_id, effective_span) {
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
    break_free(world, x, y);
    schedule_organism_neighbours(world, x, y, organism_id)
}

/// Whether `(x, y)` — a `Plant` cell belonging to `organism_id` — is
/// within `max_span` connected same-organism `Plant` hops of ground: a
/// cell (this one, or one reached through the search) touching a `Solid`-
/// kind neighbour. The direct generalization of the aux-cached path's own
/// `is_anchor` (touches `BEDROCK`) to a tree: a trunk resting on stone is
/// exactly as anchored as a stone span touching bedrock is, and a root
/// pressed against solid ground counts the same way. Growing *through*
/// soil (not yet possible — `Grow`'s candidates are gated on `is_empty`,
/// PLAN.md's own recorded gap) will extend this the same way once it
/// exists; nothing here needs to change for that, since "touches Solid"
/// stays true for a root embedded in solid-kind ground either way.
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

fn organism_is_supported(world: &World, x: i32, y: i32, organism_id: u16, max_span: u16) -> bool {
    let touches_solid_ground = |px: i32, py: i32| {
        NEIGHBOURS_4.iter().any(|&(dx, dy)| world.materials.kind(world.get(px + dx, py + dy).material) == MaterialKind::Solid)
    };
    if touches_solid_ground(x, y) {
        return true; // distance 0 -- this cell is itself the trunk base, or a root against bare ground
    }

    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    visited.insert((x, y));
    queue.push_back(((x, y), 0u16));
    while let Some(((cx, cy), dist)) = queue.pop_front() {
        if dist >= max_span {
            continue; // do not expand past the span -- nothing beyond could still be in bounds
        }
        for (dx, dy) in NEIGHBOURS_4 {
            let next @ (nx, ny) = (cx + dx, cy + dy);
            if visited.contains(&next) {
                continue;
            }
            let neighbour = world.get(nx, ny);
            if neighbour.organism_id() != organism_id || world.materials.kind(neighbour.material) != MaterialKind::Plant {
                continue; // a different organism, or non-Plant material, is a wall -- same convention diffuse_resource's own is_wall uses
            }
            visited.insert(next);
            if touches_solid_ground(nx, ny) {
                return true;
            }
            queue.push_back((next, dist + 1));
        }
    }
    false
}

/// The organism-owned mirror of `schedule_solid_neighbours` — an organism
/// cell that just broke free might have been the only thing keeping its
/// own same-organism `Plant` neighbours anchored, so they need
/// re-evaluating too. This is what turns one broken branch into a real
/// cascade for a tree, the same way the aux-cached path already does for
/// stone.
fn schedule_organism_neighbours(world: &World, x: i32, y: i32, organism_id: u16) -> Vec<ActiveSite> {
    NEIGHBOURS_4
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
fn region_has_free_face(world: &World, region: &[(i32, i32)]) -> bool {
    let cells: std::collections::HashSet<(i32, i32)> = region.iter().copied().collect();
    region.iter().any(|&(x, y)| {
        NEIGHBOURS_8.iter().any(|&(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            !cells.contains(&(nx, ny))
                && world.in_bounds(nx, ny)
                && world.get(nx, ny).material == super::material::EMPTY
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
fn crush_in_place(world: &mut World, region: &[(i32, i32)], at: (i32, i32)) -> u32 {
    // Both the length and the number of forks scale with the piece,
    // because a fist-sized failure and a 900-cell one should not come
    // apart to the same amount of damage -- the all-or-nothing outcome is
    // this project's most expensive recurring artifact and this is where
    // it would reappear.
    let extent = region
        .iter()
        .map(|&(x, y)| (x - at.0).abs().max((y - at.1).abs()))
        .max()
        .unwrap_or(0) as usize;
    let length = extent.clamp(CRACK_MIN_LENGTH, CRACK_MAX_LENGTH);
    let seed = super::rng::jitter(at.0, at.1) * std::f32::consts::TAU;
    let forks = CRACK_FORKS_BASE + region.len() / CRACK_CELLS_PER_FORK;
    // `detach: false` -- a confined crush must not unbrace the rock or
    // reschedule it. Both are right for a *blow* and wrong here:
    // unbracing confined rock tells the load model the inside of a
    // mountain has come free, and rescheduling is what turns a crush into
    // a treadmill. See `walk_fissures`' own doc for the full split.
    let fissured = walk_fissures(world, at, seed, CRACK_PRIMARIES, length, forks, false);
    // Counted here rather than inside the walker, so `FailureCounts`
    // keeps meaning exactly what it did: cells a *confined failure*
    // cracked where they stood. A blast's fissures are the blast report's
    // business (`BlastReport::cells_fissured`) and must not land in the
    // structural-failure census.
    world.structural_failures.crushed_cells += fissured;
    fissured
}

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
/// `crush_in_place` passes `detach: false`; the blast in `explosion.rs`
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
/// count stays a parameter and `crush_in_place` stays bit-identical to the
/// version this was extracted from.
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
    /// `run_to_completion`'s cursor. Unused by `advance`, which sweeps the
    /// whole queue every call.
    head: usize,
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
        Self { walkers: Vec::new(), head: 0, forks: 0, detach, scored_now: std::collections::HashSet::new(), style, glow }
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

    /// Draw the whole star now, in one call. Depth-first: walker `head` is
    /// drained completely, forks included by the time the queue reaches
    /// them, before the next walker starts.
    pub(crate) fn run_to_completion(&mut self, world: &mut World) -> u32 {
        let mut fissured = 0;
        while self.head < self.walkers.len() {
            fissured += self.step_walker(world, self.head, usize::MAX);
            self.head += 1;
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
            let cell = world.get(next.0, next.1);
            // A fissure runs through **rock**, not through the bookkeeping
            // region that was judged, and it stops where the rock does --
            // there is nothing to split at a free face.
            //
            // Confining it to `region` was tried first and was very nearly
            // a no-op, which only a counter could show: 189 confined
            // failures wrote **29 cells of fissure between them**, and
            // produced bit-identical images across two complete rewrites
            // of the crack pattern. An overload's region is the union of
            // the *subtrees* hanging off the failing section -- a sparse,
            // often thin set that the walker leaves on its first step --
            // so almost every crack died where it was born. It is also the
            // wrong picture: a crack spreading out of a boulder into the
            // massif around it is the thing being modelled, and the massif
            // is by definition not in the region that failed.
            if !is_body_material(world, cell.material) {
                done = true;
                break;
            }
            if self.detach && cell.cracked() && !self.scored_now.contains(&next) {
                budget = (budget + super::rigid::CRACK_TIP_BONUS as usize).min(ceiling);
            }
            // Which edge a fissure cuts is the one *perpendicular* to the
            // way it travels -- get this backwards and the crack runs
            // across itself, slicing the rock into rings rather than
            // parting it. `rigid::score_cracks` records the same trap.
            let mut scored = if step.0.abs() >= step.1.abs() { cell.with_crack_down(true) } else { cell.with_crack_right(true) };
            // Counted off the crack bit alone, and *before* any glow is
            // written: `cells_fissured` means "rock that was not cracked
            // here before", and a cell whose bit was already set but which
            // this walk reheated is not new damage.
            if scored != cell {
                fissured += 1;
            }
            // The crack tip is incandescent while it races (`glow`'s own
            // doc). Never cools a cell that is already hotter -- the same
            // one-way rule `scorch` uses, for the same reason: two
            // overlapping blasts, or a crack crossing something on fire.
            if self.glow > 0 && scored.temperature() < self.glow {
                scored.set_temperature(self.glow);
            }
            if scored != cell {
                world.set(next.0, next.1, scored);
            }
            if self.detach {
                self.scored_now.insert(next);
                // A fissure is where the rock has parted company with the
                // mass behind the slice, so it stops claiming to be braced
                // by it. Done for every cell the walk crosses, not only
                // the ones whose bit changed: a repeat charge has to
                // re-loosen a rim that is already scored, or the second
                // shot throws nothing.
                detach_around_crack(world, next.0, next.1);
                world.schedule_structural_check_around(next.0, next.1);
            }
            pos = next;
            // The brittle style's whole shape, in three lines: the heading
            // is untouched for a run of cells, and then turns sharply. The
            // run is counted in **cells entered**, not in walker steps --
            // a step that stays inside its own cell has not advanced the
            // fracture, and counting those would make a segment's drawn
            // length depend on how obliquely the heading crosses the grid.
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

/// How many primary fissures leave the point that gave way.
///
/// Three, so the damage reads as a star spreading out of one place --
/// which is what a crack from an impact looks like -- rather than as one
/// stroke through the rock or as a wheel of spokes.
const CRACK_PRIMARIES: usize = 3;

/// How far a fissure runs, bounded by the piece it is running through.
///
/// Scaled to the failing region's own extent between these two, because
/// the damage has to be a *distribution*: a crack that runs 40 cells
/// through a 900-cell slab and through a 30-cell one is the all-or-nothing
/// outcome in miniature. The floor keeps a small failure from producing
/// damage too faint to see; the ceiling keeps a large one from being cut
/// clean in half by a single stroke, which is displacement's job and not
/// this.
const CRACK_MIN_LENGTH: usize = 10;
const CRACK_MAX_LENGTH: usize = 55;

/// How sharply a fissure may turn at each cell, in radians.
///
/// A crack that does not wander is a ray, and a fan of rays reads as a
/// wheel rather than as broken rock. Enough to curve visibly over its
/// length, not so much that it doubles back on itself.
const CRACK_WANDER: f32 = 0.9;

/// How many side-branches a crush may throw, and on what terms.
///
/// Forking is the difference between a crack and a hatch: a branch that
/// leaves at a wide angle and dies sooner than its parent reads as rock
/// splitting, where independent strokes crossing at a point read as the
/// scribble the owner rejected. Scaled with the piece for the same reason
/// the length is.
const CRACK_FORKS_BASE: usize = 4;
const CRACK_CELLS_PER_FORK: usize = 80;
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
    if !super::rigid::fracture_failing_region(world, &slice, staged.at) {
        for &(fx, fy) in &slice {
            break_free(world, fx, fy);
        }
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

fn break_free(world: &mut World, x: i32, y: i32) {
    // Resolved per cell rather than passed in by the caller, because a
    // failing region is no longer one cell and need not be one material --
    // a shelf can be part stone and part whatever was built onto it. A cell
    // whose material has no configured debris is left alone rather than
    // deleted: "not actually participating" beats silently destroying
    // content an author forgot to pair `breaks_into` with.
    let Some(into) = world.materials.get(world.get(x, y).material).breaks_into else {
        return;
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
/// At the shipped 2048x640, measured with a probe splitting the two halves:
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
    // screen. Measured at 2048x640, with the probe splitting the two
    // halves:
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

    let mut cells = vec![Cell::EMPTY; w * h];
    for chunk in world.chunks() {
        let (ox, oy) = chunk.coord.origin();
        for ly in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let (x, y) = (ox + lx, oy + ly);
                // A chunk may hang over the world edge; the mirror is
                // exactly the bounds, so the overhang is dropped rather
                // than wrapped into the opposite side.
                if inside(x, y) {
                    cells[idx(x, y)] = chunk.get_world(x, y);
                }
            }
        }
    }

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
    let relaxable = |c: Cell| body.get(c.material.0 as usize).copied().unwrap_or(false) && c.organism_id() == 0;

    // Seed: anchors at 0, everything else at "unreachable" so a cell the
    // search never reaches ends up honestly unsupported rather than
    // accidentally reading as anchored.
    let mut dist = vec![u16::MAX; w * h];
    let mut heap: BinaryHeap<Reverse<(u16, i32, i32)>> = BinaryHeap::new();
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            if !relaxable(cells[idx(x, y)]) {
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
                !inside(nx, ny) || cells[idx(nx, ny)].material == material::BEDROCK
            });
            if anchored {
                dist[idx(x, y)] = 0;
                heap.push(Reverse((0, x, y)));
            }
        }
    }

    // Relax. `Reverse` makes this a min-heap ordered on (distance, x, y) --
    // the position tiebreak is what keeps the result identical run to run,
    // the same reason `ActiveSite`'s own `Ord` spells its tiebreak out
    // (issue #7 / determinism §8b).
    while let Some(Reverse((distance, x, y))) = heap.pop() {
        if dist[idx(x, y)] != distance {
            continue; // superseded by a shorter path already processed
        }
        for (dx, dy) in NEIGHBOURS_4 {
            let (nx, ny) = (x + dx, y + dy);
            if !inside(nx, ny) {
                continue;
            }
            // `edge_is_cracked` against the mirror. Each edge is owned by
            // exactly one of the two cells it separates, so reaching left or
            // up means asking the *neighbour* about its own right or down
            // edge -- see that function, which stays as the world-reading
            // version everything outside this pass uses.
            let cracked = match (dx, dy) {
                (1, 0) => cells[idx(x, y)].crack_right(),
                (-1, 0) => cells[idx(nx, y)].crack_right(),
                (0, 1) => cells[idx(x, y)].crack_down(),
                (0, -1) => cells[idx(x, ny)].crack_down(),
                _ => false,
            };
            if cracked {
                continue;
            }
            let neighbour = cells[idx(nx, ny)];
            if !relaxable(neighbour) {
                continue;
            }
            // The cost is paid by the cell being *supported* -- so it reads
            // the neighbour's own material, and the direction is from the
            // neighbour back to (x, y), which is the negation of the offset
            // used to reach it. `dy == -1` means (x, y) sits below the
            // neighbour, i.e. the neighbour is standing on it. Getting this
            // backwards would silently price towers as cantilevers.
            let (below, above, beside) = costs[neighbour.material.0 as usize];
            let step = match dy {
                -1 => below,
                1 => above,
                _ => beside,
            };
            let candidate = distance.saturating_add(step);
            if candidate < dist[idx(nx, ny)] {
                dist[idx(nx, ny)] = candidate;
                heap.push(Reverse((candidate, nx, ny)));
            }
        }
    }

    // Write back, only where the answer differs from what the cell already
    // holds. On generated terrain that is nearly every solid cell, so this
    // is not an optimisation so much as an assurance that a cell nothing
    // decided about is left exactly as it was -- including its dirty state,
    // which `World::set` is what maintains.
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            let cell = cells[idx(x, y)];
            if !relaxable(cell) {
                continue;
            }
            let d = dist[idx(x, y)];
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
            let anchored = NEIGHBOURS_4.iter().any(|&(dx, dy)| world.get(x + dx, y + dy).material == material::BEDROCK)
                || is_resting_on_ground(world, x, y);
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
const DETACH_DEPTH: i32 = 3;

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
/// sitting on it gets re-examined. Liquids and gases are excluded — floating
/// is buoyancy, not support, and nothing here models it.
///
/// Only the cell *directly below*: this answers "is it standing on the
/// ground", not "is there a second, weaker way to span a gap sideways".
fn is_resting_on_ground(world: &World, x: i32, y: i32) -> bool {
    world.materials.kind(world.get(x, y + 1).material) == MaterialKind::Powder
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
    /// because anything older has fallen outside `chain_window` and a
    /// player cannot disturb dozens of places in the same second.
    pub fn record_disturbance(&mut self, x: i32, y: i32) {
        let frame = self.frame;
        self.disturbances.push_back((x, y, frame));
        while self.disturbances.len() > MAX_DISTURBANCES {
            self.disturbances.pop_front();
        }
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
        self.disturbances.iter().any(|&(dx, dy, frame)| {
            self.frame.saturating_sub(frame) <= self.chain_window && (x - dx).abs().max((y - dy).abs()) <= self.chain_reach
        })
    }

}

/// How many recent disturbances are remembered. A player cannot disturb
/// more places than this inside one `chain_window`, and a blast that wants
/// a wider licence should raise `chain_reach` rather than report itself
/// many times.
const MAX_DISTURBANCES: usize = 16;

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

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 63, 63))
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

    /// A slab cut free by cracks in the *middle of a massif* must not turn
    /// into rubble, and the identical slab with a cavity above it must.
    ///
    /// **A paired comparison, and it has to be**, because either half
    /// alone is passable by cheating in one direction. "Nothing turns to
    /// rubble" passes the first by making rock indestructible, which is
    /// how four earlier support models died; "everything turns to rubble"
    /// passes the second. The pair is what pins the rule to *free space*
    /// rather than to strength.
    ///
    /// It is also the guard against the replacement artifact rather than
    /// the original one: the way this fix fails is not by leaving the
    /// mid-mountain collapse in place, it is by suppressing collapse
    /// everywhere and leaving cliffs hanging in the air.
    #[test]
    fn rock_with_nowhere_to_go_cracks_where_it_stands() {
        /// The slab is x 27..35 by y 33..37.
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
            // Cut the slab loose: cracks all the way round x 27..35, y
            // 33..37. Support cannot cross a fracture, so this leaves it
            // with no parent at all in either world.
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
                    for x in 26..36 {
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
                for x in 27..35 {
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
            |w: &World| (33..37).flat_map(|y| (27..35).map(move |x| (x, y))).filter(|&(x, y)| w.get(x, y).material == material::STONE).count();

        let mut buried = massif_with_isolated_slab(false);
        run(&mut buried, 300);
        let mut opened = massif_with_isolated_slab(true);
        run(&mut opened, 300);

        assert!(
            buried.structural_failures.confined > 0,
            "test setup: the buried slab was never judged as a confined failure, so this proves nothing"
        );
        assert_eq!(
            slab_intact(&buried),
            SLAB_CELLS,
            "a slab in the middle of a massif has nowhere to go: every cell of it must still be standing there"
        );
        assert!(
            slab_intact(&opened) < SLAB_CELLS,
            "the same slab with a cavity above it must still come apart -- {} of {SLAB_CELLS} cells still intact",
            slab_intact(&opened)
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
                w.record_disturbance(dx, dy);
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

    fn organism_wood_cell(w: &mut World, organism_id: u16) -> Cell {
        let wood = w.materials.id_of("wood").unwrap();
        Cell::new(wood, 0).with_organism_id(organism_id).with_aux(organism::pack_cell_type(organism::CellType::MatureBody))
    }

    #[test]
    fn an_organism_tree_beam_within_its_span_stays_wood() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let organism_id = w.push_organism(tree_species);
        // Anchored at the left end by a stone cell directly below it --
        // organism_is_supported's own generalization of "touches BEDROCK"
        // to "touches Solid ground". 6 cells, within wood's span of 8.
        w.set(0, 31, Cell::new(material::STONE, 0));
        for x in 0..6 {
            let cell = organism_wood_cell(&mut w, organism_id);
            w.set(x, 30, cell);
        }
        w.schedule_structural_check(5, 30); // the far end, distance 5
        run(&mut w, 200);

        for x in 0..6 {
            assert_eq!(w.get(x, 30).organism_id(), organism_id, "an anchored, in-span organism beam broke (or lost its organism_id) at x={x}");
        }
    }

    #[test]
    fn an_organism_tree_beam_exceeding_its_span_breaks_into_deadwood() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let organism_id = w.push_organism(tree_species);
        w.set(0, 31, Cell::new(material::STONE, 0));
        // 12 cells -- longer than wood's span of 8, so the far end (distance
        // 11) is unsupported once actually checked.
        for x in 0..12 {
            let cell = organism_wood_cell(&mut w, organism_id);
            w.set(x, 30, cell);
        }
        w.schedule_structural_check(11, 30);
        run(&mut w, 200);

        let deadwood = w.materials.id_of("deadwood").unwrap();
        assert_eq!(w.get(11, 30).material, deadwood, "an over-span organism-owned wood cell should have broken into deadwood");
        assert_eq!(w.get(11, 30).organism_id(), 0, "broken-free debris should no longer belong to the organism");
    }

    #[test]
    fn cutting_an_organism_trees_support_collapses_the_far_side() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let organism_id = w.push_organism(tree_species);
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
        run(&mut w, 200);
        assert_eq!(w.get(13, 30).organism_id(), organism_id, "test setup: the intact, anchored beam should not have broken yet");

        // Cut the anchor itself -- every cell in the beam is now
        // unsupported, not just one step further away, since it was the
        // *only* thing touching Solid ground.
        w.set(10, 31, Cell::EMPTY);
        w.schedule_structural_check_around(10, 31);
        w.schedule_structural_check(10, 30);
        run(&mut w, 200);

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
            let organism = w.push_organism(w.species.id_of("tree").expect("tree is compiled in"));
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
        run(&mut bare, 200);
        run(&mut loaded, 200);

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

    /// W1's crush half: `detach: false` must leave attachment and the
    /// scheduler alone. Unbracing confined rock tells the load model the
    /// inside of a mountain has come free, and rescheduling is what turns a
    /// crush into a treadmill -- both recorded as measured dead ends in
    /// `crush_in_place`'s own doc, and both are one flag away now.
    #[test]
    fn a_crush_walk_neither_unbraces_the_rock_nor_reschedules_it() {
        let mut w = test_world();
        massif(&mut w);
        let scheduled_before = w.active_site_count();
        let written = walk_fissures(&mut w, (32, 32), 0.7, 3, 30, 4, false);

        assert!(written > 0, "test setup: the walk should have scored something");
        assert_eq!(w.active_site_count(), scheduled_before, "a confined crush must schedule no structural checks");
        let unattached = (0..64).map(|x| (0..64).filter(|&y| !w.get(x, y).attached()).count()).sum::<usize>();
        assert_eq!(unattached, 0, "a confined crush must not unbrace anything: {unattached} cells lost attachment");
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
    /// needed one: a crack is a bit, the wander is keyed on position, so a
    /// second confined failure over the same rock retraces the same paths
    /// and manufactures nothing. `tick`'s "a crush that wrote nothing has
    /// nothing to propagate" guard is what stopped a crushed pocket
    /// re-failing 1,120 times every 400 frames, and it reads exactly this
    /// number.
    #[test]
    fn crushing_the_same_rock_twice_writes_nothing_the_second_time() {
        let mut w = test_world();
        massif(&mut w);
        let first = walk_fissures(&mut w, (32, 32), 0.7, 3, 30, 4, false);
        let second = walk_fissures(&mut w, (32, 32), 0.7, 3, 30, 4, false);
        assert!(first > 0, "test setup: the first crush should have scored something");
        assert_eq!(second, 0, "a re-crush wrote {second} fresh cells; confined damage must not accumulate");
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
        let mut w = World::new(Rect::new(0, 0, 255, 159));
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
    #[test]
    fn a_paced_remainder_falls_even_when_the_disturbance_cannot_reach_it() {
        let mut w = World::new(Rect::new(0, 0, 255, 159));
        let slab = ligament(&mut w);
        compute_world_distances(&mut w);
        w.chain_reach = CHAIN_MODES.iter().find(|m| m.name == "TIGHT").expect("a TIGHT chain mode").reach;
        w.record_disturbance(45, 62);
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
}
