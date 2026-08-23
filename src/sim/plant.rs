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
        // Normalised so 1.0 means "beside a fresh deposit" — the same
        // move as `INCOME_PER_NODE`'s `L_node`, applied to the crowding
        // currency. Raw `canopy_density` is denominated in
        // `GROW_CANOPY_DEPOSIT`s and decay steps, so `crowding_weight` had
        // to be an order of magnitude larger than its sibling weights to
        // mean anything, and moved whenever the deposit or the decay
        // cadence did. In deposit units the weight reads beside
        // `continuation_weight` and friends, and survives both.
        sum / count as f32 / GROW_CANOPY_DEPOSIT
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
/// Drink from adjacent water and damp soil, crediting **water**.
///
/// Shared by the active-site dispatch (a live `RootTip`) and the
/// per-organism upkeep pass (mature root tissue), which is the change that
/// makes the water economy able to balance at all.
///
/// **Uptake has to scale with root *mass*, not with tip count.** `Absorb`
/// was a `RootTip`-only behaviour, and `tree.ron` caps root tips at 10,
/// so a plant's entire water income was bounded by a constant while its
/// demand grows with a canopy of well over a thousand leaves. No setting of
/// any constant balances those two, because one of them does not scale.
/// Giving mature tissue the same behaviour makes income proportional to how
/// much root is actually in contact with damp soil — which is the quantity
/// that *should* decide it, and the one root depth and spread finally buy
/// something for.
///
/// It runs on every `MatureBody` cell, not only root ones, and that is
/// deliberate rather than sloppy: the test is contact with water-bearing
/// soil, so a trunk cell in open air draws nothing and pays only its
/// four-neighbour look. A collar cell half-buried in damp ground drinking a
/// little is right, not a bug.
fn absorb_water(world: &mut World, x: i32, y: i32, rate: f32) {

            // **Credits `water`, not `carbon`, and that one word is
            // the whole of "make roots matter".** Both arms below used
            // to add to `resource` -- the same pool `Photosynthesize`
            // fills -- so water and carbon were one currency, a root
            // supplied nothing a leaf did not already make, and a plant
            // with no roots at all ran no deficit. `Reports/plant-
            // substrate-v2-design.md` §10 step 8 called this out as the
            // step where "water becomes a real second currency with a
            // real source"; it was the half that never landed.
    let organism_id = world.get(x, y).organism_id();
    let (stock, _) = world.water_at(x, y);
    let capacity = world.organism(organism_id).map_or(0.0, |st| water_capacity_of(st.root_cells));
    let mut water = stock;
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
                        water = (water + rate).min(capacity);
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
                            let drawn = (rate * available).min(capacity - water);
                            if drawn > 0.0 {
                                water += drawn;
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
    credit_water(world, organism_id, water - stock);
}

/// Mark (or clear) a cell as a primed lateral site — see
/// `OrganismCell::primed`.
#[cfg(test)]
pub(crate) static S8E: [std::sync::atomic::AtomicU64; 6] = [
    std::sync::atomic::AtomicU64::new(0),
    std::sync::atomic::AtomicU64::new(0),
    std::sync::atomic::AtomicU64::new(0),
    std::sync::atomic::AtomicU64::new(0),
    std::sync::atomic::AtomicU64::new(0),
    std::sync::atomic::AtomicU64::new(0),
];

fn write_primed(world: &mut World, x: i32, y: i32, primed: bool) {
    if let Some(slot) = world.organism_cell_mut(x, y) {
        slot.primed = primed;
    }
}

fn write_carbon(world: &mut World, x: i32, y: i32, carbon: f32) {
    if let Some(slot) = world.organism_cell_mut(x, y) {
        slot.carbon = carbon;
    }
}

/// Loose soil's authored `penetration_resistance` at the time the root
/// economy was calibrated — the "normal ground" a root's `Grow.cost` was
/// tuned against. A named constant rather than a live lookup because the
/// baseline must not silently move if soil is re-authored; if it is,
/// re-derive the root economy rather than letting every root's bill
/// change as a side effect.
const PENETRATION_COST_BASELINE: f32 = 0.8;

/// The carbon multiplier for growing *into* `(x, y)` — hard ground is
/// expensive ground, which is what makes `penetration_force` a trait with
/// a bill instead of a free unlock (`Reports/plant-genome-design.md`
/// §4.7): without it selection saturates the force high and the slot
/// stops varying. Open air (and anything not a resisting `Powder`) is
/// 1.0; penetrated ground scales by its resistance against loose soil's,
/// so tree roots pay ~1x in soil, ~1.75x in sand, ~4.4x in gravel.
fn penetration_cost_mult(world: &World, x: i32, y: i32) -> f32 {
    let m = world.materials.get(world.get(x, y).material);
    if m.kind == MaterialKind::Powder && m.penetration_resistance > 0.0 {
        (m.penetration_resistance / PENETRATION_COST_BASELINE).max(1.0)
    } else {
        1.0
    }
}

/// This organism's leaf-economy multipliers, `(rate, transpiration)` —
/// `LOCUS_LEAF_ECONOMY` applied. `(1.0, 1.0)` for anything unregistered,
/// which keeps every non-plant caller and every pre-germination cell at
/// the species mean.
fn leaf_econ_mults(world: &World, organism_id: u16) -> (f32, f32) {
    world.organism(organism_id).map_or((1.0, 1.0), |s| {
        let a = (s.alleles[organism::LOCUS_LEAF_ECONOMY] as usize).min(organism::LEAF_RATE_ALLELES.len() - 1);
        (organism::LEAF_RATE_ALLELES[a], organism::LEAF_TRANSPIRATION_ALLELES[a])
    })
}

/// This organism's wood-density multiplier — see `organism::wood_density`
/// for why every site that budgets in units of `Grow.cost` must apply it,
/// not only the site that spends it. `1.0` for anything unregistered.
fn wood_density_mult(world: &World, organism_id: u16) -> f32 {
    world.organism(organism_id).map_or(1.0, |s| organism::wood_density(&s.alleles))
}

/// How much water a plant can hold, **proportional to its root mass**.
///
/// This is the whole reason root depth and spread buy anything: the stock
/// is what carries a plant through a dry spell, and only roots make it
/// bigger. A shallow-rooted individual runs on whatever it drew this tick;
/// a deep-rooted one has a buffer. It is also the quantity anchorage will
/// read when "weak roots topple" lands, which is the coupling that makes
/// root mass one number with two consequences.
///
/// A floor of one root cell's worth so a seedling that has just germinated,
/// with no root system at all yet, still has somewhere to put its first
/// drink.
fn water_capacity_of(root_cells: u32) -> f32 {
    organism::WATER_SCALE * root_cells.max(1) as f32
}

/// Settle one organism's water balance for this tick — the arithmetic
/// half of `organism_upkeep`'s closing block, pulled out so the identity
/// below can be asserted directly rather than inferred from a stand.
///
/// Returns `(drawn, status, desiccation)`: what leaves the stock, the
/// **stomatal term** every photosynthetic credit is multiplied by, and
/// the **desiccation term** `drought_death` sheds on.
///
/// **The two terms must not be collapsed back into one.** `status` is the
/// fraction of demand actually spent — closure-limited, so a prudent
/// individual reads low — and `desiccation` is the fraction it would have
/// fallen short of *with stomata fully open*, which is the only one that
/// says a leaf is drying out. Keying shedding on the spent side would
/// make the conservative allele shed hardest while protecting its stock
/// and the stomatal locus would select against itself
/// (`Reports/plant-genome-design.md` §4.3).
///
/// The seam that keeps the water session's `drought_death` tuning valid:
/// at `reserve <= 0` openness is 1, both draws are the same number, and
/// **`desiccation == 1 - status` exactly**. That identity is
/// `settle_water_keeps_desiccation_and_status_identical_without_a_reserve`
/// and it is what says this change was free for every species that has
/// not opted in.
fn settle_water(stock: f32, capacity: f32, demand: f32, reserve: f32) -> (f32, f32, f32) {
    // Openness ramps linearly from shut at an empty tank to fully open at
    // the reserve line; `reserve <= 0` is the pre-closure engine exactly,
    // not an approximation of it.
    let openness = if reserve <= 0.0 { 1.0 } else { ((stock / capacity.max(f32::EPSILON)) / reserve).clamp(0.0, 1.0) };
    let open_drawn = stock.min(demand);
    let drawn = stock.min(demand * openness);
    let status = if demand > 0.0 { drawn / demand } else { 1.0 };
    let desiccation = if demand > 0.0 { 1.0 - open_drawn / demand } else { 0.0 };
    (drawn, status, desiccation)
}

/// Add to the organism's water stock, bounded by `water_capacity_of`.
fn credit_water(world: &mut World, organism_id: u16, amount: f32) {
    if let Some(state) = world.organism_mut(organism_id) {
        let cap = water_capacity_of(state.root_cells);
        let before = state.water;
        state.water = (state.water + amount).min(cap);
        state.water_uptake_acc += state.water - before;
    }
}

/// How full this cell's water store is, `0.0..=1.0` — the **stomatal
/// term**.
///
/// Photosynthesis is multiplied by it, which is the whole coupling that
/// makes roots matter: a plant that cannot replace what its leaves lose
/// closes its stomata and stops earning, exactly as a real one does. It is
/// a fraction of `WATER_SCALE` rather than an absolute amount so that it
/// means the same thing in a seedling and in a mature tree.
fn water_status(world: &World, x: i32, y: i32) -> f32 {
    world.water_at(x, y).1
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
/// `1 ± variance`, from the unit draw this individual took at germination
/// (`seed_genotype`). `slot` indexes both the draw and the variance array,
/// so the two cannot drift apart.
pub fn genotype(world: &World, organism_id: u16, slot: usize, variance: f32) -> f32 {
    if variance <= 0.0 {
        return 1.0;
    }
    let draw = world.organism(organism_id).map_or(0.0, |s| s.genotype_draws[slot]);
    (1.0 + draw * variance).max(0.0)
}

/// Draw this individual's genotype, once, from **where it germinated**.
///
/// Keyed on the germination coordinate and the world seed rather than on
/// `organism_id`, and the difference is not cosmetic. Ids are handed out in
/// planting order, so an id-keyed genotype makes a plant's character a
/// property of the world's whole event history: plant one extra sapling
/// anywhere earlier and every plant after it becomes a different
/// individual. Worldgen edits, player planting and slot reuse (live now,
/// via `World::free_organism`) all shift it. Position keying survives all three,
/// costs six floats, and is stable across a save/load that restores the
/// grid — which is worth having *before* a serialiser exists, since the
/// alternative constrains one that does not.
///
/// The **germination** coordinate specifically, not the planting one: a
/// seed is a `Powder`, so it falls and rolls, and where it comes to rest is
/// where the plant actually lives.
///
/// Two individuals germinating at the same cell in a long-lived world draw
/// the same genotype. Mixing the frame in would fix that and would break
/// save/load stability, so it is deliberately not done — a repeat in the
/// same spot, seasons apart, is not a visible defect.
pub fn seed_genotype(world: &mut World, organism_id: u16, x: i32, y: i32) {
    // **An inherited genome is not redrawn.** This function keys on where
    // a seed came to rest, which is right for one a scene or the player
    // planted and is exactly wrong for one another plant set: redrawing
    // would erase the parent's genome at the moment of germination and
    // leave a population that breeds but does not inherit. See
    // `set_seed`.
    if world.organism(organism_id).is_some_and(|s| s.inherited) {
        return;
    }
    let world_seed = world.seed;
    let mut draws = [0.0f32; organism::GENOTYPE_TRAITS];
    for (slot, draw) in draws.iter_mut().enumerate() {
        // One stream per (world, position, trait), so traits vary
        // independently and neither the plant's id nor its planting order
        // appears anywhere in the key.
        let mut rng = rng::stream(world_seed, x as u64, y as u64, slot as u64);
        *draw = rng.below(10_000) as f32 / 10_000.0 * 2.0 - 1.0;
    }
    // **Colour is drawn on the same key, and it is the cheapest variety in
    // the subsystem.** Three species shared one four-brown palette and one
    // four-green palette, so a stand of sixteen individuals was sixteen
    // draws of the same two colours — which is a large part of why the
    // architectural levers of the previous phase (sympody, tropism,
    // acrotony) all fired, all counted, and changed nothing anyone could
    // see. A lever that relabels a cell cannot move a silhouette that
    // texture and colour set.
    //
    // Streams 64/65 rather than genotype slots: these draws select among
    // discrete alleles, they do not multiply a species mean, so they never
    // belonged on the draws array. Stream 64 picks the foliage band, which
    // *is* the leaf-economy allele now (band = allele, the consumer that
    // existed when the locus was cosmetic); stream 65 used to pick the
    // bark band directly and now draws the wood-density allele, with bark
    // deriving from it -- the same stream still decides what the bark
    // looks like, it just says something true. Either way a first
    // generation is a mixed stand on both strategy axes from frame one,
    // exactly as it was on both colour axes.
    // `SpeciesId` is `Copy`, so reading it out ends the organism borrow
    // before the registry one begins — `world` is `&mut` here.
    let species_id = world.organism(organism_id).map(|s| s.species);
    let (foliage, bark) = match species_id {
        Some(id) => {
            let sp = world.species.get(id);
            (sp.foliage_bands, sp.bark_bands)
        }
        None => Default::default(),
    };
    let pick = |bands: organism::PaletteBands, stream: u64| -> u8 {
        if bands.count == 0 {
            return 0;
        }
        let mut rng = rng::stream(world_seed, x as u64, y as u64, stream);
        bands.first + rng.below(bands.count as u32) as u8
    };
    let foliage_band = pick(foliage, 64);
    let density_allele = {
        let mut rng = rng::stream(world_seed, x as u64, y as u64, 65);
        rng.below(organism::LOCUS_ALLELES[organism::LOCUS_WOOD_DENSITY] as u32) as u8
    };
    let bark_band = organism::bark_band_for_density(bark, density_allele);
    // **Discrete alleles start at what the species file declares**, so an
    // authored species is the point a population diverges *from* rather
    // than an identity it is stuck with. The scaled loci start mid-range
    // (index 1 of three), so a freshly planted stand is exactly the species
    // as written and every morph is one mutation away in either direction.
    let mut alleles = [0u8; organism::DISCRETE_LOCI];
    // (Density and economy are *not* mid-range: both are founded from the
    // positional draws above, which is what keeps a first generation a
    // mixed stand on both strategy axes and both colour bands from frame
    // one -- `Reports/plant-genome-design.md` §5.)
    alleles[organism::LOCUS_BRANCH_ANGLE] = 1;
    alleles[organism::LOCUS_INTERNODE] = 1;
    alleles[organism::LOCUS_WOOD_DENSITY] = density_allele;
    if let Some(id) = species_id {
        let sp = world.species.get(id);
        // Clamped to the *locus'* range, not the palette's. The band and
        // the allele are the same number today because every species
        // declares exactly two foliage bands, but the locus has two
        // alleles by definition (`LOCUS_ALLELES`) and a wider palette
        // would otherwise found individuals carrying an allele mutation
        // can never produce -- which then snaps to a different band the
        // first time the locus jumps. A no-op at every shipped species.
        alleles[organism::LOCUS_LEAF_ECONOMY] = foliage_band
            .saturating_sub(sp.foliage_bands.first)
            .min(organism::LOCUS_ALLELES[organism::LOCUS_LEAF_ECONOMY].saturating_sub(1));
        if let Some(Behavior::Grow { sympodial, tropism, .. }) =
            sp.behaviors(CellType::GrowingTip).iter().find(|b| matches!(b, Behavior::Grow { .. }))
        {
            alleles[organism::LOCUS_SYMPODIAL] = u8::from(sympodial.at(1));
            alleles[organism::LOCUS_TROPISM] = u8::from(tropism.at(1) == organism::Tropism::Plagiotropic);
        }
    }
    if let Some(state) = world.organism_mut(organism_id) {
        state.genotype_draws = draws;
        state.foliage_band = foliage_band;
        state.bark_band = bark_band;
        state.alleles = alleles;
    }
}

/// Which palette band a new cell of this organism belongs in.
///
/// `Foliage` and `Bark` are the two the species declares; a species that
/// declares neither (moss, and anything predating bands) gets `count: 0`
/// and falls through to a uniform draw over the whole palette, which is
/// exactly the pre-band behaviour.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Band {
    Foliage,
    Bark,
}

/// The `shade` byte for a cell this organism is about to create.
///
/// `Cell::shade` is a full byte wrapped modulo the palette length at render
/// time, so a banded palette needs no engine change at all: band `b`'s four
/// tonal steps are simply entries `4b..4b+4`. Per-individual colour
/// therefore costs **no per-cell state and no render work** — the byte the
/// cell already carried for grain now carries identity as well.
fn banded_shade(world: &World, organism_id: u16, material: material::MaterialId, band: Band, rng: &mut Rng) -> u8 {
    let palette_len = world.materials.get(material).palette.len().max(1) as u32;
    let declared = world.organism(organism_id).map(|s| {
        let sp = world.species.get(s.species);
        match band {
            Band::Foliage => (sp.foliage_bands.count, s.foliage_band),
            Band::Bark => (sp.bark_bands.count, s.bark_band),
        }
    });
    match declared {
        Some((count, index)) if count > 0 => index * organism::PALETTE_BAND + rng.below(organism::PALETTE_BAND as u32) as u8,
        // Undeclared, or an organism that no longer exists: the old
        // uniform-over-the-whole-palette draw.
        _ => rng.below(palette_len) as u8,
    }
}

/// **How far one generation's genome drifts from its parent's**, per trait,
/// on the `-1..=1` unit-draw scale the genotype uses.
///
/// This is the only source of new variation in a breeding population, and it
/// sets the whole tempo: too small and a stand is a clone army that cannot
/// respond to selection, too large and offspring are uncorrelated with their
/// parents, which is not heredity at all — it is the position-keyed redraw
/// this mechanism exists to replace, wearing a family tree.
///
/// 0.08 is one twelfth of the full trait range per generation. Deliberately
/// small: `genotype_variance` widths are already the *phenotypic* spread a
/// species shows, so a draw only has to move a little to land a visibly
/// different individual, and clusters need drift slow enough that selection
/// can hold a morph together against it.
///
/// **Untuned against an outcome**, and it should be swept once a breeding
/// population runs long enough to measure allele spread per generation.
const MUTATION_SIGMA: f32 = 0.08;

/// Set one seed from `(x, y)`, carrying this parent's genome forward.
///
/// **This is the heredity channel, and until now there was none.** Every
/// genotype in the engine was drawn from `(world seed, germination
/// coordinate)` — the *place*, not the parent — so offspring of a
/// well-adapted plant were no more like it than any stranger, and selection
/// had nothing to accumulate on. A stand could be culled by drought for a
/// thousand generations and never get more drought-tolerant.
///
/// The seed is placed as a `Powder`, so it falls and rolls from wherever it
/// was set exactly as a planted seed does, and germinates by the same
/// light-and-moisture gate. Nothing downstream needs to know it had a
/// parent except `seed_genotype`, which must not redraw over the genome
/// this copies in.
fn set_seed(world: &mut World, x: i32, y: i32, parent_id: u16, seed_cost: f32, rng: &mut Rng) -> bool {
    let Some((species, draws, generation, parent_alleles)) = world
        .organism(parent_id)
        .map(|s| (s.species, s.genotype_draws, s.generation, s.alleles))
    else {
        return false;
    };
    let Some(seed_material) = world.materials.id_of("seed") else {
        return false;
    };
    // Somewhere open beside the parent cell. Deliberately *any* free
    // neighbour rather than a chosen direction: a seed is a falling powder,
    // so where it ends up is the world's business, not the plant's.
    let spots: Vec<(i32, i32)> = NEIGHBOURS_8.iter().map(|&(dx, dy)| (x + dx, y + dy)).filter(|&(nx, ny)| world.is_empty(nx, ny)).collect();
    if spots.is_empty() {
        return false;
    }
    let (sx, sy) = spots[rng.below(spots.len() as u32) as usize];
    let (foliage_first, foliage_count, bark_bands) = {
        let sp = world.species.get(species);
        (sp.foliage_bands.first, sp.foliage_bands.count, sp.bark_bands)
    };

    let child = world.push_organism(species);
    let shades = world.materials.get(seed_material).palette.len().max(1) as u32;
    let shade = rng.below(shades) as u8;
    if let Some(state) = world.organism_mut(child) {
        // Each trait drifts independently, so a genome is not a single
        // dial: two offspring of one parent can differ on branching and
        // agree on height, which is what lets a population explore corners
        // of the trait space rather than sliding along one diagonal.
        for (dst, src) in state.genotype_draws.iter_mut().zip(draws.iter()) {
            let jitter = (rng.below(2_000) as f32 / 1_000.0 - 1.0) * MUTATION_SIGMA;
            *dst = (*src + jitter).clamp(-1.0, 1.0);
        }
        // **The discrete genes: inherited whole, mutated by jumping.** A
        // locus that drifted would be a continuous axis wearing an integer
        // and the population would smear back into one cloud; jumping is
        // what lets a morph hold together between rare excursions.
        state.alleles = parent_alleles;
        for (locus, allele) in state.alleles.iter_mut().enumerate() {
            if rng.chance(organism::DISCRETE_MUTATION_CHANCE) {
                let n = organism::LOCUS_ALLELES[locus].max(1);
                *allele = rng.below(n as u32) as u8;
            }
        }
        // Both colours derive from the (possibly just-mutated) alleles.
        // Foliage has worked this way since the discrete-loci change;
        // bark used to be copied from the parent and frozen forever --
        // heritable but immutable, a channel evolution could not move.
        // Deriving it from the density allele is what lets bark tone
        // change when the wood underneath it does.
        state.foliage_band = foliage_first + state.alleles[organism::LOCUS_LEAF_ECONOMY].min(foliage_count.saturating_sub(1));
        state.bark_band = organism::bark_band_for_density(bark_bands, state.alleles[organism::LOCUS_WOOD_DENSITY]);
        // The provisioning: what the parent paid rides with the child and
        // becomes its first stake at germination -- see
        // `OrganismState::endowment` for why this is species plumbing
        // today and a locus only after the response curve is measured.
        state.endowment = seed_cost;
        state.inherited = true;
        state.generation = generation.saturating_add(1);
    }
    world.set(sx, sy, Cell::new(seed_material, shade).with_organism_id(child).with_aux(organism::pack_cell_type(CellType::Seed)));
    world.schedule_active_site(reschedule_organism(sx, sy, child, 0, 0, world.frame + SEED_TICK_INTERVAL));
    if let Some(parent) = world.organism_mut(parent_id) {
        parent.seeds_set += 1;
    }
    true
}

/// Stamp a fresh cell's hydraulic path length as one step further from the
/// collar than the cell it grew from — see `OrganismCell::path_len`.
///
/// Must be called *after* the `World::set` that creates the cell, for the
/// same reason `deposit_canopy` must: `set` is what registers the
/// `OrganismCell` this writes into.
fn write_path_len(world: &mut World, x: i32, y: i32, parent: u16) {
    if let Some(slot) = world.organism_cell_mut(x, y) {
        slot.path_len = parent.saturating_add(1);
    }
}

/// This cell's own path length, or 0 if it has no sidecar yet.
fn path_len_at(world: &World, x: i32, y: i32) -> u16 {
    world.organism_cell(x, y).map_or(0, |c| c.path_len)
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

// `SEED_FALL_SEARCH` lived here and is gone with the search cone it
// bounded -- `relocated_seed` reads the organism's own cell list now, which
// has no reach limit to tune and no gaps to fall through.

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

/// Light reaching a plant-owned or plant-adjacent cell, **at its own
/// position**.
///
/// **This used to sample one field block up, and that offset is now a
/// bug.** It existed because `rebuild_blocked` marked a whole block opaque
/// the moment any `Solid` or `Plant` cell sat inside it, and `apply_sky`
/// then skipped opaque blocks entirely — so a plant cell reading its own
/// position landed inside a block its own material had just made opaque
/// and read a permanent `0.0`, however bright the sky was one cell away.
/// That was a real deadlock (a seed that could never see enough light to
/// germinate, in open sky, forever), and reading one block up dodged it.
///
/// `apply_sky` writes the light *arriving at* a block now, occupied or
/// not, precisely so an occluder can read what it intercepts — a leaf is
/// the thing doing the intercepting, and its own reading is the arriving
/// light. So the workaround's premise is gone, and what the offset does
/// instead is make every leaf read the light one block **above** itself,
/// i.e. before its own block's attenuation. Every plant in the world
/// over-reads its income by one block of self-shading, which is the
/// strongest term in the feedback that is supposed to bound it.
///
/// `CLAUDE.md`'s "fixing a bug often exposes a constant that was
/// compensating for it", one level up: it exposed a whole mechanism.
///
/// **Noon-equivalent, not raw** — `field::noon_equivalent_light`, which
/// rescales the reading by the sky's current output so the same occlusion
/// reads the same number at any hour. Every economic decision routes
/// through this function (income, the bud-break gate, bud siting, `q`,
/// germination, moss's shade preference), and every one of them was
/// sampling a 20:1 day/night oscillator: the live frontier measured 71
/// tips at noon against 28 at night on the same stand, and shade
/// abscission was unusable at any fixed threshold because every leaf in
/// the world reads near zero at midnight. What a plant should respond to
/// is *how shaded it is*, not *what time it is* — the day/night cycle
/// stays real in the field and on screen, and stops aliasing into the
/// economy. `phototropism_dir` deliberately stays raw: it compares two
/// readings, so the phase factor cancels.
pub fn ambient_light_above(world: &World, x: i32, y: i32) -> f32 {
    super::field::noon_equivalent_light(world.field_at(x, y).light, world.frame)
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
/// A seed is a `Powder`, so it falls, rolls and settles somewhere other than
/// where it was planted — but its `ActiveSite` still names the planting
/// position. **This asks the organism's own cell list**, which is the fix
/// the previous version's doc said was "the right fix later": that list
/// exists now (Decision 2), is maintained at the single `World::set` seam
/// under both drivers, and a seed organism holds exactly one cell.
///
/// The search cone it replaces had holes. It probed `x + dx * dy.min(2)`
/// for `dx ∈ {0, -1, 1}`, so it covered columns `x, x±1` at depth 1 and
/// `x, x±2` — never `x±1` — at every depth beyond that. A seed that fell
/// two or more rows while drifting exactly one column (four frames of
/// `Powder` motion; a topple off a slope does it) was missed, its site
/// died, and the organism became a permanently inert seed cell. Rare on
/// flat harness ground, and waiting for the first sloped scene.
///
/// Deliberately seed-only. Every other organism cell is immovable, so
/// nothing else can go missing this way and nothing else pays for the
/// lookup.
fn relocated_seed(world: &World, organism_id: u16) -> Option<(i32, i32)> {
    let state = world.organism(organism_id)?;
    // Row-major minimum rather than "whatever the map yields first":
    // `cells` is a `HashMap`, and `PLAN.md` requires same-build
    // determinism. One entry in the seed case, so the ordering costs
    // nothing and stops a future multi-cell caller from being a flake.
    state
        .cells
        .keys()
        .copied()
        .filter(|&(sx, sy)| organism::cell_type(world.get(sx, sy).aux()) == Some(CellType::Seed))
        .min_by_key(|&(sx, sy)| (sy, sx))
}

fn organism_tick(world: &mut World, x: i32, y: i32, organism_id: u16, stale_ticks: u8, plastochron: u8) -> Vec<ActiveSite> {
    // A seed that fell out from under its own site: pick the search back up
    // wherever it landed instead of dropping the organism on the floor.
    if world.get(x, y).organism_id() != organism_id {
        if let Some((sx, sy)) = relocated_seed(world, organism_id) {
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
            // Same shape as `SecondaryThicken` below: seed set is driven
            // from the whole-organism upkeep walk, not from an active site,
            // so that a mature cell does not have to stay on the schedule
            // just to check whether it can breed.
            Behavior::Reproduce { .. } => {}
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
                    let own_path = path_len_at(world, x, y);
                    world.set(tx, ty, new_cell);
                    write_order(world, tx, ty, order);
                    write_path_len(world, tx, ty, own_path);
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
                branch_priming,
                penetration_force,
                turgor_source,
                turgor_yield,
                turgor_per_cell,
                turgor_taper,
                heading_inertia,
                leaf_cluster,
                juvenile_size,
                juvenile_plastochron,
                juvenile_branch,
                genotype_variance,
                // **Read at planting, not here.** These two seed
                // `LOCUS_SYMPODIAL` and `LOCUS_TROPISM` in `seed_genotype`,
                // and growth then reads the *allele* -- so the species file
                // sets where a population starts and mutation decides where
                // it goes. An unmutated stand behaves exactly as authored.
                sympodial: _,
                tropism: _,
                branch_angle,
                internode,
            } => {
                // Per-order parameters resolved once, against *this cell's*
                // own order. A tip reads only its own tier -- no traversal,
                // no whole-plant query -- which is what keeps architecture
                // local; see `organism::ByOrder`.
                // Per-order first, then this individual's own genotype on
                // top. Each trait reads its own slot -- two traits sharing
                // one would move together, which is the "one tree scaled
                // up" failure `genotype_variance` exists to avoid.
                // **The discrete genes, applied.** A locus that does not
                // reach a growth decision is an invisible gene, and this
                // project has shipped that mistake before -- sympody,
                // tropism and acrotony all fired, all counted, and moved
                // nothing anyone could see. These four scale or override
                // the species value at the point it is used.
                let alleles = world.organism(organism_id).map(|s| s.alleles).unwrap_or([0; organism::DISCRETE_LOCI]);
                let angle_scale = organism::BRANCH_ANGLE_ALLELES
                    [(alleles[organism::LOCUS_BRANCH_ANGLE] as usize).min(organism::BRANCH_ANGLE_ALLELES.len() - 1)];
                let internode_scale = organism::INTERNODE_ALLELES
                    [(alleles[organism::LOCUS_INTERNODE] as usize).min(organism::INTERNODE_ALLELES.len() - 1)];
                // Dense wood pays more per cell -- the price half of
                // `WOOD_DENSITY_ALLELES` (the strength half is in
                // `structural::organism_structural_tick`). On the cost
                // rather than the income side so it binds exactly where
                // tree.ron's tuning history says the margin lives: a
                // fresh cell's first Grow check. Roots pay it too --
                // rootwood is wood.
                let cost = cost * organism::wood_density(&alleles);
                // Slot 8: penetration, a root trait by consumption (a
                // shoot's force is 0.0 and stays 0.0 under any
                // multiplier). The variance is this behaviour's own
                // vector, so the width lives in the species file's
                // RootTip entry, authored to keep the low tail above
                // soil's 0.8 resistance -- no draw is locked out of
                // ordinary ground.
                let penetration_force = penetration_force * genotype(world, organism_id, 8, genotype_variance[8]);
                // The trunk is never sympodial and never plagiotropic
                // whatever the allele says: order 0 is the axis that has to
                // stand the plant up, and a sympodial trunk is a different
                // mechanism (Leeuwenberg) that the species file, not a
                // point mutation, should be choosing.
                let sympodial_here = order > 0 && alleles[organism::LOCUS_SYMPODIAL] != 0;
                // **TRIED AND WITHDRAWN: letting order 0 read the species
                // file's own `tropism`, so a whole-habit-prostrate species
                // (a mat, a runner, turf) could exist at all.** It had no
                // order above 0 to say it with, so that form was
                // unreachable by any data -- the argument looked sound and
                // the change was behaviour-free for every shipped species
                // (the `tree` grove sheet was byte-identical across it,
                // md5 35f6147408e8ff75ce865b38697961fc).
                //
                // It was withdrawn because the form it unlocked turned out
                // not to be a form. `assets/species/prostrate.ron` was
                // built on it and rendered against `creeper.ron`, which
                // needs no code at all, and the owner's blind verdict on
                // the pair was "Not that different" (2/5). A code change
                // whose only justification is a form class that pure data
                // already reaches is not paying for itself.
                //
                // Do not re-derive it from the same argument. What would
                // change the answer is evidence that order-0 plagiotropy
                // produces something `heading_inertia` and a low turgor
                // budget cannot -- and that is a sheet, not a syllogism.
                // See `Reports/plant-evolution-design.md` §4a's register.
                let plagiotropic_here = order > 0 && alleles[organism::LOCUS_TROPISM] != 0;

                // This cell's own distance from the collar, read once:
                // the turgor gate below reads it, and every child
                // created further down is one step further out.
                let own_path = path_len_at(world, x, y);
                // **The slot map, applied** -- `GENOTYPE_TRAITS`' own doc
                // is the contract. Branch chance is slot 0 for a shoot and
                // slot 1 for a root, so the two halves of one plant vary
                // independently at last: while a root's every multiplier
                // was the shoot's draw (and, with the root vector zeroed,
                // exactly 1.0), no amount of selection could produce a
                // deep-rooted morph. The variance read here is this
                // behaviour's own vector, so the root widths live in the
                // species file's RootTip entry.
                let is_root = cell_type == CellType::RootTip;
                let bc_slot = if is_root { 1 } else { 0 };
                let branch_chance = branch_chance.at(order) * genotype(world, organism_id, bc_slot, genotype_variance[bc_slot]);
                // The shoot's upward/light jitters are gone, not moved:
                // both measured flat across 1,024 genomes (upward at
                // +/-40%, light at +/-50%), and light steers by noise while
                // the per-column sky cast leaves no lateral gradient --
                // fix the field before ever re-adding a slot for it.
                let light_weight = light_weight.at(order);
                let upward_weight = if is_root {
                    // Slot 5. For a root this weights the moisture-or-down
                    // reference below, so the one slot genuinely is "how
                    // hard this root drives down and toward water versus
                    // wandering" -- gravitropic and hydrotropic gain in
                    // the single number the reference already blends.
                    upward_weight.at(order) * genotype(world, organism_id, 5, genotype_variance[5])
                } else {
                    upward_weight.at(order)
                };
                let mut plastochron_interval = ((plastochron_interval.at(order) as f32 * genotype(world, organism_id, 2, genotype_variance[2])).round()
                    as u8)
                    .max(u8::from(plastochron_interval.at(order) > 0));
                // **The juvenile stage.** See `juvenile_size` for why branch
                // order cannot carry this: order is position, age is not.
                // Applied after the genotype so an individual keeps its own
                // draw through both stages rather than being flattened onto
                // the species mean while young.
                let juvenile = juvenile_size > 0 && world.organism(organism_id).is_some_and(|s| s.shoot_cells < juvenile_size);
                let mut branch_chance = branch_chance;
                if juvenile {
                    plastochron_interval = ((plastochron_interval as f32 * juvenile_plastochron).round() as u8).max(u8::from(plastochron_interval > 0));
                    branch_chance *= juvenile_branch;
                }
                // **Height is the trait the clone look shows up in first**,
                // because the turgor bound is geometric and every tree
                // reaches it exactly. Jittering the per-cell cost spreads
                // the derived ceiling instead of the outcome.
                let turgor_per_cell = turgor_per_cell * genotype(world, organism_id, 3, genotype_variance[3]);
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
                    {
                        // **Hydraulic path length, not height** — see
                        // `OrganismCell::path_len` for the measurement that
                        // forced this. The vertical form bounded height and
                        // bounded width not at all, so a tree that could not
                        // grow up grew sideways forever; path length bounds
                        // both with the same term, and is what the biology
                        // says anyway.
                        //
                        // No `collar_y` read any more: the distance is
                        // stamped at creation and never moves, which is
                        // strictly better than recomputing against a collar
                        // that can.
                        let path = own_path as f32;
                        let margin = turgor_source - turgor_per_cell * path - turgor_yield;
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
                // **Where this shoot is actually going.** Falls back to the
                // one-cell local read only for a cell with no history --
                // see `organism::OrganismCell::heading` for why that read
                // alone made every stem wander.
                let stored_heading = world.organism_cell(x, y).map_or((0.0, 0.0), |c| c.heading);
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
                let heading = if stored_heading == (0.0, 0.0) { away_from_supply } else { stored_heading };

                let gravity_or_water = if cell_type == CellType::RootTip {
                    match organism::moisture_pull(world, x as f32, y as f32) {
                        Some((dir, strength)) if strength >= MIZ_THRESHOLD => dir,
                        _ => (0.0, 1.0), // down
                    }
                } else {
                    // **The tier's own reference, not a hardcoded up.**
                    // Every axis of every species was orthotropic by
                    // construction until `Tropism` landed — the reference
                    // was the literal `(0.0, -1.0)` this arm still uses
                    // for orthotropic tiers. A plagiotropic tier instead
                    // holds the *horizontal sense of its own heading*: the
                    // direction it left its parent in, which momentum has
                    // carried since the lateral departed. That is what
                    // makes a fir's branch a tier and much of the
                    // temperate broadleaf flora (Troll's model) buildable
                    // at all. Exactly-vertical headings take +x,
                    // deterministically — rare (laterals leave sideways),
                    // and a coin would make replay diverge.
                    // The allele decides, not the species file -- see
                    // `plagiotropic_here`. The species value seeded the
                    // allele at planting, so an unmutated stand behaves
                    // exactly as authored.
                    match if plagiotropic_here { organism::Tropism::Plagiotropic } else { organism::Tropism::Orthotropic } {
                        organism::Tropism::Orthotropic => (0.0, -1.0),
                        organism::Tropism::Plagiotropic => {
                            // Outward with a droop, not pure horizontal —
                            // (±0.91, +0.41) normalized. Real plagiotropic
                            // branches angle down-and-out before levelling
                            // (a spruce tier's silhouette), and the first
                            // pure-horizontal version read as nothing of
                            // the kind: a lateral leaves diagonally upward,
                            // and under high heading inertia a level
                            // reference only bent that launch into a long
                            // upward-sweeping arc — the whole stand read as
                            // leaning candelabras (p2-conifer.png, first
                            // cut). The droop pulls the arc back through
                            // horizontal instead of letting it keep its
                            // climb.
                            // The side comes from the axis's own travel,
                            // with a real dead zone: sign-of-heading alone
                            // sent every near-vertical axis RIGHT (the
                            // sign of an epsilon), and bud-flushed laterals
                            // inherited the trunk's near-vertical heading,
                            // so whole stands swept rightward in unison
                            // (first two conifer sheets). Inside the dead
                            // zone the coin is the per-(organism, cell,
                            // frame) stream, so replay holds; one sideways
                            // step later the sign locks and the coin never
                            // fires again.
                            if heading.0.abs() > 0.05 {
                                if heading.0 < 0.0 { (-0.912, 0.410) } else { (0.912, 0.410) }
                            } else if rng.flip() {
                                (0.912, 0.410)
                            } else {
                                (-0.912, 0.410)
                            }
                        }
                    }
                };

                // §2b: score every open 8-neighbour, weighted-random
                // *sample* from the positive-scoring set -- never a
                // deterministic best-direction pick, which is what would
                // actually curve-fit a silhouette.
                // Hoisted out of the candidate loop: this is a property of
                // the cell's own age, not of the direction being scored.
                let internode_here = ((internode.at(order) as f32) * internode_scale).round() as u32;
                let rigid_step = internode_here > 0 && (plastochron as u32) < internode_here;
                let mut candidates: Vec<(i32, i32, f32)> = Vec::new();
                for (dx, dy) in NEIGHBOURS_8 {
                    let (nx, ny) = (x + dx, y + dy);
                    if !growable(world, nx, ny, penetration_force) {
                        continue;
                    }
                    // Affordable-this-tick ground only. A poor root
                    // prefers soft ground -- roots really do follow the
                    // path of least resistance -- and one that can only
                    // reach rock it cannot pay for banks staleness on the
                    // empty set below, exactly like boxed-in. Open air is
                    // mult 1.0 and always passes: the base-cost gate above
                    // already guaranteed `resource >= cost`.
                    if resource < cost * penetration_cost_mult(world, nx, ny) {
                        continue;
                    }
                    let dir = normalize((dx as f32, dy as f32));
                    let density = candidate_crowding(world, nx, ny);
                    // **Crowding divides; it does not subtract.** The
                    // subtractive form had a cliff that was arithmetic, not
                    // ecology: crowding was taken off the score and the
                    // score then filtered on `> 0.0`, so past a threshold
                    // the term stopped biasing the choice and started
                    // *emptying the candidate set* — the tip banked a stale
                    // tick, and four of those is permanent retirement. The
                    // positive terms sum to ~1.15 at the outer orders, so
                    // `crowding_weight: 20` needed only `density > 0.058`
                    // to zero every direction at once; that is the measured
                    // collapse in tree.ron's old sweep (median tree 2,620
                    // cells at 12.0 against 26 at 20.0), and why the usable
                    // band sat one step under a cliff.
                    //
                    // Dividing makes crowding a preference at any weight: a
                    // fully crowded tip takes its least-bad direction
                    // instead of dying, the knob becomes monotone with no
                    // collapse to sit under, and what *stops* growth is
                    // what should stop it — the light economy (self-shading
                    // has real teeth now) and the turgor bound, not a
                    // side-effect of score arithmetic. A candidate with no
                    // positively-preferred direction still fails the filter
                    // and that is correct: boxed in is a real dead end.
                    // **The straightness budget.** Inside its first
                    // `internode` steps a fresh lateral scores on
                    // continuation *alone* -- the light, wind and tropism
                    // terms get no vote -- so it leaves along its departure
                    // direction and stays there for a set run before the
                    // environment starts steering it.
                    //
                    // This is the missing shape primitive, and `branch_angle`
                    // is inert without it: the engine models a branch as a
                    // biased random walk, so a lateral that departed at 90
                    // degrees was re-scored against `upward_weight` and the
                    // tier reference on its very next step and bent straight
                    // back alongside the trunk. That is the parallel-ropes
                    // look, and it is why these two landed as one change
                    // rather than as two independently-judgeable ones
                    // (`Reports/plant-appearance-design.md` §2.3-2.4).
                    //
                    // Costs no per-cell state: a lateral is rescheduled with
                    // `plastochron: 0`, so the lineage step the active site
                    // already carries *is* its age in cells.
                    let preference = if rigid_step {
                        dot(dir, heading) * continuation_weight
                    } else {
                        dot(dir, heading) * continuation_weight
                            + dot(dir, photo) * light_weight
                            + dot(dir, wind) * wind_weight
                            + dot(dir, gravity_or_water) * upward_weight
                    };
                    let score = preference / (1.0 + density * crowding_weight);
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
                // **The branching oscillator.** Slot 1 multiplies the
                // priming *rate*, so the interval divides by the draw: a
                // high draw primes more densely, which is the direction
                // "root branch chance" has always meant. Floored at 1 so a
                // strong draw cannot collapse the interval to zero and
                // prime every single cell.
                let priming_interval = branch_priming.at(order);
                let priming_interval = if priming_interval == 0 {
                    0
                } else {
                    let rate = genotype(world, organism_id, 1, genotype_variance[1]).max(0.05);
                    ((priming_interval as f32 / rate).round() as u8).max(1)
                };
                let prime_due = priming_interval > 0 && lineage_step.is_multiple_of(priming_interval);
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
                // Priced before `set` overwrites the target -- the
                // material being entered is what carries the resistance.
                let step_cost = cost * penetration_cost_mult(world, tx, ty);

                let shade = banded_shade(world, organism_id, cell.material, Band::Bark, &mut rng);
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
                write_path_len(world, tx, ty, own_path);
                // Momentum. A stem already lignified behind the apex cannot
                // turn sharply, so the child leaves with most of its
                // parent's heading and only a little of the step it just
                // took.
                let step = normalize(((tx - x) as f32, (ty - y) as f32));
                let blended = normalize((
                    heading.0 * heading_inertia + step.0 * (1.0 - heading_inertia),
                    heading.1 * heading_inertia + step.1 * (1.0 - heading_inertia),
                ));
                if let Some(slot) = world.organism_cell_mut(tx, ty) {
                    slot.heading = blended;
                }
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
                resource -= step_cost;
                world.set(x, y, cell.with_aux(organism::pack_cell_type(self_type_after_grow)));
                write_carbon(world, x, y, resource);
                // **Priming costs nothing and buys nothing yet.** The mark
                // is the whole point: the tip records a site as it passes
                // and moves on, and the site buys its own lateral later out
                // of whatever carbon reaches it. Splitting the decision from
                // the bill is what makes the purchase affordable at all --
                // see `OrganismCell::primed`.
                if prime_due {
                    write_primed(world, x, y, true);
                }
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
                // **`priming_interval == 0` keeps this roll; non-zero
                // replaces it.** Both would be two mechanisms competing for
                // one pool, and this is the one that could not open: it
                // demands a *second* step's carbon in the same tick the
                // first was just paid for. A shoot tip photosynthesises and
                // can meet that; a root tip met it twice in twelve thousand
                // frames. See `Behavior::Grow::branch_priming`.
                if priming_interval == 0
                    && resource >= cost
                    && rng.chance(branch_chance)
                    && world.organism_active_tip_count(organism_id, cell_type) + 1 < max_active_tips as usize
                {
                    // **A lateral gets its own candidate set, and this is
                    // the third and deepest layer of the branch-angle
                    // problem.** It used to reuse `candidates`, which is
                    // the *primary* scoring's survivors -- and that set is
                    // filtered on `score > 0` against a preference carrying
                    // `upward_weight` (0.9 on a trunk). A near-horizontal
                    // step off a vertical axis scores about zero on every
                    // term and does not survive the filter, so a wide
                    // departure was not merely unlikely, it was **not in
                    // the set at all**.
                    //
                    // Two measured attempts landed before this was found,
                    // both of which "worked" and neither of which moved the
                    // number: weighting the primary score by angular
                    // closeness gave a mean achieved departure of 40
                    // degrees against a 70 degree target, and discarding
                    // the score entirely in favour of closeness gave 48.
                    // The set was the constraint, not the weighting --
                    // `CLAUDE.md`'s "two fixes failing the same way means
                    // the approach is wrong, not the tuning", and the
                    // achieved-angle counter is the only reason it was
                    // visible.
                    //
                    // So an angled lateral scores over every growable
                    // neighbour. Only for the angled path: a species with
                    // no `branch_angle` keeps the old set and the old
                    // uniform draw exactly.
                    let target = branch_angle.at(order) * angle_scale;
                    let alt: Vec<(i32, i32, f32)> = if target > 0.0 {
                        NEIGHBOURS_8
                            .iter()
                            .map(|&(dx, dy)| (x + dx, y + dy))
                            .filter(|&(nx, ny)| (nx, ny) != (tx, ty) && growable(world, nx, ny, penetration_force))
                            .map(|(nx, ny)| (nx, ny, 1.0))
                            .collect()
                    } else {
                        candidates.into_iter().filter(|&(nx, ny, _)| (nx, ny) != (tx, ty)).collect()
                    };
                    if !alt.is_empty() {
                        // **The departure angle.** This used to be
                        // `alt[rng.below(alt.len())]` -- a uniform draw over
                        // whatever open neighbours were left, so branching
                        // *rate* was per-order species data and branching
                        // *angle* was noise, in an engine whose whole
                        // silhouette is made of branches. See
                        // `Behavior::Grow::branch_angle`.
                        //
                        // Scored, then sampled -- never an argmax. A
                        // deterministic best-direction pick is exactly what
                        // would curve-fit a silhouette, which is the
                        // objection the primary candidate loop above raises
                        // about itself, and it applies just as much here.
                        //
                        // The 8-neighbourhood quantises the achievable
                        // angles to multiples of 45 degrees, so this is a
                        // target the candidates are ranked against and not a
                        // value that can be hit; the weight falls off with
                        // the angular error rather than selecting on it.
                        let (bx, by, _) = if target <= 0.0 {
                            alt[rng.below(alt.len() as u32) as usize]
                        } else {
                            let weighted: Vec<(i32, i32, f32)> = alt
                                .iter()
                                .map(|&(nx, ny, _)| {
                                    let dir = normalize(((nx - x) as f32, (ny - y) as f32));
                                    // Angle between this step and the axis
                                    // it is leaving, in degrees.
                                    let cos = dot(dir, heading).clamp(-1.0, 1.0);
                                    let degrees = cos.acos().to_degrees();
                                    // Falls off over 45 degrees, one
                                    // neighbourhood step, so the preferred
                                    // direction is clearly favoured and its
                                    // neighbours stay reachable.
                                    let closeness = 1.0 / (1.0 + ((degrees - target).abs() / 45.0));
                                    // **The primary candidate score is
                                    // deliberately discarded here, and
                                    // measurement is why.** Multiplying by
                                    // it gave a mean achieved departure of
                                    // 40 degrees against a 70 degree
                                    // target: that score carries
                                    // `upward_weight` (0.9 on a trunk), so
                                    // a near-horizontal candidate is
                                    // already heavily penalised by it and
                                    // the closeness term could not lift it
                                    // back. The lever fired -- the counter
                                    // proved that -- and did almost
                                    // nothing, which is this project's
                                    // signature failure.
                                    //
                                    // A branch's departure angle is a
                                    // *developmental* property, not an
                                    // environmental one: the primary child
                                    // is what continues the axis by
                                    // scoring light and gravity, and the
                                    // lateral is what the architecture
                                    // places. So the lateral scores on
                                    // angle, and only crowding still gets a
                                    // vote -- it must not be pushed into a
                                    // wall. Recomputed rather than
                                    // recovered from the score, which costs
                                    // up to seven reads on a branch event;
                                    // branching is rare (about 200 events
                                    // per plant per 30,000 frames) so this
                                    // is not on the hot path.
                                    let density = candidate_crowding(world, nx, ny);
                                    (nx, ny, closeness / (1.0 + density * crowding_weight))
                                })
                                .collect();
                            let total: f32 = weighted.iter().map(|&(_, _, s)| s).sum();
                            let mut pick = (rng.below(10_000) as f32 / 10_000.0) * total;
                            let mut chosen = weighted[0];
                            for &c in &weighted {
                                if pick < c.2 {
                                    chosen = c;
                                    break;
                                }
                                pick -= c.2;
                            }
                            chosen
                        };
                        // Hard ground is priced for a lateral exactly as
                        // for the primary step; an unaffordable target
                        // just means no branch this tick.
                        let branch_step_cost = cost * penetration_cost_mult(world, bx, by);
                        if growable(world, bx, by, penetration_force) && resource >= branch_step_cost {
                            let branch_shade = banded_shade(world, organism_id, cell.material, Band::Bark, &mut rng);
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
                            write_path_len(world, bx, by, own_path);
                            // A lateral is a *new* axis, so it leaves along
                            // the step it took rather than inheriting the
                            // parent's momentum -- otherwise every branch
                            // curls back parallel to the trunk, which is
                            // the parallel-ropes look this branch already
                            // fixed once through `upward_weight`.
                            let departure = normalize(((bx - x) as f32, (by - y) as f32));
                            if let Some(slot) = world.organism_cell_mut(bx, by) {
                                slot.heading = departure;
                            }
                            // The achieved angle, not a count of attempts --
                            // see `OrganismState::departure_angle_sum`.
                            if let Some(state) = world.organism_mut(organism_id) {
                                state.lateral_departures += 1;
                                state.departure_angle_sum += dot(departure, heading).clamp(-1.0, 1.0).acos().to_degrees();
                            }
                            deposit_canopy(world, bx, by, GROW_CANOPY_DEPOSIT);
                            // No structural check here either -- see the
                            // primary child's identical case above.
                            resource -= branch_step_cost;
                            world.set(x, y, cell.with_aux(organism::pack_cell_type(self_type_after_grow)));
                            write_carbon(world, x, y, resource);
                            next.push(reschedule_organism(bx, by, organism_id, 0, 0, world.frame + ORGANISM_TICK_INTERVAL));

                            // **A sympodial tier forks instead of
                            // decorating: the fork replaces the axis.** The
                            // apex already dies every step (it just retired
                            // to `self_type_after_grow` above), so
                            // monopodiality was only ever the labelling --
                            // the primary child inheriting order and
                            // heading. On a sympodial tier's fork, the
                            // primary is re-labelled a lateral too: next
                            // tier, its own fresh heading, no inherited
                            // momentum. Both children are now equals and
                            // the axis is a stack of modules, which is
                            // Leeuwenberg's model once `ByOrder` saturates.
                            // Counted, because a sympodial run whose
                            // counter reads zero is a monopodial tree that
                            // happened to fork.
                            if sympodial_here {
                                write_order(world, tx, ty, order.saturating_add(1));
                                if let Some(slot) = world.organism_cell_mut(tx, ty) {
                                    slot.heading = step;
                                }
                                if let Some(state) = world.organism_mut(organism_id) {
                                    state.sympodial_forks += 1;
                                }
                            }
                        }
                    }
                }

                // The tropism counter -- a plagiotropic tier that never
                // grew is invisible on a sheet, and "did it fire" needs a
                // number.
                if cell_type == CellType::GrowingTip && plagiotropic_here {
                    if let Some(state) = world.organism_mut(organism_id) {
                        state.plagiotropic_steps += 1;
                    }
                }
                // Same for the straightness budget: a species can declare
                // an `internode` that never binds, and the sheet cannot
                // tell that apart from one that does.
                if rigid_step {
                    if let Some(state) = world.organism_mut(organism_id) {
                        state.rigid_steps += 1;
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
                        // Same rule for the first leaf: behind the apex,
                        // not in front of it. Falls back to any open spot
                        // if the shoot is boxed in, since a node with
                        // nowhere behind it should still bear a leaf.
                        let behind: Vec<(i32, i32)> =
                            spots.iter().copied().filter(|&(sx, sy)| dot(normalize(((sx - x) as f32, (sy - y) as f32)), heading) <= 0.35).collect();
                        let pool = if behind.is_empty() { &spots } else { &behind };
                        let (lx, ly) = pool[rng.below(pool.len() as u32) as usize];
                        // The rest of the cluster grows off the first leaf,
                        // not off the node, so foliage forms a spray
                        // hanging away from the stem rather than a ring
                        // around it. See `Behavior::Grow::leaf_cluster`.
                        let mut cluster: Vec<(i32, i32)> = vec![(lx, ly)];
                        let mut frontier = vec![(lx, ly)];
                        while cluster.len() < leaf_cluster as usize {
                            let Some(&(fx, fy)) = frontier.first() else { break };
                            frontier.remove(0);
                            let mut open: Vec<(i32, i32)> = NEIGHBOURS_8
                                .iter()
                                .map(|&(dx, dy)| (fx + dx, fy + dy))
                                // **Never adjacent to the node itself.**
                                // Only the first leaf touches the stem; the
                                // rest hang away from it. Without this a
                                // five-cell cluster can ring the shoot apex
                                // and wall its own tip in -- which is
                                // exactly what happened to a reseeded
                                // seedling, whose growth stopped dead at
                                // its first cell.
                                .filter(|&(nx, ny)| {
                                    // Never adjacent to the node, and never
                                    // *ahead* of it. Only the first leaf
                                    // touches the stem, and none of the
                                    // cluster may sit in the direction the
                                    // shoot is travelling.
                                    //
                                    // Both clauses are the same bug: a
                                    // five-cell cluster placed freely rings
                                    // the apex and walls the tip in, and a
                                    // reseeded seedling stopped dead at its
                                    // first cell because of it. Leaves are
                                    // borne *behind* an apex on a real
                                    // shoot, never through it.
                                    world.is_empty(nx, ny)
                                        && !cluster.contains(&(nx, ny))
                                        && (nx - x).abs().max((ny - y).abs()) >= 2
                                        && dot(normalize(((nx - x) as f32, (ny - y) as f32)), heading) <= 0.35
                                })
                                .collect();
                            if open.is_empty() {
                                continue;
                            }
                            let pick = open.remove(rng.below(open.len() as u32) as usize);
                            cluster.push(pick);
                            frontier.push(pick);
                        }
                        // Real `leaf` material, not the shoot's own wood --
                        // foliage burns hot and fast, weighs almost
                        // nothing, and holds up nothing, none of which
                        // `wood` expresses. Falls back to the parent's
                        // material if the species' world has no `leaf`
                        // loaded, so a stripped-down asset set degrades to
                        // the old look rather than failing to grow.
                        let leaf_material =
                            world.materials.id_of(&world.species.get(species_id).leaf_material).unwrap_or(cell.material);
                        for &(cx, cy) in &cluster {
                            let shade = banded_shade(world, organism_id, leaf_material, Band::Foliage, &mut rng);
                            let leaf_cell =
                                Cell::new(leaf_material, shade).with_organism_id(organism_id).with_aux(organism::pack_cell_type(CellType::Leaf));
                            world.set(cx, cy, leaf_cell);
                            // A leaf belongs to the shoot that bore it, not
                            // to a new tier -- it never grows, so this only
                            // matters to anything reading order off foliage.
                            write_order(world, cx, cy, order);
                            write_path_len(world, cx, cy, own_path);
                            deposit_canopy(world, cx, cy, GROW_CANOPY_DEPOSIT);
                            next.push(reschedule_organism(cx, cy, organism_id, 0, 0, world.frame + ORGANISM_TICK_INTERVAL));
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
            // Abscission runs from the upkeep pass, which visits every
            // leaf; a live tip photosynthesises but is not foliage to shed.
            Behavior::Photosynthesize { rate, shade_death: _, transpiration, drought_death: _ } => {
                let light = ambient_light_above(world, x, y);
                // **Spend water, then earn carbon in proportion to what is
                // left.** The order is the physical one: stomata open, water
                // goes out, and carbon comes in only while they are open.
                let _ = transpiration; // demand is charged once per organism, in `organism_upkeep`
                // The leaf-economy allele scales every credit -- a live
                // tip is foliage and runs the same strategy its leaves do.
                // (Its demand side is scaled where demand is summed, in
                // `organism_upkeep`'s walk.)
                let rate = rate * leaf_econ_mults(world, organism_id).0;
                let status = water_status(world, x, y);
                resource = (resource + rate * light * status).min(organism::RESOURCE_SCALE);
                write_carbon(world, x, y, resource);
            }
            Behavior::Absorb { rate } => absorb_water(world, x, y, rate),
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
            Behavior::Germinate { light_threshold, soil_water_threshold, instant } => {
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
                // **The "no germinating on another plant" guard is gone,
                // and it was deleted on evidence rather than trimmed.**
                //
                // It existed because `resting` accepts any non-empty cell
                // and a branch is non-empty: seeds are a `Powder`, so in a
                // closed stand most never reach the ground -- they come to
                // rest on the first limb that stops them and sprout there.
                // Measured before the guard: **430 of 487 organisms rooted
                // above the soil, 410 of them more than 25 rows up**, and a
                // time lapse showed eight clean trees degrading into a pile
                // of trees growing out of trees.
                //
                // It was always a symptom fix, and the owner's framing was
                // the right one: *a canopy plant cannot have roots, so it
                // should starve rather than be forbidden.* With
                // transpirational demand charged and `Absorb` crediting
                // water, that is what happens -- a seed that sprouts in a
                // crown has no soil to reach, meets none of its demand,
                // earns nothing, and is shed leaf by leaf by
                // `drought_death`.
                //
                // Verified rather than assumed, which is the whole reason
                // the guard could be deleted: with this line removed, the
                // standard grove reads **0 organisms rooted above ground at
                // 30,000 frames and 0 at 60,000**, on a stand of 33,575
                // cells that is otherwise healthy. Keeping a superseded
                // mechanism "just in case" is a documented trap here -- its
                // tests keep passing while testing nothing.
                //
                // The reproduction is kept: `examples/plant_probe.rs`
                // prints the epiphyte count on every run, so a regression
                // announces itself.
                //
                // (the old `moisture_threshold` looked like the designed answer and
                // is not: field moisture at the soil *surface*, where a
                // good seed lands, is as near zero as it is up a tree, so
                // every non-zero setting blocked all germination equally --
                // 8 established at 0.05, 0.2 and 0.5 alike. It cannot
                // separate the cases.)
                let below = world.get(x, y + 1);
                let resting = below.material != material::EMPTY;
                let ready = resting
                    && (instant || {
                    let light = ambient_light_above(world, x, y);
                    // **Reads the soil it is resting on, not the field at
                    // its own cell** -- and the comment above records why
                    // the old read could not work: field moisture at the
                    // surface is as near zero as it is up a tree, so every
                    // non-zero setting blocked germination equally.
                    //
                    // `plant_available_fraction` is the *same quantity a
                    // root will drink* -- zero at or below the wilting
                    // point, one at field capacity -- so a seed now waits
                    // on exactly the condition that decides whether it can
                    // live once it germinates. That is the owner's model:
                    // a seed on dry ground sits until rain wets the soil,
                    // rather than germinating and starving.
                    //
                    // **Gated on `water_capacity > 0` first, and that guard
                    // is load-bearing.** `update::soil_moisture` reads
                    // `aux` with no material check, and on a `Liquid` `aux`
                    // is *fill*, where 0 means FULL on the same 1000 scale
                    // as saturated soil. A seed floats (density 0.6 against
                    // water's 1.0) and `resting` accepts any non-empty
                    // cell, so without this a seed bobbing on a full pond
                    // would read bone dry -- and one on half-drained water
                    // would read well watered.
                    let holds_water = world.materials.get(below.material).water_capacity > 0;
                    let soil_water = if holds_water { update::plant_available_fraction(below) } else { 0.0 };
                    light >= light_threshold && soil_water >= soil_water_threshold
                });
                if ready {
                    return germinate(world, x, y, organism_id, cell, &mut rng);
                }
                // **Remember that it waited**, so `germinate` can tell a
                // seed that sat out a dry spell from one that sprouted the
                // moment it landed. Without this the counter would say
                // "seeds germinated", which is true of the old behaviour
                // too and therefore evidence of nothing.
                if let Some(state) = world.organism_mut(organism_id) {
                    state.deferred_germination = true;
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
    // falling, and germination should follow it down promptly rather than
    // 45 frames after it lands. No longer a *correctness* requirement:
    // `relocated_seed` reads the cell list and finds a seed wherever it
    // went, however long it fell.
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
        // **The plant passes are for plants.** Creatures share this
        // storage -- they are organisms too, and `live_organism_ids`
        // rightly returns them -- but every pass below this point is
        // plant machinery, and until the creature line and the plant
        // lines merged there was no world in which both existed, so
        // nothing ever ran one over the other.
        //
        // Measured, on the merged tree: none of them *damages* a
        // creature. Creature species declare no behaviours so nothing
        // dispatches; `settle_water` at zero demand returns 1.0;
        // `break_root_tips`, `break_buds` and `allocate_to_frontier` all
        // bail on the `Grow` entries a creature species does not have;
        // `transport` builds `Plant`-kind topology, so a creature has no
        // faces. What they do is *work*: `anchor_support` finds no
        // `Solid` neighbour for an airborne ant, settles every cell at
        // `u16::MAX`, and schedules a structural check per creature cell
        // per tick that `structural::tick` then discards on arrival
        // (`is_body_material` is `Solid | Plant`).
        //
        // **Keyed on the species' `creature` field, deliberately, and not
        // on `collar_y`.** The obvious guard -- "a creature has no collar"
        // -- is false after one tick: `organism_upkeep` sorts every cell
        // that is neither a `RootTip` nor `reinforces_powder` into its
        // shoot branch, which is every `Head` and `Segment`, and writes
        // `collar_y`/`shoot_cells`/`shoot_top_y` onto the creature's own
        // state. A `collar_y` guard would switch itself off on the second
        // tick and look like it was working.
        let is_creature = world.organism(organism_id).is_some_and(|s| world.species.get(s.species).creature.is_some());
        if !is_creature {
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
            // After `accumulate_support` rather than before, and it does not
            // matter which order they run in -- they share no state. Placed
            // here so both structural passes sit together, and after
            // `allocate_to_frontier` so a cell created this tick is already in
            // the list being walked.
            anchor_support(world, organism_id);
            // Before upkeep, so a bud that flushes this tick is already a
            // `GrowingTip` when `thicken` runs and can be counted as frontier
            // rather than thickened over on the same tick it woke up.
            break_buds(world, organism_id);
            // After `break_buds`, so a tick's single shoot flush is decided
            // before the roots ask -- and after `organism_upkeep` has set
            // `water_status`, which is what this gates on.
            break_root_tips(world, organism_id);
            organism_upkeep(world, organism_id);
        }
        // **Outside the guard, and that is load-bearing.** Reclamation is
        // the one thing here that is genuinely for *every* organism: a
        // creature whose last cell is eaten or burned between its own
        // ticks needs its slot back exactly as a dead plant does. A bare
        // `continue` above would leak it and walk straight back into
        // `pixel-physics-issues.md` #8, which is the bug this allocator
        // exists to close.
        //
        // **Reclaim the slot of an organism that has no cells left.**
        //
        // `Cell::organism_id` spends twelve bits on the slot index, so
        // there are 4,095 of them and `push_organism` never got any back:
        // its own doc said "nothing populates that list yet in this pass".
        // Every seed that is set allocates a slot, and a seed that is
        // eaten, buried, burned or germinates-and-dies never returned one,
        // so the ceiling was on *cumulative* organisms rather than live
        // ones -- a leak with a hard stop at the end of it, and the reason
        // `pixel-physics-issues.md` #8 exists.
        //
        // **Empty cell list, not a liveness search, and that is measured
        // rather than assumed.** The scoped fix was a BFS finding no live
        // tip, leaf or root; counted on a real stand at 30,000 frames, 20
        // of 31 slots held *no cells at all* while exactly one held cells
        // with nothing alive in them. The BFS buys that single slot, costs
        // a traversal per organism per tick, and orphans the cells it
        // frees -- they would point at a dead slot while still standing.
        // An empty cell list cannot orphan anything by construction: no
        // cell refers to the organism, which is what makes it empty. The
        // standing-dead-trunk case is left, deliberately and visibly, to
        // whoever decides what a dead tree's wood should *be*.
        //
        // Safe against a newly planted organism whose cell is not set yet:
        // `push_organism` and the `World::set` that gives it its first
        // cell happen inside one call (`plant_tree_species`, `set_seed`),
        // and this pass only runs between frames.
        if world.organism(organism_id).is_some_and(|s| s.cells.is_empty()) {
            world.free_organism(organism_id);
        }
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
///
/// **Denominated per NODE, and that unit is the end of a treadmill.** Its
/// predecessor (`LEAF_INCOME_PER_TICK`, per leaf *cell* of raw field
/// light) was re-derived every time anything upstream moved: 0.05 → 0.01
/// when `leaf_cluster` went to five cells per node (income quintupled and
/// eight trees fused into a hedge), 0.01 → 0.004 when occupancy made sky
/// transmission graded (the same foliage earned far more, mean tree
/// 793 → 4,983 and the stand fused). Neither change was a change to the
/// plant's biology, and both invalidated the constant anyway — a units
/// problem, not a coupling problem.
///
/// So the currency is now `L_node = MAX_LIGHT × leaf_cluster`: what one
/// healthy node's spray intercepts in open sky at noon. Income is
/// `intercepted / L_node × INCOME_PER_NODE`, which is invariant under
/// `MAX_LIGHT`, `leaf_cluster`, and the light model — a node still earns
/// one node's worth through all of them — and the number is readable:
/// carbon per fully-lit node per tick. 0.08 is the old operating point
/// expressed in the new unit (0.004 × 20), not a re-tune.
const INCOME_PER_NODE: f32 = 0.08;

/// The canonical light unit: what one node's spray intercepts in open sky
/// at noon. See `INCOME_PER_NODE` — every constant that used to be
/// denominated in raw summed field light divides by this instead, which is
/// what makes them survive light-model and cluster-size changes.
fn l_node(leaf_cluster: u8) -> f32 {
    crate::sim::field::MAX_LIGHT * leaf_cluster.max(1) as f32
}

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
/// What a root tip's share of the growth pool is worth relative to a shoot
/// tip's when the plant is under no water stress at all.
///
/// Below 1.0 because a well-watered plant should be spending on canopy --
/// that is what buys it carbon -- but well above 0, because roots still
/// have to extend into fresh soil as the ground around them dries. The
/// stress term added to it in `allocate_to_frontier` is what makes a thirsty
/// plant reverse the priority.
const ROOT_BIAS_AT_FULL_WATER: f32 = 0.5;

/// Stomatal term below which a plant will spend carbon re-initiating a root
/// tip. At or above it, demand is being met and canopy is the better buy.
///
/// Not 1.0: the term rings slightly around a met demand from tick to tick,
/// and a threshold sitting exactly at the top would fire on the noise.
const ROOT_REINITIATION_STATUS: f32 = 0.95;

/// Step onto tissue *above* the parent — the child is standing on it.
///
/// **Free, deliberately.** A vertical stem is in pure compression and
/// should stand to any height, exactly as `a_stone_tower_stands_however_
/// tall_it_gets` asserts for stone. A whole trunk therefore relaxes to 0.
///
/// `Material::support_cost_below` is clamped to `.max(1)` for the inert
/// path, and `material.rs` explains why: there, a zero let a column relax
/// to `aux == 0`, and `load::evaluate` treats distance 0 as *anchored*, so
/// the structure became immune to every failure mode including hanging in
/// mid-air. **That hole does not exist here, because this number does not
/// answer "is it attached".** Attachment is `support == u16::MAX`, a
/// separate question decided by whether the anchor walk reached the cell at
/// all. A column of zeroes is a column that cannot fail *in bending*, which
/// is correct; cut its base and every cell in it goes unreached and comes
/// down regardless of its distance.
const SUPPORT_COST_STANDING: u16 = 0;

/// Step that moves sideways, diagonals included.
///
/// This is what makes the number a **cantilever** measure rather than a
/// path length: a cell's distance is essentially its horizontal reach from
/// the stem carrying it, which is what "unsupported span" has always meant
/// for a branch. A 150-cell trunk reads 0 and an 8-cell limb reads 8.
const SUPPORT_COST_REACH: u16 = 1;

/// Extra charge when a child hangs *below* its parent.
///
/// Tension is dear, the same ordering `Material::support_cost_above`
/// encodes for stone (`stone.ron` sets `above: 3` against `below: 1`).
const SUPPORT_COST_HANGING: u16 = 2;

/// Whether this cell is one of its organism's structural anchors.
///
/// Two ways to be anchored, and the second is new:
///
/// - **Touching `Solid` ground** — the rule the old bounded search used,
///   and the direct generalization of the inert path's "touches BEDROCK".
///   A trunk resting on stone is anchored exactly as a stone span touching
///   bedrock is.
/// - **Root tissue embedded in water-holding soil.** This is what finally
///   makes `CellType::RootTip`'s own doc true — it has claimed since the
///   substrate rewrite that "structural.rs's organism branch anchors its
///   reachability search specifically on `RootTip` cells", and the code
///   anchored on nothing of the sort. A root threaded through soil really
///   does hold, which `update.rs`'s `reinforces_powder` already models from
///   the soil's side (root-threaded soil holds a slope bare soil loses); this
///   is the same fact read from the plant's side.
///
/// Discriminated by `reinforces_powder` rather than cell type, for the same
/// reason `organism_upkeep` does it that way: a retired root and a retired
/// branch are both `MatureBody`, and only the material tells them apart.
fn is_structural_anchor(world: &World, x: i32, y: i32, organism_id: u16) -> bool {
    let cell = world.get(x, y);
    if cell.organism_id() != organism_id {
        return false; // the list can outlive the grid by a tick
    }
    let root_tissue =
        world.materials.get(cell.material).reinforces_powder || organism::cell_type(cell.aux()) == Some(CellType::RootTip);
    NEIGHBOURS_4.iter().any(|&(dx, dy)| {
        let n = world.get(x + dx, y + dy);
        let m = world.materials.get(n.material);
        m.kind == MaterialKind::Solid || (root_tissue && m.kind == MaterialKind::Powder && m.water_capacity > 0)
    })
}

/// Recompute every cell's weighted distance to this organism's nearest
/// anchor — **from the anchors outward**, once per organism per tick.
///
/// Replaces `structural::organism_is_supported`, which ran a fresh bounded
/// BFS *outward from the cell being checked* on every structural check. See
/// `OrganismCell::support` for the two defects that had, both of which this
/// shape fixes rather than tunes: there is no span budget to run out of, so
/// a check fired high in a crown no longer amputates it, and the walk is
/// eight-connected like `Grow`, so a diagonal branch is not read as
/// disconnected.
///
/// A Dijkstra rather than a plain BFS because the step costs are 0/1/2 —
/// standing is free, reaching sideways costs, hanging costs most — the same
/// weighting the inert path applies through `support_cost_below/beside/
/// above`, and the same `BinaryHeap<Reverse<..>>` shape
/// `structural::compute_world_distances` already uses. A plain queue would
/// settle cells out of order and hand back distances that are merely
/// plausible.
///
/// Cells the walk never reaches keep `u16::MAX`: they are no longer part of
/// anything that touches the ground. Newly-unreached cells schedule their
/// own structural check, which is what turns a cut trunk into a crown that
/// comes down instead of one that floats and keeps growing.
///
/// Cost: one heap walk over a cell list per organism per
/// `ORGANISM_TICK_INTERVAL`. The list is built and sorted here rather than
/// shared with `accumulate_support` below, which needs a `collar_y` this
/// pass must not depend on — an organism with no shoot still has to know
/// whether it is attached.
fn anchor_support(world: &mut World, organism_id: u16) {
    let Some(state) = world.organism(organism_id) else { return };
    if state.cells.is_empty() {
        return;
    }
    // Sorted for the same determinism reason every other per-organism pass
    // sorts: `cells` is a `HashMap` with no stable iteration order, and the
    // heap's tie-break has to be a property of the world rather than of the
    // hasher's seed.
    let mut cells: Vec<(i32, i32)> = state.cells.keys().copied().collect();
    cells.sort_unstable_by_key(|&(x, y)| (y, x));
    let index: std::collections::HashMap<(i32, i32), usize> = cells.iter().enumerate().map(|(i, &p)| (p, i)).collect();

    let mut dist = vec![u16::MAX; cells.len()];
    let mut heap: std::collections::BinaryHeap<std::cmp::Reverse<(u16, i32, i32, usize)>> = std::collections::BinaryHeap::new();
    for (i, &(x, y)) in cells.iter().enumerate() {
        if is_structural_anchor(world, x, y, organism_id) {
            dist[i] = 0;
            heap.push(std::cmp::Reverse((0, y, x, i)));
        }
    }
    while let Some(std::cmp::Reverse((d, _, _, i))) = heap.pop() {
        if d > dist[i] {
            continue; // a better path settled this cell already
        }
        let (x, y) = cells[i];
        for (dx, dy) in NEIGHBOURS_8 {
            let Some(&j) = index.get(&(x + dx, y + dy)) else { continue };
            // `dy > 0` puts the child *below* its parent, so it hangs from
            // it; `dy < 0` puts it above, standing on it. Sideways is
            // charged separately, so a diagonal pays for the reach it makes
            // as well as for the direction it went.
            let vertical = if dy > 0 { SUPPORT_COST_HANGING } else { SUPPORT_COST_STANDING };
            let step = vertical + if dx != 0 { SUPPORT_COST_REACH } else { 0 };
            let next = d.saturating_add(step);
            if next < dist[j] {
                dist[j] = next;
                let (nx, ny) = cells[j];
                heap.push(std::cmp::Reverse((next, ny, nx, j)));
            }
        }
    }

    for (i, &(x, y)) in cells.iter().enumerate() {
        let was = world.organism_cell(x, y).map_or(0, |c| c.support);
        if let Some(slot) = world.organism_cell_mut(x, y) {
            slot.support = dist[i];
        }
        // **A distance that rose is the signal, and only a rise.** Straight
        // from the inert path's own reasoning (`structural::tick`): a
        // distance that *fell* means a better load path was found and
        // nothing can newly break because of it, so the falling half of a
        // wavefront costs nothing; a rise is precisely the direction that
        // can cause a failure.
        //
        // This is also the answer to a bug the first version of this pass
        // had. Scheduling only on the `u16::MAX` transition caught
        // detachment but never the cantilever case, and on the organism
        // path a check that finds a cell supported returns *no* sites — so
        // a beam whose only check fired before the organism's first tick
        // read the default `support` of 0, declared itself fine, and was
        // never asked again. The distance arrived a tick later with nothing
        // left to consume it.
        //
        // A cell that is already unreached and stays unreached does not
        // re-schedule: it is either about to be checked or has no
        // `breaks_into` to become, and re-queuing the whole detached piece
        // every tick would be a permanent load for as long as it existed.
        if dist[i] > was {
            world.schedule_structural_check(x, y);
        }
    }
}

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
/// Re-initiate a root tip from mature root tissue when the plant is short
/// of water — **the root analogue of `break_buds`, and the mechanism whose
/// absence made the whole water economy unstable.**
///
/// A shoot can always restart: extension deposits a `DormantBud` at every
/// node, so a bud bank accumulates and `break_buds` draws on it whenever the
/// economy allows. **Roots had no reservoir at all.** A seed germinates with
/// exactly one `RootTip`, new ones come only from that tip's own branching,
/// and a tip that cannot afford `cost` for `ORGANISM_STALE_LIMIT` ticks
/// retires to `MatureBody` — deliberately, and that gate must not be
/// loosened, because "a starved tip eventually stops" is what makes growth
/// terminate at all.
///
/// The consequence was that one early carbon shortage ended root growth
/// *permanently*. Measured: with transpiration charged, root cells settled
/// at a median of **3** against a baseline of 346, and the stomatal term sat
/// at 0.16 — and weighting the growth pool toward root tips (functional
/// balance) moved the stand by 3%, because there were no root tips left to
/// favour. An almost-zero delta from a lever that should be decisive is the
/// signal that the condition it keys on is degenerate.
///
/// Real roots do exactly this: laterals initiate from pericycle tissue along
/// an existing root throughout its life, not only at a growing apex.
///
/// Gated on water stress rather than run unconditionally, so a plant with
/// its demand fully met spends on canopy instead — that, with
/// `ROOT_BIAS_AT_FULL_WATER`, is the whole of functional balance.
fn break_root_tips(world: &mut World, organism_id: u16) {
    let Some(state) = world.organism(organism_id) else { return };
    let species_id = state.species;
    let status = state.water_status;
    if status >= ROOT_REINITIATION_STATUS {
        return; // demand met -- nothing to fix, and canopy is the better buy
    }
    let Some((cost, max_active_tips)) =
        world.species.get(species_id).behaviors(CellType::RootTip).iter().find_map(|b| match b {
            Behavior::Grow { cost, max_active_tips, .. } => Some((*cost, *max_active_tips)),
            _ => None,
        })
    else {
        return; // a species whose roots do not grow
    };
    // **Priced in the same currency the new tip will spend in.** Both the
    // charge and the stake below scale, and they have to move together: the
    // stake exists so the tip's first `Grow` check is not guaranteed to
    // fail, and that check is against `cost x density`. Scaling only the
    // stake would mint carbon; scaling neither leaves a dense plant's every
    // re-initiated root dead on arrival.
    let cost = cost * wood_density_mult(world, organism_id);

    let mut cells: Vec<(i32, i32)> = state.cells.keys().copied().collect();
    cells.sort_unstable_by_key(|&(x, y)| (y, x));

    // Candidates, and the richest cell to pay from -- the same shape
    // `break_buds` uses, and for the same reason: the trunk sits near the
    // carbon cap while the frontier starves.
    let mut tips = 0usize;
    let mut richest: Option<(i32, i32, f32)> = None;
    let mut candidates: Vec<(i32, i32, f32)> = Vec::new();
    for &(x, y) in &cells {
        let cell = world.get(x, y);
        if cell.organism_id() != organism_id {
            continue;
        }
        let carbon = world.carbon_at(x, y);
        if richest.is_none_or(|(_, _, best)| carbon > best) {
            richest = Some((x, y, carbon));
        }
        match organism::cell_type(cell.aux()) {
            Some(CellType::RootTip) => tips += 1,
            Some(CellType::MatureBody) if world.materials.get(cell.material).reinforces_powder => {
                // **Scored by the water actually available around it**, so a
                // new tip starts where there is something to drink and grows
                // from there. This is hydrotropism expressed as a placement
                // decision rather than a steering one, and it costs nothing
                // extra: the same four-neighbour look `Absorb` already does.
                let mut wet = 0.0f32;
                let mut open = false;
                for (dx, dy) in NEIGHBOURS_4 {
                    let n = world.get(x + dx, y + dy);
                    let m = world.materials.get(n.material);
                    if m.water_capacity > 0 {
                        wet += update::plant_available_fraction(n);
                        open = true;
                    }
                }
                // Only tissue that still has soil to grow into: converting a
                // cell walled in by its own root system spends the cost on a
                // tip that has nowhere to go and ages straight back out.
                if open {
                    candidates.push((x, y, wet));
                }
            }
            _ => {}
        }
    }
    if tips >= max_active_tips as usize || candidates.is_empty() {
        return;
    }
    let Some((rx, ry, held)) = richest else { return };
    if held < cost {
        return;
    }

    // Deterministic pick: best score, ties broken row-major. `cells` is a
    // `HashMap` underneath and an unstable order would make the choice a
    // property of the hasher's seed rather than of the world.
    candidates.sort_unstable_by(|a, b| b.2.total_cmp(&a.2).then((a.1, a.0).cmp(&(b.1, b.0))));
    let (bx, by, _) = candidates[0];
    let cell = world.get(bx, by);
    world.set(bx, by, cell.with_aux(organism::pack_cell_type(CellType::RootTip)));
    write_carbon(world, rx, ry, held - cost);
    // Stake the new tip so its first `Grow` check is not guaranteed to fail
    // -- the same courtesy `break_buds` extends to a flushing bud, and for
    // the same reason: a fresh cell reads `resource = 0` and `Grow` runs
    // before any income.
    let stake = world.carbon_at(bx, by).max(cost);
    write_carbon(world, bx, by, stake);
    let site = reschedule_organism(bx, by, organism_id, 0, 0, world.frame + ORGANISM_TICK_INTERVAL);
    world.schedule_active_site(site);
}


fn break_buds(world: &mut World, organism_id: u16) {
    let Some(state) = world.organism(organism_id) else { return };
    let species_id = state.species;
    let mut cells: Vec<(i32, i32)> = state.cells.keys().copied().collect();
    cells.sort_unstable_by_key(|&(x, y)| (y, x));

    // The price of a flush, the cost this bud's tip will then pay per
    // growth step, and the species' tip-concurrency cap -- all read from
    // the species' own `GrowingTip` `Grow`, so a species cannot set them
    // inconsistently.
    let (Some((cost, max_active_tips, leaf_cluster)), Some(bud_cost)) = (
        world.species.get(species_id).behaviors(CellType::GrowingTip).iter().find_map(|b| match b {
            Behavior::Grow { cost, max_active_tips, leaf_cluster, .. } => Some((*cost, *max_active_tips, *leaf_cluster)),
            _ => None,
        }),
        world.species.get(species_id).behaviors(CellType::DormantBud).iter().find_map(|b| match b {
            Behavior::BudBreak { cost, acrotony, .. } => Some((*cost, *acrotony)),
            _ => None,
        }),
    ) else {
        return; // a species with no buds, or none that can break
    };
    let (bud_cost, acrotony) = bud_cost;
    // The shoot's vertical span, for acrotony's positional term. Both ends
    // come from the last upkeep walk; a plant that has not had one yet (or
    // has no shoot) scores every bud as position 1.0.
    let (collar, shoot_top) = (state.collar_y, state.shoot_top_y);

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
            // Water-limited light, not raw light -- see
            // `allocate_to_frontier` for why the *same* weighting has to
            // appear in both places.
            Some(CellType::Leaf) => intercepted += ambient_light_above(world, x, y) * water_status(world, x, y),
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
                // **Acrotony's positional preference** -- elevation runs 0
                // at the collar to 1 at the shoot's top, and the species'
                // signed `acrotony` scales the bud's score by
                // `1 + acrotony * (elevation - 0.5)`. Positive renews at
                // the top (a tree keeps crowning); negative feeds the base
                // (a shrub keeps throwing new axes from its foot) -- the
                // whole-plant-scale habit flip the botany review verified.
                // Floored so extreme values reorder buds without zeroing
                // any of them; light-per-crowding still decides among buds
                // at the same height.
                let position = match (acrotony != 0.0, collar, shoot_top) {
                    (true, Some(collar), Some(top)) if collar > top => {
                        let elevation = (collar - y) as f32 / (collar - top) as f32;
                        (1.0 + acrotony * (elevation.clamp(0.0, 1.0) - 0.5)).max(0.05)
                    }
                    _ => 1.0,
                };
                buds.push((x, y, light / (1.0 + candidate_crowding(world, x, y)) * position));
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
    // The economic bound, throttled by the species' own concurrency cap.
    // The cap used to bind only in `Grow`, so a flush could push a plant
    // past it — under-enforcement the tripwire test caught (16 against 14)
    // the moment multiplicative crowding let crowded tips live long enough
    // for the cap to matter at all. One gate, both creators of frontier.
    // Income over the price of a growth step -- so the price is this
    // individual's, not the species'. Against the unscaled cost a dense
    // plant was allowed a frontier its income could not feed and a pioneer
    // capped below what it could afford, which cancels the half of the
    // density trade that is supposed to make cheap wood grow faster.
    let step_cost = cost * wood_density_mult(world, organism_id);
    let supportable =
        ((intercepted / l_node(leaf_cluster) * INCOME_PER_NODE / step_cost).floor() as usize).min(max_active_tips as usize);
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
    // The richest cell pays the flush price; the bud keeps its own stake.
    //
    // This used to `write_carbon(bx, by, bud_cost)` -- an assignment, which
    // silently destroyed whatever the bud was already holding on every
    // flush (a bud near the 4.0 cap lost ~3.8, unaccounted), and disagreed
    // with `Behavior::BudBreak`'s own doc. The floor at `bud_cost` is kept:
    // a fresh tip must afford at least one growth step or it starves on its
    // first tick, and the price the richest cell just paid is what funds
    // the top-up when the bud itself was poor. Re-reading the bud's carbon
    // after paying the richest cell is what makes the self-pay case (the
    // bud *is* the richest cell) come out right with no special case: it
    // pays, then keeps the remainder, floored.
    // The floor is `bud_cost` **or one of this individual's growth steps,
    // whichever is larger**. `bud_cost` (0.2) was written to be "roughly
    // one growth step's worth" and matches tree.ron's shoot cost exactly at
    // the authored density -- so the moment density scales that step, a
    // dense plant's flush was floored below its own first `Grow` check and
    // the comment's promise inverted into a guaranteed starve.
    write_carbon(world, rx, ry, held - bud_cost);
    let bud_stake = world.carbon_at(bx, by);
    write_carbon(world, bx, by, bud_stake.max(bud_cost).max(step_cost));
    // A flushed bud is an *axillary* meristem -- it is a lateral by
    // definition, so it starts the next tier exactly as `Grow`'s own branch
    // child does. Without this a crown rebuilt from buds would inherit
    // trunk parameters and grow straight up as a second trunk.
    let order = world.organism_cell(bx, by).map_or(0, |c| c.order);
    write_order(world, bx, by, order.saturating_add(1));
    // `path_len` is deliberately **not** touched here. A flushing bud is
    // not a new cell -- it is a stem node that already sits at its own
    // distance from the collar, and the axis it launches continues from
    // exactly there. Re-stamping it would reset a lateral high in the crown
    // to path zero and hand it the full turgor budget a second time.
    // A flushed bud launches with **no inherited heading**. The bud cell
    // is a retired stem node, so its stored heading is the stem's own
    // (near-vertical on a trunk) -- and a lateral that starts life
    // believing it is travelling straight up steers by that belief:
    // measured as whole conifer stands sweeping one direction, because
    // every trunk-bud tier fed the vertical case of the plagiotropic side
    // pick. Zeroed, the first Grow falls back to away-from-supply, which
    // points away from the stem that bears the bud -- where a real
    // axillary shoot goes.
    if let Some(slot) = world.organism_cell_mut(bx, by) {
        slot.heading = (0.0, 0.0);
    }
    let site = reschedule_organism(bx, by, organism_id, 0, 0, world.frame + ORGANISM_TICK_INTERVAL);
    world.schedule_active_site(site);
}

fn allocate_to_frontier(world: &mut World, organism_id: u16) {
    let Some(state) = world.organism(organism_id) else { return };
    if state.cells.is_empty() {
        return;
    }
    // The species' own node size, for the income currency — see
    // `INCOME_PER_NODE`. A species with no shoot `Grow` has no light
    // economy to allocate.
    let leaf_cluster = world
        .species
        .get(state.species)
        .behaviors(CellType::GrowingTip)
        .iter()
        .find_map(|b| match b {
            Behavior::Grow { leaf_cluster, .. } => Some(*leaf_cluster),
            _ => None,
        })
        .unwrap_or(1);
    // Sorted for the same determinism reason `transport` sorts: `cells` is
    // a `HashMap` and `f32` addition is not associative.
    let mut cells: Vec<(i32, i32)> = state.cells.keys().copied().collect();
    cells.sort_unstable_by_key(|&(x, y)| (y, x));

    let mut frontier: Vec<(i32, i32)> = Vec::new();
    let mut frontier_is_root: Vec<bool> = Vec::new();
    let mut donors: Vec<(i32, i32)> = Vec::new();
    let mut intercepted = 0.0f32;
    for &(x, y) in &cells {
        let cell = world.get(x, y);
        if cell.organism_id() != organism_id {
            continue;
        }
        match organism::cell_type(cell.aux()) {
            Some(t) if is_frontier(t) => {
                frontier.push((x, y));
                frontier_is_root.push(t == CellType::RootTip);
            }
            Some(CellType::Leaf) => {
                // **Intercepted light, not leaf count** -- Palubicki's `Q`.
                // A leaf buried inside the canopy sits under blocked field
                // blocks and reads almost nothing, so it contributes almost
                // nothing. Counting leaves equally made income grow with
                // mass and the stand fused into a slab; weighting by light
                // is what makes self-shading bound the plant.
                // **Water-limited, and the gate goes here rather than on
                // the credit.** `break_buds` computes `supportable` from
                // the identical expression `intercepted / l_node *
                // INCOME_PER_NODE`, so a plant's growth budget and its
                // permission to open a new bud are one number in two
                // places. Charging the deficit anywhere else -- a per-cell
                // debit, say -- would move one and not the other, and the
                // two gates would disagree about what the plant can afford.
                //
                // Liebig's law of the minimum, in the only form this engine
                // needs it: income is bounded by whichever of light and
                // water is scarcer, so a rootless plant in full sun earns
                // nothing at all however much light it intercepts.
                intercepted += ambient_light_above(world, x, y) * water_status(world, x, y);
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
    let income = intercepted / l_node(leaf_cluster) * INCOME_PER_NODE;
    let stock: f32 = donors.iter().map(|&(x, y)| world.carbon_at(x, y)).sum();
    let pool = income.min(stock);
    // **Functional balance: the plant invests in whatever is limiting it.**
    //
    // The pool used to be split evenly across every frontier cell, root tips
    // included, so a plant that could not supply its canopy kept funding
    // more canopy. That is a death spiral and it was measured as one: with
    // demand charged and the split left even, root cells fell from a median
    // of 346 to **3** and the stomatal term settled at 0.16 -- water-limited
    // income starves the very roots that would fix it, and the more it
    // starves them the more water-limited it gets.
    //
    // Real plants resolve this the same way, and it is one of the
    // best-established rules in plant ecology: allocate to the organ that
    // captures the scarcer resource. A shoot tip's weight is 1; a root tip's
    // rises as the stomatal term falls, so a well-watered plant spends on
    // canopy and a thirsty one spends on roots. `ROOT_BIAS_AT_FULL_WATER`
    // keeps roots funded even at status 1.0, because a plant with no water
    // stress still has to replace root tissue and extend into fresh soil.
    //
    // This is also what makes root traits worth selecting on -- and, with
    // `water_capacity_of` reading root mass, what makes root mass one
    // quantity with two consequences, uptake now and anchorage later.
    let status = world.organism(organism_id).map_or(1.0, |s| s.water_status);
    // **Genotype slot 6 -- root:shoot allocation bias.** A constitutively
    // root-heavy individual against a canopy gambler, multiplying the
    // whole root weight so it scales the plastic (1 - status) response
    // too; bounded above by the allometric MAX_ROOT_FRACTION cap like
    // everything else that funds roots. Variance rides the shoot Grow's
    // vector exactly as pipe_ratio's does -- one plant, one genotype.
    let alloc_variance = world
        .organism(organism_id)
        .map(|s| s.species)
        .and_then(|sp| {
            world.species.get(sp).behaviors(CellType::GrowingTip).iter().find_map(|b| match b {
                Behavior::Grow { genotype_variance, .. } => Some(genotype_variance[6]),
                _ => None,
            })
        })
        .unwrap_or(0.0);
    let root_weight = (ROOT_BIAS_AT_FULL_WATER + (1.0 - status)) * genotype(world, organism_id, 6, alloc_variance);
    let total_weight: f32 = frontier_is_root.iter().map(|&r| if r { root_weight } else { 1.0 }).sum();
    if total_weight <= 0.0 {
        return;
    }

    for (index, &(fx, fy)) in frontier.iter().enumerate() {
        let share = pool * (if frontier_is_root[index] { root_weight } else { 1.0 }) / total_weight;
        if share <= 0.0 {
            continue;
        }
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
    // Slots 4 and 7 of the genome ride this one fetch. `pipe_ratio` lives
    // on `SecondaryThicken` and the stomatal settle at the bottom of this
    // function, but the genome lives on `Grow`, so both are read from the
    // species' own `GrowingTip` vector rather than duplicated onto other
    // behaviours -- one plant, one genotype.
    let (shoot_variance, upkeep_leaf_cluster) = world
        .species
        .get(species_id)
        .behaviors(CellType::GrowingTip)
        .iter()
        .find_map(|b| match b {
            Behavior::Grow { genotype_variance, leaf_cluster, .. } => Some((*genotype_variance, *leaf_cluster)),
            _ => None,
        })
        .unwrap_or(([0.0; organism::GENOTYPE_TRAITS], 1));
    let pipe_variance = shoot_variance[4];

    // **What a primed lateral has to be able to afford**: one root growth
    // step, in the individual's own currency (density scales the price of
    // every cell it builds). `None` for a species whose roots do not grow,
    // which switches the whole primed path off rather than defaulting it.
    let root_grow = world.species.get(species_id).behaviors(CellType::RootTip).iter().find_map(|b| match b {
        Behavior::Grow { cost, max_active_tips, .. } => Some((*cost, *max_active_tips)),
        _ => None,
    });
    let density = wood_density_mult(world, organism_id);
    let (root_step_cost, root_max_tips) = root_grow.map_or((f32::INFINITY, 0), |(c, m)| (c * density, m as usize));

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

    // The leaf-economy multipliers, once for the whole walk -- the demand
    // sum below and the credit arm both read them, and they must agree.
    let (leaf_rate_mult, leaf_transp_mult) = leaf_econ_mults(world, organism_id);

    // Leaves per row, then a running total downward, so every cell can read
    // "how much foliage do I carry" without a traversal of its own.
    let mut leaves_in_row: std::collections::HashMap<i32, u32> = std::collections::HashMap::new();
    let (mut root_cells, mut shoot_cells) = (0u32, 0u32);
    let mut demand = 0.0f32;
    // Primed lateral sites that can afford themselves this tick, and the
    // root tips already standing -- both gathered in the walk below so the
    // branching decision costs no traversal of its own.
    let mut primed_ready: Vec<(i32, i32, f32)> = Vec::new();
    let mut root_tips = 0usize;
    let mut richest_cell: Option<(i32, i32, f32)> = None;
    let mut collar_y: Option<i32> = None;
    let mut shoot_top_y: Option<i32> = None;
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
            // **Transpirational demand, summed over foliage in the walk
            // that is already happening.** Driven by leaf area, which is
            // what `TRANSPIRATION_PER_ROOT_CELL`'s own doc says it should
            // always have been -- "the canopy is the pump" -- and scaled by
            // the light each leaf reads, because stomata open in light.
            //
            // A *sum*, not a count: `CLAUDE.md` prefers a continuous
            // quantity over a count of bad cells, because counts give
            // knife-edge margins and sums separate cleanly.
            if let Some(t) = ty {
                let transpiration = world.species.get(species_id).behaviors(t).iter().find_map(|b| match b {
                    Behavior::Photosynthesize { transpiration, .. } => Some(*transpiration),
                    _ => None,
                });
                if let Some(rate) = transpiration {
                    let light = ambient_light_above(world, cx, cy);
                    // The leaf-economy allele's bill: the expensive leaf
                    // spends more water for its higher rate, which is the
                    // whole trade -- see `LEAF_TRANSPIRATION_ALLELES`.
                    demand += rate * leaf_transp_mult * (light / crate::sim::field::MAX_LIGHT).clamp(0.0, 1.0);
                }
            }
        }
        // The cell to fund a lateral from -- the trunk sits near the carbon
        // cap while the frontier starves, which is why both sibling
        // mechanisms pay from here rather than from the site.
        {
            let held = world.carbon_at(cx, cy);
            if richest_cell.is_none_or(|(_, _, best)| held > best) {
                richest_cell = Some((cx, cy, held));
            }
        }
        // Root or shoot, tallied in the walk that is already happening.
        // `rootwood` is the discriminator rather than cell type, because a
        // retired root and a retired branch are both `MatureBody`.
        if world.materials.get(c.material).reinforces_powder || ty == Some(CellType::RootTip) {
            root_cells += 1;
            // **The primed site's own affordability check**, in the walk
            // that is already happening rather than in a traversal of its
            // own. A site marked by a passing tip becomes a lateral once
            // the carbon reaching it clears a step's cost -- the bill the
            // tip could never meet twice in one tick, met later by the
            // cell that will actually spend it. See `OrganismCell::primed`.
            if ty == Some(CellType::RootTip) {
                root_tips += 1;
            } else if world.organism_cell(cx, cy).is_some_and(|o| o.primed) {
                // **Scored by the water around it, and funded from the
                // plant** -- the same shape `break_root_tips` and
                // `break_buds` both already use, and the correction to a
                // first attempt that failed by measurement.
                //
                // That attempt required the site to hold a step's cost in
                // its *own* carbon. It converted essentially nothing: 35
                // primed sites visited 2,976 times over 12,000 frames
                // produced **zero** laterals, because priming marks exactly
                // the cells a tip has just retired -- the newest, poorest
                // tissue at the frontier -- while the 18% of root cells
                // that do hold a step's cost sit inboard, where carbon
                // transits from the shoot. The site is the right place to
                // decide; it was never the right place to pay from.
                let mut wet = 0.0f32;
                let mut open = false;
                for (dx, dy) in NEIGHBOURS_4 {
                    let n = world.get(cx + dx, cy + dy);
                    if world.materials.get(n.material).water_capacity > 0 {
                        wet += update::plant_available_fraction(n);
                        open = true;
                    }
                }
                // Only tissue with soil left to grow into: a site walled in
                // by its own root system spends the cost on a tip that ages
                // straight back out.
                //
                // **A walled site is un-primed rather than re-scanned
                // forever**, and that is a frame-cost fix with a measured
                // number behind it. Primed sites that can never convert
                // accumulate as a stand matures, and each one paid a
                // four-neighbour scan every upkeep tick: on the `ascii`
                // tree scene, paired and alternating over 8 runs each, the
                // settled worst frame went 0.251 ms to 0.329 ms (+31%) with
                // them left in. Clearing turns a permanent per-tick cost
                // into a one-off. A tip passing again re-primes, which is
                // the right trigger: if the soil around a buried site opens
                // up, it takes new growth to notice.
                if open {
                    primed_ready.push((cx, cy, wet));
                } else {
                    write_primed(world, cx, cy, false);
                }
            }
        } else {
            shoot_cells += 1;
            // The collar is the *lowest* shoot cell -- where the shoot
            // meets the root system. Taken from shoot tissue rather than
            // from the organism's overall extent, which would sit at the
            // bottom of the root system and make every shoot cell read as
            // implausibly high.
            collar_y = Some(collar_y.map_or(cy, |c: i32| c.max(cy)));
            // And the shoot's top, so `acrotony` can place a bud on the
            // 0..1 span between the two.
            shoot_top_y = Some(shoot_top_y.map_or(cy, |c: i32| c.min(cy)));
        }
    }
    if let Some(state) = world.organism_mut(organism_id) {
        state.root_cells = root_cells;
        state.shoot_cells = shoot_cells;
        state.collar_y = collar_y;
        state.shoot_top_y = shoot_top_y;
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
                Behavior::Photosynthesize { rate, shade_death, transpiration, drought_death } => {
                    let light = ambient_light_above(world, cx, cy);
                    // Abscission — **a graded pressure, not a threshold**,
                    // chosen on a fair paired measurement and not on the
                    // first one. The first sweep read as "any setting
                    // collapses the stand", which was a confound: the shed
                    // used to schedule a structural check, and mid-crown
                    // checks amputate (see `shed_stranded_leaves`). With
                    // the check out, a hard threshold works too — 20,044
                    // cells at cutoff 0.5 against 20,213 graded — and
                    // graded wins on the things a threshold cannot do:
                    // better crown separation (fused run 37 vs 55; the
                    // culled-on-a-line canopy re-spreads), a better-lit
                    // standing canopy (median 2.68 vs 2.15), no
                    // synchronized culls when a shadow arrives (a shading
                    // event thins over ~a thousand frames — leaves going,
                    // not a shelf being swept), and robustness to
                    // transient dips (dusk lag, a passing occluder) that a
                    // line converts into same-tick loss. The cube keeps
                    // anything better than half-lit effectively permanent.
                    // `shade_death` is the chance per tick at total
                    // darkness; 0.0 stays off.
                    //
                    // Checked before the credit, so a leaf being shed does
                    // not also earn on the tick it dies. The cell becomes
                    // `litter` rather than wood: leaving `MatureBody`
                    // behind would have shading foliage silently thicken
                    // the stem it hung from — the pipe model reading a leaf
                    // as xylem, fixed once already. It used to become
                    // `Cell::EMPTY`, and that was a conceded gap rather
                    // than a decision — foliage left the world entirely, so
                    // a forest floor never accumulated anything and there
                    // was nothing for the creature side to eat. See
                    // `shed_to_litter`.
                    if shade_death > 0.0 && organism::cell_type(world.get(cx, cy).aux()) == Some(CellType::Leaf) {
                        let darkness = (1.0 - light / crate::sim::field::MAX_LIGHT).clamp(0.0, 1.0);
                        if rng.chance(shade_death * darkness * darkness * darkness) {
                            shed_to_litter(world, cx, cy);
                            // Reclaim any spray this stranded. NOT a
                            // structural check -- see
                            // `shed_stranded_leaves` for the measured 26x
                            // reason.
                            shed_stranded_leaves(world, cx, cy, organism_id);
                            continue;
                        }
                    }
                    // Water first, carbon second and gated on it -- see the
                    // frontier arm for the ordering, and
                    // `Behavior::Photosynthesize::transpiration` for why
                    // this charge is what makes a root worth having.
                    let _ = transpiration; // charged once per organism -- see `organism_upkeep`
                    let status = water_status(world, cx, cy);
                    // **Drought abscission, the exact counterpart of the
                    // shade rule above and cubed for the same reason.** A
                    // leaf the plant cannot supply is shed, so a seedling
                    // germinated in a canopy with no soil to reach thins out
                    // and dies rather than standing there inert. Checked
                    // before the credit, like the shade rule, so a leaf
                    // being shed does not also earn on the tick it dies.
                    if drought_death > 0.0 && organism::cell_type(world.get(cx, cy).aux()) == Some(CellType::Leaf) {
                        // Desiccation, not the stomatal term: a leaf dies
                        // of drying out, not of its plant's prudence. The
                        // two are identical until a species sets
                        // `stomatal_reserve` -- see
                        // `OrganismState::water_desiccation` for the
                        // trade-inversion this split prevents.
                        let thirst = world.desiccation_at(cx, cy).clamp(0.0, 1.0);
                        if rng.chance(drought_death * thirst * thirst * thirst) {
                            shed_to_litter(world, cx, cy);
                            shed_stranded_leaves(world, cx, cy, organism_id);
                            continue;
                        }
                    }
                    resource = (resource + rate * leaf_rate_mult * light * status).min(organism::RESOURCE_SCALE);
                }
                Behavior::Transpire { rate } => {
                    transpire(world, cx, cy, rate);
                }
                Behavior::Reproduce { seed_cost, seed_chance, seed_maturity } => {
                    // Runs on every `MatureBody` cell, so the *organism's*
                    // seed rate is this chance times its canopy size -- a
                    // big tree out-breeds a small one with no rule saying
                    // so, which is the coupling that makes selection on
                    // size mean anything.
                    if seed_chance > 0.0 && shoot_cells >= seed_maturity {
                        let carbon = world.organism_cell(cx, cy).map_or(0.0, |c| c.carbon);
                        if carbon >= seed_cost && rng.chance(seed_chance) && set_seed(world, cx, cy, organism_id, seed_cost, &mut rng) {
                            write_carbon(world, cx, cy, carbon - seed_cost);
                        }
                    }
                }
                Behavior::SecondaryThicken { pipe_ratio } => {
                    // **The support this cell actually carries**, from the
                    // basipetal pass, replacing "leaves in the rows above
                    // me". The row scan was a geometric filter standing in
                    // for a topological one: a limb on the far side of the
                    // plant counted toward a stem it does not supply.
                    //
                    // In **nodes**, not raw summed light — `q_peak / L_node`
                    // — so `pipe_ratio` reads as "nodes of foliage per cell
                    // of stem width", a number with a literature analogue
                    // (a Huber-value cousin), and survives the changes that
                    // forced its four historical re-derivations (10 → 45 →
                    // 22 → 110), two of which were pure unit conversions.
                    let carried = world.organism_cell(cx, cy).map_or(0.0, |c| c.q_peak) / l_node(upkeep_leaf_cluster);
                    // Hoisted out of the call: `genotype` borrows the world
                    // and `thicken` takes it mutably.
                    let jittered_ratio = pipe_ratio * genotype(world, organism_id, 4, pipe_variance);
                    thicken(world, cx, cy, organism_id, jittered_ratio, carried, bud_survival, &mut rng);
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
                // **`Absorb` runs here now.** It used to sit in the no-op
                // group below with the comment "`Absorb` is a `RootTip`'s
                // live water uptake (mature root tissue is suberised and
                // takes up little)". That is true of a real root and was
                // fatal here: `tree.ron` caps root tips at 10, so the whole
                // plant's water income was bounded by a constant while its
                // demand scales with a canopy of over a thousand leaves.
                // See `absorb_water`.
                Behavior::Absorb { rate } => absorb_water(world, cx, cy, rate),
                Behavior::Grow { .. }
                | Behavior::Divide { .. }
                | Behavior::Germinate { .. }
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

    // **Primed sites become laterals, richest first, up to the tip cap.**
    //
    // After the walk rather than inside it: the cap is a whole-plant
    // quantity and converting mid-walk would let the count the decision
    // reads drift as the walk proceeded. Richest-first because a site
    // holding more carbon can take more steps before it starves, and
    // deterministic on ties (`cells` is a `HashMap` underneath, so an
    // unstable order would make root architecture a property of the
    // hasher's seed).
    //
    // **One lateral per tick at most, and the plant pays for it.** The
    // bill is a step's carbon, charged when the branch is actually taken
    // -- the tip never had to hold it, which is the whole repair. Capped
    // at one per tick for the same reason `break_root_tips` and
    // `break_buds` are: converting every affordable site at once would
    // spend the plant's whole stock on frontier in a single tick.
    if let Some((bx, by)) = primed_ready
        .iter()
        .max_by(|a, b| a.2.total_cmp(&b.2).then((b.1, b.0).cmp(&(a.1, a.0))))
        .map(|&(x, y, _)| (x, y))
        .filter(|_| root_tips < root_max_tips)
    {
        if let Some((rx, ry, held)) = richest_cell.filter(|&(_, _, held)| held >= root_step_cost) {
            let cell = world.get(bx, by);
            if cell.organism_id() == organism_id {
                world.set(bx, by, cell.with_aux(organism::pack_cell_type(CellType::RootTip)));
                write_primed(world, bx, by, false);
                // **A lateral starts the next tier**, exactly as a flushed
                // bud does and for the same reason. Without it every root
                // cell stays at order 0, so the root `Grow`'s `ByOrder`
                // fields cannot differentiate a primary axis from a fine
                // lateral -- `.at(order)` reads the same entry for both --
                // and the root system can only ever vary in *density*.
                // That is exactly how it read on a sheet: "more vs less
                // roots instead of fully different morphology".
                let order = world.organism_cell(bx, by).map_or(0, |c| c.order);
                write_order(world, bx, by, order.saturating_add(1));
                write_carbon(world, rx, ry, held - root_step_cost);
                // Staked so the new tip's first `Grow` check is not
                // guaranteed to fail -- the same courtesy `break_root_tips`
                // and `break_buds` both extend, and for the same reason: a
                // fresh frontier cell reads its carbon before any income.
                let stake = world.carbon_at(bx, by).max(root_step_cost);
                write_carbon(world, bx, by, stake);
                let site = reschedule_organism(bx, by, organism_id, 0, 0, world.frame + ORGANISM_TICK_INTERVAL);
                world.schedule_active_site(site);
            }
        }
    }

    // **Settle the water balance once, at the end, for the whole plant.**
    //
    // After the loop rather than before it, because `Absorb` runs inside
    // that loop on every mature root cell: this reads the stock the plant
    // actually finished the tick holding.
    //
    // Charged once per organism rather than per foliage cell, and the
    // difference is not cosmetic -- a per-cell debit oscillates against the
    // status derived from it. A plant that exactly met demand would end the
    // tick at zero, read a stomatal term of zero, and earn nothing next
    // tick despite being perfectly well watered. Drawing `min(stock,
    // demand)` in one place and reporting the fraction met is the same
    // physics without the ringing.
    //
    // `water_status` is what multiplies every photosynthetic credit and
    // every leaf's contribution to intercepted light, so this single number
    // is the entire coupling between having roots and being able to grow.
    //
    // **Genotype slot 7 -- the stomatal closure point.** Species scalar ×
    // this individual's draw gives the stock fraction below which stomata
    // begin closing: openness ramps linearly from shut at empty to fully
    // open at the reserve line, and `reserve <= 0` is the pre-closure
    // engine exactly. Computed before the mutable settle because
    // `genotype` borrows the world.
    let stomatal_reserve = world.species.get(species_id).stomatal_reserve * genotype(world, organism_id, 7, shoot_variance[7]);
    if let Some(state) = world.organism_mut(organism_id) {
        // The arithmetic lives in `settle_water`, which is where the
        // "desiccation is exactly `1 - status` until a species opts in"
        // identity is asserted -- the seam the water economy's
        // `drought_death` tuning rests on.
        let capacity = water_capacity_of(state.root_cells);
        #[cfg(test)]
        {
            // **Where closure actually fires**, bucketed by shoot size.
            // Kept rather than scratch, because the question it answers was
            // got wrong once from a plausible statistic: stock/capacity was
            // read as 0.41 against a 0.2 reserve and closure declared a
            // no-op for mature plants, from a ratio of *medians* taken
            // across different plants at one final frame. Per settle, per
            // plant, mature plants are under the line 83.5% of the time and
            // seedlings 99.1% -- the reserve is a standing throttle, not a
            // drought-only policy, because a plant chronically holds well
            // under its own root-derived capacity. `#[cfg(test)]`, and once
            // per organism tick rather than per cell.
            let bucket = if shoot_cells < 20 {
                0
            } else if shoot_cells < 200 {
                1
            } else {
                2
            };
            S8E[bucket * 2].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if stomatal_reserve > 0.0 && (state.water / capacity.max(f32::EPSILON)) < stomatal_reserve {
                S8E[bucket * 2 + 1].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        let (drawn, status, desiccation) = settle_water(state.water, capacity, demand, stomatal_reserve);
        state.water -= drawn;
        state.water_status = status;
        state.water_desiccation = desiccation;
        state.water_demand = demand;
        // Snapshot, then clear the accumulator: per tick rather than
        // cumulative, because a running total would be dominated by the
        // plant's age and could not answer "is it keeping up right now".
        state.water_uptake = state.water_uptake_acc;
        state.water_uptake_acc = 0.0;
    }
}

/// Cell types that still carry their own active site: the ones that grow.
/// Everything else is upkeep, and runs from `step_organisms`.
fn is_frontier(cell_type: CellType) -> bool {
    matches!(cell_type, CellType::Seed | CellType::GrowingTip | CellType::RootTip)
}

/// How far down a shed leaf will look for somewhere to rest.
///
/// A crown sits well above the ground and the walk has to cross it, so this
/// is deliberately generous. It bounds the work and nothing else: a leaf that
/// finds no rest inside it stays where the walk got to, never nowhere.
const LITTER_FALL_REACH: i32 = 64;

/// What a shed leaf leaves behind: a `litter` cell, not a hole.
///
/// **The shed cell stops being the plant's** -- `Cell::new` carries no
/// organism id, which is the point. It is loose matter from here on: it
/// falls, it piles at litter's own steep angle of repose, it burns as the
/// fastest fuel in the world, and it rots back into soil on `decay.rs`'s
/// schedule. That last one is what makes a forest floor a cycle rather than
/// an accumulator, and it is why this could not land until decay sites
/// stopped stranding (`Reports/open-bugs-handoff.md` §0e).
///
/// It draws from litter's palette rather than keeping the leaf's shade: the
/// greens going is what says "shed" at a glance.
///
/// `id_of` by name, like `soil`, because litter was appended to `EMBEDDED`
/// and has no `material::` constant. This runs on an abscission event, not
/// per cell per frame, so the string hash is not on a hot path. Falls back
/// to emptying the cell so a stripped asset set still sheds.
///
/// **The leaf is carried down to where it would have landed, rather than
/// written where it hung.** Writing in place and letting the powder sweep
/// take it looked equivalent and is not: a crown catches its own leaf fall.
/// Measured on the forest scene, 3,825 of 4,330 standing litter cells were
/// resting on plant tissue, with 9.5% within three rows of the surface --
/// a canopy full of debris that costs sweep time and feeds nobody, which is
/// neither what a wood looks like nor what S4 wanted litter *for*.
///
/// Requiring clear air below instead was tried and reverted: at eight cells
/// almost no leaf in a dense crown qualifies, litter fell to 157 cells and
/// the mat disappeared from the picture entirely. That filter removed the
/// mechanic rather than its waste.
fn shed_to_litter(world: &mut World, x: i32, y: i32) {
    let Some(litter) = world.materials.id_of("litter") else {
        world.set(x, y, Cell::EMPTY);
        return;
    };
    // `base_shades`, not `palette.len()`: a material may ship several
    // four-tone region families, and a random pick across all of them
    // speckles. Litter has one family today, so these agree -- reading the
    // right one now means this does not silently break if that changes.
    let shades = world.materials.get(litter).base_shades.max(1) as u32;
    let shade = world.rng.below(shades) as u8;
    // Cleared before the walk, so the leaf's own cell reads as air and is a
    // valid landing spot for the boxed-in case below.
    world.set(x, y, Cell::EMPTY);
    // Lowest air cell reached. **Starts at the leaf's own now-empty cell**,
    // which is where a leaf walled in on every side stays. An earlier version
    // started this at "no landing found" and returned without writing
    // anything, which *deleted* every leaf whose next cell down was solid --
    // a leaf low over soil or stone, or one directly above litter already
    // lying there. That is exactly the surface-reachable foliage this
    // mechanism exists to turn into food, and `CLAUDE.md` names the shape:
    // a bound on work must never gate whether the thing happens at all.
    let mut landing = y;
    let mut probe = y;
    for _ in 0..LITTER_FALL_REACH {
        let below = world.get(x, probe + 1);
        // Raw `material == EMPTY`, not `is_empty()`: the managed-aware helper
        // reads a promoted liquid body's container cells as not-empty, and
        // the question here is "can a leaf pass through".
        let air = below.material == material::EMPTY;
        // **Passes through anything owned by an organism, not merely through
        // `MaterialKind::Plant`.** A leaf does not rest on a branch it just
        // let go of, nor on an ant standing under the tree; and testing
        // ownership means litter can never be written over an organism cell.
        if !air && below.organism_id() == 0 {
            break;
        }
        probe += 1;
        if air {
            landing = probe;
        }
    }
    world.set(x, landing, Cell::new(litter, shade));
}

/// After a leaf is shed, drop any of its neighbouring leaves that no
/// longer connect to the plant.
///
/// A cluster is several cells and only the first touches the stem
/// (deliberately — `Grow`'s cluster placement keeps foliage off the
/// apex), so shedding a stem-adjacent leaf can strand the rest of its
/// spray in the air. Structural checks are reactive and will never look
/// at them, so without this they float forever.
///
/// **Not `schedule_structural_check_around`, and the difference was
/// measured at 26x.** The organism support search is hop-bounded, so a
/// structural check fired mid-crown reads any branch further than the
/// span limit from the roots as unsupported and converts it to deadwood —
/// scheduling checks from abscission amputated every tree's upper crown,
/// and the whole shedding mechanism measured as "collapses the stand at
/// any setting" (772 cells against 20,213 at the same rate, the only
/// difference being the check). Recorded here because Phase 3's damage
/// work will meet the same landmine: any mid-crown disturbance today
/// over-amputates, and that is the support model's bound, not the
/// disturbance's size.
///
/// A bounded component walk over *leaves only* asks exactly the question
/// shedding raises — "does this spray still hang from anything" — and
/// cannot touch wood. The cap is generous against the cluster size and
/// conservative on overflow: a component too big to survey completely is
/// left standing, not deleted.
pub(crate) fn shed_stranded_leaves(world: &mut World, x: i32, y: i32, organism_id: u16) {
    const COMPONENT_CAP: usize = 32;
    let mut visited: Vec<(i32, i32)> = Vec::new();
    for (sdx, sdy) in NEIGHBOURS_8 {
        let start = (x + sdx, y + sdy);
        if visited.contains(&start) {
            continue;
        }
        let c = world.get(start.0, start.1);
        if c.organism_id() != organism_id || organism::cell_type(c.aux()) != Some(CellType::Leaf) {
            continue;
        }
        let mut component = vec![start];
        let mut queue = vec![start];
        let mut anchored = false;
        let mut overflowed = false;
        while let Some((qx, qy)) = queue.pop() {
            for (dx, dy) in NEIGHBOURS_8 {
                let pos = (qx + dx, qy + dy);
                let n = world.get(pos.0, pos.1);
                if n.organism_id() != organism_id {
                    continue;
                }
                if organism::cell_type(n.aux()) == Some(CellType::Leaf) {
                    if !component.contains(&pos) {
                        if component.len() >= COMPONENT_CAP {
                            overflowed = true;
                            continue;
                        }
                        component.push(pos);
                        queue.push(pos);
                    }
                } else {
                    // Wood, a tip, a bud: the spray still hangs from the
                    // plant.
                    anchored = true;
                }
            }
            if anchored {
                break;
            }
        }
        if !anchored && !overflowed {
            for &(lx, ly) in &component {
                shed_to_litter(world, lx, ly);
            }
        }
        visited.extend(component);
    }
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
    // The did-it-fire counter for seed dormancy. Only seeds that were
    // deferred at least once count -- see `World::
    // seeds_germinated_after_waiting`.
    if world.organism(organism_id).is_some_and(|st| st.deferred_germination) {
        world.seeds_germinated_after_waiting += 1;
    }
    // No `schedule_structural_check_around` on either the new tip or the
    // root -- see the identical reasoning on `Behavior::Grow`'s own child
    // creation above. A freshly germinated seed is not yet connected to any
    // ground and is not expected to be; checking it here would destroy
    // every seedling before its root ever gets the chance to reach soil.
    // **The genotype is drawn here, and here is the only place it can be
    // drawn**: this is where the plant's real position is finally known. A
    // seed is a `Powder`, so it falls and rolls from wherever it was
    // planted, and the coordinate it comes to rest at is the one that
    // should decide what kind of individual grows. See `seed_genotype`.
    seed_genotype(world, organism_id, x, y);
    // The seed cell is `seed` material; the shoot it becomes is whatever
    // this species declares -- `wood` for every shipped tree, and the
    // reason a non-woody species is expressible at all. See
    // `SpeciesDef::shoot_material`.
    let species = world.organism(organism_id).map(|s| s.species);
    let wood = species.and_then(|sp| world.materials.id_of(&world.species.get(sp).shoot_material)).unwrap_or(cell.material);
    // After `seed_genotype`, which is what draws this individual's bands.
    let shoot_shade = banded_shade(world, organism_id, wood, Band::Bark, rng);
    world.set(
        x,
        y,
        Cell::new(wood, shoot_shade).with_organism_id(organism_id).with_aux(organism::pack_cell_type(CellType::GrowingTip)),
    );
    // **The endowment lands here.** `World::set` gives the new cell a
    // default sidecar, so the stake is written after it: a bred seed
    // starts life holding what its parent paid (`Reproduce.seed_cost`),
    // which is aimed at exactly the margin tree.ron's cost-tuning history
    // names -- a fresh cell's first `Grow` check reads its carbon before
    // any income has arrived. A planted seed's endowment is 0 and starts
    // broke, as every scene and test has always assumed. On the shoot
    // cell rather than split with the root: allocation and diffusion
    // spread it from there either way.
    let stake = world.organism(organism_id).map_or(0.0, |s| s.endowment);
    if stake > 0.0 {
        write_carbon(world, x, y, stake);
    }
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
        // anywhere. Propagation is unchanged; only the *seed* moved from a
        // hardcoded name to species data, which is the one thing that
        // comment always relied on. `update_powder`'s soil stabilization (§6d) depends on
        // being able to ask "is this a root" from the material id alone,
        // which is the reason rootwood is a material at all.
        let root_material =
            species.and_then(|sp| world.materials.id_of(&world.species.get(sp).root_material)).unwrap_or(cell.material);
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
/// **along `axis`** — this stem's true cross-section.
///
/// **`axis`, not the row, and that is the whole point.** This walked the
/// row (`y` fixed) while `thicken` placed its new cell along
/// `cross_section_axis` — so the function deciding *whether* a stem is wide
/// enough and the function deciding *where* to widen it used different
/// geometry. For a vertical trunk they agree; for any stem that leans, the
/// row-walk runs *lengthwise down the stem* and swallows every limb it
/// touches on the way.
///
/// The consequence was the whole "plants look like felt" problem. A trunk
/// measured itself at 30-70 cells wide, `leaf_count / stem_width` fell far
/// below `pipe_ratio`, and `thicken` returned without widening — every
/// time, forever. Nothing was ever more than about four cells thick, so
/// there was no trunk/limb/twig hierarchy and every species rendered as the
/// same mat of one-cell strands with a green crust.
///
/// **This is the third time this denominator has been wrong, and the first
/// two fixes changed its scope rather than its axis.** `pipe_ratio`'s own
/// history records value 10 "dividing by the whole row's cells — a limb
/// elsewhere on the row suppressed the trunk", replaced by a contiguous-run
/// denominator. A contiguous run is still a *row* run: it still merges any
/// limb that happens to touch, and on a diagonal stem it still measures
/// length. Narrowing the scope hid how wrong the axis was.
///
/// **Leaves are excluded on purpose.** The pipe model's cross-section is
/// xylem, and foliage is not xylem. Counting leaves inflated the
/// denominator by roughly 10% of all cells, and worse, `leaf_count` on the
/// numerator counts `Leaf | GrowingTip` — so the same cell appeared on both
/// sides of the ratio.
fn stem_run(world: &World, x: i32, y: i32, organism_id: u16, axis: [(i32, i32); 2]) -> usize {
    let woody = |wx: i32, wy: i32| {
        let c = world.get(wx, wy);
        c.organism_id() == organism_id && organism::cell_type(c.aux()) != Some(CellType::Leaf)
    };
    if !woody(x, y) {
        return 1;
    }
    let mut run = 1usize;
    for (dx, dy) in axis {
        let mut k = 1;
        while woody(x + dx * k, y + dy * k) && k <= MAX_STEM_RUN {
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
    let stem_width = stem_run(world, x, y, organism_id, axis);
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
            // **Banded here too, and this is the site that matters most for
            // bark colour**: secondary thickening lays far more wood than
            // extension does, so a trunk whose girth cells kept the old
            // uniform draw would read as the old brown however the shoot
            // was banded.
            let shade = banded_shade(world, organism_id, cell.material, Band::Bark, rng);
            let new_cell = Cell::new(cell.material, shade).with_organism_id(organism_id).with_aux(organism::pack_cell_type(CellType::MatureBody));
            world.set(nx, ny, new_cell);
            // Wood laid beside a trunk cell is that trunk, so it inherits
            // the order rather than starting a tier. Nothing reads order
            // off a `MatureBody` today, but leaving it 0 would quietly make
            // every thickened limb read as trunk the moment something does.
            let order = world.organism_cell(x, y).map_or(0, |c| c.order);
            write_order(world, nx, ny, order);
            // Secondary thickening lays wood *beside* an existing cell, not
            // beyond it, so the new cell is the same distance from the
            // collar rather than one step further -- it inherits, it does
            // not increment. Getting this wrong would make a fat trunk read
            // as hydraulically remote and throttle its own growth.
            let parent_path = path_len_at(world, x, y);
            if let Some(slot) = world.organism_cell_mut(nx, ny) {
                slot.path_len = parent_path;
            }
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
/// `Creature`, `Decay`, `Evaporate` and `Dissipate`, which `scheduler::step`
/// routes to `structural::tick`/`creature::tick`/`decay::tick`/
/// `evaporation::tick`/`update::dissipation_tick` instead -- the match here
/// still has to name all five variants to stay exhaustive.
pub fn tick(world: &mut World, site: &ActiveSite) -> Vec<ActiveSite> {
    match site.kind {
        ActiveKind::Organism { organism, stale_ticks, plastochron } => organism_tick(world, site.x, site.y, organism, stale_ticks, plastochron),
        ActiveKind::StructuralCheck => unreachable!("scheduler::step routes StructuralCheck to structural::tick"),
        ActiveKind::Creature { .. } => unreachable!("scheduler::step routes Creature to creature::tick"),
        ActiveKind::Decay => unreachable!("scheduler::step routes Decay to decay::tick"),
        ActiveKind::Evaporate { .. } => unreachable!("scheduler::step routes Evaporate to evaporation::tick"),
        ActiveKind::Dissipate => unreachable!("scheduler::step routes Dissipate to update::dissipation_tick"),
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
        // Moss skips the `Seed` stage entirely, so it has no germination to
        // draw at -- this is the equivalent moment. Inert while moss's own
        // `genotype_variance` is all zeroes, and correct the moment a
        // species with a `Divide` economy wants individuality.
        seed_genotype(self, organism_id, x, y);
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

    /// **What a tree lets go of has to land somewhere an animal can find it.**
    ///
    /// Both abscission paths wrote `Cell::EMPTY`, so a forest could double
    /// its foliage over a run while the food an ant could reach *fell*:
    /// `creature_space`'s census (CENSUS=1) measured the whole colony band
    /// ending at 240..480 energy across 8 seeds, with three of four presets
    /// having seeds that finish at literally zero, against fifty-two ants
    /// each needing 900 to live.
    ///
    /// Driven through `shed_stranded_leaves` rather than through a grown
    /// tree, for the reason the eat tests record: whether a *particular*
    /// canopy happens to strand a spray in N frames is a different question,
    /// and mixing it in makes this a test of tree geometry.
    #[test]
    fn a_shed_leaf_becomes_litter_rather_than_nothing() {
        let mut w = test_world();
        let leaf = w.materials.id_of("leaf").expect("leaf");
        let litter = w.materials.id_of("litter").expect("litter");
        let organism = 7u16;

        // A spray of leaves attached to nothing: no wood, no tip, no bud, so
        // `shed_stranded_leaves` sees an unanchored component and drops it.
        // A floor for it to land on. Without one the litter has nowhere to
        // come to rest and is simply dropped -- correct behaviour over a pit,
        // and a scene that would test nothing.
        for x in 40..70 {
            w.set(x, 60, Cell::new(material::STONE, 0));
        }
        let spray = [(50, 50), (51, 50), (52, 50)];
        for &pos in &spray {
            place(&mut w, pos, leaf, organism, CellType::Leaf, (0.0, 0.0));
        }
        assert_eq!(
            spray.iter().filter(|&&(x, y)| w.get(x, y).material == leaf).count(),
            spray.len(),
            "test setup: the spray was not placed, so this would pass on an empty scene"
        );

        shed_stranded_leaves(&mut w, 49, 50, organism);

        // **Counted where it lands, not where it hung**, which is the whole
        // of the change this assertion had to be rewritten for: a leaf falls
        // through its own crown and comes to rest on the floor, so the cells
        // it used to occupy are empty and the litter is on the ground below.
        // The earlier version looked only at the spray's own positions and
        // went red for the fix rather than for a fault.
        for &(x, y) in &spray {
            assert_eq!(w.get(x, y).material, material::EMPTY, "the leaf itself is gone from where it hung");
        }
        let landed: Vec<(i32, i32)> = (45..60)
            .flat_map(|x| (40..100).map(move |y| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == litter)
            .collect();
        assert_eq!(landed.len(), spray.len(), "a shed spray must become litter somewhere, not vanish -- {} of {} did", landed.len(), spray.len());
        for &(x, y) in &landed {
            assert_eq!(w.get(x, y).organism_id(), 0, "litter belongs to nobody; leaving the tree's id on it makes a falling powder part of an organism");
            assert!(y > 50, "litter must end up *below* the leaf it fell from, and this landed at {y} against a shed height of 50");
        }
    }

    /// **Shedding must not schedule a structural check**, and this is the one
    /// guard standing between S4 and a measured 26x collapse.
    ///
    /// `CLAUDE.md` records it: the organism support search is hop-bounded, so
    /// a check fired high in a crown reads everything past the span limit as
    /// unsupported and converts it to deadwood. Growth deliberately schedules
    /// none; an earlier abscission that scheduled one destroyed every shedding
    /// sweep at every setting, and it read as "the mechanism is wrong" through
    /// eight settings before anyone found the one line.
    ///
    /// S4 makes shedding *write a cell* where it used to erase one, which is
    /// exactly the kind of change that invites a helpful "shouldn't we tell
    /// the structure about this?" -- so the prohibition is asserted rather
    /// than left as a comment.
    ///
    /// **Why the bar is now zero sites rather than one per leaf.** Shedding
    /// used to schedule the decay site itself, at the cell it wrote, so this
    /// asserted `added == spray.len()`. Decay sites are scheduled by
    /// `World::end_step`'s awake->settled chunk scan instead, which does not
    /// run in a unit test -- so shedding correctly adds nothing here.
    ///
    /// A bare `added == 0` would be worse than useless: it cannot tell "no
    /// structural check was scheduled" from "`shed_stranded_leaves` never ran
    /// at all", which is exactly how a guard goes vacuous. So the zero is
    /// paired with a positive check that the spray really did become litter.
    #[test]
    fn shedding_a_leaf_schedules_decay_and_never_a_structural_check() {
        let mut w = test_world();
        let leaf = w.materials.id_of("leaf").expect("leaf");
        let organism = 9u16;
        for x in 40..70 {
            w.set(x, 60, Cell::new(material::STONE, 0));
        }
        let spray = [(50, 50), (51, 50)];
        for &pos in &spray {
            place(&mut w, pos, leaf, organism, CellType::Leaf, (0.0, 0.0));
        }
        let before = w.active_site_count();

        shed_stranded_leaves(&mut w, 49, 50, organism);

        // **The mechanism actually ran** -- without this the zero below passes
        // for a `shed_stranded_leaves` that did nothing at all.
        let litter = w.materials.id_of("litter").expect("litter is a compiled-in material");
        let landed = (40..70)
            .flat_map(|x| (40..60).map(move |y| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == litter)
            .count();
        assert_eq!(landed, spray.len(), "the spray had to become {} litter cells for this test to be asking anything; {landed} did", spray.len());

        // Nothing scheduled. A structural check fans out over a
        // neighbourhood, so it would land here as a count well above the
        // number of cells shed; the decay sites come from the settle scan,
        // which a unit test never drives.
        let added = w.active_site_count() - before;
        assert_eq!(
            added, 0,
            "shedding {} leaves added {added} active sites; it should add none. Anything here is a structural check fanning out, which amputates a crown -- see the doc on this test",
            spray.len()
        );
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
    /// Plant a seed on ground a tree can actually live on: a bed of damp
    /// soil at field capacity over a stone floor.
    ///
    /// **It used to be a bare stone shelf, and that stopped being a viable
    /// scene when water became a currency.** With transpirational demand
    /// charged and `Absorb` crediting water, a plant with no soil or free
    /// water to reach meets none of its demand, so its stomatal term is 0,
    /// so it earns no carbon and never grows -- which is exactly what the
    /// economy is *for*, and it turned seven tree tests red at once.
    ///
    /// The scene was contradicting the code (`CLAUDE.md`), and the fix
    /// belongs in the scene. A tree on bare rock with nothing to drink
    /// should fail; none of these tests is about that, and the ones that
    /// are (`a_root_penetrates_soil_but_not_stone`) build their own ground.
    ///
    /// `SOIL_FIELD_CAPACITY`, not saturated: `Powder` `aux == 0` means dry,
    /// the opposite of a `Liquid`, and starting at capacity matches
    /// `examples/common/mod.rs`'s shared plant scene so a test and a
    /// filmstrip are looking at the same world.
    fn plant_tree_on_ground(w: &mut World, x: i32, y: i32) {
        plant_tree_on_ground_with_moisture(w, x, y, material::SOIL_FIELD_CAPACITY);
    }

    /// `plant_tree_on_ground`, with the bed's soil moisture as a parameter
    /// so a paired water comparison is one argument rather than a second
    /// scene. The same call the repo already made for `soil_depth`, and for
    /// the same reason: a comparison that cannot be expressed cannot be run.
    fn plant_tree_on_ground_with_moisture(w: &mut World, x: i32, y: i32, moisture: u16) {
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        const SOIL_ROWS: i32 = 8;
        const HALF: i32 = 8;
        // **Walled, not just floored.** Soil is a `Powder`: an open-sided
        // bed avalanches off its own floor over the first few hundred
        // frames, the surface drops, and the seed rides down with it --
        // which read as "the seed never germinated" and is the second time
        // this exact scene error has cost time here (`CLAUDE.md`: a soil
        // column with no floor or walls fell out of the world and toppled).
        for fx in (x - HALF - 1)..=(x + HALF + 1) {
            w.set(fx, y + SOIL_ROWS + 1, Cell::new(material::STONE, 0));
        }
        for dy in 1..=SOIL_ROWS {
            w.set(x - HALF - 1, y + dy, Cell::new(material::STONE, 0));
            w.set(x + HALF + 1, y + dy, Cell::new(material::STONE, 0));
            for fx in (x - HALF)..=(x + HALF) {
                w.set(fx, y + dy, Cell::new(soil, 0).with_aux(moisture));
            }
        }
        w.plant_tree(x, y);
    }

    /// The pre-soil scene: a bare stone shelf and nothing to drink.
    ///
    /// Kept for the tests whose subject is free *water* rather than soil
    /// moisture -- a soil bed would supply the plant before its root ever
    /// reached the puddle, which is the confound rather than the mechanism.
    fn plant_tree_on_bare_ground(w: &mut World, x: i32, y: i32) {
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
        // Two separate stone platforms, identical except one has water in
        // it. Both get a moss seed on their own surface.
        //
        // **The water sits in a sealed pocket one row *under* the platform,
        // where it used to be a film resting on top of it. Water evaporates
        // now** (`evaporation.rs`), and the old scene's 6,000 fill of film
        // was gone long before this 4,000-frame run finished: the damp side
        // ended with a single moss cell. Confirmed by control — with
        // `water.evaporates` off, the old scene passes unchanged.
        //
        // Deepening or widening the puddle does not fix it, and the reason
        // is worth writing down because it constrains any future edit here.
        // The field resolves one cell per 8x8 block, and a block is damp
        // only if water is *inside it* — a blocked block that holds no
        // water stays at ambient however wet its neighbour is
        // (`field::step_diffusion` skips blocked cells outright). So moss
        // reads as damp only where its own block contains the water, which
        // is why this scene has always needed water spread along the whole
        // platform rather than pooled at one end. A pool in a trough beside
        // the platform leaves everything past the next block boundary dry
        // (measured: 4.000 over the pool, 0.000 two blocks away), and a
        // wide shallow pool on top evaporates (13 cells across shelters
        // only 3/7 of `evaporation::SHELTER_REACH`, and it lost 79% over
        // this run).
        //
        // Water under the rock has neither problem. It is sealed, so
        // `evaporation::is_exposed_surface` is false and it never
        // evaporates — it retires off the schedule instead, which is the
        // right answer and one this scene now exercises incidentally. Rows
        // 48..55 are one field block, so the water at y=51, the platform at
        // y=50 and the moss growing at y=49 all share it and the whole
        // platform reads damp. And seepage under rock is a better story for
        // damp stone than a puddle sitting on it ever was.
        for x in 9..31 {
            w.set(x, 50, Cell::new(material::STONE, 0)); // platform
            w.set(x, 52, Cell::new(material::STONE, 0)); // pocket floor
        }
        w.set(9, 51, Cell::new(material::STONE, 0)); // pocket walls
        w.set(30, 51, Cell::new(material::STONE, 0));
        for x in 10..30 {
            w.set(x, 51, Cell::new(material::WATER, 0));
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
        // **The scene is aligned so the upward step under test is a *damp*
        // step.** `FIELD_SCALE` is 8, so field blocks span rows 40..47 and
        // 48..55; the puddle at row 50 keeps the whole 48..55 block damp,
        // and the seed at 49 grows its first upward cell into row 48 --
        // same damp block, `damp_chance` territory. Two earlier layouts sat
        // one row higher, which put the first upward step across the block
        // boundary into dry readings, so the mechanism this test names
        // (growing over one's own cells) was gated behind a `dry_chance`
        // x `shade_factor` lottery at ~2e-4 per check. That made the test a
        // coin flip twice: once when an unrelated change shifted the RNG
        // stream, and again when phase-free light removed the nightly
        // darkness boost the 24,000-frame version had silently relied on
        // (measured: topmost row 47 of a 40..48 assertion, 31 cells --
        // passing on the boundary). The lottery was never the point; the
        // traversal was. Measured on this layout: topmost row 48 with 25
        // cells at 6,000 frames, via the damp path -- a rate, not a roll.
        let mut w = test_world();
        for x in 5..35 {
            w.set(x, 51, Cell::new(material::STONE, 0));
        }
        w.set(5, 50, Cell::new(material::STONE, 0)); // walls -- keep the
        w.set(34, 50, Cell::new(material::STONE, 0)); // puddle from draining off the sides
        for x in 10..30 {
            w.set(x, 50, Cell::new(material::WATER, 0));
        }
        w.plant_moss_seed(20, 49);
        run_with_fields(&mut w, 6000);

        let moss = w.materials.id_of("moss").unwrap();
        // The seed sits at row 48; nothing above row 48 has a stone
        // neighbour at all (stone is only at row 50, water fills row 49) --
        // the *only* way for moss to ever reach row 47 or higher is by
        // growing over another moss cell of its own organism. Any moss
        // found there is proof the patch thickened, not just spread
        // sideways hugging the water's own row.
        // The seed sits at row 49; nothing above row 49 has a stone
        // neighbour at all (stone is only at 51 and the wall tops at 50,
        // water fills 50) -- the *only* way moss reaches row 48 or higher
        // is by growing over another cell of its own organism. Any moss
        // there is proof the patch thickened rather than only hugging the
        // water's own row.
        let thickened = (5..35).any(|x| (42..49).any(|y| w.get(x, y).material == moss));
        assert!(thickened, "moss never thickened into a 2D patch, only ever grew along the original rock");
    }

    /// **A species declares what it is made of, and an unknown name falls
    /// back exactly where the hardcoded lookup did.**
    ///
    /// The three seeding sites (`germinate`'s shoot and companion root,
    /// the `Grow` arm's leaf cluster) used to name `wood`/`rootwood`/`leaf`
    /// in code, so every `Grow` species was brown stem and green leaf by
    /// construction whatever its numbers said. They now read species data
    /// (`Reports/plant-evolution-design.md` §3c). Propagation is untouched:
    /// growth still copies a parent's material to its child, so seeding one
    /// cell is what makes a whole root system rootwood.
    ///
    /// Both directions are asserted on purpose. The fallback alone would
    /// pass against code that ignored the field entirely — `id_of("wood")`
    /// resolves, so an ignored field yields wood rather than the seed's own
    /// material — but only the *positive* case proves the declared name is
    /// what is actually read.
    ///
    /// Built by copying the shipped `tree.ron` and renaming it, rather than
    /// hand-writing a minimal species: `Grow` has a wide required-field
    /// surface, and a test species that drifts from the real one tests a
    /// shape nothing ships.
    #[test]
    fn a_species_declares_its_materials_and_an_unknown_name_falls_back() {
        let base = include_str!("../../assets/species/tree.ron");
        assert_eq!(base.matches("name: \"tree\",").count(), 1, "the rename anchor must be unique or this test edits more than its target");

        let variant = |species: &str, extra: &str| -> String {
            base.replacen("name: \"tree\",", &format!("name: \"{species}\",{extra}"), 1)
        };

        let dir = std::env::temp_dir().join("pixel-physics-species-materials");
        std::fs::create_dir_all(&dir).unwrap();
        // Declares a material that does not exist -> must fall back to the
        // germinating seed's own material, which is what the hardcoded
        // lookup did for a stripped asset set.
        std::fs::write(dir.join("unknownmat.ron"), variant("unknownmat", " shoot_material: \"nosuchmaterial\",")).unwrap();
        // Declares a real material that is NOT the default -> must be used.
        std::fs::write(dir.join("leafstem.ron"), variant("leafstem", " shoot_material: \"leaf\",")).unwrap();
        // Declares nothing -> the defaults, i.e. today's tree exactly.
        std::fs::write(dir.join("plainmat.ron"), variant("plainmat", "")).unwrap();

        let seed_material_of = |species: &str| -> (material::MaterialId, material::MaterialId) {
            let mut w = test_world();
            w.species.reload(&dir).unwrap();
            let soil = w.materials.id_of("soil").expect("soil is compiled in");
            for fx in 40..=60 {
                w.set(fx, 29, Cell::new(material::STONE, 0));
                for dy in 21..29 {
                    w.set(fx, dy, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
                }
            }
            assert!(w.plant_tree_species(50, 20, species), "test setup: {species} should plant");
            let seed = w.get(50, 20).material;
            for _ in 0..2_000 {
                run_with_fields(&mut w, 1);
                if organism::cell_type(w.get(50, 20).aux()) == Some(CellType::GrowingTip) {
                    return (seed, w.get(50, 20).material);
                }
            }
            panic!("test setup: {species} never germinated");
        };

        let wood = {
            let w = test_world();
            w.materials.id_of("wood").expect("wood is compiled in")
        };
        let leaf = {
            let w = test_world();
            w.materials.id_of("leaf").expect("leaf is compiled in")
        };

        let (seed_mat, shoot) = seed_material_of("unknownmat");
        assert_eq!(shoot, seed_mat, "an unknown shoot material must fall back to the germinating cell's own material, as the hardcoded lookup did");
        assert_ne!(shoot, wood, "falling back to wood would mean the declared name was never read at all");

        let (_, shoot) = seed_material_of("leafstem");
        assert_eq!(shoot, leaf, "a species declaring a real material must be built from it");

        let (_, shoot) = seed_material_of("plainmat");
        assert_eq!(shoot, wood, "a species declaring nothing must get the defaults -- every shipped .ron depends on it");

        std::fs::remove_dir_all(&dir).ok();
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

    /// The genotype belongs to the *plant*, not to the world's event
    /// history — which is exactly what keying on `organism_id` could not
    /// give, since ids are handed out in planting order.
    ///
    /// The middle case is the one that fails against id keying: planting
    /// three saplings elsewhere *first* shifts the tree at (100, 60) from
    /// organism 1 to organism 4, so an id-keyed draw makes it a different
    /// individual for a reason that has nothing to do with it.
    /// Scratch: prints the single-tree growth curve `a_tree_eventually_
    /// stops_growing` asserts a plateau on. `#[ignore]`d, kept because that
    /// test's bar is a claim about a curve and the curve is what says
    /// whether the bar is set right.
    ///
    /// ```text
    /// cargo test --release --lib print_single_tree_growth_curve -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn print_single_tree_growth_curve() {
        let mut w = test_world();
        plant_tree_on_ground(&mut w, 50, 20);
        let wood = w.materials.id_of("wood").unwrap();
        let mut last = 0;
        for step in 1..=12 {
            run_with_fields(&mut w, 5000);
            let n = count(&w, wood);
            println!("frame {:>6}: {:>5} wood (+{})", step * 5000, n, n - last);
            last = n;
        }
    }

    #[test]
    fn genotype_follows_position_and_world_seed_not_planting_order() {
        fn draws_at_100(planted_first: &[i32], seed: u64) -> [f32; organism::GENOTYPE_TRAITS] {
            let mut w = test_world();
            w.seed = seed;
            for &x in planted_first {
                plant_tree_on_ground(&mut w, x, 60);
            }
            plant_tree_on_ground(&mut w, 100, 60);
            run_with_fields(&mut w, 400);
            let organism_id = w.get(100, 60).organism_id();
            assert_ne!(organism_id, 0, "the tree at (100, 60) should still own its own base cell");
            assert_ne!(
                organism::cell_type(w.get(100, 60).aux()),
                Some(CellType::Seed),
                "test setup needs the seed to have germinated -- nothing draws a genotype before that"
            );
            w.organism(organism_id).expect("live organism").genotype_draws
        }

        let alone = draws_at_100(&[], 12345);
        let crowded = draws_at_100(&[40, 60, 80], 12345);
        assert_eq!(alone, crowded, "planting order must not change who a plant is");
        assert!(alone.iter().any(|&d| d != 0.0), "a germinated plant should have drawn a real genotype, not the species mean");

        let other_world = draws_at_100(&[], 999);
        assert_ne!(alone, other_world, "a different world seed should grow a different individual at the same spot");
    }

    /// **Crowding must reorder choices, never veto all of them.** Under
    /// the subtractive score this tip had no candidates at all — every
    /// direction's score went negative against the ring's density, the tip
    /// banked stale ticks, and four of those is permanent retirement. That
    /// is the measured collapse cliff (median tree 26 cells at
    /// `crowding_weight: 20`), reproduced here at unit scale: this test
    /// fails on the subtractive form and must keep failing for any future
    /// form that lets crowding empty the candidate set.
    #[test]
    fn a_crowded_tip_takes_its_least_bad_direction_instead_of_dying() {
        let mut w = test_world();
        let wood = w.materials.id_of("wood").unwrap();
        let tree = w.species.id_of("tree").expect("tree species is compiled in");
        let organism_id = w.push_organism(tree);

        // A tip walled in by its own maximum-density tissue on seven of
        // eight sides. The one opening's own neighbourhood still reads the
        // ring's density, so under tree.ron's `crowding_weight: 12` the
        // subtractive score was negative in every direction.
        place(&mut w, (100, 100), wood, organism_id, CellType::GrowingTip, (2.0, 0.0));
        for (dx, dy) in NEIGHBOURS_8 {
            if (dx, dy) == (1, -1) {
                continue; // the one way out
            }
            place(&mut w, (100 + dx, 100 + dy), wood, organism_id, CellType::MatureBody, (0.0, organism::CANOPY_DENSITY_SCALE));
        }

        organism_tick(&mut w, 100, 100, organism_id, 0, 0);

        let grew = w.get(101, 99).organism_id() == organism_id;
        assert!(grew, "a tip with one open direction should grow into it however crowded it is, not stall toward retirement");
    }

    #[test]
    fn a_flushing_bud_keeps_its_standing_carbon_and_the_richest_cell_pays() {
        // The exact leak the second review found: `break_buds` used to
        // *assign* `bud_cost` onto the flushing bud, silently destroying
        // whatever the bud already held (up to ~3.8 at the cap, every
        // flush) — and the behavior doc claimed the bud itself paid. The
        // fixed contract, asserted here: the richest cell pays the price,
        // and the bud keeps its own stake.
        let mut w = test_world();
        let wood = w.materials.id_of("wood").unwrap();
        let tree = w.species.id_of("tree").expect("tree species is compiled in");
        let organism_id = w.push_organism(tree);

        // A trunk cell at the cap (the richest), a bud holding real
        // carbon, and enough sunlit foliage that the whole-plant gate
        // supports one more tip than the zero that exist.
        //
        // **The foliage count is derived from `leaf_cluster`, not written
        // down.** `supportable` is
        // ⌊intercepted / L_node · INCOME_PER_NODE / cost⌋ and
        // `L_node = MAX_LIGHT × leaf_cluster`, so the gate is denominated
        // in *nodes*: three fully-lit nodes give 3 × 0.08/0.2 = 1.2 and
        // clear the one-tip bar with margin, whatever a node is made of.
        //
        // It used to say fifteen, which was three nodes at `leaf_cluster:
        // 5` — and raising the cluster to 10 for foliage volume
        // (`Reports/plant-appearance-design.md` §5a) made the same fifteen
        // cells one and a half nodes, so the bud stopped flushing and this
        // test failed on a change that had nothing to do with what it
        // asserts. That is `CLAUDE.md`'s "when a fix changes what a number
        // *means*, re-deriving the constants that read it is part of the
        // fix": the honest repair is to state the scenario in the unit the
        // gate actually uses, so the next cluster change cannot break it.
        let leaf_cluster = match w.species.get(tree).behaviors(CellType::GrowingTip).iter().find(|b| matches!(b, Behavior::Grow { .. })) {
            Some(Behavior::Grow { leaf_cluster, .. }) => *leaf_cluster as i32,
            _ => panic!("tree's GrowingTip has a Grow behavior"),
        };
        place(&mut w, (100, 20), wood, organism_id, CellType::MatureBody, (4.0, 0.0));
        place(&mut w, (101, 20), wood, organism_id, CellType::DormantBud, (3.0, 0.0));
        for i in 0..(3 * leaf_cluster) {
            place(&mut w, (60 + i, 18), wood, organism_id, CellType::Leaf, (0.0, 0.0));
        }
        for _ in 0..30 {
            field::step(&mut w); // converge the sky so the leaves read real light
        }

        break_buds(&mut w, organism_id);

        assert_eq!(
            organism::cell_type(w.get(101, 20).aux()),
            Some(CellType::GrowingTip),
            "test setup should have let the bud flush at all"
        );
        assert!(
            (w.carbon_at(100, 20) - 3.8).abs() < 1e-3,
            "the richest cell should have paid the 0.2 flush price: {}",
            w.carbon_at(100, 20)
        );
        assert!(
            (w.carbon_at(101, 20) - 3.0).abs() < 1e-3,
            "the flushing bud's standing carbon must survive the flush, not be overwritten by the price: {}",
            w.carbon_at(101, 20)
        );
    }

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

    /// **Does a sod mat actually hold a bank?** WP-B3's acceptance item 3,
    /// and the only one of grass's four axes (`plant-evolution-design.md`
    /// §4a) that is a *consequence* rather than a look.
    ///
    /// **Paired, because outcomes here have enormous spread** and a single
    /// arm against a remembered number is a sample from a wide
    /// distribution. Both arms are the same bank, the same walls, the same
    /// disturbance and the same frame budget; the only difference is
    /// whether grass grew on it first.
    ///
    /// The scene is built so the mechanism under test can actually fire.
    /// A soil block resting on a floor does not move at all -- powders need
    /// somewhere to go -- so the bank sits on a *ledge* with open air off
    /// both ends, and temporary stone walls hold the faces while the grass
    /// establishes. **Removing those walls is the disturbance**, applied
    /// identically to both arms. Without them the bare arm would spill
    /// during the growth phase and the two arms would differ in when they
    /// were disturbed as well as in whether they had roots.
    ///
    /// What it counts is soil that ended up *below the ledge* -- material
    /// that left the bank entirely. That is a standing state, not an event
    /// rate: it cannot be inflated by grains that shuffled and came back.
    ///
    /// **Measured, both arms, same session:**
    ///
    /// ```text
    /// shed  bare 327  sod 305   (-7%)     soil that left the bank entirely
    /// crest bare 185  sod 235   (+27%)    bank surface still standing
    /// grassroot cells in the bank: 135
    /// ```
    ///
    /// Those two say different things and both are true. Total spill barely
    /// moves, because roots thread the *top* of a bank and the unrooted
    /// bulk below dominates a whole-bank count -- which is also what real
    /// sod does. Where the roots actually reach, a quarter more of the
    /// surface survives.
    ///
    /// **The crest counter measured backwards on its first run**, and it is
    /// worth keeping why: it counted *soil*, and grass converts soil cells
    /// into `grassroot`, so the rooted arm scored 141 against 185 and read
    /// as shedding harder when 135 of the gap was root tissue standing
    /// where soil had been. Counting occupancy fixed it. A metric that
    /// penalises the mechanism for having happened is this repo's own "ask
    /// what a metric counts when nothing is wrong", in a new costume.
    ///
    /// The root counter is not decoration either: without it a run where
    /// the grass failed to establish would report a plausible small margin
    /// and mean nothing at all.
    #[test]
    fn a_rooted_bank_sheds_less_soil_than_a_bare_one() {
        const LEDGE_L: i32 = 60;
        const LEDGE_R: i32 = 140;
        const BANK_TOP: i32 = 134;
        const LEDGE_Y: i32 = 150;

        fn build(with_grass: bool) -> (usize, usize, usize) {
            let mut w = test_world();
            let soil = w.materials.id_of("soil").expect("soil is compiled in");
            // The ledge, and a catch floor far below it.
            for x in LEDGE_L..=LEDGE_R {
                for y in LEDGE_Y..=LEDGE_Y + 4 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            for x in 0..200 {
                w.set(x, 190, Cell::new(material::STONE, 0));
            }
            for x in LEDGE_L..=LEDGE_R {
                for y in BANK_TOP..LEDGE_Y {
                    w.set(x, y, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
                }
            }
            // Temporary faces, removed below as the disturbance.
            for y in BANK_TOP..LEDGE_Y {
                w.set(LEDGE_L - 1, y, Cell::new(material::STONE, 0));
                w.set(LEDGE_R + 1, y, Cell::new(material::STONE, 0));
            }
            if with_grass {
                for i in 0..16 {
                    let x = LEDGE_L + 3 + i * 5;
                    w.plant_tree_species(x, BANK_TOP - 2, "grass");
                }
            }
            // Establish. Both arms run this, so the bare arm pays exactly
            // the same settling time.
            run_with_fields(&mut w, 14_000);

            // THE DISTURBANCE: the faces go.
            for y in BANK_TOP..LEDGE_Y {
                w.set(LEDGE_L - 1, y, Cell::EMPTY);
                w.set(LEDGE_R + 1, y, Cell::EMPTY);
            }
            run_with_fields(&mut w, 6_000);

            // Three numbers, because the first one alone was misleading.
            //
            // `shed` is the plan's ask -- soil that left the bank entirely.
            // `crest` is soil still standing in the top four rows of the
            // original footprint, which is what a sod mat can actually be
            // expected to hold: roots thread the top of a bank, not its
            // depth, so a total-spill count is dominated by the unrooted
            // bulk underneath and under-reports the mechanism.
            // `roots` is the "did it fire at all" counter -- a run where
            // the grass never rooted into the bank would give a plausible
            // near-zero margin that means nothing.
            let b = w.bounds().unwrap();
            let grassroot = w.materials.id_of("grassroot");
            let (mut shed, mut crest, mut roots) = (0, 0, 0);
            for y in b.min_y..=b.max_y {
                for x in b.min_x..=b.max_x {
                    let m = w.get(x, y).material;
                    if m == soil && y > LEDGE_Y + 4 {
                        shed += 1;
                    }
                    // **Occupancy, not soil.** Counting soil here was wrong
                    // and measured backwards: grass *converts* soil cells
                    // into `grassroot`, so the rooted arm scored 141
                    // against the bare arm's 185 and looked like it was
                    // shedding harder, when 135 of the difference was
                    // simply root tissue standing where soil had been. The
                    // question is "is the bank surface still there", and a
                    // sod mat holding a bank is partly made of root.
                    if m != material::EMPTY && (BANK_TOP..BANK_TOP + 4).contains(&y) && (LEDGE_L..=LEDGE_R).contains(&x) {
                        crest += 1;
                    }
                    if Some(m) == grassroot {
                        roots += 1;
                    }
                }
            }
            (shed, crest, roots)
        }

        let (bare_shed, bare_crest, bare_roots) = build(false);
        let (sod_shed, sod_crest, sod_roots) = build(true);
        // Printed, not only asserted: the acceptance asks for the margin,
        // and a passing assertion reports nothing. `--nocapture` shows it.
        println!("shed  bare {bare_shed}  sod {sod_shed}   ({:+.0}%)", 100.0 * (sod_shed as f32 / bare_shed as f32 - 1.0));
        println!("crest bare {bare_crest}  sod {sod_crest}   ({:+.0}%)", 100.0 * (sod_crest as f32 / bare_crest as f32 - 1.0));
        println!("grassroot cells in bank: bare {bare_roots}, sod {sod_roots}");

        assert_eq!(bare_roots, 0, "the bare arm somehow grew roots -- the arms are not what they claim to be");
        assert!(
            sod_roots > 0,
            "the sod arm has no grassroot cells in the bank, so nothing was reinforced and any margin here is noise, not mechanism"
        );
        assert!(
            bare_shed > 0,
            "the bare arm shed no soil at all, so the disturbance did nothing and the comparison is vacuous"
        );
        // Measured +27% (185 -> 235); the bar sits at +10%, with headroom,
        // rather than on the measured value or at a bare `>` that would
        // flake on any run that landed slightly flat.
        assert!(
            sod_crest as f32 > bare_crest as f32 * 1.10,
            "a rooted bank kept {sod_crest} crest cells against a bare bank's {bare_crest} --              reinforces_powder is buying nothing where roots actually reach"
        );
    }

    // **`a_tree_eventually_stops_growing` was retired here, 2026-08-22, and
    // no replacement is shipped -- deliberately.**
    //
    // It asserted that a tree exhausts its economy and plateaus. Once seeds
    // began waiting for water that stopped holding: the subject reached
    // 1,718 cells and was still climbing at 120,000 frames against a
    // recorded plateau of ~565. Isolated by control -- neutralising the
    // germination gate alone restored it.
    //
    // The cause was never the tree's economy. A mature tree draws the soil
    // around it toward the wilting point, so its own seedlings can no longer
    // clear their germination threshold; they sit dormant instead of
    // becoming competitors, and the uncontested parent keeps growing. **The
    // test was measuring crowding and calling it economy.** The owner's call
    // was to accept that: a solitary well-watered tree growing without bound
    // is correct, and a mature tree suppressing its own seedlings by drying
    // the ground is what a real stand does.
    //
    // **Two replacements were written and both had their premise falsified
    // by the first run**, which is why there is a comment here instead of a
    // test. "A tree grows less on less water" is false as stated: measured
    // over 12,000 frames on the same bed, the thirsty arm grew **982 cells
    // against the watered arm's 734**, and **428 wood cells against 299**.
    //
    // **That is backwards from real plants, and it is logged as a defect
    // rather than explained away** (`Reports/open-bugs-handoff.md` U).
    // Drought reduces total biomass and reduces secondary growth in
    // particular -- narrow rings in dry years is the whole basis of
    // dendrochronology. What actually rises under drought is the
    // root:shoot *ratio*, and it rises because shoots suffer more, not
    // because roots gain. Here every quantity goes up when water is short.
    //
    // The likely mechanism, unproven: `break_root_tips` is gated on
    // `water_status < 0.95`, so water stress *triggers* root re-initiation
    // -- but the stress does not appear to throttle the carbon that pays
    // for it, so scarcity buys extra tissue at no cost. A compensation
    // response with the penalty missing.
    //
    // So the honest state is: **growth here is not monotone in water**, and
    // any future guard has to say which quantity it means -- shoot mass,
    // total mass, or time-to-plateau -- and be measured before it is
    // asserted. `plant_tree_on_ground_with_moisture` below exists so that
    // comparison is one argument away when someone has a premise worth
    // testing.



    #[test]
    fn roots_consume_adjacent_water() {
        let mut w = test_world();
        // Seed near y=20, not the old system's y=100 -- see
        // `two_trees_grown_from_the_same_seed_differ`'s own doc on why
        // `Germinate`'s real light gate needs to be within a few field
        // rows of open sky. The water tank keeps the same offsets *below*
        // the seed as the old test used (floor 10 rows down, water
        // starting 2 rows down), just shifted to match.
        // Bare ground on purpose: with a soil bed the root's demand is met
        // before it ever reaches the tank, and the test would pass or fail
        // on soil moisture rather than on the free-water path it names.
        plant_tree_on_bare_ground(&mut w, 50, 20);
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
    // -- a fully dead organism's storage never being freed -- was recorded
    // here as a known gap for as long as `free_organism_slots` was popped
    // but never pushed.
    //
    // **That gap is closed now, from both ends, and the two halves landed
    // on separate branches.** `World::free_organism` returns the slot
    // (creatures needed it first: a laying queen allocates on her own
    // schedule and exhausts the 12-bit index in one long session), and
    // `step_organisms` above calls it for any organism whose cell list has
    // gone empty. That emptiness test is deliberately not the BFS-from-
    // roots liveness check this note used to ask for -- it is the weaker
    // definition that cannot orphan a standing cell, since a cell still
    // referring to the organism is exactly what keeps the list non-empty.
    // A standing dead trunk still holds its slot, on purpose, and belongs
    // to whoever decides what dead wood should *be*.
    // See `a_dead_plants_slot_is_reused_and_its_old_id_stays_dead`.

    // --- A plant reads real light at its own position --------------------
    //
    // **These guarded a workaround, and now guard the invariant that
    // replaced it.** `rebuild_blocked` used to mark a whole field block
    // opaque the instant any `Solid`/`Plant` cell sat inside it, and
    // `apply_sky` skipped opaque blocks, so a plant cell reading `field_at`
    // at its own exact position read a self-blocked `0.0` forever, however
    // bright the sky was one cell away. `ambient_light_above` dodged that
    // by sampling one field block up.
    //
    // `apply_sky` writes the light *arriving at* a block now, occupied or
    // not — a leaf is the thing intercepting, so its own reading is what
    // reaches it — and the offset went with the premise. What these tests
    // assert is unchanged and is the point either way: **a plant in open
    // sky earns real light.** They failed under the old rule without the
    // offset, and they pass under the new one without it, which is the
    // difference between a workaround and a fix.

    #[test]
    fn a_seed_germinates_in_open_sky_reading_light_at_its_own_position() {
        let mut w = test_world();
        // y=20 is arbitrary now and used to be load-bearing: while
        // `ambient_light_above` sampled a block *above* itself, a shallower
        // planting depth sent that read out of the world, which is opaque,
        // and the test would have failed for the geometry rather than for
        // the mechanism. The read is at the cell's own position now, so
        // there is no offset to keep in bounds.
        plant_tree_on_ground(&mut w, 100, 20);
        run_with_fields(&mut w, 400); // several germination checks (ORGANISM_TICK_INTERVAL apart)

        let cell_type = organism::cell_type(w.get(100, 20).aux());
        assert_ne!(cell_type, Some(CellType::Seed), "a seed in open sky should have germinated, not stayed a Seed forever");
    }

    #[test]
    fn photosynthesize_gains_resource_in_open_sky_reading_light_at_its_own_position() {
        let mut w = test_world();
        let tree_species = w.species.id_of("tree").expect("tree species must be loaded");
        let wood = w.materials.id_of("wood").unwrap();
        let organism_id = w.push_organism(tree_species);
        let aux = organism::pack_cell_type(CellType::GrowingTip);
        // **Standing on stone, which it did not need to be until
        // `anchor_support` landed.** This is a lone wood cell in open sky,
        // and the structural model has always said such a cell is
        // unsupported; nothing ever asked, because growth deliberately
        // schedules no structural checks and the old search only ran when
        // something else disturbed the cell. The anchor pass now evaluates
        // every organism cell once a tick, so the cell fell and the test
        // read `resource = 0` from an empty cell.
        //
        // The scene was contradicting the code (`CLAUDE.md`), and the fix
        // belongs in the scene: this test is about `Photosynthesize`
        // reading light at its own position, and a floor below it changes
        // nothing about the light arriving from above.
        // Damp soil under it, not bare stone. Two separate reasons, both
        // introduced since this test was written: `anchor_support` now
        // evaluates every organism cell, so a cell floating in open sky is
        // correctly read as detached and falls; and photosynthesis is
        // multiplied by the stomatal term, so a plant with nothing to drink
        // earns nothing however bright the sky is. Neither is what this
        // test is about -- it is about `Photosynthesize` reading light at
        // its own position -- and soil below changes nothing about the
        // light arriving from above.
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        let rootwood = w.materials.id_of("rootwood").expect("rootwood is compiled in");
        for fx in 98..=102 {
            for dy in 21..=24 {
                w.set(fx, dy, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
            }
            w.set(fx, 25, Cell::new(material::STONE, 0));
        }
        // **A minimal *plant*, not a lone shoot cell**, and both halves are
        // now required. `anchor_support` reads a cell floating in open sky
        // as detached and drops it, so the tip needs ground; and
        // photosynthesis is multiplied by the stomatal term, so the
        // organism needs something that can drink. `GrowingTip` carries
        // `Grow` and `Photosynthesize` and no `Absorb` -- a shoot with no
        // root earns nothing, which is the economy working, not a bug to
        // route around.
        //
        // The subject is unchanged: whether `Photosynthesize` credits from
        // the light at its own position.
        w.set(100, 21, Cell::new(rootwood, 0).with_organism_id(organism_id).with_aux(organism::pack_cell_type(CellType::RootTip)));
        w.set(100, 20, Cell::new(wood, 0).with_organism_id(organism_id).with_aux(aux));
        let root_site = reschedule_organism(100, 21, organism_id, 0, 0, w.frame + ORGANISM_TICK_INTERVAL);
        w.schedule_active_site(root_site);
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
                // **Planted individuals only.** Seeds are a `Powder` and can
                // come to rest *on a branch*, germinate there and grow from a
                // collar high in another plant's canopy -- so the world's
                // topmost organism cell stopped being a statement about the
                // turgor ceiling the moment reproduction landed. The claim is
                // that a plant cannot out-grow its own hydraulic budget, and
                // generation 0 is the set this scene planted at ground level.
                let top = (b.min_y..=b.max_y)
                    .find(|&y| {
                        (b.min_x..=b.max_x).any(|x| {
                            let id = w.get(x, y).organism_id();
                            id != 0 && w.organism_state(id).is_some_and(|s| s.generation == 0)
                        })
                    })
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
    /// The y=128 boundary would do the same and is no longer ruled out for
    /// the reason this comment used to give — that `LIGHT_DECAY` put
    /// `Germinate`'s threshold ~75 rows below open sky, so a seedling there
    /// was too dark to grow. `apply_sky`'s column cast retired that: open
    /// air reads full brightness at any depth. The seam under test is the
    /// same either way, so the geometry stays as measured rather than being
    /// moved for its own sake.
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
    /// connected piece.
    ///
    /// **Abscission no longer deletes**, since the ecology line landed:
    /// both real sites call `shed_to_litter`, which writes a falling
    /// `litter` powder rather than `Cell::EMPTY`. The connectivity question
    /// is unchanged, and emptying is if anything the stricter version of
    /// it — an emptied cell is the worst case a shed cell can leave behind.
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
            let (base, variance, juvenile_plastochron) = w
                .species
                .get(tree)
                .behaviors(CellType::GrowingTip)
                .iter()
                .find_map(|b| match b {
                    Behavior::Grow { plastochron, genotype_variance, juvenile_plastochron, .. } => {
                        Some((*plastochron, *genotype_variance, *juvenile_plastochron))
                    }
                    _ => None,
                })
                .expect("tree's GrowingTip grows");
            // A one-cell test plant is always below `juvenile_size`, so
            // the juvenile multiplier applies -- and hard-coding around
            // that is how this test broke when the stage landed.
            let jittered = (base.at(order) as f32 * genotype(&w, organism_id, 2, variance[2])).round().max(1.0);
            let interval = ((jittered * juvenile_plastochron).round() as u8).max(1);
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
            // An interval of 1 leafs on every step, so "due" and "not due"
            // are the same case and the offsets below cannot discriminate.
            // That is reachable now rather than hypothetical: a one-cell
            // test plant is juvenile, and `juvenile_plastochron` of 0.25 on
            // the outer orders' base of 2 rounds to 1. Skipped rather than
            // asserted, because it is a real configuration and not a bug --
            // orders 0 and 1 still carry the test.
            if interval <= 1 {
                continue;
            }
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

    // --- The genome re-map's guard set --------------------------------
    //
    // `Reports/plant-genome-design.md` §5 is the contract these protect;
    // the handoff's §3 names each one and what it is for. They are
    // deliberately a mix of pure-function identities and paired stand
    // comparisons: an identity says the seam did not move, and only a
    // paired run says a lever reaches the world at all.

    /// **The seam that made the stomatal locus free to land.**
    ///
    /// The water economy tuned `drought_death: 0.003` against shedding
    /// keyed on `1 - water_status`. Splitting the two terms is only
    /// harmless because a species that has not set `stomatal_reserve`
    /// gets the identical number back: openness is 1, both draws are
    /// `min(stock, demand)`, and desiccation is `1 - status` *exactly*,
    /// not nearly. If this ever fails, `plant-genome-design.md` §4.3's
    /// seam is broken and every species' drought tuning is back open.
    ///
    /// A grid rather than a couple of hand-picked points, because the
    /// interesting cases are the boundaries: an empty tank, an exactly-met
    /// demand, a surplus, and demand 0 (the branch where status is 1 by
    /// definition and desiccation must still be 0, not 1).
    #[test]
    fn settle_water_keeps_desiccation_and_status_identical_without_a_reserve() {
        let capacity = 10.0;
        for stock_i in 0..=20 {
            let stock = stock_i as f32 * 0.5;
            for demand_i in 0..=20 {
                let demand = demand_i as f32 * 0.5;
                let (drawn, status, desiccation) = settle_water(stock, capacity, demand, 0.0);
                assert_eq!(
                    desiccation,
                    1.0 - status,
                    "with no reserve the two terms must be the same number: stock {stock}, demand {demand} gave status {status}, desiccation {desiccation}"
                );
                assert!(drawn <= stock + f32::EPSILON, "a plant cannot spend water it does not hold");
                assert!((0.0..=1.0).contains(&status) && (0.0..=1.0).contains(&desiccation), "both terms are fractions");
            }
        }
    }

    /// The other half of the same seam: **closure may only ever move the
    /// two terms apart in one direction.** A prudent plant spends less
    /// (lower status) while drying out no faster (desiccation unchanged),
    /// so desiccation can never exceed `1 - status`. The reverse would be
    /// the trade-inversion §4.3 exists to prevent -- a conservative
    /// individual shedding hardest while its tank is still full.
    #[test]
    fn a_stomatal_reserve_costs_earnings_without_adding_thirst() {
        let capacity = 10.0;
        let mut closure_ever_bit = false;
        for stock_i in 0..=20 {
            let stock = stock_i as f32 * 0.5;
            for demand_i in 0..=20 {
                let demand = demand_i as f32 * 0.5;
                for reserve in [0.1, 0.2, 0.5, 1.0] {
                    let (_, status, desiccation) = settle_water(stock, capacity, demand, reserve);
                    let (_, open_status, open_desiccation) = settle_water(stock, capacity, demand, 0.0);
                    assert!(
                        desiccation <= 1.0 - status + 1e-6,
                        "prudence read as thirst: reserve {reserve}, stock {stock}, demand {demand} gave status {status}, desiccation {desiccation}"
                    );
                    assert!(status <= open_status + 1e-6, "closing stomata must never earn *more* than leaving them open");
                    assert_eq!(desiccation, open_desiccation, "desiccation is the open-stomata shortfall and must not depend on the reserve at all");
                    if status < open_status - 1e-6 {
                        closure_ever_bit = true;
                    }
                }
            }
        }
        // The vacuity check `CLAUDE.md` asks for: an inequality that no
        // case ever exercises passes for the wrong reason.
        assert!(closure_ever_bit, "no grid point actually closed any stomata -- this test would pass on a dead lever");
    }

    /// **Hard ground is expensive ground** -- the bill that makes
    /// `penetration_force` a trait instead of a free unlock
    /// (`plant-genome-design.md` §4.7). Without it selection saturates the
    /// slot high and it stops varying, which is precisely how slots 1 and
    /// 5 died the first time.
    ///
    /// The numbers are resistance against loose soil's 0.8, so soil is
    /// exactly 1.0 by construction and everything softer is floored there
    /// rather than paying a *discount* for easy ground.
    #[test]
    fn penetration_cost_scales_with_resistance() {
        let mut w = test_world();
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        let gravel = w.materials.id_of("gravel").expect("gravel is compiled in");
        w.set(10, 10, Cell::new(soil, 0));
        w.set(11, 10, Cell::new(material::SAND, 0));
        w.set(12, 10, Cell::new(gravel, 0));
        w.set(13, 10, Cell::new(material::STONE, 0));
        // (14, 10) left empty.

        let mult = |x: i32| penetration_cost_mult(&w, x, 10);
        assert!((mult(10) - 1.0).abs() < 1e-6, "soil is the baseline and must cost exactly 1x; got {}", mult(10));
        assert!((mult(11) - 1.75).abs() < 1e-6, "sand's 1.4 against soil's 0.8 is 1.75x; got {}", mult(11));
        assert!((mult(12) - 4.375).abs() < 1e-6, "gravel's 3.5 against soil's 0.8 is 4.375x; got {}", mult(12));
        // A `Solid` is refused by `growable` and never reached here, so
        // the multiplier must not invent a price for it -- a root does not
        // pay to fail.
        assert!((mult(13) - 1.0).abs() < 1e-6, "stone is not penetrable ground and must not be priced; got {}", mult(13));
        assert!((mult(14) - 1.0).abs() < 1e-6, "open air is free; got {}", mult(14));
    }

    /// **The seed is its provisions.** `Reproduce.seed_cost` used to be
    /// deducted from the parent and vanish; a big seed bought its child
    /// nothing, which is half of why the seed-strategy locus was deferred
    /// rather than allocated (`plant-genome-design.md` §4.8). The
    /// plumbing that makes the endowment→establishment curve measurable
    /// is that the paid cost rides on the seed and lands as the
    /// seedling's first carbon.
    ///
    /// **Two runs of one scene, not one run against a remembered
    /// number.** Both individuals are `inherited`, so both carry the same
    /// (zero) genome and germinate on the same frame from the same draw;
    /// the only difference in the world is the stake, so the difference in
    /// carbon *is* the stake. Comparing against a hardcoded 0.3 would
    /// instead be asserting whatever income happened to arrive on the
    /// germination tick.
    #[test]
    fn a_bred_seedling_starts_with_its_parents_stake() {
        const STAKE: f32 = 0.3; // tree.ron's own `Reproduce.seed_cost`.
        let carbon_at_germination = |endowment: f32| -> f32 {
            let mut w = test_world();
            plant_tree_on_ground(&mut w, 50, 20);
            let id = w.get(50, 20).organism_id();
            assert_ne!(id, 0, "test setup: the planted seed should own its own cell");
            let state = w.organism_mut(id).expect("a just-planted seed has state");
            // `inherited` is what `set_seed` marks a bred seed with, and
            // it is load-bearing here for two reasons: it stops
            // `seed_genotype` redrawing over the genome at germination,
            // and it is the state a real bred seed is actually in.
            state.inherited = true;
            state.endowment = endowment;
            for _ in 0..2_000 {
                run_with_fields(&mut w, 1);
                if organism::cell_type(w.get(50, 20).aux()) == Some(CellType::GrowingTip) {
                    return w.carbon_at(50, 20);
                }
            }
            panic!("test setup: the seed never germinated -- check the scene still holds soil under it");
        };

        let broke = carbon_at_germination(0.0);
        let staked = carbon_at_germination(STAKE);
        assert!(
            (staked - broke - STAKE).abs() < 1e-4,
            "a bred seedling should start holding exactly what its parent paid: staked {staked}, unprovisioned {broke}, difference {} against a stake of {STAKE}",
            staked - broke
        );
        assert!(broke < STAKE, "test setup: an unprovisioned seedling must start poorer than the stake, or this measures nothing (got {broke})");
    }

    /// **The bill on the acquisitive leaf.** `LOCUS_LEAF_ECONOMY` is only
    /// a trade because the rate and the transpirational demand move
    /// together; a free rate axis would be selection candy and the locus
    /// would saturate in one direction, which is test 2 of
    /// `plant-genome-design.md` §2 and the reason two of the old
    /// continuous slots were dead.
    ///
    /// Hand-built and identical on both sides on purpose. Growing two
    /// stands and dividing demand by leaf count would compare canopies of
    /// different size *and* different light — the multiplier is per leaf
    /// per unit light, so anything that changes either confounds it. Two
    /// plants of the same shape under the same open sky leave the allele
    /// as the only difference, and the ratio comes out at the table value.
    #[test]
    fn expensive_leaves_demand_more_water() {
        let mut w = test_world();
        let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
        let leaf = w.materials.id_of("leaf").expect("leaf is a compiled-in material");
        let tree = w.species.id_of("tree").expect("tree is a compiled-in species");

        // One scene, two plants: same shape, same row, open sky over both.
        let mut build = |x: i32, economy: u8| -> u16 {
            let id = w.push_organism(tree);
            place(&mut w, (x, 50), wood, id, CellType::MatureBody, (1.0, 0.0));
            for dx in -2..=2 {
                place(&mut w, (x + dx, 49), leaf, id, CellType::Leaf, (1.0, 0.0));
            }
            let state = w.organism_mut(id).expect("a pushed organism has state");
            // `inherited` so nothing redraws the genome; only the economy
            // allele differs between the two.
            state.inherited = true;
            state.alleles[organism::LOCUS_LEAF_ECONOMY] = economy;
            id
        };
        let acquisitive = build(40, 0);
        let conservative = build(120, 1);

        // The demand sum reads `ambient_light_above`, so the sky cast has
        // to have run -- with no light there is no demand at all and the
        // ratio below would be 0/0. `noon_equivalent_light` divides the
        // day cycle out, so the phase this lands on does not matter.
        for _ in 0..8 {
            field::step(&mut w);
        }
        organism_upkeep(&mut w, acquisitive);
        organism_upkeep(&mut w, conservative);

        let demand = |id: u16| w.organism(id).expect("live organism").water_demand;
        let (dark, pale) = (demand(acquisitive), demand(conservative));
        assert!(dark > 0.0 && pale > 0.0, "test setup: neither plant read any light, so this measures nothing (dark {dark}, pale {pale})");

        let expected = organism::LEAF_TRANSPIRATION_ALLELES[0] / organism::LEAF_TRANSPIRATION_ALLELES[1];
        let ratio = dark / pale;
        assert!(
            (ratio - expected).abs() < 0.01,
            "the expensive leaf must spend its allele's share more water: demand {dark} against {pale} is {ratio}x, expected {expected}x"
        );
    }

    /// Scratch for WP-A: what carbon does a cell actually hold, by class?
    ///
    /// The primed-site repair has a choice of funding source -- the primed
    /// site's own local carbon, or the plant's richest cell the way
    /// `break_root_tips` and `break_buds` both already do. Choosing the
    /// first without measuring would risk rebuilding the starved gate one
    /// cell over, which is the whole defect being repaired.
    ///
    /// ```text
    /// cargo test --lib print_carbon_by_cell_class -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn print_carbon_by_cell_class() {
        let mut w = test_world();
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        const HALF: i32 = 30;
        const ROWS: i32 = 30;
        let (px, py) = (100, 40);
        for fx in (px - HALF - 1)..=(px + HALF + 1) {
            w.set(fx, py + ROWS + 1, Cell::new(material::STONE, 0));
        }
        for dy in 1..=ROWS {
            w.set(px - HALF - 1, py + dy, Cell::new(material::STONE, 0));
            w.set(px + HALF + 1, py + dy, Cell::new(material::STONE, 0));
            for fx in (px - HALF)..=(px + HALF) {
                w.set(fx, py + dy, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
            }
        }
        w.plant_tree(px, py);
        run_with_fields(&mut w, 12_000);

        let b = w.bounds().unwrap();
        let mut root_mature: Vec<f32> = Vec::new();
        let mut shoot_mature: Vec<f32> = Vec::new();
        let mut root_tips: Vec<f32> = Vec::new();
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let c = w.get(x, y);
                if c.organism_id() == 0 {
                    continue;
                }
                let carbon = w.carbon_at(x, y);
                let is_root = w.materials.get(c.material).reinforces_powder;
                match organism::cell_type(c.aux()) {
                    Some(CellType::RootTip) => root_tips.push(carbon),
                    Some(CellType::MatureBody) if is_root => root_mature.push(carbon),
                    Some(CellType::MatureBody) => shoot_mature.push(carbon),
                    _ => {}
                }
            }
        }
        let stat = |name: &str, v: &mut Vec<f32>| {
            if v.is_empty() {
                println!("  {name:>14}: none");
                return;
            }
            v.sort_by(f32::total_cmp);
            let pick = |q: f32| v[((v.len() - 1) as f32 * q) as usize];
            println!(
                "  {name:>14}: n {:>5}  min {:.3}  p50 {:.3}  p90 {:.3}  max {:.3}  >=0.25: {:.0}%",
                v.len(),
                v[0],
                pick(0.5),
                pick(0.9),
                v[v.len() - 1],
                100.0 * v.iter().filter(|&&c| c >= 0.25).count() as f32 / v.len() as f32
            );
        };
        println!("carbon by cell class (root Grow.cost is 0.25):");
        stat("root tips", &mut root_tips);
        stat("root mature", &mut root_mature);
        stat("shoot mature", &mut shoot_mature);
    }

    /// **Root and shoot branching read different slots, and slot 1 finally
    /// reaches the world.**
    ///
    /// The guard the genome re-map promised and could not write. Slot 1 was
    /// wired correctly from the day it landed and produced a *bit-identical*
    /// stand at every draw, because its consumer sat behind a gate that
    /// demanded a second step's carbon in the tick the first was paid: 351
    /// root growth steps, 2 affordable, 0 fired
    /// (`Reports/plant-genome-design.md` §8a). The economy was not the
    /// defect and was not changed; the shape of the purchase was
    /// (`OrganismCell::primed`).
    ///
    /// Both halves of the claim are asserted. **Root mass must order with
    /// the draw** — that is the lever working. **Shoot mass must not** —
    /// that is the lever being a *root* lever, which is the whole point of
    /// slot 1 existing separately from slot 0.
    ///
    /// Bars are set from the measured pairing with headroom, per house
    /// convention, never sitting on it: measured 336 / 386 / 448 root cells
    /// at draws −1 / 0 / +1 (a 33% spread) against shoot 2418 / 2380 / 2432
    /// (2%). The bars below are 10% and 20% — comfortably inside the
    /// measured effect and comfortably outside the measured noise, so the
    /// test fails on the mechanism dying rather than on a re-tune.
    ///
    /// Two runs rather than three: the endpoints carry the claim and the
    /// midpoint costs another 12,000 frames.
    /// `print_root_branch_slot_pairing` keeps the full reproduction.
    #[test]
    fn root_and_shoot_branching_read_different_slots() {
        let (root_low, shoot_low) = root_slot_run(1, 1, -1.0, 12_000);
        let (root_high, shoot_high) = root_slot_run(1, 1, 1.0, 12_000);

        assert!(
            root_high as f32 > root_low as f32 * 1.10,
            "slot 1 must order root mass: draw -1 grew {root_low} root cells, draw +1 grew {root_high}              (measured 336 against 448; before the primed-site repair both were 352, bit-identical)"
        );
        let shoot_spread = (shoot_high as f32 - shoot_low as f32).abs() / shoot_low.max(1) as f32;
        assert!(
            shoot_spread < 0.20,
            "slot 1 is a ROOT slot and must not move the shoot: {shoot_low} against {shoot_high} is {:.0}%              (measured 2%). A shoot moving with it means the draw is reaching slot 0's consumer.",
            shoot_spread * 100.0
        );
    }

    /// **A dead plant gives its slot back, and its old id stays dead.**
    ///
    /// `Cell::organism_id` spends twelve bits on the slot index, so there
    /// are 4,095 of them, and until now `free_organism_slots` was popped
    /// but never pushed — every seed ever set consumed a slot permanently.
    /// The ceiling was on *cumulative* organisms rather than live ones
    /// (`pixel-physics-issues.md` #8).
    ///
    /// The half that matters more than the recycling is the second
    /// assertion: a stale id must resolve to `None` rather than silently
    /// reading whatever organism has since taken the slot. That is the
    /// whole reason the id carries a generation, and it was never
    /// exercised against a real free/reuse cycle before — only against
    /// hand-encoded ids, because nothing freed anything.
    #[test]
    fn a_dead_plants_slot_is_reused_and_its_old_id_stays_dead() {
        let mut w = test_world();
        plant_tree_on_ground(&mut w, 50, 20);
        let doomed = w.get(50, 20).organism_id();
        assert_ne!(doomed, 0, "test setup: the planted seed should own its cell");
        let (slots_before, live_before) = w.organism_slot_usage();

        // Kill it outright: erase every cell it owns, which is what a
        // burned or dug-out plant leaves behind.
        let b = w.bounds().unwrap();
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                if w.get(x, y).organism_id() == doomed {
                    w.set(x, y, Cell::EMPTY);
                }
            }
        }
        // One organism pass is what notices.
        run(&mut w, ORGANISM_TICK_INTERVAL as usize * 2);

        assert!(w.organism_state(doomed).is_none(), "a plant with no cells left should have given its slot back");
        let (_, live_after) = w.organism_slot_usage();
        assert!(live_after < live_before, "the live-slot count should have fallen: {live_before} -> {live_after}");

        // The slot is reused rather than the table growing.
        plant_tree_on_ground(&mut w, 120, 20);
        let reborn = w.get(120, 20).organism_id();
        let (slots_after, _) = w.organism_slot_usage();
        assert_eq!(slots_after, slots_before, "a freed slot must be reused before the table grows: {slots_before} -> {slots_after}");

        // **The generational check, against a real reuse.** Same slot
        // index, different generation, so the old id is not the new plant.
        assert_ne!(reborn, doomed, "the reused slot must hand out a different id");
        assert!(w.organism_state(doomed).is_none(), "the stale id must not resolve to the organism that took its slot");
        assert!(w.organism_state(reborn).is_some(), "the new plant's id must resolve");

        // A stale scheduled site must not resurrect anything or panic.
        w.schedule_active_site(reschedule_organism(50, 20, doomed, 0, 0, w.frame + 1));
        run(&mut w, 4);
        assert_eq!(w.get(50, 20).material, material::EMPTY, "a stale site must not regrow a dead plant's cell");
        assert!(w.organism_state(doomed).is_none(), "a stale site must not revive the dead id");
    }

    /// Scratch for WP-D: which liveness rule would actually reclaim a slot?
    ///
    /// Two candidates. "No cells at all" cannot orphan anything and needs
    /// no traversal; "no live tip/leaf/root" also catches a standing dead
    /// trunk, but freeing that leaves its cells pointing at a dead slot.
    /// Which is worth building depends on which case actually occurs, so
    /// this counts both on a real stand before either is written.
    ///
    /// ```text
    /// cargo test --lib print_dead_organism_shapes -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn print_dead_organism_shapes() {
        let mut w = test_world();
        for x in [40, 90, 140] {
            plant_tree_on_ground(&mut w, x, 60);
        }
        run_with_fields(&mut w, 30_000);

        let (mut no_cells, mut cells_no_live, mut live, mut total) = (0, 0, 0, 0);
        let mut stranded_cells = 0usize;
        for id in w.live_organism_ids() {
            let Some(state) = w.organism_state(id) else { continue };
            total += 1;
            let cells: Vec<(i32, i32)> = state.cells.keys().copied().collect();
            if cells.is_empty() {
                no_cells += 1;
                continue;
            }
            let any_live = cells.iter().any(|&(x, y)| {
                w.get(x, y).organism_id() == id
                    && matches!(
                        organism::cell_type(w.get(x, y).aux()),
                        Some(CellType::GrowingTip) | Some(CellType::Leaf) | Some(CellType::RootTip) | Some(CellType::Seed)
                    )
            });
            if any_live {
                live += 1;
            } else {
                cells_no_live += 1;
                stranded_cells += cells.len();
            }
        }
        println!("organism slots after 30,000 frames, 3 trees:");
        println!("  total live slots            {total}");
        println!("  with no cells at all        {no_cells}   <- free-able with zero orphan risk");
        println!("  cells but nothing alive     {cells_no_live}   ({stranded_cells} cells would be orphaned)");
        println!("  genuinely alive             {live}");
    }

    /// Scratch for §8e: **where** does the stomatal reserve actually close?
    ///
    /// The wet-scene deltas were predicted to be ~0 and were not (12% of
    /// stand mass, two established plants). The tidy explanation —
    /// capacity scales with root mass, so a big plant sits under its own
    /// reserve line — was refuted by measurement: stock/capacity came out
    /// at 0.41 against a 0.2 reserve, so openness clamps to 1.0 for a
    /// mature plant. The surviving candidate is the *seedling*, which
    /// carries ~11 root cells and therefore a capacity of 44 against
    /// almost no stock, right where establishment margins are thinnest.
    ///
    /// This counts closure events bucketed by shoot size on a wet stand.
    /// If the hypothesis is right, closure concentrates in the smallest
    /// bucket and is rare in the largest.
    ///
    /// ```text
    /// cargo test --lib print_closure_by_plant_size -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn print_closure_by_plant_size() {
        for c in S8E.iter() {
            c.store(0, std::sync::atomic::Ordering::Relaxed);
        }
        let mut w = test_world();
        for x in [40, 90, 140] {
            plant_tree_on_ground(&mut w, x, 60);
        }
        run_with_fields(&mut w, 30_000);
        let g = |i: usize| S8E[i].load(std::sync::atomic::Ordering::Relaxed);
        println!("closure by shoot size (wet stand, stomatal_reserve as shipped):");
        for (i, name) in ["seedling (<20)", "young (<200)", "mature (>=200)"].iter().enumerate() {
            let (total, closed) = (g(i * 2), g(i * 2 + 1));
            println!("  {name:>16}: {closed:>7} closed of {total:>7} settles  ({:.1}%)", 100.0 * closed as f32 / total.max(1) as f32);
        }
    }

    /// Scratch for WP-A: one run, for sweeping `branch_priming` and for
    /// confirming `MAX_ROOT_FRACTION` still binds at an extreme setting.
    ///
    /// ```text
    /// cargo test --lib print_priming_point -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn print_priming_point() {
        let (root, shoot) = root_slot_run(1, 1, 0.0, 12_000);
        let total = (root + shoot).max(1) as f32;
        println!("  ROOT {root}  SHOOT {shoot}  root-share {:.3}  (MAX_ROOT_FRACTION is 0.5)", root as f32 / total);
    }

    /// **The reproduction for the root slots, and for the one of them
    /// that does not reach the world.**
    ///
    /// Written to set the bar for the guard test the handoff asked for
    /// (`root_and_shoot_branching_read_different_slots`, "assert
    /// `root_cells` orders with the draw"). That test is **not here**,
    /// because this probe says its premise is false, and the numbers are
    /// kept so nobody has to find that out twice. One tree, a 61x30
    /// walled soil bed — far more room than `plant_tree_on_ground`'s
    /// 17x8, deliberately, so the scene cannot be what bounds the root
    /// system — 12,000 frames, genome frozen with `inherited` and one
    /// slot moved to ±1.0:
    ///
    /// ```text
    /// slot                 draw -1        draw 0        draw +1
    ///  1 root branch    352 / 2440    352 / 2440    352 / 2440   (root/shoot cells)
    ///  5 root tropism   444 / 2453    352 / 2440    362 / 2353
    ///  6 allocation     305 / 2621    352 / 2440    440 / 2478
    /// ```
    ///
    /// Slot 6 orders monotonically and its root:shoot ratio with it
    /// (0.116 / 0.144 / 0.178) — §4.6's claim, holding. Slot 5 moves the
    /// stand but not monotonically in *count*, which is expected: §4.5's
    /// claim is about the depth histogram (deep and narrow against
    /// shallow and wide), and a wandering low-gain root laying more
    /// shallow cells is that claim, not a contradiction of it.
    ///
    /// **Slot 1 is bit-identical at every draw**, and per `CLAUDE.md` an
    /// exactly-zero delta means suspecting the condition before the
    /// lever. It is the condition. Instrumenting the branch gate
    /// (counters since removed — they were diagnostics, not something to
    /// leave in `organism_tick`) over one 12,000-frame run:
    ///
    /// ```text
    /// root growth steps reaching the branch gate  351
    ///   of those, holding `resource >= cost`        2
    ///   of those, the 0.04 roll firing              0
    /// carbon a root tip holds at the gate: mean 0.053, max 1.72, cost 0.25
    /// ```
    ///
    /// A root tip finishes its step holding about a fifth of what a
    /// second step costs, so `branch_chance` — whatever the genome
    /// multiplies it by — is behind a gate the root economy clears twice
    /// in twelve thousand frames. The multiplier itself is plumbed
    /// correctly (0.5 / 1.0 / 1.5 read back at the consumer); it has
    /// nothing to act on. The same counters under slot 6 at +1.0 — which
    /// is the lever that funds roots — read 53 affordances and one firing,
    /// so this is starvation, not dead code.
    ///
    /// ```text
    /// cargo test --lib print_root_branch_slot_pairing -- --ignored --nocapture
    /// ```
    /// **Which frames does it not rain on, at the seed the slot pairing
    /// uses?** — the control `Reports/open-bugs-handoff.md` §A names and
    /// nobody had run.
    ///
    /// `weather::at` is a pure function of `(seed, frame)`, so a dry window
    /// can be found without stepping a world at all. This exists because
    /// the plant harness `run`/`run_with_fields` drive `update::step`,
    /// whose very first call is `weather::step` — every plant test is
    /// rained on, and the numbers the slot pairing was calibrated against
    /// were measured on a branch that had no weather at all.
    ///
    /// ```text
    /// cargo test --lib print_dry_window_for_the_slot_seed -- --ignored --nocapture
    /// ```
    /// **A seed waits on dry ground, and germinates when rain arrives** —
    /// the paired guard for the dormancy mechanic.
    ///
    /// Three arms, because two cannot separate the failures. Dry-only would
    /// pass if germination were broken outright; wet-only would pass if the
    /// gate were ignored entirely. The third arm — the same dry bed, wetted
    /// part-way through — is the only one that shows the seed *waited and
    /// then acted*, which is the mechanic. Same scene, same seed, same
    /// frames; one number different.
    ///
    /// Written because **no existing test could catch a broken dry gate**:
    /// every bed in this suite is built at `SOIL_FIELD_CAPACITY`, so a gate
    /// that always returned true stayed green everywhere.
    #[test]
    fn a_seed_waits_for_water_and_germinates_when_it_arrives() {
        // The wilting point exactly: plant-available water there is zero,
        // so this is the driest ground that is still soil rather than an
        // arbitrary small number.
        let dry = run_seed_bed(material::SOIL_WILTING_POINT, None);
        let wet = run_seed_bed(material::SOIL_FIELD_CAPACITY, None);
        let rained = run_seed_bed(material::SOIL_WILTING_POINT, Some(material::SOIL_FIELD_CAPACITY));

        assert!(
            !dry.germinated,
            "a seed on soil at the wilting point should still be waiting: plant-available water there is exactly              zero, so germinating would be the old bug -- sprout, then starve -- in a new costume"
        );
        assert!(
            wet.germinated,
            "a seed on soil at field capacity should have germinated and did not; the gate is reading something              other than the soil below it, or the threshold is above field capacity"
        );
        assert!(
            rained.germinated && rained.after_waiting > 0,
            "a seed that sat on dry ground and was then rained on should have germinated, and should be counted as              having waited first: germinated={} after_waiting={}. This is the mechanic itself, and it is the arm              that fails if dormancy is really just a slow start.",
            rained.germinated,
            rained.after_waiting
        );
    }

    struct SeedBed {
        germinated: bool,
        after_waiting: u32,
    }

    /// One seed on a walled, floored bed at a chosen soil moisture. If
    /// `wet_to` is set, the bed is re-wetted half way through, which is what
    /// rain would do.
    ///
    /// Floored **and** walled for the reason `plant_tree_on_ground` records:
    /// an open-sided powder bed avalanches out from under the seed, and a
    /// seed that rode its own bed downhill is not a seed that declined to
    /// germinate.
    fn run_seed_bed(moisture: u16, wet_to: Option<u16>) -> SeedBed {
        let mut w = test_world();
        // **Seed 1, from frame 0, because that window is provably rain-free**
        // -- `print_dry_window_for_the_slot_seed` measures 0 of frames
        // 0..12,000 precipitating at this seed, with the first rain at
        // 14,400. Without pinning it, `run_with_fields` drives the CA, the
        // CA drives `weather::step`, and a shower would wet the dry arm and
        // quietly turn this into a test of nothing.
        w.seed = 1;
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        let (x, y) = (100, 60);
        const HALF: i32 = 10;
        const ROWS: i32 = 6;
        let lay = |w: &mut World, aux: u16| {
            for fx in (x - HALF - 1)..=(x + HALF + 1) {
                w.set(fx, y + ROWS + 1, Cell::new(material::STONE, 0));
            }
            for dy in 1..=ROWS {
                w.set(x - HALF - 1, y + dy, Cell::new(material::STONE, 0));
                w.set(x + HALF + 1, y + dy, Cell::new(material::STONE, 0));
                for fx in (x - HALF)..=(x + HALF) {
                    let cell = w.get(fx, y + dy);
                    if cell.material == soil || cell.material == material::EMPTY {
                        w.set(fx, y + dy, Cell::new(soil, 0).with_aux(aux));
                    }
                }
            }
        };
        lay(&mut w, moisture);
        w.plant_tree(x, y);
        let id = w.get(x, y).organism_id();
        // `run_with_fields`, not `run`: germination gates on light as well
        // as water, and `run` never solves the light channel -- so every
        // arm would read dark and refuse for the wrong reason.
        run_with_fields(&mut w, 1_000);
        if let Some(target) = wet_to {
            lay(&mut w, target);
            run_with_fields(&mut w, 1_000);
        }
        // Germinated means the organism grew past the single seed cell it
        // started as -- a count, not a picture, and it does not care which
        // cell type it became.
        let cells = w.organism(id).map_or(0, |st| st.cells.len());
        SeedBed { germinated: cells > 1, after_waiting: w.seeds_germinated_after_waiting }
    }

    #[test]
    #[ignore]
    fn print_dry_window_for_the_slot_seed() {
        use super::super::weather;
        const SEED: u64 = 1;
        const NEED: u64 = 12_000;
        const HORIZON: u64 = weather::WEATHER_EPOCH_FRAMES * 60;

        let dry = |f: u64| weather::at(SEED, f).kind == weather::Precipitation::None;

        // Sampled every 30 frames, the same stride `weather.rs`'s own
        // `a_rainy_frame` helper uses: an epoch is thousands of frames, so
        // a 30-frame stride cannot miss a precipitation event whole.
        let (mut best_start, mut best_len) = (0u64, 0u64);
        let (mut run_start, mut run_len) = (0u64, 0u64);
        for f in (0..HORIZON).step_by(30) {
            if dry(f) {
                if run_len == 0 {
                    run_start = f;
                }
                run_len += 30;
                if run_len > best_len {
                    best_len = run_len;
                    best_start = run_start;
                }
            } else {
                run_len = 0;
            }
        }
        println!("seed {SEED}: longest dry run {best_len} frames starting at {best_start} (need {NEED})");
        let wet: u64 = (0..NEED).step_by(30).filter(|&f| !dry(f)).count() as u64 * 30;
        println!("seed {SEED}: frames 0..{NEED} — {wet} of them precipitating ({:.0}%)", 100.0 * wet as f32 / NEED as f32);
        for epoch in 0..6u64 {
            let f = epoch * weather::WEATHER_EPOCH_FRAMES;
            let w = weather::at(SEED, f);
            println!("  epoch {epoch} (frame {f}): {:?} intensity {:.2}", w.kind, w.intensity);
        }
    }

    #[test]
    #[ignore]
    fn print_root_branch_slot_pairing() {
        for slot in [1usize, 5, 6] {
            for draw in [-1.0f32, 0.0, 1.0] {
                let (root, shoot) = root_slot_run(1, slot, draw, 12_000);
                println!("slot {slot}  draw {draw:>5.1}  root {root:>5}  shoot {shoot:>5}  ratio {:.3}", root as f32 / shoot.max(1) as f32);
            }
        }
    }

    /// **The seed sweep `root_and_shoot_branching_read_different_slots`
    /// should have been from the start.**
    ///
    /// That guard pairs a single seed at draws -1 and +1 and asserts the
    /// ordering. On a system where twelve identical trees from one genome
    /// span 31 to 153 cells, one seed per arm cannot tell "the lever broke"
    /// from "this seed reshuffled" -- and after the plant-line merge it
    /// reads 371 against 315, inverted from the 336/448 it was calibrated
    /// on. This prints the same pairing across seeds so the question is
    /// answerable: if the ordering holds on the mean and the per-seed signs
    /// are mixed, the guard's shape was wrong; if it is inverted across the
    /// board, the lever is genuinely broken.
    ///
    /// House convention for a guard over a procedural system is an order
    /// statistic over N seeds, not one seed (`CLAUDE.md`, Conventions).
    #[test]
    #[ignore]
    fn print_root_branch_slot_seed_sweep() {
        let (mut lows, mut highs, mut agree) = (Vec::new(), Vec::new(), 0usize);
        println!("seed   root(-1)  root(+1)   ratio  ordered");
        for seed in 1u64..=8 {
            let (low, _) = root_slot_run(seed, 1, -1.0, 12_000);
            let (high, _) = root_slot_run(seed, 1, 1.0, 12_000);
            let ratio = high as f32 / low.max(1) as f32;
            let ok = high as f32 > low as f32 * 1.10;
            if ok {
                agree += 1;
            }
            println!("{seed:>4}   {low:>8}  {high:>8}   {ratio:>5.2}  {}", if ok { "yes" } else { "NO" });
            lows.push(low as f32);
            highs.push(high as f32);
        }
        let mean = |v: &Vec<f32>| v.iter().sum::<f32>() / v.len() as f32;
        let (ml, mh) = (mean(&lows), mean(&highs));
        println!("mean   {ml:>8.1}  {mh:>8.1}   {:>5.2}", mh / ml.max(1.0));
        println!("seeds where +1 beat -1 by the guard's 10%: {agree}/8");
    }

    /// A deep, wide, walled soil bed — root branching needs somewhere to
    /// express itself, and `plant_tree_on_ground`'s 17x8 bed bounds the
    /// root system by the scene rather than by the genome.
    fn root_slot_run(seed: u64, slot: usize, draw: f32, frames: usize) -> (u32, u32) {
        let mut w = test_world();
        w.seed = seed;
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        const HALF: i32 = 30;
        const ROWS: i32 = 30;
        let (x, y) = (100, 40);
        for fx in (x - HALF - 1)..=(x + HALF + 1) {
            w.set(fx, y + ROWS + 1, Cell::new(material::STONE, 0));
        }
        for dy in 1..=ROWS {
            w.set(x - HALF - 1, y + dy, Cell::new(material::STONE, 0));
            w.set(x + HALF + 1, y + dy, Cell::new(material::STONE, 0));
            for fx in (x - HALF)..=(x + HALF) {
                w.set(fx, y + dy, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
            }
        }
        w.plant_tree(x, y);
        let id = w.get(x, y).organism_id();
        assert_ne!(id, 0, "test setup: the planted seed should own its own cell");
        {
            let state = w.organism_mut(id).expect("a just-planted seed has state");
            // `inherited` freezes the genome so `seed_genotype` cannot
            // redraw over it at germination -- the only supported way to
            // hand a plant a chosen genotype.
            state.inherited = true;
            state.genotype_draws[slot] = draw;
            // The species-authored discrete genome, which is what a fresh
            // draw would have given every locus but the two positional
            // ones. Without this an `inherited` plant wears allele 0
            // everywhere and is a differently-shaped tree.
            state.alleles[organism::LOCUS_BRANCH_ANGLE] = 1;
            state.alleles[organism::LOCUS_INTERNODE] = 1;
            state.alleles[organism::LOCUS_WOOD_DENSITY] = 1;
        }
        run_with_fields(&mut w, frames);
        let state = w.organism(id).expect("the organism should still be alive");
        (state.root_cells, state.shoot_cells)
    }
}
