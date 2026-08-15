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
//! path. **Attachment is lost by destruction** — `break_free` and
//! `rigid::try_promote_failing_region` both drop it — so breaking a piece
//! out of a cliff is exactly the moment it stops being held, which is what
//! makes mining produce debris that falls.
//!
//! # Why one step's cost depends on its direction
//!
//! The relaxation used to charge a flat 1 per step regardless of where the
//! support came from, which makes a 1-cell tower built up from the ground
//! fail at exactly the height a 1-cell cantilever reaches sideways. Real
//! rock does not behave that way: it is strong in compression and weak in
//! bending and tension. `support_cost_below`/`_beside`/`_above` split that
//! flat 1 three ways, so standing on something is cheap, leaning out is
//! dear, and hanging is dearest. A wall stands to any height; an overhang
//! still snaps at its span. All three default to 1, which is exactly the
//! behaviour they replaced.
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
//! instead of one it was never asked to sit. An 8-cell floor is thicker
//! reaches bedrock through free downward steps, and the ledges reach it
//! through the walls they are cut into.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

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
    // `break_free`/`world.set` below need `&mut World` and every one of
    // these is `Copy`.
    let (span, breaks_into, cost_below, cost_beside, cost_above) = {
        let m = world.materials.get(cell.material);
        (
            // Attached rock is braced by the mass the slice cannot show, so
            // it reaches much further -- but it is not immune, which is the
            // whole point. See `MaterialDef::attached_span_bonus`.
            if cell.attached() { m.max_unsupported_span.saturating_mul(m.attached_span_bonus) } else { m.max_unsupported_span },
            m.breaks_into,
            m.support_cost_below,
            m.support_cost_beside,
            m.support_cost_above,
        )
    };

    // Fractures cut what the cell can carry. This is what turns a crack
    // from a mark into damage: a scored overhang gives way where an intact
    // one of the same reach holds, and each further blow shortens the reach
    // again -- so a piece visibly weakens before it goes, rather than
    // switching from fine to fallen.
    let span = span.saturating_mul(depth_factor(world, x, y));
    let span = weakened_by_cracks(world, x, y, span);

    // Bedrock is the only outright anchor. Attachment deliberately is *not*
    // one: it buys a much larger span above instead. Anchoring on it made an
    // undercut shelf unfallable -- its interior was still attached, so still
    // held, no matter how much was dug from beneath it. Terrain still stands
    // because the massif reaches bedrock through free downward steps
    // (`support_cost_below` is 0), not because it is exempt.
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

    if new_distance > span {
        let Some(into) = breaks_into else {
            // No configured fallback: this material's span is finite but it
            // has nowhere to fall back to. Treated as "not actually
            // participating" rather than silently deleting content a
            // author forgot to pair `breaks_into` with -- stays Solid,
            // stops checking (matches an unset span's own behaviour).
            return Vec::new();
        };
        // M8: a failure big enough to read as a *chunk* leaves as one
        // coherent falling piece rather than dissolving into loose grains
        // one cell at a time. Falls back to the per-cell conversion below
        // when the region is too small to be worth it or too large to be a
        // chunk at all -- see `rigid::try_promote_failing_region`.
        if !super::rigid::try_promote_failing_region(world, x, y) {
            break_free(world, x, y, into);
        }
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
    if organism_is_supported(world, x, y, organism_id, max_span) {
        // Supported and, unlike the aux-cached path, always an exact
        // answer rather than one still converging -- nothing more to do
        // until something else disturbs this organism's own structure.
        return Vec::new();
    }
    let Some(into) = material.breaks_into else {
        // Same "not actually participating" framing the aux-cached path
        // above uses for the identical case.
        return Vec::new();
    };
    break_free(world, x, y, into);
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

fn break_free(world: &mut World, x: i32, y: i32, into: MaterialId) {
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

/// How much further rock reaches for being *deep* rather than thin.
///
/// A one-cell ledge and a forty-cell-thick cap had exactly the same span,
/// so a massive block of stone was as fragile as a shelf and any noticeable
/// overhang on a thick column tore itself off. Reported from play: a tall
/// thick column with "a shallow overhang ... crumbles by itself".
///
/// Section depth is the missing term, and it is the dominant one in real
/// beams — bending capacity grows with the square of depth, which is why a
/// joist is deep rather than wide. Measured *vertically*, because that is
/// the direction perpendicular to the span of a horizontal overhang, and a
/// horizontal overhang is the case that fails.
///
/// # Why this is not the thickness rule that was tried and reverted
///
/// An earlier model measured burial depth in *all* directions and used it
/// to anchor a cell outright. It made thickness into immunity: nothing at
/// or past the threshold could fail anywhere, mid-air included. This is a
/// bounded multiplier on span, so a deep beam reaches much further and
/// still fails if pushed — and it measures the *beam's* depth rather than
/// how buried a cell is, so a cell on the top face of a thick slab gets the
/// slab's full depth instead of the zero its own burial would suggest.
/// That surface-erosion problem is what sank the earlier attempt's repair.
fn depth_factor(world: &World, x: i32, y: i32) -> u16 {
    let mut depth = 1u16;
    for dir in [-1, 1] {
        let mut step = 1;
        while depth < MAX_SECTION_DEPTH {
            let probe = world.get(x, y + dir * step);
            if !is_body_material(world, probe.material) || probe.material == material::EMPTY {
                break;
            }
            depth += 1;
            step += 1;
        }
    }
    1 + depth / DEPTH_PER_SPAN_STEP
}

/// Deepest section worth measuring. Past this the reach is already far
/// beyond anything a 512-wide world contains, and each extra cell is another
/// read for nothing.
const MAX_SECTION_DEPTH: u16 = 40;

/// Cells of section depth per extra multiple of the base span.
const DEPTH_PER_SPAN_STEP: u16 = 4;

/// Reduce `span` by how badly `(x, y)` is fractured.
///
/// Counts cracked edges, not *intact* ones, and the distinction matters: a
/// cell at a free rock face has fewer neighbours than one buried in a slab,
/// and being at a surface is not damage. Only a fracture is.
///
/// Scales linearly with how much of the cell's cross-section is still
/// joined, floored at 1 so a heavily scored cell is very weak rather than
/// instantly gone — the point is a piece that sags and then fails, not one
/// that vanishes the moment it is touched. All four edges cracked needs no
/// handling here: the relaxation simply finds no path through the cell at
/// all.
///
/// **Engine constant, not `.ron` data, and that is a gap rather than a
/// decision.** `Reports/design-philosophy.md` §2a says a gameplay-facing
/// constant graduates to hot-reloadable data immediately, and "different
/// materials should break differently" is an explicit near-term goal — how
/// brittle a material is under damage is exactly the sort of thing that
/// wants a slider. Left as a constant only because the load model in
/// `Reports/fracture-mechanics-design.md` §1c replaces this arithmetic
/// wholesale, and authoring a field about to be reinterpreted is worse than
/// authoring one once.
fn weakened_by_cracks(world: &World, x: i32, y: i32, span: u16) -> u16 {
    let cracked = NEIGHBOURS_4.iter().filter(|&&(dx, dy)| edge_is_cracked(world, x, y, dx, dy)).count() as u16;
    if cracked == 0 {
        return span;
    }
    let intact = CRACK_FACES.saturating_sub(cracked).max(1);
    (span / CRACK_FACES).saturating_mul(intact).max(1)
}

/// Edges a cell has, and so the denominator for how much of its section a
/// fracture has taken away.
const CRACK_FACES: u16 = 4;

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
    let r = DETACH_DEPTH;
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

        let debris = stone_debris(&w);
        assert_eq!(w.get(tip, 30).material, debris, "a cantilever cell past stone's span should have broken into stone's debris material");
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
    fn brushed_stone_is_foreground_and_unattached_terrain_is_not() {
        // The distinction the whole model now rests on, asserted directly
        // rather than through its consequences: material the player places
        // is foreground and has to earn its support, while terrain says of
        // itself that it is backed by the mass behind the slice.
        //
        // This replaces `a_blob_of_brushed_stone_does_not_crumble_in_mid_air`,
        // whose claim was reversed on purpose. That test pinned a mid-air
        // blob hanging there forever, which was the *symptom* of inferring
        // support from geometry -- the same rule that made every built
        // structure indestructible, reported from play. A rock painted in
        // open air with nothing under it should come down.
        let mut w = test_world();
        w.paint_capsule((32, 32), (32, 32), 6, material::STONE, 1.0);
        assert!(!w.get(32, 32).attached(), "brushed stone must be foreground");

        // Attachment buys *reach*, not immunity -- an attached cell that
        // cannot find an anchor at all still fails, which is what lets an
        // undercut shelf come down. Asserted as the span it gets rather than
        // as anchoring, because anchoring is exactly what made shelves
        // unfallable.
        let m = w.materials.get(material::STONE);
        assert!(m.attached_span_bonus > 1, "stone should get a real bonus from being part of the massif");
        let attached = Cell::new(material::STONE, 0).with_attached(true);
        let loose = Cell::new(material::STONE, 0);
        assert!(attached.attached() && !loose.attached(), "attachment must be carried on the cell, not inferred");
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
        Cell::new(wood, 0).with_organism_id(organism_id).with_aux(organism::pack_aux(organism::CellType::MatureBody, 0.0))
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
}
