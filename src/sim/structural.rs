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
use super::chunk::Rect;
use super::material::{self, MaterialId, MaterialKind};
use super::scheduler::{ActiveKind, ActiveSite};
use super::world::World;

/// The four-neighbourhood, in the order every tie in this subsystem breaks
/// on. `load.rs` derives support parents with the same `argmin` the
/// relaxation below uses, so it has to iterate in the identical order or
/// the two disagree about which neighbour holds a cell up whenever two
/// paths cost the same — which is most of a solid slab.
pub const NEIGHBOURS_4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

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
    let is_anchor = NEIGHBOURS_4.iter().any(|&(dx, dy)| world.get(x + dx, y + dy).material == material::BEDROCK)
        || is_resting_on_ground(world, x, y);

    let new_distance: u16 = if is_anchor {
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
    if let super::load::ChainVerdict::Failing(failure) = verdict {
        // The forest this describes is about to change out from under it.
        world.load_cache.clear();
        world.structural_failures.record(failure.mode, failure.region.len());
        let reach = (failure.at.0 - x).unsigned_abs() + (failure.at.1 - y).unsigned_abs();
        world.structural_failures.record_reach(reach);
        let region = failure.region;
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
    // Saturating on purpose: a material with no finite span (moss's own)
    // must stay effectively unbounded under any multiplier.
    let density = world.organism(cell.organism_id()).map_or(1.0, |s| {
        organism::WOOD_DENSITY_ALLELES[(s.alleles[organism::LOCUS_WOOD_DENSITY] as usize).min(organism::WOOD_DENSITY_ALLELES.len() - 1)]
    });
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
    let over_span = !detached && max_span != u16::MAX && support > effective_span;
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
    break_free(world, x, y);
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
/// **9.1 ms for the sandbox's 512x320 terrain (6,616 solid cells), paid
/// once at generation and never per frame** — reported by
/// `examples/ascii.rs`, which times it against the same terrain built
/// without this pass so the figure is attributed rather than asserted.
///
/// The relaxation itself is the cheap half: cells inside bulk rock reach
/// bedrock through free downward steps and settle immediately, so the
/// search runs along free *surfaces* rather than through volumes. What
/// dominates is the **seeding scan** — one `World::get` per cell across the
/// whole world, hashed per lookup, which is issue #5's exact pattern
/// ("~164k hashed `World::get` calls… index the chunk directly instead")
/// showing up in a new place. So the honest claim is that the *search*
/// scales with surface area while this function as written still scales
/// with world volume.
///
/// That is acceptable precisely because it is one-off, and it stops being
/// world-sized at all under M10 streaming, where this becomes a per-chunk
/// pass (§6b: "a cheap BFS from bedrock, once per chunk", with anchor
/// distance living on the coarse layer) and the scan is bounded by a chunk.
/// If it ever needs to be faster before then, iterating chunks directly is
/// the fix, not a cleverer search. It no-ops on an unbounded world rather
/// than pretending to handle that case here.
pub fn compute_world_distances(world: &mut World) {
    let Some(bounds) = world.bounds() else {
        return; // unbounded (M10) -- see the doc above
    };

    // Seed: anchors at 0, everything else at "unreachable" so a cell the
    // search never reaches ends up honestly unsupported rather than
    // accidentally reading as anchored.
    let mut heap: BinaryHeap<Reverse<(u16, i32, i32)>> = BinaryHeap::new();
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            let cell = world.get(x, y);
            if !is_relaxable(world, cell) {
                continue;
            }
            let anchored = NEIGHBOURS_4.iter().any(|&(dx, dy)| world.get(x + dx, y + dy).material == material::BEDROCK);
            let distance = if anchored { 0 } else { u16::MAX };
            world.set(x, y, cell.with_aux(distance));
            if anchored {
                heap.push(Reverse((0, x, y)));
            }
        }
    }

    // Relax. `Reverse` makes this a min-heap ordered on (distance, x, y) --
    // the position tiebreak is what keeps the result identical run to run,
    // the same reason `ActiveSite`'s own `Ord` spells its tiebreak out
    // (issue #7 / determinism §8b).
    while let Some(Reverse((distance, x, y))) = heap.pop() {
        if world.get(x, y).aux() != distance {
            continue; // superseded by a shorter path already processed
        }
        for (dx, dy) in NEIGHBOURS_4 {
            if edge_is_cracked(world, x, y, dx, dy) {
                continue;
            }
            let (nx, ny) = (x + dx, y + dy);
            let neighbour = world.get(nx, ny);
            if !is_relaxable(world, neighbour) {
                continue;
            }
            // The cost is paid by the cell being *supported* -- so it reads
            // the neighbour's own material, and the direction is from the
            // neighbour back to (x, y), which is the negation of the offset
            // used to reach it. `dy == -1` means (x, y) sits below the
            // neighbour, i.e. the neighbour is standing on it. Getting this
            // backwards would silently price towers as cantilevers.
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
const CRACK_DETACH_DEPTH: i32 = 2;

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
}

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

    #[test]
    fn an_organism_tree_beam_within_its_span_stays_wood() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let organism_id = w.push_organism(tree_species);
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
        let organism_id = w.push_organism(tree_species);
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
            let organism = w.push_organism(w.species.id_of("tree").expect("tree is compiled in"));
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

}
