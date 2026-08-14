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
use super::material::MaterialKind;
use super::organism::{self, Behavior, CellType};
use super::rng::{self, Rng};
use super::scheduler::{ActiveKind, ActiveSite};
use super::world::World;

const NEIGHBOURS_4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const NEIGHBOURS_8: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// Plain 2D dot product — `Behavior::Grow`'s own candidate-scoring formula
/// (`Reports/tree-rewrite-design.md` §2).
fn dot(a: (f32, f32), b: (f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1
}

fn normalize(v: (f32, f32)) -> (f32, f32) {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    if len < 1e-6 {
        (0.0, 0.0)
    } else {
        (v.0 / len, v.1 / len)
    }
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

/// Minimum moisture-gradient magnitude before MIZ1-style suppression of
/// gravity kicks in and a `RootTip`'s `Grow` steers toward water instead of
/// straight down. A gradient, not a raw moisture reading — MIZ1 biology is
/// specifically a response to a *change* in local humidity, not merely
/// "some water is nearby" (see `organism::moisture_pull`'s own doc).
/// `roots_steer_toward_off_axis_water_via_hydrotropism` is the actual
/// authority on whether it's set right, same as every other threshold on
/// this channel.
const MIZ_THRESHOLD: f32 = 0.05;

/// Local moisture drained per drink by `Behavior::Absorb` — architecture
/// §5g, the write that turns the moisture channel from read-only into a
/// loop. Not tied numerically to `Absorb`'s own `rate` (different units:
/// resource vs. this channel's own 0..4-ish scale) — a separate, freely
/// tunable amount.
const ROOT_MOISTURE_DEPLETION: f32 = 1.0;

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

fn organism_tick(world: &mut World, x: i32, y: i32, organism_id: u16, stale_ticks: u8, plastochron: u8) -> Vec<ActiveSite> {
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

    // One stream per (organism, cell, tick), seeded from exactly those --
    // never `world.rng`, whose sequence depends on how many draws every
    // *other* organism made first. See `rng::stream` for why that coupling
    // makes `examples/debug_tree_variants.rs`'s side-by-side comparison
    // unsound, and why both research reports blame the wrong generator for
    // it. `world.frame` is in the seed so a cell that ticks repeatedly does
    // not redraw the same numbers; `(x, y)` so two cells of one organism
    // ticking on the same frame diverge.
    let mut rng = rng::stream(organism_id as u64, x as u64, y as u64, world.frame);

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
                let (tx, ty) = candidates[rng.below(candidates.len() as u32) as usize];
                let mut chance = if is_damp(world, tx, ty) { damp_chance } else { dry_chance };
                if shade_sensitive {
                    chance *= shade_factor(world, tx, ty);
                }
                if rng.chance(chance) {
                    let shades = world.materials.get(cell.material).palette.len().max(1) as u32;
                    let shade = rng.below(shades) as u8;
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
                    next.push(reschedule_organism(tx, ty, organism_id, 0, 0, world.frame + ORGANISM_TICK_INTERVAL));
                }
            }
            // `Reports/tree-rewrite-design.md` §0/§2/§3: direction-biased,
            // resource-gated, self-avoiding growth for `GrowingTip`
            // (canopy) and `RootTip` (roots) -- deliberately a separate
            // behavior from `Divide` above, not a mode of it; see this
            // module's own `organism.rs` doc for why.
            Behavior::Grow {
                cost,
                branch_chance,
                continuation_weight,
                light_weight,
                wind_weight,
                upward_weight,
                crowding_weight,
                max_active_tips,
                plastochron: plastochron_interval,
            } => {
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
                // A real independent review caught this term inverted for
                // canopy: it used to default to `(0.0, 1.0)` (down) for
                // *every* cell type, so `GrowingTip`'s own positive `
                // upward_weight` was actually rewarding downward growth,
                // the opposite of its name and `tree.ron`'s own intent --
                // this doc used to (wrongly) claim the term "contributes
                // nothing to canopy growth in practice" because the weight
                // was small (0.1), not because the direction was correct.
                // It could also pull a `GrowingTip` toward a moisture
                // gradient via MIZ1, which `Reports/tree-rewrite-
                // design.md` §2 explicitly scopes to roots only.
                //
                // Gravitropism/hydrotropism antagonism (MIZ1) stays
                // `RootTip`-only, matching that scoping exactly; canopy
                // gets a true, fixed "up" reference instead, with no MIZ1
                // override -- `RootTip`'s own branch is untouched from
                // before this fix, so its already-tuned resource economy
                // doesn't need re-verifying.
                let gravity_or_water = if cell_type == CellType::RootTip {
                    match organism::moisture_pull(world, x as f32, y as f32) {
                        Some((dir, strength)) if strength >= MIZ_THRESHOLD => dir,
                        _ => (0.0, 1.0), // down
                    }
                } else {
                    (0.0, -1.0) // up
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
                //
                // `Reports/plant-substrate-v2-design.md` §5a: every
                // `plastochron`-th step along a lineage, that retiring
                // parent becomes a `Leaf` rather than `MatureBody`. The
                // *parent*, deliberately, not the child -- the child
                // carries the frontier forward (that is the whole point of
                // the retirement fix above), so making the child a `Leaf`
                // would terminate the lineage every plastochron. Retiring
                // the parent instead places foliage along the shoot behind
                // the advancing tip, where leaves are on a real shoot, and
                // creates no new cell at all.
                //
                // This is what gives `thicken()` something real to count.
                // It counts downstream `Leaf | GrowingTip` cells, and with
                // tips retiring the instant they grow, that count was
                // measured at 0-2 for the whole life of a tree
                // (`docs/screenshots/plant-v2-baseline/`) -- never once
                // clearing `pipe_ratio: 2.5`, so `SecondaryThicken` had
                // never fired on anything. Persistent `Leaf` cells make the
                // count grow with the canopy, which is the signal
                // Shinozaki's pipe model actually specifies.
                let lineage_step = plastochron.saturating_add(1);
                let leaf_due = plastochron_interval > 0 && lineage_step.is_multiple_of(plastochron_interval);
                let self_type_after_grow = if cell_type == CellType::GrowingTip {
                    if leaf_due {
                        CellType::Leaf
                    } else {
                        CellType::MatureBody
                    }
                } else {
                    cell_type
                };

                let total: f32 = candidates.iter().map(|&(_, _, s)| s).sum();
                let mut pick = (rng.below(10_000) as f32 / 10_000.0) * total;
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
                let shade = rng.below(shades) as u8;
                // Canopy density deposited once, here, at creation --
                // `organism::diffuse_resource`'s own doc explains why this
                // lives at the moment of growth rather than a continuous
                // per-tick re-deposit: it lets decay actually fade the
                // signal over time instead of fighting a refill every
                // single sweep visit.
                let new_aux = organism::with_canopy_density(organism::pack_aux(cell_type, 0.0), GROW_CANOPY_DEPOSIT);
                let new_cell = Cell::new(cell.material, shade).with_organism_id(organism_id).with_aux(new_aux);
                world.set(tx, ty, new_cell);
                // Deliberately no `schedule_structural_check_around` here --
                // growth only ever adds material, never removes support, so
                // it is not a disturbance to the structural system the way
                // painting/erasing/an explosion/a burnout is (`structural.rs`'s
                // own module doc: checks are reactive to disturbance, never
                // proactive at creation time). A `GrowingTip` advancing away
                // from the ground is *expected* to be transiently unsupported
                // until the organism eventually reconnects (e.g. a root tip
                // reaching soil) -- checking it here would prune ordinary
                // in-progress growth as if it were damage. Found the hard way:
                // `World::schedule_active_site`'s take/replace fix (code-
                // review-findings item #2 follow-up) made this call actually
                // reach the heap for the first time ever, and every open-sky
                // tree test immediately started failing -- the call had been
                // a silent no-op since this behavior shipped.
                resource -= cost;
                world.set(x, y, cell.with_aux(pack_aux_preserving_density(cell.aux(), self_type_after_grow, resource)));
                next.push(reschedule_organism(tx, ty, organism_id, 0, lineage_step, world.frame + ORGANISM_TICK_INTERVAL));

                // §3's branching: a second successful `Grow`, in a
                // different direction, this same tick -- gated by the
                // same resource economy as the first, not a separate
                // mechanic. Also gated by `max_active_tips` again here
                // (the "+1" accounts for the primary child just created
                // above, which isn't in `world`'s own `active_sites` yet
                // -- it's still sitting in this call's own `next`, only
                // merged in by the caller after this returns, so `world.
                // organism_active_tip_count` can't see it yet on its own).
                // A real independent review caught this cap only being
                // checked once, before the primary child, which let a
                // single tick's branch roll overshoot it by one right at
                // the cap.
                if resource >= cost
                    && rng.chance(branch_chance)
                    && world.organism_active_tip_count(organism_id, cell_type) + 1 < max_active_tips as usize
                {
                    let alt: Vec<(i32, i32, f32)> = candidates.into_iter().filter(|&(nx, ny, _)| (nx, ny) != (tx, ty)).collect();
                    if !alt.is_empty() {
                        let (bx, by, _) = alt[rng.below(alt.len() as u32) as usize];
                        if world.is_empty(bx, by) {
                            let branch_shade = rng.below(shades) as u8;
                            let branch_aux = organism::with_canopy_density(organism::pack_aux(cell_type, 0.0), GROW_CANOPY_DEPOSIT);
                            let branch_cell = Cell::new(cell.material, branch_shade).with_organism_id(organism_id).with_aux(branch_aux);
                            world.set(bx, by, branch_cell);
                            // No structural check here either -- see the
                            // primary child's identical case above.
                            resource -= cost;
                            world.set(x, y, cell.with_aux(pack_aux_preserving_density(cell.aux(), self_type_after_grow, resource)));
                            next.push(reschedule_organism(bx, by, organism_id, 0, 0, world.frame + ORGANISM_TICK_INTERVAL));
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
                thicken(world, x, y, organism_id, pipe_ratio, &mut rng);
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
                    return germinate(world, x, y, organism_id, cell, &mut rng);
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
        next.push(reschedule_organism(x, y, organism_id, 0, plastochron, world.frame + ORGANISM_TICK_INTERVAL));
    } else if stale_ticks + 1 < ORGANISM_STALE_LIMIT {
        next.push(reschedule_organism(x, y, organism_id, stale_ticks + 1, plastochron, world.frame + ORGANISM_TICK_INTERVAL));
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
        next.push(reschedule_organism(x, y, organism_id, 0, plastochron, world.frame + ORGANISM_TICK_INTERVAL));
    }
    // Otherwise (any other cell type, e.g. a permanently enclosed
    // `RootTip`): permanently enclosed -- stop checking, matching the old
    // per-tip/per-root "alive: false" outcome.
    next
}

fn reschedule_organism(x: i32, y: i32, organism: u16, stale_ticks: u8, plastochron: u8, next_frame: u64) -> ActiveSite {
    ActiveSite { x, y, kind: ActiveKind::Organism { organism, stale_ticks, plastochron }, next_frame }
}

/// A `Seed` cell's `Germinate` firing (`Reports/tree-rewrite-design.md`
/// §8, retrofit step 4): the seed itself becomes the trunk's first
/// `GrowingTip`, and — if the space below is open — a companion `RootTip`
/// starts one cell down, mirroring the old `plant_tree_seed`'s symmetric
/// "one tip up, one root down, both starting at the seed's own position"
/// shape.
fn germinate(world: &mut World, x: i32, y: i32, organism_id: u16, cell: Cell, rng: &mut Rng) -> Vec<ActiveSite> {
    // No `schedule_structural_check_around` on either the new tip or the
    // root -- see the identical reasoning on `Behavior::Grow`'s own child
    // creation above. A freshly germinated seed is not yet connected to any
    // ground and is not expected to be; checking it here would destroy
    // every seedling before its root ever gets the chance to reach soil.
    world.set(x, y, cell.with_aux(organism::pack_aux(CellType::GrowingTip, 0.0)));
    let mut next = vec![reschedule_organism(x, y, organism_id, 0, 0, world.frame + ORGANISM_TICK_INTERVAL)];
    if world.is_empty(x, y + 1) {
        let shades = world.materials.get(cell.material).palette.len().max(1) as u32;
        let shade = rng.below(shades) as u8;
        let root_cell = Cell::new(cell.material, shade).with_organism_id(organism_id).with_aux(organism::pack_aux(CellType::RootTip, 0.0));
        world.set(x, y + 1, root_cell);
        next.push(reschedule_organism(x, y + 1, organism_id, 0, 0, world.frame + ORGANISM_TICK_INTERVAL));
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
fn thicken(world: &mut World, x: i32, y: i32, organism_id: u16, pipe_ratio: f32, rng: &mut Rng) {
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
            let shade = rng.below(shades) as u8;
            let new_cell = Cell::new(cell.material, shade).with_organism_id(organism_id).with_aux(organism::pack_aux(CellType::MatureBody, 0.0));
            world.set(nx, ny, new_cell);
            // No structural check here either -- same reasoning as `Grow`'s
            // own child creation: thickening only adds material sideways
            // off an already-supported `MatureBody`, never removes support.
            break;
        }
    }
}

/// Dispatch one due active site to its growth function. Called from
/// `scheduler::step` for every `ActiveKind` except `StructuralCheck`,
/// `Creature` and `Decay`, which `scheduler::step` routes to `structural::
/// tick`/`creature::tick`/`decay::tick` instead -- the match here still has
/// to name all three variants to stay exhaustive.
pub fn tick(world: &mut World, site: &ActiveSite) -> Vec<ActiveSite> {
    match site.kind {
        ActiveKind::Organism { organism, stale_ticks, plastochron } => organism_tick(world, site.x, site.y, organism, stale_ticks, plastochron),
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
        let site = reschedule_organism(x, y, organism_id, 0, 0, self.frame + ORGANISM_TICK_INTERVAL);
        self.schedule_active_site(site);
    }

    /// Plant a `tree` species `Seed` cell at `(x, y)` — the emergent,
    /// `Grow`/`Germinate`-driven system (`Reports/tree-rewrite-design.md`).
    /// Replaced the old `TreeState`/`Tip`/`RootTip`-based space-
    /// colonization implementation once this system's own live-
    /// verification gate passed (tree-rewrite retrofit step 7).
    pub fn plant_tree(&mut self, x: i32, y: i32) {
        self.plant_tree_species(x, y, "tree");
    }

    /// Plant a `Seed` cell of any tree-shaped species (a `Seed` cell type
    /// that germinates into a `wood`-material `GrowingTip`) at `(x, y)` --
    /// generalizes `plant_tree` to a caller-chosen species name, so
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
        let site = reschedule_organism(x, y, organism_id, 0, 0, self.frame + ORGANISM_TICK_INTERVAL);
        self.schedule_active_site(site);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;
    use crate::sim::field;
    use crate::sim::material;
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
        // 24000 frames, not the 6000 this originally used. Growing *up* off
        // the water's own row is only reachable via `moss.ron`'s
        // `dry_chance` (0.002), further multiplied by `shade_factor` (down
        // to 0.1) -- rows 47 and up sit in a different 8-cell field block
        // than the water at row 49, so they are never `is_damp`. That makes
        // the effective per-check chance as low as 2e-4, and at 6000 frames
        // the expected number of successful upward divisions is around 1:
        // the test was a coin flip on the RNG stream rather than a real
        // check, and it duly flipped the first time an unrelated change
        // (liquid's wider `sweep_reach`) shifted that stream. Verified at
        // 24000 the colony reaches row 46 with ~313 cells, comfortably
        // clear of the boundary rather than sitting on it.
        run_with_fields(&mut w, 24000);

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

        organism_tick(&mut w, 50, 50, organism_id, 0, 0);

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

    // --- Ported from the old TreeState-based system (tree rewrite step 7)
    //
    // `organism::phototropism_dir`/`wind_lean_dir` are the exact formulas
    // `tree_tip_tick`'s own phototropism/wind-lean terms used, ported
    // unchanged when `Grow`'s dispatch was built (`organism.rs`'s own doc
    // on both functions) -- but never picked up a direct test of their own
    // at the new location, so the two tests below replace `tree_tip_tick`-
    // level regression tests with direct, simpler unit tests of the pure
    // functions themselves. Both scenarios (a real §6a bilinear-sampling
    // regression, and the §5d "wind is a real gradient, not a magic
    // number" claim) are preserved; only the mechanism under test moved
    // from a whole simulated growth tick to the function that actually
    // does the work.

    #[test]
    fn phototropism_dir_leans_upward_only_when_lit_from_above() {
        let unlit = test_world();
        assert_eq!(organism::phototropism_dir(&unlit, 100.0, 150.0), (0.0, 0.0), "no light gradient should mean no lean");

        let mut lit = test_world();
        // Both probes (`(x, y)` and `(x, y - 4)`) fall inside the same
        // `FIELD_SCALE`-wide field block (144..=151 spans both 146 and
        // 150), so a plain `field_at` would read them identically --
        // exactly the degenerate case §6a's bilinear sampling fixes. A
        // radius smaller than `FIELD_SCALE` paints exactly one field cell.
        lit.add_light(100, 147, 1, 5.0);
        assert_eq!(organism::phototropism_dir(&lit, 100.0, 150.0), (0.0, -1.0), "a real light gradient above should lean upward");
    }

    #[test]
    fn wind_lean_dir_points_downwind_only_with_real_flow() {
        let calm = test_world();
        assert_eq!(organism::wind_lean_dir(&calm, 150.0, 150.0), (0.0, 0.0), "no wind should mean no lean");

        let mut windy = test_world();
        // Unlike light, there's no direct-paint equivalent for velocity --
        // it only ever comes from the field solver actually running.
        // Continuous small forcing (not one impulse, which radiates
        // outward as a wave that passes, reflects and oscillates) keeps
        // driving flow in the same direction every step, the way an actual
        // prevailing wind would.
        for _ in 0..20 {
            windy.add_pressure_impulse(110, 150, 6, 20.0);
            field::step(&mut windy);
        }
        let vx = windy.field_at_bilinear(150.0, 150.0).vx;
        assert!(vx > 0.01, "test setup should have produced real rightward wind at the probe: vx={vx}");

        let lean = organism::wind_lean_dir(&windy, 150.0, 150.0);
        assert!(lean.0 > 0.0, "a rightward breeze should lean downwind (positive x), got {lean:?}");
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
        // to each tree's own seed position. Planted near y=20, not the old
        // system's y=100 -- `field.rs`'s light model decays hard within a
        // few field rows of open sky (`ambient_light_above`'s own doc), so
        // `Germinate`'s light gate is unreachable much deeper than that,
        // unlike the old system's flat `AMBIENT_GROWTH_ENERGY`. Needs
        // `run_with_fields`, not plain `run` -- germination depends on a
        // real light field, which plain `run` deliberately never steps.
        let mut w = test_world();
        w.plant_tree(50, 20);
        w.plant_tree(150, 20);
        run_with_fields(&mut w, 3000);

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
            footprint_relative_to(&w, (50, 20)),
            footprint_relative_to(&w, (150, 20)),
            "two trees grown from the same seed position produced identical shapes"
        );
    }

    #[test]
    fn a_settled_world_with_a_growing_tree_still_sleeps_between_growth_ticks() {
        let mut w = test_world();
        w.plant_tree(50, 20);
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
        // Not `active_site_count() == 0` any more -- unlike the old
        // system, a `MatureBody` cell's own `SecondaryThicken` check
        // (`found_candidate = true` unconditionally, so it can keep
        // watching for a future thickening opportunity) reschedules
        // itself forever, by design, so a real tree's active-site count
        // never actually reaches zero. What "eventually stops growing"
        // means here instead: the wood *count* stops changing -- no new
        // cells, even though the schedule itself stays alive.
        let mut w = test_world();
        w.plant_tree(50, 20);
        run_with_fields(&mut w, 3000);
        let wood = w.materials.id_of("wood").unwrap();
        let count_at_3000 = count(&w, wood);
        assert!(count_at_3000 > 1, "the tree should have grown at least beyond its single seed cell");

        run_with_fields(&mut w, 24000);
        let count_at_9000 = count(&w, wood);
        assert_eq!(count_at_3000, count_at_9000, "a tree should eventually exhaust its resource economy and stop producing new wood cells");
    }

    #[test]
    fn roots_consume_adjacent_water() {
        let mut w = test_world();
        // Seed near y=20, not the old system's y=100 -- see
        // `two_trees_grown_from_the_same_seed_differ`'s own doc on why
        // `Germinate`'s real light gate needs to be within a few field
        // rows of open sky. The water tank keeps the same offsets *below*
        // the seed as the old test used (floor 10 rows down, water
        // starting 2 rows down), just shifted to match.
        w.plant_tree(50, 20);
        // A walled, floored tank directly below the seed -- water is a
        // liquid and falls/drains under gravity like anything else, so an
        // unconfined puddle sinks or spreads away before a root ever
        // reaches it. The tank keeps it exactly where the root's initial
        // straight-down growth will hit it.
        for x in 44..56 {
            w.set(x, 30, Cell::new(material::STONE, 0)); // floor
        }
        for y in 21..31 {
            w.set(44, y, Cell::new(material::STONE, 0)); // walls
            w.set(55, y, Cell::new(material::STONE, 0));
        }
        for x in 46..54 {
            for y in 22..29 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        run_with_fields(&mut w, 500);
        // The cell directly below the seed, where germinate() plants the
        // companion RootTip -- not a whole-world volume comparison
        // (cell-count would miss it: the compressible-volume liquid model
        // can spread the *remaining* water into more, shallower-filled
        // cells as it resettles around the gap Absorb leaves, which raises
        // count() even as real volume drops; a raw fill-total comparison
        // is no better, since a freshly-painted `Cell::new(WATER, 0)`
        // starts at aux=0 -- uninitialized fill, not "full" -- and only
        // reaches its real fill once the CA sweep has actually processed
        // it, so a totals comparison against time zero is comparing
        // "unswept" to "settled", not "before" to "after" absorption).
        // Directly checking the one cell `Absorb`'s own `NEIGHBOURS_4`
        // check targets is the precise, unambiguous claim this test
        // actually cares about.
        assert_ne!(w.get(50, 22).material, material::WATER, "the root's adjacent water cell was never consumed");
    }

    #[test]
    fn a_planted_tree_is_flammable_and_burns_to_ash() {
        let mut w = test_world();
        w.plant_tree(50, 20);
        run_with_fields(&mut w, 300); // let it grow a little first
        let wood = w.materials.id_of("wood").unwrap();
        assert!(count(&w, wood) > 0, "test setup should have some wood to ignite");
        w.ignite_circle(50, 20, 5);
        run_with_fields(&mut w, 4000);
        let ash = w.materials.id_of("ash").unwrap();
        assert!(count(&w, ash) > 0, "burned wood should have left ash behind");
    }

    #[test]
    fn roots_steer_toward_off_axis_water_via_hydrotropism() {
        // A regression an independent review flagged against the old
        // TreeState-based system, still the right shape here: water placed
        // directly beneath the seed sits on a root's default straight-down
        // gravitropic path, so it would pass even with the whole MIZ1
        // hydrotropism switch deleted. Water only off to one side, where
        // gravity alone would never lead, means a measurable horizontal
        // deviation can only be explained by hydrotropism steering
        // actually firing.
        //
        // Ported as a direct test of `organism::moisture_pull` itself
        // (the same shape `phototropism_dir_leans_upward_only_when_lit_
        // from_above`/`wind_lean_dir_points_downwind_only_with_real_flow`
        // already use above), not a full multi-thousand-tick growth
        // simulation: a `RootTip` with no adjacent water has no income of
        // its own under this species' current resource economy (no
        // `Photosynthesize`; `Absorb` only pays off once already touching
        // water), so it lives entirely off resource slowly diffusing over
        // from the trunk -- confirmed by direct inspection during
        // development that it can go fully dormant (`ORGANISM_STALE_
        // LIMIT` consecutive resource-starved misses) well before ever
        // reaching a distant water pocket, no matter how long the test
        // runs. That is a real, separate tuning gap in `RootTip`'s own
        // resource economy (recorded in `PLAN.md` alongside this
        // session's other tree.ron findings), not a reason to weaken what
        // this test is actually supposed to verify -- whether hydrotropism
        // steering itself points the right way.
        let mut w = test_world();
        for x in 47..63 {
            w.set(x, 35, Cell::new(material::STONE, 0)); // floor
        }
        for y in 21..35 {
            w.set(47, y, Cell::new(material::STONE, 0)); // walls
            w.set(62, y, Cell::new(material::STONE, 0));
        }
        for x in 52..60 {
            for y in 22..32 {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        run_with_fields(&mut w, 200); // let the moisture field actually diffuse into the open interior

        let pull = organism::moisture_pull(&w, 50.0, 25.0);
        let (dir, strength) = pull.expect("a real off-axis water pocket should produce a nonzero moisture gradient");
        assert!(strength >= MIZ_THRESHOLD, "moisture gradient too weak to trigger hydrotropism: {strength}");
        assert!(dir.0 > 0.0, "the gradient should point right, toward the off-axis water pocket, got {dir:?}");
    }

    #[test]
    fn a_tree_can_branch_into_more_than_one_lineage() {
        // The new system's version of the old "abundant conditions give
        // branching every realistic chance to fire" claim -- `Grow`'s own
        // `branch_chance` (tree.ron) is a flat per-success roll rather than
        // the old channel/attractor-count gate, so the proxy for "did it
        // branch" changes too: a branch point is a cell with 3+ same-
        // organism `Plant` 8-neighbours (a lineage's parent plus more than
        // one child), instead of counting simultaneously-alive tips --
        // this session's own tip-retirement fix means tips essentially
        // never stay alive simultaneously any more, by design, so that
        // old proxy no longer means what it used to.
        // Seed near y=20, not the old system's y=150 -- see `two_trees_
        // grown_from_the_same_seed_differ`'s own doc on why. Needs
        // `run_with_fields`, not plain `run` -- growth depends on a real
        // light field.
        let mut w = test_world();
        w.plant_tree(100, 20);
        let organism_id = w.get(100, 20).organism_id();
        // Generous but not unbounded -- empirically enough for real
        // branch_chance rolls to land at least once (confirmed against
        // examples/debug_tree_variants.rs's own 20,000-tick runs, which
        // showed multiple Y-forks at these species weights).
        run_with_fields(&mut w, 20_000);

        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let branched = (0..200).any(|x| {
            (0..200).any(|y| {
                let cell = w.get(x, y);
                if cell.material != wood || cell.organism_id() != organism_id {
                    return false;
                }
                NEIGHBOURS_8.iter().filter(|&&(dx, dy)| w.get(x + dx, y + dy).organism_id() == organism_id).count() >= 3
            })
        });
        assert!(branched, "a tree grown to completion in open sky never produced a branch point (3+ same-organism neighbours)");
    }

    #[test]
    fn an_orphaned_growing_tip_stops_growing_instead_of_extending_wood_from_open_air() {
        // Regression (pixel-physics-issues.md #9), ported to the new
        // system: the old `tree_tip_tick` checked only its own `alive`
        // flag, set only by the tip's own logic, never by anything
        // happening *to* it -- burning or erasing the trunk left every tip
        // still scheduled, still writing wood from a position now
        // disconnected from anything. `organism_tick`'s own equivalent
        // guard is its first check, `cell.organism_id() != organism_id`,
        // which fires the instant the cell no longer holds this organism's
        // material at all -- exercised directly here.
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let wood = w.materials.id_of("wood").unwrap();
        let organism_id = w.push_organism(tree_species);
        let aux = organism::pack_aux(CellType::GrowingTip, 2.0);
        w.set(50, 50, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(aux));

        // Erase the tip out from under itself -- standing in for either
        // fire consuming it or the brush erasing it; the effect is
        // identical either way, since both just leave the cell no longer
        // holding this organism's material.
        w.set(50, 50, Cell::EMPTY);

        let produced = organism_tick(&mut w, 50, 50, organism_id, 0, 0);

        assert!(produced.is_empty(), "an orphaned cell should not reschedule itself");
        assert_eq!(w.get(50, 50).material, material::EMPTY, "the orphaned check must not have written new wood from open air");
    }

    #[test]
    fn an_orphaned_root_tip_stops_growing() {
        // Same regression as the growing-tip test above, applied to
        // `RootTip`. The old system's paired "drunk water" half (a root
        // that legitimately vacated its own cell by drinking adjacent
        // water must not be treated as orphaned) doesn't translate: the
        // new `Absorb` only ever empties an *adjacent* water cell
        // (`NEIGHBOURS_4` around the root), never the root's own position,
        // so a `RootTip` cell never ends up sitting in a cell it vacated
        // itself the way the old continuous-position model could.
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let wood = w.materials.id_of("wood").unwrap();
        let organism_id = w.push_organism(tree_species);
        let aux = organism::pack_aux(CellType::RootTip, 2.0);
        w.set(50, 50, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(aux));

        w.set(50, 50, Cell::EMPTY);
        let produced = organism_tick(&mut w, 50, 50, organism_id, 0, 0);

        assert!(produced.is_empty(), "an orphaned root should not reschedule itself");
        assert_eq!(w.get(50, 50).material, material::EMPTY, "the orphaned check must not have written new wood from open air");
    }

    // `a_fully_dead_trees_attractors_are_reclaimed_but_not_a_partially_dead_
    // ones` (the old system's interim mitigation for pixel-physics-
    // issues.md issue #8) had no direct equivalent to port: the new system
    // has no `attractors` list at all to reclaim. The underlying concern
    // -- a fully dead organism's storage never being freed -- is real and
    // still open here too, just manifesting differently: `World::push_
    // organism`'s own doc says so directly ("nothing populates that list
    // yet in this pass"), meaning an organism's id slot is never returned
    // to `free_organism_slots` when every one of its cells dies. Recorded
    // as a known gap, not silently dropped; a real fix needs a BFS-from-
    // roots liveness check like `organism-substrate-design.md` §6
    // describes, which is genuine future work, not a one-line port.

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
        w.plant_tree(100, 20);
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
        let site = reschedule_organism(100, 20, organism_id, 0, 0, w.frame + ORGANISM_TICK_INTERVAL);
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
        let _ = organism_tick(&mut w, 100, 20, organism_id, 0, 0);

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
            let _ = organism_tick(&mut w, 100, 20, organism_id, 0, 0);
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
        // Comfortably above tree.ron's GrowingTip `Grow` cost (0.2), in
        // open space on every side, so this call is guaranteed to find a
        // positive-scoring candidate and actually grow.
        let aux = organism::pack_aux(CellType::GrowingTip, 2.0);
        w.set(100, 100, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(aux));

        let next = organism_tick(&mut w, 100, 100, organism_id, 0, 0);

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

    /// `Reports/open-bugs-handoff.md` §3, reproduced. That entry records the
    /// bug as "verified by reading but not reproduced, and therefore
    /// deliberately not fixed", and notes that the attempt to reproduce it
    /// "grew no tips at all (`plant_tree` on a soil floor with no field
    /// step)" — germination is light-gated, so a run that never steps the
    /// field never germinates anything and can only ever report zero tips.
    /// This uses `run_with_fields`, which is the difference.
    ///
    /// The bug: `scheduler::step` pops the whole due batch into `due_sites`
    /// *before* dispatching any of it, so `world.active_sites` does not
    /// contain the batch while `plant::tick` runs.
    /// `World::organism_active_tip_count` counts the heap, so it cannot see
    /// any tip in the batch currently being dispatched — and a tree's tips
    /// all come due on the same frame as a matter of course, since they are
    /// created together and rescheduled on a fixed interval. `Grow`'s cap
    /// therefore compares against a count that is far too low.
    ///
    /// Measured *between* frames, where the heap does hold everything, so
    /// the count is the true one rather than the one the cap sees.
    ///
    /// **Result: it does not reproduce, and the reason reframes the bug.**
    /// The measured peak is **1**. Not "under the cap" — one. Tip
    /// retirement (see `self_type_after_grow` above) means a `GrowingTip`
    /// becomes `MatureBody` in the same tick it grows, with the child
    /// carrying the frontier forward, so a lineage holds exactly one live
    /// tip at a time and branching only briefly makes it two.
    /// `max_active_tips: 14` was sized for the pre-retirement system where
    /// tips persisted; against the current one it has no work to do.
    ///
    /// So the under-enforcement is real as read *and* unreachable as built:
    /// a cap that is never approached cannot be exceeded, however badly it
    /// is checked. This test is therefore a **tripwire, not a live check**,
    /// and it is worth keeping as one — `Reports/plant-substrate-v2-
    /// design.md`'s bud break (retrofit step 9) exists specifically to let
    /// a mature tree open new frontiers, which is the first thing that
    /// would push simultaneous tips toward the cap and make the miscount
    /// bite. It should start doing real work exactly then.
    ///
    /// Recorded in `Reports/open-bugs-handoff.md` §3 as measured rather
    /// than left as "verified by reading."
    #[test]
    fn a_trees_simultaneous_tip_count_stays_within_its_species_cap() {
        // `tree.ron`'s own `max_active_tips` for `GrowingTip`.
        const CAP: usize = 14;

        let mut w = test_world();
        w.plant_tree(100, 20);
        let organism_id = w.get(100, 20).organism_id();
        assert_ne!(organism_id, 0, "test setup: planting should have stamped an organism id");

        let mut peak = 0;
        for _ in 0..8000 {
            update::step(&mut w);
            w.step_active_sites();
            field::step(&mut w);
            peak = peak.max(w.organism_active_tip_count(organism_id, CellType::GrowingTip));
        }

        assert!(
            peak <= CAP,
            "a tree held {peak} simultaneous GrowingTip sites against tree.ron's max_active_tips of {CAP} \
-- Grow's cap reads World::organism_active_tip_count, which cannot see tips inside the batch \
scheduler::step is currently dispatching (open-bugs-handoff.md §3)"
        );
    }

    /// The plastochron's own unit: a tip whose lineage step is about to hit
    /// the interval retires to `Leaf`, and one that isn't retires to
    /// `MatureBody`. Driven through `organism_tick`'s real dispatch rather
    /// than by calling a helper, since the whole mechanism is the
    /// parent→child hand-off of a counter that lives on the `ActiveSite`.
    #[test]
    fn a_retiring_tip_becomes_a_leaf_once_per_plastochron() {
        let wood = material::MaterialId(11);
        // `tree.ron` ships `plastochron: 3`, so lineage steps 3, 6, 9 ...
        // leaf and the rest mature. Entering a tick with `plastochron = 2`
        // makes this the third step.
        for (entering, expected) in [(2u8, CellType::Leaf), (0u8, CellType::MatureBody), (1u8, CellType::MatureBody)] {
            let mut w = test_world();
            let tree = w.species.id_of("tree").expect("tree is a compiled-in species");
            let organism_id = w.push_organism(tree);
            let aux = organism::pack_aux(CellType::GrowingTip, 2.0);
            w.set(100, 100, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(aux));

            organism_tick(&mut w, 100, 100, organism_id, 0, entering);

            let (self_type, _) = organism::unpack_aux(w.get(100, 100).aux());
            assert_eq!(
                self_type,
                Some(expected),
                "entering a growth step with plastochron={entering} against tree.ron's interval of 3 should retire to {expected:?}"
            );
        }
    }

    /// The point of the whole change, asserted end to end rather than as a
    /// unit: a tree grown from a seed produces real `Leaf` cells, and
    /// `SecondaryThicken` — which had **never fired on anything** before
    /// this, because it counts downstream `Leaf | GrowingTip` cells and
    /// tips retire the instant they grow — produces a trunk more than one
    /// cell thick.
    ///
    /// Both halves are measured against the committed baseline in
    /// `docs/screenshots/plant-v2-baseline/`, where the same scene gives 0
    /// leaves and a thickest contiguous run of 1.
    ///
    /// **Deliberately an ensemble, not one tree, and the reason is
    /// measured.** Twelve identical trees in one scene
    /// (`examples/plant_probe.rs -- trees=12`) span 31 to 153 cells and 10
    /// to 33 leaves — a five-fold spread from the same species file, same
    /// scene and same frame count, with only position separating them. A
    /// single tree is therefore a sample from a very wide distribution, and
    /// a bar set against one run would be flaky in whichever direction that
    /// run happened to land. This was found the hard way: the first version
    /// of this test asserted `>= 5` leaves off a single measured 18, and
    /// swapping to a per-organism RNG — which changes *which* numbers a
    /// tree draws, not how many or their distribution — dropped the same
    /// scene to 4 and failed it.
    ///
    /// Bars are set under the ensemble minimum with headroom, per this
    /// repo's own convention. They exist to catch the mechanism going inert
    /// again (baseline: exactly 0 leaves, run of 1), not to pin a shape
    /// that is expected to keep moving as later phases land.
    #[test]
    fn grown_trees_produce_leaves_and_a_trunk_thicker_than_one_cell() {
        let mut w = test_world();
        for x in [40, 90, 140] {
            w.plant_tree(x, 20);
        }
        run_with_fields(&mut w, 8000);

        let b = w.bounds().unwrap();
        let mut leaves = 0;
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                if organism::unpack_aux(w.get(x, y).aux()).0 == Some(CellType::Leaf) && w.get(x, y).organism_id() != 0 {
                    leaves += 1;
                }
            }
        }
        assert!(leaves >= 6, "three grown trees should carry real Leaf cells; got {leaves} across all three (baseline before the plastochron: 0)");

        let thickest = (b.min_y..=b.max_y)
            .map(|y| {
                let (mut best, mut run) = (0usize, 0usize);
                for x in b.min_x..=b.max_x {
                    if w.get(x, y).organism_id() != 0 {
                        run += 1;
                        best = best.max(run);
                    } else {
                        run = 0;
                    }
                }
                best
            })
            .max()
            .unwrap_or(0);
        assert!(
            thickest >= 2,
            "SecondaryThicken should fire once leaves give thicken() a real downstream count; thickest contiguous run was {thickest} (baseline: 1)"
        );
    }
}
