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
//! # Why confinement is an anchor
//!
//! Distance-to-bedrock alone is the wrong question to ask of *bulk*
//! material, and no choice of `max_unsupported_span` fixes it. A cell
//! buried 500 cells deep inside a mountain has a path length of 500 —
//! vastly over any sane span — while being the most supported cell in the
//! world. Read literally, that model condemns the interior of every
//! mountain, which is exactly why terrain had to be exempted from checking
//! altogether before this existed (see the next section).
//!
//! The fix comes from what this world *is*. The play world is a 2D
//! vertical **slice** through a 3D world (`Reports/worldgen-design.md`
//! §0: "the world is 2D; the worldgen is 3D"). A real cave ceiling is held
//! up largely by rock out of plane. Requiring every cell to trace an
//! in-plane path to bedrock asks the slice to justify support it
//! structurally has no way to observe — so a cell confined on every side
//! the slice *can* see (`is_confined`, per-material
//! `confinement_radius`) is treated as anchored outright. That is also
//! honest rock mechanics rather than a convenience: confinement is what
//! gives rock its strength, and failure initiates at free faces, not in
//! confined interiors.
//!
//! What falls out, from one local test:
//!
//! - Bulk terrain is *genuinely* anchored rather than accidentally exempt,
//!   which is what `Reports/worldgen-design.md` §6b ("the
//!   structural-integrity landmine") asks for.
//! - A thick cave ceiling stands at any width; a thin one collapses. The
//!   confinement radius **is** the minimum stable thickness, which is §7's
//!   open problem ("a noise-defined ceiling has no bounded thickness")
//!   answered rather than worked around.
//! - A thick, fully disconnected floating island is now permanently
//!   stable. **This is deliberate**, not an oversight: in a 2D slice you
//!   cannot prove it *isn't* supported out of plane, and terrain you can
//!   trust is worth more than a collapse nobody asked for. The
//!   count-to-infinity collapse described below still fires for genuinely
//!   thin unsupported structures, which is where it was always doing the
//!   real work.
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
    let (span, breaks_into, confinement_radius, cost_below, cost_beside, cost_above) = {
        let m = world.materials.get(cell.material);
        (
            m.max_unsupported_span,
            m.breaks_into,
            m.confinement_radius,
            m.support_cost_below,
            m.support_cost_beside,
            m.support_cost_above,
        )
    };

    // Two ways to be an anchor, not one. Touching bedrock is the original
    // and still the literal case; being *confined* is the one that makes
    // bulk terrain structurally valid rather than merely exempt from
    // checking -- see the module doc's "Why confinement is an anchor".
    let is_anchor = NEIGHBOURS_4.iter().any(|&(dx, dy)| world.get(x + dx, y + dy).material == material::BEDROCK)
        || is_confined(world, x, y, confinement_radius);

    let new_distance: u16 = if is_anchor {
        0
    } else {
        let mut best = u16::MAX;
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
/// The relaxation itself is the cheap half, and confinement is why: cells
/// inside bulk rock are anchors by local test alone and never relax, so the
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
            let radius = world.materials.get(cell.material).confinement_radius;
            let anchored = NEIGHBOURS_4.iter().any(|&(dx, dy)| world.get(x + dx, y + dy).material == material::BEDROCK)
                || is_confined(world, x, y, radius);
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

/// Whether `(x, y)` is confined enough by surrounding `Solid` material to
/// count as an anchor in its own right — every cell within Chebyshev
/// `radius` being `Solid`, so the minimum self-supporting thickness is
/// `2 * radius + 1`. See the module doc for why confinement stands in for
/// out-of-plane support, and `MaterialDef::confinement_radius` for why 0
/// means *disabled* rather than "radius zero".
///
/// Deliberately `Solid` only, not `is_body_material`'s `Solid | Plant`: a
/// tree growing against a cliff face does not hold the cliff up. Bedrock
/// and the out-of-bounds sentinel are both `Solid`, so world edges confine
/// — the same single-check trick the bedrock anchor test already relies on.
/// The cell itself is included in the scan, which is what keeps a `Plant`
/// cell from ever self-confining regardless of what its species data says.
fn is_confined(world: &World, x: i32, y: i32, radius: u16) -> bool {
    if radius == 0 {
        return false;
    }
    let r = radius as i32;
    (-r..=r).all(|dy| {
        (-r..=r).all(|dx| world.materials.kind(world.get(x + dx, y + dy).material) == MaterialKind::Solid)
    })
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
    fn a_stone_cantilever_exceeding_its_tolerance_breaks_into_gravel() {
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

        let gravel = w.materials.id_of("gravel").unwrap();
        assert_eq!(w.get(tip, 30).material, gravel, "a cantilever cell past stone's span should have broken into gravel");
        assert_eq!(w.get(0, 30).material, material::STONE, "the anchored root of the cantilever should still be standing");
    }

    #[test]
    fn a_stone_tower_stands_however_tall_it_gets() {
        // The other half of the direction-weighting claim, and the reason
        // the test above had to change shape. Rock is strong in
        // compression: a wall or pillar built up from the ground is
        // supported at every step, so it must not snap at the same reach a
        // cantilever does. One cell wide, so confinement plays no part --
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
        // `a_stone_cantilever_exceeding_its_tolerance_breaks_into_gravel`
        // for why a stack no longer breaks at all.
        let span = w.materials.get(material::STONE).max_unsupported_span as i32;
        for x in 0..=(span + 6) {
            w.set(x, 30, Cell::new(material::STONE, 0));
        }
        let (tx, ty) = (span + 6, 30); // the tip, past the span, will break
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
        // A cantilever, matching the shape
        // `a_stone_cantilever_exceeding_its_tolerance_breaks_into_gravel`
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
        // `confinement_radius` or `support_cost_*`, which also checks that
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
    // The claim `Reports/worldgen-design.md` §7 records as unsolved ("a
    // noise-defined ceiling has no bounded thickness"). These two are
    // deliberately a matched pair at the exact boundary -- `2r + 1` stands
    // and `2r - 1` does not -- so they fail if confinement stops meaning
    // "minimum stable thickness" in either direction, rather than only
    // catching a ceiling that got more fragile.

    /// A floating horizontal slab `thickness` cells deep, spanning the whole
    /// world so its only in-plane anchors are the two world edges, with open
    /// air above and below it so nothing is resting on anything.
    fn ceiling_world(thickness: i32) -> World {
        let mut w = test_world();
        for y in 30..(30 + thickness) {
            for x in 0..64 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        // Disturb the middle, the furthest point from either edge anchor.
        w.schedule_structural_check_around(32, 30 + thickness / 2);
        w
    }

    #[test]
    fn a_ceiling_at_the_confinement_thickness_holds_itself_up() {
        let mut w = test_world();
        let r = w.materials.get(material::STONE).confinement_radius as i32;
        assert!(r > 0, "test is meaningless unless stone actually opts into confinement");
        let span = w.materials.get(material::STONE).max_unsupported_span as i32;
        assert!(32 > span, "the world's midpoint must be past the span, or the edges alone would hold it");

        w = ceiling_world(2 * r + 1);
        run(&mut w, 600);

        assert_eq!(
            w.get(32, 30 + r).material,
            material::STONE,
            "a slab exactly 2r+1 thick should contain a confined row and hold itself up regardless of width"
        );
    }

    #[test]
    fn a_ceiling_below_the_confinement_thickness_comes_down() {
        let mut w = test_world();
        let r = w.materials.get(material::STONE).confinement_radius as i32;
        let thin = 2 * r - 1;
        assert!(thin > 0, "test needs a positive thickness to be a ceiling at all");

        w = ceiling_world(thin);
        run(&mut w, 600);

        let gravel = w.materials.id_of("gravel").unwrap();
        assert_eq!(
            w.get(32, 30 + thin / 2).material,
            gravel,
            "a slab one cell under the confinement thickness has no confined row anywhere in it, so its midpoint is 32 lateral steps from the nearest edge anchor and should have come down"
        );
    }

    #[test]
    fn a_thick_floating_island_stays_up_on_purpose() {
        // The deliberate, documented consequence of treating confinement as
        // an anchor (module doc): in a 2D slice you cannot prove an island
        // *isn't* held up out of plane, so a thick one stays. Pinned as a
        // test because it is exactly the kind of behaviour a later change
        // might "fix" without realising it was chosen.
        let mut w = test_world();
        let r = w.materials.get(material::STONE).confinement_radius as i32;
        let size = 2 * r + 3;
        // Away from every world edge, so no edge sentinel anchors it and
        // confinement is genuinely the only thing holding it up.
        for y in 20..(20 + size) {
            for x in 20..(20 + size) {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        w.schedule_structural_check_around(20 + size / 2, 20 + size / 2);
        run(&mut w, 600);

        assert_eq!(
            w.get(20 + size / 2, 20 + size / 2).material,
            material::STONE,
            "a floating island thicker than the confinement diameter should stay up -- see the module doc on why this is chosen, not accidental"
        );
    }

    #[test]
    fn a_blob_of_brushed_stone_does_not_crumble_in_mid_air() {
        // The reported complaint, encoded directly: "when I use the stone
        // brush it collapses into powder and falls to the ground." Goes
        // through `paint_capsule` -- the real brush path, including its own
        // `schedule_structural_check_around` call -- at `App`'s default
        // `brush_radius` of 6, in open air well clear of every world edge so
        // no edge sentinel can anchor it by accident.
        //
        // Before confinement this had no path to any anchor at all, so its
        // distances climbed without bound (the count-to-infinity dynamic the
        // module doc describes) until every cell passed its span. Now the
        // core of a 13-cell-wide disc is confined, and confined rock is
        // anchored rock.
        let mut w = test_world();
        w.paint_capsule((32, 32), (32, 32), 6, material::STONE, 1.0);
        assert_eq!(w.get(32, 32).material, material::STONE, "test setup: the brush should have painted stone");
        run(&mut w, 600);

        assert_eq!(w.get(32, 32).material, material::STONE, "a brushed stone blob crumbled in mid-air");
        let gravel = w.materials.id_of("gravel").unwrap();
        let crumbled = (0..64).any(|x| (0..64).any(|y| w.get(x, y).material == gravel));
        assert!(!crumbled, "no part of a brushed stone blob should have broken into gravel");
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
