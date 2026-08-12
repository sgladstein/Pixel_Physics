//! M16: plant growth — moss, and trees with roots.
//!
//! Dispatched from `scheduler::step` once per due active site. Everything
//! here writes through the ordinary `World::get`/`set` — this runs as its
//! own frame phase, not inside the CA sweep, so there is no `CellSurface`
//! genericity to worry about (see the M5 `surface.rs` doc for why that
//! mattered there and doesn't here).
//!
//! Grounded in `research/m16-plant-biology.md`, not invented from
//! plausible-looking rules — the module doc on each growth function says
//! which real mechanism it's translating and what's simplified. Read that
//! file before touching the constants below; they're not arbitrary.

use super::cell::Cell;
use super::field::FIELD_SCALE;
use super::material::{self, MaterialKind};
use super::organism::{self, Behavior, CellType};
use super::scheduler::{ActiveKind, ActiveSite};
use super::world::World;

const NEIGHBOURS_4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const NEIGHBOURS_8: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// Plain 2D dot product — `Behavior::Grow`'s own candidate-scoring formula
/// (`Reports/tree-rewrite-design.md` §2), not otherwise needed by anything
/// already in this file (the old space-colonization code never needed a
/// dot product, only vector addition/normalization).
fn dot(a: (f32, f32), b: (f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1
}

/// The canopy-density crowding signal `Grow`'s candidate scoring reads for
/// a given open neighbour — the average density among *that candidate's
/// own* same-organism 8-neighbours, not the candidate cell's own `aux`.
/// `organism::diffuse_resource` is a no-op for `organism_id() == 0`
/// (mirroring `diffuse_heat`'s own early return for thermally-inert
/// material), so density never diffuses *into* the empty cells a candidate
/// always is — reading a candidate's own aux read a permanent `0.0` no
/// matter how densely it was actually surrounded, silently turning
/// `crowding_weight` into a no-op and leaving growth with no working
/// self-avoidance term at all (`Reports/tree-rewrite-design.md` §2b's own
/// "deposit → diffuse → decay → follow" self-avoidance mechanism, verified
/// on paper by two independent design reviews, neither of which caught
/// that the "follow" step queries the wrong side of the occupied/empty
/// boundary). Averaging over same-organism neighbours approximates what
/// diffusion's own math would have produced at this position had it been
/// able to write into empty cells, consistent with `diffuse_resource`'s
/// own neighbour-average formula for the same channel.
fn candidate_crowding(world: &World, x: i32, y: i32, organism_id: u16) -> f32 {
    let mut sum = 0.0f32;
    let mut count = 0u32;
    for (dx, dy) in NEIGHBOURS_8 {
        let n = world.get(x + dx, y + dy);
        if n.organism_id() == organism_id {
            sum += organism::canopy_density(n.aux());
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f32
    }
}

/// Canopy density's decay factor, applied once per `organism_tick` call
/// (this module's own scheduling cadence, `ORGANISM_TICK_INTERVAL` apart)
/// rather than once per CA frame the way an earlier version of this
/// mechanism did. That earlier placement (inside `organism::diffuse_
/// resource`, which every awake chunk's CA sweep visits every single
/// frame) decayed a fresh deposit to zero within about ten frames at its
/// old per-frame rate — long before a neighbour's own next `Grow` check,
/// `ORGANISM_TICK_INTERVAL` (45) frames later on average, ever got a
/// chance to read it. Found by live verification (`docs/screenshots/
/// tree-rewrite-live-verification/`): growth still read as a dense round
/// clump even after fixing the separate bug where density was being
/// erased outright (see `pack_aux_preserving_density`'s own doc) — the
/// decay-cadence mismatch was erasing what that fix had just started
/// preserving, just more slowly.
///
/// A multiplicative retain fraction, not a flat subtraction: applied once
/// every `ORGANISM_TICK_INTERVAL`, 0.5 halves a fresh deposit each cycle
/// a cell's own tick fires, which comfortably clears `with_canopy_density`'s
/// quantization half-step (`CANOPY_DENSITY_SCALE / 15.0 / 2 ≈ 0.133`) on
/// every single application regardless of the density's current value —
/// unlike the old per-frame placement, this decay never needs to be tuned
/// small to survive many consecutive calls before the next real read, so
/// it doesn't reopen the quantization-lock bug class `organism.rs`'s own
/// `CANOPY_DENSITY_DECAY` history already found once (see that module's
/// diff for the original fix). Still fades a deposit toward zero over a
/// handful of cycles once nothing nearby keeps refreshing it — the same
/// "let later growth reclaim space near mature wood" intent the original
/// mechanism described, now actually reachable by the checks that matter.
const CANOPY_DENSITY_DECAY_PER_TICK: f32 = 0.5;

/// `organism::pack_aux` alone always resets canopy density to zero (its
/// own doc: "Bits 12-15 start zero") -- every ordinary resource/type
/// update in this dispatch that isn't a brand-new `Grow`/`Divide` child
/// (which correctly gets a fresh deposit via `with_canopy_density`) needs
/// to carry the *existing* cell's density forward instead, or a candidate
/// tip's own deposited signal gets silently erased by the very next
/// `Photosynthesize`/`Absorb`/`Divide` write to that same cell -- typically
/// within the same or the next `organism_tick` cycle, well before decay or
/// diffusion ever get a chance to fade it on their own. `existing_aux`
/// should be the cell's aux from *before* this tick's writes (`organism_
/// tick`'s own `cell.aux()`, read once at the top) -- nothing in this
/// dispatch modifies density mid-tick, so that single read stays valid for
/// every self-update this tick makes.
fn pack_aux_preserving_density(existing_aux: u16, cell_type: CellType, resource: f32) -> u16 {
    organism::with_canopy_density(organism::pack_aux(cell_type, resource), organism::canopy_density(existing_aux))
}

// --- Organism-owned cells (`Reports/organism-substrate-design.md`) ------
//
// Generic dispatch for any species — moss is the only one retrofitted so
// far (§7 of the design report's retrofit order: simplest possible smoke
// test for the scheduler/species-data plumbing). Trees and the worm stay
// on their own dedicated code below/in `creature.rs` until their own
// retrofit lands; see `PLAN.md`'s note on why that's deferred rather than
// rushed in the same pass.

/// Frames between an organism cell's behavior checks. Reused from moss's
/// old `MOSS_TICK_INTERVAL` — not yet a per-species value, since moss is
/// the only caller; a second species needing a different cadence is the
/// actual trigger to make this data instead of a constant.
const ORGANISM_TICK_INTERVAL: u64 = 45;

/// Consecutive checks that found nothing to do (`Divide` found no growable
/// candidate) an organism cell tolerates before it stops rescheduling
/// itself — generalizes moss's old `MOSS_STALE_LIMIT`, same reasoning:
/// bounded so a permanently enclosed cell doesn't sit on the active-site
/// list forever, not an immediate death on the first empty check since a
/// candidate can be transiently unavailable.
const ORGANISM_STALE_LIMIT: u8 = 4;

/// How much canopy density `Behavior::Grow` deposits into a newly-created
/// cell, once, at creation — `organism::diffuse_resource` (and its own
/// `CANOPY_DENSITY_DECAY` doc) handles spreading and fading it from there.
/// A little under half `organism::CANOPY_DENSITY_SCALE`, so a single fresh
/// deposit is real (visibly above the diffusion/decay noise floor) without
/// already saturating the scale on its own.
const GROW_CANOPY_DEPOSIT: f32 = 1.5;

/// Below this reading, `field_at`'s ambient humidity counts as "dry" —
/// architecture report §4. Untuned against anything real, same as every
/// other threshold on this channel; `moss_spreads_over_damp_stone_and_not_
/// over_dry` is the actual authority on whether it's set right.
const DAMP_MOISTURE_THRESHOLD: f32 = 0.3;

fn is_damp(world: &World, x: i32, y: i32) -> bool {
    world.field_at(x, y).moisture > DAMP_MOISTURE_THRESHOLD
}

/// Light reaching a plant-owned or plant-adjacent cell, read from just
/// outside the cell's own position rather than at it. `rebuild_blocked`
/// marks a whole `FIELD_SCALE`-sided block opaque the moment *any* `Solid`
/// or `Plant` cell sits inside it (deliberately, so a canopy shades what's
/// beneath it) — which means a plant cell reading `field_at` at its own
/// exact position always lands inside a block its own material just made
/// opaque, and reads a permanent `0.0` regardless of how bright the sky
/// is one cell away. Never surfaced before this: moss's own light read
/// (`shade_factor`, below) only ever multiplies a probability that fires
/// either way, so a silently-always-shaded reading still looked plausible;
/// the tree rewrite's `Germinate`/`Photosynthesize` are the first behaviors
/// to treat a light reading as a hard gate, which is what turned this from
/// a quiet skew into a permanent deadlock (a seed that can never see
/// enough light to germinate, in open sky, forever). Offsetting the sample
/// upward by one field block is the same trick `phototropism_dir` already
/// uses (`light_above`, in this same file) for exactly this reason, just
/// applied to an absolute reading instead of a relative comparison.
fn ambient_light_above(world: &World, x: i32, y: i32) -> f32 {
    world.field_at(x, y - FIELD_SCALE).light
}

/// Lower ambient light reads as more shaded, which favours spreading —
/// real moss's actual preference (shade slows evaporation), not a made-up
/// bonus. Floored rather than let hit zero, since total darkness isn't a
/// hard "never" biologically, just a strong preference. Only consulted by
/// `Divide` when a species opts in (`shade_sensitive`) — a species with no
/// reason to care about light shouldn't pay for the field read.
fn shade_factor(world: &World, x: i32, y: i32) -> f32 {
    let light = ambient_light_above(world, x, y);
    (1.0 - (light / 2.0).clamp(0.0, 1.0)).max(0.1)
}

/// A candidate cell can grow into if it touches either `Solid` ground or
/// another cell already owned by the same organism — real moss (the only
/// case so far) forms a thickening 2D patch by growing over its own
/// earlier growth, not a single-cell-wide line that can only ever hug the
/// original rock. Without the same-organism case, a cell whose one solid
/// neighbour is already-grown moss (not raw stone) would read as having
/// nowhere to grow from, and every growth front would dead-end after one
/// step.
fn has_growable_neighbour(world: &World, x: i32, y: i32, organism_id: u16) -> bool {
    NEIGHBOURS_4.iter().any(|&(dx, dy)| {
        let neighbour = world.get(x + dx, y + dy);
        world.materials.kind(neighbour.material) == MaterialKind::Solid || neighbour.organism_id() == organism_id
    })
}

fn organism_tick(world: &mut World, x: i32, y: i32, organism_id: u16, stale_ticks: u8) -> Vec<ActiveSite> {
    let cell = world.get(x, y);
    // The cell may have burned, been erased, or already be something else
    // by the time its schedule comes due — mirrors `moss_tick`'s own check,
    // generalized: no longer this organism's cell at all, nothing to run.
    if cell.organism_id() != organism_id {
        return Vec::new();
    }
    // A stale organism id (freed and not yet reused, or a bug) — same
    // "nothing to grow from" outcome as the material check above.
    let Some(state) = world.organism(organism_id) else {
        return Vec::new();
    };
    let species_id = state.species;
    let (Some(mut cell_type), mut resource) = organism::unpack_aux(cell.aux()) else {
        return Vec::new(); // unrecognized cell-type bits -- nothing this dispatch knows how to run
    };
    // Cloned out of the registry rather than held as a borrow: the behavior
    // loop below needs `&mut World` (to paint a new cell, roll the RNG),
    // which a live borrow of `world.species` would conflict with. Species
    // data is small (a handful of behaviors per cell type), so this is
    // cheap relative to the field reads a single `Divide` already does.
    let behaviors: Vec<Behavior> = world.species.get(species_id).behaviors(cell_type).to_vec();

    // Canopy density decays once per call, on this function's own
    // schedule -- see `CANOPY_DENSITY_DECAY_PER_TICK`'s own doc for why
    // this replaced an earlier per-CA-frame placement. Written and rebound
    // into `cell` immediately, before any behavior below reads or
    // preserves density via `pack_aux_preserving_density`, so every write
    // this tick carries the already-decayed value forward, not the
    // pre-decay one.
    let decayed_density = organism::canopy_density(cell.aux()) * CANOPY_DENSITY_DECAY_PER_TICK;
    let cell = cell.with_aux(organism::with_canopy_density(cell.aux(), decayed_density));
    world.set(x, y, cell);

    let mut next = Vec::new();
    let mut found_candidate = false;
    for behavior in behaviors {
        match behavior {
            Behavior::Divide { cost, damp_chance, dry_chance, shade_sensitive } => {
                if resource < cost {
                    continue; // temporary resource shortfall -- try again next tick, not a dead end
                }
                let mut candidates = Vec::new();
                for (dx, dy) in NEIGHBOURS_4 {
                    let (nx, ny) = (x + dx, y + dy);
                    if world.is_empty(nx, ny) && has_growable_neighbour(world, nx, ny, organism_id) {
                        candidates.push((nx, ny));
                    }
                }
                if candidates.is_empty() {
                    continue;
                }
                found_candidate = true;
                let (tx, ty) = candidates[world.rng.below(candidates.len() as u32) as usize];
                let mut chance = if is_damp(world, tx, ty) { damp_chance } else { dry_chance };
                if shade_sensitive {
                    chance *= shade_factor(world, tx, ty);
                }
                if world.rng.chance(chance) {
                    let shades = world.materials.get(cell.material).palette.len().max(1) as u32;
                    let shade = world.rng.below(shades) as u8;
                    // The new cell starts at 0, not at the parent's own
                    // post-cost level -- giving it `resource - cost` (an
                    // earlier version's bug, caught by independent review)
                    // would double-count: the parent already pays `cost`
                    // below, so handing the child that same leftover value
                    // too manufactures `resource - cost` worth of resource
                    // out of nothing on every division. `cost` is what the
                    // division consumes, not what it transfers.
                    let new_cell = Cell::new(cell.material, shade)
                        .with_organism_id(organism_id)
                        .with_aux(organism::pack_aux(cell_type, 0.0));
                    world.set(tx, ty, new_cell);
                    resource -= cost;
                    world.set(x, y, cell.with_aux(pack_aux_preserving_density(cell.aux(), cell_type, resource)));
                    next.push(reschedule_organism(tx, ty, organism_id, 0, world.frame + ORGANISM_TICK_INTERVAL));
                }
            }
            // `Reports/tree-rewrite-design.md` §0/§2/§3: direction-biased,
            // resource-gated, self-avoiding growth for `GrowingTip`
            // (canopy) and `RootTip` (roots) -- deliberately a separate
            // behavior from `Divide` above, not a mode of it; see this
            // module's own `organism.rs` doc for why.
            Behavior::Grow { cost, branch_chance, continuation_weight, light_weight, wind_weight, upward_weight, crowding_weight, max_active_tips } => {
                if resource < cost {
                    continue;
                }
                if world.organism_active_tip_count(organism_id, cell_type) >= max_active_tips as usize {
                    // At the species' own cap -- try again later, the same
                    // "temporary shortfall, not a dead end" framing
                    // `Divide`'s own resource gate already uses.
                    continue;
                }

                // §2a: direction inferred from the local same-organism
                // neighbourhood average, not a stored parent/direction --
                // always well-defined, including at a branch point where a
                // freshly-created tip has two same-organism neighbours
                // (the true lineage parent plus a sibling from the same
                // branch event): the average of both is still a real
                // vector, never degenerate for any valid 8-neighbour
                // offset pair (`Reports/tree-rewrite-design.md` §2a's own
                // worked proof).
                let mut away_sum = (0.0f32, 0.0f32);
                let mut same_organism_neighbours = 0u32;
                for (dx, dy) in NEIGHBOURS_8 {
                    if world.get(x + dx, y + dy).organism_id() == organism_id {
                        away_sum.0 += dx as f32;
                        away_sum.1 += dy as f32;
                        same_organism_neighbours += 1;
                    }
                }
                let away_from_growth = if same_organism_neighbours > 0 {
                    normalize((-away_sum.0, -away_sum.1))
                } else {
                    (0.0, -1.0) // the seed's very first Grow: straight up, same fallback the old Tip's initial dir used
                };

                let photo = organism::phototropism_dir(world, x as f32, y as f32);
                let wind = organism::wind_lean_dir(world, x as f32, y as f32);
                // Gravitropism/hydrotropism antagonism (MIZ1): contributes
                // nothing to canopy growth in practice, since `tree.ron`
                // sets `GrowingTip`'s own `upward_weight` near zero -- the
                // weight does the species-differentiation work here, not a
                // cell-type branch inside this dispatch (`Reports/tree-
                // rewrite-design.md` §2's own "the weight does the work"
                // shape, applied to this term too).
                let gravity_or_water = match organism::moisture_pull(world, x as f32, y as f32) {
                    Some((dir, strength)) if strength >= MIZ_THRESHOLD => dir,
                    _ => (0.0, 1.0), // down
                };

                // §2b: score every open 8-neighbour, weighted-random
                // *sample* from the positive-scoring set -- never a
                // deterministic best-direction pick, which is what would
                // actually curve-fit a silhouette.
                let mut candidates: Vec<(i32, i32, f32)> = Vec::new();
                for (dx, dy) in NEIGHBOURS_8 {
                    let (nx, ny) = (x + dx, y + dy);
                    if !world.is_empty(nx, ny) {
                        continue;
                    }
                    let dir = normalize((dx as f32, dy as f32));
                    let density = candidate_crowding(world, nx, ny, organism_id);
                    let score = dot(dir, away_from_growth) * continuation_weight
                        + dot(dir, photo) * light_weight
                        + dot(dir, wind) * wind_weight
                        + dot(dir, gravity_or_water) * upward_weight
                        - density * crowding_weight;
                    if score > 0.0 {
                        candidates.push((nx, ny, score));
                    }
                }
                if candidates.is_empty() {
                    continue; // every direction actively discouraged, or nothing open -- a genuine dead end, not forced through
                }
                found_candidate = true;

                // A `GrowingTip` that successfully grows retires to
                // `MatureBody` immediately, in the same tick, rather than
                // staying an equally-eligible candidate to grow *again*
                // from the same position next cycle. Without this, the
                // frontier never actually advances: live-verification logs
                // showed 78% of all Grow-evaluated positions revisited 3+
                // times each (one cell hit 8 times), because a cell that
                // had already grown a child kept being just as eligible to
                // sprout an unrelated *second* child from the same spot,
                // and a third, and so on, up to the species' own `max_
                // active_tips` cap -- radiating growth from a small,
                // static set of hub points instead of tips advancing
                // outward, which is what actually produced the dense round
                // clump `Reports/tree-rewrite-design.md` §11's own gate
                // exists to catch, not a scoring-weight tuning problem.
                // The newly created child (and branch child, if any) carry
                // the frontier forward as the new active `GrowingTip`s;
                // `RootTip` is untouched (`tree.ron`'s roots aren't the
                // shape under investigation here, and have no `MatureBody`-
                // equivalent "settled root" type to retire into yet).
                let self_type_after_grow = if cell_type == CellType::GrowingTip { CellType::MatureBody } else { cell_type };

                let total: f32 = candidates.iter().map(|&(_, _, s)| s).sum();
                let mut pick = (world.rng.below(10_000) as f32 / 10_000.0) * total;
                let mut chosen = candidates[0];
                for &c in &candidates {
                    if pick < c.2 {
                        chosen = c;
                        break;
                    }
                    pick -= c.2;
                }
                let (tx, ty, _) = chosen;

                let shades = world.materials.get(cell.material).palette.len().max(1) as u32;
                let shade = world.rng.below(shades) as u8;
                // Canopy density deposited once, here, at creation --
                // `organism::diffuse_resource`'s own doc explains why this
                // lives at the moment of growth rather than a continuous
                // per-tick re-deposit: it lets decay actually fade the
                // signal over time instead of fighting a refill every
                // single sweep visit.
                let new_aux = organism::with_canopy_density(organism::pack_aux(cell_type, 0.0), GROW_CANOPY_DEPOSIT);
                let new_cell = Cell::new(cell.material, shade).with_organism_id(organism_id).with_aux(new_aux);
                world.set(tx, ty, new_cell);
                world.schedule_structural_check_around(tx, ty);
                resource -= cost;
                world.set(x, y, cell.with_aux(pack_aux_preserving_density(cell.aux(), self_type_after_grow, resource)));
                next.push(reschedule_organism(tx, ty, organism_id, 0, world.frame + ORGANISM_TICK_INTERVAL));

                // §3's branching: a second successful `Grow`, in a
                // different direction, this same tick -- gated by the
                // same resource economy as the first, not a separate
                // mechanic.
                if resource >= cost && world.rng.chance(branch_chance) {
                    let alt: Vec<(i32, i32, f32)> = candidates.into_iter().filter(|&(nx, ny, _)| (nx, ny) != (tx, ty)).collect();
                    if !alt.is_empty() {
                        let (bx, by, _) = alt[world.rng.below(alt.len() as u32) as usize];
                        if world.is_empty(bx, by) {
                            let branch_shade = world.rng.below(shades) as u8;
                            let branch_aux = organism::with_canopy_density(organism::pack_aux(cell_type, 0.0), GROW_CANOPY_DEPOSIT);
                            let branch_cell = Cell::new(cell.material, branch_shade).with_organism_id(organism_id).with_aux(branch_aux);
                            world.set(bx, by, branch_cell);
                            world.schedule_structural_check_around(bx, by);
                            resource -= cost;
                            world.set(x, y, cell.with_aux(pack_aux_preserving_density(cell.aux(), self_type_after_grow, resource)));
                            next.push(reschedule_organism(bx, by, organism_id, 0, world.frame + ORGANISM_TICK_INTERVAL));
                        }
                    }
                }

                // Update the loop's own `cell_type` last, after every
                // child (and branch child) above has already been created
                // using the *original* type -- only a later behavior this
                // same tick (`tree.ron`'s `GrowingTip` also has
                // `Photosynthesize`) should see the retirement, not this
                // arm's own child-creation code.
                cell_type = self_type_after_grow;
            }
            Behavior::Photosynthesize { rate } => {
                let light = ambient_light_above(world, x, y);
                resource = (resource + rate * light).min(organism::RESOURCE_SCALE);
                world.set(x, y, cell.with_aux(pack_aux_preserving_density(cell.aux(), cell_type, resource)));
            }
            Behavior::Absorb { rate } => {
                // `Reports/tree-rewrite-design.md` §5: only the passive,
                // stationary drink-in-place mechanic -- the second
                // mechanic (`RootTip`'s `Grow` growing directly into a
                // water cell) lives in the `Grow` dispatch above once a
                // species' `RootTip` candidate scoring treats `Liquid` as
                // absorb-and-advance rather than blocked. Not yet wired
                // there in this pass (`Grow`'s candidate loop above only
                // checks `is_empty`) -- a documented, honest gap, not a
                // silent one: `tree.ron`'s roots rely on this drink-in-
                // place path alone for water uptake until that's added.
                for (dx, dy) in NEIGHBOURS_4 {
                    let (nx, ny) = (x + dx, y + dy);
                    if world.materials.kind(world.get(nx, ny).material) == MaterialKind::Liquid {
                        world.set(nx, ny, Cell::EMPTY);
                        resource = (resource + rate).min(organism::RESOURCE_SCALE);
                        world.deplete_moisture(nx, ny, 1, ROOT_MOISTURE_DEPLETION);
                    }
                }
                world.set(x, y, cell.with_aux(pack_aux_preserving_density(cell.aux(), cell_type, resource)));
            }
            Behavior::SecondaryThicken { pipe_ratio } => {
                thicken(world, x, y, organism_id, pipe_ratio);
                // A `MatureBody` cell's own periodic upkeep, distinct from
                // `Divide`/`Grow`'s "found a growth candidate" signal --
                // it has no candidate to find, but still needs to keep
                // being rechecked. Reuses `ORGANISM_TICK_INTERVAL` rather
                // than the design report's own suggested dedicated 60-
                // frame cadence -- an implementation-time simplification,
                // not a design change; the report's own §6/§8 both
                // explicitly leave room for calls like this.
                found_candidate = true;
            }
            Behavior::Germinate { light_threshold, moisture_threshold, instant } => {
                let ready = instant || {
                    let light = ambient_light_above(world, x, y);
                    let moisture = world.field_at(x, y).moisture;
                    light >= light_threshold && moisture >= moisture_threshold
                };
                if ready {
                    return germinate(world, x, y, organism_id, cell);
                }
                // Not ready yet: waiting on a condition that hasn't
                // arrived, not a dead end -- the same "temporary
                // shortfall" framing `Divide`'s resource gate uses, not
                // the staleness counter (a seed isn't "stuck", it just
                // hasn't germinated yet).
                found_candidate = true;
            }
            // A tag other systems (`structural.rs`) read directly off
            // `CellType`/species data -- no per-tick behavior of its own.
            Behavior::StructuralAnchor => {}
        }
    }

    if found_candidate || !next.is_empty() {
        // A candidate existed this tick (whether or not any behavior's own
        // roll succeeded) -- reset the staleness counter, mirroring
        // `moss_tick`'s old reasoning: staleness tracks "had somewhere to
        // try", not "successfully grew".
        next.push(reschedule_organism(x, y, organism_id, 0, world.frame + ORGANISM_TICK_INTERVAL));
    } else if stale_ticks + 1 < ORGANISM_STALE_LIMIT {
        next.push(reschedule_organism(x, y, organism_id, stale_ticks + 1, world.frame + ORGANISM_TICK_INTERVAL));
    } else if cell_type == CellType::GrowingTip {
        // `Reports/tree-rewrite-design.md` §4: the staleness-limit
        // transition to `MatureBody` made real, not just asserted -- an
        // independent review of the design caught that describing this in
        // prose without an actual `world.set` here would leave
        // `StructuralAnchor`/`SecondaryThicken` (both gated on `MatureBody`
        // in `tree.ron`) never firing on anything, since nothing would
        // ever actually carry that cell type. Carries the tip's own
        // current `resource` forward rather than resetting it.
        world.set(x, y, cell.with_aux(pack_aux_preserving_density(cell.aux(), CellType::MatureBody, resource)));
        // `MatureBody` still needs its own periodic upkeep if the species
        // gave it any behavior (`SecondaryThicken`) -- one more check now,
        // at the standard interval, so it doesn't just go permanently
        // silent the instant it transitions.
        next.push(reschedule_organism(x, y, organism_id, 0, world.frame + ORGANISM_TICK_INTERVAL));
    }
    // Otherwise (any other cell type, e.g. a permanently enclosed
    // `RootTip`): permanently enclosed -- stop checking, matching the old
    // per-tip/per-root "alive: false" outcome.
    next
}

fn reschedule_organism(x: i32, y: i32, organism: u16, stale_ticks: u8, next_frame: u64) -> ActiveSite {
    ActiveSite { x, y, kind: ActiveKind::Organism { organism, stale_ticks }, next_frame }
}

/// A `Seed` cell's `Germinate` firing (`Reports/tree-rewrite-design.md`
/// §8, retrofit step 4): the seed itself becomes the trunk's first
/// `GrowingTip`, and — if the space below is open — a companion `RootTip`
/// starts one cell down, mirroring the old `plant_tree_seed`'s symmetric
/// "one tip up, one root down, both starting at the seed's own position"
/// shape.
fn germinate(world: &mut World, x: i32, y: i32, organism_id: u16, cell: Cell) -> Vec<ActiveSite> {
    world.set(x, y, cell.with_aux(organism::pack_aux(CellType::GrowingTip, 0.0)));
    world.schedule_structural_check_around(x, y);
    let mut next = vec![reschedule_organism(x, y, organism_id, 0, world.frame + ORGANISM_TICK_INTERVAL)];
    if world.is_empty(x, y + 1) {
        let shades = world.materials.get(cell.material).palette.len().max(1) as u32;
        let shade = world.rng.below(shades) as u8;
        let root_cell = Cell::new(cell.material, shade).with_organism_id(organism_id).with_aux(organism::pack_aux(CellType::RootTip, 0.0));
        world.set(x, y + 1, root_cell);
        world.schedule_structural_check_around(x, y + 1);
        next.push(reschedule_organism(x, y + 1, organism_id, 0, world.frame + ORGANISM_TICK_INTERVAL));
    }
    next
}

/// Bound on `SecondaryThicken`'s own downstream-leaf-count flood fill —
/// `Reports/organism-substrate-design.md` §4's own cited cap, "the same
/// order-of-magnitude cap `structural.rs`'s own worst-case neighbourhood
/// traversal already implies is safe per reactive check."
const MAX_THICKEN_SCAN_CELLS: usize = 2000;

/// `SecondaryThicken`, on a `MatureBody` cell: count downstream `Leaf`
/// cells of the same organism through connected `Plant` neighbours
/// (`organism::reachable_from_anchors`, a counting variant), grow sideways
/// into an adjacent empty cell once `leaf_count / current_width >
/// pipe_ratio` — Shinozaki's pipe model theory,
/// `Reports/organism-substrate-design.md` §4's own citation and
/// derivation. `current_width` is a cheap local stand-in (how many
/// same-organism cells sit immediately left/right on this cell's own row)
/// for a real cross-sectional measurement, which would need actual
/// perpendicular-to-growth-axis geometry this engine doesn't track per
/// cell — an honest simplification, not a hidden one.
fn thicken(world: &mut World, x: i32, y: i32, organism_id: u16, pipe_ratio: f32) {
    let is_plant = |c: Cell| c.organism_id() == organism_id && world.materials.kind(c.material) == MaterialKind::Plant;
    let reached = organism::reachable_from_anchors(world, [(x, y)], is_plant, MAX_THICKEN_SCAN_CELLS);
    // Counts `Leaf` *and* `GrowingTip` cells as "downstream photosynthetic
    // load" -- `tree.ron` gives `GrowingTip` its own `Photosynthesize`
    // alongside `Grow` rather than spawning a separate `Leaf` cell type
    // this pass's `Grow` has no mechanism to create (it only ever makes
    // more of its own parent's cell type). A species that *does* grow a
    // distinct `Leaf` type would still be counted correctly here; this
    // isn't `GrowingTip`-specific, it's "every cell type this organism
    // actually uses to catch light."
    let leaf_count = reached
        .iter()
        .filter(|&&(rx, ry)| matches!(organism::unpack_aux(world.get(rx, ry).aux()).0, Some(CellType::Leaf) | Some(CellType::GrowingTip)))
        .count();
    let width = 1 + NEIGHBOURS_4
        .iter()
        .filter(|&&(dx, dy)| dy == 0 && world.get(x + dx, y).organism_id() == organism_id)
        .count();
    if (leaf_count as f32 / width as f32) <= pipe_ratio {
        return;
    }
    for (dx, dy) in [(-1, 0), (1, 0)] {
        let (nx, ny) = (x + dx, y + dy);
        if world.is_empty(nx, ny) {
            let cell = world.get(x, y);
            let shades = world.materials.get(cell.material).palette.len().max(1) as u32;
            let shade = world.rng.below(shades) as u8;
            let new_cell = Cell::new(cell.material, shade).with_organism_id(organism_id).with_aux(organism::pack_aux(CellType::MatureBody, 0.0));
            world.set(nx, ny, new_cell);
            world.schedule_structural_check_around(nx, ny);
            break;
        }
    }
}

// --- Trees and roots ------------------------------------------------------
//
// Space colonization (Runions/Lane/Prusinkiewicz 2007) for the canopy
// shape, extended with two more real mechanisms from the deep-dive
// research pass rather than left as a bare attractor-seeking loop:
//
// - **Auxin canalization** (Prusinkiewicz et al. 2009, PNAS) for branching
//   and apical dominance, translated at three levels rather than one —
//   worth being precise about which parts are genuine cross-tip
//   competition and which are simpler mechanisms that happen to produce a
//   similar-looking result:
//   1. Each `Tip` carries a scalar `channel` that grows on a successful
//      growth step and decays when the tip can't find anything to grow
//      toward, dying below a minimum. This part is *not* cross-tip
//      competition by itself — a tip's channel only ever responds to its
//      own history.
//   2. When a tip's channel is strong enough to spawn a branch, the new
//      branch's starting channel is *debited from the parent's*, not
//      created for free — this is the one place one tip's channel value
//      actually depends on another's, the minimum needed to call it
//      competition rather than N independent scalars that happen to share
//      a name with a real mechanism.
//   3. The bulk of the real "one leader, suppressed siblings" effect
//      actually comes from two *other*, more ordinary mechanisms already
//      needed elsewhere: tips share one `attractors` list that shrinks as
//      any tip consumes nearby points, so whichever tip reaches a cluster
//      first starves siblings of *targets* (plain space colonization); and
//      all tips (and now roots — see `ROOT_GROWTH_COST`) draw from one
//      flat `energy` pool, so a tip can starve even with attractors still
//      in reach. Real auxin canalization is a richer, more actively
//      cross-suppressing mechanism than this — competing transport
//      channels racing to self-reinforce, not shared-resource depletion —
//      and (1)-(3) above is what's actually implemented, not a full model
//      of it.
// - **Gravitropism vs. hydrotropism** (MIZ1 antagonism) for root growth
//   direction. Real roots pick one or the other, not a blend — gravity
//   normally wins, but a strong local moisture gradient actively suppresses
//   the gravity response so water wins instead. Translated as a hard
//   switch on whether a `RootTip`'s local water pull exceeds a threshold.
// - **Oscillator-based lateral root priming** (periodic priming pulses,
//   not a flat per-tick branch probability) for root branching — a root
//   tip only *considers* branching every `ROOT_PRIME_INTERVAL` ticks, and
//   only actually branches if local conditions (water) support it.
//
// Explicitly simplified rather than modeled in full: cytokinin is not a
// separate diffusing field (the research's "bonus" push-pull signal);
// gravitropic setpoint angle is a fixed constant per branch rather than a
// continuously-regulated flux balance; and canopy light competition is a
// direct light-field read rather than Palubicki et al.'s full shadow-voxel
// propagation — a tip's growth direction leans toward locally brighter
// cells, but branches don't yet cast their own shadows into the field.

const ATTRACTOR_COUNT: usize = 50;
const SCATTER_HALF_WIDTH: f32 = 36.0;
const SCATTER_HEIGHT: f32 = 56.0;
const INFLUENCE_RADIUS: f32 = 16.0;
const KILL_DISTANCE: f32 = 5.0;
const SEGMENT_LENGTH: f32 = 2.0;
/// Below this speed, the field's velocity at a tip's position doesn't count
/// as wind at all — architecture §5d. Below the threshold: no lean, `(0.0,
/// 0.0)`, the same shape `photo` already uses for "no gradient, no nudge."
const WIND_SPEED_THRESHOLD: f32 = 0.05;
/// Fixed magnitude of the wind lean once the threshold above is cleared —
/// deliberately *not* scaled by how fast the wind actually is. `field.rs`
/// clamps pressure but never velocity (only damps it), so a magnitude-
/// proportional lean would let a nearby explosion's transient shockwave
/// swamp `avg`/`dir` entirely for as long as the wave takes to pass —
/// found by independent review, which measured a realistic-strength
/// impulse producing a wind term several times the combined magnitude of
/// every other input to this formula. A fixed nudge, direction-only,
/// mirrors `photo`'s own fixed `0.25` and keeps the same "gentle lean, not
/// an override" guarantee regardless of how strong the local field
/// disturbance happens to be. Untuned against anything real, same as every
/// other weight in this formula; `a_tip_leans_downwind_of_a_steady_breeze`
/// is the actual authority on whether it's set right.
const WIND_LEAN_MAGNITUDE: f32 = 0.2;
const TREE_TICK_INTERVAL: u64 = 20;
const GROWTH_COST: f32 = 4.0;
/// Baseline photosynthesis-like energy trickle, independent of roots —
/// real trees don't stop growing entirely without root water, just slower.
const AMBIENT_GROWTH_ENERGY: f32 = 1.2;
const CHANNEL_GROWTH: f32 = 0.10;
const CHANNEL_DECAY: f32 = 0.22;
const MIN_CHANNEL: f32 = 0.15;
const NEW_BRANCH_CHANNEL: f32 = 0.35;
const BRANCH_CHANNEL_THRESHOLD: f32 = 0.7;
const BRANCH_MIN_ATTRACTORS: usize = 3;
const BRANCH_CHANCE: f32 = 0.12;
const BRANCH_ANGLE_RAD: f32 = 0.6;
const MAX_TIPS_PER_TREE: usize = 14;

const ROOT_SEGMENT_LENGTH: f32 = 2.0;
const ROOT_TICK_INTERVAL: u64 = 25;
/// How far apart `moisture_pull`'s two opposing probes sit, per axis —
/// architecture §6a's bilinear-sampling reasoning applies here exactly like
/// it does to tree phototropism's own probe offset: too small and the
/// probes round to the same coarse field block, too large and the "local"
/// gradient stops being local.
const MOISTURE_SENSOR_OFFSET: f32 = 4.0;
/// Minimum moisture-gradient magnitude before MIZ1-style suppression of
/// gravity kicks in and the root steers toward water instead of straight
/// down. A gradient, not a raw moisture reading — MIZ1 biology is
/// specifically a response to a *change* in local humidity, not merely
/// "some water is nearby" (see `moisture_pull`'s own doc). Retuned from the
/// pre-field version's 0.15 (a fraction of sensed neighbours, 0..1 scale) to
/// this channel's own difference-of-readings scale; `roots_steer_toward_
/// off_axis_water_via_hydrotropism` is the actual authority on whether it's
/// set right, same as every other threshold on this channel.
const MIZ_THRESHOLD: f32 = 0.05;
const ROOT_WATER_ENERGY: f32 = 5.0;
/// Local moisture drained per drink — architecture §5g, the write that
/// turns the moisture channel from read-only into a loop. Not tied
/// numerically to `ROOT_WATER_ENERGY` (different units: energy vs. this
/// channel's own 0..4-ish scale) — a separate, freely tunable amount.
const ROOT_MOISTURE_DEPLETION: f32 = 1.0;
/// Cost debited from the shared energy pool for each step of root growth
/// through soil (not water, which is a net gain via `ROOT_WATER_ENERGY`).
/// Real root growth costs the plant carbon same as shoot growth does — a
/// tree with a fully starved canopy (every tip dead) should not be able to
/// keep drilling roots outward for free, which the original version let it
/// do, contradicting `TreeState::energy`'s own doc that every tip *and
/// root* draws from one competitive pool. Smaller than `GROWTH_COST`: real
/// root tissue is metabolically cheaper to build than woody shoot tissue.
const ROOT_GROWTH_COST: f32 = 1.5;
const ROOT_PRIME_INTERVAL: u32 = 6;
const ROOT_BRANCH_ANGLE_RAD: f32 = 1.1;
const MAX_ROOTS_PER_TREE: usize = 10;
/// Consecutive energy-starved ticks a root tolerates before it stops
/// rescheduling itself and goes dormant, mirroring `MOSS_STALE_LIMIT` for
/// the same reason: once every tree tip has died (attractors exhausted,
/// nothing left generating `AMBIENT_GROWTH_ENERGY`) and there's no water
/// nearby to fund `ROOT_WATER_ENERGY`, a root waiting on `energy >=
/// ROOT_GROWTH_COST` would otherwise wait forever — an amount that will
/// never arrive is not a temporary shortfall. Caught by a test
/// (`a_tree_eventually_stops_growing`) that expected the whole tree to go
/// fully dormant and found one immortal root left checking every tick
/// instead, once root growth started costing energy at all.
const ROOT_STARVATION_LIMIT: u8 = 8;

#[derive(Clone, Copy)]
struct Tip {
    pos: (f32, f32),
    dir: (f32, f32),
    channel: f32,
    alive: bool,
}

#[derive(Clone, Copy)]
struct RootTip {
    pos: (f32, f32),
    dir: (f32, f32),
    ticks_since_prime: u32,
    /// Consecutive ticks spent waiting on `ROOT_GROWTH_COST` energy that
    /// never arrived. Reset on every successful growth step (including a
    /// water absorption, since that's itself a net energy gain, not a
    /// wait).
    starved_ticks: u8,
    /// Whether this tip's own active-site cell (its last-written position)
    /// currently holds this tree's `wood`, as opposed to `Empty`. A root
    /// that just grew through soil writes wood there and this is `true`;
    /// a root that just drank an adjacent water cell absorbs it and
    /// advances into the now-empty space with no wood cell left behind
    /// (see `root_tip_tick`'s own comment on why), so this is `false`.
    /// Exists so `root_tip_tick`'s own-cell validity check (mirroring
    /// `tree_tip_tick`'s and `moss_tick`'s) can distinguish "this root's
    /// last-written cell has actually been disturbed" from "this root's
    /// last-written cell is legitimately empty because it drank there" —
    /// the latter is not evidence of anything happening *to* the root, and
    /// checking `Cell::EMPTY` unconditionally would kill a perfectly
    /// healthy root the tick after it drinks.
    resting_on_wood: bool,
    alive: bool,
}

/// Per-tree state too large to fit in a `Cell`'s spare bits (an attractor
/// list, several growth tips, several root tips) — lives in `World`
/// alongside the schedule, indexed by `ActiveKind::TreeTip`/`RootTip`.
pub struct TreeState {
    attractors: Vec<(f32, f32)>,
    tips: Vec<Tip>,
    roots: Vec<RootTip>,
    /// Shared pool every tip and root draws from/credits to — this is what
    /// makes canalization actually competitive: two simultaneously-growing
    /// tips are fighting over the same energy, not each getting their own.
    energy: f32,
    wood: material::MaterialId,
}

/// Plant a tree seed: a trunk-base cell, a scattered attractor field above
/// it, and the first growth tip and root tip. No-ops if the position isn't
/// empty or the `wood` material isn't loaded.
pub fn plant_tree_seed(world: &mut World, x: i32, y: i32) -> Vec<ActiveSite> {
    let Some(wood) = world.materials.id_of("wood") else {
        return Vec::new();
    };
    if !world.is_empty(x, y) {
        return Vec::new();
    }

    let shades = world.materials.get(wood).palette.len().max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    world.set(x, y, Cell::new(wood, shade));

    let mut attractors = Vec::with_capacity(ATTRACTOR_COUNT);
    for _ in 0..ATTRACTOR_COUNT {
        let ax = x as f32 + (world.rng.below(2001) as f32 / 1000.0 - 1.0) * SCATTER_HALF_WIDTH;
        let ay = y as f32 - (world.rng.below(1001) as f32 / 1000.0) * SCATTER_HEIGHT;
        attractors.push((ax, ay));
    }

    let tip = Tip { pos: (x as f32, y as f32), dir: (0.0, -1.0), channel: 0.5, alive: true };
    let root = RootTip { pos: (x as f32, y as f32), dir: (0.0, 1.0), ticks_since_prime: 0, starved_ticks: 0, resting_on_wood: true, alive: true };
    let state = TreeState { attractors, tips: vec![tip], roots: vec![root], energy: GROWTH_COST, wood };
    let tree_id = world.push_tree(state);

    vec![
        reschedule(x, y, ActiveKind::TreeTip { tree: tree_id, tip: 0 }, world.frame + TREE_TICK_INTERVAL),
        reschedule(x, y, ActiveKind::RootTip { tree: tree_id, root: 0 }, world.frame + ROOT_TICK_INTERVAL),
    ]
}

fn tree_tip_tick(world: &mut World, x: i32, y: i32, tree_id: u32, tip_id: u32) -> Vec<ActiveSite> {
    if !world.tree(tree_id).tips[tip_id as usize].alive {
        return Vec::new();
    }
    // Mirrors moss_tick's own check (see its comment) -- `alive` is set
    // false only by this function's own logic (channel decayed past
    // `MIN_CHANNEL`), never by anything happening *to* the tip. Without
    // this, burning the tree or erasing its trunk with the brush leaves
    // every tip's schedule intact: each tick still accrues
    // `AMBIENT_GROWTH_ENERGY`, still consults its attractors, and still
    // calls `world.set` from a position that is now open air, disconnected
    // from anything -- a burned tree keeps growing forever. `(x, y)` is
    // always this tip's own last-written wood cell (the position the site
    // was rescheduled at after every successful growth step, including the
    // seed cell for a freshly planted tree), so comparing against it
    // directly is exact, not approximate.
    let wood_id = world.tree(tree_id).wood;
    if world.get(x, y).material != wood_id {
        world.tree_mut(tree_id).tips[tip_id as usize].alive = false;
        reclaim_if_tree_is_fully_dead(world, tree_id);
        return Vec::new();
    }
    world.tree_mut(tree_id).energy += AMBIENT_GROWTH_ENERGY;

    let (pos, dir) = {
        let tip = &world.tree(tree_id).tips[tip_id as usize];
        (tip.pos, tip.dir)
    };
    let nearby: Vec<(f32, f32)> = world
        .tree(tree_id)
        .attractors
        .iter()
        .copied()
        .filter(|&a| dist(pos, a) <= INFLUENCE_RADIUS)
        .collect();

    if nearby.is_empty() {
        // A genuine dead end -- nothing left worth growing toward from
        // here -- decays the channel same as before.
        let channel = {
            let tip = &mut world.tree_mut(tree_id).tips[tip_id as usize];
            tip.channel -= CHANNEL_DECAY;
            tip.channel
        };
        if channel <= MIN_CHANNEL {
            world.tree_mut(tree_id).tips[tip_id as usize].alive = false;
            reclaim_if_tree_is_fully_dead(world, tree_id);
            return Vec::new();
        }
        return vec![reschedule(x, y, ActiveKind::TreeTip { tree: tree_id, tip: tip_id }, world.frame + TREE_TICK_INTERVAL)];
    }
    if world.tree(tree_id).energy < GROWTH_COST {
        // A temporary resource shortfall, not a pathing failure -- the
        // channel itself represents transport capacity toward a real
        // target, which doesn't wither just because the rest of the plant
        // is short on energy this tick. Wait without decaying; separated
        // from the `nearby.is_empty()` case above deliberately, since
        // conflating "no attractors" with "no energy" here was measured to
        // make `channel` decay far more often than it could grow (energy
        // waits happen roughly 2-3x more often than successful growth
        // ticks at this tuning), so channel could practically never cross
        // `BRANCH_CHANNEL_THRESHOLD` -- caught by a test that actually
        // checked whether branching ever happened, not just "did it grow."
        return vec![reschedule(x, y, ActiveKind::TreeTip { tree: tree_id, tip: tip_id }, world.frame + TREE_TICK_INTERVAL)];
    }

    let mut avg = (0.0f32, 0.0f32);
    for a in &nearby {
        let d = normalize((a.0 - pos.0, a.1 - pos.1));
        avg.0 += d.0;
        avg.1 += d.1;
    }
    avg = normalize(avg);

    // Gentle phototropic lean: bias toward the brighter of "here" and
    // "just above" rather than Palubicki's full shadow-voxel propagation —
    // see the module doc for why that's the one piece left simplified.
    // Bilinear, not block-nearest (architecture §6a): `pos` already carries
    // sub-cell precision, and a 4-pixel-up probe is well within the same
    // `FIELD_SCALE`-sided field block `field_at` would read, which would
    // otherwise compare light-equal almost every tick and leave phototropism
    // inert exactly like the light channel itself was before §2.
    let light_here = world.field_at_bilinear(pos.0, pos.1).light;
    let light_above = world.field_at_bilinear(pos.0, pos.1 - 4.0).light;
    let photo = if light_above > light_here { (0.0, -0.25) } else { (0.0, 0.0) };

    // Wind lean — architecture §5d: "a vector field explosions write into,
    // and vegetation that does not know it exists." A real, if gentle,
    // growth-time bias rather than a per-frame visual sway (nothing in this
    // engine's rendering can bend an already-placed cell), the same way a
    // real tree exposed to a prevailing wind grows with a permanent lean,
    // not just a momentary sway. Read at the tip's own position, not
    // averaged or probed a second point like `photo` — velocity is already
    // a smooth field-scale quantity, not a point sample that needs a
    // second reading to expose a gradient.
    //
    // Direction-only, fixed magnitude — not scaled by wind speed. See
    // `WIND_LEAN_MAGNITUDE`'s own doc for why: velocity has no upper clamp
    // in `field.rs` the way pressure does, so scaling by raw magnitude let
    // a nearby explosion's shockwave dominate this formula outright for as
    // long as the transient took to pass, which is not "a gentle lean" by
    // any reading.
    let wind = world.field_at_bilinear(pos.0, pos.1);
    let wind_speed = (wind.vx * wind.vx + wind.vy * wind.vy).sqrt();
    let wind_lean = if wind_speed > WIND_SPEED_THRESHOLD {
        (wind.vx / wind_speed * WIND_LEAN_MAGNITUDE, wind.vy / wind_speed * WIND_LEAN_MAGNITUDE)
    } else {
        (0.0, 0.0)
    };

    let new_dir = normalize((
        avg.0 * 0.75 + dir.0 * 0.25 + photo.0 + wind_lean.0,
        avg.1 * 0.75 + dir.1 * 0.25 + photo.1 + wind_lean.1,
    ));
    let new_pos = (pos.0 + new_dir.0 * SEGMENT_LENGTH, pos.1 + new_dir.1 * SEGMENT_LENGTH);
    let (wx, wy) = (new_pos.0.round() as i32, new_pos.1.round() as i32);

    let wood = world.tree(tree_id).wood;
    let target = world.get(wx, wy);
    // Blocked by *any* non-empty cell, including wood — this tree's own,
    // or another tree's. `wood` is one shared `MaterialId` across every
    // tree, so comparing against it doesn't distinguish "my own trunk" from
    // "a different tree's" at all; the original version let a tip tunnel
    // straight through any already-grown wood as if it were open air,
    // spending energy on a step that created nothing and could keep
    // advancing (and branching) from a position physically inside another
    // tree's trunk.
    let blocked = !target.is_empty();

    if blocked {
        // Grew into something solid (or the world edge): this tip's
        // channel loses the competition here, same as finding no
        // attractors — it withers rather than forcing through.
        let channel = {
            let tip = &mut world.tree_mut(tree_id).tips[tip_id as usize];
            tip.channel -= CHANNEL_DECAY;
            tip.channel
        };
        if channel <= MIN_CHANNEL {
            world.tree_mut(tree_id).tips[tip_id as usize].alive = false;
            reclaim_if_tree_is_fully_dead(world, tree_id);
            return Vec::new();
        }
        return vec![reschedule(x, y, ActiveKind::TreeTip { tree: tree_id, tip: tip_id }, world.frame + TREE_TICK_INTERVAL)];
    }

    let shades = world.materials.get(wood).palette.len().max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    world.set(wx, wy, Cell::new(wood, shade));
    world.tree_mut(tree_id).energy -= GROWTH_COST;
    world.tree_mut(tree_id).attractors.retain(|&a| dist(new_pos, a) > KILL_DISTANCE);

    let channel = {
        let tip = &mut world.tree_mut(tree_id).tips[tip_id as usize];
        tip.pos = new_pos;
        tip.dir = new_dir;
        tip.channel = (tip.channel + CHANNEL_GROWTH).min(1.0);
        tip.channel
    };

    let mut next = vec![reschedule(wx, wy, ActiveKind::TreeTip { tree: tree_id, tip: tip_id }, world.frame + TREE_TICK_INTERVAL)];

    let can_branch = channel > BRANCH_CHANNEL_THRESHOLD
        && nearby.len() >= BRANCH_MIN_ATTRACTORS
        && world.tree(tree_id).tips.len() < MAX_TIPS_PER_TREE;
    if can_branch && world.rng.chance(BRANCH_CHANCE) {
        let angle = if world.rng.flip() { BRANCH_ANGLE_RAD } else { -BRANCH_ANGLE_RAD };
        let branch = Tip { pos: new_pos, dir: rotate(new_dir, angle), channel: NEW_BRANCH_CHANNEL, alive: true };
        let branch_id = world.tree(tree_id).tips.len() as u32;
        world.tree_mut(tree_id).tips.push(branch);
        // The new channel draws capacity away from the parent's — real
        // auxin canalization is competing sources, not independent ones,
        // and without this a parent tip's channel value never depended on
        // anything but its own history, i.e. no cross-tip interaction at
        // all despite the name. Costing the parent exactly what the new
        // branch starts with keeps the tree's total channel "budget"
        // meaningful rather than manufacturing capacity for free.
        world.tree_mut(tree_id).tips[tip_id as usize].channel -= NEW_BRANCH_CHANNEL;
        next.push(reschedule(wx, wy, ActiveKind::TreeTip { tree: tree_id, tip: branch_id }, world.frame + TREE_TICK_INTERVAL));
    }

    next
}

fn root_tip_tick(world: &mut World, x: i32, y: i32, tree_id: u32, root_id: u32) -> Vec<ActiveSite> {
    if !world.tree(tree_id).roots[root_id as usize].alive {
        return Vec::new();
    }
    // Mirrors tree_tip_tick's own check, with one wrinkle: a root's
    // last-written cell is only *sometimes* wood (see
    // `RootTip::resting_on_wood`'s doc) -- when it drank water there
    // instead of growing through soil, the cell is legitimately `Empty`,
    // and checking for wood unconditionally would kill a perfectly healthy
    // root the tick after it drinks. Only validate when the tip actually
    // expects wood to still be there.
    let wood_id = world.tree(tree_id).wood;
    let resting_on_wood = world.tree(tree_id).roots[root_id as usize].resting_on_wood;
    if resting_on_wood && world.get(x, y).material != wood_id {
        world.tree_mut(tree_id).roots[root_id as usize].alive = false;
        reclaim_if_tree_is_fully_dead(world, tree_id);
        return Vec::new();
    }

    let (pos, dir, mut ticks) = {
        let r = &world.tree(tree_id).roots[root_id as usize];
        (r.pos, r.dir, r.ticks_since_prime)
    };

    // Roots drink from every adjacent water cell before moving, crediting
    // the tree's shared energy pool — the mechanic that actually ties
    // plant growth to the rest of the material physics.
    for (dx, dy) in NEIGHBOURS_4 {
        let (nx, ny) = (pos.0 as i32 + dx, pos.1 as i32 + dy);
        if world.materials.kind(world.get(nx, ny).material) == MaterialKind::Liquid {
            world.set(nx, ny, Cell::EMPTY);
            world.tree_mut(tree_id).energy += ROOT_WATER_ENERGY;
            world.deplete_moisture(nx, ny, 1, ROOT_MOISTURE_DEPLETION);
        }
    }

    // Gravitropism vs. hydrotropism: a real antagonism switch (MIZ1
    // suppresses the gravity response under a strong moisture gradient),
    // not a blend of the two pulls.
    let gravity = (0.0f32, 1.0f32);
    let water_pull = moisture_pull(world, pos.0, pos.1);
    let base = match water_pull {
        Some((dir, strength)) if strength >= MIZ_THRESHOLD => dir,
        _ => gravity,
    };
    let new_dir = normalize((base.0 * 0.7 + dir.0 * 0.3, base.1 * 0.7 + dir.1 * 0.3));
    let new_pos = (pos.0 + new_dir.0 * ROOT_SEGMENT_LENGTH, pos.1 + new_dir.1 * ROOT_SEGMENT_LENGTH);
    let (wx, wy) = (new_pos.0.round() as i32, new_pos.1.round() as i32);

    // A root pushes through loose/open ground, not solid rock — the same
    // "displaceable substrate" boundary the burrowing-creature research
    // (M18) uses, reused here since it's the same real constraint. Growing
    // into a water pocket is a third case, not a subset of either: it's
    // absorbed on the spot (same as the neighbour-drink check above) and
    // the tip advances into the now-empty space rather than leaving a
    // solid wood cell behind, which would be true growth *into* water
    // rather than the water actually being taken up. Without this case a
    // root approaching any wet ground would simply die at its edge, never
    // reaching the moisture it grew toward in the first place.
    let target_kind = world.materials.kind(world.get(wx, wy).material);
    let wood = world.tree(tree_id).wood;
    // Whether this tick's outcome leaves wood at (wx, wy) or leaves it
    // empty (the water-drink case) -- stored on the tip below so next
    // tick's own-cell validity check knows which of those two states is
    // the *expected*, undisturbed one. See `RootTip::resting_on_wood`.
    let resting_on_wood = target_kind != MaterialKind::Liquid;
    match target_kind {
        MaterialKind::Liquid => {
            world.set(wx, wy, Cell::EMPTY);
            world.tree_mut(tree_id).energy += ROOT_WATER_ENERGY;
            world.deplete_moisture(wx, wy, 1, ROOT_MOISTURE_DEPLETION);
            world.tree_mut(tree_id).roots[root_id as usize].starved_ticks = 0;
        }
        MaterialKind::Empty | MaterialKind::Powder => {
            // Growing through soil costs energy from the same shared pool
            // tree tips draw from — real root growth costs the plant
            // carbon too, same as shoot growth, and this is what makes
            // `TreeState::energy` an actually-competitive shared resource
            // rather than something only tips participate in. Not enough
            // yet: wait where it is (this tick's water-drink above still
            // happened and is kept) rather than moving for free.
            if world.tree(tree_id).energy < ROOT_GROWTH_COST {
                let starved = {
                    let r = &mut world.tree_mut(tree_id).roots[root_id as usize];
                    r.starved_ticks += 1;
                    r.starved_ticks
                };
                // An amount that will never arrive (every tip dead, no
                // water nearby to fund the shortfall) is not a temporary
                // wait — see `ROOT_STARVATION_LIMIT`'s doc.
                if starved >= ROOT_STARVATION_LIMIT {
                    world.tree_mut(tree_id).roots[root_id as usize].alive = false;
                    reclaim_if_tree_is_fully_dead(world, tree_id);
                    return Vec::new();
                }
                return vec![reschedule(
                    x,
                    y,
                    ActiveKind::RootTip { tree: tree_id, root: root_id },
                    world.frame + ROOT_TICK_INTERVAL,
                )];
            }
            let shades = world.materials.get(wood).palette.len().max(1) as u32;
            let shade = world.rng.below(shades) as u8;
            world.set(wx, wy, Cell::new(wood, shade));
            world.tree_mut(tree_id).energy -= ROOT_GROWTH_COST;
            world.tree_mut(tree_id).roots[root_id as usize].starved_ticks = 0;
        }
        _ => {
            world.tree_mut(tree_id).roots[root_id as usize].alive = false;
            reclaim_if_tree_is_fully_dead(world, tree_id);
            return Vec::new();
        }
    }

    ticks += 1;
    let mut next = Vec::new();
    let mut primed = false;
    if ticks >= ROOT_PRIME_INTERVAL {
        ticks = 0;
        primed = true;
    }
    {
        let r = &mut world.tree_mut(tree_id).roots[root_id as usize];
        r.pos = new_pos;
        r.dir = new_dir;
        r.ticks_since_prime = ticks;
        r.resting_on_wood = resting_on_wood;
    }
    next.push(reschedule(wx, wy, ActiveKind::RootTip { tree: tree_id, root: root_id }, world.frame + ROOT_TICK_INTERVAL));

    // Oscillator-based priming, not a flat per-tick probability: only
    // *consider* branching periodically, and only follow through if local
    // conditions (water nearby) actually support a new lateral root.
    if primed && is_damp(world, wx, wy) && world.tree(tree_id).roots.len() < MAX_ROOTS_PER_TREE {
        let angle = if world.rng.flip() { ROOT_BRANCH_ANGLE_RAD } else { -ROOT_BRANCH_ANGLE_RAD };
        let branch =
            RootTip { pos: new_pos, dir: rotate(new_dir, angle), ticks_since_prime: 0, starved_ticks: 0, resting_on_wood, alive: true };
        let branch_id = world.tree(tree_id).roots.len() as u32;
        world.tree_mut(tree_id).roots.push(branch);
        next.push(reschedule(wx, wy, ActiveKind::RootTip { tree: tree_id, root: branch_id }, world.frame + ROOT_TICK_INTERVAL));
    }

    next
}

/// Direction and magnitude of the moisture gradient at `(x, y)` — replaces
/// the old O(r²) hand-rolled neighbour scan (architecture §4, "hand-rolled
/// sensing that should be field reads") with two pairs of opposing field
/// reads, one per axis, the same central-difference shape `tree_tip_tick`'s
/// phototropism probe uses. Bilinear (§6a), not block-nearest: the two
/// probes on either axis are only `2 * MOISTURE_SENSOR_OFFSET` world cells
/// apart, well inside the same coarse field block block-nearest reads would
/// flatten to identical values.
///
/// `None` when the gradient is flat (`(0.0, 0.0)`, no direction to
/// `normalize`) — open dry ground with no nearby source, or a spot exactly
/// balanced between two sources, both of which should fall through to plain
/// gravitropism rather than pick an arbitrary direction.
fn moisture_pull(world: &World, x: f32, y: f32) -> Option<((f32, f32), f32)> {
    let gx =
        world.field_at_bilinear(x + MOISTURE_SENSOR_OFFSET, y).moisture - world.field_at_bilinear(x - MOISTURE_SENSOR_OFFSET, y).moisture;
    let gy =
        world.field_at_bilinear(x, y + MOISTURE_SENSOR_OFFSET).moisture - world.field_at_bilinear(x, y - MOISTURE_SENSOR_OFFSET).moisture;
    let magnitude = (gx * gx + gy * gy).sqrt();
    if magnitude <= f32::EPSILON {
        return None;
    }
    Some((normalize((gx, gy)), magnitude))
}

fn reschedule(x: i32, y: i32, kind: ActiveKind, next_frame: u64) -> ActiveSite {
    ActiveSite { x, y, kind, next_frame }
}

/// Once every tip and root of a tree has died, its `attractors` list (up to
/// `ATTRACTOR_COUNT` points, by far the largest part of `TreeState`) serves
/// no further purpose — nothing left will ever consult it. `TreeState`
/// itself still never shrinks (`World::push_tree`'s own doc explains why:
/// `ActiveKind::TreeTip`/`RootTip` index into it by position, and
/// reassigning a slot would invalidate a live `ActiveSite` — the real fix
/// is generational indices with a free list, `pixel-physics-issues.md`
/// issue #8's own "Direction"), but freeing the one large allocation that
/// is genuinely dead is a real, low-risk partial mitigation cheap enough to
/// do inline at every death site, rather than waiting on that larger
/// rewrite. Checked (not assumed) at every call site rather than only when
/// the *last* tip or root dies, since which one dies last isn't tracked —
/// this runs a few extra times over a tree's life, each one O(tips + roots)
/// and cheap, in exchange for not needing to track that separately.
fn reclaim_if_tree_is_fully_dead(world: &mut World, tree_id: u32) {
    let fully_dead = {
        let tree = world.tree(tree_id);
        tree.tips.iter().all(|t| !t.alive) && tree.roots.iter().all(|r| !r.alive)
    };
    if fully_dead {
        world.tree_mut(tree_id).attractors = Vec::new(); // drops the old allocation; an empty Vec has none of its own
    }
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn normalize(v: (f32, f32)) -> (f32, f32) {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    if len < 1e-6 {
        (0.0, 0.0)
    } else {
        (v.0 / len, v.1 / len)
    }
}

fn rotate(v: (f32, f32), radians: f32) -> (f32, f32) {
    let (s, c) = radians.sin_cos();
    (v.0 * c - v.1 * s, v.0 * s + v.1 * c)
}

/// Dispatch one due active site to its growth function. Called from
/// `scheduler::step` for every `ActiveKind` except `StructuralCheck`,
/// `Creature` and `Decay`, which `scheduler::step` routes to `structural::
/// tick`/`creature::tick`/`decay::tick` instead -- the match here still has
/// to name all three variants to stay exhaustive.
pub fn tick(world: &mut World, site: &ActiveSite) -> Vec<ActiveSite> {
    match site.kind {
        ActiveKind::Organism { organism, stale_ticks } => organism_tick(world, site.x, site.y, organism, stale_ticks),
        ActiveKind::TreeTip { tree, tip } => tree_tip_tick(world, site.x, site.y, tree, tip),
        ActiveKind::RootTip { tree, root } => root_tip_tick(world, site.x, site.y, tree, root),
        ActiveKind::StructuralCheck => unreachable!("scheduler::step routes StructuralCheck to structural::tick"),
        ActiveKind::Creature { .. } => unreachable!("scheduler::step routes Creature to creature::tick"),
        ActiveKind::Decay => unreachable!("scheduler::step routes Decay to decay::tick"),
    }
}

impl World {
    /// Plant a moss seed at `(x, y)` if it's empty, and schedule its first
    /// growth check. A no-op (returns nothing to schedule) if the position
    /// isn't empty, the `moss` material isn't loaded, or the `moss`
    /// species isn't loaded (`Reports/organism-substrate-design.md`'s new
    /// species registry, parallel to but independent from materials).
    pub fn plant_moss_seed(&mut self, x: i32, y: i32) {
        let Some(moss_material) = self.materials.id_of("moss") else {
            return;
        };
        let Some(moss_species) = self.species.id_of("moss") else {
            return;
        };
        if !self.is_empty(x, y) {
            return;
        }
        let shades = self.materials.get(moss_material).palette.len().max(1) as u32;
        let shade = self.rng.below(shades) as u8;
        let organism_id = self.push_organism(moss_species);
        let aux = organism::pack_aux(CellType::GrowingTip, 0.0);
        self.set(x, y, Cell::new(moss_material, shade).with_organism_id(organism_id).with_aux(aux));
        let site = reschedule_organism(x, y, organism_id, 0, self.frame + ORGANISM_TICK_INTERVAL);
        self.schedule_active_site(site);
    }

    /// Plant a tree seed at `(x, y)` — see `plant_tree_seed` above.
    pub fn plant_tree(&mut self, x: i32, y: i32) {
        for site in plant_tree_seed(self, x, y) {
            self.schedule_active_site(site);
        }
    }

    /// Plant a `tree` species `Seed` cell at `(x, y)` — `Reports/tree-
    /// rewrite-design.md` §1/§11's own corrected retrofit order: a
    /// deliberately separate entry point from `plant_tree` above, so the
    /// new `Grow`/`Germinate`-driven system can be built and live-
    /// verified with the old `TreeState`-based implementation completely
    /// untouched, reachable only through its own existing `plant_tree`/`T`
    /// key, until the new system's own visual-verification gate passes.
    pub fn plant_tree_v2(&mut self, x: i32, y: i32) {
        self.plant_tree_species(x, y, "tree");
    }

    /// Plant a `Seed` cell of any tree-shaped species (a `Seed` cell type
    /// that germinates into a `wood`-material `GrowingTip`) at `(x, y)` --
    /// generalizes `plant_tree_v2` to a caller-chosen species name, so
    /// several differently-tuned variants (same behavior shapes, different
    /// weights -- e.g. `Grow`'s `cost` vs `Photosynthesize`'s `rate`) can
    /// be planted side by side in one scene and compared empirically,
    /// rather than editing `tree.ron` and re-running once per candidate.
    /// Returns whether planting actually happened -- `false` if the
    /// position isn't empty or `wood`/the named species isn't loaded,
    /// mirroring `plant_moss_seed`'s own no-op preconditions.
    pub fn plant_tree_species(&mut self, x: i32, y: i32, species_name: &str) -> bool {
        let Some(wood) = self.materials.id_of("wood") else {
            return false;
        };
        let Some(tree_species) = self.species.id_of(species_name) else {
            return false;
        };
        if !self.is_empty(x, y) {
            return false;
        }
        let shades = self.materials.get(wood).palette.len().max(1) as u32;
        let shade = self.rng.below(shades) as u8;
        let organism_id = self.push_organism(tree_species);
        let aux = organism::pack_aux(CellType::Seed, 0.0);
        self.set(x, y, Cell::new(wood, shade).with_organism_id(organism_id).with_aux(aux));
        let site = reschedule_organism(x, y, organism_id, 0, self.frame + ORGANISM_TICK_INTERVAL);
        self.schedule_active_site(site);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::field;
    use crate::sim::update;

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 199, 199))
    }

    fn run(w: &mut World, frames: usize) {
        for _ in 0..frames {
            update::step(w);
            w.step_active_sites();
        }
    }

    /// Same as `run`, plus `field::step` every frame, ordered last to match
    /// `App::update`'s own real frame order. Only the tests that actually
    /// depend on a field channel (moisture, light) use this instead of
    /// plain `run` — most of this module's tests deliberately don't touch
    /// the field solver at all, isolating CA/scheduler behaviour from field
    /// behaviour the same way `field.rs`'s own module doc explains for the
    /// reverse case.
    fn run_with_fields(w: &mut World, frames: usize) {
        for _ in 0..frames {
            update::step(w);
            w.step_active_sites();
            field::step(w);
        }
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

    #[test]
    fn planting_on_occupied_ground_is_a_no_op() {
        let mut w = test_world();
        w.set(50, 50, Cell::new(material::STONE, 0));
        w.plant_tree(50, 50);
        w.plant_moss_seed(50, 50);
        assert_eq!(w.get(50, 50).material, material::STONE, "planting overwrote existing material");
        assert_eq!(w.active_site_count(), 0, "a no-op plant should not schedule anything");
    }

    #[test]
    fn moss_spreads_over_damp_stone_and_not_over_dry() {
        let mut w = test_world();
        // Two separate stone platforms, identical except one has a shallow
        // puddle resting directly on it. Both get a moss seed at their own
        // edge. The puddle is placed directly rather than poured from
        // above and left to find its own level: a narrow platform with
        // open edges lets water drain off the sides and fall away
        // entirely, leaving nothing actually damp by the time growth
        // starts checking.
        for x in 9..31 {
            w.set(x, 50, Cell::new(material::STONE, 0));
        }
        w.set(9, 49, Cell::new(material::STONE, 0)); // walls -- keep the
        w.set(30, 49, Cell::new(material::STONE, 0)); // puddle from draining
        for x in 12..18 {
            w.set(x, 49, Cell::new(material::WATER, 0));
        }
        for x in 60..80 {
            w.set(x, 50, Cell::new(material::STONE, 0)); // far from any water
        }
        w.plant_moss_seed(20, 49);
        w.plant_moss_seed(70, 49);
        run_with_fields(&mut w, 4000);

        let moss = w.materials.id_of("moss").unwrap();
        let damp_moss = (10..30).filter(|&x| (44..51).any(|y| w.get(x, y).material == moss)).count();
        let dry_moss = (60..80).filter(|&x| (44..51).any(|y| w.get(x, y).material == moss)).count();
        // A relative comparison, not "dry must stay at exactly 1" -- the
        // dry side's small nonzero chance (real moss can survive brief dry
        // spells) means an occasional stray spread there is expected, not
        // a bug; what must hold is that damp spreads meaningfully more.
        assert!(damp_moss > 3, "moss did not spread over damp stone: {damp_moss} cells");
        assert!(
            damp_moss > dry_moss * 3,
            "moss should spread much more over damp stone ({damp_moss}) than dry ({dry_moss})"
        );
    }

    #[test]
    fn moss_thickens_into_a_patch_by_growing_over_its_own_earlier_growth() {
        // `has_growable_neighbour`'s same-organism_id branch: without it, a
        // cell whose only solid neighbour is already-grown moss (not raw
        // stone) reads as having nowhere to grow from, and a colony can
        // only ever be a single-cell-wide line hugging the original rock,
        // never thickening into a real patch.
        let mut w = test_world();
        for x in 5..35 {
            w.set(x, 50, Cell::new(material::STONE, 0));
        }
        w.set(5, 49, Cell::new(material::STONE, 0)); // walls -- keep the
        w.set(34, 49, Cell::new(material::STONE, 0)); // puddle from draining off the sides
        for x in 10..30 {
            w.set(x, 49, Cell::new(material::WATER, 0));
        }
        w.plant_moss_seed(20, 48);
        run_with_fields(&mut w, 6000);

        let moss = w.materials.id_of("moss").unwrap();
        // The seed sits at row 48; nothing above row 48 has a stone
        // neighbour at all (stone is only at row 50, water fills row 49) --
        // the *only* way for moss to ever reach row 47 or higher is by
        // growing over another moss cell of its own organism. Any moss
        // found there is proof the patch thickened, not just spread
        // sideways hugging the water's own row.
        let thickened = (5..35).any(|x| (40..48).any(|y| w.get(x, y).material == moss));
        assert!(thickened, "moss never thickened into a 2D patch, only ever grew along the original rock");
    }

    #[test]
    fn divide_deducts_cost_from_the_parent_without_manufacturing_resource() {
        // A synthetic species with a real Divide cost -- moss's own `cost`
        // is 0.0 and can't exercise this at all. The exact bug an
        // independent review caught: an earlier version wrote `resource -
        // cost` onto the *new* cell while leaving the parent's `aux`
        // completely untouched, silently manufacturing `resource - cost`
        // worth of resource out of nothing on every division (the parent
        // kept its full amount *and* the child got a nonzero amount too).
        let material_dir = std::env::temp_dir().join("pixel-physics-divide-cost-material");
        std::fs::create_dir_all(&material_dir).unwrap();
        std::fs::write(material_dir.join("costtest.ron"), "(name: \"costtest\", kind: Plant, density: 0.3, colors: [(1, 2, 3)])").unwrap();
        let species_dir = std::env::temp_dir().join("pixel-physics-divide-cost-species");
        std::fs::create_dir_all(&species_dir).unwrap();
        std::fs::write(
            species_dir.join("costtest.ron"),
            "(name: \"costtest\", cell_types: [(GrowingTip, [Divide(cost: 1.0, damp_chance: 1.0, dry_chance: 1.0, shade_sensitive: false)])])",
        )
        .unwrap();

        let mut w = test_world();
        w.materials.reload(&material_dir).unwrap();
        w.species.reload(&species_dir).unwrap();
        let material = w.materials.id_of("costtest").unwrap();
        let species = w.species.id_of("costtest").unwrap();

        let organism_id = w.push_organism(species);
        let start_resource = 3.0;
        let seed = Cell::new(material, 0).with_organism_id(organism_id).with_aux(organism::pack_aux(CellType::GrowingTip, start_resource));
        w.set(50, 50, seed);

        organism_tick(&mut w, 50, 50, organism_id, 0);

        let (_, parent_resource) = organism::unpack_aux(w.get(50, 50).aux());
        // `damp_chance`/`dry_chance` are both 1.0, so exactly one of the
        // four open neighbours divides successfully -- the RNG only picks
        // *which* one.
        let total_child_resource: f32 = NEIGHBOURS_4
            .iter()
            .map(|&(dx, dy)| w.get(50 + dx, 50 + dy))
            .filter(|c| c.organism_id() == organism_id)
            .map(|c| organism::unpack_aux(c.aux()).1)
            .sum();

        assert!(
            (parent_resource - (start_resource - 1.0)).abs() < 0.02,
            "the parent should have paid the division's cost: expected ~{}, got {parent_resource}",
            start_resource - 1.0
        );
        assert!(
            total_child_resource < 0.02,
            "a freshly divided cell should start at 0 resource, not inherit any: got {total_child_resource}"
        );

        std::fs::remove_dir_all(&material_dir).ok();
        std::fs::remove_dir_all(&species_dir).ok();
    }

    #[test]
    fn a_tip_leans_more_steeply_upward_when_lit_from_above() {
        // Architecture §6a regression: `tree_tip_tick`'s phototropism term
        // (`light_here` vs `light_above`, both read via `field_at_bilinear`
        // since this change) only ever nudges the growth direction's own
        // *y*-component. If the attractor pull is already purely vertical,
        // that nudge is invisible after normalization -- a purely-vertical
        // vector nudged further vertical is still purely vertical. So this
        // test deliberately gives the tip a single attractor up-and-to-one-
        // side, producing a growth direction with a real horizontal
        // component for the light-driven vertical nudge to compete against:
        // a lit tip should climb a little steeper (smaller x drift, more
        // negative y) than an otherwise-identical unlit one.
        //
        // Constructs `TreeState` directly rather than through
        // `plant_tree_seed` (whose 50 randomly scattered attractors would
        // add noise this test doesn't need) and calls `tree_tip_tick`
        // directly for one tick rather than running the scheduler, so the
        // only difference between the two worlds is the light field.
        fn tick_once_and_get_new_pos(w: &mut World, lit: bool) -> (f32, f32) {
            let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
            let (x, y) = (100, 150);
            w.set(x, y, Cell::new(wood, 0));
            let tip = Tip { pos: (x as f32, y as f32), dir: (0.0, -1.0), channel: 0.5, alive: true };
            let root = RootTip { pos: (x as f32, y as f32), dir: (0.0, 1.0), ticks_since_prime: 0, starved_ticks: 0, resting_on_wood: true, alive: true };
            let state = TreeState {
                attractors: vec![(x as f32 + 8.0, y as f32 - 8.0)], // up-and-right, within INFLUENCE_RADIUS
                tips: vec![tip],
                roots: vec![root],
                energy: 9999.0, // never gated on GROWTH_COST
                wood,
            };
            let tree_id = w.push_tree(state);
            if lit {
                // Both probes (`(x, y)` and `(x, y - 4)`) fall inside the
                // same `FIELD_SCALE`-wide field block (144..=151 spans both
                // 146 and 150), so a plain `field_at` would read them
                // identically -- exactly the degenerate case §6a fixes.
                // `field_at_bilinear` blends each toward the block *below*
                // it, weighted by how far into the block it sits: `y - 4`
                // (closer to this block's own top) leans toward this lit
                // block, `y` itself (closer to the block below) leans
                // toward the unlit one below it, so lighting only this one
                // block already gives `light_above > light_here`. A radius
                // smaller than `FIELD_SCALE` paints exactly one field cell.
                w.add_light(x, y - 3, 1, 5.0);
            }
            tree_tip_tick(w, x, y, tree_id, 0);
            w.tree(tree_id).tips[0].pos
        }

        let mut unlit = test_world();
        let unlit_pos = tick_once_and_get_new_pos(&mut unlit, false);
        let mut lit = test_world();
        let lit_pos = tick_once_and_get_new_pos(&mut lit, true);

        // Setup check: the lit world's tip should actually have seen a
        // brighter reading above it than at its own position, or the rest
        // of this test is meaningless.
        let light_here = lit.field_at_bilinear(100.0, 150.0).light;
        let light_above = lit.field_at_bilinear(100.0, 146.0).light;
        assert!(
            light_above > light_here,
            "test setup should have made the field above the tip brighter than at the tip: \
             here={light_here}, above={light_above}"
        );

        assert!(
            lit_pos.1 < unlit_pos.1,
            "a lit tip should climb higher (smaller y) in one tick than an unlit one: lit={lit_pos:?}, unlit={unlit_pos:?}"
        );
        assert!(
            lit_pos.0 < unlit_pos.0,
            "a lit tip's extra vertical pull should also reduce its horizontal drift: lit={lit_pos:?}, unlit={unlit_pos:?}"
        );
    }

    #[test]
    fn a_tip_leans_downwind_of_a_steady_breeze() {
        // Architecture §5d: "a vector field explosions write into, and
        // vegetation that does not know it exists." A single attractor
        // straight up (no horizontal pull of its own, same trick as the
        // phototropism test above) means any horizontal drift in the
        // result can only have come from wind.
        //
        // Unlike light, there's no `add_light`-style direct paint for
        // velocity -- it only ever comes from the field solver actually
        // running (`add_pressure_impulse` + enough `field::step` calls for
        // the resulting outward flow to reach the tip), so this needs a
        // real pressure impulse upwind of the tip rather than a one-line
        // field write.
        fn tick_once_and_get_new_pos(w: &mut World, windy: bool) -> (f32, f32) {
            let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
            let (x, y) = (150, 150);
            w.set(x, y, Cell::new(wood, 0));
            let tip = Tip { pos: (x as f32, y as f32), dir: (0.0, -1.0), channel: 0.5, alive: true };
            let root = RootTip { pos: (x as f32, y as f32), dir: (0.0, 1.0), ticks_since_prime: 0, starved_ticks: 0, resting_on_wood: true, alive: true };
            let state = TreeState {
                attractors: vec![(x as f32, y as f32 - 8.0)], // straight up, no horizontal pull
                tips: vec![tip],
                roots: vec![root],
                energy: 9999.0,
                wood,
            };
            let tree_id = w.push_tree(state);
            if windy {
                // Upwind of the tip, re-applied every step rather than
                // fired once -- a single impulse radiates outward as a
                // wave that passes, reflects and oscillates (independent
                // review measured the tip's own vx crossing back to
                // *negative* by the time a one-shot version of this test
                // reached step 27), which is a shockwave, not a steady
                // breeze. Continuous small forcing instead keeps driving
                // flow in the same direction every step, the way an actual
                // prevailing wind would, and stays positive far more
                // reliably than picking one lucky step count out of a
                // decaying oscillation ever could.
                for _ in 0..20 {
                    w.add_pressure_impulse(x - 40, y, 6, 20.0);
                    field::step(w);
                }
            }
            tree_tip_tick(w, x, y, tree_id, 0);
            w.tree(tree_id).tips[0].pos
        }

        let mut calm = test_world();
        let calm_pos = tick_once_and_get_new_pos(&mut calm, false);
        let mut windy = test_world();
        let windy_pos = tick_once_and_get_new_pos(&mut windy, true);

        let vx = windy.field_at_bilinear(150.0, 150.0).vx;
        assert!(vx > 0.01, "test setup should have produced real rightward wind at the tip: vx={vx}");
        assert!(
            (calm_pos.0 - 150.0).abs() < 0.01,
            "a straight-up attractor with no wind should climb with zero horizontal drift: calm={calm_pos:?}"
        );
        assert!(
            windy_pos.0 > calm_pos.0,
            "a tip in a rightward breeze should drift further right than an otherwise-identical calm one: \
             calm={calm_pos:?}, windy={windy_pos:?}"
        );
    }

    #[test]
    fn two_trees_grown_from_the_same_seed_differ() {
        // Two separate `World`s planting at the *same* position would draw
        // identical attractor scatters and grow identically -- `World::new`
        // always starts `Rng::default()` from the same fixed seed, so two
        // structurally-identical runs are, correctly, reproducible rather
        // than randomly different. That's a property of the RNG, not a
        // bug (see `rng.rs`'s own module doc on why determinism was never
        // required but reproducibility is free when nothing disturbs it).
        // The plan's actual claim -- "two trees grown from the same seed
        // differ" -- means within one running session, where the shared
        // `world.rng` has already advanced by the time a second tree is
        // planted. Plant both in the same world, far enough apart not to
        // compete for the same attractors, and compare shapes normalized
        // to each tree's own seed position.
        let mut w = test_world();
        w.plant_tree(50, 100);
        w.plant_tree(150, 100);
        run(&mut w, 3000);

        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let footprint_relative_to = |w: &World, origin: (i32, i32)| -> Vec<(i32, i32)> {
            let bounds = w.bounds().unwrap();
            let mut cells = Vec::new();
            for y in bounds.min_y..=bounds.max_y {
                for x in bounds.min_x..=bounds.max_x {
                    if w.get(x, y).material == wood && (x - origin.0).abs() < 40 {
                        cells.push((x - origin.0, y - origin.1));
                    }
                }
            }
            cells
        };
        assert_ne!(
            footprint_relative_to(&w, (50, 100)),
            footprint_relative_to(&w, (150, 100)),
            "two trees grown from the same seed position produced identical shapes"
        );
    }

    #[test]
    fn a_settled_world_with_a_growing_tree_still_sleeps_between_growth_ticks() {
        let mut w = test_world();
        w.plant_tree(50, 100);
        // A handful of frames is enough for the CA sweep itself to settle
        // (a single static wood cell has nothing to move); the tree keeps
        // growing on its own, much slower schedule.
        run(&mut w, 5);
        assert_eq!(
            w.active_chunk_count(),
            0,
            "a static plant cell should not keep the CA sweep's chunks awake"
        );
        assert!(w.active_site_count() > 0, "the tree should still have pending growth ticks");
    }

    #[test]
    fn a_tree_eventually_stops_growing() {
        let mut w = test_world();
        w.plant_tree(50, 100);
        run(&mut w, 6000);
        assert_eq!(
            w.active_site_count(),
            0,
            "a tree should eventually exhaust its attractors/energy and stop scheduling growth"
        );
        let wood = w.materials.id_of("wood").unwrap();
        assert!(count(&w, wood) > 1, "the tree should have grown at least beyond its single seed cell");
    }

    #[test]
    fn roots_consume_adjacent_water() {
        let mut w = test_world();
        w.plant_tree(50, 100);
        // A walled, floored tank directly below the seed -- water is a
        // liquid and falls/drains under gravity like anything else, so an
        // unconfined puddle sinks or spreads away before a root (which
        // only moves once every `ROOT_TICK_INTERVAL` frames) ever reaches
        // it. The tank keeps it exactly where the root's initial
        // straight-down growth will hit it.
        for x in 44..56 {
            w.set(x, 110, Cell::new(material::STONE, 0)); // floor
        }
        for y in 101..111 {
            w.set(44, y, Cell::new(material::STONE, 0)); // walls
            w.set(55, y, Cell::new(material::STONE, 0));
        }
        for x in 46..54 {
            for y in 102..109 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        let water_before = count(&w, material::WATER);
        run(&mut w, 500);
        let water_after = count(&w, material::WATER);
        assert!(water_after < water_before, "roots never consumed any adjacent water");
    }

    #[test]
    fn a_planted_tree_is_flammable_and_burns_to_ash() {
        let mut w = test_world();
        w.plant_tree(50, 100);
        run(&mut w, 300); // let it grow a little first
        let wood = w.materials.id_of("wood").unwrap();
        assert!(count(&w, wood) > 0, "test setup should have some wood to ignite");
        w.ignite_circle(50, 100, 5);
        run(&mut w, 4000);
        let ash = w.materials.id_of("ash").unwrap();
        assert!(count(&w, ash) > 0, "burned wood should have left ash behind");
    }

    #[test]
    fn roots_steer_toward_off_axis_water_via_hydrotropism() {
        // A regression an independent review flagged: the previous version
        // of this test placed water directly beneath the seed, on the
        // root's default straight-down gravitropic path -- it would have
        // passed identically even with the whole MIZ1 hydrotropism switch
        // deleted, since gravity alone reaches straight-down water. This
        // one places water only off to one side, where gravity alone would
        // never lead, so a measurable horizontal deviation can only be
        // explained by the hydrotropism steering actually firing.
        let mut w = test_world();
        w.plant_tree(50, 100);
        // A walled, floored tank whose *open interior* the root falls
        // straight down into (walls at x=47/63 don't block x=50, which
        // sits between them), but whose water is offset to the right
        // (x=52..60) rather than directly below the seed -- close enough
        // for the moisture field's own diffusion to carry a real gradient
        // over to x=50 by the time the root's descent reaches the water's
        // depth, without gravity alone ever carrying it there.
        for x in 47..63 {
            w.set(x, 115, Cell::new(material::STONE, 0)); // floor
        }
        for y in 101..115 {
            w.set(47, y, Cell::new(material::STONE, 0)); // walls
            w.set(62, y, Cell::new(material::STONE, 0));
        }
        for x in 52..60 {
            for y in 102..112 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        run_with_fields(&mut w, 1500);

        // Tree id 0: the first (only) tree planted into a fresh `World`.
        let root_x_positions: Vec<f32> = w.tree(0).roots.iter().map(|r| r.pos.0).collect();
        let max_rightward_reach = root_x_positions.iter().cloned().fold(f32::MIN, f32::max);
        assert!(
            max_rightward_reach > 51.0,
            "no root deviated toward the off-axis water pocket (furthest reach: {max_rightward_reach}, seed at x=50)"
        );
    }

    #[test]
    fn a_tree_can_produce_multiple_simultaneous_tips_via_branching() {
        // Direct internal-state check, not a visual proxy -- `tests` is a
        // child module of `plant`, so `TreeState`'s private fields are
        // visible here despite not being `pub`. A dense, wide attractor
        // field and a long run give branching (gated on `channel >
        // BRANCH_CHANNEL_THRESHOLD` and `nearby.len() >=
        // BRANCH_MIN_ATTRACTORS`, see `tree_tip_tick`) every realistic
        // chance to fire; if a future change silently broke it (an
        // off-by-one in the threshold, the attractor-count gate never
        // satisfiable), this is what would actually catch that.
        let mut w = test_world();
        w.plant_tree(100, 150);
        run(&mut w, 6000);

        // Tree id 0: the first (only) tree planted into a fresh `World`.
        let tip_count = w.tree(0).tips.len();
        assert!(
            tip_count > 1,
            "a tree grown to completion with abundant attractors never branched (ended with {tip_count} tip(s))"
        );
    }

    #[test]
    fn an_orphaned_tree_tip_stops_growing_instead_of_extending_wood_from_open_air() {
        // Regression (pixel-physics-issues.md #9): tree_tip_tick checked
        // only its own `alive` flag, which `alive` false is set only by the
        // tip's own logic (channel decayed past MIN_CHANNEL) -- never by
        // anything happening *to* it. Burn the trunk, or erase it with the
        // brush, and every tip kept its schedule: still accruing energy,
        // still consulting attractors, still writing wood from a position
        // now disconnected from anything. Direct internal-state
        // construction, same as `a_tree_can_produce_multiple_simultaneous_
        // tips_via_branching` above -- `tests` is a child module of `plant`.
        let mut w = test_world();
        let wood_id = w.materials.id_of("wood").unwrap();
        w.set(50, 50, Cell::new(wood_id, 0));
        let tip = Tip { pos: (50.0, 50.0), dir: (0.0, -1.0), channel: 0.5, alive: true };
        let state = TreeState { attractors: Vec::new(), tips: vec![tip], roots: Vec::new(), energy: 1000.0, wood: wood_id };
        let tree_id = w.push_tree(state);

        // Erase the trunk out from under the tip -- standing in for either
        // fire consuming it or the brush erasing it; the effect on the tip
        // is identical either way, since both just leave the cell no
        // longer holding this tree's wood.
        w.set(50, 50, Cell::EMPTY);

        let produced = tree_tip_tick(&mut w, 50, 50, tree_id, 0);

        assert!(produced.is_empty(), "an orphaned tip should not reschedule itself");
        assert!(!w.tree(tree_id).tips[0].alive, "an orphaned tip should mark itself dead, not keep ticking forever");
        assert_eq!(w.get(50, 50).material, material::EMPTY, "the dead tip must not have written new wood from open air");
    }

    #[test]
    fn an_orphaned_root_tip_stops_growing_but_a_root_resting_in_drunk_water_is_unaffected() {
        // Same regression as the tree-tip test above, applied to roots --
        // with the added wrinkle roots have and tree tips don't: a root's
        // own last-written cell is only *sometimes* wood
        // (`RootTip::resting_on_wood`). This test checks both halves: an
        // orphaned wood-resting root actually dies, and a root that simply
        // drank water at its current position (a legitimately `Empty`
        // cell, not a disturbance) is correctly left alone.
        let mut w = test_world();
        let wood_id = w.materials.id_of("wood").unwrap();

        w.set(50, 50, Cell::new(wood_id, 0));
        let wood_root = RootTip { pos: (50.0, 50.0), dir: (0.0, 1.0), ticks_since_prime: 0, starved_ticks: 0, resting_on_wood: true, alive: true };
        // (60, 50) stays Empty -- standing in for a root that just drank an
        // adjacent water cell and advanced into the now-empty space.
        let drunk_root = RootTip { pos: (60.0, 50.0), dir: (0.0, 1.0), ticks_since_prime: 0, starved_ticks: 0, resting_on_wood: false, alive: true };
        let state = TreeState {
            attractors: Vec::new(),
            tips: Vec::new(),
            roots: vec![wood_root, drunk_root],
            energy: 1000.0,
            wood: wood_id,
        };
        let tree_id = w.push_tree(state);

        // Erase the wood-resting root's trunk -- (60, 50) is left exactly
        // as it already was (Empty), matching the drink-then-advance case.
        w.set(50, 50, Cell::EMPTY);

        let wood_root_result = root_tip_tick(&mut w, 50, 50, tree_id, 0);
        assert!(wood_root_result.is_empty(), "an orphaned wood-resting root should not reschedule itself");
        assert!(!w.tree(tree_id).roots[0].alive, "an orphaned wood-resting root should mark itself dead");

        // The drunk-water root must be judged on its own merits (whether it
        // can find somewhere to grow from (60, 50)), not killed outright
        // just because its own cell reads as Empty -- that is its expected,
        // undisturbed state.
        let before_alive = w.tree(tree_id).roots[1].alive;
        let _ = root_tip_tick(&mut w, 60, 50, tree_id, 1);
        assert_eq!(
            w.tree(tree_id).roots[1].alive, before_alive,
            "a root resting in a cell it legitimately left empty by drinking must not be killed by the orphan check alone"
        );
    }

    #[test]
    fn a_fully_dead_trees_attractors_are_reclaimed_but_not_a_partially_dead_ones() {
        // Interim mitigation for pixel-physics-issues.md issue #8:
        // TreeState never shrinks (tips/roots index into it by position,
        // and reassigning a slot would invalidate a live ActiveSite -- the
        // real fix is generational indices, not attempted here), but its
        // `attractors` list (up to ATTRACTOR_COUNT points, by far the
        // largest part of TreeState) can be freed the moment every tip and
        // root has died, since nothing will ever consult it again.
        let mut w = test_world();
        let wood_id = w.materials.id_of("wood").unwrap();
        let attractors = vec![(0.0, 0.0); ATTRACTOR_COUNT];
        let alive_tip = Tip { pos: (0.0, 0.0), dir: (0.0, -1.0), channel: 0.5, alive: true };
        let dead_tip = Tip { pos: (0.0, 0.0), dir: (0.0, -1.0), channel: 0.0, alive: false };
        let dead_root =
            RootTip { pos: (0.0, 0.0), dir: (0.0, 1.0), ticks_since_prime: 0, starved_ticks: 0, resting_on_wood: true, alive: false };
        let state = TreeState { attractors, tips: vec![alive_tip, dead_tip], roots: vec![dead_root], energy: 0.0, wood: wood_id };
        let tree_id = w.push_tree(state);

        reclaim_if_tree_is_fully_dead(&mut w, tree_id);
        assert!(!w.tree(tree_id).attractors.is_empty(), "a tree with a still-living tip should not have its attractors reclaimed yet");

        w.tree_mut(tree_id).tips[0].alive = false; // the last living tip dies
        reclaim_if_tree_is_fully_dead(&mut w, tree_id);
        assert!(w.tree(tree_id).attractors.is_empty(), "a fully dead tree's attractors should have been reclaimed");
    }

    // --- Self-blocking light regression (`ambient_light_above`) ----------
    //
    // `rebuild_blocked` marks a whole field block opaque the instant any
    // `Solid`/`Plant` cell sits inside it, so a plant cell reading
    // `field_at` at its own exact position always reads a self-blocked
    // `0.0`, forever, regardless of how bright the sky is one cell away.
    // Never caught before this: moss's own light read only ever scales a
    // probability that fires either way, so a silently-always-shaded
    // reading still looked plausible. `Germinate`/`Photosynthesize` are the
    // first behaviors to treat a light reading as a hard gate/income
    // source, which turns the same self-blocking into a permanent deadlock.
    // These tests plant close enough to the sky (`field.rs`'s light model
    // decays hard within a few field rows) that light genuinely reaches the
    // cell from outside, and would fail if either behavior went back to
    // reading `world.field_at(x, y)` directly at its own position.

    #[test]
    fn a_seed_germinates_in_open_sky_despite_its_own_position_self_blocking_light() {
        let mut w = test_world();
        // `ambient_light_above` reads `FIELD_SCALE` (8) world units above
        // its own position -- y=20 keeps that offset read (y=12) safely
        // in-bounds and within light's real reach, unlike a too-shallow
        // planting depth (y < FIELD_SCALE) whose offset read would go
        // negative and land out of the world entirely, which reads as
        // blocked/zero regardless of the fix under test.
        w.plant_tree_v2(100, 20);
        run_with_fields(&mut w, 400); // several germination checks (ORGANISM_TICK_INTERVAL apart)

        let (cell_type, _) = organism::unpack_aux(w.get(100, 20).aux());
        assert_ne!(cell_type, Some(CellType::Seed), "a seed in open sky should have germinated, not stayed a Seed forever");
    }

    #[test]
    fn photosynthesize_gains_resource_in_open_sky_despite_its_own_position_self_blocking_light() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let wood = w.materials.id_of("wood").unwrap();
        let organism_id = w.push_organism(tree_species);
        let aux = organism::pack_aux(CellType::GrowingTip, 0.0);
        w.set(100, 20, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(aux));
        let site = reschedule_organism(100, 20, organism_id, 0, w.frame + ORGANISM_TICK_INTERVAL);
        w.schedule_active_site(site);

        run_with_fields(&mut w, (ORGANISM_TICK_INTERVAL as usize) * 3);

        let (_, resource) = organism::unpack_aux(w.get(100, 20).aux());
        assert!(resource > 0.0, "Photosynthesize should have accumulated resource from real sky light, got {resource}");
    }

    // --- Self-avoidance crowding regression (`candidate_crowding`) -------
    //
    // `organism::diffuse_resource` is a no-op for `organism_id() == 0`
    // (mirroring `diffuse_heat`'s own early return for thermally-inert
    // material), so canopy density never diffuses *into* an empty cell.
    // `Grow`'s candidate loop originally read `canopy_density` at the
    // candidate cell's own `aux` -- always empty, so always exactly `0.0`,
    // no matter how densely it was actually surrounded, which silently
    // turned `crowding_weight` into a no-op. `Reports/tree-rewrite-
    // design.md` §2b's own "deposit -> diffuse -> decay -> follow" self-
    // avoidance mechanism -- one of the four originally-blocking findings
    // from this design's first independent review -- was verified on paper
    // by two separate review rounds, neither of which caught that the
    // "follow" step queried the wrong side of the occupied/empty boundary.

    #[test]
    fn candidate_crowding_reads_density_from_the_candidates_own_occupied_neighbours_not_its_own_always_empty_cell() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let wood = w.materials.id_of("wood").unwrap();
        let organism_id = w.push_organism(tree_species);
        // A same-organism neighbour immediately left of the candidate,
        // carrying a real deposited canopy density.
        let neighbour_aux = organism::with_canopy_density(organism::pack_aux(CellType::GrowingTip, 0.0), 2.0);
        w.set(49, 50, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(neighbour_aux));
        // The candidate itself is empty -- reading its own aux directly
        // (the bug this guards against) would always read exactly 0.0.
        assert!(w.is_empty(50, 50));

        let density = candidate_crowding(&w, 50, 50, organism_id);
        assert!(density > 0.0, "candidate_crowding should see the neighbour's deposited density, not the always-empty candidate's own aux, got {density}");
    }

    #[test]
    fn candidate_crowding_ignores_a_different_organisms_density() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let wood = w.materials.id_of("wood").unwrap();
        let this_organism = w.push_organism(tree_species);
        let other_organism = w.push_organism(tree_species);
        let neighbour_aux = organism::with_canopy_density(organism::pack_aux(CellType::GrowingTip, 0.0), 3.0);
        w.set(49, 50, Cell::new(wood, 0).with_organism_id(other_organism).with_aux(neighbour_aux));

        let density = candidate_crowding(&w, 50, 50, this_organism);
        assert_eq!(density, 0.0, "a different organism's canopy density should not count as this organism's own crowding");
    }

    // --- Decay-cadence regression (`CANOPY_DENSITY_DECAY_PER_TICK`) ------
    //
    // Decay used to apply once per CA frame, inside `organism::diffuse_
    // resource` -- fast enough to erase a fresh deposit within about ten
    // frames, long before a neighbour's own next `Grow` check
    // (`ORGANISM_TICK_INTERVAL`, 45 frames, on average) ever read it. Moved
    // to `organism_tick`'s own per-cell cadence instead.

    #[test]
    fn a_fresh_deposit_survives_one_organism_tick_cycle_on_a_neighbour() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let wood = w.materials.id_of("wood").unwrap();
        let organism_id = w.push_organism(tree_species);
        let aux = organism::with_canopy_density(organism::pack_aux(CellType::MatureBody, 1.0), GROW_CANOPY_DEPOSIT);
        w.set(100, 20, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(aux));

        // One full organism_tick cycle on this cell itself -- the same
        // cadence a neighbour's own Grow check would be running on.
        let _ = organism_tick(&mut w, 100, 20, organism_id, 0);

        let density = organism::canopy_density(w.get(100, 20).aux());
        assert!(density > 0.0, "a fresh deposit should still be visible to a neighbour after one organism_tick cycle, got {density}");
    }

    #[test]
    fn canopy_density_decays_across_organism_tick_cycles_without_getting_stuck() {
        // Guards the same quantization-lock bug class the old per-frame
        // decay placement's own history already found once
        // (`CANOPY_DENSITY_DECAY_PER_TICK`'s own doc) -- moved to a new
        // location, so the same "doesn't stay permanently stuck at the
        // packed maximum" property needs to hold here too.
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let wood = w.materials.id_of("wood").unwrap();
        let organism_id = w.push_organism(tree_species);
        let aux = organism::with_canopy_density(organism::pack_aux(CellType::MatureBody, 1.0), organism::CANOPY_DENSITY_SCALE);
        w.set(100, 20, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(aux));

        for _ in 0..20 {
            let _ = organism_tick(&mut w, 100, 20, organism_id, 0);
        }

        let density = organism::canopy_density(w.get(100, 20).aux());
        assert!(
            density < organism::CANOPY_DENSITY_SCALE,
            "density should decay across repeated organism_tick calls, not stay stuck at the packed maximum, got {density}"
        );
    }

    // --- Tip-retirement regression ----------------------------------------
    //
    // A `GrowingTip` that successfully grew a child used to stay exactly as
    // eligible to grow *another*, unrelated child from the same position
    // next cycle -- live-verification logs showed 78% of all `Grow`-
    // evaluated positions revisited 3+ times each, radiating growth from a
    // small set of static hub points instead of tips advancing outward,
    // which is what actually produced a dense round clump instead of
    // anything reading as a tree. A `GrowingTip` should retire to
    // `MatureBody` the instant it successfully grows; the newly created
    // child carries the frontier forward instead.

    #[test]
    fn a_growing_tip_retires_to_mature_body_after_successfully_growing() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let wood = w.materials.id_of("wood").unwrap();
        let organism_id = w.push_organism(tree_species);
        // Comfortably above tree.ron's GrowingTip `Grow` cost (0.4), in
        // open space on every side, so this call is guaranteed to find a
        // positive-scoring candidate and actually grow.
        let aux = organism::pack_aux(CellType::GrowingTip, 2.0);
        w.set(100, 100, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(aux));

        let next = organism_tick(&mut w, 100, 100, organism_id, 0);

        let (self_type, _) = organism::unpack_aux(w.get(100, 100).aux());
        assert_eq!(self_type, Some(CellType::MatureBody), "a GrowingTip that just grew should retire to MatureBody, not stay an equally-eligible growth candidate");

        // Exactly one newly created cell nearby should carry the frontier
        // forward as the new active GrowingTip.
        let new_tips: Vec<(i32, i32)> = NEIGHBOURS_8
            .iter()
            .map(|&(dx, dy)| (100 + dx, 100 + dy))
            .filter(|&(nx, ny)| organism::unpack_aux(w.get(nx, ny).aux()).0 == Some(CellType::GrowingTip))
            .collect();
        assert_eq!(new_tips.len(), 1, "expected exactly one new GrowingTip child, got {new_tips:?}");
        assert!(!next.is_empty(), "the new child should be scheduled");
    }
}
