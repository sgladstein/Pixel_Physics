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
use super::scheduler::{ActiveKind, ActiveSite};
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
    // Weather before the sweep, so rain landing this frame is material the
    // sweep then moves -- rather than a drop that sits for a frame before
    // anything notices it. Both drivers, deliberately: `CLAUDE.md`'s "two
    // drivers, and the app runs the parallel one".
    super::weather::step(world);
    // Springs beside weather and for the same reason: water that arrives
    // this frame is material the sweep then moves. Both drivers.
    super::spring::step(world);

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

    // Organism resource/canopy-density transport used to run from here, on
    // the CA sweep. It now runs as one pass per organism from
    // `plant::step_organisms` -- `Reports/plant-substrate-v2-design.md`
    // §3d, forced by the scalars moving to `OrganismState` where a
    // `CellSurface` cannot reach them, and wanted independently because
    // dispatching from the sweep tied transport to chunk wakefulness.
    //
    // The requirement that put it here in the first place is unchanged and
    // still met: a `MatureBody` trunk cell keeps relaying resource while
    // staying off the active-site schedule (`design-philosophy.md` §3).
    // Iterating the organism's own cell list satisfies that directly,
    // rather than by leaning on the sweep to visit every cell of an awake
    // chunk. Do not reintroduce a per-cell call here.

    match surface.materials().kind(cell.material) {
        MaterialKind::Powder => update_powder(surface, x, y, cell, rightward),
        MaterialKind::Liquid => {
            let moved = update_liquid(surface, x, y, rightward);
            // The sweep's entire part in evaporation: hand a *settled*
            // surface cell to the active-site scheduler and forget about it.
            // Doing the evaporating here instead is what was built and
            // reverted -- a settled chunk is not swept, so still water, the
            // one state this mechanic is about, is the one state this arm
            // never sees again (`evaporation.rs`'s module doc).
            //
            // Gated on `!moved` so a waterfall does not queue a site per
            // cell per frame while it is in flight, and on the material flag
            // at this dispatch site, which already holds the `Cell`.
            if !moved && surface.materials().get(cell.material).evaporates {
                super::evaporation::schedule_from_sweep(surface, x, y);
            }
            moved
        }
        MaterialKind::Gas => update_gas(surface, x, y, rightward),
        MaterialKind::Empty | MaterialKind::Solid | MaterialKind::Plant | MaterialKind::Creature => false,
    };
}

/// Falls straight down, then diagonally, then creeps along the slope.
/// How much of a saturated cell's surplus moves down per visit. Not a
/// rate with a physical name — real infiltration follows Richards'
/// equation and this is a relaxation constant — but it is what turns
/// drainage into a visible *wetting front* descending over time rather
/// than an instant teleport, which is the behaviour worth having.
const SOIL_DRAINAGE_RATE: f32 = 0.25;

/// Share of a moisture *difference* that capillary action moves per visit,
/// before the wetness scaling in `update_soil_water` is applied.
///
/// Well under drainage's rate on purpose: unsaturated flow is genuinely much
/// slower than gravity drainage, and this is what keeps a wetting front
/// reading as a front descending through soil rather than a blob that
/// instantly averages itself out.
const SOIL_CAPILLARY_RATE: f32 = 0.06;

/// Is this grain held in place by a root threading through it?
///
/// Four-neighbour, not eight: a root crossing a shear plane reinforces the
/// soil it is actually embedded in, and a diagonal corner-touch is a weaker
/// claim than this binary rule should be making. Erring toward *less*
/// stabilization keeps the mechanic from quietly freezing whole hillsides
/// off one stray root.
///
/// Matched by name once, not by a hardcoded id: `rootwood` is data like
/// every other material, and a world loaded without it simply has no
/// root-reinforced soil rather than failing.
fn root_reinforced<S: CellSurface>(surface: &S, x: i32, y: i32) -> bool {
    [(0, -1), (0, 1), (-1, 0), (1, 0)]
        .iter()
        .any(|&(dx, dy)| surface.materials().get(surface.get(x + dx, y + dy).material).reinforces_powder)
}

/// Whether this cell is resting against woody tissue -- a branch, a trunk,
/// or a landed log.
///
/// `root_reinforced` above read from the other side: there a root holds the
/// soil it threads through, here a branch holds the leaves that came down on
/// it. Same four neighbours, same "before any movement rule" placement, and
/// the same reason it asks the *material* rather than organism state -- this
/// function holds a bare `Cell` and has no route to an organism.
///
/// Four neighbours rather than eight, deliberately. A leaf touching a branch
/// only at a corner is not sitting on it, and eight would let a spray hang
/// diagonally off a twig's tip in open air.
///
/// # Two steps, not one, and the second one is measured
///
/// **Touching a branch was too strict to mean what it was written for.** The
/// owner's complaint is that a felled crown's leaves come off the branch, and
/// he has made it four times; this rule shipped, and they still did. The
/// reason is that a crown is mostly leaf, so most leaf cells rest on *other
/// leaves* rather than on wood, and a one-step rule lets every one of them
/// go. Censused on the settled `scene=fell` pile (`filmstrip`'s "foliage by
/// steps to the nearest wood"), of 1,122 foliage cells:
///
/// | steps to wood | 1 | 2 | 3 | 4 | 5+ | none |
/// |---|---|---|---|---|---|---|
/// | cells | **405** | 201 | 130 | 85 | 214 | 87 |
///
/// So one step holds **36%** and two holds **53%** — which is the first
/// depth at which "most of the leaves stay on the branch" is true rather
/// than nearly true.
///
/// **Two and not more**, because the cost of going deeper is the recorded
/// dead end and not the arithmetic. `deadleaf.ron` records what happened when
/// foliage was made a `Solid`: the crown froze in mid-air, the first pieces
/// to land became a scaffold nothing could fail, and the tree died standing
/// in the shape it grew in. Depth is how near that this gets — every step
/// freezes a thicker shell of foliage around each limb, and at four or five
/// a dense pile of mixed log and leaf is immobile wholesale. Two doubles what
/// holds while still letting a leaf three cells out of a spray fall.
///
/// Foliage in open air is untouched at any depth: the chain has to terminate
/// on *wood*, so a drift resting on nothing but itself still falls.
fn on_a_branch<S: CellSurface>(surface: &S, x: i32, y: i32) -> bool {
    const NEIGHBOURS: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];
    let woody = |x: i32, y: i32| surface.materials().get(surface.get(x, y).material).woody;
    if NEIGHBOURS.iter().any(|&(dx, dy)| woody(x + dx, y + dy)) {
        return true;
    }
    // The second step, and it is only paid by foliage that is *not* already
    // on a branch -- the 64% of it that the one-step rule was dropping.
    NEIGHBOURS.iter().any(|&(dx, dy)| {
        let (nx, ny) = (x + dx, y + dy);
        surface.materials().get(surface.get(nx, ny).material).clings_to_wood
            && NEIGHBOURS.iter().any(|&(ex, ey)| woody(nx + ex, ny + ey))
    })
}

/// Infiltration and gravity drainage for one `Powder` cell.
///
/// Two rules, both local, run before the movement rules below:
///
/// - **Infiltration.** An adjacent `Liquid` is drunk by unsaturated soil.
///   This is what makes rain, a burst pipe or a spilled bucket soak *into*
///   ground instead of running over it as though the ground were glass.
/// - **Drainage.** Soil above field capacity passes its surplus to the
///   cell below, capped by that cell's remaining room. That is
///   `transfer_liquid_vertical`'s own shape at a different scale, and it
///   is what produces a descending wetting front rather than a uniformly
///   damp block.
///
/// Together these close a loop the moisture channel has never had. Today
/// `field.rs` forces moisture to `MAX_MOISTURE` wherever a `Liquid` cell
/// sits and that is its only source — deposit, and nothing else. Now:
/// liquid infiltrates soil, soil holds and drains it, roots drink it and
/// deplete it, depleted soil reads dry again. `design-philosophy.md` §0's
/// point is that behaviour count scales with closed loops, not systems.
///
/// Returns whether anything was written, so the caller can keep its own
/// "did this cell do something" bookkeeping honest.
fn update_soil_water<S: CellSurface>(surface: &mut S, x: i32, y: i32) -> bool {
    let cell = surface.get(x, y);
    // Opt-in per material -- see `Material::water_capacity`. A powder that
    // holds no water neither absorbs nor drains, so sand and gravel behave
    // exactly as they did and the engine's liquid conservation tallies stay
    // true.
    let capacity = surface.materials().get(cell.material).water_capacity;
    if capacity == 0 {
        return false;
    }
    let here = soil_moisture(cell);
    let mut moisture = here;

    // Infiltration: soil drinks an adjacent liquid, taking only what fits
    // and leaving the rest as water.
    //
    // **Both halves of this were wrong, and the first was reported from
    // live play** — "there is no mechanism for water getting absorbed into
    // soil and increasing its moisture", which was correct as observed even
    // though the mechanism was right here.
    //
    // The gate used to be `moisture + SOIL_SATURATED / 2 <= capacity`,
    // meaning soil could only absorb while under **half** saturated. Field
    // capacity is 620 of 1000, so any ground at or above field capacity —
    // which is what ordinary damp soil between rain events *is*, and what
    // `filmstrip`'s `forest` scene starts at — was completely waterproof. A
    // puddle sat on top of it forever. The gate existed to stop a nearly
    // full cell destroying a whole water cell for a sliver of moisture,
    // which was a real problem; it was just solved in the wrong place.
    //
    // Solving it properly removes the need for the gate: take `min(fill,
    // room)` and **write the remainder back as water** instead of deleting
    // the cell outright. Absorbing a cell whole and keeping only part of it
    // was a silent mass leak, and one the old gate hid rather than fixed —
    // `infiltration_conserves_water_it_cannot_fit` passed against the gated
    // version only because infiltration never ran at all in that setup.
    for (dx, dy) in [(0, -1), (-1, 0), (1, 0), (0, 1)] {
        if moisture >= capacity {
            break;
        }
        let n = surface.get(x + dx, y + dy);
        if surface.materials().kind(n.material) != MaterialKind::Liquid {
            continue;
        }
        let fill = liquid_fill(n);
        let taken = fill.min(capacity - moisture);
        if taken == 0 {
            continue;
        }
        moisture += taken;
        if taken >= fill {
            surface.set(x + dx, y + dy, Cell::EMPTY);
        } else {
            // Partly drunk. `aux` on a `Liquid` is its fill, and 0 there
            // means *full* rather than empty, so a genuinely drained cell
            // must become `Cell::EMPTY` and never `with_aux(0)` — the
            // gotcha `material::LIQUID_FULL`'s own doc exists for.
            surface.set(x + dx, y + dy, n.with_aux(fill - taken));
        }
        break; // one neighbour per visit -- infiltration is not instant
    }

    // **Capillary redistribution: wet soil feeds dry soil, in any
    // direction.** Distinct from the gravity drainage below, and both are
    // real:
    //
    // - *Saturated* flow is gravity-driven, downward only, and only above
    //   field capacity. That is the drainage rule below (Darcy).
    // - *Unsaturated* flow follows gradients of **matric potential** — the
    //   suction dry soil exerts — and moves water sideways and even upward,
    //   from wetter soil toward drier. This is Richards' equation's other
    //   half, and it is why a wetting front spreads laterally as well as
    //   descending, and why a soil profile evens out between rain events.
    //   Without it a plume stays a plume forever and a root drinking a cell
    //   dry gets no resupply from the damp soil beside it.
    //
    // **The rate falls steeply as soil dries, and that is the load-bearing
    // detail rather than a refinement.** Unsaturated hydraulic conductivity
    // drops by orders of magnitude between saturation and the wilting
    // point: wet soil redistributes readily, dry soil barely at all. Scaling
    // the exchange by the wetter cell's own saturation captures the
    // direction of that dependence. Named honestly as a linear stand-in for
    // a relationship that is really closer to a power law — the ordering is
    // faithful, the curve is not.
    //
    // Only the `+x` and `+y` faces are visited, so each shared face is
    // handled exactly once and the exchange conserves rather than depending
    // on sweep order.
    for (dx, dy) in [(1, 0), (0, 1)] {
        let n = surface.get(x + dx, y + dy);
        let n_capacity = surface.materials().get(n.material).water_capacity;
        if n_capacity == 0 {
            continue;
        }
        let there = soil_moisture(n);
        if moisture == there {
            continue;
        }
        let (wetter, drier) = if moisture > there { (moisture, there) } else { (there, moisture) };
        // **A rest threshold, so standing dampness is representable — and
        // its value is derived, not tuned.** Without one, capillary
        // equalizes every gradient toward ±1 unit, so any local wetness (a
        // generated water table, a pond's damp margin, a watered plot)
        // keeps its chunks awake until the whole soil bed is uniform.
        //
        // The value must be at least the drainable band —
        // `SOIL_SATURATED - SOIL_FIELD_CAPACITY` — and here is the argument,
        // which cost a real derivation session: a saturated zone borders
        // unsaturated soil somewhere, and every cell in that border is
        // pulled by two rules with incompatible rest states. Drainage is
        // only at rest at or below field capacity; capillary is only at
        // rest across gradients at or under this threshold. Any cell
        // holding between field capacity and saturation, with room in the
        // cell below, drains — and capillary then refills it from the
        // saturated side, forever. A perpetual two-cell pump, at every
        // water-table boundary in the world. The *only* standing profile is
        // saturation stepping straight down to field capacity in one cell,
        // which requires the threshold to span that step. Physically this
        // is the specific-yield band: suction cannot hold water that
        // gravity can drain, so gradients inside the drainable band do not
        // stand in nature either.
        //
        // What must keep flowing, and does: a root drying its cell toward
        // the wilting point (180) beside field-capacity soil (620) is a
        // 440 gradient; a wetting front from standing water starts near
        // 1000 against dry soil. Both clear 380. What stops is the final
        // sub-band remainder, which was pure churn.
        const SOIL_CAPILLARY_REST: u16 = material::SOIL_SATURATED - material::SOIL_FIELD_CAPACITY;
        if wetter - drier <= SOIL_CAPILLARY_REST {
            continue;
        }
        // Wetness is a *fraction of the wetter cell's own* capacity, and
        // the room available is the *receiver's*. Both used to read this
        // cell's `capacity` regardless of which side was which, so a
        // neighbour with a smaller capacity could be pushed above its own
        // limit and a wetter neighbour's fraction was computed against the
        // wrong denominator.
        //
        // **Latent, not live** -- `water_capacity` is opt-in and `soil` is
        // the only material that has it, so every exchange today is
        // soil-to-soil with equal capacities and both readings agree. It
        // goes live the moment a second water-holding powder exists, which
        // is exactly what widening `water_capacity` to sand would do (see
        // `Material::water_capacity`'s own note on that being flagged
        // rather than done). Fixed now because the cost is two lines and
        // the failure would be a silent over-fill. Found by independent
        // review.
        let wetter_capacity = if moisture > there { capacity } else { n_capacity };
        let wetness = wetter as f32 / wetter_capacity.max(1) as f32;
        let room = if moisture > there { n_capacity.saturating_sub(there) } else { capacity.saturating_sub(moisture) };
        let moved = ((((wetter - drier) as f32) * SOIL_CAPILLARY_RATE * wetness) as u16).min(room);
        if moved == 0 {
            continue;
        }
        if moisture > there {
            moisture -= moved;
            surface.set(x + dx, y + dy, n.with_aux(there + moved));
        } else {
            moisture += moved;
            surface.set(x + dx, y + dy, n.with_aux(there - moved));
        }
    }

    if moisture > material::SOIL_FIELD_CAPACITY {
        let below = surface.get(x, y + 1);
        let below_capacity = surface.materials().get(below.material).water_capacity;
        if below_capacity > 0 {
            let room = below_capacity.saturating_sub(soil_moisture(below));
            let surplus = moisture - material::SOIL_FIELD_CAPACITY;
            let moved = ((surplus as f32 * SOIL_DRAINAGE_RATE) as u16).min(room);
            if moved > 0 {
                moisture -= moved;
                surface.set(x, y + 1, below.with_aux(soil_moisture(below) + moved));
            }
        }
    }

    if moisture != here {
        surface.set(x, y, cell.with_aux(moisture));
        // **The other place soil gets wet**, and the second of the only two
        // moments a damp cell can be put on the drying schedule -- see
        // `evaporation::schedule_damp_soil`. It has to happen here, while
        // the chunk is provably awake, because a settled damp bed is never
        // swept again: the `false` return below is what lets it sleep, and
        // a hook keyed on being visited would fire exactly never.
        //
        // Cheap and self-limiting: the predicate rejects anything buried or
        // already at the dry floor before a site is built, and
        // `World::schedule_active_site` dedups by position, so a bed under a
        // long storm ends up with one site per surface cell rather than one
        // per drop.
        super::evaporation::schedule_damp_soil(surface, x, y);
        return true;
    }
    false
}

fn update_powder<S: CellSurface>(surface: &mut S, x: i32, y: i32, cell: Cell, rightward: bool) -> bool {
    // Water first: a grain that is about to move should carry the moisture
    // it just absorbed with it, and `move_cell` copies the whole cell.
    //
    // **The opt-in check is here, not inside `update_soil_water`, and that
    // placement is a measured requirement rather than a tidiness
    // preference.** Guarding inside the function still cost a
    // `surface.get` per powder cell per frame to find out the material
    // holds no water, and the `ascii` sand-and-water stress scene -- which
    // is entirely sand, holding no water at all -- went from ~8.1 ms worst
    // frame to 10.2-12.7 ms. `CLAUDE.md` is explicit that frame cost is a
    // hard constraint and not a tiebreaker, and paying 25-50% for a
    // feature that does nothing in that scene is not a trade worth making.
    // The caller already has `cell`, so reusing it makes the check a Vec
    // index and nothing else.
    // One lookup for both opt-ins: the caller already has `cell`, so this is
    // a `Vec` index and two field reads rather than two indexes.
    let def = surface.materials().get(cell.material);
    let holds_water = def.water_capacity > 0;
    let clings = def.clings_to_wood;
    let self_supporting = def.self_supporting;
    let wet_changed = if holds_water { update_soil_water(surface, x, y) } else { false };

    // **A worked wall holds itself up -- the whole of why a burrow can be a
    // place rather than a five-frame event.**
    //
    // Measured before this existed (`examples/burrow_probe.rs`): a shaft, a
    // gallery and a chamber cut into a bed of `soil` read 63%/67%/82% open
    // one frame later, and the gallery is **gone by frame 5**. The same
    // excavation in `stone` holds 100% at every frame, so it is the powder
    // rules and not the scene. What closes it is the unconditional
    // straight-down move below -- the roof simply falls into the hole --
    // and *no* setting of `friction_angle` reaches that: repose governs
    // `roll_along_slope` only, and its own doc says a pile can only get
    // flatter, never overhanging. At 89 degrees the roll reach goes to zero
    // and `try_move(x, y + 1)` still fires.
    //
    // So the difference between a bank of loose grains and a wall someone
    // worked is stated as **data on the material**, which is `CLAUDE.md`'s
    // "when a rule must tell apart two things that can look identical,
    // state the difference as data" -- the same finding four successive
    // support models produced by trying to infer it from shape.
    //
    // Placed above `root_reinforced` because it is strictly cheaper: a
    // `bool` already in hand against four neighbour fetches. Placed *below*
    // the water pass because the pass is what decides whether this cell is
    // still packed at all -- see the waterlogging branch.
    //
    // Read off the material at the dispatch site that already holds the
    // `Cell`, per the note on the water opt-in above: every other powder in
    // the world pays one `Vec` index and a `bool` test for this, and the
    // all-sand stress scene has no ants in it.
    if self_supporting {
        // **The graded half, and the reason this is not the binary
        // `CLAUDE.md` warns about.** A packing that works by grain contact
        // has nothing to grip with once the pore space is full, so above
        // field capacity the wall reverts to whatever `slumps_into` names
        // and the ordinary fall rules take the tunnel down from there. A
        // gallery driven below the water table, or one that floods, comes
        // in; a dry one stands.
        //
        // The cell is re-read rather than reusing `cell` because
        // `update_soil_water` has just written this frame's moisture into
        // it, and reading the stale copy would answer with last frame's
        // water. Only a `self_supporting` cell pays this second `get`.
        //
        // `SOIL_FIELD_CAPACITY` rather than an authored per-material
        // threshold: it is already the engine-wide line between water the
        // pore space holds and water that drains through
        // (`update_soil_water`'s drainage rule uses the same one), so a
        // resting column sits at or below it by construction and this
        // branch only fires where water is arriving faster than it leaves.
        //
        // **Chosen here rather than in `decay.rs`**, which was the other
        // candidate: that channel is scheduled on a chunk's awake ->
        // settled transition and re-checks every 200 frames against the
        // *coarse field* moisture. Both halves are wrong for this. A tunnel
        // taking on water is in an awake chunk, which is exactly when a
        // settle-scheduled site does not exist; and a block-nearest field
        // read cannot see one cell's water (`CLAUDE.md`'s coarse-field
        // gotcha, four bugs and counting). Here the trigger is the cell's
        // own `aux`, one frame after the water arrives.
        let here = surface.get(x, y);
        if here.aux() > material::SOIL_FIELD_CAPACITY {
            // Copied out before the write: `slumps_into` is `Copy`, and
            // holding the `&Material` across `surface.set` would borrow
            // `surface` immutably and mutably at once.
            let loose = surface.materials().get(here.material).slumps_into;
            if let Some(loose) = loose {
                // Everything but the material is carried across --
                // moisture (`aux`), the palette index, the attached flag,
                // temperature. A wall that slumps is the *same ground*,
                // and rebuilding the cell would silently dry it out, which
                // on a `Powder` is what `aux == 0` means.
                let mut slumped = here;
                slumped.material = loose;
                surface.set(x, y, slumped);
                // Reported as a write so the chunk stays awake and the
                // collapse this just unlocked actually runs. The cell
                // falls on its next visit rather than this one, which
                // avoids acting on the stale `cell` bound above.
                return true;
            }
        }
        return wet_changed;
    }

    // **Root-reinforced soil does not fall.** One check, before any of the
    // movement rules, and it is the mirror of "too much weight breaks a
    // branch" pointing the other way: today the world acts on plants
    // (light, moisture, wind, fire) and plants act back on almost nothing.
    //
    // `Reports/plant-substrate-v2-design.md` §6d, including its correction
    // of `PLAN.md`'s original framing. The plan proposed "extending
    // anchor-distance credit outward from a root into adjacent soil",
    // which cannot work: `Powder` cells take no part in `structural.rs` at
    // all -- they have no anchor distance and never break free, they simply
    // fall via this function every frame. There is no distance to extend
    // credit into. The correct place is here.
    //
    // Grounded in measured geotechnics rather than analogy. Roots crossing
    // a shear plane act as laterally loaded fibres in tension, resolving
    // into a tangential component that adds **apparent cohesion** to the
    // soil -- the Wu-Waldron model, from Waldron (1977), SSSAJ 41:843-849
    // and Wu, McKinnell & Swanston (1979), Can. Geotech. J. 16:19-33, still
    // the baseline in slope-stability practice.
    //
    // **Simplification, named:** apparent cohesion is a continuous strength
    // increment and this is a binary "does not move". The graded version
    // (root-adjacent soil gets a reduced roll reach, so it holds a steeper
    // slope without being fully immobile) is strictly more faithful and is
    // the obvious upgrade if binary reads as too absolute. Starting binary
    // because it is one check and immediately verifiable by eye.
    //
    // Asked of the *material*, which is the whole reason `rootwood` is a
    // material rather than a shaded `CellType`: this function holds a bare
    // `Cell` and has no route to organism state.
    // Gated on `holds_water`, which is already in hand, so a powder that
    // is not soil never pays the four neighbour fetches this needs. Same
    // reasoning as the water pass above: the sand-and-water stress scene
    // must not pay for a plant mechanic it has no plants for.
    if holds_water && root_reinforced(surface, x, y) {
        return wet_changed;
    }

    // **Foliage stays on the branch it came down on.** Owner, twice: *"most
    // of the leaves should stay on the branch"*, and before that *"They can
    // stay on the branch if that is easier."*
    //
    // A severed limb already rides down whole with its leaves attached
    // (`rigid::fell_severed_tissue`). What went wrong is what happens after
    // it lands: the foliage arrives as a powder, and a powder falls off
    // whatever it landed on, so within a few frames the leaves had slid
    // clear of the wood and pooled around it.
    //
    // The tempting fix is to make the tier a `Solid` instead, and it is a
    // recorded dead end -- built, rendered, and it leaves the tree **dying
    // standing in the shape it grew in**, because the first pieces to land
    // form a scaffold that nothing can ever fail. `deadleaf.ron`'s header
    // carries the measurement. This is the narrower claim that was actually
    // wanted, and the difference matters: foliage in open air still falls
    // like anything else, and only foliage *touching wood* holds.
    //
    // Gated on a flag read off the material already in hand, for exactly the
    // reason the water pass above states at length: a powder that is not
    // foliage must never pay the four neighbour fetches. Every other powder
    // in the world pays one `Vec` index and a `bool` test -- which is
    // `CLAUDE.md`'s "guard hot-path work at the call site that already has
    // the data", and the sand-and-water stress scene has no foliage in it at
    // all.
    if clings && on_a_branch(surface, x, y) {
        return wet_changed;
    }

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

    // Sinking into a liquid, a grain sometimes goes diagonally instead of
    // straight down -- drag, and the reason a real blob of sand hitting
    // water fans into a plume rather than descending as a slab.
    //
    // Not a refusal: the grain still moves, every frame, by exactly one
    // cell. That matters. Rate-limiting the sink instead (so the grain
    // sometimes stays put) was tried and is a trap -- a deferral that
    // writes nothing lets the chunk settle, and the grain freezes in
    // mid-water forever with the world asleep around it.
    //
    // Only when there is genuinely liquid below: a grain falling through
    // air, or resting on ground, is unaffected.
    let sinking = !below.is_empty() && surface.materials().kind(below.material) == MaterialKind::Liquid;
    let (first, second) = if surface.rng().flip() { (-1, 1) } else { (1, -1) };
    if sinking && surface.rng().chance(SINK_SPREAD) && (try_move(surface, x, y, x + first, y + 1) || try_move(surface, x, y, x + second, y + 1)) {
        return true;
    }

    if !hole_from_a_sideways_escape && try_move(surface, x, y, x, y + 1) {
        return true;
    }
    if try_move(surface, x, y, x + first, y + 1) || try_move(surface, x, y, x + second, y + 1) {
        return true;
    }
    if fall_through_organism(surface, x, y, cell, below) {
        return true;
    }
    if roll_along_slope(surface, x, y, rightward) {
        return true;
    }
    if slide_past_organism(surface, x, y, cell, rightward) {
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

        // **A material that rots also schedules its decay check here, on the
        // one frame it stops moving -- but this is now the *residual* path,
        // not the primary one.**
        //
        // `World::end_step` scans a chunk on its awake->settled transition and
        // schedules a site for every decayable cell in it, riding the scan
        // `recompute_reach` is already doing. That is the general mechanism:
        // it reaches ash painted straight from the brush, and litter that was
        // already lying there before the chunk woke, neither of which ever
        // passes through this branch.
        //
        // What it cannot reach is a cell in a chunk that **never settles** --
        // and the obvious permanently-awake chunk is the forest floor under a
        // working ant colony, which is precisely where litter matters. This
        // covers that. Both funnel through `schedule_active_site`'s dedup, so
        // the pair cannot stack sites or turn the rot rate into a function of
        // how often the ground was disturbed.
        //
        // **Inside the `flowing` branch, not beside it.** `flowing` is set by
        // the shared move helper for any powder that moved, so this branch is
        // the settle *transition*: it fires once per landing and is already
        // skipped for a cell merely sitting there. An earlier version sat one
        // line lower, on every resting frame, costing a dedup hash probe per
        // settled litter cell per frame for as long as its chunk stayed awake:
        // `ascii` mean 1.794 -> 2.285 ms, and the litter itself was not what
        // cost it.
        //
        // Rotting *inline* here, where the material is already in hand, was
        // built and reverted: a settled chunk sleeps, so the powder update
        // stops running on exactly the cells that need it. Only a *scheduled*
        // site reaches a cell in a sleeping chunk, which is what the scheduler
        // is for.
        if surface.materials().get(cell.material).decays_into.is_some() {
            surface.schedule_active_site(crate::sim::scheduler::ActiveSite {
                x,
                y,
                kind: crate::sim::scheduler::ActiveKind::Decay,
                // Scaled through `Surface::organism_due`, not `frame() +
                // DECAY_TICK_INTERVAL`: decay rides `growth_slowdown`, and
                // this site landed on `main` after the world clock was cut,
                // so the merge is where it would otherwise have opted out.
                next_frame: surface.organism_due(crate::sim::decay::DECAY_TICK_INTERVAL),
            });
        }
    }
    // A cell that only moved *water* has not moved, but it has written, so
    // its chunk must stay awake long enough for the wetting front to keep
    // descending. Reporting `false` here would let a soaking column settle
    // mid-front and freeze the water part-way down.
    //
    // The converse matters just as much for frame cost: once moisture
    // stops changing, `update_soil_water` writes nothing and this returns
    // false, so a settled damp bed sleeps exactly like a dry one. That is
    // the property to check if soil ever starts keeping chunks awake.
    wet_changed
}

/// How far a leaf will tunnel through living tissue in one frame.
///
/// **A bound on work, never a gate on whether the fall happens** — the
/// distinction `CLAUDE.md` names, and the reason this is not a
/// `LITTER_FALL_REACH`-style 512. Exhausting it leaves the cell exactly
/// where it was, to try again next frame from one cell lower once whatever
/// is beneath it has moved; it never resolves to "so it rests here". A run
/// of branch thicker than this simply takes more than one frame to fall
/// through, which is what a longer fall should look like anyway.
///
/// Capped at `MAX_REACH` because that is the hard per-frame movement cap
/// `parallel.rs`'s cross-chunk write-disjointness proof is keyed on: a
/// downward write of at most 32 can only ever land in the chunk row
/// immediately below, which is a different `cy` and therefore never active
/// in the same pass. 16 rather than 32 because no crown in this engine has
/// a 16-cell-thick branch and halving it halves the worst-case scan.
const ORGANISM_TUNNEL_REACH: i32 = 16;
// The bound above is an invariant, so state it where a future edit trips
// over it rather than leaving it to the prose — `parallel.rs` does the same
// for `MAX_REACH == CHUNK_SIZE / 2`.
const _: () = assert!(
    ORGANISM_TUNNEL_REACH <= MAX_REACH,
    "a per-frame move beyond MAX_REACH breaks parallel.rs's cross-chunk write-disjointness proof"
);

/// A shed leaf does not sit on the branch below it — it goes past.
///
/// **The 2D-slice rule, applied where a cell comes to rest rather than only
/// where it is created.** `plant::shed_to_litter` already walks a leaf down
/// through its own crown at the moment of abscission, on exactly this
/// reasoning: the world is one vertical slice of a 3D wood, a branch one
/// cell wide is not a shelf spanning the tree's whole depth, and a leaf
/// falling past it is the common outcome. That covered the shedding event
/// and nothing else, so litter that reached mid-canopy by *any* other route
/// stopped on the first branch and stayed — and because `shed_to_litter`'s
/// walk halts at the first cell that is not organism-owned, every leaf shed
/// in that column then stacked on top of it. Measured paired on
/// `litter_probe frames=12000 trees=8`, one build each: standing litter more
/// than 32 rows above the ground **23% → 0%**, resting on plant tissue
/// **57.6% → 44.1%**, and the four cells still airborne at frame 12,000 gone.
///
/// Only cells whose material opts in (`Material::falls_through_organisms`,
/// litter alone today) pass through; `deadwood` — a snapped branch — is
/// chunky enough to hang up in a crown and deliberately still does.
///
/// **Through organism cells only, and it must find real air on the far
/// side.** The scan stops dead at the first cell that is neither air nor
/// organism-owned, which is what preserves the drift piled against a trunk
/// that `litter.ron`'s 42-degree friction angle exists to build: under a
/// root collar's shoulder is packed ground, not air, so that pile finds
/// nothing to fall into and stays exactly where it is. The two cases look
/// identical to a one-cell test and are opposite verdicts — see
/// `litter_probe`'s module doc, which spells out why its `against-plant`
/// column counts both.
///
/// Never displaces anything: the landing cell must be empty, so this can no
/// more overwrite a plant cell than `shed_to_litter` can.
///
/// Called only after the straight-down and both diagonal moves have already
/// failed, so ordinary falling and ordinary sliding-off-a-branch are
/// untouched and this runs on a cell that is genuinely wedged. `below` is
/// passed in rather than re-fetched — the caller has it, and this is the
/// hottest path in the engine.
fn fall_through_organism<S: CellSurface>(surface: &mut S, x: i32, y: i32, cell: Cell, below: Cell) -> bool {
    // Cheapest discriminator first, and it is a flag read on a `Cell` the
    // caller already holds: nothing that is not sitting on living tissue
    // pays even the material lookup. A pile of sand answers in one branch.
    if below.organism_id() == 0 {
        return false;
    }
    if !surface.materials().get(cell.material).falls_through_organisms {
        return false;
    }
    // A range rather than a counter incremented in the body: clippy's
    // `explicit_counter_loop` rejects the latter, and it is right that this
    // reads better -- but note the versions disagree, so a green local
    // clippy is not evidence here. 1.94.1 accepted the counter form and CI's
    // 1.98.0 refused it.
    for probe in (y + 1)..(y + 1 + ORGANISM_TUNNEL_REACH) {
        let here = surface.get(x, probe);
        // Raw `material == EMPTY`, not `is_empty()`, for the reason
        // `shed_to_litter` gives: the managed-aware helper reads a promoted
        // liquid body's container cells as not-empty, and the question here
        // is "can a leaf pass through".
        if here.material == material::EMPTY {
            return try_move(surface, x, y, x, probe);
        }
        if here.organism_id() == 0 {
            return false;
        }
    }
    false
}

/// How far a leaf will slip sideways past a trunk in one frame.
///
/// A trunk is a few cells across, so this only has to clear one. Bounded for
/// the same reason `ORGANISM_TUNNEL_REACH` is, and by the same constant.
const ORGANISM_SLIP_REACH: i32 = 8;
const _: () = assert!(
    ORGANISM_SLIP_REACH <= MAX_REACH,
    "a per-frame move beyond MAX_REACH breaks parallel.rs's cross-chunk write-disjointness proof"
);

/// A drift of leaves spills *round* a trunk rather than climbing up it.
///
/// **The horizontal half of the 2D-slice rule, and it was the half still on
/// screen.** `fall_through_organism` above stopped leaves resting on
/// branches, and measured on the honest metric it worked -- 4 of 497 standing
/// cells left with any air under them. The owner still reported *"it didn't
/// look like the leaves were all on the floor"*, and he was right: what was
/// left is a pile that **cannot spread sideways**. A litter cell rolls
/// downhill only into open space, so a drift wedged between two trunks has
/// nowhere to go but up, and it climbs out of the forest floor into the lower
/// crown as a narrow column. It is grounded the whole way, so every measure
/// that asks *what is under this cell* calls it floor -- and at play zoom it
/// reads as exactly the thing that was supposed to be fixed. Measured on
/// `litter_probe frames=12000 trees=8` before this landed: **24% of standing
/// litter more than eight rows up, and 68% of that with plant tissue
/// immediately to its left or right.**
///
/// In three dimensions a drift banks round a trunk instead of stacking on top
/// of it, for the same reason a falling leaf passes a branch: the trunk does
/// not occupy the whole depth of the slice.
///
/// **The destination must be somewhere it can actually fall from, and that is
/// what makes this terminate.** Requiring only an empty cell on the far side
/// would let a leaf shuffle left and right across a trunk for ever, which
/// costs nothing in physics and keeps the chunk awake -- the frame-cost
/// failure `CLAUDE.md` names. Requiring open air *beneath* the destination
/// means every slip is immediately followed by a fall, so the cell's row
/// strictly increases and it can never return to the row it left. It may
/// zig-zag down past a trunk; it cannot oscillate on one.
///
/// Called only after the straight-down, both diagonals, the tunnel and
/// `roll_along_slope` have all failed, so this runs on a cell that is
/// genuinely walled in. The material test is first because, unlike the
/// vertical case, neither neighbour is in the caller's hand -- so a pile of
/// sand pays one `Vec` index and no cell fetches at all.
fn slide_past_organism<S: CellSurface>(surface: &mut S, x: i32, y: i32, cell: Cell, rightward: bool) -> bool {
    if !surface.materials().get(cell.material).falls_through_organisms {
        return false;
    }
    // Sweep parity picks which side to try first, exactly as the diagonal
    // fall and the roll do, so a symmetric pile does not develop a bias.
    let (first, second) = if rightward { (1, -1) } else { (-1, 1) };
    for dir in [first, second] {
        // The neighbour must be living tissue: this is "go round the trunk",
        // never a general licence to jump a gap.
        if surface.get(x + dir, y).organism_id() == 0 {
            continue;
        }
        for step in 1..=ORGANISM_SLIP_REACH {
            let tx = x + dir * step;
            let here = surface.get(tx, y);
            if here.organism_id() != 0 {
                continue;
            }
            // First cell past the trunk. It is only a destination if the leaf
            // can fall from it; anything else and this direction is refused
            // rather than searched further, since a second trunk beyond the
            // first is not "round the trunk" any more.
            if here.material == material::EMPTY && surface.get(tx, y + 1).material == material::EMPTY && try_move(surface, x, y, tx, y) {
                return true;
            }
            break;
        }
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
            // Land where the cell comes to *rest*, not one row down.
            //
            // `find_lateral_descent` picks `tx` precisely because it is a
            // column the cell can fall down, so the destination is open by
            // construction: moving to `(tx, y + 1)` always produced a cell
            // with air above it (the open column) and air below it (the very
            // reason it was chosen). One such cell is a droplet. A whole
            // surface row doing it in the same frame is a *sheet* -- and
            // that is the reported "horizontal banding": a comb of detached
            // one-cell ledges along a spreading front, air above and below
            // each one.
            //
            // Continuing down to where it would settle costs at most
            // `LIQUID_SETTLE_DROP` extra rows and removes the artifact
            // outright, because the cell lands supported instead of hanging.
            // Measured on a falling, spreading body, as cells belonging to a
            // horizontal run of six or more such films: **277 -> 0**, and of
            // twelve or more, **188 -> 0**. Fully-enclosed holes 12 -> 0.
            // Levelling time 311 -> 291 frames, so it is slightly *faster*,
            // and the stress-scene worst frame is unchanged.
            let mut ty = y + 1;
            let limit = y + 1 + LIQUID_SETTLE_DROP;
            while ty < limit && surface.in_bounds(tx, ty + 1) && surface.is_empty(tx, ty + 1) {
                ty += 1;
            }
            if try_move(surface, x, y, tx, ty) {
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
    if src_fill < dst_fill + surface.materials().get(src.material).min_transfer {
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
/// Water held in a `Powder` cell, `0..=SOIL_SATURATED`.
///
/// **Read the sign carefully: `0` means dry here**, the opposite of
/// `liquid_fill` below, where `0` means full. `material::SOIL_SATURATED`'s
/// own doc has the reasoning; the practical consequence is that soil from
/// worldgen, the brush or any existing test starts dry with no call site
/// needing to know, whereas a liquid created the same way starts full.
///
/// No managed-cell subtlety to worry about: only `Powder` uses `aux` this
/// way, and a `Powder` is never a liquid body's container cell.
pub fn soil_moisture(cell: Cell) -> u16 {
    cell.aux().min(material::SOIL_SATURATED)
}

/// The fraction of *plant available water* present, `0.0..=1.0` — the band
/// between the permanent wilting point and field capacity, which is the
/// only water a root can actually take up.
///
/// Exactly zero at or below the wilting point, which is the whole point of
/// that breakpoint: drought becomes a terminal failure rather than a slow
/// one, and a root in dust gets nothing rather than a little.
///
/// Public for the same reason `liquid_fill` below is, and it is the same
/// hazard: `examples/divergence.rs` reports the wet/dry axis it was *given*
/// beside the one it can still measure at the end of the run, and a harness
/// re-deriving this arithmetic would be free to get the wilting-point
/// breakpoint wrong in exactly the direction that makes a washed-out axis
/// look intact.
pub fn plant_available_fraction(cell: Cell) -> f32 {
    let m = soil_moisture(cell) as f32;
    let wp = material::SOIL_WILTING_POINT as f32;
    let fc = material::SOIL_FIELD_CAPACITY as f32;
    ((m - wp) / (fc - wp)).clamp(0.0, 1.0)
}

/// How full a `Liquid` cell is, in `material::LIQUID_FULL` units.
///
/// **`aux == 0` on a `Liquid` means *full*, not empty** — the convention
/// `CLAUDE.md` lists first among the gotchas, because getting it backwards
/// manufactures water out of nothing. This is the one place that knows it,
/// which is why the harnesses read fill through here rather than off
/// `Cell::aux` (`examples/filmstrip.rs`'s ice census does, and would have
/// had the convention inverted if it had not).
///
/// Public (originally `pub(crate)` for the tests that sum fill volume)
/// because harnesses need per-cell fill too — `examples/ascii.rs`'s
/// river-cost drain accounts what it deletes — and an example re-deriving
/// this two-liner is how the convention gets written backwards somewhere
/// nobody reviews.
pub fn liquid_fill(cell: Cell) -> u16 {
    let aux = cell.aux();
    if aux == 0 {
        material::LIQUID_FULL
    } else {
        aux
    }
}

/// Total fill volume of one liquid material across the world — the census
/// helper, public so harnesses never re-derive it. **Volume, not cell
/// count**: a `Liquid` cell holds continuous fill and near-empty cells
/// fringe every artifact, so counting cells is the recorded metric trap
/// (`CLAUDE.md`, "Liquids: measure column *volume*"). And it goes through
/// `liquid_fill` above rather than reading `aux` directly, because
/// `aux == 0` on a liquid means FULL — the single easiest convention in the
/// engine to get backwards, and exactly the kind of place an
/// independently-written census in an example would get it backwards.
/// A full-world scan: for tests and harness printouts at coarse intervals,
/// never for a per-frame decision.
pub fn liquid_volume(world: &World, id: material::MaterialId) -> u64 {
    let Some(b) = world.bounds() else { return 0 };
    let mut total = 0u64;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let cell = world.get(x, y);
            if cell.material == id {
                total += liquid_fill(cell) as u64;
            }
        }
    }
    total
}

/// How much further than one row a `find_lateral_descent` move may continue
/// down, to land supported rather than hanging in mid-air.
///
/// Two, and the value genuinely matters less than it looks: 2, 4, 8 and 16
/// all take the whisker count to exactly zero, because the cell only has to
/// reach the local surface, not fall to the floor. Unbounded (`MAX_REACH`)
/// works too and costs a great deal -- the stress scene's worst frame went
/// from ~9.3 ms to 31 ms, since the scan runs on every one of well over a
/// million lateral descents. Two is the smallest value that does the whole
/// job, so it is the one that pays least for it.
/// How often a grain sinking through a liquid takes a diagonal step instead
/// of a straight one -- see `update_powder`.
const SINK_SPREAD: f32 = 0.5;

const LIQUID_SETTLE_DROP: i32 = 2;

/// Rises, then spreads — biased downwind. Gases are the mirror of liquids
/// under gravity.
///
/// The wind bias is the first thing in this engine that lets the M13 field
/// move *material*. Until it landed, the field's velocity channel had two
/// consumers — its own semi-Lagrangian advection of heat/light/moisture, and
/// `organism::wind_lean_dir`'s tree-tip lean — and neither of them displaces
/// a single cell. So an explosion's pressure impulse would propagate and
/// reflect across the whole world, sway some branches, and otherwise be
/// invisible. A blast now visibly blows its own smoke outward before the
/// plume drifts off, which is the shock becoming something you can see
/// rather than something the F-key overlay can confirm.
///
/// Implemented as a *bias on an existing choice*, not as a new movement
/// mode: `update_gas` already picks which horizontal direction to try first
/// with a fair coin, and this only weights that coin. A gas in still air
/// behaves exactly as it did (the coin stays fair below
/// `WIND_BIAS_THRESHOLD`), and even in strong wind the gas still rises
/// first — wind steers a plume, it does not stop it from being buoyant.
fn update_gas<S: CellSurface>(surface: &mut S, x: i32, y: i32, rightward: bool) -> bool {
    // Dissipation: the only rule anywhere in the engine that removes a gas
    // cell. Before this, nothing did — smoke rose, spread, found a ceiling
    // and stayed there for the rest of the session, which
    // `Reports/explosion-stone-review.md` §11 found while looking at
    // something else and parked as open item 8. A buried blast's crater kept
    // a permanent grey cap.
    //
    // Rolled *before* the movement attempts, deliberately, and that gets
    // every gas cell exactly one roll per frame: a cell that moves lands in
    // a row the bottom-to-top sweep has not reached yet, is flagged `moved`,
    // and `update_cell` skips it there — so rolling after a move would give
    // rising smoke no roll at all on the frames it is actually rising, and a
    // plume would only ever thin once it had stopped.
    //
    // **The roll is skipped entirely at 0.0** rather than left to
    // `Rng::chance`'s own `p <= 0.0` early-out, which costs the same either
    // way at runtime but means a gas that does not dissipate draws exactly
    // the numbers it drew before this landed. Smoke's own stream does shift,
    // because a draw was inserted ahead of `wind_biased_order`'s: a
    // deterministic *behaviour change* (same build, same seed, same result),
    // not a break in determinism.
    //
    // **One rate, not two, and that was a decision rather than an
    // omission.** The obvious refinement is a higher rate for gas that
    // could not move, so a ceiling pool clears faster than a plume thins,
    // and it is cheap to write — the blocked case is already a branch at
    // the bottom of this function. It is not taken, for two reasons that
    // point the same way. Trapped gas already has its own path
    // (`ActiveKind::Dissipate`, scheduled from that same branch), so the
    // second constant would be tuning one path against the other rather
    // than adding a behaviour. And it argues the wrong way physically: real
    // smoke sealed into a pocket lingers *longer* than smoke free to mix,
    // not shorter. What the complaint was actually about was permanence,
    // and permanence is gone at one rate — the sealed-crater guard's last
    // smoke cell goes at frame 430, about seven seconds, against never.
    let dissipation = surface.materials().get(surface.get(x, y).material).dissipation;
    if dissipation > 0.0 && surface.rng().chance(dissipation) {
        surface.set(x, y, Cell::EMPTY);
        return true;
    }

    let (first, second, lean) = wind_biased_order(surface, x, y);
    // Strong wind takes the downwind diagonal *before* rising straight up.
    //
    // This ordering is the whole mechanism, and getting it wrong made the
    // first version of this a complete no-op: with the straight-up move
    // tried first, a plume in open air always has an empty cell above it, so
    // it rose vertically and the horizontal preference below was never
    // consulted at all. The bias only ever applied to gas already trapped
    // under a ceiling — which is the one case where it looks like nothing.
    // Caught by rendering the same blast with the bias on and off and
    // getting two identical contact sheets; no test would have noticed,
    // because the rule genuinely "used" the wind on every call.
    if lean && try_move(surface, x, y, x + first, y - 1) {
        return true;
    }
    if try_move(surface, x, y, x, y - 1) {
        return true;
    }
    if try_move(surface, x, y, x + first, y - 1) || try_move(surface, x, y, x + second, y - 1) {
        return true;
    }

    // Capped for the same reason as `SURFACE_SEARCH`: a rule must not read
    // further than the sweep region is widened.
    let dispersion = (surface.materials().get(surface.get(x, y).material).dispersion as i32).min(MAX_REACH);
    let moved = flow_sideways(surface, x, y, first, dispersion, 1, rightward)
        || flow_sideways(surface, x, y, second, dispersion, 1, rightward);

    // Nowhere to go. Hand this cell to the active-site scheduler, because
    // the roll above is about to stop happening: a chunk whose gas has
    // stopped moving settles within a couple of dozen frames and is not
    // swept again. See `scheduler::ActiveKind::Dissipate` for the
    // measurement, and `evaporation.rs`'s module doc for the identical
    // lesson learned one material-kind over. The sweep's whole part is this
    // one line — schedule and forget.
    if !moved && dissipation > 0.0 {
        schedule_dissipation(surface, x, y);
    }
    moved
}

/// How often a *trapped* gas cell is rolled once the sweep has stopped
/// visiting it. One second.
///
/// It is a sampling interval, not a second rate: `dissipation_tick` rolls
/// the compounded probability of the material's own per-tick chance over
/// this many ticks, so lengthening or shortening it changes how granular the
/// clearing looks and not how fast it happens. Kept a Rust constant rather
/// than graduating to `.ron` on `design-philosophy.md` §2a's own test — a
/// non-programmer tuning "how fast does smoke go" reaches for
/// `dissipation`, and reaching for this one instead would get them nothing.
const DISSIPATION_CHECK_INTERVAL: u64 = 60;

fn schedule_dissipation<S: CellSurface>(surface: &mut S, x: i32, y: i32) {
    surface.schedule_active_site(ActiveSite {
        x,
        y,
        kind: ActiveKind::Dissipate,
        next_frame: surface.frame() + DISSIPATION_CHECK_INTERVAL,
    });
}

/// Dispatch a due `ActiveKind::Dissipate` site. `scheduler::step` never
/// routes any other `ActiveKind` here.
///
/// **Why the probability is compounded rather than reused.** The sweep rolls
/// `dissipation` once per tick; this runs once per
/// `DISSIPATION_CHECK_INTERVAL` ticks, so rolling the same number here would
/// make trapped smoke sixty times longer-lived than drifting smoke — the
/// exact backwards-of-intended result `evaporation.rs`'s reverted first
/// version produced when a lake outlasted a puddle. `1 - (1 - p)^n` is the
/// chance of *at least one* success in the n ticks the site stood in for, so
/// a cell that sits still and one that keeps moving have the same half-life
/// however the world happens to schedule them.
///
/// Deliberately does **not** ask whether the cell could move again by now.
/// It looks like a free retirement — if it can rise, the sweep owns it — and
/// it is the kind of gate that turns a mechanic off: a cell whose escape
/// opened while its chunk was asleep would be handed back to a sweep that is
/// not running. Rolling it anyway costs one draw a second and cannot strand
/// anything. The overlap the other way (a cell that has a pending site
/// *and* is being swept) is real and bounded: it doubles the rate for at
/// most the ~19 frames a chunk stays awake after its gas settles, against a
/// number picked by eye in the first place.
pub fn dissipation_tick(world: &mut World, site: &ActiveSite) -> Vec<ActiveSite> {
    let (x, y) = (site.x, site.y);
    let material = world.materials.get(world.get(x, y).material);
    // Structurally finished: the gas moved on, burned, or was dug out, and
    // something else is here now. Nothing to reschedule — if gas comes back,
    // the sweep is awake for it and schedules afresh.
    if material.kind != MaterialKind::Gas || material.dissipation <= 0.0 {
        return Vec::new();
    }

    let per_check = 1.0 - (1.0 - material.dissipation).powi(DISSIPATION_CHECK_INTERVAL as i32);
    if world.rng.chance(per_check) {
        world.set(x, y, Cell::EMPTY);
        // Nothing rescheduled here even though the neighbours are now free
        // to move: this write dirties the chunk, which wakes it, which is
        // the sweep picking them up again on its own.
        return Vec::new();
    }
    vec![ActiveSite { x, y, kind: ActiveKind::Dissipate, next_frame: world.frame + DISSIPATION_CHECK_INTERVAL }]
}

/// A prevailing breeze, added to whatever the field reports.
///
/// **Applied here rather than in the field, and that placement is the whole
/// point.** The obvious implementation is a real forcing term in
/// `field::step` — a small `vx` nudge on every unblocked cell, so heat
/// advection and `organism::wind_lean_dir` feel it too. That was built,
/// measured, and reverted: it destroys the field-sleep optimization (issue
/// #4). A uniform velocity in a bounded world hits walls, which creates
/// divergence, which creates pressure, which creates more velocity — so
/// `is_converged` never returns true and the solve runs forever. Measured on
/// `examples/ascii.rs`'s own sleep scene, the settled-field frame cost went
/// from **0.0002 ms to 3.55 ms**, permanently, on every scene; six field
/// tests failed alongside it. `CLAUDE.md` is explicit that frame cost is a
/// hard constraint and not a tiebreaker, and 3.55 ms a frame forever is not
/// a price worth paying for smoke that leans.
///
/// So this is deliberately the cheaper, lesser thing: a constant the *gas
/// rule alone* reads, costing nothing and changing nothing else. It is
/// honestly an approximation rather than physics — this "wind" does not
/// carry heat, does not sway trees, and does not appear in the F-key
/// velocity overlay, all of which a real field-level breeze would. What it
/// does buy is the one thing the coupling needed to be visible outside the
/// two seconds after an explosion: smoke that drifts somewhere instead of
/// rising straight up. Set to 0.0 to remove it entirely.
///
/// If a real field-level wind is ever wanted, the actual prerequisite is
/// making the solver settle *with* a steady forcing term present — not
/// re-attempting the same nudge and re-discovering the same 3.55 ms.
///
/// The magnitude is set against `WIND_BIAS_FULL_SPEED`'s own measured scale,
/// not chosen in isolation: at 0.35 (the first value tried) this produced a
/// downwind preference of 0.53 against a fair 0.5 and was invisible. At 2.0
/// it is a clear but not overwhelming lean, and it stays far below the ~86
/// an explosion's own transient peaks at — so a blast still dominates its
/// own neighbourhood completely and the breeze only takes over once the
/// shock has passed, which is the ordering that reads correctly.
const PREVAILING_DRIFT: f32 = 2.0;

/// Below this wind speed a gas picks its horizontal direction with a fair
/// coin, exactly as it always did. Not zero: the field's velocity channel
/// approaches its fixed point asymptotically and settles to *near* rather
/// than exactly zero (`field::SETTLE_EPSILON_VELOCITY` exists for the same
/// reason), so without a threshold a long-settled world would still show
/// every plume leaning one way forever on numerical residue.
const WIND_BIAS_THRESHOLD: f32 = 0.01;

/// Strongest the wind bias can get, as a probability of choosing downwind.
/// Capped below 1.0 deliberately: at exactly 1.0 every gas cell in a windy
/// region picks the identical direction on the identical frame, and a plume
/// stops reading as a diffuse cloud and starts reading as a solid block
/// sliding sideways — the same failure `explosion::Tuning::debris_jitter`
/// and `particle::Particle::drag` were both added to prevent, for the same
/// underlying reason (a shared decision made identically by many cells at
/// once).
const MAX_WIND_BIAS: f32 = 0.85;

/// Wind speed at which the bias reaches `MAX_WIND_BIAS`.
///
/// **Measured, not guessed** — the first value here was 0.35, picked by eye
/// with no idea of the channel's actual scale, which saturated the bias
/// permanently and told me nothing. Probing `field_at(..).vx` around a real
/// blast: the peak reaches ~86 about eight frames in, falls through ~20 by
/// frame 24, sits between 1 and 3 for another forty frames, and is under 1
/// by frame 80. A settled world reads essentially zero (the solver's own
/// `SETTLE_EPSILON_VELOCITY` is 0.001). This value puts the *sustained*
/// post-blast wind partway up the ramp and the initial shock firmly at the
/// top, so the lean visibly eases off as the shock passes rather than
/// switching between two states.
const WIND_BIAS_FULL_SPEED: f32 = 4.0;

/// How much of the bias also decides whether the gas leans downwind *before*
/// rising, rather than only which side it prefers once it cannot rise. See
/// `update_gas`'s own comment — without this the whole mechanism is a no-op
/// in open air. Below 1.0 so even a howling wind lets some of a plume rise
/// vertically, which is what keeps it looking like a cloud being pushed
/// rather than a solid diagonal streak.
const MAX_LEAN_CHANCE: f32 = 0.7;

/// Which horizontal direction a gas tries first, weighted downwind, and
/// whether the wind is strong enough for it to lean that way *before* rising.
///
/// Returns the same `(first, second)` pair `update_gas` always used, so that
/// part of the caller is unchanged apart from where the pair comes from. In
/// still air this is exactly a fair coin and `lean` is always false, so a
/// gas behaves precisely as it did before any of this existed.
fn wind_biased_order<S: CellSurface>(surface: &mut S, x: i32, y: i32) -> (i32, i32, bool) {
    let (field_vx, _) = surface.field_wind_at(x, y);
    let vx = field_vx + PREVAILING_DRIFT;
    if vx.abs() <= WIND_BIAS_THRESHOLD {
        let (a, b) = if surface.rng().flip() { (-1, 1) } else { (1, -1) };
        return (a, b, false);
    }
    let downwind = if vx > 0.0 { 1 } else { -1 };
    // 0.5 (no preference) at the threshold, rising to `MAX_WIND_BIAS` at
    // `WIND_BIAS_FULL_SPEED`.
    let strength = (vx.abs() / WIND_BIAS_FULL_SPEED).clamp(0.0, 1.0);
    let downwind_chance = 0.5 + (MAX_WIND_BIAS - 0.5) * strength;
    let (first, second) = if surface.rng().chance(downwind_chance) {
        (downwind, -downwind)
    } else {
        (-downwind, downwind)
    };
    // Only lean ahead of rising when the preferred side actually *is*
    // downwind -- otherwise a cell that just rolled against the bias would
    // be pushed upwind ahead of its own buoyancy, which is the opposite of
    // the intent.
    let lean = first == downwind && surface.rng().chance(strength * MAX_LEAN_CHANCE);
    (first, second, lean)
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
        // `path.managed()` explicitly, *not* left to `is_empty()`.
        //
        // `is_empty()` is managed-aware, so it does block a body's
        // materially-empty container cells. But a promoted body's own liquid
        // cells are not materially empty and hold the same material as the
        // scanning cell, so `path.material != src.material` was false for
        // them and the scan walked straight across a lake's surface -- past
        // its far edge, and out the other side. That is exactly the
        // "skating along a managed surface hunting for an edge to fall off"
        // that `update_liquid`'s absorption guard exists to prevent, and
        // that guard only covers a body directly *below* the scanning cell.
        // Found by review; the comment here previously claimed the opposite
        // of what the condition did.
        if !path.is_empty() && (path.managed() || path.material != src.material) {
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
        // A splash *candidate*, reported before the swap while both cells
        // are still readable, and acted on by nobody here — see
        // `CellSurface::report_splash`. Nothing below writes a cell, so
        // `Reports/open-bugs-handoff.md` §2's displacement striping is
        // untouched: this is a read and a push.
        //
        // The conditions, cheapest first, because this sits in the hottest
        // path in the engine and a sand blob entering a pool runs it
        // thousands of times a frame:
        //
        // - **Downward only.** A splash is something falling *into* water.
        // - **The displaced cell full.** A particle lands as a *whole* cell
        //   (`particle::land`), so throwing a fringe cell holding 40 units
        //   of fill and getting a full one back is water out of nothing —
        //   the same failure `fire::transform`'s aux table just had fixed.
        //   Free: `dst` is already in hand.
        //
        // **What is deliberately *not* checked here: whether the site has
        // air above it.** That is the condition that makes it a free
        // surface rather than the middle of a pool, and `throw_splashes`
        // does test it -- but it costs a `World::get` at a position this
        // function does not already hold, and measured on ascii's
        // `stress: a full screen of sand and water (serial)` -- which is
        // this branch's worst case by construction -- paying it here took
        // the minimum worst frame over three runs from **66.8 ms to
        // 72.7 ms**. Every site the sweep reports is re-checked a frame
        // later anyway (a site can drain, freeze or be buried in between),
        // so doing it once, there, in a loop bounded by
        // `MAX_SPLASH_SITES`, costs nothing in the hottest path in the
        // engine and rejects exactly the same sites.
        if ty > y && dst_kind == MaterialKind::Liquid && liquid_fill(dst) >= SPLASH_MIN_FILL && surface.rng().chance(SPLASH_CHANCE) {
            surface.report_splash(x, y, 1.0);
        }
        surface.move_cell(x, y, tx, ty, revisited);
        return true;
    }
    false
}

/// Chance that one displacement of full liquid by something denser is
/// recorded as a splash candidate.
///
/// The knob that keeps this from reading as spray: a sand blob entering a
/// pool displaces thousands of cells a frame, and the geometric conditions
/// alone would still leave dozens a frame. What should read as a splash is
/// a handful of droplets thrown clear, not a sheet of water leaving the
/// pool. Measured on `filmstrip scene=splash` over 660 frames: 80 droplets
/// at this value, 499 with the roll removed entirely.
const SPLASH_CHANCE: f32 = 0.25;

/// How full a displaced liquid cell must be before a droplet may be taken
/// out of it — see the call site.
///
/// **Full, and nothing less, and that is measured.** `particle::land`
/// writes a *whole* cell wherever a droplet comes down, so taking a cell
/// holding 900 and giving back 1,000 makes 0.1 of a cell out of nothing —
/// exactly the shape of the melt bug `fire::transform` had. It is small per
/// droplet and it accumulates: at `freeze_min_fill`'s 900 the `scene=splash`
/// run measured **30,351.3 cell-equivalents at the start and 30,363.3 at
/// the end** over 499 droplets. At `LIQUID_FULL` the same run closes.
const SPLASH_MIN_FILL: u16 = material::LIQUID_FULL;

#[cfg(test)]
mod tests {

    /// **A tunnel with a lining keeps its roof; the same tunnel without one
    /// does not.** Both halves are asserted in one test on purpose.
    ///
    /// A test that only checked the lined case would be green with
    /// `self_supporting` deleted from `update_powder` in every world where
    /// the geometry happened not to collapse, and `CLAUDE.md`'s standing
    /// rule is that green must be evidence about the code rather than about
    /// the test. Asserting the unlined arm collapses is that guard put in
    /// where it cannot be skipped: the two arms are the same block, the same
    /// carve and the same frame budget, and they differ in the material of
    /// the shell. Deleting the early return makes the *first* assertion fail.
    ///
    /// The serial driver, deliberately -- this is a claim about the movement
    /// rules, and `step` is the one that isolates them.
    /// `examples/burrow_probe.rs` runs the same excavation through
    /// `frame::step` (and so through `parallel::step`, which is what the app
    /// calls), which is where the whole-engine version of this claim lives.
    #[test]
    fn a_lined_gallery_keeps_its_roof_and_an_unlined_one_does_not() {
        use super::super::chunk::Rect;
        use super::super::world::World;

        // One bed, one carve, run twice -- the only difference is whether the
        // shell around the void is worked. Returns cells still materially
        // empty; **raw material equality, not `is_empty`**, which is
        // managed-aware and answers a different question.
        fn standing(lined: bool) -> usize {
            let mut w = World::new(Rect::new(0, 0, 63, 63));
            let soil = w.materials.id_of("soil").expect("soil is compiled in");
            let packed = w.materials.id_of("packedsoil").expect("packedsoil is compiled in");
            for x in 4..60 {
                for y in 20..56 {
                    w.set(x, y, Cell::new(soil, 0));
                }
            }
            let gallery: Vec<(i32, i32)> =
                (12..52).flat_map(|x| (36..39).map(move |y| (x, y))).collect();
            for &(x, y) in &gallery {
                w.set(x, y, Cell::EMPTY);
            }
            if lined {
                let void: std::collections::HashSet<(i32, i32)> = gallery.iter().copied().collect();
                for &(x, y) in &gallery {
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let n = (x + dx, y + dy);
                            if !void.contains(&n) && w.get(n.0, n.1).material == soil {
                                w.set(n.0, n.1, Cell::new(packed, 0));
                            }
                        }
                    }
                }
            }
            for _ in 0..200 {
                step(&mut w);
            }
            gallery.iter().filter(|&&(x, y)| w.get(x, y).material == material::EMPTY).count()
        }

        let total = 40 * 3;
        let lined = standing(true);
        let bare = standing(false);
        assert_eq!(lined, total, "a lined gallery must not close: {lined}/{total} cells left open");
        assert!(
            bare * 4 < total,
            "the unlined control must collapse, or the lined arm proves nothing about the lining: \
             {bare}/{total} cells left open"
        );
    }

    /// **Waterlogged lining reverts to loose soil**, which is the whole of
    /// what keeps a burrow from being immortal.
    ///
    /// Written against the cell's own held water rather than the coarse
    /// moisture field, because that is what the rule reads: a block-nearest
    /// field sample cannot see one cell (`CLAUDE.md`'s coarse-field gotcha).
    /// The dry cell in the same block is the specificity half -- without it
    /// this passes just as well for a rule that un-packs everything.
    #[test]
    fn a_lining_above_field_capacity_slumps_back_to_soil() {
        use super::super::chunk::Rect;
        use super::super::world::World;

        let mut w = World::new(Rect::new(0, 0, 31, 31));
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        let packed = w.materials.id_of("packedsoil").expect("packedsoil is compiled in");
        for x in 0..32 {
            w.set(x, 30, Cell::new(soil, 0));
            w.set(x, 31, Cell::new(soil, 0));
        }
        // Two lining cells side by side on the floor, one saturated and one
        // dry, with nowhere to fall so the material read is unambiguous.
        w.set(10, 29, Cell::new(packed, 0).with_aux(material::SOIL_SATURATED));
        w.set(20, 29, Cell::new(packed, 0));

        step(&mut w);

        assert_eq!(
            w.get(10, 29).material,
            soil,
            "a lining above SOIL_FIELD_CAPACITY should have slumped back to loose soil"
        );
        assert_eq!(
            w.get(20, 29).material,
            packed,
            "a dry lining must stay packed -- otherwise the rule is 'un-pack everything'"
        );
        assert!(
            w.get(10, 29).aux() > material::SOIL_FIELD_CAPACITY,
            "the slumped cell must carry its water across; rebuilding it would read as dry ground"
        );
    }

    /// Capillary flow must respect the *receiver's* capacity, not the
    /// sender's."""
    ///
    /// Needs two water-holding powders with different capacities to be
    /// observable at all: with equal capacities the drier cell is by
    /// definition below its own limit, so the clamp can never bind and the
    /// bug is invisible. `soil` is the only such material shipped, so this
    /// writes a second one (`tightsoil`, a third of soil's capacity) into a
    /// temp directory and loads it additively -- the same trick
    /// `examples/debug_tree_variants.rs` uses for species.
    #[test]
    fn capillary_flow_never_pushes_a_neighbour_past_its_own_capacity() {
        use super::super::chunk::Rect;
        use super::super::world::World;

        let dir = std::env::temp_dir().join("pixel_physics_capillary_capacity_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("tightsoil.ron"),
            format!(
                r#"(
    name: "tightsoil",
    display: "Tight soil",
    kind: Powder,
    density: 1.5,
    friction_angle: 40.0,
    colors: [(90, 70, 50)],
    water_capacity: {},
    penetration_resistance: 0.8,
)
"#,
                material::SOIL_SATURATED / 3
            ),
        )
        .unwrap();

        let mut w = World::new(Rect::new(0, 0, 31, 31));
        w.materials.reload(&dir).expect("tightsoil should parse");
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        let tight = w.materials.id_of("tightsoil").expect("just loaded");
        let tight_capacity = w.materials.get(tight).water_capacity;

        // A saturated ordinary soil cell beside a nearly-full tight one, so
        // the exchange runs from soil into a neighbour with much less room.
        w.set(10, 10, Cell::new(soil, 0).with_aux(material::SOIL_SATURATED));
        w.set(11, 10, Cell::new(tight, 0).with_aux(tight_capacity - 1));

        for _ in 0..200 {
            update_soil_water(&mut w, 10, 10);
            update_soil_water(&mut w, 11, 10);
        }

        let held = soil_moisture(w.get(11, 10));
        assert!(
            held <= tight_capacity,
            "a neighbour must never be filled past its own water_capacity: {held} > {tight_capacity}"
        );
    }

    use super::*;
    use crate::sim::parallel;

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

    /// **A leaf does not sit on the branch it just let go of.**
    ///
    /// `plant::shed_to_litter` already walked a shed leaf down through its
    /// own crown, so this is not about the abscission event -- it is about
    /// every other way a litter cell reaches mid-canopy (landing on a
    /// raindrop's cell, on other debris, on a branch that grew under it).
    /// Before `Material::falls_through_organisms` those cells stopped on the
    /// first branch for ever, and `shed_to_litter`'s own walk then stacked
    /// every subsequent leaf in that column on top of them: `litter_probe
    /// frames=12000 trees=8` read 57.6% of standing litter resting on plant
    /// tissue, with 23% of it more than 32 rows above the ground; after,
    /// 44.1% and none at all above 32 rows.
    ///
    /// **Three arms rather than one assertion, because two of them are the
    /// controls `CLAUDE.md` asks for** and neither is free to add later.
    /// All three were confirmed to go red for their own fault before this
    /// was committed:
    ///
    /// - `sand`, in the identical scene, must *not* pass through -- the
    ///   specificity control. Deleting the material test makes it land on
    ///   the floor at row 126 instead of resting at 59.
    /// - litter over a branch with *packed ground* under it must not move.
    ///   That is the drift piled against a trunk that `litter.ron`'s
    ///   42-degree friction angle exists to build, and `litter_probe`'s
    ///   module doc records that reading it as the failure case cost a whole
    ///   detour. Deleting the "stop at the first non-organism cell" clause
    ///   sends it through the collar and the ground both, to row 126.
    /// - and the positive arm itself: turning the flag off in `litter.ron`
    ///   leaves the leaf sitting at row 59, on the branch.
    ///
    /// The branch is three cells wide and three thick, and its cells carry a
    /// real `organism_id`, because that -- not `MaterialKind::Plant` -- is
    /// what the rule tests, for the same reason `shed_to_litter` gives: a
    /// leaf should fall past an ant standing under the tree too.
    #[test]
    fn litter_falls_through_a_branch_but_sand_and_a_trunk_side_drift_do_not() {
        let mut w = world_with_floor();
        let litter = w.materials.id_of("litter").expect("litter is compiled in");
        // `wood`, not `material::WOOD` -- wood was appended to `EMBEDDED` and
        // has no `material::` constant, exactly as `shed_to_litter` says of
        // litter itself.
        let wood = w.materials.id_of("wood").expect("wood is compiled in");
        // **Three cells wide, and that is load-bearing rather than
        // decorative.** On a one-cell branch both diagonal moves are into
        // clear air, so *every* powder slides off within a frame and all
        // three arms pass whatever this rule does -- the scene would not
        // contain the situation it claims to measure, which `CLAUDE.md`
        // names as the way four conclusions in this project went wrong.
        let branch = |w: &mut World, x: i32, y0: i32, y1: i32, id: u16| {
            for y in y0..=y1 {
                for dx in -1..=1 {
                    w.set(x + dx, y, Cell::new(wood, 0).with_organism_id(id));
                }
            }
        };

        // Arm 1: litter wedged on a branch with clear air all the way down.
        branch(&mut w, 20, 60, 62, 1);
        w.set(20, 59, Cell::new(litter, 0));
        // Arm 2: sand in the identical situation -- the specificity control.
        branch(&mut w, 40, 60, 62, 2);
        w.set(40, 59, Cell::new(material::SAND, 0));
        // Arm 3: a collar of trunk sitting on ground, litter resting on the
        // collar. It must stay: the scan stops at the first cell that is not
        // organism-owned, so a leaf never eats its way through the floor.
        // This is the drift piled against a trunk that `litter.ron`'s
        // 42-degree friction angle exists to build, and `litter_probe`'s
        // module doc records that reading it as the failure case cost a
        // whole detour.
        //
        // **The ground is a two-row ledge with clear air beneath it, and
        // that is what makes this arm able to fail at all.** Written first
        // as a solid 67-row column, it stayed green for the fault it is
        // named for -- the scan simply ran out of its 16-cell budget inside
        // the stone and returned "no" for the wrong reason, so deleting the
        // stop-at-non-organism clause changed nothing. Two rows leave real
        // air inside the budget, so a version that tunnelled through
        // anything would put this cell on the floor at row 126.
        //
        // **Stone, not soil**: a tall soil column is a *powder* and slumps
        // to its own angle of repose within a few hundred frames, leaving
        // air under the collar and quietly turning this arm into a second
        // copy of arm 1 -- which is how an earlier draft failed.
        //
        // Eleven wide rather than three, for the same reason arm 1's branch
        // is three: on a narrow plinth `roll_along_slope` walks the grain to
        // the edge and it falls off the *side*, which is ordinary repose and
        // would fail this arm for a reason that has nothing to do with
        // tunnelling. Litter's 42-degree friction angle puts its roll reach
        // at one to two cells, so five either side is well clear.
        for y in 60..=61 {
            for dx in -5..=5 {
                w.set(60 + dx, y, Cell::new(material::STONE, 0));
            }
        }
        for y in 57..=59 {
            for dx in -5..=5 {
                w.set(60 + dx, y, Cell::new(wood, 0).with_organism_id(3));
            }
        }
        w.set(60, 56, Cell::new(litter, 0));

        // Long enough for a 67-row fall at one cell a frame, with room to
        // spare; every assertion is on where something came to *rest*, so a
        // budget that is merely generous cannot make an arm pass.
        run(&mut w, 300);

        // Searched over a band of columns, not down one: a cell that reaches
        // the floor rolls off at its own angle of repose, so pinning the
        // column would make this a test of where it rolled to. The bands are
        // wide apart, so no arm can see another's cell.
        let find = |w: &World, m: material::MaterialId, x0: i32, x1: i32| {
            (0..128).flat_map(move |y| (x0..=x1).map(move |x| (x, y))).find(|&(x, y)| w.get(x, y).material == m)
        };

        let (_, landed) = find(&w, litter, 0, 30).expect("the litter cell vanished entirely");
        assert!(
            landed >= 126,
            "litter must fall past a branch to the floor: it stopped at row {landed}, with the branch at rows 60-62"
        );
        // The branch itself is untouched: tunnelling only ever moves into
        // air, so it can no more overwrite a plant cell than `shed_to_litter`
        // can.
        for y in 60..=62 {
            assert_eq!(w.get(20, y).material, wood, "the branch at row {y} was displaced by the leaf falling past it");
        }

        let (_, sand) = find(&w, material::SAND, 31, 50).expect("the sand cell vanished entirely");
        assert_eq!(
            sand, 59,
            "sand must still rest on a branch -- this rule is a material opt-in, not a change to how every powder falls"
        );

        let drift = find(&w, litter, 51, 80).expect("the drift cell vanished entirely");
        assert_eq!(
            drift,
            (60, 56),
            "a drift resting on a trunk must not eat through the ground under it: the scan stops at the first cell that is not organism-owned"
        );
    }

    /// **A drift of leaves spills round a trunk instead of climbing it.**
    ///
    /// The horizontal half of the same rule, and the half that was still on
    /// screen after `fall_through_organism` landed. Leaves had stopped
    /// resting on branches -- 4 of 497 standing cells with any air under them
    /// -- and the owner still reported *"it didn't look like the leaves were
    /// all on the floor"*, correctly: a pile wedged against a trunk cannot
    /// roll sideways into wood, so it climbed out of the floor into the lower
    /// crown as a narrow column. Grounded the whole way, so every measure
    /// that asks *what is under this cell* called it floor. Measured on
    /// `litter_probe frames=12000 trees=8`: **24% of standing litter more
    /// than eight rows up with 68% of that boxed in by plant tissue, against
    /// 4.9% and a highest cell of 10 rows after.**
    ///
    /// **The third arm is the one that matters and is the one that would go
    /// blind.** Requiring only an empty cell on the far side of the trunk
    /// passes the first two arms identically and lets a leaf shuffle left and
    /// right across a trunk for ever, keeping its chunk awake -- physics that
    /// costs nothing and frame time that costs everything. Requiring open air
    /// *beneath* the destination is what makes each slip a descent. So arm 3
    /// puts level ground on both sides and asserts nothing moves; it is red
    /// for a version that drops that clause, and all three were confirmed red
    /// for their own fault before this was committed.
    ///
    /// **What this guard does *not* cover, stated because a green run of it
    /// will otherwise be read as covering everything.** Deleting the
    /// "neighbour must be living tissue" clause leaves all three arms green,
    /// because every arm here has a trunk in that position. That clause is
    /// still load-bearing and is not decoration: without it the rule becomes
    /// "a settled grain drops into any adjacent hole", which bypasses
    /// `roll_along_slope`'s two-angle repose hysteresis whenever
    /// `stability_reach_at` returns 0 -- the settled cell never gets asked to
    /// roll, so this would answer in its place. A scene that discriminates it
    /// needs a settled litter cell beside an open drop with no plant tissue
    /// anywhere, and it is not built here.
    #[test]
    fn a_leaf_drift_slips_round_a_trunk_only_when_there_is_somewhere_lower_to_go() {
        let mut w = world_with_floor();
        let litter = w.materials.id_of("litter").expect("litter is compiled in");
        let wood = w.materials.id_of("wood").expect("wood is compiled in");

        // A plateau with a trunk on its edge. The leaf sits on the plateau
        // top, wedged against the trunk: it cannot fall (stone below), cannot
        // go diagonally (stone one side, trunk the other), cannot tunnel (the
        // cell below is not organism-owned) and cannot roll (the trunk blocks
        // the only downhill path). Every earlier rule has already declined by
        // the time this one is asked, which is the situation it exists for.
        let plateau = |w: &mut World, x0: i32, x1: i32| {
            for x in x0..=x1 {
                for y in 120..=126 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
        };
        let trunk = |w: &mut World, x: i32, y0: i32, id: u16| {
            for y in y0..=126 {
                w.set(x, y, Cell::new(wood, 0).with_organism_id(id));
            }
        };

        // Arm 1: open ground beyond the trunk, seven rows lower. Must slip.
        plateau(&mut w, 10, 19);
        trunk(&mut w, 20, 114, 1);
        w.set(19, 119, Cell::new(litter, 0));

        // Arm 2: sand in the identical geometry -- the specificity control.
        plateau(&mut w, 70, 79);
        trunk(&mut w, 80, 114, 2);
        w.set(79, 119, Cell::new(material::SAND, 0));

        // Arm 3: the plateau continues past the trunk, so there is nothing
        // lower on the far side. Must not move -- see the doc above.
        plateau(&mut w, 40, 60);
        trunk(&mut w, 50, 114, 3);
        w.set(49, 119, Cell::new(litter, 0));

        run(&mut w, 2000);

        // **The world must have gone to sleep, and this is arm 3's real
        // assertion.** Its position check alone is blind: dropping the
        // "somewhere lower" clause makes the leaf shuffle across the trunk
        // with period two, so after any even number of frames it is back on
        // the square it started from and the coordinates match perfectly.
        // Caught by putting that exact fault back and watching the test stay
        // green -- which is why this line exists and why the budget above is
        // generous rather than tight. An oscillating cell writes every frame,
        // so its chunk can never settle, and that is the cost the clause is
        // there to avoid rather than a physics detail.
        assert_eq!(
            w.active_chunk_count(),
            0,
            "the world never settled: something is still writing every frame, which is what a leaf shuffling back and \
             forth across a trunk looks like"
        );

        let find = |w: &World, m: material::MaterialId, x0: i32, x1: i32| {
            (0..128).flat_map(move |y| (x0..=x1).map(move |x| (x, y))).find(|&(x, y)| w.get(x, y).material == m)
        };

        let (slipped_x, slipped_y) = find(&w, litter, 0, 35).expect("the leaf vanished entirely");
        assert!(
            slipped_x > 20 && slipped_y >= 126,
            "a leaf wedged against a trunk must spill round it to the lower ground: it is at ({slipped_x}, {slipped_y}), \
             with the trunk at x=20 and the floor at row 126"
        );
        // The trunk is untouched: a slip only ever moves into empty space, so
        // this can no more displace wood than the vertical tunnel can.
        for y in 114..=126 {
            assert_eq!(w.get(20, y).material, wood, "the trunk at row {y} was displaced by the leaf going round it");
        }

        let (sand_x, sand_y) = find(&w, material::SAND, 65, 95).expect("the sand vanished entirely");
        assert_eq!(
            (sand_x, sand_y),
            (79, 119),
            "sand must stay wedged against the trunk -- this rule is a material opt-in, not a change to how every powder piles"
        );

        let (level_x, level_y) = find(&w, litter, 36, 64).expect("the level-ground leaf vanished entirely");
        assert_eq!(
            (level_x, level_y),
            (49, 119),
            "with level ground on the far side there is nowhere lower to go, so the leaf must not slip: without that clause \
             it shuffles across the trunk for ever and keeps the chunk awake"
        );
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

    /// A liquid must not scan *through* a promoted body's cells.
    ///
    /// `find_lateral_descent` walks sideways looking for a column to fall
    /// from, stopping at the first cell that is neither open space nor more
    /// of its own liquid. A body's cells are made of that same liquid, so
    /// they passed that test and the scan flowed straight through them --
    /// the "skating along a managed surface hunting for an edge to fall off"
    /// that `update_liquid`'s absorption guard exists to stop, and that
    /// guard only covers a body directly *below* the scanning cell.
    ///
    /// Uses `set_owned` to mark a short managed strip rather than promoting
    /// a real body, deliberately: `MIN_BODY_COLUMNS` is 32 and
    /// `LIQUID_LATERAL_REACH` is 24, so a scan can never reach across a
    /// genuine body and the predicate would go untested. `liquid.rs`'s own
    /// tests use `set_owned` as tooling the same way.
    ///
    /// Latent rather than live today, since nothing in production promotes a
    /// body (`127e177`) -- which is why review found it and play did not.
    #[test]
    fn a_liquid_does_not_scan_through_managed_cells_of_its_own_material() {
        let floor_y = 40;
        let mut w = World::new(Rect::new(0, 0, 79, 63));
        for x in 0..80 {
            w.set(x, floor_y, Cell::new(material::STONE, 0));
        }
        // Scanning cell, then three managed cells of the same material, then
        // a column it would dearly like to fall down.
        w.set(10, floor_y - 1, Cell::new(material::WATER, 0));
        for x in 11..14 {
            w.set(x, floor_y - 1, Cell::new(material::WATER, 0));
            let owned = w.get(x, floor_y - 1).with_managed(true);
            w.set_owned(x, floor_y - 1, owned);
        }
        w.set(15, floor_y, Cell::EMPTY);
        assert!(w.get(12, floor_y - 1).managed(), "test setup: the strip should be managed");

        assert_eq!(
            find_lateral_descent(&w, 10, floor_y - 1, 1),
            None,
            "the scan passed through managed cells to reach the drop beyond them"
        );

        // Control: with the strip unmanaged, the very same scan does find it,
        // so the assertion above is about `managed` and not about geometry.
        for x in 11..14 {
            let plain = w.get(x, floor_y - 1).with_managed(false);
            w.set_owned(x, floor_y - 1, plain);
        }
        assert_eq!(find_lateral_descent(&w, 10, floor_y - 1, 1), Some(15));
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
        // Was a `const` assert against the old hardcoded constant. Now a
        // runtime one against water's own value, since the dead band moved
        // onto `Material` to become live-tunable -- the bound is the same,
        // it is just no longer knowable at compile time.
        let w0 = World::new(Rect::new(0, 0, 1, 1));
        assert!(
            w0.materials.get(material::WATER).min_transfer <= 16,
            "B-9: water's min_transfer must be <= 16 for this budget"
        );

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

    /// The highest the smoke ever got, at any point in the run.
    ///
    /// Sampled every frame rather than read off the end state, and that is
    /// not fussiness: smoke is *mortal* now (`Material::dissipation`), so a
    /// single cell inspected after a fixed number of frames is a coin toss
    /// on the half-life rather than a statement about buoyancy. Both
    /// rise tests below used to do exactly that, and `smoke_rises` failed
    /// on the frame dissipation landed — the cell had reached y = 0 and
    /// then dissipated, which is the rule working, reported as the rule
    /// under test being broken. A peak cannot be answered by the removal
    /// rule at all, which is what makes it the right quantity here
    /// (`CLAUDE.md`: assert the property, not an instant fitted to one
    /// trajectory).
    fn highest_smoke_reached(w: &mut World, frames: usize) -> i32 {
        let mut peak = 128;
        for _ in 0..frames {
            step(w);
            if let Some(y) = (0..128).find(|&y| (0..128).any(|x| w.get(x, y).material == material::SMOKE)) {
                peak = peak.min(y);
            }
        }
        peak
    }

    /// A blob rather than the single cell this used to place, for the same
    /// reason `highest_smoke_reached` samples every frame: one mortal cell
    /// asked to survive a hundred frames of climbing is a coin toss on the
    /// half-life. Measured — the lone cell dissipated at y = 71, two thirds
    /// of the way up, and reported "smoke did not rise". Twenty-five cells
    /// make the topmost survivor a near-certainty without changing what
    /// buoyancy has to do.
    #[test]
    fn smoke_rises() {
        let mut w = world_with_floor();
        for y in 116..121 {
            for x in 62..67 {
                w.set(x, y, Cell::new(material::SMOKE, 0));
            }
        }
        let peak = highest_smoke_reached(&mut w, 200);
        assert!(peak < 20, "smoke did not rise: the highest it ever got was y = {peak}");
    }

    /// A rising plume must lean downwind, not go straight up.
    ///
    /// The first version of the wind bias passed every "does it use the
    /// wind" reading you could take of the code and still did *nothing*:
    /// `update_gas` tried the straight-up move first, and a plume in open
    /// air always has an empty cell above it, so the wind-weighted
    /// horizontal choice was only ever consulted for gas already trapped
    /// under a ceiling. Two contact sheets of the same blast, bias on and
    /// bias off, came out identical. Measuring the plume's *centroid* is
    /// what settled it — +19.4 cells of sideways drift with the fix against
    /// -3.2 without — and a centroid is what this asserts, for the same
    /// reason: the topmost or leftmost smoke cell is noise, the population's
    /// centre of mass is the signal.
    #[test]
    fn a_rising_plume_leans_downwind() {
        let mut w = world_with_floor();
        for x in 60..70 {
            for y in 110..120 {
                w.set(x, y, Cell::new(material::SMOKE, 0));
            }
        }
        let centroid_x = |w: &World| {
            let cells: Vec<i32> = (0..128)
                .flat_map(|y| (0..128).map(move |x| (x, y)))
                .filter(|&(x, y)| w.get(x, y).material == material::SMOKE)
                .map(|(x, _)| x)
                .collect();
            assert!(!cells.is_empty(), "the smoke vanished entirely");
            cells.iter().sum::<i32>() as f32 / cells.len() as f32
        };
        let before = centroid_x(&w);
        run(&mut w, 60);
        let after = centroid_x(&w);

        // Direction keyed off the constant rather than hardcoded, so
        // flipping the prevailing wind does not silently invert this test
        // into asserting the opposite of what it means.
        let drift = (after - before) * PREVAILING_DRIFT.signum();
        assert!(
            drift > 1.0,
            "a plume in a {PREVAILING_DRIFT} wind drifted {:.2} cells downwind ({before:.1} -> {after:.1})",
            after - before
        );
    }

    /// Wind steers a plume; it must not stop it being buoyant.
    ///
    /// `MAX_LEAN_CHANCE` is capped below 1.0 precisely so some of a plume
    /// always takes the straight-up move. At 1.0 every gas cell in a windy
    /// region leans on the same frame and the plume stops reading as a cloud
    /// being pushed and starts reading as a solid diagonal streak — the same
    /// "many cells making one decision identically" failure
    /// `explosion::Tuning::debris_jitter` and `Particle::drag` both exist to
    /// prevent. This pins the observable half of that: smoke in a wind still
    /// has to climb.
    #[test]
    fn wind_steers_a_plume_without_stopping_it_rising() {
        let mut w = world_with_floor();
        for x in 60..70 {
            for y in 110..120 {
                w.set(x, y, Cell::new(material::SMOKE, 0));
            }
        }
        run(&mut w, 60);
        let top = (0..128)
            .find(|&y| (0..128).any(|x| w.get(x, y).material == material::SMOKE))
            .expect("the smoke vanished entirely");
        assert!(top < 90, "smoke in a crosswind failed to rise: topmost cell at y = {top}, started at 110");
    }

    #[test]
    fn smoke_rises_through_water() {
        let mut w = world_with_floor();
        for y in 60..127 {
            for x in 0..128 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        // A blob, for the reason `smoke_rises` above records.
        for y in 124..127 {
            for x in 63..66 {
                w.set(x, y, Cell::new(material::SMOKE, 0));
            }
        }
        let peak = highest_smoke_reached(&mut w, 400);
        assert!(peak < 60, "smoke did not rise through water: the highest it ever got was y = {peak}");
    }

    /// A stone shell around a pocket packed with smoke, with no route out.
    ///
    /// The point of sealing it: a plume in open air thins whether or not
    /// `Material::dissipation` works at all, because it rises off the top of
    /// the world and leaves. Only a closed box makes disappearance mean
    /// removal.
    fn sealed_pocket_of_smoke() -> World {
        let mut w = world_with_floor();
        for y in 60..80 {
            for x in 40..70 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for y in 63..77 {
            for x in 43..67 {
                w.set(x, y, Cell::new(material::SMOKE, 0));
            }
        }
        w
    }

    /// Frames a sealed pocket is allowed to take, set from measurement with
    /// headroom: the 336-cell pocket below empties at frame 1,687 under the
    /// serial driver and 1,565 under the parallel one, so this leaves a
    /// little over 2x.
    ///
    /// A ceiling on how long the *last* cell may linger, which is not the
    /// quantity the mechanic is judged on -- what matters on screen is the
    /// bulk going in the first few seconds (measured separately, by
    /// `smoke_clears_at_the_rate_the_material_asks_for`). The last straggler
    /// is simply the only thing a test can state without arbitrating a look.
    const BUDGET: usize = 3_500;

    fn smoke_count(w: &World) -> usize {
        (0..128)
            .flat_map(|y| (0..128).map(move |x| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == material::SMOKE)
            .count()
    }

    /// Smoke sealed in a pocket has to go away — and this is the case that
    /// says whether the rule is *reachable*, not merely correct.
    ///
    /// **It caught the first version of this mechanic doing nothing.**
    /// Dissipation started life as a roll on the CA sweep and nothing else,
    /// which is correct code that the sweep stops running: a chunk whose gas
    /// has settled sleeps, and gas that has settled is the whole complaint.
    /// This pocket lost 25 of its 336 cells and kept the other 311 for 2,500
    /// frames. `CLAUDE.md`'s "a test can pass because the code under it is
    /// dead", pointed at the gate rather than at the rule — an open plume
    /// clears either way, because it rises off the top of the world.
    ///
    /// So it runs a **full frame**, sweep then active sites, in `App::
    /// update`'s own order. A test that called only the CA driver would be
    /// exercising the half that provably does not reach this case.
    fn a_sealed_pocket_of_smoke_clears(sweep: fn(&mut World)) {
        let mut w = sealed_pocket_of_smoke();
        let before = smoke_count(&w);
        assert_eq!(before, 336, "the scene is not the one this test thinks it is");

        let mut cleared_at = None;
        for frame in 1..=BUDGET {
            sweep(&mut w);
            w.step_active_sites();
            if smoke_count(&w) == 0 {
                cleared_at = Some(frame);
                break;
            }
        }
        assert!(
            cleared_at.is_some(),
            "{} of {before} smoke cells were still sealed in the pocket after {BUDGET} frames",
            smoke_count(&w)
        );

        // The stone is not allowed to have gone anywhere either -- a "fix"
        // that ate the shell would clear the pocket just as thoroughly.
        // `CLAUDE.md`: a guard must be able to fail for the replacement
        // artifact, not only the original one.
        assert_eq!(w.get(41, 61).material, material::STONE, "the shell went missing");
    }

    #[test]
    fn a_sealed_pocket_of_smoke_clears_serial() {
        a_sealed_pocket_of_smoke_clears(step);
    }

    /// The app runs the parallel driver, and dissipation draws from the rng
    /// -- which `ChunkView` splits per chunk. Both drivers, deliberately.
    #[test]
    fn a_sealed_pocket_of_smoke_clears_parallel() {
        a_sealed_pocket_of_smoke_clears(parallel::step);
    }

    /// The material's number has to set the *rate*, and the rate has to be
    /// the one `MaterialDef::dissipation`'s own arithmetic claims.
    ///
    /// Written because the obvious failure mode of a probabilistic removal
    /// rule is that it reads the field once and then behaves like a switch,
    /// which a test asserting only "the smoke went" cannot tell from
    /// working code. It also pins the doc table, which is the thing anyone
    /// tuning smoke will actually read.
    ///
    /// **A standing count at a fixed frame, not the frame the last cell
    /// goes.** That was the first version and it flaked: the time for the
    /// last of 336 cells is an extreme order statistic — a Gumbel with
    /// roughly ±320 frames of spread at this rate — so it measured 2.53x
    /// for a halving that is exactly 2x by construction. How many are
    /// *left* at a fixed frame is a binomial over all 336 and is tight.
    #[test]
    fn smoke_clears_at_the_rate_the_material_asks_for() {
        const AT: usize = 600;
        let remaining = |rate: f32| {
            let mut w = sealed_pocket_of_smoke();
            w.materials.get_mut(material::SMOKE).dissipation = rate;
            for _ in 0..AT {
                step(&mut w);
                w.step_active_sites();
            }
            smoke_count(&w)
        };
        let (slow, fast) = (remaining(0.004), remaining(0.008));

        // 336 * (1 - 0.004)^600 = 30.3 cells left, and the band is a little
        // over 2x either side of it rather than snug against the sample:
        // what is being pinned is that the half-life is the documented one,
        // not that this particular run landed where it landed.
        assert!(
            (12..=70).contains(&slow),
            "at 0.004 the pocket had {slow} of 336 cells left after {AT} frames; the arithmetic in \
             `MaterialDef::dissipation` says ~30, so the rate the engine runs is not the rate the \
             material asked for"
        );
        // 336 * (1 - 0.008)^600 = 2.7. Stated as a comparison rather than a
        // second absolute band because that is the half that fails if
        // `dissipation` is ever read once and cached: a switch gives these
        // two the same answer.
        assert!(
            fast * 3 < slow,
            "doubling the rate left {fast} cells against {slow} — the two settings are not behaving \
             like two different rates"
        );
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

    /// Wu-Waldron, as a behaviour rather than a citation: soil threaded by
    /// roots holds where bare soil slumps.
    ///
    /// Measured as the *difference* between two identical slopes, one
    /// rooted and one not, rather than against an absolute. A bare pile's
    /// final shape depends on friction angle, repose hysteresis and the
    /// sweep's own ordering, none of which this rule is about; the paired
    /// comparison cancels all of it and leaves only the effect under test.
    #[test]
    fn root_threaded_soil_holds_a_slope_that_bare_soil_loses() {
        let build = |rooted: bool| -> World {
            let mut w = World::new(Rect::new(0, 0, 63, 63));
            let soil = w.materials.id_of("soil").expect("soil is compiled in");
            let rootwood = w.materials.id_of("rootwood").expect("rootwood is compiled in");
            for x in 0..64 {
                w.set(x, 50, Cell::new(material::STONE, 0));
            }
            // A steep bank: a right triangle of soil that cannot stand at
            // soil's own angle of repose and must slump.
            for step in 0..20 {
                for y in (49 - step)..50 {
                    w.set(20 + step, y, Cell::new(soil, 0));
                }
            }
            if rooted {
                // A root *system*, not one strand. The rule reinforces a
                // grain's four neighbours, so a single root column holds
                // only the two columns beside it -- measured at one cell of
                // difference, which is real but says nothing about whether
                // the mechanic matters. Root density is the variable, and a
                // grown tree spreads roots across many columns (see
                // `docs/screenshots/plant-v2-leaves/forest-root-systems.png`),
                // so the representative case is several strands.
                for root_x in [24, 28, 32, 36] {
                    for y in 30..50 {
                        w.set(root_x, y, Cell::new(rootwood, 0));
                    }
                }
            }
            w
        };

        // How far the toe of the bank has spread past where soil started.
        let spread = |w: &World| -> i32 {
            let soil = w.materials.id_of("soil").unwrap();
            (0..64)
                .filter(|&x| (0..50).any(|y| w.get(x, y).material == soil))
                .max()
                .unwrap_or(0)
        };

        let mut bare = build(false);
        let mut rooted = build(true);
        run(&mut bare, 600);
        run(&mut rooted, 600);

        let (bare_spread, rooted_spread) = (spread(&bare), spread(&rooted));
        assert!(
            rooted_spread < bare_spread,
            "root-reinforced soil should slump less far than bare soil: rooted reached {rooted_spread}, bare reached {bare_spread}"
        );
    }

    #[test]
    fn soil_absorbs_a_puddle_and_the_water_shows_up_as_held_moisture() {
        let mut w = World::new(Rect::new(0, 0, 63, 63));
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        for x in 0..64 {
            for y in 40..50 {
                w.set(x, y, Cell::new(soil, 0));
            }
        }
        for x in 20..30 {
            w.set(x, 39, Cell::new(material::WATER, 0));
        }
        let water_before = count(&w, material::WATER);
        assert!(water_before > 0, "test setup: there should be a puddle");

        run(&mut w, 400);

        assert_eq!(count(&w, material::WATER), 0, "a puddle on dry soil should soak in, not sit on top");
        let held: u32 = (0..64)
            .flat_map(|y| (0..64).map(move |x| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == soil)
            .map(|(x, y)| soil_moisture(w.get(x, y)) as u32)
            .sum();
        assert!(
            held > 0,
            "water that soaked in has to be *somewhere* -- it is stored as per-cell moisture, not destroyed"
        );
    }

    /// Reported from live play: "there is no mechanism for water getting
    /// absorbed into soil and increasing its moisture." Correct as
    /// observed, and this is the reproduction.
    ///
    /// Damp ground — not dry ground — is the case that matters, because it
    /// is the case a scene actually contains: soil between rain events sits
    /// near field capacity, which is what `filmstrip`'s `forest` scene
    /// starts at.
    #[test]
    fn damp_soil_still_absorbs_a_puddle() {
        let mut w = World::new(Rect::new(0, 0, 63, 63));
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        for x in 0..64 {
            for y in 40..50 {
                w.set(x, y, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
            }
        }
        for x in 20..30 {
            w.set(x, 39, Cell::new(material::WATER, 0));
        }

        run(&mut w, 600);

        assert_eq!(
            count(&w, material::WATER),
            0,
            "a puddle sitting on damp soil should soak in; ground at field capacity is not waterproof"
        );
    }

    /// Infiltration must not destroy the water it cannot fit.
    ///
    /// A cell is absorbed whole, so any fill beyond the receiving cell's
    /// remaining room used to vanish — invisible in play, and exactly the
    /// class of silent mass leak the engine's own conservation tests exist
    /// to catch elsewhere.
    #[test]
    fn infiltration_conserves_water_it_cannot_fit() {
        let mut w = World::new(Rect::new(0, 0, 31, 31));
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        // One soil cell with only a little room, under a full water cell,
        // walled so nothing can flow away and confuse the accounting.
        for y in 18..24 {
            w.set(14, y, Cell::new(material::STONE, 0));
            w.set(16, y, Cell::new(material::STONE, 0));
        }
        w.set(15, 22, Cell::new(material::STONE, 0));
        w.set(15, 21, Cell::new(soil, 0).with_aux(material::SOIL_SATURATED - 200));
        w.set(15, 20, Cell::new(material::WATER, 0));

        let total = |w: &World| -> u32 {
            let b = w.bounds().unwrap();
            (b.min_y..=b.max_y)
                .flat_map(|y| (b.min_x..=b.max_x).map(move |x| (x, y)))
                .map(|(x, y)| {
                    let c = w.get(x, y);
                    if c.material == material::WATER {
                        liquid_fill(c) as u32
                    } else if c.material == soil {
                        soil_moisture(c) as u32
                    } else {
                        0
                    }
                })
                .sum()
        };
        let before = total(&w);
        run(&mut w, 200);
        assert_eq!(total(&w), before, "water absorbed beyond a cell's room must stay water, not vanish");
    }

    #[test]
    fn saturated_soil_drains_downward_into_drier_soil_below() {
        let mut w = World::new(Rect::new(0, 0, 15, 63));
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        // A floor *and* walls, or the sample positions go stale on their
        // own. Soil is a `Powder`: an unsupported column falls to the
        // bottom of the world, and a one-cell-wide column that *is*
        // supported still topples sideways into a pile at its angle of
        // repose. Either leaves the sampled cells empty, which reads
        // exactly like "drainage did nothing". Contain it so the test
        // measures drainage rather than granular collapse.
        for x in 0..16 {
            w.set(x, 60, Cell::new(material::STONE, 0));
        }
        for y in 39..60 {
            w.set(7, y, Cell::new(material::STONE, 0));
            w.set(9, y, Cell::new(material::STONE, 0));
        }
        // A saturated cap over dry soil: the wetting front should descend.
        for y in 40..60 {
            w.set(8, y, Cell::new(soil, 0));
        }
        w.set(8, 40, Cell::new(soil, 0).with_aux(material::SOIL_SATURATED));

        run(&mut w, 300);

        // Field capacity plus a small dead band, not exactly field
        // capacity. `moved` truncates to `u16`, so once the surplus falls
        // under `1 / SOIL_DRAINAGE_RATE` the transfer rounds to zero and
        // drainage stops a few units high — measured at 623 against a
        // capacity of 620. That is a real property of an integer store and
        // not worth chasing: exactness is explicitly not a goal here, and
        // the alternative (draining the last few units over many more
        // frames) is the "still shuffling fill for another quarter of an
        // hour" cost `CLAUDE.md` warns about. Bar set above the measured
        // value with headroom rather than on it.
        let top = soil_moisture(w.get(8, 40));
        assert!(
            top <= material::SOIL_FIELD_CAPACITY + 16,
            "soil above field capacity must shed its surplus downward; top still holds {top}"
        );
        let below: u32 = (41..60).map(|y| soil_moisture(w.get(8, y)) as u32).sum();
        assert!(below > 0, "the surplus has to arrive somewhere below, as a descending wetting front");
    }

    #[test]
    fn a_powder_that_holds_no_water_neither_absorbs_nor_drains() {
        // The property that keeps every existing liquid-conservation tally
        // true: only a material opting in via `water_capacity` takes part.
        let mut w = World::new(Rect::new(0, 0, 31, 31));
        for x in 0..32 {
            w.set(x, 20, Cell::new(material::SAND, 0));
        }
        for x in 10..20 {
            w.set(x, 19, Cell::new(material::WATER, 0));
        }
        // Total *fill*, not a cell count. The compressible-volume model
        // spreads a fixed volume of water into more, shallower cells as it
        // settles, so occupancy rises while volume does not -- `CLAUDE.md`
        // lists exactly this ("measure fill, not occupancy") as a trap that
        // has already cost this project real time, and counting cells here
        // reported water *increasing* from 10 to 32.
        let volume = |w: &World| -> u32 {
            let b = w.bounds().unwrap();
            (b.min_y..=b.max_y)
                .flat_map(|y| (b.min_x..=b.max_x).map(move |x| (x, y)))
                .filter(|&(x, y)| w.get(x, y).material == material::WATER)
                .map(|(x, y)| liquid_fill(w.get(x, y)) as u32)
                .sum()
        };
        let before = volume(&w);
        run(&mut w, 200);
        assert_eq!(volume(&w), before, "sand holds no water, so no volume may disappear into it");
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

    /// The whisker bar: detached one-cell-tall ledges of water along a
    /// spreading front, reported from live play as "horizontal banding" and
    /// distinct from the row banding below — that one is a fill deficit
    /// *inside* the body, this is the shape of its *edge*.
    ///
    /// **Counts cells in a horizontal run of six or more**, not film cells
    /// outright. That distinction is the whole reason this bug took so long:
    /// a cell in free fall has air above and below it too, because that is
    /// what falling looks like, so counting film cells counts every falling
    /// droplet in the world. Attributing film *creation* under the loose
    /// definition blamed the plain straight-down fall for 76% of them, which
    /// is true and useless. With runs, the cause is unambiguous —
    /// `find_lateral_descent` on: 277 in runs of 6+, 188 in runs of 12+;
    /// off: 13 and 0.
    ///
    /// Bar 40, measured **0**. Fixed by landing a lateral descent where the
    /// cell comes to rest rather than one row down — see
    /// `LIQUID_SETTLE_DROP`. Three other candidates were measured and
    /// rejected first, and are recorded so they are not retried:
    ///
    /// - **Disable `find_lateral_descent`.** Takes runs of 12+ to zero and
    ///   destroys the property that rule exists for: water reads as sand.
    /// - **Land at `(tx, y)` and fall next frame.** Runs of 6+ 277 -> 55,
    ///   but fully-enclosed holes 12 -> 215 and levelling 311 -> 1265
    ///   frames. Rejected on both.
    /// - **Shrink `LIQUID_LATERAL_REACH`.** A pure trade against levelling
    ///   speed with no path to zero.
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

        let mut worst = 0;
        for _ in 0..400 {
            parallel::step(&mut w);
            worst = worst.max(comb_cells(&w, WIDTH, HEIGHT));
        }
        assert!(
            worst <= 40,
            "a spreading front is shedding {worst} cells' worth of detached one-cell ledges              (bar 40, measured 0; before the fix 277)"
        );
    }

    /// The same bar on the geometry the one above cannot reach, and the
    /// reason it is a separate test rather than a wider crop of that one:
    /// **`fall` and `pour` both spread across a floor**, so a film there has
    /// something under it within a row or two and `LIQUID_SETTLE_DROP` puts
    /// it down on that. Water poured onto a short *unwalled shelf* spreads
    /// with open air under most of its length and then pours off both ends,
    /// and that is where a residual comb still lives — 38 cells at its worst
    /// against `fall`'s 0. (`examples/film_probe.rs` reports 32 on the same
    /// scene; it also runs the active-site and field passes, which this bare
    /// `parallel::step` loop does not, and evaporation trims a few teeth.)
    ///
    /// Which matters because the bar above is 40: the geometry that sheds
    /// the most was sitting *under an untested bar*, exactly the "check that
    /// a guard's inputs actually vary what it guards" failure. Bar 80 here,
    /// measured 38, and 233 with `LIQUID_SETTLE_DROP` disabled — so it can
    /// still catch the artifact coming back without flaking on the residue.
    ///
    /// The films this scene sheds are also the only ones in the engine that
    /// are substantially *partial* (19% of them below 40% fill, against 2.5%
    /// on `fall`), which is the population any fill-keyed render treatment
    /// of this bug would have to act on. See `Reports/open-bugs-handoff.md`
    /// §1 for why that treatment is not worth building at these numbers.
    #[test]
    fn a_shelf_pour_does_not_shed_a_comb_either() {
        const WIDTH: i32 = 512;
        const HEIGHT: i32 = 320;
        let mut w = World::new(Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
        stone_floor(&mut w, WIDTH, HEIGHT, 8);
        for x in 180..332 {
            for y in 200..204 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        for x in 236..276 {
            for y in 120..190 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }

        let mut worst = 0;
        for _ in 0..400 {
            parallel::step(&mut w);
            worst = worst.max(comb_cells(&w, WIDTH, HEIGHT));
        }
        assert!(
            worst <= 80,
            "a shelf pour is shedding {worst} cells' worth of detached one-cell ledges \
             (bar 80, measured 38 here and 32 under the probe's fuller step; \
             with LIQUID_SETTLE_DROP disabled 233)"
        );
    }

    /// Cells belonging to a horizontal run of six or more one-cell-tall
    /// water films — the whisker quantity, shared by the two bars above.
    ///
    /// **Runs, not films.** A cell in free fall has air above and below it
    /// too, because that is what falling looks like, so a raw film count
    /// counts every droplet in the world: `find_lateral_descent` on gave 277
    /// in runs of 6+ against 13 with it off, where the raw count barely
    /// moved.
    ///
    /// **A per-frame snapshot, and deliberately not a persistence count.**
    /// The obvious refinement — only count films that are still films at the
    /// same cell next frame — reads *zero* on a world where the comb is
    /// unmistakable by eye, because the comb travels: the front advances one
    /// diagonal step per frame and every tooth is a different cell each
    /// time. Measured with `LIQUID_SETTLE_DROP` disabled: 247 cells in runs
    /// of 6+, sustained for hundreds of frames, and not one *cell* survived
    /// three frames. See `examples/film_probe.rs`.
    fn comb_cells(w: &World, width: i32, height: i32) -> usize {
        let is_film = |x: i32, y: i32| {
            w.get(x, y).material == material::WATER && w.get(x, y - 1).is_empty() && w.get(x, y + 1).is_empty()
        };
        let mut total = 0usize;
        for y in 1..height - 1 {
            let mut run = 0usize;
            for x in 1..width {
                if x < width - 1 && is_film(x, y) {
                    run += 1;
                } else {
                    if run >= 6 {
                        total += run;
                    }
                    run = 0;
                }
            }
        }
        total
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
