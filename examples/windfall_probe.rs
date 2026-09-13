//! **Does the food ever reach the floor, and does anything ever reach the
//! food?** — the fruit → windfall counter the lab programme asked for.
//!
//! Gate 0 is *"an ant reaches generation 2"*, and the round before this one
//! settled that the block is **reach rather than economy**: flowers and fruit
//! stand twenty to forty rows up a stem, `windfall` is the only ground-level
//! form of either, and the census said it "never exceeds one standing cell".
//! That single sentence is three different findings wearing one number —
//! **few are produced**, **they are eaten the moment they land**, or **they
//! decay** — and they want three different fixes. Nothing here could tell
//! them apart, which is what this binary is for.
//!
//! Three things it does that no existing harness does:
//!
//! * **It counts production and standing stock separately, and divides one
//!   by the other.** `World::fruit_dropped` is the far-side effect counter
//!   for `plant::drop_organ` — every windfall that was ever created. The
//!   standing census is what is on the floor *now*. Little's law closes the
//!   two: `mean standing / production rate` is the **mean time a windfall
//!   spends standing**, so a stock of one cell is readable as "one is made
//!   every 400 frames and lasts 300" or as "four hundred are made and each
//!   lasts one frame", which are opposite worlds. Neither number alone says
//!   which.
//! * **It censuses windfall by *height*, not merely by count.** A windfall
//!   lodged in the canopy is food an ant cannot reach, and it is the same
//!   cell in the same census as one lying in the leaf litter. `on the floor`
//!   here means *within reach of an animal standing on the ground*, which is
//!   the only sense in which "the fruit reached the ground" is a claim about
//!   feeding.
//! * **`handout=` is the positive control, and it is the point of the
//!   binary.** `CLAUDE.md`'s worst-recurring failure is a number that is
//!   arithmetically correct and answers a different question, and its remedy
//!   is to construct the case whose answer you already know. Dropping fresh
//!   windfall cells on the colony's own doorstep every N frames removes the
//!   reach problem entirely and changes nothing else: if the colony still
//!   does not breed, the diagnosis was wrong and the block is the bank
//!   ceiling. If it breeds, reach is the whole of it and the fix belongs
//!   between the plant and the floor.
//!
//! ```text
//! cargo run --release --example windfall_probe -- frames=24000
//! cargo run --release --example windfall_probe -- frames=24000 colonies=0   # the no-ant control
//! cargo run --release --example windfall_probe -- frames=24000 gut=-1.0 handout=200
//! ```

use pixel_physics::lab::scenario::{Placement, Scenario};
use pixel_physics::lab::scene::LabBox;
use pixel_physics::render::Renderer;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::creature::{birth_cost, diet_yield, food_value, reproduce_at};
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::material::MaterialId;
use pixel_physics::sim::organism::{self, CellType, TRAIT_GUT_BIAS};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;
use std::collections::HashSet;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// How far above the soil surface a cell still counts as **on the floor**.
///
/// An ant is a two-cell chain standing on the ground and reaching into its
/// head's 8-neighbourhood, so anything within a couple of rows of the
/// surface is food it can take without climbing. Litter piles, so the
/// surface an ant walks on drifts upward through a run; three rows is that
/// drift plus the reach, and it is deliberately generous — the finding this
/// harness exists to make is about food that is *unreachable*, and a
/// generous floor band makes that finding harder to get rather than easier.
const FLOOR_BAND: i32 = 3;

struct Sample {
    windfall: usize,
    windfall_floor: usize,
    fruit: usize,
    flower: usize,
    /// Rows above the soil surface the highest ant head reached.
    ant_high: i32,
    /// Ants whose head is off the floor — i.e. up a stem.
    ants_aloft: usize,
    ants: usize,
    /// Rows above the soil of the lowest and highest standing organ.
    ///
    /// **The height pair is what turns "the ants never got near a flower"
    /// into a fix.** An organ standing lower than the ants climb is a
    /// *steering* failure — they can get there and do not — and one standing
    /// higher than they ever climb is a *supply* failure, which belongs to
    /// whatever puts fruit on the ground. Same census, opposite conclusions,
    /// and `best_offer` alone cannot tell them apart.
    organ_low: i32,
    organ_high: i32,
    /// Flower + fruit cells within `FLOOR_BAND` of the soil -- the same
    /// reach sense `windfall_floor` uses, applied to the organ that is
    /// still on the plant. Answers "how much of the standing crop could an
    /// animal on the ground reach without climbing" as a count rather than
    /// the `organ_low`/`organ_high` pair, which says only the extremes.
    organ_floor: usize,
}

fn census(world: &World, ground_y: i32, width: i32, windfall_id: Option<MaterialId>) -> Sample {
    let mut s = Sample {
        windfall: 0, windfall_floor: 0, fruit: 0, flower: 0,
        ant_high: 0, ants_aloft: 0, ants: 0, organ_low: i32::MAX, organ_high: 0, organ_floor: 0,
    };
    // **By organism, not by grid sweep.** Every cell this census cares about
    // is organism-owned — a windfall is a fresh child organism's `Seed` cell
    // (`plant::drop_organ`), an organ is its parent's — so the registry sees
    // all of them at a few hundred lookups instead of 163,840 per sample.
    // That is what makes sampling often enough for Little's law affordable.
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        let is_creature = world.species.get(state.species).creature.is_some();
        if is_creature {
            s.ants += 1;
            if let Some(&(_, hy)) = state.chain.first() {
                s.ant_high = s.ant_high.max(ground_y - hy);
                s.ants_aloft += usize::from(ground_y - hy > FLOOR_BAND);
            }
            continue;
        }
        for &(x, y) in state.cells.keys() {
            let cell = world.get(x, y);
            match organism::cell_type(cell.aux()) {
                Some(CellType::Fruit) | Some(CellType::Flower) => {
                    if organism::cell_type(cell.aux()) == Some(CellType::Fruit) {
                        s.fruit += 1;
                    } else {
                        s.flower += 1;
                    }
                    s.organ_low = s.organ_low.min(ground_y - y);
                    s.organ_high = s.organ_high.max(ground_y - y);
                    if y >= ground_y - FLOOR_BAND {
                        s.organ_floor += 1;
                    }
                }
                // A seed and a windfall are the same `CellType`; the
                // material is what says the seed came down inside a fruit,
                // and it is the material that carries the 960.
                Some(CellType::Seed) if Some(cell.material) == windfall_id => {
                    s.windfall += 1;
                    if y >= ground_y - FLOOR_BAND {
                        s.windfall_floor += 1;
                    }
                }
                _ => {}
            }
        }
    }
    // Windfall that has been dropped by a carrier, or otherwise parted from
    // its organism, is still food standing on the floor. Counted from the
    // grid only across the floor band, which is 512 x 7 cells rather than
    // the world.
    if let Some(wid) = windfall_id {
        for y in (ground_y - FLOOR_BAND)..=(ground_y + FLOOR_BAND) {
            for x in 0..width {
                let c = world.get(x, y);
                if c.material == wid && c.organism_id() == 0 {
                    s.windfall += 1;
                    s.windfall_floor += 1;
                }
            }
        }
    }
    s
}

/// Windfall cell positions right now, restricted to the floor band --
/// `census`'s own two sources (an organism-owned `Seed` cell wearing the
/// windfall material, and an ownerless grid cell of the same material)
/// summed as positions instead of a count, so a fate diff can tell *which*
/// cell left rather than only that the total moved.
///
/// **Floor band only, deliberately**: a fate resolves where an animal can
/// reach it or where decay proceeds, and both happen at any height, but
/// the reach question this whole binary exists to answer is about the
/// floor -- canopy-lodged windfall is already reported separately
/// (`organ_high`/`ant_high`), and tracking it here too would triple the
/// per-sample cost for a fate this probe cannot act on anyway.
fn windfall_floor_positions(world: &World, ground_y: i32, width: i32, windfall_id: MaterialId) -> HashSet<(i32, i32)> {
    let mut set = HashSet::new();
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_some() {
            continue;
        }
        for &(x, y) in state.cells.keys() {
            if y < ground_y - FLOOR_BAND || y > ground_y + FLOOR_BAND {
                continue;
            }
            let cell = world.get(x, y);
            if organism::cell_type(cell.aux()) == Some(CellType::Seed) && cell.material == windfall_id {
                set.insert((x, y));
            }
        }
    }
    for y in (ground_y - FLOOR_BAND)..=(ground_y + FLOOR_BAND) {
        for x in 0..width {
            let c = world.get(x, y);
            if c.material == windfall_id && c.organism_id() == 0 {
                set.insert((x, y));
            }
        }
    }
    set
}

/// Every cell an ant's whole chain occupies right now -- not just the head
/// `census` reads for height -- used only as the adjacency test the fate
/// diff below needs.
fn ant_positions(world: &World) -> HashSet<(i32, i32)> {
    let mut set = HashSet::new();
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_none() {
            continue;
        }
        set.extend(state.chain.iter().copied());
    }
    set
}

fn adjacent_to_any(pos: (i32, i32), set: &HashSet<(i32, i32)>) -> bool {
    let (x, y) = pos;
    (-1..=1).any(|dy| (-1..=1).any(|dx| set.contains(&(x + dx, y + dy))))
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(24_000);
    let sample_every: u64 = arg("sample").unwrap_or(30);
    let gut: f32 = arg("gut").unwrap_or(f32::NAN);
    let handout: u64 = arg("handout").unwrap_or(0);
    // How often the windfall-fate diff samples the floor band. Coarser
    // than `sample_every` on purpose -- see `windfall_floor_positions`'s
    // doc for what this trades away.
    let fate_every: u64 = arg("fate").unwrap_or(4);
    // **`milestones=30000,60000,90000,120000`** -- exact frames to print a
    // standing-stock snapshot at, on top of the periodic log. Not
    // hardcoded here: which frames matter is a property of the *session*
    // asking (the owner's own play length, a round's checkpoint), not of
    // the mechanism, so it is a parameter like every other knob on this
    // binary rather than a constant baked into it.
    let milestones: Vec<u64> = arg::<String>("milestones")
        .map(|s| s.split(',').filter_map(|v| v.trim().parse().ok()).collect())
        .unwrap_or_default();
    // **A picture of the bed at the end**, because "a colony that breeds
    // eats the stand" is a judge-by-eye claim and this project's rule is to
    // post the artifact rather than describe it. The counts that go beside
    // it are the `SUMMARY` line below.
    let png: String = arg("png").unwrap_or_default();
    // **`scenario=<name>` builds the whole bed from a saved scenario**,
    // `labforage`'s own pattern and for the same reason: the owner's
    // played bed is a mixed-species bed laid out by column with a colony
    // founded on a timeline, neither of which `founders=`/`colonies=`
    // below can express. A bad name refuses at load rather than quietly
    // running the default bed under the wrong label -- `CLAUDE.md`'s "an
    // unknown argument is silently ignored", which this binary carried
    // until now: `scenario=played_bed` parsed as nothing and ran the
    // eight-herb harness bed with no warning.
    let scenario: Option<Scenario> = arg::<String>("scenario").map(|n| {
        let mut sc = Scenario::load(&n).unwrap_or_else(|e| {
            eprintln!("scenario {n}: {e}");
            std::process::exit(2);
        });
        // `seed=` overrides the scenario's own bed seed -- see
        // `labforage.rs`'s identical line for why this has to land on
        // `sc.bed` rather than on `spec` below.
        if let Some(sd) = arg::<u64>("seed") {
            sc.bed.seed = sd;
        }
        sc
    });
    let spec = match &scenario {
        Some(s) => s.bed.clone(),
        None => LabBox {
            founders: arg("founders").unwrap_or(8),
            colonies: arg("colonies").unwrap_or(1),
            compartments: arg("walls").unwrap_or(1),
            seed: arg("seed").unwrap_or(1),
            ..LabBox::default()
        },
    };
    // Echo the parameters before building anything, scenario named right
    // here -- `plant_probe`'s 3.5-hour lesson (`CLAUDE.md`): a knob nobody
    // can see the value of is a knob nobody can tell is disconnected, and
    // that includes whether `scenario=` was even recognised.
    println!(
        "windfall probe: {frames} frames, sample every {sample_every} fate every {fate_every} | founders={} colonies={} seed={} handout={handout}{}",
        spec.founders, spec.colonies, spec.seed,
        scenario.as_ref().map(|s| format!(" scenario={} ({})", s.name, s.question)).unwrap_or_default()
    );
    // **The bed is built with no ants in it, and the colonies are founded
    // afterwards at the same columns `LabBox` would have used.** An ant's
    // `gut_bias` is read off the *organism* (`creature::gut_of`), and
    // `place_creature` copies it from the species def at the moment of
    // placement — so a species-level write after the founders are standing
    // reaches nobody, and the run measures the neutral gut while the header
    // says otherwise. `stamp_probe` records paying for exactly that failure.
    // Deferring the colony is the same scene, one step later.
    //
    // A scenario founds on its own timeline instead (`played_bed`'s colony
    // arrives at frame 6,000, not frame 0), so there is nothing to defer:
    // `s.build()` is called once, up front, and `tick_timeline` in the
    // loop below does the founding whenever the file says to.
    let (mut world, founders_planted, founders_asked, mut ants_placed) = match &scenario {
        Some(s) => {
            let (w, _p, sp) = s.build();
            println!(
                "  scenario {}: {} cells, {} plants, {} animals, {} settings applied",
                s.name, sp.cells, sp.plants, sp.animals, sp.settings
            );
            (w, sp.plants, sp.plants, sp.animals)
        }
        None => {
            let bare = LabBox { colonies: 0, ..spec.clone() };
            let (w, p) = bare.build_counted();
            (w, p.planted, p.asked, p.ants)
        }
    };

    let threshold: f32 = arg("threshold").unwrap_or(f32::NAN);
    if gut.is_finite() || threshold.is_finite() {
        let species = world.species.id_of("ant").expect("ant species");
        let mut def = world.species.get(species).creature.clone().expect("ant is a creature");
        if gut.is_finite() {
            def.traits[TRAIT_GUT_BIAS] = gut.clamp(-1.0, 1.0);
        }
        if threshold.is_finite() {
            def.reproduce_threshold = threshold;
        }
        world.species.set_creature(species, def);
    }
    if scenario.is_none() {
        for x in spec.colony_columns() {
            ants_placed += world.found_colony(x, spec.ground_y - 2);
        }
    }
    let def = world.species.get(world.species.id_of("ant").expect("ant")).creature.clone().expect("creature");
    let founder_gut = world
        .live_organism_ids()
        .iter()
        .filter_map(|id| world.organism(*id))
        .find(|s| world.species.get(s.species).creature.is_some())
        .map(|s| s.traits[TRAIT_GUT_BIAS]);

    let windfall_id = world.materials.id_of("windfall");
    // The positive control for `WF_DEBUG`'s own finding: rules out a
    // material-id mix-up (windfall/seed/litter/soil resolving to the same
    // `MaterialId`) before trusting anything the appearance trace says.
    if std::env::var("WF_DEBUG").as_deref() == Ok("1") {
        eprintln!("[wf ids] windfall={windfall_id:?} seed={:?} litter={:?} soil={:?}", world.materials.id_of("seed"), world.materials.id_of("litter"), world.materials.id_of("soil"));
    }
    let bar = birth_cost(&def);

    println!(
        "  founders {founders_planted}/{founders_asked} ants {ants_placed}"
    );
    println!(
        "  ant: start_energy {:.0} crop {:.0} digest {:.2}/tick body_energy {:.0} x {} cells | bar {bar:.0} (buds at {:.0}) | gut {:+.2} (founder reads {})",
        def.start_energy,
        def.crop_capacity,
        def.digest_rate,
        def.body_energy,
        def.body.len(),
        reproduce_at(&def).unwrap_or(f32::NAN),
        def.traits[TRAIT_GUT_BIAS],
        founder_gut.map_or("NO LIVE ANT".to_string(), |g| format!("{g:+.2}")),
    );
    let bias = founder_gut.unwrap_or(def.traits[TRAIT_GUT_BIAS]);
    for name in ["leaf", "flower", "fruit", "windfall"] {
        if let Some(id) = world.materials.id_of(name) {
            let cell = Cell::new(id, 0);
            let y = diet_yield(&world, cell, bias);
            // **A rate and a time, not a ceiling.** The bank has no roof
            // since the crop landed -- an ant digests what it carries at
            // `digest_rate` and what limits it is how long it can keep
            // feeding. So the readable question is how many ticks of this
            // food a child costs, and whether the animal out-eats its own
            // upkeep at all.
            let face = food_value(&world, cell);
            let quality = if face > 0.0 { y / face } else { 0.0 };
            let upkeep = def.idle_cost_per_cell * def.body.len() as f32;
            let net = def.digest_rate * quality - upkeep;
            println!(
                "    {name:<9} face {face:>6.0}  to this gut {y:>6.0}  -> net {net:>+7.3}/tick  {}",
                if net > 0.0 { format!("a child in {:.0} ticks of feeding", bar / net) } else { "never: upkeep outruns it".to_string() },
            );
        }
    }

    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();

    let mut samples = 0u64;
    let (mut wf_sum, mut wf_floor_sum) = (0u64, 0u64);
    let (mut wf_max, mut wf_floor_max) = (0usize, 0usize);
    let (mut fruit_sum, mut flower_sum) = (0u64, 0u64);
    let mut ant_high_max = 0i32;
    let mut organ_low_min = i32::MAX;
    let mut organ_high_max = 0i32;
    let mut aloft_sum = 0u64;
    let mut organ_floor_sum = 0u64;
    let mut handed_out = 0u64;
    // **The colony's own columns, read from wherever the colony actually
    // comes from.** `spec.colony_columns()` is right for the harness bed
    // but empty for a scenario, which founds through `placements`/
    // `timeline` `Colony` entries instead (`labforage.rs`'s identical
    // problem and fix: without this, `handout=` on a scenario indexes an
    // empty `Vec` and panics on the first payout). Falls back to bed
    // centre only if a scenario truly places no colony at all.
    let colony_cols: Vec<i32> = match &scenario {
        Some(s) => {
            let mut v: Vec<i32> = s
                .placements
                .iter()
                .chain(s.timeline.iter().map(|e| &e.what))
                .filter_map(|p| match p {
                    Placement::Colony { x, .. } => Some(*x),
                    _ => None,
                })
                .collect();
            v.sort_unstable();
            v.dedup();
            if v.is_empty() {
                v.push(spec.width / 2);
            }
            v
        }
        None => spec.colony_columns(),
    };
    let nest_col = colony_cols[0];

    // Windfall-fate bookkeeping: the floor-band position set from the
    // previous fate sample, and the ants standing near it then -- see
    // `windfall_floor_positions`'s doc for why floor band only.
    let mut fate_prev_wf: HashSet<(i32, i32)> = HashSet::new();
    let mut fate_prev_ants: HashSet<(i32, i32)> = HashSet::new();
    let mut fate_eaten_or_carried = 0u64;
    let mut fate_rotted = 0u64;
    let mut fate_unclear = 0u64;
    let soil_id = world.materials.id_of("soil");

    for f in 0..=frames {
        if let Some(sc) = &scenario {
            // The scenario's own timeline, before the census on the same
            // frame -- `labforage.rs`'s identical ordering, for the
            // identical reason: a colony founded this frame should be
            // visible to this frame's sample, not next frame's.
            let arrived = pixel_physics::lab::scenario::tick_timeline(sc, &mut world, &spec);
            ants_placed += arrived.animals;
        }
        if f % sample_every == 0 {
            let s = census(&world, spec.ground_y, spec.width, windfall_id);
            samples += 1;
            wf_sum += s.windfall as u64;
            wf_floor_sum += s.windfall_floor as u64;
            wf_max = wf_max.max(s.windfall);
            wf_floor_max = wf_floor_max.max(s.windfall_floor);
            fruit_sum += s.fruit as u64;
            flower_sum += s.flower as u64;
            ant_high_max = ant_high_max.max(s.ant_high);
            aloft_sum += s.ants_aloft as u64;
            organ_floor_sum += s.organ_floor as u64;
            if s.organ_low != i32::MAX {
                organ_low_min = organ_low_min.min(s.organ_low);
                organ_high_max = organ_high_max.max(s.organ_high);
            }
            if f % (sample_every * 40) == 0 {
                let st = world.creature_stats;
                println!(
                    "  frame {f:>6}: flower {:>4} fruit {:>4} windfall {:>3} ({} on the floor) | dropped {:>4} blocked {:>6} | \
                     ants {:>3} births {:>3} deaths {:>3} eats {:>5} | best offer {:>6.0} best bite {:>6.0} peak bank {:>6.0}",
                    s.flower, s.fruit, s.windfall, s.windfall_floor,
                    world.fruit_dropped, world.organ_ripening_blocked,
                    s.ants, st.births, st.deaths, st.eats, st.best_offer, st.best_bite, st.peak_bank,
                );
            }
            if milestones.contains(&f) {
                println!(
                    "MILESTONE frame={f} seed={} flower={} fruit={} windfall={} windfallfloor={} organfloor={} organhigh={} \
                     liveants={} dropped={} shattered={} eatencarried={fate_eaten_or_carried} rotted={fate_rotted} \
                     fateunclear={fate_unclear} wfgerm={} loosegerm={}",
                    spec.seed, s.flower, s.fruit, s.windfall, s.windfall_floor, s.organ_floor, s.organ_high,
                    s.ants,
                    world.fruit_dropped, world.organ_shattered_to_windfall,
                    world.windfall_germination_x.len(),
                    world.germinations.saturating_sub(world.windfall_germination_x.len() as u64),
                );
            }
        }
        // **The windfall-fate diff.** A cell present at the last fate
        // sample and gone at this one either rotted (decayed into `soil`
        // or into nothing, `windfall.ron`'s own `decays_into`/`decay_yield`)
        // or an ant took it -- and an ant taking it is a `pickup`, whether
        // the mouthful is then digested on the spot or carried off and
        // dropped, because `src/sim/creature.rs:5039`/`:5076` book pickup
        // and eat as one event ("the two verbs merged when the decision
        // between them went away") and the eventual drop
        // (`creature.rs:5147`/`:5149`) can land anywhere. So this diff
        // can only ever resolve two fates from the world side, not three
        // -- see the printed note at the end.
        if let Some(wid) = windfall_id {
            if f % fate_every == 0 {
                let cur_wf = windfall_floor_positions(&world, spec.ground_y, spec.width, wid);
                let cur_ants = ant_positions(&world);
                // **`WF_DEBUG=1` says where a windfall cell was first seen,
                // and by whom.** Left in rather than thrown away: this is
                // how the ecology round found a still-open discrepancy
                // between standing windfall and `World::fruit_dropped` +
                // `organ_shattered_to_windfall` on `scenario=played_bed`
                // (see `Reports/lanes/evolution-lab-ecology-measure.md`) --
                // cells appear here already `organism_id=0`, which neither
                // production counter's call site can produce on its own,
                // and every other `breaks_into`/decay/shed path was checked
                // and ruled out. `SNAP_PROBE` (`structural.rs`) is the same
                // pattern for the same reason: a counter alone cannot aim a
                // camera at a single cell.
                if std::env::var("WF_DEBUG").as_deref() == Ok("1") {
                    for &(x, y) in cur_wf.difference(&fate_prev_wf) {
                        let c = world.get(x, y);
                        eprintln!(
                            "[wf appear] frame {f} ({x},{y}) organism_id={} aux={} ct={:?} fruit_dropped={} shattered={}",
                            c.organism_id(), c.aux(), organism::cell_type(c.aux()), world.fruit_dropped, world.organ_shattered_to_windfall
                        );
                    }
                }
                for &pos in fate_prev_wf.difference(&cur_wf) {
                    let ant_adjacent = adjacent_to_any(pos, &fate_prev_ants) || adjacent_to_any(pos, &cur_ants);
                    let now = world.get(pos.0, pos.1).material;
                    if ant_adjacent {
                        fate_eaten_or_carried += 1;
                    } else if Some(now) == soil_id || now == pixel_physics::sim::material::EMPTY {
                        fate_rotted += 1;
                    } else {
                        fate_unclear += 1;
                    }
                }
                fate_prev_wf = cur_wf;
                fate_prev_ants = cur_ants;
            }
        }
        if handout > 0 && f > 0 && f % handout == 0 {
            if let Some(wid) = windfall_id {
                let x = colony_cols[(handed_out as usize) % colony_cols.len()];
                // Just above the surface, so it falls the last cell itself
                // and comes to rest on whatever the floor is by then.
                for dy in 1..=6 {
                    let y = spec.ground_y - dy;
                    if world.get(x, y).material == pixel_physics::sim::material::EMPTY {
                        world.set(x, y, Cell::new(wid, 0));
                        handed_out += 1;
                        break;
                    }
                }
            }
        }
        if f < frames {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        }
    }

    if !png.is_empty() {
        let (vw, vh) = (spec.width as u32, spec.height as u32);
        let mut buf = vec![0u8; (vw * vh * 4) as usize];
        let touched = world.take_touched_chunks();
        Renderer::new().draw(&world, &particles, &touched, &mut buf, (vw, vh), true);
        image::save_buffer(&png, &buf, vw, vh, image::ColorType::Rgba8).expect("writing the bed");
        println!("  wrote {png} ({vw}x{vh})");
    }

    let st = world.creature_stats;
    let mut deepest = 0u16;
    let mut live_ants = 0usize;
    let mut plants = 0usize;
    for id in world.live_organism_ids() {
        let Some(s) = world.organism(id) else { continue };
        if world.species.get(s.species).creature.is_some() {
            live_ants += 1;
            deepest = deepest.max(s.generation);
        } else {
            plants += 1;
        }
    }
    let mean_wf = wf_sum as f64 / samples.max(1) as f64;
    let mean_wf_floor = wf_floor_sum as f64 / samples.max(1) as f64;
    // **`fruit_dropped` alone undercounts production**, on any bed where
    // anything snaps: `World::organ_shattered_to_windfall`'s doc is the
    // finding -- `fruit.ron`'s `breaks_into: "windfall"` lets a standing
    // fruit or flower become windfall by losing structural support, with
    // no ripening and no `fruit_dropped` tick. Both paths are counted here
    // because Little's law needs *all* production, not just the
    // deliberate kind.
    let produced_dropped = world.fruit_dropped;
    let produced_shattered = world.organ_shattered_to_windfall;
    let produced = (produced_dropped + produced_shattered) as f64;
    // **Little's law, and it is the whole reason production and stock are
    // both here.** `mean standing = production rate x mean standing time`,
    // so the residence time falls out of two counters neither of which can
    // give it alone. Reported as `n/a` rather than as a divide-by-zero when
    // nothing was ever produced, because "no windfall was made" and "every
    // windfall vanished instantly" are the two findings this must separate.
    let life = if produced > 0.0 { format!("{:.0} frames", mean_wf * frames as f64 / produced) } else { "n/a (none produced)".to_string() };
    let life_floor =
        if produced > 0.0 { format!("{:.0} frames", mean_wf_floor * frames as f64 / produced) } else { "n/a".to_string() };

    println!("\n  the fruit -> windfall pipeline over {frames} frames:");
    println!("    organs built (flower + fruit set)      {}", world.organs_built);
    println!("    ripening refused for want of budget    {}", world.organ_ripening_blocked);
    println!(
        "    windfalls created: dropped (ripe, let go) {produced_dropped}  shattered (organ lost support) {produced_shattered}  total {}",
        produced_dropped + produced_shattered
    );
    println!("    handed out by this harness             {handed_out}");
    println!("    mean standing: flower {:.1}  fruit {:.1}  windfall {mean_wf:.2} ({mean_wf_floor:.2} on the floor)",
        flower_sum as f64 / samples.max(1) as f64, fruit_sum as f64 / samples.max(1) as f64);
    println!("    peak standing windfall {wf_max} ({wf_floor_max} on the floor)");
    println!(
        "    organ height above the soil: lowest ever {} rows, highest ever {organ_high_max} rows | mean flower+fruit within ground reach {:.2} of mean {:.2} standing | ants aloft, mean {:.2} of {} at the end",
        if organ_low_min == i32::MAX { "none stood".to_string() } else { format!("{organ_low_min}") },
        organ_floor_sum as f64 / samples.max(1) as f64,
        (flower_sum + fruit_sum) as f64 / samples.max(1) as f64,
        aloft_sum as f64 / samples.max(1) as f64,
        live_ants,
    );
    println!("    mean time a windfall stands: {life}  (on the floor: {life_floor})");

    let wf_departures = fate_eaten_or_carried + fate_rotted + fate_unclear;
    println!("\n  windfall fate (floor band, sampled every {fate_every} frames):");
    println!(
        "    departures observed {wf_departures} = eaten-or-carried {fate_eaten_or_carried} + rotted (soil/gone, no ant near) {fate_rotted} + unclear {fate_unclear}  | against windfalls created {} (dropped {produced_dropped} + shattered {produced_shattered})",
        produced_dropped + produced_shattered
    );
    println!(
        "    NOTE: eaten and carried are the SAME event on the world side and cannot be split without touching src/sim/creature.rs (owned by another lane this round): \
         a bite always goes into the crop first (pickups creature.rs:5039, eats creature.rs:5076 -- \"the two verbs merged when the decision between them went away\") \
         and the crop cell can be dropped anywhere later (drops creature.rs:5147, deliveries creature.rs:5149), so the world only ever sees the pickup moment. \
         Splitting them needs a per-ant crop trace across frames -- the hook a later lane should add is a material-keyed counter beside creature.rs:5039."
    );

    let wf_germ = world.windfall_germination_x.len() as u64;
    let loose_germ = world.germinations.saturating_sub(wf_germ);
    println!("\n  seedlings by origin:");
    println!(
        "    germinations from a windfall {wf_germ} | from a loose seed {loose_germ} | total germinations {} (germinations_in_place {}, a relabel-in-place overcount to watch, per open-bugs §Z4)",
        world.germinations, world.germinations_in_place
    );
    if wf_germ > 0 {
        let mut hist = [0u32; 9];
        for &x in &world.windfall_germination_x {
            let d = (x - nest_col).unsigned_abs();
            hist[(d / 32).min(8) as usize] += 1;
        }
        let bins: Vec<String> = (0..9)
            .map(|i| if i == 8 { format!("256+:{}", hist[8]) } else { format!("{}-{}:{}", i * 32, (i + 1) * 32, hist[i]) })
            .collect();
        println!("    distance |x - nest_col={nest_col}| in 32-col bins: {}", bins.join(" "));
    } else {
        println!("    distance histogram: n/a (no windfall-sourced germination this run)");
    }
    println!(
        "\n  the animals: ants {live_ants} plants {plants} | births {} denied-no-space {} deaths {} eats {} | deepest generation {deepest}",
        st.births, st.births_denied_no_space, st.deaths, st.eats
    );
    // **The larder, which is what the provisioning rule actually eats
    // from.** Pickups say food was lifted, deliveries say it reached the
    // nest, and the standing count says whether it is still there — three
    // numbers, because a colony that never picks anything up and one whose
    // stores are eaten as fast as they arrive both report an empty nest.
    println!(
        "    the larder: pickups {} deliveries {} drops {} | food cells standing within reach of a nest cell right now: {}",
        st.pickups, st.deliveries, st.drops, nest_larder(&world, spec.width, spec.height),
    );
    println!(
        "    reach: best mouthful ever OFFERED {:.0} | best ever SWALLOWED {:.0} | peak bank ever held {:.0} against a bar of {bar:.0} | highest an ant head got {ant_high_max} rows above the soil",
        st.best_offer, st.best_bite, st.peak_bank
    );
    println!(
        "SUMMARY seed={} gut={bias:.2} handout={handout} frames={frames} founders={} ants0={} \
         dropped={} shattered={produced_shattered} handed={handed_out} meanwf={mean_wf:.3} meanwffloor={mean_wf_floor:.3} maxwf={wf_max} \
         organlow={organ_low_min} organhigh={organ_high_max} organfloor={:.2} aloft={:.2} \
         deliveries={} larder={} \
         eatencarried={fate_eaten_or_carried} rotted={fate_rotted} fateunclear={fate_unclear} \
         wfgerm={wf_germ} loosegerm={loose_germ} \
         births={} deaths={} liveants={live_ants} plants={plants} gen={deepest} eats={} bestoffer={:.0} bestbite={:.0} peakbank={:.0} bar={bar:.0} anthigh={ant_high_max}",
        spec.seed, founders_planted, ants_placed, produced_dropped,
        organ_floor_sum as f64 / samples.max(1) as f64,
        aloft_sum as f64 / samples.max(1) as f64,
        st.deliveries,
        nest_larder(&world, spec.width, spec.height),
        st.births, st.deaths, st.eats, st.best_offer, st.best_bite, st.peak_bank,
    );
}

/// Food cells standing in the 8-neighbourhood of a nest cell — the colony's
/// stores, as an animal standing in them would find them.
///
/// **Counted through `food_value`, the one definition of what a mouthful is
/// worth**, so a census of the larder cannot disagree with what an ant gets
/// for biting into it (`CLAUDE.md`'s canopy-density failure is the case in
/// the other direction).
fn nest_larder(world: &World, width: i32, height: i32) -> usize {
    let Some(nest) = world.materials.id_of("nest") else { return 0 };
    let mut n = 0;
    for y in 0..height {
        for x in 0..width {
            if food_value(world, world.get(x, y)) <= 0.0 {
                continue;
            }
            let beside_nest = (-1..=1).any(|dy| {
                (-1..=1).any(|dx| (dx, dy) != (0, 0) && world.get(x + dx, y + dy).material == nest)
            });
            n += usize::from(beside_nest);
        }
    }
    n
}
