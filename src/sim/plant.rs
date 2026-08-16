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
use super::rng::{self, Rng};
use super::scheduler::{ActiveKind, ActiveSite};
use super::update;
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
/// Can a growing cell of this species advance into `(x, y)`?
///
/// Open air always. **Plus, for a cell type with real `penetration_force`
/// (roots), a `Powder` whose `Material::penetration_resistance` it can
/// overcome** — `Reports/plant-substrate-v2-design.md`'s Decision 1(ii),
/// the reason a root can finally leave open air. Until this existed a root
/// could only ever extend into a literal void, so a tree planted on soil
/// grew no root system at all, which is the long-standing "no roots in the
/// test scene" report.
///
/// **A raw `material == EMPTY` test, deliberately not `World::is_empty`.**
/// `Cell::is_empty()` is managed-aware: a promoted liquid body's container
/// cells are materially empty but report as occupied, and the question here
/// is "is there material in the way", not "is this position available".
/// `CLAUDE.md` lists this as a gotcha that has already caused a real bug;
/// nothing promotes a body in production yet, so getting it wrong would be
/// latent rather than visible, which is worse.
///
/// One cell is converted, never a pushed column. A real root tip does not
/// shove a column of soil ahead of itself — it sheds lubricating border
/// cells and the soil deforms plastically and locally around it, and roots
/// preferentially follow existing pores and old channels rather than
/// displacing bulk soil at all. Converting one cell is closer to that than
/// a piston would be, as well as cheaper.
fn growable(world: &World, x: i32, y: i32, penetration_force: f32) -> bool {
    let cell = world.get(x, y);
    if cell.material == material::EMPTY {
        return true;
    }
    if penetration_force <= 0.0 {
        return false;
    }
    let m = world.materials.get(cell.material);
    // `Powder` only. `Solid` never yields however hard a root pushes --
    // which preserves the already-playtested behaviour that a tree on bare
    // stone fails to root, rather than silently letting roots eat the floor.
    // `Liquid` is `Absorb`'s business, not a thing to grow *through*.
    m.kind == MaterialKind::Powder && m.penetration_resistance < penetration_force
}

/// Push the water held in the soil cell at `(x, y)` into its neighbours,
/// before something overwrites that cell.
///
/// **A root growing into soil used to delete the water that was there.**
/// `growable` lets a `RootTip` enter a penetrable `Powder`, and `Grow` then
/// writes its new cell straight over it — replacing the `aux` that, for a
/// `Powder`, *is* the moisture. In the `forest` scene every root cell
/// silently destroyed `SOIL_FIELD_CAPACITY` (620) units; a hundred-cell
/// root system lost roughly sixty-two saturated cells' worth. Nothing
/// noticed because no conservation tally covers held water — the liquid
/// tallies only know about `Liquid` cells, which is `Material::water_
/// capacity`'s own recorded caveat. Found by independent review.
///
/// Displacing rather than crediting, deliberately: handing it to the root
/// as resource would double-count against `Absorb`, which is the behaviour
/// that already exists to take water *up*. A root physically displaces the
/// soil it grows into; the water in that soil goes into the soil around it.
///
/// Any remainder that will not fit in the neighbourhood is genuinely lost,
/// and that is honest rather than swept aside: the alternative is refusing
/// the growth, which would make root architecture depend on local soil
/// saturation in a way nothing in the design asks for. Worth revisiting if
/// held water ever gets a conservation tally.
fn displace_soil_water(world: &mut World, x: i32, y: i32) {
    let cell = world.get(x, y);
    if world.materials.get(cell.material).water_capacity == 0 {
        return;
    }
    let mut carried = update::soil_moisture(cell);
    if carried == 0 {
        return;
    }
    for (dx, dy) in NEIGHBOURS_8 {
        if carried == 0 {
            break;
        }
        let (nx, ny) = (x + dx, y + dy);
        let n = world.get(nx, ny);
        let n_capacity = world.materials.get(n.material).water_capacity;
        if n_capacity == 0 {
            continue;
        }
        // Bounded by the *neighbour's* capacity, not this cell's -- the
        // same asymmetry `update.rs`'s capillary exchange gets wrong and
        // gets away with only because `soil` is currently the one material
        // that holds water at all.
        let held = update::soil_moisture(n);
        let moved = carried.min(n_capacity.saturating_sub(held));
        if moved == 0 {
            continue;
        }
        world.set(nx, ny, n.with_aux(held + moved));
        carried -= moved;
    }
    // **Write the remainder back rather than trusting the caller to
    // overwrite this cell.** Every production caller does overwrite it a
    // line later, so this is invisible there -- but leaving the source at
    // its original reading makes the function *create* water when called on
    // its own, which is exactly what its conservation test caught. A
    // function that only conserves when its caller happens to clean up
    // after it is a trap for the next caller.
    if carried != update::soil_moisture(cell) {
        let now = world.get(x, y);
        world.set(x, y, now.with_aux(carried));
    }
}

/// Local foliage proximity at a growth candidate — **any organism's, not
/// just this one's.**
///
/// The `organism_id` filter this used to carry has been removed, and the
/// reason is a citation rather than a preference. The channel is a
/// stigmergic stand-in for **shade-avoidance signalling**: a real shoot
/// senses the red/far-red ratio of light *reflected off nearby foliage*
/// and shifts its growth away from it. A phytochrome cannot ask whose leaf
/// it bounced off. Filtering by owner was a defensible reading while this
/// was framed as *self*-avoidance, and it is the wrong reading of the
/// mechanism it models.
///
/// **This is `Reports/tree-architecture-research.md` §7c, and it answers a
/// problem 2D creates.** A crown of radius `R` has `~R³` of volume to
/// branch into in three dimensions and `~R²` of area in two, so the same
/// branch count is `R` times denser here and neighbouring structures merge
/// far more readily than any 3D-calibrated model expects. Real forests
/// solve exactly this with **crown shyness** — the gaps adjacent trees
/// leave between their canopies — and the far-red mechanism is the one that
/// transfers, precisely *because* it is owner-blind: one rule keeps a tree
/// from merging with itself **and** with its neighbour.
///
/// Note this deliberately does *not* consult `organism_id` at all now, so
/// hand-painted inert `wood` contributes nothing (it carries no
/// `canopy_density`), while any organism's foliage does.
fn candidate_crowding(world: &World, x: i32, y: i32) -> f32 {
    let mut sum = 0.0f32;
    let mut count = 0u32;
    for (dx, dy) in NEIGHBOURS_8 {
        if world.get(x + dx, y + dy).organism_id() != 0 {
            sum += world.canopy_density_at(x + dx, y + dy);
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
/// every `ORGANISM_TICK_INTERVAL`, 0.5 halves a fresh deposit each cycle a
/// cell's own tick fires. Unlike the old per-frame placement, this decay
/// never needs to be tuned small to survive many consecutive calls before
/// the next real read. Still fades a deposit toward zero over a handful of
/// cycles once nothing nearby keeps refreshing it — the same "let later
/// growth reclaim space near mature wood" intent the original mechanism
/// described, now actually reachable by the checks that matter.
///
/// **Re-examined when the scalar left `aux` (Decision 2 step 2c) and kept
/// at 0.5 — but what it does changed, and the change is not cosmetic.**
/// The old doc justified the value partly by its clearing the 4-bit
/// quantization half-step on every application. It did not: a halving
/// applied to a 4-bit field *cannot get below one quantum*, because
/// 0.267 × 0.5 = 0.133 rounds straight back to 0.267. Density therefore
/// had a permanent floor and this decay silently stopped after two steps
/// — see `organism::CANOPY_DENSITY_SCALE` for the measured trace.
///
/// The rate is unchanged because a halving is right on its own terms
/// (scale-free, negligible within a handful of ticks). What changed is
/// that the decay now actually reaches zero.
///
/// **Measured consequence, recorded rather than compensated for.** Paired
/// 24-tree ensembles across the migration, same scene and frame count
/// (`plant_probe -- trees=24 frames=8000`):
///
/// | | floor at 0.267 | decays to 0 |
/// |---|---|---|
/// | total organism cells | 5872–5881 | 5806 |
/// | cells per tree, mean | 248.2 | 241.9 |
/// | cells per tree, median | 114 | 85 |
/// | height, median | 40 | 30 |
/// | leaves, median | 15 | 11 |
/// | rows >1 cell wide, mean | 42% | 35% |
///
/// The left column is a *range over three runs of one binary*, because the
/// old code was not reproducible run to run — see this pass's own note on
/// sorting the cell list, which is what made the right column a single
/// number. The shape row is the exception and is worth knowing: it read
/// **42 on four consecutive baseline runs**, so it is a robust population
/// statistic and the 42 → 35 shift is real rather than noise.
///
/// **What causes that shift is NOT established, and specifically it is not
/// this constant** — replaying the old 4-bit quantization on the migrated
/// code moves the metric 35 → 36, one point of seven. Eight candidates were
/// A/B'd and all were ruled out; the table is in `PLAN.md` rather than
/// duplicated here.
///
/// **And the shift is largely not real.** `rows >1 cell wide` is dominated
/// by the basal pancake `thicken()` produces, not by trunk thickness:
/// measured *above the base*, mean stem thickness went 11.8 → **13.3**,
/// i.e. slightly thicker, while the thickest single row went 61 → 121.
/// The metric fell because the distribution widened, not because trunks
/// thinned. Lowering `pipe_ratio` from 10.0 to 6.0 restores the metric to
/// 43% and simultaneously takes biomass from 5,806 to 13,795 by drawing a
/// slab across the whole ground — tuning the artifact, not the tree.
///
/// So this constant is left alone, and so is `pipe_ratio`. The real fix is
/// the `thicken()` change `PLAN.md` already lists as known-open.
///
/// **This is `CLAUDE.md`'s "fixing a bug often exposes a constant that was
/// compensating for it", and the constants are deliberately not re-tuned
/// here** — `Reports/plant-substrate-v2-design.md` §10 forbids `.ron`
/// edits at this step precisely so the economy pass tunes once, against
/// the final transport mechanism, rather than twice.
const CANOPY_DENSITY_DECAY_PER_TICK: f32 = 0.5;

/// Write a cell's carbon back to the sidecar.
///
/// Replaces the `world.set(x, y, cell.with_aux(pack_aux_preserving_density(
/// ...)))` that every resource update in `organism_tick` used to be, and is
/// a different kind of write in one way worth knowing: **it does not touch
/// the grid, so it does not dirty a chunk.** Spending or gaining resource
/// no longer wakes the CA sweep. A cell-*type* transition still goes
/// through `World::set`, because that really is a change to the grid.
fn write_carbon(world: &mut World, x: i32, y: i32, carbon: f32) {
    if let Some(slot) = world.organism_cell_mut(x, y) {
        slot.carbon = carbon;
    }
}

/// Stamp a freshly created cell's branch order.
///
/// Like `deposit_canopy`, must follow the `World::set` that creates the
/// cell — `set` registers the `OrganismCell` this writes into, so calling
/// it first is a silent no-op that would leave every cell at order 0, which
/// looks exactly like "the species file has no tiers".
/// One individual's multiplier on one species parameter — the genotype
/// jitter described on `organism::Behavior::Grow::genotype_variance`.
///
/// Uniform in `1 ± variance`, drawn from the organism id and a per-trait
/// `salt` so traits vary independently. Stateless on purpose: an organism's
/// genotype is a pure function of its id, so it needs no storage, survives
/// every cell of the plant being replaced, and cannot drift.
pub fn genotype(organism_id: u16, salt: u64, variance: f32) -> f32 {
    if variance <= 0.0 {
        return 1.0;
    }
    let mut rng = rng::stream(organism_id as u64, salt, 0, 0);
    let unit = rng.below(10_000) as f32 / 10_000.0 * 2.0 - 1.0;
    (1.0 + unit * variance).max(0.0)
}

fn write_order(world: &mut World, x: i32, y: i32, order: u8) {
    if let Some(slot) = world.organism_cell_mut(x, y) {
        slot.order = order;
    }
}

/// Stamp a freshly created cell's canopy-density deposit.
///
/// Must be called *after* the `World::set` that creates the cell: `set` is
/// what registers the `OrganismCell` this writes into, so calling it first
/// is a silent no-op. Clamped on write rather than trusting the caller,
/// since `CANOPY_DENSITY_SCALE` is now a behavioural bound with nothing in
/// the storage enforcing it.
fn deposit_canopy(world: &mut World, x: i32, y: i32, density: f32) {
    if let Some(slot) = world.organism_cell_mut(x, y) {
        slot.canopy_density = density.clamp(0.0, organism::CANOPY_DENSITY_SCALE);
    }
}

// `pack_aux_preserving_density` lived here, and is deliberately gone.
//
// It existed because `pack_aux` rewrote the whole `aux` word, so every
// ordinary resource/type write in this dispatch silently zeroed the canopy
// density packed into the same word -- a real bug, found by live
// verification, and patched by threading the pre-tick `aux` through every
// write site so each could put the density back.
//
// With the scalars in `OrganismCell` the two fields are no longer
// co-located, so a cell-type write cannot clobber a density and there is
// nothing to preserve. This is one of the four mechanisms
// `Reports/plant-substrate-v2-design.md` §3e predicted the migration would
// fix for free, and the only interesting thing about it now is that the
// bug class is unrepresentable rather than fixed: `with_aux` writes a cell
// type, `world.organism_cell_mut` writes a scalar, and neither can reach
// the other's storage.

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

/// Upper bound on how many behaviors one cell type may carry, sized so the
/// dispatch buffer above never allocates. Raise it if a species file needs
/// more -- a `debug_assert` catches overflow rather than silently dropping
/// behaviors.
const MAX_BEHAVIORS_PER_CELL_TYPE: usize = 8;

/// How often an **ungerminated seed** is checked, while it may still be
/// falling.
///
/// Much shorter than `ORGANISM_TICK_INTERVAL`, and the reason is bookkeeping
/// rather than biology: an `ActiveSite` names a fixed position, but a seed
/// is a `Powder` and moves. A seed falls about a cell a frame, so at 45
/// frames it could be 45 cells from where its site says it is, while at 4 it
/// is within a handful and `relocated_seed` can find it. A seed is also
/// exactly one cell that exists only briefly, so the extra checks cost
/// nothing worth measuring.
const SEED_TICK_INTERVAL: u64 = 4;

/// How far below its recorded position to look for a seed that has fallen.
/// Generous against `SEED_TICK_INTERVAL`'s own bound (roughly one cell per
/// frame), so a seed dropped down a long shaft is still found.
const SEED_FALL_SEARCH: i32 = 12;

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

/// How much held water a root takes from one soil cell in a tick at full
/// uptake, on `material::SOIL_SATURATED`'s scale.
///
/// Sized so a root draws a soil cell from field capacity down toward the
/// wilting point over a handful of its own ticks rather than instantly:
/// the interesting behaviour is a root system *competing* with itself and
/// its neighbours for a finite local store, and an instant drain would
/// make every root independent of every other. Untuned beyond that
/// reasoning, and a first-class target for the economy pass.
const SOIL_UPTAKE_PER_TICK: u16 = 60;

/// Water drawn out of adjacent soil by **transpiration**, per root cell per
/// organism tick, on `material::SOIL_SATURATED`'s scale — and credited to
/// nothing.
///
/// **This is the physically dominant term, and it was missing entirely.**
/// Of all the water a plant takes up, only on the order of 1-3% is retained
/// for growth and photosynthesis; the other ~97-99% is transpired, moving
/// up the xylem and out through the stomata into the air. `Absorb` models
/// only the small retained fraction, because that is the part that becomes
/// `resource` — so a tree could stand in soil indefinitely and barely dry
/// it, which is not what a real tree does to the ground beneath it.
///
/// Deliberately *not* credited as resource. Transpired water is lost, not
/// eaten. Crediting it would inflate the energy economy this phase is
/// explicitly not re-tuning, and would also be wrong: the plant gets no
/// food from the 98%.
///
/// **Where the scaling comes from, and its honest limitation.** Real
/// transpiration is driven by *leaf* area and evaporative demand, not by
/// root count — the canopy is the pump. This draws per root cell instead,
/// which gets the right behaviour for the right *structural* reason: root
/// and shoot mass stay roughly proportional as a plant grows (a conserved
/// root:shoot ratio), so a bigger canopy sits on a bigger root system and
/// draws more. The scaling with tree size is faithful; the driver is a
/// stand-in. Driving it from a real leaf count needs the whole-organism
/// totals Decision 2's sidecar introduces, and `Reports/plant-substrate-v2-
/// design.md` §6 already sanctions holding such totals there.
///
/// Small per cell on purpose: a mature root system is many cells, so the
/// *organism's* draw is the sum and grows with it.
const TRANSPIRATION_PER_ROOT_CELL: u16 = 12;

/// Most of an organism's cells that may be root before root growth stops.
///
/// Real plants hold a roughly conserved **root:shoot ratio** — for trees,
/// roots are typically on the order of 20-50% of total biomass, because
/// roots are built from carbon the canopy fixes and a large root system
/// cannot be funded by a small crown. Nothing expressed that here, and the
/// consequence was not subtle: once soil moisture gave roots income
/// everywhere, a stand converted essentially an entire soil bed to root
/// tissue.
///
/// It went unnoticed for a while because the active-site scheduler was
/// accidentally throttling it — `MAX_SITES_PER_FRAME` capped the due
/// backlog, so growth was rate-limited by a *budget* rather than by
/// biology. Moving mature cells off the schedule removed that accident and
/// exposed the missing bound underneath, which is worth recording: a
/// performance limit standing in for a design one hides the design one.
///
/// `0.5` is the generous end of the real range, chosen so the rule bites
/// only on runaway growth rather than shaping ordinary root systems.
const MAX_ROOT_FRACTION: f32 = 0.5;

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

/// Where this organism's seed actually is, given a site that says `(x, y)`.
///
/// A seed is a `Powder` now, so it falls, rolls and settles somewhere other
/// than where it was planted — but its `ActiveSite` still names the planting
/// position. Rather than teach the scheduler to follow a moving cell (which
/// wants the per-organism cell list Decision 2 builds, and is the right fix
/// later), a seed that has left its recorded position is looked for
/// directly below it, in the narrow cone a falling grain can reach.
///
/// Deliberately seed-only. Every other organism cell is immovable, so
/// nothing else can go missing this way and nothing else pays for the
/// search.
fn relocated_seed(world: &World, x: i32, y: i32, organism_id: u16) -> Option<(i32, i32)> {
    for dy in 1..=SEED_FALL_SEARCH {
        for dx in [0, -1, 1] {
            let (nx, ny) = (x + dx * dy.min(2), y + dy);
            let c = world.get(nx, ny);
            if c.organism_id() == organism_id && organism::cell_type(c.aux()) == Some(CellType::Seed) {
                return Some((nx, ny));
            }
        }
    }
    None
}

fn organism_tick(world: &mut World, x: i32, y: i32, organism_id: u16, stale_ticks: u8, plastochron: u8) -> Vec<ActiveSite> {
    // A seed that fell out from under its own site: pick the search back up
    // wherever it landed instead of dropping the organism on the floor.
    if world.get(x, y).organism_id() != organism_id {
        if let Some((sx, sy)) = relocated_seed(world, x, y, organism_id) {
            return vec![reschedule_organism(sx, sy, organism_id, stale_ticks, plastochron, world.frame + SEED_TICK_INTERVAL)];
        }
    }
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
    let Some(mut cell_type) = organism::cell_type(cell.aux()) else {
        return Vec::new(); // unrecognized cell-type bits -- nothing this dispatch knows how to run
    };
    // The resource scalar now comes from the sidecar rather than out of
    // `aux` alongside the cell type. Read once into a local and written
    // back through `world.organism_cell_mut` at each point the old code
    // re-packed it, so the shape of this dispatch is unchanged.
    let mut resource = world.carbon_at(x, y);
    // **Branch order**, read once here for the same reason `resource` is:
    // every `Grow` parameter that varies by tier indexes on it, and every
    // cell this tick creates is stamped from it. See
    // `organism::OrganismCell::order` and `organism::ByOrder`.
    let order = world.organism_cell(x, y).map_or(0, |c| c.order);
    // Copied out of the registry rather than held as a borrow: the behavior
    // loop below needs `&mut World` (to paint a new cell, roll the RNG),
    // which a live borrow of `world.species` would conflict with.
    //
    // **Into a stack buffer, not a `Vec`.** This used to `.to_vec()`, on the
    // reasoning that species data is small -- true, and it still allocated
    // on the heap once per organism cell per tick. That was invisible while
    // a tree was ~18 cells; a six-tree stand reaching 3,000 cells makes it
    // roughly 350,000 allocations over a 6,000-frame run. `Behavior` is
    // `Copy` and no cell type carries more than a handful, so a fixed array
    // costs nothing and allocates never.
    let mut behavior_buf = [None::<Behavior>; MAX_BEHAVIORS_PER_CELL_TYPE];
    let behavior_count = {
        let defined = world.species.get(species_id).behaviors(cell_type);
        debug_assert!(
            defined.len() <= MAX_BEHAVIORS_PER_CELL_TYPE,
            "a cell type defines more behaviors than the dispatch buffer holds -- raise MAX_BEHAVIORS_PER_CELL_TYPE"
        );
        let n = defined.len().min(MAX_BEHAVIORS_PER_CELL_TYPE);
        for (slot, behavior) in behavior_buf.iter_mut().zip(&defined[..n]) {
            *slot = Some(*behavior);
        }
        n
    };

    // Canopy density decays once per call, on this function's own
    // schedule -- see `CANOPY_DENSITY_DECAY_PER_TICK`'s own doc for why
    // this replaced an earlier per-CA-frame placement.
    //
    // **The guard this used to need is gone, and so is the problem it
    // solved.** While density lived in `aux`, decaying it was a `World::set`
    // and therefore dirtied the cell's chunk, so an organism kept the CA
    // sweep awake merely by existing -- which is what made
    // `a_settled_world_with_a_growing_tree_still_sleeps_between_growth_
    // ticks` start failing once trees reached 70+ cells, and forced a
    // write-only-if-changed guard to work around it. A sidecar write
    // touches no chunk, so the decay cannot wake anything and the guard has
    // nothing left to guard. Keeping it would only skip a float multiply.
    if let Some(slot) = world.organism_cell_mut(x, y) {
        slot.canopy_density *= CANOPY_DENSITY_DECAY_PER_TICK;
    }

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
    for behavior in behavior_buf.into_iter().take(behavior_count).flatten() {
        match behavior {
            // Evaluated once per organism in `break_buds`, never from the
            // bud's own tick -- and a `DormantBud` carries no active site
            // at all, so this arm is unreachable in practice.
            Behavior::BudBreak { .. } => {}
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
                    let new_cell = Cell::new(cell.material, shade).with_organism_id(organism_id).with_aux(organism::pack_cell_type(cell_type));
                    // `set` registers the child with a zeroed `OrganismCell`,
                    // which is exactly the "starts at 0 resource, not
                    // inheriting any" the old explicit `pack_aux(_, 0.0)`
                    // spelled out.
                    world.set(tx, ty, new_cell);
                    write_order(world, tx, ty, order);
                    resource -= cost;
                    write_carbon(world, x, y, resource);
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
                penetration_force,
                turgor_source,
                turgor_yield,
                turgor_per_cell,
                turgor_taper,
                genotype_variance,
            } => {
                // Per-order parameters resolved once, against *this cell's*
                // own order. A tip reads only its own tier -- no traversal,
                // no whole-plant query -- which is what keeps architecture
                // local; see `organism::ByOrder`.
                // Per-order first, then this individual's own genotype on
                // top. The salts are arbitrary but must stay distinct and
                // stable -- two traits sharing a salt move together, which
                // is the "one tree scaled up" failure `genotype_variance`
                // exists to avoid.
                let branch_chance = branch_chance.at(order) * genotype(organism_id, 1, genotype_variance[0]);
                let light_weight = light_weight.at(order) * genotype(organism_id, 6, genotype_variance[5]);
                let upward_weight = upward_weight.at(order) * genotype(organism_id, 2, genotype_variance[1]);
                let plastochron_interval =
                    ((plastochron_interval.at(order) as f32 * genotype(organism_id, 3, genotype_variance[2])).round() as u8).max(u8::from(plastochron_interval.at(order) > 0));
                // **Height is the trait the clone look shows up in first**,
                // because the turgor bound is geometric and every tree
                // reaches it exactly. Jittering the per-cell cost spreads
                // the derived ceiling instead of the outcome.
                let turgor_per_cell = turgor_per_cell * genotype(organism_id, 4, genotype_variance[3]);
                // **These three gates deliberately do *not* set
                // `found_candidate`, and that is what makes growth
                // terminate.** The staleness counter is how "temporarily
                // short" becomes "permanently short": a tip that cannot
                // afford `cost` tick after tick eventually ages out and
                // retires, which is the entire mechanism behind
                // `a_tree_eventually_stops_growing`. Marking them as "had
                // somewhere to try" was tried and reverted -- it makes a
                // resource-starved tree grow forever.
                //
                // What the ageing-out must *not* do is drop a cell on the
                // floor, which is the bug this pass fixed a few lines
                // below: see the stale-limit retirement.
                if resource < cost {
                    continue;
                }
                // **The height bound** -- `organism::Behavior::Grow`'s
                // turgor fields, and `Reports/tree-extension-biology.md`
                // §2c. Turgor at the apex is the collar's turgor less the
                // gravitational cost of lifting water to this row, and a
                // cell wall only extends while that exceeds the yield
                // threshold.
                //
                // **This is the only gate in the system built from geometry
                // rather than resource state**, and that is the whole point.
                // Every resource signal equalizes when growth stops -- carbon
                // fills every cell to its cap, crowding decays everywhere,
                // conductance relaxes to basal everywhere -- so a rule keyed
                // on any of them fires on every cell at once, which is
                // exactly how the reverted bud break ran away. Height cannot
                // do that: the apex stays at the top and the collar at the
                // bottom, permanently.
                //
                // Skipped entirely when `turgor_per_cell` is 0, which is a
                // real species value meaning "this plant has no height
                // limit" -- a moss mat, a vine -- and is also why `RootTip`
                // growth is unaffected without a special case.
                if turgor_per_cell > 0.0 {
                    let collar = world.organism(organism_id).and_then(|s| s.collar_y);
                    if let Some(collar) = collar {
                        // Rows *above* the collar; y grows downward.
                        let height = (collar - y).max(0) as f32;
                        let margin = turgor_source - turgor_per_cell * height - turgor_yield;
                        if margin <= 0.0 {
                            continue;
                        }
                        // **The taper, and why the hard cutoff alone was
                        // not enough.** Lockhart's equation makes wall
                        // extension rate proportional to `(P - Y)`, not a
                        // step at zero, and the difference is the whole
                        // silhouette: with a step, every lineage in the
                        // plant runs at full speed right up to one row and
                        // terminates there, so growth piles up under the
                        // bound like sediment and each crown reads as a
                        // flat horizontal plate. Measured on eight trees:
                        // separated crowns, staggered heights, clear boles
                        // -- and every one of them capped with a straight
                        // edge.
                        //
                        // Tapering makes the last stretch stochastic
                        // instead. A lineage near the bound grows slowly,
                        // and slow growth accumulates `stale_ticks`, so
                        // some lineages retire before others -- the top
                        // fades out over a band rather than stopping on a
                        // line. `genotype_variance` then offsets that band
                        // per individual, which is why the two changes
                        // belong together.
                        if turgor_taper > 0.0 {
                            let full = (turgor_source - turgor_yield).max(f32::EPSILON);
                            if !rng.chance((margin / full / turgor_taper).min(1.0)) {
                                continue;
                            }
                        }
                    }
                }
                // Allometry: a root system is bounded by the canopy that
                // funds it. Reads the counts `step_organisms` refreshed --
                // no traversal here.
                if cell_type == CellType::RootTip {
                    if let Some(state) = world.organism(organism_id) {
                        let total = state.root_cells + state.shoot_cells;
                        if total > 0 && (state.root_cells as f32 / total as f32) >= MAX_ROOT_FRACTION {
                            continue;
                        }
                    }
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
                // **`away_from_supply`, replacing `away_from_growth`** --
                // Decision 6 §7g. Polarity adds no new scoring term and no
                // new species parameter; it replaces the computation behind
                // the existing `continuation_weight`, which keeps
                // `tree.ron`'s `0.7` meaning what it meant and keeps the
                // economy pass comparing a six-weight blend rather than a
                // seven-weight one.
                //
                // The geometric average below stays as the fallback, and is
                // reached in exactly two cases: an organism whose supply
                // field is still uniformly basal (a seed's first `Grow`,
                // before any flux has been carried), and a cell with no
                // same-organism 4-neighbour to be supplied *by*. So the
                // first growth step of every organism is bit-identical to
                // before, and §2a's zero-neighbour `(0.0, -1.0)` proof
                // survives untouched.
                let away_from_supply = match organism::supply_direction(world, x, y) {
                    Some((sx, sy)) => (-sx, -sy),
                    None if same_organism_neighbours > 0 => normalize((-away_sum.0, -away_sum.1)),
                    None => (0.0, -1.0), // the seed's very first Grow: straight up, same fallback the old Tip's initial dir used
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
                    if !growable(world, nx, ny, penetration_force) {
                        continue;
                    }
                    let dir = normalize((dx as f32, dy as f32));
                    let density = candidate_crowding(world, nx, ny);
                    let score = dot(dir, away_from_supply) * continuation_weight
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
                // **The retiring parent always becomes `MatureBody`; a leaf
                // is spawned *beside* the stem instead** (below, once the
                // child and any branch child exist so their cells can be
                // excluded).
                //
                // This corrects `Reports/plant-substrate-v2-design.md` §5a,
                // which converts the retiring parent itself into a `Leaf`
                // and counts "requires no new cell creation whatsoever" as
                // a virtue. That virtue is what made the trunk read as
                // wood-wood-leaf-wood-wood-leaf -- the stem was built
                // *through* its own foliage. Reported from live play as
                // the pink and purple looking "layered, which is not how I
                // would expect it to be", which is exactly right: real
                // leaves attach laterally at nodes, they are not segments
                // of the stem.
                //
                // Three things were wrong with it, in increasing severity:
                //
                // 1. `thicken()` is `MatureBody`-only, so a stem could not
                //    thicken at every plastochron-th cell -- the patchy
                //    thickening reported alongside it.
                // 2. `structural.rs`'s `organism_is_supported` filters on
                //    `organism_id` and `Plant` kind and never looks at cell
                //    type, so a leaf in the stem *carried structural load* --
                //    which the design doc's own §6a explicitly forbids
                //    ("treating it as a load path would let a canopy hold up
                //    a trunk").
                // 3. Decision 4 gives leaves a lifespan and abscission. With
                //    leaves as stem segments, shedding them would cut the
                //    trunk into disconnected pieces every plastochron. That
                //    would have landed two phases later as a mystifying
                //    "trees fall apart" bug.
                // **A `RootTip` retires too, and it must.** This used to
                // keep `RootTip` as `RootTip`, with the honest note that
                // roots "have no `MatureBody`-equivalent 'settled root'
                // type to retire into yet". That was harmless only because
                // roots could not grow at all: `Grow` required an empty
                // neighbour and a root sitting in soil never had one.
                //
                // The moment Decision 1(ii) let roots enter soil and
                // Decision 3 gave them something to eat, every root cell
                // stayed an eligible growing tip forever and the frontier
                // multiplied instead of advancing — sprawling horizontal
                // mats spanning the whole soil bed within 20,000 frames,
                // which is the identical failure the canopy's own
                // tip-retirement fix was written for (78% of Grow-evaluated
                // positions revisited 3+ times, radiating from static hubs).
                //
                // `MatureBody` is the right home rather than a new type:
                // settled root tissue genuinely does thicken and anchor,
                // which is exactly the behaviour set `tree.ron` already
                // attaches to it.
                //
                // **Except at a node, where it becomes a `DormantBud`.**
                // The metamer is internode + leaf + axillary bud, so the
                // same event that places a leaf places the bud that leaf
                // subtends -- one bud per node, deposited by extension and
                // by nothing else. `CellType::DormantBud`'s doc has why
                // that "and by nothing else" is the whole mechanism.
                //
                // A bud is stem tissue and stays stem tissue: same
                // material, same `StructuralAnchor`, load-bearing. It is a
                // `MatureBody` with one thing left it may do.
                //
                // Roots are excluded by `leaf_due` -- `tree.ron` gives the
                // `RootTip` a plastochron of 0, which disables nodes
                // entirely, so a root system deposits no buds and cannot
                // sprout shoots underground.
                let self_type_after_grow = if cell_type == CellType::GrowingTip && leaf_due {
                    CellType::DormantBud
                } else if matches!(cell_type, CellType::GrowingTip | CellType::RootTip) {
                    CellType::MatureBody
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
                let new_cell = Cell::new(cell.material, shade).with_organism_id(organism_id).with_aux(organism::pack_cell_type(cell_type));
                displace_soil_water(world, tx, ty);
                world.set(tx, ty, new_cell);
                // Straight continuation of this shoot, so the child keeps
                // the parent's order. The lateral below is what increments.
                write_order(world, tx, ty, order);
                // The deposit has to follow `set`, not ride along with it:
                // `set` is what registers the cell's `OrganismCell`, so
                // there is nothing to write into until it has run.
                deposit_canopy(world, tx, ty, GROW_CANOPY_DEPOSIT);
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
                world.set(x, y, cell.with_aux(organism::pack_cell_type(self_type_after_grow)));
                write_carbon(world, x, y, resource);
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
                        if growable(world, bx, by, penetration_force) {
                            let branch_shade = rng.below(shades) as u8;
                            let branch_cell =
                                Cell::new(cell.material, branch_shade).with_organism_id(organism_id).with_aux(organism::pack_cell_type(cell_type));
                            displace_soil_water(world, bx, by);
                            world.set(bx, by, branch_cell);
                            // **The only place order increases.** A lateral
                            // is one branching further from the seed, so it
                            // starts the next tier: rarer branching becomes
                            // more frequent branching, a bare shoot becomes
                            // a leafy one, and the trunk/limb/twig
                            // distinction exists without any cell having to
                            // work out which of those it is.
                            write_order(world, bx, by, order.saturating_add(1));
                            deposit_canopy(world, bx, by, GROW_CANOPY_DEPOSIT);
                            // No structural check here either -- see the
                            // primary child's identical case above.
                            resource -= cost;
                            world.set(x, y, cell.with_aux(organism::pack_cell_type(self_type_after_grow)));
                            write_carbon(world, x, y, resource);
                            next.push(reschedule_organism(bx, by, organism_id, 0, 0, world.frame + ORGANISM_TICK_INTERVAL));
                        }
                    }
                }

                // The plastochron's leaf, placed *laterally* off the node
                // that just retired -- see `self_type_after_grow` above for
                // why it is no longer the retiring cell itself.
                //
                // Any still-empty 8-neighbour will do, and it is picked at
                // random rather than at a fixed offset: a fixed side would
                // be the same one-sided-by-construction mistake `thicken()`
                // just had fixed (always growing left), and phyllotaxis --
                // the real rule placing leaves around a shoot -- is a
                // mechanism this engine does not model and should not fake
                // with an authored pattern (`design-philosophy.md` §2b
                // forbids the authored *outcome*, not the simple rule).
                //
                // Costs no resource, deliberately, matching what the
                // previous type-flip cost. A real leaf is built from
                // carbon and should charge for it; adding that price now
                // would change the economy this phase is explicitly not
                // tuning (§10 step 3 is the single pass that sets it, after
                // polarity). Flagged there rather than silently free.
                if leaf_due && cell_type == CellType::GrowingTip {
                    let spots: Vec<(i32, i32)> = NEIGHBOURS_8
                        .iter()
                        .map(|&(dx, dy)| (x + dx, y + dy))
                        .filter(|&(nx, ny)| world.is_empty(nx, ny))
                        .collect();
                    if !spots.is_empty() {
                        let (lx, ly) = spots[rng.below(spots.len() as u32) as usize];
                        // Real `leaf` material, not the shoot's own wood --
                        // foliage burns hot and fast, weighs almost
                        // nothing, and holds up nothing, none of which
                        // `wood` expresses. Falls back to the parent's
                        // material if the species' world has no `leaf`
                        // loaded, so a stripped-down asset set degrades to
                        // the old look rather than failing to grow.
                        let leaf_material = world.materials.id_of("leaf").unwrap_or(cell.material);
                        let leaf_shades = world.materials.get(leaf_material).palette.len().max(1) as u32;
                        let leaf_shade = rng.below(leaf_shades) as u8;
                        let leaf_cell =
                            Cell::new(leaf_material, leaf_shade).with_organism_id(organism_id).with_aux(organism::pack_cell_type(CellType::Leaf));
                        world.set(lx, ly, leaf_cell);
                        // A leaf belongs to the shoot that bore it, not to
                        // a new tier -- it never grows, so this only
                        // matters to anything reading order off foliage.
                        write_order(world, lx, ly, order);
                        deposit_canopy(world, lx, ly, GROW_CANOPY_DEPOSIT);
                        next.push(reschedule_organism(lx, ly, organism_id, 0, 0, world.frame + ORGANISM_TICK_INTERVAL));
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
                write_carbon(world, x, y, resource);
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
                    let n = world.get(nx, ny);
                    match world.materials.kind(n.material) {
                        MaterialKind::Liquid => {
                            world.set(nx, ny, Cell::EMPTY);
                            resource = (resource + rate).min(organism::RESOURCE_SCALE);
                            world.deplete_moisture(nx, ny, 1, ROOT_MOISTURE_DEPLETION);
                        }
                        // **Drink from damp soil** -- Decision 3's §4d path
                        // 2, and the fix for a gap `PLAN.md` recorded and
                        // could not close: "`RootTip` has no income source
                        // of its own besides `Absorb` (which only pays off
                        // once already touching water) -- a root with no
                        // adjacent water lives entirely off resource slowly
                        // diffusing over from the trunk, and can permanently
                        // go dormant... well before ever reaching a water
                        // pocket even a few cells away." Confirmed there at
                        // both 1,500 and 6,000 ticks as a permanent stall,
                        // not a timing issue.
                        //
                        // That is a *missing mechanism*, not a mis-tuned
                        // one, which is why the cost/rate tuning pass
                        // PLAN.md proposed as the remedy could never have
                        // fixed it: tuning cannot adjust an income source
                        // that does not exist. A root embedded in ordinary
                        // damp soil now has continuous income proportional
                        // to how damp that soil is.
                        //
                        // Credited on plant *available* water, so it is
                        // exactly zero at or below the wilting point. Soil
                        // drier than that gives a root nothing at all,
                        // which is what makes drought terminal rather than
                        // merely slow.
                        MaterialKind::Powder => {
                            let available = update::plant_available_fraction(n);
                            if available > 0.0 {
                                let drawn = (rate * available).min(organism::RESOURCE_SCALE - resource);
                                if drawn > 0.0 {
                                    resource += drawn;
                                    // Take the water actually drunk out of
                                    // the soil, so the loop closes: a root
                                    // dries the ground around itself, and
                                    // the moisture field notices.
                                    let taken = (drawn / rate.max(f32::EPSILON) * SOIL_UPTAKE_PER_TICK as f32) as u16;
                                    let left = update::soil_moisture(n).saturating_sub(taken);
                                    world.set(nx, ny, n.with_aux(left));
                                    world.deplete_moisture(nx, ny, 1, ROOT_MOISTURE_DEPLETION);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                write_carbon(world, x, y, resource);
            }
            Behavior::Transpire { rate } => transpire(world, x, y, rate),
            Behavior::SecondaryThicken { pipe_ratio } => {
                let _ = pipe_ratio;
                // Thickening runs from `step_organisms`, not from an active
                // site. Leaving it here would put every mature cell back on
                // the schedule, which is the cost this change removes.
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
                // **A seed germinates where it lands, never in mid-air.**
                // Now that a seed is a `Powder` it falls, and without this
                // it could meet its light and moisture conditions on the way
                // down and sprout a tree hanging in the air -- which is the
                // very first thing the owner reported about tree growth
                // ("the tree just starts growing in mid-air, no matter where
                // you place it"). Requiring something underneath makes that
                // structurally impossible rather than unlikely.
                //
                // A raw material test, not `world.is_empty`: the question is
                // "is there anything holding this up", which is what
                // `is_empty`'s managed-aware meaning does *not* answer.
                let resting = world.get(x, y + 1).material != material::EMPTY;
                let ready = resting
                    && (instant || {
                    let light = ambient_light_above(world, x, y);
                    let moisture = world.field_at(x, y).moisture;
                    light >= light_threshold && moisture >= moisture_threshold
                });
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

    // An ungerminated seed keeps the fast cadence -- it may still be
    // falling, and `relocated_seed`'s search bound assumes it.
    let interval = if cell_type == CellType::Seed { SEED_TICK_INTERVAL } else { ORGANISM_TICK_INTERVAL };
    // **A cell that is no longer a frontier leaves the schedule.** Its
    // upkeep runs from `step_organisms` over the organism's own cell list.
    // This is what makes the active-site heap track the number of *growing*
    // things rather than an organism's total size -- M16's own stated
    // principle ("plants only change at their tips -- a trunk is inert"),
    // which the implementation inverted for as long as `SecondaryThicken`
    // reported work on every mature cell forever.
    if !is_frontier(cell_type) && next.is_empty() {
        return next;
    }
    if found_candidate || !next.is_empty() {
        // A candidate existed this tick (whether or not any behavior's own
        // roll succeeded) -- reset the staleness counter, mirroring
        // `moss_tick`'s old reasoning: staleness tracks "had somewhere to
        // try", not "successfully grew".
        next.push(reschedule_organism(x, y, organism_id, 0, plastochron, world.frame + interval));
    } else if stale_ticks + 1 < ORGANISM_STALE_LIMIT {
        next.push(reschedule_organism(x, y, organism_id, stale_ticks + 1, plastochron, world.frame + interval));
    } else if matches!(cell_type, CellType::GrowingTip | CellType::RootTip) {
        // `Reports/tree-rewrite-design.md` §4: the staleness-limit
        // transition to `MatureBody` made real, not just asserted -- an
        // independent review of the design caught that describing this in
        // prose without an actual `world.set` here would leave
        // `StructuralAnchor`/`SecondaryThicken` (both gated on `MatureBody`
        // in `tree.ron`) never firing on anything, since nothing would
        // ever actually carry that cell type. Carries the tip's own
        // current `resource` forward rather than resetting it -- which is
        // now automatic: the retirement is a cell-type write, and the
        // sidecar entry it does not touch is where the resource lives.
        //
        // **Covers `RootTip` as well as `GrowingTip`, and used to cover
        // only the latter.** A `RootTip` that aged out therefore matched no
        // branch at all: never rescheduled, never retired, and skipped by
        // `organism_upkeep` (which ignores frontier cell types). It became
        // a phantom — invisible to every pass, yet still counted in
        // `root_cells` and still occupying a slot against `max_active_tips`,
        // so it went on tightening the allometry ratio that had blocked it
        // in the first place. Found by independent review.
        //
        // `MatureBody` is the right home for the same reason
        // `self_type_after_grow` already gives for the *successful* path:
        // settled root tissue genuinely does thicken and anchor, and that
        // is the behaviour set `tree.ron` attaches to `MatureBody`. This
        // just makes the two paths agree.
        world.set(x, y, cell.with_aux(organism::pack_cell_type(CellType::MatureBody)));
        write_carbon(world, x, y, resource);
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

/// Upkeep for every organism's **mature** cells — one pass per organism,
/// instead of one active site per cell.
///
/// **This is the structural half of the plant subsystem's cost problem.**
/// M16's own design states the principle plainly: "plants only change at
/// their tips — a trunk is inert", and only growing cells should stay
/// scheduled. The implementation inverted it. A `MatureBody` cell
/// rescheduled itself forever, because `SecondaryThicken` always reported
/// work to do, so the active-site heap grew with an organism's *total size*
/// rather than with its frontier — measured at 2,698 sites for six trees,
/// and 6s to 41s across a 3x frame increase.
///
/// Mature cells now leave the schedule entirely (`organism_tick` stops
/// rescheduling them) and are visited here through the organism's own cell
/// list, which is what `OrganismState::cells` was built for.
///
/// Two costs collapse at once:
///
/// - **The scheduler stops carrying them.** Sites fall to roughly the number
///   of live tips, which is what the heap was designed to hold.
/// - **`thicken()`'s flood fill is replaced by one row histogram.** Its leaf
///   count is "foliage above this row", and it was re-deriving that with a
///   whole-organism traversal per cell — quadratic in tree size. Counting
///   leaves per row once, then scanning downward, gives every cell the same
///   number in O(cells + rows). This is *exactly* the quantity the flood
///   fill produced: `thicken` already filtered its result to cells above,
///   and for one connected organism "reachable and above" and "above" are
///   the same set.
///
/// Organisms are staggered by id so they do not all fall due on one frame.
pub fn step_organisms(world: &mut World) {
    for organism_id in world.live_organism_ids() {
        // Spread the load: each organism keeps the same cadence as the
        // active-site schedule, on its own offset.
        if !(world.frame + organism_id as u64).is_multiple_of(ORGANISM_TICK_INTERVAL) {
            continue;
        }
        // Transport first, then upkeep. The order matters and is the same
        // order the two had before this pass existed: transport ran on the
        // CA sweep across the 45 frames *leading up to* this tick, so
        // upkeep has always read an already-diffused value. Running it
        // after would hand `Photosynthesize`/`Absorb`/decay the previous
        // tick's distribution.
        organism::transport(world, organism_id);
        allocate_to_frontier(world, organism_id);
        // Before both of the passes that read it.
        accumulate_support(world, organism_id);
        // Before upkeep, so a bud that flushes this tick is already a
        // `GrowingTip` when `thicken` runs and can be counted as frontier
        // rather than thickened over on the same tick it woke up.
        break_buds(world, organism_id);
        organism_upkeep(world, organism_id);
    }
}

/// Hand every growing tip a **share** of the plant's carbon, rather than
/// leaving it to whatever diffusion happened to deliver.
///
/// **This is the measured defect it exists for.** Instrumenting tip deaths
/// across 8 trees / 14,000 frames: 78.6% of tips died on the resource gate,
/// and at the moment of death the dying tip held 0.051 carbon while its
/// best neighbour held 0.971 — nearly 5x the growth cost. **88% of tips
/// died beside a cell that could have paid for them**, with the trunk
/// pinned at the `RESOURCE_SCALE` cap. The plant was never out of carbon.
/// Only the frontier was.
///
/// The cause is structural and it is polarity's doing.
/// `organism::transport` settles a face at `carbon[j] = carbon[i] ·
/// c_ij/c_ji`, and a fresh tip has basal conductance on every face while
/// the established strand beside it points *back down the plant*, toward
/// the sinks that earlier flux canalized. So the newest tissue sits at the
/// poorest end of a gradient built before it existed — measured at roughly
/// 1:19.
///
/// **A share, not a reserve — and that distinction is the whole point.**
/// `Reports/tree-procedural-prior-art.md` records that every published
/// model recomputes the plant's income and re-divides it among the
/// surviving frontier each cycle; none gives a bud a reserve it owns and
/// must defend. A share also cannot saturate everywhere at once, which is
/// the failure that killed the reverted bud break: shares sum to income by
/// construction, so adding tips lowers every tip's share and the system
/// self-throttles instead of running away.
///
/// Deliberately *not* a top-up from nowhere: carbon is moved out of the
/// richest cells, so the pass conserves. A plant with nothing to spare
/// gives its tips nothing, and starvation still kills — it just stops
/// killing tips that the plant could easily have fed.
/// Carbon contributed to the frontier's pool per unit of light a `Leaf`
/// intercepts, each organism tick.
///
/// This sets the exchange rate between *intercepted light* and
/// *extension*, which is the ratio that bounds a plant's size. Weighting by
/// light rather than by leaf count is the load-bearing part: leaf count
/// grows with the plant, so dividing it gave every tip a share that never
/// fell and the stand fused into a slab. Intercepted light does not grow
/// with the plant, because a canopy shades its own interior — which is
/// what makes the bound close.
///
/// Not yet a per-species value, and it should become one — see
/// `Reports/tree-architecture-research.md` §0b. Left as a constant only
/// because it has one caller and one species; the moment a second plant
/// form wants a different foliage economy it belongs in the `.ron`.
const LEAF_INCOME_PER_TICK: f32 = 0.05;

/// Flush at most one dormant bud into a `GrowingTip`, if the light the
/// plant is intercepting can support another one.
///
/// **This is the only thing in the engine that creates frontier**, and the
/// gate on it is the whole design. Two properties matter, and the reverted
/// bud-break attempt had neither:
///
/// 1. **The signal is whole-plant, not local.** Every local "am I idle"
///    quantity saturates at once when a plant stops growing — carbon fills
///    every cell to `RESOURCE_SCALE` (the transport clamp guarantees it),
///    crowding decays everywhere within two ticks, and conductance relaxes
///    to basal everywhere because there is no flux. So a local test fires
///    on *every* mature cell simultaneously and budding becomes
///    proportional to volume. Intercepted light does not do that: it is
///    bounded by foliage that shades itself.
/// 2. **The bound is a supportable count, not a cap.** `supportable` is
///    Palubicki's `n = ⌊v⌋` in miniature — income divided by what one tip
///    costs. Adding a tip does not change income, so each flush moves the
///    plant one step closer to its own ceiling and the ceiling moves only
///    when the crown actually catches more light. Capping instead ("one bud
///    per organism per tick") was considered and is not enough on its own:
///    it converts exponential growth into linear growth, which still fills
///    the world.
///
/// **Known defect: this gate is backwards for recovery, and it is why a
/// damaged plant does nothing.** Measured with `filmstrip`'s `cut=`: topping
/// two grown trees removed 1,344 living cells, and over the next 7,400
/// frames neither rebuilt a crown -- they sat flat-topped, with a faint
/// greening at the cut face and nothing more. The reason is right here.
/// `supportable` is driven by *intercepted light*, so losing foliage lowers
/// it: the event that should most urgently drive rebuilding instead reduces
/// the drive to rebuild. It is a rich-get-richer economy with no reserve.
///
/// Real plants resprout from **stored** reserves, and this one is holding
/// them -- a grown trunk sits at `RESOURCE_SCALE` throughout. The obvious
/// fix, adding stock to the numerator, is the exact mistake
/// `allocate_to_frontier` documents: stock grows with mass, so every tip's
/// share stays high forever and the stand fuses into a slab (38,605 cells
/// against 1,723). What separates the two cases is *memory* -- a plant that
/// has lost foliage should mobilise, a plant that never had any should not,
/// and only a high-water mark can tell them apart. That is Phase 3's
/// monotone girth memory, which is the prerequisite and is not built yet.
///
/// One per tick on top of that is a *rate* limit, not the bound — it keeps
/// a plant that has just been shaded from re-flushing its whole bud bank in
/// a single tick, and it keeps this function's cost at one pass.
/// Accumulate, basipetally, how much foliage each cell supports — the
/// **basipetal pass** of `Reports/tree-architecture-implementation-plan.md`
/// Phase 3, and Palubicki's `Q`.
///
/// One breadth-first walk rooted at the plant's below-ground tissue gives a
/// parent ordering; running that order in reverse sums each cell's own
/// intercepted light plus every child's total into its parent. Two linear
/// passes over a vector the organism already materialises each tick.
///
/// **Eight-neighbour, because `Grow` writes at eight.** A four-neighbour
/// walk sees a diagonally-grown shoot as a disconnected fragment, which is
/// a mistake this repo has already made once and recorded in `CLAUDE.md`.
///
/// A 2D thickened trunk is a *blob* of cells rather than a tree graph, so
/// the walk yields a spanning tree rather than the true topology; the
/// row-major sort makes which spanning tree deterministic, which is all
/// that is required.
fn accumulate_support(world: &mut World, organism_id: u16) {
    let Some(state) = world.organism(organism_id) else { return };
    let Some(collar) = state.collar_y else { return };
    let mut cells: Vec<(i32, i32)> = state.cells.keys().copied().collect();
    cells.sort_unstable_by_key(|&(x, y)| (y, x));
    let index: std::collections::HashMap<(i32, i32), usize> = cells.iter().enumerate().map(|(i, &p)| (p, i)).collect();

    // Roots first: everything at or below the collar is the anchor, so the
    // accumulation flows toward the ground the way sap pressure does.
    let mut order: Vec<usize> = Vec::with_capacity(cells.len());
    let mut parent: Vec<Option<usize>> = vec![None; cells.len()];
    let mut seen = vec![false; cells.len()];
    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for (i, &(_, y)) in cells.iter().enumerate() {
        if y >= collar {
            seen[i] = true;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        order.push(i);
        let (x, y) = cells[i];
        for (dx, dy) in NEIGHBOURS_8 {
            let Some(&j) = index.get(&(x + dx, y + dy)) else { continue };
            if seen[j] {
                continue;
            }
            seen[j] = true;
            parent[j] = Some(i);
            queue.push_back(j);
        }
    }

    // Each cell's own contribution, then the reverse sweep. A leaf is worth
    // what it actually earns -- `ambient_light_above`, the same quantity
    // `allocate_to_frontier` divides -- rather than 1, so a leaf buried in
    // canopy thickens nothing. Counting leaves equally is what made income
    // grow with mass and fused the stand into a slab.
    let mut q: Vec<f32> = cells
        .iter()
        .map(|&(x, y)| {
            let c = world.get(x, y);
            if c.organism_id() == organism_id && organism::cell_type(c.aux()) == Some(CellType::Leaf) {
                ambient_light_above(world, x, y)
            } else {
                0.0
            }
        })
        .collect();
    for &i in order.iter().rev() {
        if let Some(p) = parent[i] {
            q[p] += q[i];
        }
    }
    for (i, &(x, y)) in cells.iter().enumerate() {
        if let Some(slot) = world.organism_cell_mut(x, y) {
            // Max-accumulate: the high-water mark never falls. See
            // `OrganismCell::q_peak`.
            slot.q_peak = slot.q_peak.max(q[i]);
        }
    }
}

fn break_buds(world: &mut World, organism_id: u16) {
    let Some(state) = world.organism(organism_id) else { return };
    let species_id = state.species;
    let mut cells: Vec<(i32, i32)> = state.cells.keys().copied().collect();
    cells.sort_unstable_by_key(|&(x, y)| (y, x));

    // The price of a flush, and the cost this bud's tip will then pay per
    // growth step -- both read from the species' own `GrowingTip` `Grow`,
    // so a species cannot set them inconsistently.
    let (Some(cost), Some(bud_cost)) = (
        world.species.get(species_id).behaviors(CellType::GrowingTip).iter().find_map(|b| match b {
            Behavior::Grow { cost, .. } => Some(*cost),
            _ => None,
        }),
        world.species.get(species_id).behaviors(CellType::DormantBud).iter().find_map(|b| match b {
            Behavior::BudBreak { cost, .. } => Some(*cost),
            _ => None,
        }),
    ) else {
        return; // a species with no buds, or none that can break
    };

    let mut intercepted = 0.0f32;
    let mut tips = 0usize;
    let mut buds: Vec<(i32, i32, f32)> = Vec::new();
    let mut richest: Option<(i32, i32, f32)> = None;
    for &(x, y) in &cells {
        let cell = world.get(x, y);
        if cell.organism_id() != organism_id {
            continue;
        }
        match organism::cell_type(cell.aux()) {
            Some(CellType::Leaf) => intercepted += ambient_light_above(world, x, y),
            // Shoot tips only. A root tip is frontier too, but it is fed by
            // a different economy and does not compete for light.
            Some(CellType::GrowingTip) => tips += 1,
            // **Light discounted by how crowded the bud already is.**
            // Brightest-wins alone builds a flat cap: the brightest bud is
            // always the one on top of whatever the plant has already
            // built, so every flush lands on the same row and the crown
            // spreads sideways into a plate at exactly the turgor bound.
            // Dividing by local foliage moves the choice to buds with room
            // -- a dim bud on a bare stretch of stem beats a bright one
            // buried in canopy -- which is the same far-red reading
            // `candidate_crowding` uses, applied to *where to start* a
            // shoot rather than to where to extend one.
            Some(CellType::DormantBud) => {
                let light = ambient_light_above(world, x, y);
                buds.push((x, y, light / (1.0 + candidate_crowding(world, x, y))));
            }
            _ => {}
        }
        let held = world.carbon_at(x, y);
        if richest.is_none_or(|(_, _, best)| held > best) {
            richest = Some((x, y, held));
        }
    }
    if buds.is_empty() {
        return;
    }
    let supportable = (intercepted * LEAF_INCOME_PER_TICK / cost).floor() as usize;
    if tips >= supportable {
        return;
    }
    // Somebody has to pay for it. Drawn from the richest cell, which is
    // where the carbon actually is: the trunk sits at the cap while the
    // frontier starves, and that gradient is `organism::transport`'s doing
    // rather than an accident -- see `supply_direction`.
    let Some((rx, ry, held)) = richest else { return };
    if held < bud_cost {
        return;
    }

    // Best light-per-crowding, not the highest or the newest. A bud in deep
    // shade that flushes builds a shoot into shade, which then earns
    // nothing; a bud with no room builds a plate. Ties break by position so
    // the choice is deterministic.
    buds.sort_unstable_by(|a, b| b.2.total_cmp(&a.2).then((a.1, a.0).cmp(&(b.1, b.0))));
    let (bx, by, _) = buds[0];

    let cell = world.get(bx, by);
    world.set(bx, by, cell.with_aux(organism::pack_cell_type(CellType::GrowingTip)));
    write_carbon(world, rx, ry, held - bud_cost);
    write_carbon(world, bx, by, bud_cost);
    // A flushed bud is an *axillary* meristem -- it is a lateral by
    // definition, so it starts the next tier exactly as `Grow`'s own branch
    // child does. Without this a crown rebuilt from buds would inherit
    // trunk parameters and grow straight up as a second trunk.
    let order = world.organism_cell(bx, by).map_or(0, |c| c.order);
    write_order(world, bx, by, order.saturating_add(1));
    let site = reschedule_organism(bx, by, organism_id, 0, 0, world.frame + ORGANISM_TICK_INTERVAL);
    world.schedule_active_site(site);
}

fn allocate_to_frontier(world: &mut World, organism_id: u16) {
    let Some(state) = world.organism(organism_id) else { return };
    if state.cells.is_empty() {
        return;
    }
    // Sorted for the same determinism reason `transport` sorts: `cells` is
    // a `HashMap` and `f32` addition is not associative.
    let mut cells: Vec<(i32, i32)> = state.cells.keys().copied().collect();
    cells.sort_unstable_by_key(|&(x, y)| (y, x));

    let mut frontier: Vec<(i32, i32)> = Vec::new();
    let mut donors: Vec<(i32, i32)> = Vec::new();
    let mut intercepted = 0.0f32;
    for &(x, y) in &cells {
        let cell = world.get(x, y);
        if cell.organism_id() != organism_id {
            continue;
        }
        match organism::cell_type(cell.aux()) {
            Some(t) if is_frontier(t) => frontier.push((x, y)),
            Some(CellType::Leaf) => {
                // **Intercepted light, not leaf count** -- Palubicki's `Q`.
                // A leaf buried inside the canopy sits under blocked field
                // blocks and reads almost nothing, so it contributes almost
                // nothing. Counting leaves equally made income grow with
                // mass and the stand fused into a slab; weighting by light
                // is what makes self-shading bound the plant.
                intercepted += ambient_light_above(world, x, y);
                donors.push((x, y));
            }
            Some(_) => donors.push((x, y)),
            None => {}
        }
    }
    if frontier.is_empty() || donors.is_empty() {
        return;
    }

    // **The pool is bounded by foliage, not by the stock — and getting
    // this wrong is what turns the mechanism into a blob.**
    //
    // The first version divided everything the plant was *holding*. Stored
    // carbon grows with mass (every mature cell sits near
    // `RESOURCE_SCALE`), so the share per tip stayed high no matter how
    // many tips there were, growth compounded, and the stand fused into a
    // solid canopy — 38,605 cells against 1,723, with every tree merged
    // into its neighbours. That is precisely the failure
    // `Reports/tree-procedural-prior-art.md` warns of: *"the failure mode
    // is that `Q_base` grows in proportion to bud count, so every bud's
    // share stays above 1 forever."*
    //
    // In the literature the quantity divided is **income**, and income is
    // bounded by intercepted light. Leaf count is this engine's stand-in
    // for that: it is what actually earns carbon, and it is already limited
    // by self-shading, since a leaf buried in canopy sits under blocked
    // field blocks and reads almost no light.
    //
    // So the feedback that bounds growth is a ratio of two things the plant
    // itself changes: **foliage over frontier.** Adding tips without adding
    // leaves shrinks every tip's share until none can afford `cost`, and
    // extension stops without anything being killed or capped. Shed a limb
    // and the survivors' share rises again.
    let income = intercepted * LEAF_INCOME_PER_TICK;
    let stock: f32 = donors.iter().map(|&(x, y)| world.carbon_at(x, y)).sum();
    let pool = income.min(stock);
    let share = pool / frontier.len() as f32;
    if share <= 0.0 {
        return;
    }

    for &(fx, fy) in &frontier {
        let held = world.carbon_at(fx, fy);
        let mut wanted = (share - held).min(organism::RESOURCE_SCALE - held);
        if wanted <= 0.0 {
            continue;
        }
        // Drawn from the richest donors first, which is where the carbon
        // actually is -- the trunk sits at the cap while the tip starves.
        let mut order: Vec<(i32, i32)> = donors.clone();
        order.sort_unstable_by(|a, b| world.carbon_at(b.0, b.1).total_cmp(&world.carbon_at(a.0, a.1)));
        for (dx, dy) in order {
            if wanted <= 0.0 {
                break;
            }
            let available = world.carbon_at(dx, dy);
            let moved = wanted.min(available);
            if moved <= 0.0 {
                continue;
            }
            write_carbon(world, dx, dy, available - moved);
            wanted -= moved;
        }
        write_carbon(world, fx, fy, share.min(organism::RESOURCE_SCALE).max(held));
    }
}

fn organism_upkeep(world: &mut World, organism_id: u16) {
    let Some(state) = world.organism(organism_id) else {
        return;
    };
    let species_id = state.species;
    if state.cells.is_empty() {
        return;
    }
    // Sorted for the same reason `organism::transport` sorts: `cells` is a
    // `HashMap` with no stable iteration order, and this loop rolls a
    // per-cell RNG stream and can write cells (`thicken`), so an unstable
    // order makes a run non-reproducible. Row-major, matching the sweep.
    //
    // **This was a live determinism bug, not a precaution.** The previous
    // version iterated the `HashSet` directly, and Rust's default hasher is
    // seeded per *process*, so the same binary on the same scene gave 5877,
    // 5872 and 5881 organism cells on three consecutive runs. With the sort
    // it gives 5806 three times. `PLAN.md` requires same-build determinism;
    // it was not being met here.
    let mut cells: Vec<(i32, i32)> = state.cells.keys().copied().collect();
    cells.sort_unstable_by_key(|&(x, y)| (y, x));

    // How likely a bud is to survive being thickened past. Read once for
    // the organism rather than per cell: it belongs to the *bud's* own
    // `BudBreak`, but it is `thicken` -- running on a `MatureBody` several
    // cells away -- that consumes it, and a species defines it once.
    // Slot 4 of the genome. `pipe_ratio` lives on `SecondaryThicken` but
    // the genome lives on `Grow`, so it is read from the species' own
    // `GrowingTip` here rather than duplicated onto a second behaviour --
    // one plant, one genotype.
    let pipe_variance = world
        .species
        .get(species_id)
        .behaviors(CellType::GrowingTip)
        .iter()
        .find_map(|b| match b {
            Behavior::Grow { genotype_variance, .. } => Some(genotype_variance[4]),
            _ => None,
        })
        .unwrap_or(0.0);

    let bud_survival = world
        .species
        .get(species_id)
        .behaviors(CellType::DormantBud)
        .iter()
        .find_map(|b| match b {
            Behavior::BudBreak { thickening_survival, .. } => Some(*thickening_survival),
            _ => None,
        })
        .unwrap_or(1.0);

    // Leaves per row, then a running total downward, so every cell can read
    // "how much foliage do I carry" without a traversal of its own.
    let mut leaves_in_row: std::collections::HashMap<i32, u32> = std::collections::HashMap::new();
    let (mut root_cells, mut shoot_cells) = (0u32, 0u32);
    let mut collar_y: Option<i32> = None;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;
    for &(cx, cy) in &cells {
        min_y = min_y.min(cy);
        max_y = max_y.max(cy);
        let c = world.get(cx, cy);
        if c.organism_id() != organism_id {
            continue;
        }
        let ty = organism::cell_type(c.aux());
        if matches!(ty, Some(CellType::Leaf) | Some(CellType::GrowingTip)) {
            *leaves_in_row.entry(cy).or_insert(0) += 1;
        }
        // Root or shoot, tallied in the walk that is already happening.
        // `rootwood` is the discriminator rather than cell type, because a
        // retired root and a retired branch are both `MatureBody`.
        if world.materials.get(c.material).reinforces_powder || ty == Some(CellType::RootTip) {
            root_cells += 1;
        } else {
            shoot_cells += 1;
            // The collar is the *lowest* shoot cell -- where the shoot
            // meets the root system. Taken from shoot tissue rather than
            // from the organism's overall extent, which would sit at the
            // bottom of the root system and make every shoot cell read as
            // implausibly high.
            collar_y = Some(collar_y.map_or(cy, |c: i32| c.max(cy)));
        }
    }
    if let Some(state) = world.organism_mut(organism_id) {
        state.root_cells = root_cells;
        state.shoot_cells = shoot_cells;
        state.collar_y = collar_y;
    }
    let mut leaves_above: std::collections::HashMap<i32, u32> = std::collections::HashMap::new();
    let mut running = 0u32;
    for y in min_y..=max_y {
        leaves_above.insert(y, running);
        running += leaves_in_row.get(&y).copied().unwrap_or(0);
    }

    for (cx, cy) in cells {
        let cell = world.get(cx, cy);
        if cell.organism_id() != organism_id {
            continue; // burned, erased, or overwritten since the list was taken
        }
        let Some(cell_type) = organism::cell_type(cell.aux()) else {
            continue;
        };
        let mut resource = world.carbon_at(cx, cy);
        if is_frontier(cell_type) {
            continue; // still on the active-site schedule, handled there
        }

        let mut rng = rng::stream(organism_id as u64, cx as u64, cy as u64, world.frame);
        let mut behavior_buf = [None::<Behavior>; MAX_BEHAVIORS_PER_CELL_TYPE];
        let behavior_count = {
            let defined = world.species.get(species_id).behaviors(cell_type);
            let n = defined.len().min(MAX_BEHAVIORS_PER_CELL_TYPE);
            for (slot, behavior) in behavior_buf.iter_mut().zip(&defined[..n]) {
                *slot = Some(*behavior);
            }
            n
        };
        // Canopy density decays here too, or it never decays on mature
        // tissue at all. It used to fall out of `organism_tick` running on
        // every cell; taking mature cells off the schedule took their decay
        // with it, which would freeze old growth's crowding signal forever
        // and permanently bar new growth from reclaiming space near mature
        // wood -- the exact property `CANOPY_DENSITY_DECAY_PER_TICK`'s own
        // doc says the mechanism is for.
        //
        // **The `changed` flag that used to wrap this whole block is
        // gone.** It existed because both scalars lived in `aux`, so
        // updating either meant a `World::set` and a dirtied chunk, and a
        // mature cell decaying its density every tick kept the sweep awake
        // forever. Neither scalar is on the grid now, so neither write can
        // wake anything, and the guard has nothing left to protect.
        if let Some(slot) = world.organism_cell_mut(cx, cy) {
            slot.canopy_density *= CANOPY_DENSITY_DECAY_PER_TICK;
        }
        for behavior in behavior_buf.into_iter().take(behavior_count).flatten() {
            match behavior {
                Behavior::Photosynthesize { rate } => {
                    let light = ambient_light_above(world, cx, cy);
                    resource = (resource + rate * light).min(organism::RESOURCE_SCALE);
                }
                Behavior::Transpire { rate } => {
                    transpire(world, cx, cy, rate);
                }
                Behavior::SecondaryThicken { pipe_ratio } => {
                    // **The support this cell actually carries**, from the
                    // basipetal pass, replacing "leaves in the rows above
                    // me". The row scan was a geometric filter standing in
                    // for a topological one: a limb on the far side of the
                    // plant counted toward a stem it does not supply.
                    let carried = world.organism_cell(cx, cy).map_or(0.0, |c| c.q_peak);
                    thicken(world, cx, cy, organism_id, pipe_ratio * genotype(organism_id, 5, pipe_variance), carried, bud_survival, &mut rng);
                }
                // Frontier behaviours never run here -- a mature cell has
                // no growth to do, and `Germinate` belongs to a `Seed`.
                // Frontier behaviours never run here -- a mature cell has
                // no growth to do, `Germinate` belongs to a `Seed`, and
                // `Absorb` is a `RootTip`'s live water uptake (mature root
                // tissue is suberised and takes up little, which is why
                // `Transpire` above is what mature roots do instead).
                // `BudBreak` is here too, and it is the one behaviour that
                // deliberately does *not* run from the cell that carries
                // it. Its gate is a whole-organism quantity (income against
                // frontier), so `break_buds` evaluates it once per organism
                // per tick instead -- see that function for why a per-cell
                // version cannot work.
                Behavior::Grow { .. }
                | Behavior::Divide { .. }
                | Behavior::Germinate { .. }
                | Behavior::Absorb { .. }
                | Behavior::BudBreak { .. }
                | Behavior::StructuralAnchor => {}
            }
        }
        // Re-checked: a behaviour above may have destroyed this cell (fire,
        // a collapse) since it was sampled, and writing carbon into a slot
        // that has since changed hands would credit the wrong organism.
        if world.get(cx, cy).organism_id() == organism_id {
            write_carbon(world, cx, cy, resource);
        }
    }
}

/// Cell types that still carry their own active site: the ones that grow.
/// Everything else is upkeep, and runs from `step_organisms`.
fn is_frontier(cell_type: CellType) -> bool {
    matches!(cell_type, CellType::Seed | CellType::GrowingTip | CellType::RootTip)
}

/// Draw water from adjacent soil and lose it to the air.
///
/// No resource is credited: transpired water is lost, not eaten -- see
/// `TRANSPIRATION_PER_ROOT_CELL` for why this is the physically dominant
/// term and why crediting it would be wrong. Shared by the active-site
/// dispatch (a `RootTip`) and the per-organism upkeep pass (mature root
/// tissue), which are the two places root cells are visited from.
fn transpire(world: &mut World, x: i32, y: i32, rate: f32) {
    let draw = (TRANSPIRATION_PER_ROOT_CELL as f32 * rate) as u16;
    if draw == 0 {
        return;
    }
    for (dx, dy) in NEIGHBOURS_4 {
        let (nx, ny) = (x + dx, y + dy);
        let n = world.get(nx, ny);
        if world.materials.get(n.material).water_capacity == 0 {
            continue;
        }
        let held = update::soil_moisture(n);
        if held == 0 {
            continue;
        }
        world.set(nx, ny, n.with_aux(held.saturating_sub(draw)));
        // Vented upward rather than consumed: the moisture field is where
        // the air's humidity lives, so a stand of trees humidifies the air
        // above it.
        world.deplete_moisture(nx, ny, 1, -ROOT_MOISTURE_DEPLETION);
    }
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
    // The seed cell is `seed` material; the shoot it becomes is wood.
    let wood = world.materials.id_of("wood").unwrap_or(cell.material);
    let shades = world.materials.get(wood).palette.len().max(1) as u32;
    let shoot_shade = rng.below(shades) as u8;
    world.set(
        x,
        y,
        Cell::new(wood, shoot_shade).with_organism_id(organism_id).with_aux(organism::pack_cell_type(CellType::GrowingTip)),
    );
    let mut next = vec![reschedule_organism(x, y, organism_id, 0, 0, world.frame + ORGANISM_TICK_INTERVAL)];
    // The companion root starts wherever this species' own `RootTip` could
    // *grow*, which since Decision 1(ii) includes penetrable soil and not
    // just open air.
    //
    // This gate is why a seed planted on soil produced no root system at
    // all: the cell below a seed resting on the ground is, by definition,
    // ground. It read as a scene problem ("the test room has a stone
    // floor") and was really this — on soil it failed for the same reason,
    // just less obviously. Deliberately left alone until now rather than
    // loosened earlier: refusing to root in bare *stone* is correct
    // behaviour, so the gate could not be fixed honestly until roots had
    // somewhere legitimate to go.
    let root_force = world
        .species
        .get(world.organism(organism_id).map(|s| s.species).expect("germinating organism exists"))
        .behaviors(CellType::RootTip)
        .iter()
        .find_map(|b| match b {
            Behavior::Grow { penetration_force, .. } => Some(*penetration_force),
            _ => None,
        })
        .unwrap_or(0.0);
    if growable(world, x, y + 1, root_force) {
        // The companion root is `rootwood`, and that choice propagates for
        // free: every cell `Grow` creates copies its parent's material, so
        // the whole root system below ground comes out as rootwood while
        // the shoot above stays wood, with no cell-type-to-material table
        // anywhere. `update_powder`'s soil stabilization (§6d) depends on
        // being able to ask "is this a root" from the material id alone,
        // which is the reason rootwood is a material at all.
        let root_material = world.materials.id_of("rootwood").unwrap_or(cell.material);
        let shades = world.materials.get(root_material).palette.len().max(1) as u32;
        let shade = rng.below(shades) as u8;
        let root_cell = Cell::new(root_material, shade).with_organism_id(organism_id).with_aux(organism::pack_cell_type(CellType::RootTip));
        displace_soil_water(world, x, y + 1);
        world.set(x, y + 1, root_cell);
        next.push(reschedule_organism(x, y + 1, organism_id, 0, 0, world.frame + ORGANISM_TICK_INTERVAL));
    }
    next
}

// `MAX_THICKEN_SCAN_CELLS` is gone along with the flood fill it bounded --
// `step_organisms` computes the same leaf count once per organism, so there
// is no per-cell traversal left to cap.

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
/// The two grid steps perpendicular to the stem's own axis at `(x, y)` --
/// the direction a stem thickens *around* itself.
///
/// The axis comes from polarity: `supply_direction` is where carbon
/// arrives from, which is along the stem, so the perpendicular of it is the
/// cross-section. That is what the pipe model means by cross-section, and
/// it is the first consumer of the conductance field outside transport.
///
/// **Falls back to horizontal**, which is what this always used to be, when
/// no supply direction is established (a young cell, or `VEIN_GAIN` off).
/// So a vertical trunk behaves exactly as before -- its perpendicular *is*
/// left/right -- and only a diagonal or horizontal stem changes.
fn cross_section_axis(world: &World, x: i32, y: i32) -> [(i32, i32); 2] {
    let Some((ax, ay)) = organism::supply_direction(world, x, y) else {
        return [(-1, 0), (1, 0)];
    };
    // Perpendicular, then quantized onto the 8-neighbourhood. The threshold
    // is cos(67.5 deg), which splits the circle into eight equal sectors.
    let (px, py) = (-ay, ax);
    const T: f32 = 0.383;
    let sx = if px > T {
        1
    } else if px < -T {
        -1
    } else {
        0
    };
    let sy = if py > T {
        1
    } else if py < -T {
        -1
    } else {
        0
    };
    if sx == 0 && sy == 0 {
        return [(-1, 0), (1, 0)];
    }
    [(-sx, -sy), (sx, sy)]
}


#[allow(clippy::too_many_arguments)]
/// The contiguous run of *woody* same-organism cells through `(x, y)`
/// along the row — this stem's cross-section at this height.
///
/// **Leaves are excluded on purpose.** The pipe model's cross-section is
/// xylem, and foliage is not xylem. Counting leaves inflated the
/// denominator by roughly 10% of all cells, and worse, `leaf_count` on the
/// numerator counts `Leaf | GrowingTip` — so the same cell appeared on both
/// sides of the ratio.
fn stem_run(world: &World, x: i32, y: i32, organism_id: u16) -> usize {
    let woody = |wx: i32| {
        let c = world.get(wx, y);
        c.organism_id() == organism_id && organism::cell_type(c.aux()) != Some(CellType::Leaf)
    };
    if !woody(x) {
        return 1;
    }
    let mut run = 1usize;
    for dir in [-1i32, 1] {
        let mut k = 1;
        while woody(x + dir * k) && k <= MAX_STEM_RUN {
            run += 1;
            k += 1;
        }
    }
    run
}

/// Bound on the `stem_run` walk. A stem wider than this is not a stem, and
/// an unbounded walk would put an O(organism) cost back in the per-cell
/// path that `can_widen`'s early rejection exists to keep out.
const MAX_STEM_RUN: i32 = 32;

#[allow(clippy::too_many_arguments)]
fn thicken(world: &mut World, x: i32, y: i32, organism_id: u16, pipe_ratio: f32, leaf_count: f32, bud_survival: f32, rng: &mut Rng) {
    let axis = cross_section_axis(world, x, y);
    // **Cheapest possible rejection first.** The flood fill below is
    // O(organism size), and it ran for *every* `MatureBody` cell every tick
    // -- so the cost of thickening was quadratic in tree size, and measured
    // at roughly half of all simulation time for a six-tree stand (41s to
    // 20s with thickening disabled over 6,000 frames).
    //
    // A cell with no free side cannot thicken no matter what the count says,
    // and in a solid trunk that is almost every cell. Two `get`s instead of
    // a two-thousand-cell flood fill.
    //
    // Deliberately does *not* stop the cell rescheduling: a side can open
    // later (a neighbour burns, is dug out, or the tree grows past it), and
    // its downstream leaf count rises as the canopy grows, so going inert
    // here would permanently freeze a trunk that should still be thickening.
    // The saving is in the work per tick, not in the number of ticks.
    let can_widen = axis.iter().any(|&(dx, dy)| {
        let n = world.get(x + dx, y + dy);
        n.material == material::EMPTY
            || (n.organism_id() == organism_id && organism::cell_type(n.aux()) == Some(CellType::Leaf))
    });
    if !can_widen {
        return;
    }

    // `leaf_count` is the foliage this cell carries -- every `Leaf` or
    // `GrowingTip` above it -- and it arrives precomputed from
    // `step_organisms`.
    //
    // It used to be derived here, by flood filling the whole organism from
    // this cell and then filtering the result to cells above. That is the
    // same set for a connected organism, and it cost a traversal *per cell
    // per tick*, which made thickening quadratic in tree size and about
    // half of all simulation time. Counting leaves per row once for the
    // whole organism and scanning downward produces the identical number in
    // O(cells + rows).
    //
    // Shinozaki's pipe model is what makes "above" the right filter: a stem
    // cross-section is proportional to the leaf area it supplies, and
    // supply is directional -- a trunk carries the whole canopy, a twig
    // near the top carries almost nothing. Counting the entire organism
    // instead (which the undirected flood fill did before that filter was
    // added) thickens every cell equally and produces a slab.
    // **The gate compares foliage above against *this stem's* cross-section
    // at this height** -- Shinozaki's pipe model as stated.
    //
    // Three earlier versions all measured the wrong quantity, and the
    // history is worth keeping because each failed differently:
    //
    // - **Immediate neighbours.** The growing end of a run always had one
    //   neighbour behind it and open air ahead, so it read 2 however wide
    //   the trunk had become, passed forever, and spread sideways without
    //   limit.
    // - **A local probe** -- the run perpendicular to `supply_direction`,
    //   or tissue density in a disc. Both *under*-read inside a blob,
    //   because a blob is porous, so both ran away. The disc was worse than
    //   the axis walk (31,591 wood against 12,039).
    // - **The row total.** Fixed the under-reading, and introduced
    //   systematic *over*-reading on a branched tree: an independent review
    //   measured **53% of occupied rows containing more than one separate
    //   run**, so a limb elsewhere on the same row silently suppressed
    //   thickening in the trunk. Worst case observed was 23 cells across 9
    //   runs read as one 23-wide stem.
    //
    // The per-stem run is the quantity the pipe model actually names, and
    // it is viable now in a way it was not before: it under-read inside a
    // blob, and the allocation change means the plant is no longer a blob.
    // If blobs ever return, this reverts to under-reading -- so a failure
    // here looks like runaway thickening, and the guard below is aimed at
    // exactly that.
    let stem_width = stem_run(world, x, y, organism_id);
    if (leaf_count / stem_width as f32) <= pipe_ratio {
        return;
    }
    // Which side to try first is a coin flip, not always left.
    //
    // This used to be a flat `[(-1, 0), (1, 0)]` with a `break`, so a cell
    // with open space on *both* sides always thickened leftward -- and
    // since every thickening cell made the same choice, a trunk fattened
    // entirely to one side instead of around itself. Reported from live
    // play as thickening looking "kind of weird", which it did: the shape
    // was one-sided by construction.
    //
    // Exactly the bug this repo already fixed once for liquids -- see
    // `68371d7`, "Alternate which edge a body sheds, instead of always the
    // left", where a promoted body spread in one direction only for the
    // same reason. Worth naming as a class: a two-option loop with a
    // `break` is a directional bias unless something breaks the tie.
    //
    // Drawn from the organism's own stream (`rng::stream`), so it stays
    // deterministic and stays independent of what every other organism is
    // doing.
    let sides = if rng.flip() { axis } else { [axis[1], axis[0]] };
    for (dx, dy) in sides {
        let (nx, ny) = (x + dx, y + dy);
        // A thickening stem may grow *through* its own leaves, not only
        // into open air.
        //
        // This is what keeps foliage on twigs rather than on the trunk,
        // and it is emergent rather than authored — there is no rule here
        // about height, or about "trunk" versus "branch", neither of which
        // a cell can locally know. `thicken()` fires where the downstream
        // leaf count is high relative to local width, which is near the
        // base; those cells then consume the leaves beside them, while
        // distal twigs carry too small a downstream count to thicken and
        // keep theirs. A trunk ends up bare and the outer canopy leafy as
        // a *side effect* of the pipe model, which is the shape
        // `design-philosophy.md` §2b asks for.
        //
        // Grounded: secondary growth really does sever leaf traces. A
        // thickening stem cuts off the vascular connection to leaves borne
        // on it, and they are shed — which is why mature trunks are bare
        // and why the exceptions (epicormic shoots on trunks) are exactly
        // the case that needs a *separate* mechanism, per
        // `research/m16-plant-biology.md` §5's fire-resprouting note.
        //
        // The leaf is consumed outright rather than becoming falling
        // detritus. Decision 4's abscission is what makes shed foliage a
        // real object with a lifespan; until it lands, a leaf overwritten
        // by wood is the honest minimum and is noted as a simplification
        // rather than dressed up.
        let own_leaf = {
            let n = world.get(nx, ny);
            n.organism_id() == organism_id && organism::cell_type(n.aux()) == Some(CellType::Leaf)
        };
        if world.is_empty(nx, ny) || own_leaf {
            let cell = world.get(x, y);
            let shades = world.materials.get(cell.material).palette.len().max(1) as u32;
            let shade = rng.below(shades) as u8;
            let new_cell = Cell::new(cell.material, shade).with_organism_id(organism_id).with_aux(organism::pack_cell_type(CellType::MatureBody));
            world.set(nx, ny, new_cell);
            // Wood laid beside a trunk cell is that trunk, so it inherits
            // the order rather than starting a tier. Nothing reads order
            // off a `MatureBody` today, but leaving it 0 would quietly make
            // every thickened limb read as trunk the moment something does.
            let order = world.organism_cell(x, y).map_or(0, |c| c.order);
            write_order(world, nx, ny, order);
            // **The cambium kills the buds it outpaces**, and that is
            // where the clear bole comes from -- for free, with no rule
            // about height and none about which cells are trunk. The trunk
            // is what thickens, so the trunk is what loses its buds; a twig
            // carries too small a downstream leaf count to thicken and
            // keeps every bud it has. Epicormic resprouting on a mature
            // trunk is exactly the exception that needs its own mechanism
            // later, per research/m16-plant-biology.md 5.
            for (bdx, bdy) in NEIGHBOURS_8 {
                let (px, py) = (x + bdx, y + bdy);
                let b = world.get(px, py);
                if b.organism_id() != organism_id || organism::cell_type(b.aux()) != Some(CellType::DormantBud) {
                    continue;
                }
                if !rng.chance(bud_survival) {
                    world.set(px, py, b.with_aux(organism::pack_cell_type(CellType::MatureBody)));
                }
            }
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
        let aux = organism::pack_cell_type(CellType::GrowingTip);
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
        // `seed` material, not `wood`: a seed is a Powder and falls to the
        // ground on its own rather than hanging wherever it was placed.
        // Falls back to `wood` so a stripped asset set still plants.
        let seed_material = self.materials.id_of("seed").or_else(|| self.materials.id_of("wood"));
        let Some(seed_material) = seed_material else {
            return false;
        };
        let Some(tree_species) = self.species.id_of(species_name) else {
            return false;
        };
        if !self.is_empty(x, y) {
            return false;
        }
        let shades = self.materials.get(seed_material).palette.len().max(1) as u32;
        let shade = self.rng.below(shades) as u8;
        let organism_id = self.push_organism(tree_species);
        let aux = organism::pack_cell_type(CellType::Seed);
        self.set(x, y, Cell::new(seed_material, shade).with_organism_id(organism_id).with_aux(aux));
        // Checked often while it is still falling -- see SEED_TICK_INTERVAL.
        let site = reschedule_organism(x, y, organism_id, 0, 0, self.frame + SEED_TICK_INTERVAL);
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

    /// Place an organism-owned cell and set its sidecar scalars.
    ///
    /// Replaces the `with_aux(pack_aux(ty, carbon))` one-liner these tests
    /// used before Decision 2 step 2c. Two steps rather than one now, and
    /// the order is not optional: `World::set` is what registers the
    /// `OrganismCell`, so the scalars can only be written after it.
    fn place(w: &mut World, (x, y): (i32, i32), m: material::MaterialId, organism_id: u16, ty: CellType, (carbon, density): (f32, f32)) {
        w.set(x, y, Cell::new(m, 0).with_organism_id(organism_id).with_aux(organism::pack_cell_type(ty)));
        if let Some(slot) = w.organism_cell_mut(x, y) {
            slot.carbon = carbon;
            slot.canopy_density = density;
        }
    }

    /// Plant a seed **on ground**, which every tree test now needs.
    ///
    /// A seed is a `Powder` and falls, and `Germinate` requires something
    /// underneath it, so a seed planted into open air drops to the bottom
    /// of the world instead of sprouting where it was put. Every tree test
    /// here used to plant into mid-air and rely on it germinating there --
    /// they were encoding the very first thing the owner reported about
    /// tree growth ("the tree just starts growing in mid-air, no matter
    /// where you place it"), which is now structurally impossible.
    ///
    /// The floor is one row of stone directly under the seed, wide enough
    /// that a seed which rolls a little still lands on it.
    fn plant_tree_on_ground(w: &mut World, x: i32, y: i32) {
        for fx in (x - 6)..=(x + 6) {
            w.set(fx, y + 1, Cell::new(material::STONE, 0));
        }
        w.plant_tree(x, y);
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
        plant_tree_on_ground(&mut w, 50, 50);
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
        place(&mut w, (50, 50), material, organism_id, CellType::GrowingTip, (start_resource, 0.0));

        organism_tick(&mut w, 50, 50, organism_id, 0, 0);

        let parent_resource = w.carbon_at(50, 50);
        // `damp_chance`/`dry_chance` are both 1.0, so exactly one of the
        // four open neighbours divides successfully -- the RNG only picks
        // *which* one.
        let total_child_resource: f32 = NEIGHBOURS_4
            .iter()
            .filter(|&&(dx, dy)| w.get(50 + dx, 50 + dy).organism_id() == organism_id)
            .map(|&(dx, dy)| w.carbon_at(50 + dx, 50 + dy))
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
        plant_tree_on_ground(&mut w, 50, 20);
        plant_tree_on_ground(&mut w, 150, 20);
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
        plant_tree_on_ground(&mut w, 50, 20);
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
        plant_tree_on_ground(&mut w, 50, 20);
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
        plant_tree_on_ground(&mut w, 50, 20);
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
        plant_tree_on_ground(&mut w, 50, 20);
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
        for individual in 0..8u16 {
        // **Several individuals, not one.** `genotype_variance` gives each
        // organism its own branch chance at +/-60%, so a single tree either
        // branching or not is a test of one draw rather than of the rule --
        // and organism 1 draws low enough to never fork. Same correction as
        // `a_lateral_starts_the_next_branch_order_and_a_continuation_does_
        // not`, which was bitten by this first.
        let mut w = test_world();
        let tree = w.species.id_of("tree").expect("tree is a compiled-in species");
        for _ in 0..individual {
            w.push_organism(tree);
        }
        plant_tree_on_ground(&mut w, 100, 20);
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
        if branched {
            return;
        }
        }
        panic!("not one of eight individuals grown to completion in open sky produced a branch point (3+ same-organism neighbours)");
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
        place(&mut w, (50, 50), wood, organism_id, CellType::GrowingTip, (2.0, 0.0));

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
        place(&mut w, (50, 50), wood, organism_id, CellType::RootTip, (2.0, 0.0));

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
        plant_tree_on_ground(&mut w, 100, 20);
        run_with_fields(&mut w, 400); // several germination checks (ORGANISM_TICK_INTERVAL apart)

        let cell_type = organism::cell_type(w.get(100, 20).aux());
        assert_ne!(cell_type, Some(CellType::Seed), "a seed in open sky should have germinated, not stayed a Seed forever");
    }

    #[test]
    fn photosynthesize_gains_resource_in_open_sky_despite_its_own_position_self_blocking_light() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let wood = w.materials.id_of("wood").unwrap();
        let organism_id = w.push_organism(tree_species);
        let aux = organism::pack_cell_type(CellType::GrowingTip);
        w.set(100, 20, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(aux));
        let site = reschedule_organism(100, 20, organism_id, 0, 0, w.frame + ORGANISM_TICK_INTERVAL);
        w.schedule_active_site(site);

        run_with_fields(&mut w, (ORGANISM_TICK_INTERVAL as usize) * 3);

        let resource = w.carbon_at(100, 20);
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
        place(&mut w, (49, 50), wood, organism_id, CellType::GrowingTip, (0.0, 2.0));
        // The candidate itself is empty -- reading its own aux directly
        // (the bug this guards against) would always read exactly 0.0.
        assert!(w.is_empty(50, 50));

        let density = candidate_crowding(&w, 50, 50);
        assert!(density > 0.0, "candidate_crowding should see the neighbour's deposited density, not the always-empty candidate's own aux, got {density}");
    }

    /// **Reversed, deliberately, and this test used to assert the
    /// opposite.** It was
    /// `candidate_crowding_ignores_a_different_organisms_density`, and it
    /// was a fair reading while this channel was framed as *self*-
    /// avoidance.
    ///
    /// `Reports/tree-architecture-research.md` §7c reframes it: the channel
    /// is a stand-in for shade-avoidance signalling, where a shoot senses
    /// the red/far-red ratio of light reflected off nearby foliage — and a
    /// phytochrome cannot ask whose leaf it came off. Owner-blindness is
    /// the mechanism, not a relaxation of it, and it is what produces
    /// **crown shyness**: the gaps real adjacent trees leave between their
    /// canopies.
    ///
    /// It matters here more than in a real forest because this world is 2D.
    /// A crown has `~R³` of volume to branch into in three dimensions and
    /// `~R²` of area in two, so neighbouring structures merge far more
    /// readily than any 3D-calibrated model expects, and one owner-blind
    /// rule keeps a tree from merging with itself *and* with its neighbour.
    #[test]
    fn candidate_crowding_sees_a_neighbouring_organisms_foliage_too() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let wood = w.materials.id_of("wood").unwrap();
        let this_organism = w.push_organism(tree_species);
        let other_organism = w.push_organism(tree_species);
        let _ = this_organism;
        place(&mut w, (49, 50), wood, other_organism, CellType::GrowingTip, (0.0, 3.0));

        let density = candidate_crowding(&w, 50, 50);
        assert!(
            density > 0.0,
            "a neighbouring organism's foliage must register as crowding -- that is crown shyness,              and it is the one rule keeping two trees from merging in a 2D world. Got {density}"
        );
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
        place(&mut w, (100, 20), wood, organism_id, CellType::MatureBody, (1.0, GROW_CANOPY_DEPOSIT));

        // One full organism_tick cycle on this cell itself -- the same
        // cadence a neighbour's own Grow check would be running on.
        let _ = organism_tick(&mut w, 100, 20, organism_id, 0, 0);

        let density = w.canopy_density_at(100, 20);
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
        place(&mut w, (100, 20), wood, organism_id, CellType::MatureBody, (1.0, organism::CANOPY_DENSITY_SCALE));

        for _ in 0..20 {
            let _ = organism_tick(&mut w, 100, 20, organism_id, 0, 0);
        }

        let density = w.canopy_density_at(100, 20);
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
        place(&mut w, (100, 100), wood, organism_id, CellType::GrowingTip, (2.0, 0.0));

        let next = organism_tick(&mut w, 100, 100, organism_id, 0, 0);

        let self_type = organism::cell_type(w.get(100, 100).aux());
        assert_eq!(self_type, Some(CellType::MatureBody), "a GrowingTip that just grew should retire to MatureBody, not stay an equally-eligible growth candidate");

        // Exactly one newly created cell nearby should carry the frontier
        // forward as the new active GrowingTip.
        let new_tips: Vec<(i32, i32)> = NEIGHBOURS_8
            .iter()
            .map(|&(dx, dy)| (100 + dx, 100 + dy))
            .filter(|&(nx, ny)| organism::cell_type(w.get(nx, ny).aux()) == Some(CellType::GrowingTip))
            .collect();
        assert_eq!(new_tips.len(), 1, "expected exactly one new GrowingTip child, got {new_tips:?}");
        assert!(!next.is_empty(), "the new child should be scheduled");
    }

    /// The gate on Decision 2 step 1: every organism's cell list must agree
    /// exactly with a full scan of the grid.
    ///
    /// The design doc calls this step "where the real bugs are", because a
    /// list maintained at a dozen creation and removal sites drifts the
    /// moment one is missed — and a *stale* entry pointing at a cell that
    /// is now someone else's is far worse than a leaked one. Registration
    /// hooks `World::set` instead, so it is complete by construction, and
    /// this asserts that claim against every path a real run exercises
    /// rather than against the paths anyone remembered to list.
    ///
    /// Deliberately exercises destruction as well as growth: fire burning
    /// cells away, an explosion, and the brush erasing straight through a
    /// trunk are all removal paths that know nothing about organisms.
    #[test]
    fn every_organism_cell_list_agrees_with_the_grid() {
        let mut w = test_world();
        plant_tree_on_ground(&mut w, 60, 40);
        plant_tree_on_ground(&mut w, 120, 40);
        w.plant_moss_seed(30, 39);
        run_with_fields(&mut w, 4000);

        let check = |w: &World, when: &str| {
            let b = w.bounds().unwrap();
            let mut scanned: std::collections::HashMap<u16, std::collections::HashSet<(i32, i32)>> = Default::default();
            for y in b.min_y..=b.max_y {
                for x in b.min_x..=b.max_x {
                    let id = w.get(x, y).organism_id();
                    if id != 0 {
                        scanned.entry(id).or_default().insert((x, y));
                    }
                }
            }
            for (&id, cells) in &scanned {
                let state = w.organism(id).unwrap_or_else(|| panic!("{when}: organism {id} owns cells but has no state"));
                let listed: std::collections::HashSet<(i32, i32)> = state.cells.keys().copied().collect();
                assert_eq!(&listed, cells, "{when}: organism {id}'s cell list disagrees with the grid");
            }
            // And nothing recorded that is no longer really there.
            for id in scanned.keys() {
                if let Some(state) = w.organism(*id) {
                    for &(cx, cy) in state.cells.keys() {
                        assert_eq!(w.get(cx, cy).organism_id(), *id, "{when}: {id} lists ({cx},{cy}) which it does not own");
                    }
                }
            }
        };
        check(&w, "after growth");

        // Destruction paths that have no idea organisms exist.
        w.paint_circle(60, 30, 6, material::EMPTY);
        run_with_fields(&mut w, 200);
        check(&w, "after erasing through a canopy");

        w.ignite_circle(120, 36, 5);
        run_with_fields(&mut w, 3000);
        check(&w, "after fire");
    }

    /// The same agreement, under the **parallel** driver, with a seed that
    /// actually moves — and then a check that the seedling grows.
    ///
    /// **This is the test whose absence cost a whole debugging session.**
    /// `every_organism_cell_list_agrees_with_the_grid` above runs
    /// `update::step`, the serial driver, so it never constructs a
    /// `ChunkView` and cannot observe the one write seam that bypasses
    /// `World::set` (`parallel::ChunkView::set`'s same-chunk branch).
    /// `CLAUDE.md`'s "two drivers, and the app runs the parallel one —
    /// test both" is exactly this, and the cost of ignoring it was a
    /// falling seed dropping out of its own organism's cell list while
    /// remaining in the grid.
    ///
    /// **A seed dropped from a height, not planted on the ground**, because
    /// a seed is the only organism cell that moves and the desync only
    /// happens on a move. Planted at rest, this passes against the broken
    /// code — which is what made the bug look like tree-shape variance
    /// rather than corruption.
    ///
    /// It asserts growth as well as bookkeeping, deliberately. A list that
    /// merely *agrees* is not the property that matters; the property is
    /// that a cell missing from the list becomes unreachable to everything
    /// keyed on it — `carbon_at` reads 0, `write_carbon` is a silent no-op
    /// and `transport` never visits it — so the seedling germinates and
    /// then never grows again. Asserting agreement alone would pass on a
    /// future variant that keeps the list honest but loses the scalars.
    #[test]
    fn a_falling_seed_stays_in_its_organisms_cell_list_under_the_parallel_driver() {
        let mut w = test_world();
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
        for x in 40..160 {
            for y in 100..106 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
            for y in 60..100 {
                w.set(x, y, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
            }
        }
        // 25 cells of fall, matching `filmstrip`'s `forest` scene, which is
        // where this was found.
        w.plant_tree(100, 35);

        for _ in 0..4000 {
            super::super::parallel::step(&mut w);
            w.step_active_sites();
            field::step(&mut w);
        }

        let b = w.bounds().unwrap();
        let mut scanned: std::collections::HashMap<u16, std::collections::HashSet<(i32, i32)>> = Default::default();
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let id = w.get(x, y).organism_id();
                if id != 0 {
                    scanned.entry(id).or_default().insert((x, y));
                }
            }
        }
        assert!(!scanned.is_empty(), "the seed should have landed and germinated at all");
        for (&id, cells) in &scanned {
            let state = w.organism(id).unwrap_or_else(|| panic!("organism {id} owns cells but has no state"));
            let listed: std::collections::HashSet<(i32, i32)> = state.cells.keys().copied().collect();
            assert_eq!(&listed, cells, "organism {id}'s cell list disagrees with the grid after a fall under the parallel driver");
        }

        let grown: usize = scanned.values().map(|c| c.len()).sum();
        assert!(
            grown > 20,
            "a seedling from a dropped seed should have grown into a tree, got {grown} cells -- \
             a shoot missing from the cell list reads 0 carbon forever and stalls at germination"
        );
    }

    /// A root growing into soil must displace the water that was there,
    /// not delete it.
    ///
    /// The cell a root grows into is a `Powder` whose `aux` *is* its
    /// moisture, and `Grow` writes straight over it. Before this, every
    /// root cell destroyed whatever that cell held — in the `forest` scene,
    /// 620 units each. No conservation tally covers held water (the liquid
    /// tallies only know about `Liquid` cells), so nothing caught it.
    #[test]
    fn a_root_growing_into_soil_displaces_its_water_rather_than_destroying_it() {
        let mut w = test_world();
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");

        // A patch of soil, the middle cell full and the rest with room.
        for x in 49..=51 {
            for y in 49..=51 {
                w.set(x, y, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY / 2));
            }
        }
        w.set(50, 50, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));

        let total_before: u32 = (49..=51)
            .flat_map(|x| (49..=51).map(move |y| (x, y)))
            .map(|(x, y)| update::soil_moisture(w.get(x, y)) as u32)
            .sum();

        displace_soil_water(&mut w, 50, 50);

        let moved_out = update::soil_moisture(w.get(50, 50)) as u32;
        let total_after: u32 = (49..=51)
            .flat_map(|x| (49..=51).map(move |y| (x, y)))
            .map(|(x, y)| update::soil_moisture(w.get(x, y)) as u32)
            .sum();

        // The centre cell's own reading is untouched -- the caller is about
        // to overwrite it -- so conservation is measured as "everything
        // that was there is now in the neighbours plus the doomed cell".
        assert_eq!(
            total_after, total_before,
            "displacing must conserve: the neighbourhood held {total_before} and now holds {total_after}"
        );
        assert!(
            moved_out < material::SOIL_FIELD_CAPACITY as u32,
            "the cell about to be overwritten should have handed its water on, still holds {moved_out}"
        );
    }

    /// The turgor gate gives a **derived** height ceiling that does not
    /// depend on the scene.
    ///
    /// `h_max = (turgor_source − turgor_yield) / turgor_per_cell`. Every
    /// bound tried before this one was either an arbitrary cap or a ratio,
    /// and ratios bound *proportions* rather than size — a fact that cost a
    /// session to establish. This one falls out of three species numbers.
    ///
    /// **Scene-independence is the property under test**, not the exact
    /// value. Every plant conclusion on this branch until now was taken in
    /// a world where trees grew until they hit the top edge, so shape
    /// numbers measured the scene rather than the plant. A bound that moves
    /// when the sky gets deeper is not a bound.
    #[test]
    fn the_turgor_gate_caps_height_independently_of_how_much_sky_there_is() {
        let heights: Vec<i32> = [140, 220]
            .iter()
            .map(|&ground| {
                let scene = common_scene(ground);
                let mut w = scene;
                for _ in 0..30_000 {
                    super::super::parallel::step(&mut w);
                    w.step_active_sites();
                    field::step(&mut w);
                }
                let b = w.bounds().expect("world has bounds");
                // The *tallest* tree, because the claim under test is a
                // ceiling: no tree may exceed it, and at least one should
                // approach it. A mean would be dominated by the
                // establishment failures instead.
                let top = (b.min_y..=b.max_y)
                    .find(|&y| (b.min_x..=b.max_x).any(|x| w.get(x, y).organism_id() != 0))
                    .unwrap_or(ground);
                ground - top
            })
            .collect();

        // `tree.ron`'s own numbers give 0.9 / 0.0075 = 120 rows. Allow
        // slack for the tip that crosses the threshold mid-step and for
        // ordinary run-to-run spread, but not enough to let a
        // scene-dependent result through.
        for &h in &heights {
            assert!(h < 160, "turgor should cap height near 120 rows, got {h}");
        }
        // The scene-independence claim: 80 extra rows of sky must not buy
        // 80 extra rows of tree. Before the turgor gate it bought all of
        // them, every time, in every scene tried.
        let spread = (heights[0] - heights[1]).abs();
        assert!(
            spread < 45,
            "height must not track available sky -- 80 extra rows of it changed the tallest tree by {spread} rows: {heights:?}"
        );
    }

    /// Builds the same soil/stone/seed geometry `examples/common` does, so
    /// the guard above is asking about the harness scene rather than a
    /// bespoke one.
    fn common_scene(ground: i32) -> World {
        let mut w = World::new(Rect::new(0, 0, 255, ground + 60));
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
        for x in 0..256 {
            for y in (ground + 34)..(ground + 40) {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
            for y in ground..(ground + 34) {
                w.set(x, y, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
            }
        }
        // Four trees, not one. Per-tree spread on this branch measures
        // 53x and 2-3 in 8 fail to establish at all, so a single draw
        // cannot say whether a *cap* was reached -- only whether that one
        // tree happened to reach it.
        for i in 1..=4 {
            w.plant_tree(i * 51, ground - 25);
        }
        w
    }

    /// A `RootTip` that ages out must retire, not vanish.
    ///
    /// At the staleness limit only a `GrowingTip` used to retire to
    /// `MatureBody`. A `RootTip` matched no branch: it was never
    /// rescheduled, never retired, and `organism_upkeep` skips frontier
    /// cell types — so nothing visited it again. It stayed a `RootTip`
    /// forever, invisible to every pass, while still counting toward
    /// `root_cells` and still holding a slot against `max_active_tips`,
    /// tightening the very allometry ratio that had blocked it.
    ///
    /// Driven through the allometry gate specifically, because that is the
    /// condition that produces the failure in the field: an organism that
    /// is already more than `MAX_ROOT_FRACTION` root blocks every root tip
    /// on every tick, so they all age out together.
    #[test]
    fn a_root_tip_that_ages_out_retires_instead_of_becoming_a_phantom() {
        let mut w = test_world();
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let rootwood = w.materials.id_of("rootwood").unwrap_or(wood);
        let tree = w.species.id_of("tree").expect("tree is a compiled-in species");
        let organism_id = w.push_organism(tree);

        // An organism that is overwhelmingly root, so the allometry gate
        // blocks its tip every tick.
        for x in 40..60 {
            place(&mut w, (x, 60), rootwood, organism_id, CellType::MatureBody, (4.0, 0.0));
        }
        place(&mut w, (50, 61), rootwood, organism_id, CellType::RootTip, (4.0, 0.0));
        // Directly, not via `step_organisms`, which is frame-gated on
        // `ORGANISM_TICK_INTERVAL` and would silently not run at frame 0.
        organism_upkeep(&mut w, organism_id); // refresh root_cells / shoot_cells
        let state = w.organism(organism_id).expect("live");
        assert!(
            state.root_cells as f32 / (state.root_cells + state.shoot_cells) as f32 >= MAX_ROOT_FRACTION,
            "the scene must actually trip the allometry gate, or this tests nothing"
        );

        // Enough ticks to exhaust the staleness counter.
        for stale in 0..=ORGANISM_STALE_LIMIT {
            organism_tick(&mut w, 50, 61, organism_id, stale, 0);
        }

        assert_eq!(
            organism::cell_type(w.get(50, 61).aux()),
            Some(CellType::MatureBody),
            "a root tip blocked by allometry until it aged out must retire to MatureBody, not stay              an unschedulable RootTip that still counts against root_cells and max_active_tips"
        );
    }

    /// A widening trunk must stop widening, and the *end* of the run is
    /// where the old rule could not tell that it had.
    ///
    /// `thicken`'s gate is `leaf_count / stem_width > pipe_ratio`, where
    /// `stem_width` is the contiguous run of *woody* same-organism cells
    /// through this cell — the pipe model's cross-section at a height.
    ///
    /// Four earlier versions measured the wrong quantity and each failed
    /// differently: immediate neighbours (the run's end always read 2), the
    /// run perpendicular to `supply_direction` and tissue density in a disc
    /// (both under-read inside a porous blob and ran away), and the row
    /// total (over-read on a branched tree, where 53% of occupied rows hold
    /// more than one separate run, so a limb suppressed the trunk).
    #[test]
    fn a_trunk_already_at_its_pipe_model_width_refuses_to_widen_further() {
        let mut w = test_world();
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let tree = w.species.id_of("tree").expect("tree is a compiled-in species");
        let organism_id = w.push_organism(tree);

        // A five-cell horizontal run, open air on both ends.
        for x in 50..55 {
            place(&mut w, (x, 50), wood, organism_id, CellType::MatureBody, (1.0, 0.0));
        }
        let mut rng = rng::stream(organism_id as u64, 54, 50, 0);

        // 30 leaves above, against `tree.ron`'s own pipe_ratio of 10.
        thicken(&mut w, 54, 50, organism_id, 10.0, 30.0, 1.0, &mut rng);

        assert_eq!(
            w.get(55, 50).organism_id(),
            0,
            "a five-wide trunk carrying 30 leaves is already past the pipe model's bound (30/5 = 6 <= 10)              and must not widen further -- reading only immediate neighbours makes the run's end see              width 2 forever, which is the unbounded lateral slab"
        );

        // ...and it still widens when it genuinely is too thin for its load.
        let mut rng = rng::stream(organism_id as u64, 54, 50, 1);
        thicken(&mut w, 54, 50, organism_id, 10.0, 300.0, 1.0, &mut rng);
        assert_ne!(w.get(55, 50).organism_id(), 0, "300 leaves on a five-wide trunk is under-built and should still thicken");
    }

    /// **A limb elsewhere on the same row must not stop the trunk
    /// thickening.**
    ///
    /// This is the case that retired the row-total denominator. A branched
    /// tree has several separate runs across most of its height — an
    /// independent review measured **53% of occupied rows** holding more
    /// than one, worst case 23 cells spread over 9 runs read as a single
    /// 23-wide stem. Summing them makes every limb suppress every other
    /// limb on its row, and suppress the trunk hardest, because the trunk
    /// is the one carrying enough foliage for the gate to matter.
    ///
    /// Constructed so the two denominators give opposite answers: a
    /// 3-wide trunk under 40 leaves is under-built (40/3 = 13.3 > 10 --
    /// thicken), while the row total of 3 + 5 = 8 says it is fine
    /// (40/8 = 5 <= 10 -- refuse). Under the row-total version this
    /// assertion fails.
    #[test]
    fn a_separate_limb_on_the_same_row_does_not_suppress_the_trunk() {
        let mut w = test_world();
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let tree = w.species.id_of("tree").expect("tree is a compiled-in species");
        let organism_id = w.push_organism(tree);

        // The trunk: three cells at x=20..23.
        for x in 20..23 {
            place(&mut w, (x, 50), wood, organism_id, CellType::MatureBody, (1.0, 0.0));
        }
        // A limb of the same tree, five cells, well clear across the row.
        for x in 40..45 {
            place(&mut w, (x, 50), wood, organism_id, CellType::MatureBody, (1.0, 0.0));
        }

        let mut rng = rng::stream(organism_id as u64, 22, 50, 0);
        thicken(&mut w, 22, 50, organism_id, 10.0, 40.0, 1.0, &mut rng);

        assert_ne!(
            w.get(23, 50).organism_id(),
            0,
            "a 3-wide trunk carrying 40 leaves is under-built (40/3 = 13.3 > 10) and must thicken;              summing it with an unrelated 5-wide limb on the same row reads 8 across and refuses"
        );
    }

    /// Leaves are foliage, not xylem, and must not count as cross-section.
    ///
    /// They were on **both sides** of the ratio: `leaf_count` counts
    /// `Leaf | GrowingTip`, and the row total counted every organism cell
    /// including those same leaves. A leafy row therefore inflated its own
    /// denominator with the very cells whose load it was meant to carry.
    #[test]
    fn foliage_is_not_counted_as_trunk_cross_section() {
        let mut w = test_world();
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let leaf = w.materials.id_of("leaf").expect("leaf is a compiled-in material");
        let tree = w.species.id_of("tree").expect("tree is a compiled-in species");
        let organism_id = w.push_organism(tree);

        // Three cells of trunk, then five leaves continuing the same run.
        for x in 20..23 {
            place(&mut w, (x, 50), wood, organism_id, CellType::MatureBody, (1.0, 0.0));
        }
        for x in 23..28 {
            place(&mut w, (x, 50), leaf, organism_id, CellType::Leaf, (1.0, 0.0));
        }

        // Called on the trunk's *outer* cell -- the middle of a solid run
        // has no free side, so `can_widen` rejects it before the gate.
        let mut rng = rng::stream(organism_id as u64, 20, 50, 0);
        thicken(&mut w, 20, 50, organism_id, 10.0, 40.0, 1.0, &mut rng);

        assert_ne!(
            w.get(19, 50).organism_id(),
            0,
            "the stem here is 3 wood cells, not 8 -- counting the attached leaves as              cross-section says 40/8 = 5 <= 10 and refuses to thicken an under-built trunk"
        );
    }

    /// The same failure again, one seam over: a seed whose final fall step
    /// **crosses a chunk boundary**.
    ///
    /// `ChunkView::set` splits writes in two. A same-chunk write goes
    /// straight into the worker's own `Chunk` and is handled by
    /// `ChunkOutcome::organism_moves`. A *remote* write is queued and
    /// replayed by `run_pass` — through `World::set_owned`, which calls
    /// `write_cell` directly and therefore skips `World::set`'s
    /// `reindex_organism_cell` exactly as the same-chunk path does. Fixing
    /// only the first half left the second half broken, and the comment on
    /// `organism_moves` asserted the opposite ("a remote write is replayed
    /// through `World::set`") — an independent review caught it.
    ///
    /// Geometry is deliberate rather than incidental: `CHUNK_SIZE` is 64,
    /// so soil topped at y=65 makes the seed's last step land on **y=64**,
    /// the first row of chunk row 1, written from a cell in chunk row 0.
    /// That single step is the only remote write in the whole fall.
    ///
    /// The y=128 boundary would do the same but cannot be used: `field.rs`'s
    /// `LIGHT_DECAY` puts `Germinate`'s threshold around 75 rows below open
    /// sky, so a seedling there is too dark to grow and the test would fail
    /// for a reason that has nothing to do with the seam.
    #[test]
    fn a_seed_landing_across_a_chunk_boundary_stays_in_its_cell_list() {
        let mut w = test_world();
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
        // Stone under the soil, or the soil is a `Powder` with nothing
        // beneath it and falls out of the scene -- which reads exactly like
        // "the mechanism does nothing" and is the scene error `CLAUDE.md`
        // names twice.
        for x in 40..120 {
            for y in 80..87 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
            for y in 65..80 {
                w.set(x, y, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
            }
        }
        w.plant_tree(60, 30);

        for _ in 0..4000 {
            super::super::parallel::step(&mut w);
            w.step_active_sites();
            field::step(&mut w);
        }

        let b = w.bounds().unwrap();
        let mut scanned: std::collections::HashMap<u16, std::collections::HashSet<(i32, i32)>> = Default::default();
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let id = w.get(x, y).organism_id();
                if id != 0 {
                    scanned.entry(id).or_default().insert((x, y));
                }
            }
        }
        assert!(!scanned.is_empty(), "the seed should have landed and germinated");
        for (&id, cells) in &scanned {
            let state = w.organism(id).unwrap_or_else(|| panic!("organism {id} owns cells but has no state"));
            let listed: std::collections::HashSet<(i32, i32)> = state.cells.keys().copied().collect();
            assert_eq!(&listed, cells, "organism {id}'s cell list disagrees with the grid after a cross-chunk landing");
        }
        let grown: usize = scanned.values().map(|c| c.len()).sum();
        assert!(grown > 10, "a seedling that landed across a chunk boundary should still grow, got {grown} cells");
    }

    /// Decision 1(ii): a root grows *into* penetrable soil, a shoot does
    /// not, and neither goes through stone however hard it pushes.
    ///
    /// Driven through `growable` directly rather than a full simulation:
    /// the gate is the whole mechanism, and a grown-tree test would make
    /// "did a root reach this cell" depend on the resource economy as well
    /// (which is exactly the still-open `RootTip` income stall).
    #[test]
    fn a_root_penetrates_soil_but_not_stone_and_a_shoot_penetrates_neither() {
        let mut w = test_world();
        let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
        let gravel = w.materials.id_of("gravel").expect("gravel is a compiled-in material");
        w.set(10, 10, Cell::new(soil, 0));
        w.set(11, 10, Cell::new(material::STONE, 0));
        w.set(12, 10, Cell::new(gravel, 0));

        // `tree.ron`'s own RootTip force, and its GrowingTip's (0.0).
        const ROOT: f32 = 1.2;
        const SHOOT: f32 = 0.0;

        assert!(growable(&w, 10, 10, ROOT), "a root should push through loose soil (0.8 MPa against 1.2)");
        assert!(!growable(&w, 11, 10, ROOT), "no root may enter Solid stone, whatever its force");
        assert!(!growable(&w, 12, 10, ROOT), "gravel at 3.5 MPa is past the 2-3 MPa bound where root elongation stops");
        assert!(!growable(&w, 10, 10, SHOOT), "a canopy shoot has no penetrating force and must stay in open air");
        assert!(growable(&w, 50, 50, SHOOT), "open air is always growable");
    }

    /// Leaves hang *off* the stem, never form part of it — asserted the way
    /// the bug would actually have surfaced rather than by inspecting
    /// adjacency.
    ///
    /// Decision 4 gives leaves a lifespan and abscission. While the
    /// plastochron converted a retiring stem cell *into* a `Leaf` (the
    /// design doc's §5a as written), shedding foliage would have cut the
    /// trunk into disconnected pieces every plastochron — a "trees fall
    /// apart" bug landing two phases after the change that caused it. This
    /// deletes every `Leaf` and asserts the woody skeleton is still one
    /// connected piece, which is precisely what abscission will do.
    ///
    /// It also covers the structural half: `organism_is_supported` filters
    /// on `organism_id` and `Plant` kind without ever checking cell type,
    /// so a leaf embedded in a stem silently carried load, which the design
    /// doc's own §6a forbids.
    #[test]
    fn shedding_every_leaf_does_not_disconnect_the_stem() {
        // **The setup has to hunt for an individual that grew leaves.**
        // The plastochron is jittered at +/-40% per organism, and in a
        // 20-row sky a tree that draws a long interval finishes with none
        // at all -- organism 1 does exactly that. Searching for a suitable
        // individual is honest here in a way that widening the assertion
        // would not be: what is under test is what happens *after* leaves
        // are shed, so a tree with no leaves is not a weaker case, it is
        // not the case at all.
        let (mut w, organism_id) = (0..8u16)
            .find_map(|individual| {
                let mut w = test_world();
                let tree = w.species.id_of("tree").expect("tree is a compiled-in species");
                for _ in 0..individual {
                    w.push_organism(tree);
                }
                plant_tree_on_ground(&mut w, 100, 20);
                run_with_fields(&mut w, 8000);
                let b = w.bounds()?;
                let id = (b.min_y..=b.max_y)
                    .flat_map(|y| (b.min_x..=b.max_x).map(move |x| (x, y)))
                    .map(|(x, y)| w.get(x, y).organism_id())
                    .find(|&id| id != 0)?;
                let any_leaf = (b.min_y..=b.max_y).flat_map(|y| (b.min_x..=b.max_x).map(move |x| (x, y))).any(|(x, y)| {
                    let c = w.get(x, y);
                    c.organism_id() == id && organism::cell_type(c.aux()) == Some(CellType::Leaf)
                });
                any_leaf.then_some((w, id))
            })
            .expect("test setup: none of eight individuals grew a leaf to shed");

        let b = w.bounds().unwrap();
        // Abscise everything, the way a lifespan eventually will.
        let leaves: Vec<(i32, i32)> = (b.min_y..=b.max_y)
            .flat_map(|y| (b.min_x..=b.max_x).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let c = w.get(x, y);
                c.organism_id() == organism_id && organism::cell_type(c.aux()) == Some(CellType::Leaf)
            })
            .collect();
        assert!(!leaves.is_empty(), "test setup: the tree should have grown leaves to shed");
        for &(x, y) in &leaves {
            w.set(x, y, Cell::EMPTY);
        }

        let wood: Vec<(i32, i32)> = (b.min_y..=b.max_y)
            .flat_map(|y| (b.min_x..=b.max_x).map(move |x| (x, y)))
            .filter(|&(x, y)| w.get(x, y).organism_id() == organism_id)
            .collect();
        assert!(!wood.is_empty(), "test setup: shedding leaves should not have removed the whole tree");

        // One flood fill from any surviving cell must reach all of them.
        let is_plant = |c: Cell| c.organism_id() == organism_id && w.materials.kind(c.material) == MaterialKind::Plant;
        let reached = organism::reachable_from_anchors(&w, [wood[0]], is_plant, 100_000);
        assert_eq!(
            reached.len(),
            wood.len(),
            "shedding every leaf left the stem in more than one piece: {} of {} cells reachable from the base. \
Leaves must hang off the stem, not be segments of it",
            reached.len(),
            wood.len()
        );
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
        plant_tree_on_ground(&mut w, 100, 20);
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
    ///
    /// **Also the guard on `ByOrder` reaching the plastochron at all.** The
    /// interval is per branch order now (`tree.ron` runs `[12, 5, 2, 2]`),
    /// so the same lineage step leafs at one order and not at another. A
    /// version that read order 0 for every cell -- which is what a missing
    /// `write_order` after a `World::set` silently produces -- passes the
    /// order-0 rows below and fails the order-2 ones.
    #[test]
    fn a_retiring_tip_becomes_a_leaf_once_per_plastochron() {
        let wood = material::MaterialId(11);
        // **The interval is read from the species rather than written
        // here**, because it is now two things composed: the per-order list
        // *and* this individual's `genotype_variance` draw. Hard-coding
        // `tree.ron`'s numbers made this test fail the moment jitter landed
        // -- correctly, but uselessly, since what it is guarding is that
        // `ByOrder` reaches the plastochron at all, not what the species
        // file happens to say today.
        //
        // Entering a tick with `plastochron = n` makes this lineage step
        // `n + 1`, and a leaf is due when that is a multiple of the
        // interval for this cell's order.
        for (order, offset, expect_leaf) in
            [(0u8, 0i32, true), (0, 1, false), (0, -1, false), (2, 0, true), (2, 1, false), (3, 0, true), (3, -1, false)]
        {
            let mut w = test_world();
            let tree = w.species.id_of("tree").expect("tree is a compiled-in species");
            let organism_id = w.push_organism(tree);
            let (base, variance) = w
                .species
                .get(tree)
                .behaviors(CellType::GrowingTip)
                .iter()
                .find_map(|b| match b {
                    Behavior::Grow { plastochron, genotype_variance, .. } => Some((*plastochron, *genotype_variance)),
                    _ => None,
                })
                .expect("tree's GrowingTip grows");
            let interval = ((base.at(order) as f32 * genotype(organism_id, 3, variance[2])).round() as u8).max(1);
            // Two whole intervals in, then stepped off it by `offset`, so
            // the "due" and "not due" cases are the same distance apart
            // whatever the interval turns out to be.
            let Some(entering) = (interval as i32 * 2 - 1 + offset).try_into().ok().filter(|&e: &u8| e < u8::MAX) else {
                continue;
            };
            place(&mut w, (100, 100), wood, organism_id, CellType::GrowingTip, (2.0, 0.0));
            write_order(&mut w, 100, 100, order);

            organism_tick(&mut w, 100, 100, organism_id, 0, entering);

            // The retiring cell is *always* wood now. It used to become the
            // leaf itself, which built the stem out of alternating wood and
            // foliage -- see `self_type_after_grow`'s own comment for the
            // three things that broke.
            // Still wood, never foliage -- but a node retires to
            // `DormantBud` rather than plain `MatureBody`, which is the
            // metamer's third part (internode + leaf + bud) and the same
            // event that placed the leaf.
            let self_type = organism::cell_type(w.get(100, 100).aux());
            let expected = if expect_leaf { CellType::DormantBud } else { CellType::MatureBody };
            assert!(interval > 1, "order {order}'s interval collapsed to {interval}; the offsets below stop discriminating");
            assert_eq!(
                self_type,
                Some(expected),
                "a retiring tip must always become wood, never a leaf, whatever the plastochron says --                  and a node's wood is a bud"
            );

            let leaves = NEIGHBOURS_8
                .iter()
                .map(|&(dx, dy)| (100 + dx, 100 + dy))
                .filter(|&(nx, ny)| {
                    let c = w.get(nx, ny);
                    c.organism_id() == organism_id && organism::cell_type(c.aux()) == Some(CellType::Leaf)
                })
                .count();
            assert_eq!(
                leaves,
                usize::from(expect_leaf),
                "order {order} entering a growth step with plastochron={entering} against its own interval of {interval}: should have produced {} lateral leaf, got {leaves}",
                usize::from(expect_leaf)
            );
        }
    }

    /// Order is inherited straight ahead and incremented sideways.
    ///
    /// The whole architecture mechanism in one assertion: without the
    /// increment every cell is a trunk and `ByOrder` is an expensive way to
    /// write a scalar.
    ///
    /// **Run over several individuals, not one, and the reason is
    /// measured.** Branching is a roll at `branch_chance[0] = 0.03`, and a
    /// lineage only gets as many rolls as it gets growth steps before the
    /// turgor bound stops it — about a hundred. `genotype_variance` then
    /// moves both numbers per individual, so a single tree branching inside
    /// any fixed budget is a coin flip. An earlier version ran one organism
    /// for 400 ticks and passed until jitter landed, at which point that
    /// organism drew a low branch chance and a high per-cell turgor cost
    /// and never branched at all. Twelve individuals is a test of the rule;
    /// one is a test of one draw.
    #[test]
    fn a_lateral_starts_the_next_branch_order_and_a_continuation_does_not() {
        let wood = material::MaterialId(11);
        let mut branched = 0;
        let mut trunk_seen = false;
        for individual in 0..12u16 {
            let mut w = test_world();
            let tree = w.species.id_of("tree").expect("tree is a compiled-in species");
            // Burn ids so each pass is a genuinely different genotype --
            // `genotype` is keyed on the organism id and nothing else.
            let mut organism_id = w.push_organism(tree);
            for _ in 0..individual {
                organism_id = w.push_organism(tree);
            }
            place(&mut w, (100, 100), wood, organism_id, CellType::GrowingTip, (2.0, 0.0));

            let mut orders: Vec<u8> = Vec::new();
            for frame in 0..600 {
                w.frame = frame;
                let tips: Vec<(i32, i32)> = w
                    .organism(organism_id)
                    .expect("organism")
                    .cells
                    .keys()
                    .copied()
                    .filter(|&(x, y)| organism::cell_type(w.get(x, y).aux()) == Some(CellType::GrowingTip))
                    .collect();
                if tips.is_empty() {
                    break; // every lineage retired -- nothing left to roll
                }
                for (x, y) in tips {
                    write_carbon(&mut w, x, y, 2.0);
                    organism_tick(&mut w, x, y, organism_id, 0, 0);
                }
                orders = w
                    .organism(organism_id)
                    .expect("organism")
                    .cells
                    .keys()
                    .filter_map(|&(x, y)| w.organism_cell(x, y).map(|c| c.order))
                    .collect();
                if orders.contains(&1) {
                    break;
                }
            }
            trunk_seen |= orders.contains(&0);
            branched += usize::from(orders.contains(&1));
        }

        assert!(trunk_seen, "the original shoot must stay at order 0");
        assert!(
            branched > 0,
            "not one of twelve individuals produced an order-1 cell -- every cell reading 0 means `write_order` never ran              on the branch child, which looks exactly like a species file with no tiers"
        );
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
    ///
    /// **The leaf bar dropped from 6 to 2 when the plastochron became
    /// per-branch-order**, and that is the mechanism working rather than
    /// regressing. `tree.ron` runs `[12, 5, 2, 2]`: the trunk tier leafs a
    /// quarter as often as the old flat 3 did, which is what clears the
    /// bole. This scene is 20 rows of sky, so its trees are almost entirely
    /// order 0 and see the sparsest tier and nothing else — it measured 4
    /// here against 663 stand-wide leaves in the 200-row harness scene
    /// (`plant_probe -- trees=8`, up from 521). A 20-row scene is the wrong
    /// place to judge foliage; it is still the right place to catch zero.
    #[test]
    fn grown_trees_produce_leaves_and_a_trunk_thicker_than_one_cell() {
        let mut w = test_world();
        for x in [40, 90, 140] {
            plant_tree_on_ground(&mut w, x, 20);
        }
        run_with_fields(&mut w, 8000);

        let b = w.bounds().unwrap();
        let mut leaves = 0;
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                if organism::cell_type(w.get(x, y).aux()) == Some(CellType::Leaf) && w.get(x, y).organism_id() != 0 {
                    leaves += 1;
                }
            }
        }
        assert!(leaves >= 2, "three grown trees should carry real Leaf cells; got {leaves} across all three (baseline before the plastochron: 0, measured here: 4)");

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
